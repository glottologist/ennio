use async_trait::async_trait;
use ennio_core::error::EnnioError;
use ennio_core::id::ProjectId;
use ennio_core::scm::{
    CICheck, CIStatus, MergeReadiness, PRInfo, PRState, Review, ReviewComment, ReviewDecision, Scm,
};
use reqwest::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use tracing::debug;
use url::Url;

pub struct GitHubScm {
    client: Client,
    token: String,
}

impl GitHubScm {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            token: token.into(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    fn repo_from_project<'a>(&self, project_id: &'a ProjectId) -> &'a str {
        project_id.as_str()
    }

    fn api_url(&self, repo: &str, path: &str) -> Result<Url, EnnioError> {
        let base = format!("https://api.github.com/repos/{repo}{path}");
        Url::parse(&base).map_err(|e| EnnioError::Scm {
            message: format!("invalid GitHub API URL: {e}"),
        })
    }

    async fn github_request(
        &self,
        method: reqwest::Method,
        url: Url,
        body: Option<&serde_json::Value>,
    ) -> Result<Option<serde_json::Value>, EnnioError> {
        let mut req = self
            .client
            .request(method.clone(), url)
            .header(AUTHORIZATION, self.auth_header())
            .header(USER_AGENT, "ennio")
            .header(ACCEPT, "application/vnd.github+json");

        if let Some(b) = body {
            req = req.json(b);
        }

        let resp = req.send().await.map_err(|e| EnnioError::Scm {
            message: format!("GitHub API request failed: {e}"),
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(EnnioError::Scm {
                message: format!("GitHub API {status}: {text}"),
            });
        }

        if method == reqwest::Method::GET {
            let json = resp.json().await.map_err(|e| EnnioError::Scm {
                message: format!("failed to parse GitHub response: {e}"),
            })?;
            Ok(Some(json))
        } else {
            Ok(None)
        }
    }

    async fn github_get(&self, url: Url) -> Result<serde_json::Value, EnnioError> {
        self.github_request(reqwest::Method::GET, url, None)
            .await
            .map(|v| v.expect("GET always returns body"))
    }

    async fn github_put(&self, url: Url, body: &serde_json::Value) -> Result<(), EnnioError> {
        self.github_request(reqwest::Method::PUT, url, Some(body))
            .await?;
        Ok(())
    }

    async fn github_patch(&self, url: Url, body: &serde_json::Value) -> Result<(), EnnioError> {
        self.github_request(reqwest::Method::PATCH, url, Some(body))
            .await?;
        Ok(())
    }
}

fn parse_pr_info(value: &serde_json::Value) -> Result<PRInfo, EnnioError> {
    let number = value["number"].as_i64().ok_or_else(|| EnnioError::Scm {
        message: "missing PR number".to_owned(),
    })?;

    let state_str = value["state"].as_str().unwrap_or("open");
    let merged = value["merged"].as_bool().unwrap_or(false);
    let draft = value["draft"].as_bool().unwrap_or(false);

    let state = if merged {
        PRState::Merged
    } else if draft {
        PRState::Draft
    } else {
        match state_str {
            "closed" => PRState::Closed,
            _ => PRState::Open,
        }
    };

    let number_i32 = i32::try_from(number).map_err(|_| EnnioError::Scm {
        message: format!("PR number {number} exceeds i32"),
    })?;

    Ok(PRInfo {
        number: number_i32,
        title: value["title"].as_str().unwrap_or("").to_owned(),
        url: value["html_url"].as_str().unwrap_or("").to_owned(),
        state,
        branch: value["head"]["ref"].as_str().unwrap_or("").to_owned(),
        base_branch: value["base"]["ref"].as_str().unwrap_or("").to_owned(),
        author: value["user"]["login"].as_str().unwrap_or("").to_owned(),
        draft,
    })
}

#[async_trait]
impl Scm for GitHubScm {
    fn name(&self) -> &str {
        "github"
    }

    async fn detect_pr(
        &self,
        project_id: &ProjectId,
        branch: &str,
    ) -> Result<Option<PRInfo>, EnnioError> {
        let repo = self.repo_from_project(project_id);
        let mut url = self.api_url(repo, "/pulls")?;
        url.query_pairs_mut()
            .append_pair("head", branch)
            .append_pair("state", "open");

        debug!(repo, branch, "detecting PR");

        let data = self.github_get(url).await?;
        let prs = data.as_array().ok_or_else(|| EnnioError::Scm {
            message: "expected array of PRs".to_owned(),
        })?;

        match prs.first() {
            Some(pr) => Ok(Some(parse_pr_info(pr)?)),
            None => Ok(None),
        }
    }

