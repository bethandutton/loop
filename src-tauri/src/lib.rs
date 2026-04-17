mod db;
mod keychain;
mod github;
mod linear;
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
    pub shared_services: Arc<services::ServiceManager>,
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

// ---- Task commands ----

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

// ---- Linear picker (read-only) ----

#[derive(Clone, serde::Serialize)]
struct LinearPickerIssue {
    id: String,
    identifier: String,
    title: String,
    status: String,
    priority: i64,
    branch_name: Option<String>,
    project: Option<String>,
    tags: Vec<String>,
    in_current_cycle: bool,
    cycle_label: Option<String>,
}

#[tauri::command]
async fn fetch_linear_issues_live() -> Result<Vec<LinearPickerIssue>, String> {
    let token = keychain::get_secret("linear_api_token")?
        .ok_or("No Linear API token configured")?;
    let client = LinearClient::new(&token);
    let issues = client.get_assigned_issues().await?;

    let now = chrono::Utc::now().to_rfc3339();
    Ok(issues.into_iter().map(|i| {
        let status = linear::map_linear_state_to_status(&i);
        let tags: Vec<String> = i.labels.nodes.iter().map(|l| l.name.clone()).collect();
        let in_current_cycle = i.cycle.as_ref()
            .map(|c| {
                let started = c.starts_at.as_deref().map(|s| s <= now.as_str()).unwrap_or(false);
                let not_ended = c.ends_at.as_deref().map(|e| e >= now.as_str()).unwrap_or(true);
                started && not_ended
            })
            .unwrap_or(false);
        let cycle_label = i.cycle.as_ref().and_then(|c| {
            if let Some(n) = c.name.as_ref().filter(|s| !s.is_empty()) {
                Some(n.clone())
            } else {
                c.number.map(|n| format!("Cycle {}", n))
            }
        });
        LinearPickerIssue {
            id: i.id,
            identifier: i.identifier,
            title: i.title,
            status: status.to_string(),
            priority: i.priority,
            branch_name: i.branch_name,
            project: i.project.map(|p| p.name),
            tags,
            in_current_cycle,
            cycle_label,
        }
    }).collect())
}

