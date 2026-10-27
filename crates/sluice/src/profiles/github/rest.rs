//! [`Forge`] over GitHub's REST API, as the app with its installation
//! token: pull requests, their reviews, the checks on their heads and
//! their timelines read; sluice's check run and comments posted. A list
//! is read a hundred at a time until a page comes back short. A pull
//! request is marked a draft or ready for review through GraphQL, the one
//! API that can.

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use super::app::{App, Bearer, Call, Repo};
use super::forge::{
    Check, Comment, Forge, Posted, Posting, Pull, Review, Standing, Status, SLUICE_CHECK,
};

/// The most a page of a list holds.
const PAGE: usize = 100;

impl App {
    fn repo_path(repo: &Repo, rest: &str) -> String {
        format!("/repos/{}/{}{rest}", repo.owner, repo.name)
    }

    /// Every item of a list, page by page; `items` picks the list out of
    /// a page's JSON.
    fn all(
        &self,
        repo: &Repo,
        path: &str,
        items: fn(&Value) -> Option<&Vec<Value>>,
    ) -> Result<Vec<Value>> {
        let sep = if path.contains('?') { '&' } else { '?' };
        let mut out = Vec::new();
        for page in 1.. {
            let v = self.get(
                &format!("{path}{sep}per_page={PAGE}&page={page}"),
                Bearer::Repo(repo),
            )?;
            let got = items(&v).cloned().unwrap_or_default();
            let n = got.len();
            out.extend(got);
            if n < PAGE {
                break;
            }
        }
        Ok(out)
    }
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn pull(v: &Value) -> Result<Pull> {
    Ok(Pull {
        number: v["number"]
            .as_u64()
            .ok_or_else(|| anyhow!("a pull request with no number"))?,
        node_id: text(&v["node_id"]),
        title: text(&v["title"]),
        body: text(&v["body"]),
        author: text(&v["user"]["login"]),
        draft: v["draft"].as_bool().unwrap_or(false),
        open: v["state"] == "open",
        head: text(&v["head"]["sha"]),
        head_ref: text(&v["head"]["ref"]),
        head_repo: text(&v["head"]["repo"]["full_name"]),
        base: text(&v["base"]["ref"]),
    })
}

/// A check run's standing from its status and conclusion.
fn run_standing(v: &Value) -> Standing {
    if v["status"] != "completed" {
        return Standing::Pending;
    }
    match v["conclusion"].as_str() {
        Some("success" | "neutral" | "skipped") => Standing::Passed,
        _ => Standing::Failed,
    }
}

/// A commit status's standing from its state.
fn status_standing(state: &str) -> Standing {
    match state {
        "success" => Standing::Passed,
        "pending" => Standing::Pending,
        _ => Standing::Failed,
    }
}

fn posting_json(name: &str, posting: &Posting) -> Value {
    let (status, conclusion) = match posting.status {
        Status::Queued => ("queued", None),
        Status::InProgress => ("in_progress", None),
        Status::Failure => ("completed", Some("failure")),
        Status::Success => ("completed", Some("success")),
    };
    // GitHub takes no output without a summary.
    let summary = if posting.summary.is_empty() {
        &posting.title
    } else {
        &posting.summary
    };
    let mut v = json!({
        "name": name,
        "status": status,
        "output": { "title": posting.title, "summary": summary },
    });
    if let Some(text) = &posting.text {
        v["output"]["text"] = json!(text);
    }
    if let Some(c) = conclusion {
        v["conclusion"] = json!(c);
    }
    v
}

/// The body of the REST request that opens a pull request, a draft or
/// ready for review.
fn open_request(base: &str, head_ref: &str, title: &str, body: &str, draft: bool) -> Value {
    json!({ "title": title, "head": head_ref, "base": base, "body": body, "draft": draft })
}

/// The GraphQL request that marks the pull request `node_id` a draft, or
/// ready for review.
fn draft_request(node_id: &str, draft: bool) -> Value {
    let mutation = if draft {
        "convertPullRequestToDraft"
    } else {
        "markPullRequestReadyForReview"
    };
    json!({
        "query": format!(
            "mutation($id: ID!) {{ {mutation}(input: {{pullRequestId: $id}}) \
             {{ pullRequest {{ isDraft }} }} }}"
        ),
        "variables": { "id": node_id },
    })
}

/// A GraphQL reply, an error when it carries any: GraphQL answers 200 to
/// a request it refuses.
fn graphql_reply(v: &Value) -> Result<()> {
    match v["errors"].as_array() {
        Some(errors) if !errors.is_empty() => {
            let why: Vec<&str> = errors
                .iter()
                .map(|e| e["message"].as_str().unwrap_or("an error with no message"))
                .collect();
            bail!("POST /graphql: {}", why.join("; "))
        }
        _ => Ok(()),
    }
}

fn posted(v: &Value) -> Option<Posted> {
    let status = match (v["status"].as_str(), v["conclusion"].as_str()) {
        (Some("completed"), Some("success")) => Status::Success,
        (Some("completed"), _) => Status::Failure,
        (Some("in_progress"), _) => Status::InProgress,
        _ => Status::Queued,
    };
    let title = text(&v["output"]["title"]);
    let summary = text(&v["output"]["summary"]);
    Some(Posted {
        id: v["id"].as_u64()?,
        name: text(&v["name"]),
        posting: Posting {
            text: v["output"]["text"].as_str().map(String::from),
            status,
            // Posted with the title for an empty summary: read back as
            // empty, so it reads the same as what is posted.
            summary: if summary == title {
                String::new()
            } else {
                summary
            },
            title,
        },
    })
}

impl Forge for App {
    fn pulls(&self, repo: &Repo, base: &str) -> Result<Vec<Pull>> {
        let path = Self::repo_path(repo, &format!("/pulls?state=open&base={base}"));
        self.all(repo, &path, Value::as_array)?
            .iter()
            .map(pull)
            .collect()
    }

