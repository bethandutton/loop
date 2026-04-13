mod db;
mod keychain;
mod linear;

use db::Database;
use linear::LinearClient;
use std::sync::Arc;
use tauri::Emitter;
use tauri::menu::{MenuBuilder, SubmenuBuilder};

pub struct AppState {
    pub db: Arc<Database>,
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
    title: String,
    priority: i64,
    status: String,
    branch_name: Option<String>,
    tags: Vec<String>,
}

#[tauri::command]
async fn fetch_linear_tickets() -> Result<Vec<TicketCard>, String> {
    let token = keychain::get_secret("linear_api_token")?
        .ok_or("No Linear API token configured")?;

    let client = LinearClient::new(&token);
    let issues = client.get_assigned_issues().await?;

    let tickets: Vec<TicketCard> = issues
        .into_iter()
        .map(|issue| {
            let status = linear::map_linear_state_to_status(&issue.state);
            let tags: Vec<String> = issue.labels.nodes.into_iter().map(|l| l.name).collect();
            TicketCard {
                id: issue.id,
                title: issue.title,
                priority: issue.priority,
                status: status.to_string(),
                branch_name: issue.branch_name,
                tags,
            }
        })
        .collect();

    Ok(tickets)
}

#[tauri::command]
async fn verify_linear_token(token: String) -> Result<String, String> {
    let client = LinearClient::new(&token);
    let user = client.get_viewer().await?;
    Ok(user.name)
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
        .manage(AppState { db: Arc::new(db) })
        .setup(|app| {
            // Build native macOS menu
            let app_submenu = SubmenuBuilder::new(app, "Loop")
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
            verify_linear_token,
            store_token,
            get_token,
            delete_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
