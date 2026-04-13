use std::path::Path;
use std::process::Command;

pub fn fetch_origin(repo_path: &str, branch: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["fetch", "origin", branch])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run git fetch: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git fetch failed: {}", stderr));
    }
    Ok(())
}

pub fn resolve_branch_name(
    identifier: &str,
    title: &str,
    linear_branch_name: Option<&str>,
) -> String {
    if let Some(name) = linear_branch_name {
        if !name.is_empty() {
            return name.to_string();
        }
    }
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = if slug.len() > 50 { &slug[..50] } else { &slug };
    format!("{}-{}", identifier.to_lowercase(), slug)
}

pub enum BranchStatus {
    DoesNotExist,
    ExistsLocal,
    ExistsRemote,
    ExistsBoth,
}

pub fn branch_exists(repo_path: &str, branch_name: &str) -> Result<BranchStatus, String> {
    let local = Command::new("git")
        .args(["rev-parse", "--verify", branch_name])
        .current_dir(repo_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let remote = Command::new("git")
        .args(["rev-parse", "--verify", &format!("origin/{}", branch_name)])
        .current_dir(repo_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    Ok(match (local, remote) {
        (true, true) => BranchStatus::ExistsBoth,
        (true, false) => BranchStatus::ExistsLocal,
        (false, true) => BranchStatus::ExistsRemote,
        (false, false) => BranchStatus::DoesNotExist,
    })
}

pub fn create_worktree(
    repo_path: &str,
    worktrees_dir: &str,
    branch_name: &str,
    base_ref: &str,
) -> Result<String, String> {
    let worktree_path = Path::new(worktrees_dir).join(branch_name);
    let worktree_str = worktree_path.to_string_lossy().to_string();

    // Ensure worktrees dir exists
    std::fs::create_dir_all(worktrees_dir)
        .map_err(|e| format!("Failed to create worktrees directory: {}", e))?;

    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            &worktree_str,
            "-b",
            branch_name,
            base_ref,
        ])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run git worktree add: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git worktree add failed: {}", stderr));
    }

    Ok(worktree_str)
}

pub fn use_existing_worktree(
    repo_path: &str,
    worktrees_dir: &str,
    branch_name: &str,
) -> Result<String, String> {
    let worktree_path = Path::new(worktrees_dir).join(branch_name);
    let worktree_str = worktree_path.to_string_lossy().to_string();

    if worktree_path.exists() {
        return Ok(worktree_str);
    }

    // Branch exists but no worktree — create worktree for existing branch
    std::fs::create_dir_all(worktrees_dir)
        .map_err(|e| format!("Failed to create worktrees directory: {}", e))?;

    let output = Command::new("git")
        .args(["worktree", "add", &worktree_str, branch_name])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run git worktree add: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git worktree add failed: {}", stderr));
    }

    Ok(worktree_str)
}

pub fn copy_env_files(source: &str, target: &str, patterns: &[String]) -> Result<(), String> {
    for pattern in patterns {
        let full_pattern = format!("{}/{}", source, pattern);
        if let Ok(paths) = glob::glob(&full_pattern) {
            for entry in paths.flatten() {
                if let Some(filename) = entry.file_name() {
                    let dest = Path::new(target).join(filename);
                    if !dest.exists() {
                        std::fs::copy(&entry, &dest)
                            .map_err(|e| format!("Failed to copy {}: {}", entry.display(), e))?;
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn remove_worktree(repo_path: &str, worktree_path: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["worktree", "remove", "--force", worktree_path])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run git worktree remove: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git worktree remove failed: {}", stderr));
    }
    Ok(())
}
