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
    pub project: Option<ProjectRef>,
    pub assignee: Option<AssigneeRef>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRef {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssigneeRef {
    pub name: String,
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
    #[serde(rename = "startsAt")]
    pub starts_at: Option<String>,
    #[serde(rename = "endsAt")]
    pub ends_at: Option<String>,
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

    pub async fn get_viewer_team_id(&self) -> Result<String, String> {
        #[derive(serde::Deserialize)]
        struct TeamsData {
            viewer: ViewerTeams,
        }
        #[derive(serde::Deserialize)]
        struct ViewerTeams {
            #[serde(rename = "teamMemberships")]
            team_memberships: TeamMembershipConnection,
        }
        #[derive(serde::Deserialize)]
        struct TeamMembershipConnection {
            nodes: Vec<TeamMembership>,
        }
        #[derive(serde::Deserialize)]
        struct TeamMembership {
            team: TeamRef,
        }
        #[derive(serde::Deserialize)]
        struct TeamRef {
            id: String,
        }

        let data: TeamsData = self
            .query(
                r#"query {
                    viewer {
                        teamMemberships(first: 1) {
                            nodes {
                                team { id }
                            }
                        }
                    }
                }"#,
            )
            .await?;

        data.viewer
            .team_memberships
            .nodes
            .into_iter()
            .next()
            .map(|m| m.team.id)
            .ok_or_else(|| "No team found for current user".to_string())
    }

    pub async fn get_viewer_id(&self) -> Result<String, String> {
        let viewer = self.get_viewer().await?;
        Ok(viewer.id)
    }

    pub async fn create_issue(
        &self,
        team_id: &str,
        title: &str,
        description: &str,
        priority: i64,
        assignee_id: &str,
    ) -> Result<LinearIssue, String> {
        let escaped_title = title.replace('\\', "\\\\").replace('"', "\\\"");
        let escaped_desc = description.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");

        let query = format!(
            r#"mutation {{
                issueCreate(input: {{
                    teamId: "{team_id}",
                    title: "{escaped_title}",
                    description: "{escaped_desc}",
                    priority: {priority},
                    assigneeId: "{assignee_id}"
                }}) {{
                    success
                    issue {{
                        id
                        identifier
                        title
                        description
                        priority
                        branchName
                        state {{ name type }}
                        labels {{ nodes {{ name }} }}
                        cycle {{ id }}
                        createdAt
                        updatedAt
                    }}
                }}
            }}"#
        );

        #[derive(serde::Deserialize)]
        struct CreateData {
            #[serde(rename = "issueCreate")]
            issue_create: IssueCreatePayload,
        }
        #[derive(serde::Deserialize)]
        struct IssueCreatePayload {
            success: bool,
            issue: LinearIssue,
        }

        let data: CreateData = self.query(&query).await?;
        if !data.issue_create.success {
            return Err("Linear issueCreate returned success=false".to_string());
        }
        Ok(data.issue_create.issue)
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
                                    startsAt
                                    endsAt
                                }
                                project {
                                    name
                                }
                                assignee {
                                    name
                                }
                                createdAt
                                updatedAt
                            }
                        }
                    }
                }"#,
            )
            .await?;
        Ok(data.viewer.assigned_issues.nodes)
    }
}

/// Map Linear state + cycle to Herd's internal status.
/// Uses both `state_type` (Linear's category) and `state.name` (the user's
/// custom workflow state name) to distinguish columns like "In Review".
fn is_current_cycle(cycle: &CycleRef) -> bool {
    let now = chrono::Utc::now().to_rfc3339();
    let started = cycle.starts_at.as_deref().map(|s| s <= now.as_str()).unwrap_or(false);
    let not_ended = cycle.ends_at.as_deref().map(|e| e >= now.as_str()).unwrap_or(true);
    started && not_ended
}

pub fn map_linear_state_to_status(issue: &LinearIssue) -> &'static str {
    let name = issue.state.name.to_lowercase();

    match issue.state.state_type.as_str() {
        "backlog" | "unstarted" => {
            if issue.cycle.as_ref().map(|c| is_current_cycle(c)).unwrap_or(false) {
                "todo"
            } else {
                "backlog"
            }
        }
        "started" => {
            if name.contains("waiting") || (name.contains("review") && !name.contains("human") && !name.contains("input") && !name.contains("feedback")) {
                "waiting_for_review"
            } else if name.contains("ready to merge") || name.contains("approved") || name.contains("merge") {
                "ready_to_merge"
            } else if name.contains("human") || name.contains("input") || name.contains("feedback") || name.contains("attention") || name.contains("blocked") {
                "human_input"
            } else {
                "in_progress"
            }
        }
        "completed" => "done",
        _ => "backlog",
    }
}