    async fn get_pr_state(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<PRState, EnnioError> {
        let repo = self.repo_from_project(project_id);
        let url = self.api_url(repo, &format!("/pulls/{pr_number}"))?;
        let data = self.github_get(url).await?;
        let info = parse_pr_info(&data)?;
        Ok(info.state)
    }

    async fn merge_pr(&self, project_id: &ProjectId, pr_number: i32) -> Result<(), EnnioError> {
        let repo = self.repo_from_project(project_id);
        let url = self.api_url(repo, &format!("/pulls/{pr_number}/merge"))?;
        let body = serde_json::json!({ "merge_method": "squash" });
        self.github_put(url, &body).await
    }

    async fn close_pr(&self, project_id: &ProjectId, pr_number: i32) -> Result<(), EnnioError> {
        let repo = self.repo_from_project(project_id);
        let url = self.api_url(repo, &format!("/pulls/{pr_number}"))?;
        let body = serde_json::json!({ "state": "closed" });
        self.github_patch(url, &body).await
    }

    async fn get_ci_checks(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<Vec<CICheck>, EnnioError> {
        let repo = self.repo_from_project(project_id);
        let pr_url = self.api_url(repo, &format!("/pulls/{pr_number}"))?;
        let pr_data = self.github_get(pr_url).await?;
        let sha = pr_data["head"]["sha"]
            .as_str()
            .ok_or_else(|| EnnioError::Scm {
                message: "missing head SHA on PR".to_owned(),
            })?;

        let checks_url = self.api_url(repo, &format!("/commits/{sha}/check-runs"))?;
        let data = self.github_get(checks_url).await?;

        let empty = Vec::new();
        let check_runs = data["check_runs"].as_array().unwrap_or(&empty);

        let checks = check_runs
            .iter()
            .map(|cr| {
                let status_str = cr["conclusion"].as_str().unwrap_or("pending");
                let status = match status_str {
                    "success" => CIStatus::Passing,
                    "failure" | "timed_out" => CIStatus::Failing,
                    "cancelled" => CIStatus::Cancelled,
                    "skipped" => CIStatus::Skipped,
                    _ => {
                        if cr["status"].as_str() == Some("in_progress") {
                            CIStatus::Running
                        } else {
                            CIStatus::Pending
                        }
                    }
                };

                CICheck {
                    name: cr["name"].as_str().unwrap_or("").to_owned(),
                    status,
                    url: cr["html_url"].as_str().map(String::from),
                    conclusion: cr["conclusion"].as_str().map(String::from),
                }
            })
            .collect();

        Ok(checks)
    }

    async fn get_ci_summary(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<CIStatus, EnnioError> {
        let checks = self.get_ci_checks(project_id, pr_number).await?;

        if checks.is_empty() {
            return Ok(CIStatus::Pending);
        }

        let any_failing = checks.iter().any(|c| c.status == CIStatus::Failing);
        if any_failing {
            return Ok(CIStatus::Failing);
        }

        let any_running = checks
            .iter()
            .any(|c| c.status == CIStatus::Running || c.status == CIStatus::Pending);
        if any_running {
            return Ok(CIStatus::Running);
        }

        let all_passing = checks
            .iter()
            .all(|c| c.status == CIStatus::Passing || c.status == CIStatus::Skipped);
        if all_passing {
            return Ok(CIStatus::Passing);
        }

        Ok(CIStatus::Pending)
    }

    async fn get_reviews(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<Vec<Review>, EnnioError> {
        let repo = self.repo_from_project(project_id);
        let url = self.api_url(repo, &format!("/pulls/{pr_number}/reviews"))?;
        let data = self.github_get(url).await?;

        let reviews_arr = data.as_array().ok_or_else(|| EnnioError::Scm {
            message: "expected array of reviews".to_owned(),
        })?;

        let reviews = reviews_arr
            .iter()
            .map(|r| {
                let state_str = r["state"].as_str().unwrap_or("PENDING");
                let state = match state_str {
                    "APPROVED" => ReviewDecision::Approved,
                    "CHANGES_REQUESTED" => ReviewDecision::ChangesRequested,
                    "DISMISSED" => ReviewDecision::Dismissed,
                    _ => ReviewDecision::Pending,
                };

                let submitted_at = r["submitted_at"]
                    .as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc));

                Review {
                    author: r["user"]["login"].as_str().unwrap_or("").to_owned(),
                    state,
                    body: r["body"].as_str().map(String::from),
                    submitted_at,
                }
            })
            .collect();

        Ok(reviews)
    }

    async fn get_review_decision(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<ReviewDecision, EnnioError> {
        let reviews = self.get_reviews(project_id, pr_number).await?;

        let mut latest_per_author: std::collections::HashMap<&str, &ReviewDecision> =
            std::collections::HashMap::new();

        for review in &reviews {
            latest_per_author.insert(&review.author, &review.state);
        }

        let any_changes = latest_per_author
            .values()
            .any(|d| **d == ReviewDecision::ChangesRequested);
        if any_changes {
            return Ok(ReviewDecision::ChangesRequested);
        }

        let any_approved = latest_per_author
            .values()
            .any(|d| **d == ReviewDecision::Approved);
        if any_approved {
            return Ok(ReviewDecision::Approved);
        }

        Ok(ReviewDecision::Pending)
    }

    async fn get_pending_comments(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<Vec<ReviewComment>, EnnioError> {
        let repo = self.repo_from_project(project_id);
        let url = self.api_url(repo, &format!("/pulls/{pr_number}/comments"))?;
        let data = self.github_get(url).await?;

        let comments_arr = data.as_array().ok_or_else(|| EnnioError::Scm {
            message: "expected array of comments".to_owned(),
        })?;

        let comments = comments_arr
            .iter()
            .map(|c| {
                let line = c["line"].as_u64().and_then(|l| u32::try_from(l).ok());

                ReviewComment {
                    author: c["user"]["login"].as_str().unwrap_or("").to_owned(),
                    body: c["body"].as_str().unwrap_or("").to_owned(),
                    path: c["path"].as_str().map(String::from),
                    line,
                }
            })
            .collect();

        Ok(comments)
    }

    async fn get_mergeability(
        &self,
        project_id: &ProjectId,
        pr_number: i32,
    ) -> Result<MergeReadiness, EnnioError> {
        let repo = self.repo_from_project(project_id);
        let url = self.api_url(repo, &format!("/pulls/{pr_number}"))?;
        let data = self.github_get(url).await?;

        let mergeable = data["mergeable"].as_bool().unwrap_or(false);
        let no_conflicts = data["mergeable_state"].as_str() != Some("dirty");

        let ci_status = self.get_ci_summary(project_id, pr_number).await?;
        let ci_passing = ci_status == CIStatus::Passing;

        let review_decision = self.get_review_decision(project_id, pr_number).await?;
        let approved = review_decision == ReviewDecision::Approved;

        let mut reasons = Vec::new();
        if !ci_passing {
            reasons.push("CI not passing".to_owned());
        }
        if !approved {
            reasons.push("not approved".to_owned());
        }
        if !no_conflicts {
            reasons.push("merge conflicts".to_owned());
        }

        Ok(MergeReadiness {
            mergeable: mergeable && ci_passing && approved && no_conflicts,
            ci_passing,
            approved,
            no_conflicts,
            reasons,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ennio_core::scm::PRState;
    use proptest::prelude::*;

    fn arb_pr_json() -> impl Strategy<Value = serde_json::Value> {
        (
            1i64..1_000_000,
            ".*",
            prop::bool::ANY,
            prop::bool::ANY,
            prop::sample::select(vec!["open", "closed"]),
            "[a-zA-Z0-9_/-]+",
            "[a-zA-Z0-9_/-]+",
            "[a-zA-Z0-9_-]+",
        )
            .prop_map(
                |(number, title, merged, draft, state, branch, base, author)| {
                    serde_json::json!({
                        "number": number,
                        "title": title,
                        "merged": merged,
                        "draft": draft,
                        "state": state,
                        "html_url": format!("https://github.com/test/repo/pull/{number}"),
                        "head": {"ref": branch},
                        "base": {"ref": base},
                        "user": {"login": author}
                    })
                },
            )
    }

    proptest! {
        #[test]
        fn parse_pr_info_never_panics(json in arb_pr_json()) {
            let result = parse_pr_info(&json);
            prop_assert!(result.is_ok());
            let info = result.unwrap();
            prop_assert!(info.number > 0);
        }

        #[test]
        fn parse_pr_info_merged_takes_precedence(
            number in 1i64..100_000,
            draft in prop::bool::ANY,
            state in prop::sample::select(vec!["open", "closed"]),
        ) {
            let json = serde_json::json!({
                "number": number,
                "title": "test",
                "merged": true,
                "draft": draft,
                "state": state,
                "html_url": "",
                "head": {"ref": "b"},
                "base": {"ref": "main"},
                "user": {"login": "u"}
            });
            let info = parse_pr_info(&json).unwrap();
            prop_assert_eq!(info.state, PRState::Merged);
        }

        #[test]
        fn parse_pr_info_draft_when_not_merged(
            number in 1i64..100_000,
            state in prop::sample::select(vec!["open", "closed"]),
        ) {
            let json = serde_json::json!({
                "number": number,
                "title": "test",
                "merged": false,
                "draft": true,
                "state": state,
                "html_url": "",
                "head": {"ref": "b"},
                "base": {"ref": "main"},
                "user": {"login": "u"}
            });
            let info = parse_pr_info(&json).unwrap();
            prop_assert_eq!(info.state, PRState::Draft);
        }

        #[test]
        fn parse_pr_info_missing_number_errors(title in ".*") {
            let json = serde_json::json!({
                "title": title,
                "merged": false,
                "draft": false,
                "state": "open",
                "html_url": "",
                "head": {"ref": "b"},
                "base": {"ref": "main"},
                "user": {"login": "u"}
            });
            prop_assert!(parse_pr_info(&json).is_err());
        }
    }
}
