mod db;
mod keychain;
mod linear;
mod github;
mod pty;
mod services;
mod worktree;

use db::Database;
use linear::LinearClient;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri::menu::{MenuBuilder, SubmenuBuilder};

pub struct AppState {
    pub db: Arc<Database>,
    pub sessions: Arc<pty::SessionManager>,
    pub services: Arc<services::ServiceManager>,
}

// ---- Settings commands ----

#[tauri::command]
fn get_setting(state: tauri::State<AppState>, key: String) -> Result<Option<String>, String> {
    state.db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_setting(
    state: tauri::State<AppState>,
    app: tauri::AppHandle,
    key: String,
    value: String,
) -> Result<(), String> {
    state
        .db
        .set_setting(&key, &value)
        .map_err(|e| e.to_string())?;
    app.emit("setting_changed", SettingChangedPayload { key, value })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Clone, serde::Serialize)]
struct SettingChangedPayload {
    key: String,
    value: String,
}

// ---- Onboarding / Repo commands ----

#[tauri::command]
fn has_repos(state: tauri::State<AppState>) -> Result<bool, String> {
    state.db.has_repos().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_repo(
    state: tauri::State<AppState>,
    name: String,
    path: String,
    worktrees_dir: String,
    primary_branch: String,
    preview_port: i64,
) -> Result<String, String> {
    state
        .db
        .create_repo(&name, &path, &worktrees_dir, &primary_branch, preview_port)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_active_repo(state: tauri::State<AppState>) -> Result<Option<db::RepoRow>, String> {
    state.db.get_active_repo().map_err(|e| e.to_string())
}

// ---- Repo detection ----

#[derive(Clone, serde::Serialize)]
struct DetectedRepoInfo {
    name: String,
    primary_branch: String,
    worktrees_dir: String,
}

#[tauri::command]
fn detect_repo_info(path: String) -> Result<DetectedRepoInfo, String> {
    let repo_path = std::path::Path::new(&path);

    if !repo_path.join(".git").exists() && !repo_path.is_dir() {
        return Err("Not a valid directory or git repository".into());
    }

    let name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo")
        .to_string();

    let primary_branch = std::process::Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
        .current_dir(&path)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().trim_start_matches("origin/").to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "main".to_string());

    let parent = repo_path.parent().unwrap_or(repo_path);
    let worktrees_dir = parent
        .join(format!("{}-worktrees", name))
        .to_string_lossy()
        .to_string();

    Ok(DetectedRepoInfo {
        name,
        primary_branch,
        worktrees_dir,
    })
}

// ---- Claude Code detection ----

#[tauri::command]
fn check_claude_code() -> Result<ClaudeCodeStatus, String> {
    let which = std::process::Command::new("which")
        .arg("claude")
        .output()
        .ok();

    let installed = which
        .as_ref()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let path = which.and_then(|o| {
        if o.status.success() {
            String::from_utf8(o.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        } else {
            None
        }
    });

    let authenticated = if installed {
        std::process::Command::new("claude")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        false
    };

    Ok(ClaudeCodeStatus {
        installed,
        path,
        authenticated,
    })
}

#[derive(Clone, serde::Serialize)]
struct ClaudeCodeStatus {
    installed: bool,
    path: Option<String>,
    authenticated: bool,
}

// ---- Linear commands ----

#[derive(Clone, serde::Serialize)]
struct TicketCard {
    id: String,
    identifier: String,
    title: String,
    priority: i64,
    status: String,
    branch_name: Option<String>,
    tags: Vec<String>,
    project: Option<String>,
    assignee: Option<String>,
    created_at: String,
    updated_at: String,
}

#[tauri::command]
async fn fetch_linear_tickets(state: tauri::State<'_, AppState>) -> Result<Vec<TicketCard>, String> {
    let token = keychain::get_secret("linear_api_token")?
        .ok_or("No Linear API token configured")?;

    let client = LinearClient::new(&token);
    let issues = client.get_assigned_issues().await?;

    // Get active repo for upsert
    let repo_id = state.db.get_active_repo()
        .map_err(|e| e.to_string())?
        .map(|r| r.id)
        .unwrap_or_default();

    let tickets: Vec<TicketCard> = issues
        .into_iter()
        .map(|issue| {
            let status = linear::map_linear_state_to_status(&issue);
            let tags: Vec<String> = issue.labels.nodes.iter().map(|l| l.name.clone()).collect();
            let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());

            // Persist to SQLite
            let _ = state.db.upsert_ticket(
                &issue.id,
                &issue.identifier,
                &repo_id,
                &issue.title,
                status,
                issue.priority,
                &tags_json,
                issue.branch_name.as_deref(),
                &issue.created_at,
                &issue.updated_at,
            );

            TicketCard {
                id: issue.id.clone(),
                identifier: issue.identifier,
                title: issue.title,
                priority: issue.priority,
                status: status.to_string(),
                branch_name: issue.branch_name,
                tags,
                project: issue.project.as_ref().map(|p| p.name.clone()),
                assignee: issue.assignee.as_ref().map(|a| a.name.clone()),
                created_at: issue.created_at,
                updated_at: issue.updated_at,
            }
        })
        .collect();

    Ok(tickets)
}

#[tauri::command]
fn get_tickets(state: tauri::State<AppState>) -> Result<Vec<TicketCard>, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?;
    let repo_id = match repo {
        Some(r) => r.id,
        None => return Ok(vec![]),
    };
    let rows = state.db.get_all_tickets(&repo_id).map_err(|e| e.to_string())?;
    let tickets = rows.into_iter().map(|r| {
        let tags: Vec<String> = serde_json::from_str(&r.tags).unwrap_or_default();
        TicketCard {
            id: r.id,
            identifier: r.identifier,
            title: r.title,
            priority: r.priority,
            status: r.status,
            branch_name: r.branch_name,
            tags,
            project: None, // Not stored in SQLite yet
            assignee: None,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }).collect();
    Ok(tickets)
}

#[tauri::command]
fn update_ticket_status(state: tauri::State<AppState>, ticket_id: String, status: String) -> Result<(), String> {
    state.db.update_ticket_status(&ticket_id, &status).map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_linear_ticket(
    state: tauri::State<'_, AppState>,
    title: String,
    description: String,
    priority: i64,
) -> Result<TicketCard, String> {
    let token = keychain::get_secret("linear_api_token")?
        .ok_or("No Linear API token configured")?;

    let client = LinearClient::new(&token);
    let team_id = client.get_viewer_team_id().await?;
    let assignee_id = client.get_viewer_id().await?;
    let issue = client.create_issue(&team_id, &title, &description, priority, &assignee_id).await?;

    let status = linear::map_linear_state_to_status(&issue);
    let tags: Vec<String> = issue.labels.nodes.iter().map(|l| l.name.clone()).collect();
    let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());

    // Persist to SQLite
    let repo_id = state.db.get_active_repo()
        .map_err(|e| e.to_string())?
        .map(|r| r.id)
        .unwrap_or_default();
    let _ = state.db.upsert_ticket(
        &issue.id, &issue.identifier, &repo_id, &issue.title,
        status, issue.priority, &tags_json, issue.branch_name.as_deref(),
        &issue.created_at, &issue.updated_at,
    );

    Ok(TicketCard {
        id: issue.id,
        identifier: issue.identifier,
        title: issue.title,
        priority: issue.priority,
        status: status.to_string(),
        branch_name: issue.branch_name,
        tags,
        project: issue.project.as_ref().map(|p| p.name.clone()),
        assignee: issue.assignee.as_ref().map(|a| a.name.clone()),
        created_at: issue.created_at,
        updated_at: issue.updated_at,
    })
}

#[tauri::command]
async fn verify_linear_token(token: String) -> Result<String, String> {
    let client = LinearClient::new(&token);
    let user = client.get_viewer().await?;
    Ok(user.name)
}

// ---- Plan commands ----

#[tauri::command]
async fn get_ticket_description(ticket_id: String) -> Result<Option<String>, String> {
    let token = keychain::get_secret("linear_api_token")?
        .ok_or("No Linear API token configured")?;

    let _client = LinearClient::new(&token);
    let query = format!(
        r#"query {{
            issue(id: "{}") {{
                description
            }}
        }}"#,
        ticket_id
    );

    #[derive(serde::Deserialize)]
    struct IssueData {
        issue: IssueDesc,
    }
    #[derive(serde::Deserialize)]
    struct IssueDesc {
        description: Option<String>,
    }

    let body = serde_json::json!({ "query": query });
    let resp = reqwest::Client::new()
        .post("https://api.linear.app/graphql")
        .header("Authorization", &token)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    #[derive(serde::Deserialize)]
    struct GqlResp {
        data: Option<IssueData>,
    }

    let gql: GqlResp = resp.json().await.map_err(|e| e.to_string())?;
    Ok(gql.data.and_then(|d| d.issue.description))
}