#[tauri::command]
fn import_linear_task(
    state: tauri::State<AppState>,
    linear_id: String,
    identifier: String,
    title: String,
    branch_name: Option<String>,
    priority: Option<i64>,
) -> Result<TicketCard, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;

    let prio = priority.unwrap_or(0);

    // Resolve a branch name — prefer Linear's, else derive from identifier + title
    let resolved_branch = worktree::resolve_branch_name(
        &identifier,
        &title,
        branch_name.as_deref(),
    );

    // Persist the task
    state.db.import_task(&linear_id, &identifier, &repo.id, &title, Some(&resolved_branch), prio)
        .map_err(|e| e.to_string())?;

    // Create a worktree from origin/primary — best-effort, non-fatal
    let _ = worktree::fetch_origin(&repo.path, &repo.primary_branch);
    let status = worktree::branch_exists(&repo.path, &resolved_branch)
        .unwrap_or(worktree::BranchStatus::DoesNotExist);
    let wt_result = match status {
        worktree::BranchStatus::DoesNotExist => {
            let base = format!("origin/{}", repo.primary_branch);
            worktree::create_worktree(&repo.path, &repo.worktrees_dir, &resolved_branch, &base)
        }
        _ => worktree::use_existing_worktree(&repo.path, &repo.worktrees_dir, &resolved_branch),
    };
    if let Ok(path) = wt_result {
        let _ = state.db.update_ticket_branch(&linear_id, &resolved_branch, &path, "");
    }

    Ok(TicketCard {
        id: linear_id,
        identifier,
        title,
        priority: prio,
        status: "todo".to_string(),
        branch_name: Some(resolved_branch),
        tags: vec![],
        project: None,
        assignee: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
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
fn create_task(
    state: tauri::State<AppState>,
    title: String,
    description: Option<String>,
    priority: Option<i64>,
) -> Result<TicketCard, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;

    let id = uuid::Uuid::new_v4().to_string();
    let next = state.db.next_task_number(&repo.id).map_err(|e| e.to_string())?;
    let identifier = format!("T-{:03}", next);
    let prio = priority.unwrap_or(0);

    state.db.create_task(
        &id,
        &identifier,
        &repo.id,
        &title,
        description.as_deref().unwrap_or(""),
        prio,
    ).map_err(|e| e.to_string())?;

    Ok(TicketCard {
        id,
        identifier,
        title,
        priority: prio,
        status: "todo".to_string(),
        branch_name: None,
        tags: vec![],
        project: None,
        assignee: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    })
}


#[tauri::command]
fn update_ticket_priority(state: tauri::State<AppState>, ticket_id: String, priority: i64) -> Result<(), String> {
    state.db.update_ticket_priority(&ticket_id, priority).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_task(state: tauri::State<AppState>, ticket_id: String) -> Result<(), String> {
    state.db.delete_task(&ticket_id).map_err(|e| e.to_string())
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

    // Load the task from the local DB to derive a branch name
    let tickets = state.db.get_all_tickets(&repo.id).map_err(|e| e.to_string())?;
    let ticket = tickets.iter().find(|t| t.id == ticket_id)
        .ok_or("Task not found")?;

    let branch_name = worktree::resolve_branch_name(
        &ticket.identifier,
        &ticket.title,
        ticket.branch_name.as_deref(),
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
async fn create_pr(state: tauri::State<'_, AppState>, branch_name: String) -> Result<String, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;

    let worktree_path = std::path::Path::new(&repo.worktrees_dir).join(&branch_name);
    if !worktree_path.exists() {
        return Err(format!("Worktree not found at {}", worktree_path.display()));
    }

    let output = tokio::process::Command::new("gh")
        .args(["pr", "create", "--fill"])
        .current_dir(&worktree_path)
        .output()
        .await
        .map_err(|e| format!("Failed to run gh: {}. Is the GitHub CLI installed?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let msg = if !stderr.is_empty() { stderr } else { stdout };
        return Err(if msg.is_empty() { "gh pr create failed".to_string() } else { msg });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// ---- Embedded PR webview (child of main window) ----

fn find_pr_webview(app: &tauri::AppHandle) -> Option<tauri::Webview> {
    let window = app.get_webview_window("main")?;
    window
        .webviews()
        .into_iter()
        .find(|(label, _)| label == "pr-embed")
        .map(|(_, wv)| wv)
}

#[tauri::command]
async fn embed_pr_webview(
    app: tauri::AppHandle,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    use tauri::webview::WebviewBuilder;
    use tauri::{LogicalPosition, LogicalSize, WebviewUrl};

    let parsed: tauri::Url = url.parse().map_err(|e: url::ParseError| format!("Invalid URL: {}", e))?;

    if let Some(existing) = find_pr_webview(&app) {
        existing
            .set_position(LogicalPosition::new(x, y))
            .map_err(|e: tauri::Error| e.to_string())?;
        existing
            .set_size(LogicalSize::new(width, height))
            .map_err(|e: tauri::Error| e.to_string())?;
        existing
            .show()
            .map_err(|e: tauri::Error| e.to_string())?;
        existing
            .navigate(parsed)
            .map_err(|e: tauri::Error| e.to_string())?;
        return Ok(());
    }

    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    // WebviewWindow wraps a Window that supports add_child when the "unstable" feature is enabled
    window
        .as_ref()
        .window()
        .add_child(
            WebviewBuilder::new("pr-embed", WebviewUrl::External(parsed)),
            LogicalPosition::new(x, y),
            LogicalSize::new(width, height),
        )
        .map_err(|e: tauri::Error| format!("Failed to add child webview: {}", e))?;

    Ok(())
}

#[tauri::command]
fn resize_pr_webview(
    app: tauri::AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    use tauri::{LogicalPosition, LogicalSize};
    if let Some(wv) = find_pr_webview(&app) {
        wv.set_position(LogicalPosition::new(x, y))
            .map_err(|e: tauri::Error| e.to_string())?;
        wv.set_size(LogicalSize::new(width, height))
            .map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn hide_pr_webview(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(wv) = find_pr_webview(&app) {
        wv.hide().map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}


// ---- Agent detection + launch ----

#[derive(Clone, serde::Serialize)]
struct AgentAvailability {
    claude_code: bool,
    codex: bool,
    gemini: bool,
    aider: bool,
}

fn has_command(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
fn check_agents() -> AgentAvailability {
    AgentAvailability {
        claude_code: has_command("claude"),
        codex: has_command("codex"),
        gemini: has_command("gemini"),
        aider: has_command("aider"),
    }
}

#[tauri::command]
async fn start_agent(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    ticket_id: String,
    agent: String,
) -> Result<StartTicketResult, String> {
    // Resolve the binary path for the requested agent
    let cli = match agent.as_str() {
        "claude_code" => "claude",
        "codex" => "codex",
        "gemini" => "gemini",
        "aider" => "aider",
        other => return Err(format!("Unknown agent: {}", other)),
    };

    let cli_path = std::process::Command::new("which")
        .arg(cli)
        .output()
        .ok()
        .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string()) } else { None })
        .ok_or_else(|| format!("{} not found on PATH", cli))?;

    // Same worktree setup as start_ticket, but use the chosen CLI
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo configured")?;

    let tickets = state.db.get_all_tickets(&repo.id).map_err(|e| e.to_string())?;
    let ticket = tickets.iter().find(|t| t.id == ticket_id)
        .ok_or("Task not found")?;

    let branch_name = worktree::resolve_branch_name(
        &ticket.identifier,
        &ticket.title,
        ticket.branch_name.as_deref(),
    );

    worktree::fetch_origin(&repo.path, &repo.primary_branch)?;

    let status = worktree::branch_exists(&repo.path, &branch_name)?;
    let worktree_path = match status {
        worktree::BranchStatus::DoesNotExist => {
            let base_ref = format!("origin/{}", repo.primary_branch);
            worktree::create_worktree(&repo.path, &repo.worktrees_dir, &branch_name, &base_ref)?
        }
        _ => worktree::use_existing_worktree(&repo.path, &repo.worktrees_dir, &branch_name)?,
    };

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
        &cli_path,
        &scrollback_path.to_string_lossy(),
        app.clone(),
    )?;

    let _ = state.db.update_ticket_status(&ticket_id, "working");
    let _ = state.db.update_ticket_branch(&ticket_id, &branch_name, &worktree_path, &session_id);

    Ok(StartTicketResult { session_id, branch_name, worktree_path })
}

// ---- Linear description + image proxy ----

#[tauri::command]
async fn fetch_linear_description(ticket_id: String) -> Result<Option<String>, String> {
    let token = match keychain::get_secret("linear_api_token")? {
        Some(t) => t,
        None => return Ok(None),
    };
    let query = format!(
        r#"query {{ issue(id: "{}") {{ description }} }}"#,
        ticket_id
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

    #[derive(serde::Deserialize)]
    struct GqlResp { data: Option<IssueData> }
    #[derive(serde::Deserialize)]
    struct IssueData { issue: IssueDesc }
    #[derive(serde::Deserialize)]
    struct IssueDesc { description: Option<String> }

    let gql: GqlResp = resp.json().await.map_err(|e| e.to_string())?;
    Ok(gql.data.and_then(|d| d.issue.description))
}

#[tauri::command]
async fn fetch_linear_image(url: String) -> Result<Vec<u8>, String> {
    let token = keychain::get_secret("linear_api_token")?
        .ok_or("No Linear API token configured")?;
    let resp = reqwest::Client::new()
        .get(&url)
        .header("Authorization", &token)
        .send()
        .await
        .map_err(|e| format!("Image fetch failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Image fetch error: {}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("Failed to read image body: {}", e))?;
    Ok(bytes.to_vec())
}

// ---- Session activity ----

#[derive(Clone, serde::Serialize)]
struct SessionActivity {
    session_id: String,
    ticket_id: String,
    state: String, // "thinking" | "attention" | "idle"
}

#[tauri::command]
fn get_session_activity(state: tauri::State<AppState>) -> Vec<SessionActivity> {
    state.sessions.activity_snapshot()
        .into_iter()
        .map(|(sid, tid, s)| SessionActivity { session_id: sid, ticket_id: tid, state: s })
        .collect()
}

#[tauri::command]
fn mark_session_visited(state: tauri::State<AppState>, session_id: String) -> Result<(), String> {
    state.sessions.mark_visited(&session_id);
    Ok(())
}

// ---- Services (shared _local worktree) ----

fn local_worktree_path(repo: &db::RepoRow) -> std::path::PathBuf {
    std::path::Path::new(&repo.worktrees_dir).join("_local")
}

/// Ensure a shared `_local` worktree exists checked out to the given branch (or primary).
/// If the main repo already has `node_modules`, symlink it into `_local` so dev servers
/// don't need another install.
fn ensure_local_worktree(repo: &db::RepoRow, desired_branch: Option<&str>) -> Result<String, String> {
    let local_path = local_worktree_path(repo);
    let branch = desired_branch.unwrap_or(&repo.primary_branch);

    if !local_path.exists() {
        std::fs::create_dir_all(&repo.worktrees_dir).map_err(|e| e.to_string())?;
        let out = std::process::Command::new("git")
            .args(["worktree", "add", &local_path.to_string_lossy(), branch])
            .current_dir(&repo.path)
            .output()
            .map_err(|e| format!("Failed to create _local worktree: {}", e))?;
        if !out.status.success() {
            let out2 = std::process::Command::new("git")
                .args(["worktree", "add", &local_path.to_string_lossy(), &repo.primary_branch])
                .current_dir(&repo.path)
                .output()
                .map_err(|e| e.to_string())?;
            if !out2.status.success() {
                return Err(format!("git worktree add failed: {}", String::from_utf8_lossy(&out2.stderr)));
            }
        }
    }

    // pnpm workspaces keep per-package node_modules, so a single symlink at
    // the worktree root misses most of the `.bin` entries. Since pnpm uses a
    // content-addressed global store, running install in the worktree is
    // cheap — mostly symlinks into `~/Library/pnpm/store`.
    //
    // For npm/yarn we keep the root-node_modules symlink shortcut to avoid
    // doubling disk usage per worktree.
    let main_nm = std::path::Path::new(&repo.path).join("node_modules");
    let local_nm = local_path.join("node_modules");
    let is_pnpm_workspace = std::path::Path::new(&repo.path).join("pnpm-workspace.yaml").is_file();

    if is_pnpm_workspace {
        // If a stale empty node_modules is blocking install, remove it.
        if local_nm.is_dir() {
            let empty = std::fs::read_dir(&local_nm).map(|mut d| d.next().is_none()).unwrap_or(false);
            if empty {
                let _ = std::fs::remove_dir(&local_nm);
            }
        }
    } else if main_nm.is_dir() && !local_nm.exists() {
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(&main_nm, &local_nm);
        }
    }

    Ok(local_path.to_string_lossy().to_string())
}

#[derive(Clone, serde::Serialize)]
struct LocalServicesState {
    scripts: Vec<services::ServiceDef>,
    has_package_json: bool,
    node_modules_installed: bool,
    package_manager: String,
    local_path: String,
    current_branch: Option<String>,
    running: Vec<services::ServiceStatus>,
}

#[tauri::command]
fn local_services_info(state: tauri::State<AppState>) -> Result<LocalServicesState, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;
    let local_path = ensure_local_worktree(&repo, None)?;
    let (scripts, has_package_json, node_modules_installed, package_manager) =
        services::detect_scripts(&local_path)?;

    let current_branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&local_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    Ok(LocalServicesState {
        scripts,
        has_package_json,
        node_modules_installed,
        package_manager,
        local_path,
        current_branch,
        running: state.services.list_running(),
    })
}

#[tauri::command]
fn switch_local_branch(
    state: tauri::State<AppState>,
    branch: String,
) -> Result<String, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;
    let local_path = ensure_local_worktree(&repo, None)?;

    // Fetch origin to make sure the branch ref exists
    let _ = std::process::Command::new("git")
        .args(["fetch", "origin"])
        .current_dir(&repo.path)
        .output();

    // Try checkout: local branch first, then from origin, then just stay put.
    let out = std::process::Command::new("git")
        .args(["checkout", &branch])
        .current_dir(&local_path)
        .output()
        .map_err(|e| format!("git checkout failed: {}", e))?;

    if !out.status.success() {
        // Check if origin/<branch> exists before trying -B
        let ref_check = std::process::Command::new("git")
            .args(["rev-parse", "--verify", &format!("origin/{}", &branch)])
            .current_dir(&local_path)
            .output()
            .ok()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if ref_check {
            let _ = std::process::Command::new("git")
                .args(["checkout", "-B", &branch, &format!("origin/{}", &branch)])
                .current_dir(&local_path)
                .output();
            // Non-fatal if this fails: stay on whatever branch _local is on.
        }
        // If branch doesn't exist locally or remotely, stay on current branch.
    }

    state.services.update_current_branch(&branch);
    Ok(branch)
}

#[tauri::command]
fn start_local_service(
    state: tauri::State<AppState>,
    app: tauri::AppHandle,
    script_name: String,
) -> Result<String, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;
    let local_path = ensure_local_worktree(&repo, None)?;

    let current_branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&local_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    state.services.start_service(&script_name, &local_path, current_branch.as_deref(), app)?;
    Ok(script_name)
}

#[tauri::command]
fn stop_local_service(state: tauri::State<AppState>, script_name: String) -> Result<(), String> {
    state.services.stop_service(&script_name)
}

#[tauri::command]
fn get_local_service_scrollback(state: tauri::State<AppState>, script_name: String) -> Result<Vec<u8>, String> {
    state.services.get_scrollback(&script_name)
}

#[tauri::command]
fn list_local_services(state: tauri::State<AppState>) -> Vec<services::ServiceStatus> {
    state.services.list_running()
}

#[tauri::command]
fn install_local_deps(
    state: tauri::State<AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;
    let local_path = ensure_local_worktree(&repo, None)?;
    state.services.start_install(&local_path, app)?;
    Ok("install".to_string())
}

// ---- Shared services (one instance across all worktrees) ----
//
// Shared services run from the MAIN repo checkout (not a worktree). They're
// persistent across task switches — e.g. workflow engine, API, DB, queues.
// Which scripts are "shared" vs "frontend" is declared in `.herd.json`
// at the repo root.

#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
struct HerdConfig {
    #[serde(default)]
    frontend: Option<String>,
    #[serde(default)]
    shared: Vec<String>,
}

fn herd_config_path(repo: &db::RepoRow) -> std::path::PathBuf {
    std::path::Path::new(&repo.path).join(".herd.json")
}

fn read_herd_config(repo: &db::RepoRow) -> HerdConfig {
    let path = herd_config_path(repo);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<HerdConfig>(&s).ok())
        .unwrap_or_default()
}

#[tauri::command]
fn get_herd_config(state: tauri::State<AppState>) -> Result<HerdConfig, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;
    Ok(read_herd_config(&repo))
}

#[tauri::command]
fn save_herd_config(state: tauri::State<AppState>, config: HerdConfig) -> Result<(), String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;
    let path = herd_config_path(&repo);
    let body = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| format!("Failed to write .herd.json: {}", e))
}

/// Auto-suggest a default config by inspecting the repo's package.json.
/// Picks `dev:app` > `dev` > `start` as the frontend; everything else
/// that looks like a long-running dev command becomes "shared".
#[tauri::command]
fn suggest_herd_config(state: tauri::State<AppState>) -> Result<HerdConfig, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;
    let (scripts, _, _, _) = services::detect_scripts(&repo.path)?;
    let names: Vec<String> = scripts.into_iter().map(|s| s.name).collect();

    let frontend = ["dev:app", "dev:web", "dev", "start"]
        .iter()
        .find(|candidate| names.iter().any(|n| n == *candidate))
        .map(|s| s.to_string());

    // Anything else starting with "dev" or common backend names -> shared suggestion.
    let shared: Vec<String> = names.iter()
        .filter(|n| Some(n.as_str()) != frontend.as_deref())
        .filter(|n| n.starts_with("dev") || n.starts_with("serve") || n.starts_with("api") || n.starts_with("worker") || n.starts_with("queue"))
        .cloned()
        .collect();

    Ok(HerdConfig { frontend, shared })
}

#[derive(Clone, serde::Serialize)]
struct SharedServicesState {
    scripts: Vec<services::ServiceDef>,
    configured_shared: Vec<String>,
    frontend: Option<String>,
    has_package_json: bool,
    node_modules_installed: bool,
    package_manager: String,
    repo_path: String,
    running: Vec<services::ServiceStatus>,
}

#[tauri::command]
fn shared_services_info(state: tauri::State<AppState>) -> Result<SharedServicesState, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;
    let (scripts, has_package_json, node_modules_installed, package_manager) =
        services::detect_scripts(&repo.path)?;
    let config = read_herd_config(&repo);
    Ok(SharedServicesState {
        scripts,
        configured_shared: config.shared,
        frontend: config.frontend,
        has_package_json,
        node_modules_installed,
        package_manager,
        repo_path: repo.path.clone(),
        running: state.shared_services.list_running(),
    })
}

