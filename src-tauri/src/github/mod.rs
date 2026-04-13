use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct GitHubClient {
    client: Client,
    token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: i64,
    pub title: String,
    pub state: String,
    pub draft: bool,
    pub html_url: String,
    pub merged: Option<bool>,
    pub user: GitHubUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: i64,
    pub state: String, // "APPROVED", "CHANGES_REQUESTED", "COMMENTED", "DISMISSED"
    pub user: GitHubUser,
    pub submitted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: i64,
    pub body: String,
    pub user: GitHubUser,
    pub created_at: String,
}

impl GitHubClient {
    pub fn new(token: &str) -> Self {
        GitHubClient {
            client: Client::new(),
            token: token.to_string(),
        }
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, String> {
        let resp = self.client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "herd-app")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
            .map_err(|e| format!("GitHub API request failed: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("GitHub API error {}: {}", status, text));
        }

        resp.json::<T>()
            .await
            .map_err(|e| format!("Failed to parse GitHub response: {}", e))
    }

    /// Find an open PR for a given branch
    pub async fn get_pr_by_branch(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<Option<PullRequest>, String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls?head={}:{}&state=open",
            owner, repo, owner, branch
        );
        let prs: Vec<PullRequest> = self.get(&url).await?;
        Ok(prs.into_iter().next())
    }

    /// Get reviews for a PR
    pub async fn get_pr_reviews(
        &self,
        owner: &str,
        repo: &str,
        pr_number: i64,
    ) -> Result<Vec<Review>, String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}/reviews",
            owner, repo, pr_number
        );
        self.get(&url).await
    }

    /// Get issue comments on a PR
    pub async fn get_pr_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: i64,
    ) -> Result<Vec<Comment>, String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/issues/{}/comments",
            owner, repo, pr_number
        );
        self.get(&url).await
    }

    /// Get review comments (inline code comments) on a PR
    pub async fn get_pr_review_comments(
        &self,
        owner: &str,
        repo: &str,
        pr_number: i64,
    ) -> Result<Vec<Comment>, String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}/comments",
            owner, repo, pr_number
        );
        self.get(&url).await
    }

    /// Get a specific PR (to check merged status)
    pub async fn get_pr(
        &self,
        owner: &str,
        repo: &str,
        pr_number: i64,
    ) -> Result<PullRequest, String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}",
            owner, repo, pr_number
        );
        self.get(&url).await
    }

    /// Get the authenticated user's login
    pub async fn get_viewer_login(&self) -> Result<String, String> {
        let user: GitHubUser = self.get("https://api.github.com/user").await?;
        Ok(user.login)
    }
}

/// Parse owner/repo from a git remote URL
pub fn parse_owner_repo(repo_path: &str) -> Result<(String, String), String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to get remote URL: {}", e))?;

    if !output.status.success() {
        return Err("No origin remote found".to_string());
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Parse HTTPS: https://github.com/owner/repo.git
    // Parse SSH: git@github.com:owner/repo.git
    let parts = if url.contains("github.com/") {
        url.split("github.com/").last()
    } else if url.contains("github.com:") {
        url.split("github.com:").last()
    } else {
        None
    };

    let parts = parts.ok_or("Could not parse GitHub URL from remote")?;
    let clean = parts.trim_end_matches(".git");
    let mut split = clean.splitn(2, '/');
    let owner = split.next().ok_or("Could not parse owner")?.to_string();
    let repo = split.next().ok_or("Could not parse repo")?.to_string();

    Ok((owner, repo))
}