#[tauri::command]
async fn save_plan_to_linear(ticket_id: String, content: String) -> Result<(), String> {
    let token = keychain::get_secret("linear_api_token")?
        .ok_or("No Linear API token configured")?;

    let escaped = content.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
    let query = format!(
        r#"mutation {{
            issueUpdate(id: "{}", input: {{ description: "{}" }}) {{
                success
            }}
        }}"#,
        ticket_id, escaped
    );

    let body = serde_json::json!({ "query": query });
    let resp = reqwest::Client::new()
        .post("https://api.linear.app/graphql")
        .header("Authorization", &token)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Linear API error: {}", resp.status()));
    }

    Ok(())
}

#[tauri::command]
async fn enhance_plan(
    _ticket_id: String,
    title: String,
    current_plan: String,
) -> Result<String, String> {
    let api_key = keychain::get_secret("anthropic_api_key")?
        .ok_or("No Anthropic API key configured. Add one in Settings.")?;

    let user_message = if current_plan.trim().is_empty() {
        format!("Ticket title: {}\n\nThere is no plan yet. Create a structured plan for this ticket.", title)
    } else {
        format!("Ticket title: {}\n\nCurrent plan:\n{}\n\nImprove this plan.", title, current_plan)
    };

    let body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 2048,
        "system": "You are a technical planning assistant. Given a ticket title and optional current plan, produce an improved plan with clear structure: Goal, Approach, Tasks (as checkboxes), and Testing strategy. Return markdown only, no commentary.",
        "messages": [
            { "role": "user", "content": user_message }
        ]
    });

    let resp = reqwest::Client::new()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Anthropic API request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Anthropic API error {}: {}", status, text));
    }

    #[derive(serde::Deserialize)]
    struct AnthropicResponse {
        content: Vec<ContentBlock>,
    }
    #[derive(serde::Deserialize)]
    struct ContentBlock {
        text: Option<String>,
    }

    let result: AnthropicResponse = resp.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;
    let text = result.content.into_iter()
        .filter_map(|b| b.text)
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        return Err("Empty response from Anthropic API".to_string());
    }

    Ok(text)
}

