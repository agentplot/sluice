//! [`Forge`] over GitHub's REST API, as the app with its installation
//! token: pull requests, their reviews, the checks on their heads and
//! their timelines read; sluice's check run and comments posted. A list
//! is read a hundred at a time until a page comes back short.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use super::app::{App, Repo};
use super::forge::{Check, Forge, Posted, Posting, Pull, Review, Standing, Status, SLUICE_CHECK};

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
        let token = self.token(repo)?;
        let sep = if path.contains('?') { '&' } else { '?' };
        let mut out = Vec::new();
        for page in 1.. {
            let v = self.get(&format!("{path}{sep}per_page={PAGE}&page={page}"), &token)?;
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

fn posting_json(posting: &Posting) -> Value {
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
        "name": SLUICE_CHECK,
        "status": status,
        "output": { "title": posting.title, "summary": summary },
    });
    if let Some(c) = conclusion {
        v["conclusion"] = json!(c);
    }
    v
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
        posting: Posting {
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
        let token = self.token(repo)?;
        match self.get_found(&Self::repo_path(repo, &format!("/pulls/{number}")), &token)? {
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
            .filter(|r| !(r["name"] == SLUICE_CHECK && r["app"]["id"].as_u64() == Some(self.id())))
            .map(|r| Check {
                name: text(&r["name"]),
                standing: run_standing(r),
            })
            .collect();
        let token = self.token(repo)?;
        let combined = self.get(
            &Self::repo_path(repo, &format!("/commits/{rev}/status?per_page={PAGE}")),
            &token,
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

    fn posted(&self, repo: &Repo, rev: &str) -> Result<Option<Posted>> {
        let path = Self::repo_path(
            repo,
            &format!(
                "/commits/{rev}/check-runs?check_name={SLUICE_CHECK}&app_id={}",
                self.id()
            ),
        );
        let runs = self.all(repo, &path, |v| v["check_runs"].as_array())?;
        Ok(runs
            .iter()
            .filter(|r| r["name"] == SLUICE_CHECK && r["app"]["id"].as_u64() == Some(self.id()))
            .find_map(posted))
    }

    fn post(&self, repo: &Repo, rev: &str, existing: Option<u64>, posting: &Posting) -> Result<()> {
        let token = self.token(repo)?;
        let mut body = posting_json(posting);
        match existing {
            Some(id) => {
                self.patch_api(
                    &Self::repo_path(repo, &format!("/check-runs/{id}")),
                    &token,
                    &body.to_string(),
                )?;
            }
            None => {
                body["head_sha"] = json!(rev);
                self.post_api(
                    &Self::repo_path(repo, "/check-runs"),
                    &token,
                    &body.to_string(),
                )?;
            }
        }
        Ok(())
    }

    fn comments(&self, repo: &Repo, number: u64) -> Result<Vec<String>> {
        let path = Self::repo_path(repo, &format!("/issues/{number}/comments"));
        Ok(self
            .all(repo, &path, Value::as_array)?
            .iter()
            .map(|c| text(&c["body"]))
            .collect())
    }

    fn comment(&self, repo: &Repo, number: u64, body: &str) -> Result<()> {
        let token = self.token(repo)?;
        self.post_api(
            &Self::repo_path(repo, &format!("/issues/{number}/comments")),
            &token,
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
    ) -> Result<Pull> {
        let token = self.token(repo)?;
        let v = self.post_api(
            &Self::repo_path(repo, "/pulls"),
            &token,
            &json!({ "title": title, "head": head_ref, "base": base, "body": body, "draft": false })
                .to_string(),
        )?;
        pull(&v)
    }

    fn edit(&self, repo: &Repo, number: u64, body: &str) -> Result<()> {
        let token = self.token(repo)?;
        self.patch_api(
            &Self::repo_path(repo, &format!("/pulls/{number}")),
            &token,
            &json!({ "body": body }).to_string(),
        )?;
        Ok(())
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
            };
            let mut v = posting_json(&p);
            v["id"] = json!(3);
            assert_eq!(posted(&v).unwrap().posting, p);
        }
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