    fn pull(&self, repo: &Repo, number: u64) -> Result<Option<Pull>> {
        match self.get_found(
            &Self::repo_path(repo, &format!("/pulls/{number}")),
            Bearer::Repo(repo),
        )? {
            None => Ok(None),
            Some(v) => Ok(Some(pull(&v)?)),
        }
    }

    fn reviews(&self, repo: &Repo, number: u64) -> Result<Vec<Review>> {
        let path = Self::repo_path(repo, &format!("/pulls/{number}/reviews"));
        Ok(self
            .all(repo, &path, Value::as_array)?
            .iter()
            .map(|r| Review {
                user: text(&r["user"]["login"]),
                state: text(&r["state"]),
                commit: text(&r["commit_id"]),
            })
            .collect())
    }

    fn checks(&self, repo: &Repo, rev: &str) -> Result<Vec<Check>> {
        let runs = self.all(
            repo,
            &Self::repo_path(repo, &format!("/commits/{rev}/check-runs")),
            |v| v["check_runs"].as_array(),
        )?;
        let mut out: Vec<Check> = runs
            .iter()
            .filter(|r| r["app"]["id"].as_u64() != Some(self.id()))
            .map(|r| Check {
                name: text(&r["name"]),
                standing: run_standing(r),
            })
            .collect();
        let combined = self.get(
            &Self::repo_path(repo, &format!("/commits/{rev}/status?per_page={PAGE}")),
            Bearer::Repo(repo),
        )?;
        for s in combined["statuses"].as_array().into_iter().flatten() {
            out.push(Check {
                name: text(&s["context"]),
                standing: status_standing(s["state"].as_str().unwrap_or_default()),
            });
        }
        Ok(out)
    }

    fn readied(&self, repo: &Repo, number: u64) -> Result<Vec<DateTime<Utc>>> {
        let path = Self::repo_path(repo, &format!("/issues/{number}/timeline"));
        Ok(self
            .all(repo, &path, Value::as_array)?
            .iter()
            .filter(|e| e["event"] == "ready_for_review")
            .filter_map(|e| DateTime::parse_from_rfc3339(e["created_at"].as_str()?).ok())
            .map(|t| t.with_timezone(&Utc))
            .collect())
    }

    fn posted_all(&self, repo: &Repo, rev: &str) -> Result<Vec<Posted>> {
        let path = Self::repo_path(
            repo,
            &format!("/commits/{rev}/check-runs?app_id={}", self.id()),
        );
        let runs = self.all(repo, &path, |v| v["check_runs"].as_array())?;
        Ok(runs
            .iter()
            .filter(|r| r["app"]["id"].as_u64() == Some(self.id()))
            .filter(|r| {
                let n = r["name"].as_str().unwrap_or_default();
                n == SLUICE_CHECK || n.starts_with(&format!("{SLUICE_CHECK} / "))
            })
            .filter_map(posted)
            .collect())
    }

    fn post_named(
        &self,
        repo: &Repo,
        rev: &str,
        name: &str,
        existing: Option<u64>,
        posting: &Posting,
    ) -> Result<()> {
        let mut body = posting_json(name, posting);
        match existing {
            Some(id) => {
                self.patch_api(
                    &Self::repo_path(repo, &format!("/check-runs/{id}")),
                    Bearer::Repo(repo),
                    &body.to_string(),
                )?;
            }
            None => {
                body["head_sha"] = json!(rev);
                self.post_api(
                    Call::Make,
                    &Self::repo_path(repo, "/check-runs"),
                    Bearer::Repo(repo),
                    &body.to_string(),
                )?;
            }
        }
        Ok(())
    }

    fn comment_list(&self, repo: &Repo, number: u64) -> Result<Vec<Comment>> {
        let path = Self::repo_path(repo, &format!("/issues/{number}/comments"));
        Ok(self
            .all(repo, &path, Value::as_array)?
            .iter()
            .map(|c| Comment {
                id: c["id"].as_u64().unwrap_or_default(),
                body: text(&c["body"]),
                url: text(&c["html_url"]),
            })
            .collect())
    }