#[tauri::command]
async fn update_ticket_title(ticket_id: String, title: String) -> Result<(), String> {
    let token = keychain::get_secret("linear_api_token")?
        .ok_or("No Linear API token configured")?;

    let escaped = title.replace('\\', "\\\\").replace('"', "\\\"");
    let query = format!(
        r#"mutation {{ issueUpdate(id: "{}", input: {{ title: "{}" }}) {{ success }} }}"#,
        ticket_id, escaped
    );
    let body = serde_json::json!({ "query": query });
    let resp = reqwest::Client::new()
        .post("https://api.linear.app/graphql")
        .header("Authorization", &token)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Linear API error: {}", resp.status()));
    }
    Ok(())
}

#[tauri::command]
fn update_ticket_priority(state: tauri::State<AppState>, ticket_id: String, priority: i64) -> Result<(), String> {
    state.db.update_ticket_priority(&ticket_id, priority).map_err(|e| e.to_string())
}

// ---- Session / Worktree commands ----

#[tauri::command]
async fn start_ticket(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    ticket_id: String,
) -> Result<StartTicketResult, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo configured")?;

    // Fetch the ticket's branch name from Linear
    let token = keychain::get_secret("linear_api_token")?
        .ok_or("No Linear API token configured")?;
    let client = LinearClient::new(&token);
    let issues = client.get_assigned_issues().await?;
    let issue = issues.iter().find(|i| i.id == ticket_id)
        .ok_or("Ticket not found in Linear")?;

    let branch_name = worktree::resolve_branch_name(
        &issue.identifier,
        &issue.title,
        issue.branch_name.as_deref(),
    );

    // Fetch origin
    worktree::fetch_origin(&repo.path, &repo.primary_branch)?;

    // Check if branch exists
    let status = worktree::branch_exists(&repo.path, &branch_name)?;
    let worktree_path = match status {
        worktree::BranchStatus::DoesNotExist => {
            let base_ref = format!("origin/{}", repo.primary_branch);
            worktree::create_worktree(&repo.path, &repo.worktrees_dir, &branch_name, &base_ref)?
        }
        _ => {
            worktree::use_existing_worktree(&repo.path, &repo.worktrees_dir, &branch_name)?
        }
    };

    // Copy env files
    let copy_patterns = state.db.get_setting("copy_files")
        .ok()
        .flatten()
        .unwrap_or_else(|| ".env*".to_string());
    let patterns: Vec<String> = copy_patterns.split(',').map(|s| s.trim().to_string()).collect();
    let local_path = std::path::Path::new(&repo.worktrees_dir).join("_local");
    if local_path.exists() {
        let _ = worktree::copy_env_files(&local_path.to_string_lossy(), &worktree_path, &patterns);
    }

    // Find claude CLI
    let claude_path = std::process::Command::new("which")
        .arg("claude")
        .output()
        .ok()
        .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string()) } else { None })
        .ok_or("Claude Code CLI not found. Install it first.")?;

    // Create session
    let session_id = uuid::Uuid::new_v4().to_string();
    let scrollback_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("Herd")
        .join("scrollbacks");
    let scrollback_path = scrollback_dir.join(format!("{}.log", session_id));

    state.sessions.spawn_session(
        &session_id,
        &ticket_id,
        &worktree_path,
        &claude_path,
        &scrollback_path.to_string_lossy(),
        app.clone(),
    )?;

    // Update ticket in DB
    let _ = state.db.update_ticket_status(&ticket_id, "in_progress");
    let _ = state.db.update_ticket_branch(&ticket_id, &branch_name, &worktree_path, &session_id);

    Ok(StartTicketResult {
        session_id,
        branch_name,
        worktree_path,
    })
}

