mod db;
mod keychain;

use db::Database;
use std::sync::Arc;
use tauri::Emitter;

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
        .manage(AppState { db: Arc::new(db) })
        .invoke_handler(tauri::generate_handler![
            get_setting,
            set_setting,
            has_repos,
            create_repo,
            get_active_repo,
            store_token,
            get_token,
            delete_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