    fn edit_comment(&self, repo: &Repo, id: u64, body: &str) -> Result<()> {
        self.patch_api(
            &Self::repo_path(repo, &format!("/issues/comments/{id}")),
            Bearer::Repo(repo),
            &json!({ "body": body }).to_string(),
        )?;
        Ok(())
    }

    fn comment(&self, repo: &Repo, number: u64, body: &str) -> Result<()> {
        self.post_api(
            Call::Make,
            &Self::repo_path(repo, &format!("/issues/{number}/comments")),
            Bearer::Repo(repo),
            &json!({ "body": body }).to_string(),
        )?;
        Ok(())
    }

    fn open(
        &self,
        repo: &Repo,
        base: &str,
        head_ref: &str,
        title: &str,
        body: &str,
        draft: bool,
    ) -> Result<Pull> {
        let v = self.post_api(
            Call::Make,
            &Self::repo_path(repo, "/pulls"),
            Bearer::Repo(repo),
            &open_request(base, head_ref, title, body, draft).to_string(),
        )?;
        pull(&v)
    }

    fn edit(&self, repo: &Repo, number: u64, body: &str) -> Result<()> {
        self.patch_api(
            &Self::repo_path(repo, &format!("/pulls/{number}")),
            Bearer::Repo(repo),
            &json!({ "body": body }).to_string(),
        )?;
        Ok(())
    }

    fn set_draft(&self, repo: &Repo, number: u64, draft: bool) -> Result<()> {
        let pull = self
            .pull(repo, number)?
            .ok_or_else(|| anyhow!("{repo} has no pull request {number}"))?;
        if pull.node_id.is_empty() {
            bail!("GitHub gave {repo}'s pull request {number} no node id");
        }
        // Marking a draft or ready sets a state.
        let v = self.post_api(
            Call::Set,
            "/graphql",
            Bearer::Repo(repo),
            &draft_request(&pull.node_id, draft).to_string(),
        )?;
        graphql_reply(&v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What sluice posts reads back the same, so a round that would post
    /// it again posts nothing.
    #[test]
    fn a_posting_reads_back_as_posted() {
        for (status, summary) in [
            (Status::Queued, "needs 1 review"),
            (Status::InProgress, ""),
            (Status::Failure, "failed: #gate"),
            (Status::Success, ""),
        ] {
            let p = Posting {
                status,
                title: "t".into(),
                summary: summary.into(),
                text: (status == Status::Failure).then_some("| a | b |".into()),
            };
            let mut v = posting_json(SLUICE_CHECK, &p);
            v["id"] = json!(3);
            assert_eq!(posted(&v).unwrap().posting, p);
        }
    }

    /// A draft and a ready mark are one GraphQL mutation each, keyed by
    /// the pull request's node id; an `errors` array is an error, though
    /// GraphQL answers it with 200.
    #[test]
    fn marking_a_draft_or_ready() {
        let draft = draft_request("PR_kw1", true);
        let q = draft["query"].as_str().unwrap();
        assert!(
            q.contains("convertPullRequestToDraft(input: {pullRequestId: $id})"),
            "{q}"
        );
        assert!(q.starts_with("mutation($id: ID!)"), "{q}");
        assert_eq!(draft["variables"]["id"], "PR_kw1");
        let ready = draft_request("PR_kw1", false);
        let q = ready["query"].as_str().unwrap();
        assert!(
            q.contains("markPullRequestReadyForReview(input: {pullRequestId: $id})"),
            "{q}"
        );
        assert!(!q.contains("convertPullRequestToDraft"), "{q}");

        let ok = json!({ "data": { "markPullRequestReadyForReview": { "pullRequest": { "isDraft": false } } } });
        assert!(graphql_reply(&ok).is_ok());
        let refused = json!({ "data": null, "errors": [
            { "message": "Resource not accessible by integration" },
            { "type": "FORBIDDEN" },
        ] });
        let e = graphql_reply(&refused).unwrap_err().to_string();
        assert!(
            e.contains("Resource not accessible by integration")
                && e.contains("an error with no message"),
            "{e}"
        );
        assert!(graphql_reply(&json!({ "errors": [] })).is_ok());
    }

    /// A pull request is opened a draft only when asked.
    #[test]
    fn opening_a_draft_or_ready() {
        let draft = open_request("main", "unit/a", "A", "body", true);
        assert_eq!(draft["draft"], true);
        assert_eq!(
            (
                &draft["base"],
                &draft["head"],
                &draft["title"],
                &draft["body"]
            ),
            (
                &json!("main"),
                &json!("unit/a"),
                &json!("A"),
                &json!("body")
            )
        );
        assert_eq!(
            open_request("main", "unit/a", "A", "", false)["draft"],
            false
        );
    }

    #[test]
    fn standings() {
        assert_eq!(
            run_standing(&json!({"status": "in_progress"})),
            Standing::Pending
        );
        assert_eq!(
            run_standing(&json!({"status": "completed", "conclusion": "neutral"})),
            Standing::Passed
        );
        assert_eq!(
            run_standing(&json!({"status": "completed", "conclusion": "cancelled"})),
            Standing::Failed
        );
        assert_eq!(status_standing("error"), Standing::Failed);
    }
}
