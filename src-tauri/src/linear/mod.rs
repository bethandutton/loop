use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct LinearClient {
    client: Client,
    token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearIssue {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: i64,
    pub state: LinearState,
    #[serde(default)]
    pub labels: LabelConnection,
    #[serde(rename = "branchName")]
    pub branch_name: Option<String>,
    pub cycle: Option<CycleRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearState {
    pub name: String,
    #[serde(rename = "type")]
    pub state_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LabelConnection {
    #[serde(default)]
    pub nodes: Vec<LinearLabel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleRef {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearLabel {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearUser {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct ViewerData {
    viewer: LinearUser,
}

#[derive(Debug, Deserialize)]
struct IssuesData {
    viewer: ViewerIssues,
}

#[derive(Debug, Deserialize)]
struct ViewerIssues {
    #[serde(rename = "assignedIssues")]
    assigned_issues: IssueConnection,
}

#[derive(Debug, Deserialize)]
struct IssueConnection {
    nodes: Vec<LinearIssue>,
}

impl LinearClient {
    pub fn new(token: &str) -> Self {
        LinearClient {
            client: Client::new(),
            token: token.to_string(),
        }
    }

    async fn query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
    ) -> Result<T, String> {
        let body = serde_json::json!({ "query": query });

        let resp = self
            .client
            .post("https://api.linear.app/graphql")
            .header("Authorization", &self.token)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Linear API request failed: {}", e))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(format!("Linear API error {}: {}", status, text));
        }

        let gql: GraphQLResponse<T> = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse Linear response: {} — body: {}", e, &text[..200.min(text.len())]))?;

        if let Some(errors) = gql.errors {
            let msgs: Vec<String> = errors.into_iter().map(|e| e.message).collect();
            return Err(format!("Linear GraphQL errors: {}", msgs.join(", ")));
        }

        gql.data.ok_or_else(|| "No data in Linear response".to_string())
    }

    pub async fn get_viewer(&self) -> Result<LinearUser, String> {
        let data: ViewerData = self
            .query(
                r#"query {
                    viewer {
                        id
                        name
                        email
                    }
                }"#,
            )
            .await?;
        Ok(data.viewer)
    }

    pub async fn get_assigned_issues(&self) -> Result<Vec<LinearIssue>, String> {
        let data: IssuesData = self
            .query(
                r#"query {
                    viewer {
                        assignedIssues(
                            first: 100
                            filter: {
                                state: {
                                    type: { nin: ["canceled"] }
                                }
                            }
                            orderBy: updatedAt
                        ) {
                            nodes {
                                id
                                identifier
                                title
                                description
                                priority
                                branchName
                                state {
                                    name
                                    type
                                }
                                labels {
                                    nodes {
                                        name
                                    }
                                }
                                cycle {
                                    id
                                }
                            }
                        }
                    }
                }"#,
            )
            .await?;
        Ok(data.viewer.assigned_issues.nodes)
    }
}

/// Map Linear state + cycle to Loop's internal status
/// - backlog/unstarted + no cycle = "backlog"
/// - backlog/unstarted + in cycle = "todo"
/// - started = "in_progress"
/// - completed = "done"
pub fn map_linear_state_to_status(issue: &LinearIssue) -> &'static str {
    match issue.state.state_type.as_str() {
        "backlog" | "unstarted" => {
            if issue.cycle.is_some() {
                "todo"
            } else {
                "backlog"
            }
        }
        "started" => "in_progress",
        "completed" => "done",
        _ => "backlog",
    }
}