#[derive(Clone, serde::Serialize)]
struct StartTicketResult {
    session_id: String,
    branch_name: String,
    worktree_path: String,
}

#[tauri::command]
fn get_scrollback(state: tauri::State<AppState>, session_id: String) -> Result<Vec<u8>, String> {
    state.sessions.get_scrollback(&session_id)
}

#[tauri::command]
fn write_to_session(state: tauri::State<AppState>, session_id: String, data: Vec<u8>) -> Result<(), String> {
    state.sessions.write_to_session(&session_id, &data)
}

#[tauri::command]
fn kill_session(state: tauri::State<AppState>, session_id: String) -> Result<(), String> {
    state.sessions.kill_session(&session_id)
}

// ---- Local column / Service commands ----

#[tauri::command]
fn switch_local_branch(state: tauri::State<AppState>, branch_name: String) -> Result<(), String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;

    let local_path = std::path::Path::new(&repo.worktrees_dir).join("_local");

    // Ensure _local worktree exists
    if !local_path.exists() {
        std::fs::create_dir_all(&repo.worktrees_dir).map_err(|e| e.to_string())?;
        let output = std::process::Command::new("git")
            .args(["worktree", "add", &local_path.to_string_lossy(), &repo.primary_branch])
            .current_dir(&repo.path)
            .output()
            .map_err(|e| format!("Failed to create _local worktree: {}", e))?;
        if !output.status.success() {
            return Err(format!("git worktree add failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
    }

    // Stop all running services first
    state.services.stop_all()?;

    // Checkout the branch
    let output = std::process::Command::new("git")
        .args(["checkout", &branch_name])
        .current_dir(&local_path)
        .output()
        .map_err(|e| format!("Failed to checkout: {}", e))?;

    if !output.status.success() {
        return Err(format!("git checkout failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    Ok(())
}

#[tauri::command]
fn get_local_branch(state: tauri::State<AppState>) -> Result<Option<String>, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?;
    let repo = match repo {
        Some(r) => r,
        None => return Ok(None),
    };

    let local_path = std::path::Path::new(&repo.worktrees_dir).join("_local");
    if !local_path.exists() {
        return Ok(None);
    }

    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&local_path)
        .output()
        .map_err(|e| format!("Failed to get branch: {}", e))?;

    if output.status.success() {
        Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn detect_services(state: tauri::State<AppState>) -> Result<Vec<services::ServiceDef>, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;

    let local_path = std::path::Path::new(&repo.worktrees_dir).join("_local");
    if !local_path.exists() {
        return Ok(vec![]);
    }

    services::detect_scripts(&local_path.to_string_lossy())
}

#[tauri::command]
fn start_service(state: tauri::State<AppState>, script_name: String) -> Result<String, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;

    let local_path = std::path::Path::new(&repo.worktrees_dir).join("_local");
    let service_id = uuid::Uuid::new_v4().to_string();

    state.services.start_service(&service_id, &script_name, &local_path.to_string_lossy())?;

    Ok(service_id)
}

#[tauri::command]
fn stop_service(state: tauri::State<AppState>, service_id: String) -> Result<(), String> {
    state.services.stop_service(&service_id)
}

#[tauri::command]
fn stop_all_services(state: tauri::State<AppState>) -> Result<(), String> {
    state.services.stop_all()
}

#[tauri::command]
fn get_running_services(state: tauri::State<AppState>) -> Vec<services::ServiceStatus> {
    state.services.list_running()
}

// ---- GitHub commands ----

#[tauri::command]
async fn check_pr_status(state: tauri::State<'_, AppState>, branch_name: String) -> Result<Option<PrInfo>, String> {
    let token = match keychain::get_secret("github_api_token")? {
        Some(t) => t,
        None => return Ok(None),
    };

    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;

    let (owner, repo_name) = github::parse_owner_repo(&repo.path)?;
    let client = github::GitHubClient::new(&token);

    let pr = match client.get_pr_by_branch(&owner, &repo_name, &branch_name).await? {
        Some(pr) => pr,
        None => return Ok(None),
    };

    let reviews = client.get_pr_reviews(&owner, &repo_name, pr.number).await.unwrap_or_default();
    let comments = client.get_pr_comments(&owner, &repo_name, pr.number).await.unwrap_or_default();

    let approved = reviews.iter().any(|r| r.state == "APPROVED");
    let changes_requested = reviews.iter().any(|r| r.state == "CHANGES_REQUESTED");
    let comment_count = comments.len() as i64;

    Ok(Some(PrInfo {
        number: pr.number,
        title: pr.title,
        url: pr.html_url,
        state: pr.state,
        draft: pr.draft,
        merged: pr.merged.unwrap_or(false),
        approved,
        changes_requested,
        comment_count,
    }))
}

#[derive(Clone, serde::Serialize)]
struct PrInfo {
    number: i64,
    title: String,
    url: String,
    state: String,
    draft: bool,
    merged: bool,
    approved: bool,
    changes_requested: bool,
    comment_count: i64,
}

#[tauri::command]
fn get_github_repo_url(state: tauri::State<AppState>) -> Result<Option<String>, String> {
    let repo = match state.db.get_active_repo().map_err(|e| e.to_string())? {
        Some(r) => r,
        None => return Ok(None),
    };
    let (owner, name) = github::parse_owner_repo(&repo.path)?;
    Ok(Some(format!("https://github.com/{}/{}", owner, name)))
}

// ---- Keychain commands ----

#[tauri::command]
fn store_token(key: String, value: String) -> Result<(), String> {
    keychain::store_secret(&key, &value)
}

#[tauri::command]
fn get_token(key: String) -> Result<Option<String>, String> {
    keychain::get_secret(&key)
}

#[tauri::command]
fn delete_token(key: String) -> Result<(), String> {
    keychain::delete_secret(&key)
}

// ---- App entry point ----

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Database::new().expect("Failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            db: Arc::new(db),
            sessions: Arc::new(pty::SessionManager::new()),
            services: Arc::new(services::ServiceManager::new()),
        })
        .setup(|app| {
            // Build native macOS menu
            let app_submenu = SubmenuBuilder::new(app, "Herd")
                .about(None)
                .separator()
                .item(
                    &tauri::menu::MenuItem::with_id(
                        app,
                        "preferences",
                        "Preferences...",
                        true,
                        Some("CmdOrCtrl+,"),
                    )?,
                )
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?;

            let edit_submenu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .select_all()
                .build()?;

            let view_submenu = SubmenuBuilder::new(app, "View")
                .item(
                    &tauri::menu::MenuItem::with_id(
                        app,
                        "toggle_right",
                        "Toggle Right Panel",
                        true,
                        Some("CmdOrCtrl+B"),
                    )?,
                )
                .separator()
                .fullscreen()
                .build()?;

            let window_submenu = SubmenuBuilder::new(app, "Window")
                .minimize()
                .maximize()
                .close_window()
                .build()?;

            let menu = MenuBuilder::new(app)
                .item(&app_submenu)
                .item(&edit_submenu)
                .item(&view_submenu)
                .item(&window_submenu)
                .build()?;

            app.set_menu(menu)?;

            // Start background Linear polling task
            {
                let handle = app.handle().clone();
                let db = app.state::<AppState>().db.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                        let token = match keychain::get_secret("linear_api_token") {
                            Ok(Some(t)) => t,
                            _ => continue,
                        };
                        let repo_id = match db.get_active_repo() {
                            Ok(Some(r)) => r.id,
                            _ => continue,
                        };
                        let client = LinearClient::new(&token);
                        if let Ok(issues) = client.get_assigned_issues().await {
                            for issue in &issues {
                                let status = linear::map_linear_state_to_status(issue);
                                let tags: Vec<String> = issue.labels.nodes.iter().map(|l| l.name.clone()).collect();
                                let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
                                let _ = db.upsert_ticket(
                                    &issue.id,
                                    &issue.identifier,
                                    &repo_id,
                                    &issue.title,
                                    status,
                                    issue.priority,
                                    &tags_json,
                                    issue.branch_name.as_deref(),
                                    &issue.created_at,
                                    &issue.updated_at,
                                );
                            }
                            let _ = handle.emit("tickets_updated", ());
                        }
                    }
                });
            }

            // Start background GitHub polling task (60s)
            {
                let handle = app.handle().clone();
                let db = app.state::<AppState>().db.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                        let gh_token = match keychain::get_secret("github_api_token") {
                            Ok(Some(t)) => t,
                            _ => continue,
                        };
                        let repo = match db.get_active_repo() {
                            Ok(Some(r)) => r,
                            _ => continue,
                        };
                        let (owner, repo_name) = match github::parse_owner_repo(&repo.path) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let gh_client = github::GitHubClient::new(&gh_token);
                        let viewer_login = gh_client.get_viewer_login().await.unwrap_or_default();

                        // Get tickets with branches from DB
                        let tickets = match db.get_all_tickets(&repo.id) {
                            Ok(t) => t,
                            Err(_) => continue,
                        };

                        for ticket in &tickets {
                            let branch = match &ticket.branch_name {
                                Some(b) if !b.is_empty() => b.clone(),
                                _ => continue,
                            };

                            // Check for PR
                            if let Ok(Some(pr)) = gh_client.get_pr_by_branch(&owner, &repo_name, &branch).await {
                                // Check for new comments from others
                                let comments = gh_client.get_pr_comments(&owner, &repo_name, pr.number).await.unwrap_or_default();
                                let reviews = gh_client.get_pr_reviews(&owner, &repo_name, pr.number).await.unwrap_or_default();

                                let has_new_external_comments = comments.iter().any(|c| c.user.login != viewer_login);
                                let approved = reviews.iter().any(|r| r.state == "APPROVED");
                                let merged = pr.merged.unwrap_or(false);

                                let new_status = if merged {
                                    "done"
                                } else if approved {
                                    "ready_to_merge"
                                } else if has_new_external_comments && ticket.status != "human_input" {
                                    "human_input"
                                } else if ticket.status == "in_progress" {
                                    "waiting_for_review"
                                } else {
                                    &ticket.status
                                };

                                if new_status != ticket.status {
                                    let _ = db.update_ticket_status(&ticket.id, new_status);
                                    let _ = handle.emit("tickets_updated", ());
                                }
                            }
                        }
                    }
                });
            }

            // Handle menu events
            let app_handle = app.handle().clone();
            app.on_menu_event(move |_app, event| {
                match event.id().as_ref() {
                    "preferences" => {
                        let _ = app_handle.emit("open_settings", ());
                    }
                    "toggle_right" => {
                        let _ = app_handle.emit("toggle_right_column", ());
                    }
                    _ => {}
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_setting,
            set_setting,
            has_repos,
            create_repo,
            get_active_repo,
            detect_repo_info,
            check_claude_code,
            fetch_linear_tickets,
            get_tickets,
            update_ticket_status,
            update_ticket_priority,
            update_ticket_title,
            create_linear_ticket,
            verify_linear_token,
            get_ticket_description,
            save_plan_to_linear,
            enhance_plan,
            start_ticket,
            get_scrollback,
            write_to_session,
            kill_session,
            switch_local_branch,
            get_local_branch,
            detect_services,
            start_service,
            stop_service,
            stop_all_services,
            get_running_services,
            check_pr_status,
            get_github_repo_url,
            store_token,
            get_token,
            delete_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
