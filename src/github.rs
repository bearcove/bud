use eyre::Result;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub labels: Vec<Label>,
    pub comments: Vec<Comment>,
    pub state: String,
    pub assignees: Vec<Assignee>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Label {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Assignee {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Comment {
    pub author: Option<User>,
    pub body: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

pub fn infer_repo() -> Result<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(eyre::eyre!(
            "failed to infer repo from git remote origin: {stderr}"
        ));
    }

    let raw = String::from_utf8(output.stdout)?.trim().to_string();
    parse_repo_from_remote(&raw)
}

fn parse_repo_from_remote(remote: &str) -> Result<String> {
    let path = if let Some((_, rest)) = remote.split_once("github.com:") {
        rest
    } else if let Some((_, rest)) = remote.split_once("github.com/") {
        rest
    } else {
        return Err(eyre::eyre!(
            "unsupported remote URL format for GitHub: {remote}"
        ));
    };

    let cleaned = path.trim().trim_start_matches('/').trim_end_matches(".git");
    let mut parts = cleaned.split('/');
    let owner = parts.next().unwrap_or_default();
    let repo = parts.next().unwrap_or_default();
    if owner.is_empty() || repo.is_empty() || parts.next().is_some() {
        return Err(eyre::eyre!(
            "could not parse owner/repo from remote URL: {remote}"
        ));
    }
    Ok(format!("{owner}/{repo}"))
}

pub fn sync_issues(repo: &str) -> Result<Vec<Issue>> {
    let output = Command::new("gh")
        .args([
            "issue",
            "list",
            "-R",
            repo,
            "--json",
            "number,title,body,labels,comments,state,assignees,createdAt,updatedAt",
            "--limit",
            "100",
        ])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(eyre::eyre!("gh issue list failed: {stderr}"));
    }
    let issues: Vec<Issue> = serde_json::from_slice(&output.stdout)?;
    Ok(issues)
}

pub fn write_issue_files(repo: &str, issues: &[Issue]) -> Result<(PathBuf, usize)> {
    let dir_name = repo.replace('/', "-");
    let dir = PathBuf::from("/tmp/bud-issues").join(dir_name);
    std::fs::create_dir_all(&dir)?;

    for issue in issues {
        let labels = if issue.labels.is_empty() {
            "none".to_string()
        } else {
            issue.labels
                .iter()
                .map(|l| l.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let assignees = if issue.assignees.is_empty() {
            "none".to_string()
        } else {
            issue.assignees
                .iter()
                .map(|a| a.login.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let body = issue.body.as_deref().unwrap_or("").trim();
        let body = if body.is_empty() {
            "(no description)"
        } else {
            body
        };

        let mut content = String::new();
        content.push_str(&format!("# #{}: {}\n\n", issue.number, issue.title));
        content.push_str(&format!("**State:** {}\n", issue.state));
        content.push_str(&format!("**Labels:** {labels}\n"));
        content.push_str(&format!("**Created:** {}\n", short_date(&issue.created_at)));
        content.push_str(&format!("**Updated:** {}\n", short_date(&issue.updated_at)));
        content.push_str(&format!("**Assignees:** {assignees}\n\n"));
        content.push_str("---\n\n");
        content.push_str(body);
        content.push_str("\n\n---\n\n## Comments\n\n");
        if issue.comments.is_empty() {
            content.push_str("(no comments)\n");
        } else {
            for comment in &issue.comments {
                let author = comment
                    .author
                    .as_ref()
                    .map(|u| u.login.as_str())
                    .unwrap_or("unknown");
                let comment_body = comment.body.as_deref().unwrap_or("").trim();
                let comment_body = if comment_body.is_empty() {
                    "(no comment body)"
                } else {
                    comment_body
                };
                content.push_str(&format!(
                    "### @{author} ({}):\n{comment_body}\n\n",
                    short_date(&comment.created_at)
                ));
            }
        }

        std::fs::write(dir.join(format!("{}.md", issue.number)), content)?;
    }

    Ok((dir, issues.len()))
}

fn short_date(value: &str) -> &str {
    value.get(..10).unwrap_or(value)
}