#[tauri::command]
fn start_shared_service(
    state: tauri::State<AppState>,
    app: tauri::AppHandle,
    script_name: String,
) -> Result<String, String> {
    let repo = state.db.get_active_repo().map_err(|e| e.to_string())?
        .ok_or("No active repo")?;
    state.shared_services.start_service(&script_name, &repo.path, None, app)?;
    Ok(script_name)
}

#[tauri::command]
fn stop_shared_service(state: tauri::State<AppState>, script_name: String) -> Result<(), String> {
    state.shared_services.stop_service(&script_name)
}

#[tauri::command]
fn get_shared_service_scrollback(state: tauri::State<AppState>, script_name: String) -> Result<Vec<u8>, String> {
    state.shared_services.get_scrollback(&script_name)
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            db: Arc::new(db),
            sessions: Arc::new(pty::SessionManager::new()),
            services: Arc::new(services::ServiceManager::new("service_output")),
            shared_services: Arc::new(services::ServiceManager::new("shared_service_output")),
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
            get_tickets,
            update_ticket_status,
            update_ticket_priority,
            create_task,
            delete_task,
            fetch_linear_issues_live,
            import_linear_task,
            start_ticket,
            check_agents,
            start_agent,
            fetch_linear_description,
            fetch_linear_image,
            local_services_info,
            switch_local_branch,
            start_local_service,
            stop_local_service,
            get_local_service_scrollback,
            list_local_services,
            install_local_deps,
            get_herd_config,
            save_herd_config,
            suggest_herd_config,
            shared_services_info,
            start_shared_service,
            stop_shared_service,
            get_shared_service_scrollback,
            get_session_activity,
            mark_session_visited,
            get_scrollback,
            write_to_session,
            kill_session,
            check_pr_status,
            create_pr,
            embed_pr_webview,
            resize_pr_webview,
            hide_pr_webview,
            store_token,
            get_token,
            delete_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
