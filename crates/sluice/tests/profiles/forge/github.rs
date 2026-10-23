//! A fake of GitHub's API for one repository, `o/r`, on a port the system
//! picks: the app's installation and tokens, and the pull request calls
//! the app makes, answered from a [`FileForge`]. Given the bare
//! repository that stands for GitHub's git, it keeps each open pull
//! request's head at its branch there and writes `refs/pull/<n>/head`, as
//! GitHub does. It marks a pull request a draft or ready for review
//! through GraphQL, as GitHub alone does, unless told to refuse. Each
//! test crate that includes it uses a part.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use sluice::profiles::github::fake::FileForge;
use sluice::profiles::github::forge::{Forge, Posting, Pull, Standing, Status};
use sluice::profiles::github::Repo;

use crate::support::http::{json, Answer, Fake, Request};

pub const APP: u64 = 4242;
pub const INSTALLATION: u64 = 7;
/// The pull request calls' prefix.
const REPO: &str = "/repos/o/r";

pub fn repo() -> Repo {
    Repo::from_url("https://github.com/o/r.git").unwrap()
}

pub struct GitHub {
    pub forge: Arc<FileForge>,
    pub fake: Fake,
    /// While set, GraphQL refuses to mark a pull request a draft or ready.
    refuse: Arc<AtomicBool>,
}

impl GitHub {
    /// The fake API over the fake GitHub kept in `file`; `bare` is the
    /// repository GitHub's git is, if any.
    pub fn start(file: &Path, bare: Option<&Path>) -> GitHub {
        let forge = Arc::new(FileForge::new(file, &repo()));
        let held = forge.clone();
        let bare = bare.map(Path::to_path_buf);
        let refuse = Arc::new(AtomicBool::new(false));
        let refusing = refuse.clone();
        let fake = Fake::start(move |r| {
            if let Some(b) = &bare {
                sync(&held, b);
            }
            if r.method == "POST" && r.path == "/graphql" {
                return graphql(&held, refusing.load(Ordering::SeqCst), r);
            }
            answer(&held, bare.as_deref(), r)
        });
        GitHub {
            forge,
            fake,
            refuse,
        }
    }

    pub fn url(&self) -> &str {
        self.fake.url()
    }

    /// Refuse, or take again, a request to mark a pull request a draft or
    /// ready for review.
    pub fn refuse_drafts(&self, refuse: bool) {
        self.refuse.store(refuse, Ordering::SeqCst);
    }
}

/// GitHub's GraphQL API, for the two mutations sluice makes: a pull
/// request, by its node id, marked a draft or ready for review. GraphQL
/// answers a request it refuses with 200 and `errors`.
fn graphql(forge: &FileForge, refuse: bool, r: &Request) -> Answer {
    let body: Value = serde_json::from_str(&r.body).unwrap_or(Value::Null);
    let query = body["query"].as_str().unwrap_or_default();
    let error = |kind: &str, message: &str| {
        json(
            200,
            json!({ "data": null, "errors": [{ "type": kind, "message": message }] }),
        )
    };
    let (mutation, draft) = if query.contains("convertPullRequestToDraft") {
        ("convertPullRequestToDraft", true)
    } else if query.contains("markPullRequestReadyForReview") {
        ("markPullRequestReadyForReview", false)
    } else {
        return error("UNKNOWN", &format!("the fake answers no {query}"));
    };
    if refuse {
        return error("FORBIDDEN", "Resource not accessible by integration");
    }
    let id = body["variables"]["id"].as_str().unwrap_or_default();
    let Some(n) = forge
        .held()
        .unwrap()
        .pulls
        .iter()
        .find(|p| p.node_id == id)
        .map(|p| p.number)
    else {
        return error(
            "NOT_FOUND",
            &format!("Could not resolve to a node with the global id of '{id}'"),
        );
    };
    forge.set_draft(n, draft).unwrap();
    json(
        200,
        json!({ "data": { mutation: { "pullRequest": { "isDraft": draft } } } }),
    )
}

/// GitHub's pull requests follow their branches.
fn sync(forge: &FileForge, bare: &Path) {
    let Ok(held) = forge.held() else { return };
    for p in held.pulls.iter().filter(|p| p.open && p.from_repo(&repo())) {
        let Some(head) = rev(bare, &format!("refs/heads/{}", p.head_ref)) else {
            continue;
        };
        if head != p.head {
            forge.push(p.number, &head).unwrap();
        }
        set_ref(bare, &format!("refs/pull/{}/head", p.number), &head);
    }
    // A pull request whose head is in its branch is merged.
    for p in held.pulls.iter().filter(|p| p.open && !p.head.is_empty()) {
        let into = format!("refs/heads/{}", p.base);
        let merged = crate::support::git_command()
            .arg("--git-dir")
            .arg(bare)
            .args(["merge-base", "--is-ancestor", &p.head, &into])
            .output()
            .unwrap()
            .status
            .success();
        if merged {
            forge.close(p.number).unwrap();
        }
    }
}

pub fn rev(bare: &Path, name: &str) -> Option<String> {
    let out = crate::support::git_command()
        .arg("--git-dir")
        .arg(bare)
        .args(["rev-parse", "--verify", "-q", name])
        .output()
        .unwrap();
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn set_ref(bare: &Path, name: &str, to: &str) {
    let out = crate::support::git_command()
        .arg("--git-dir")
        .arg(bare)
        .args(["update-ref", name, to])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn pull_json(p: &Pull) -> Value {
    json!({
        "number": p.number,
        "node_id": p.node_id,
        "title": p.title,
        "body": p.body,
        "user": { "login": p.author },
        "draft": p.draft,
        "state": if p.open { "open" } else { "closed" },
        "head": { "sha": p.head, "ref": p.head_ref, "repo": { "full_name": p.head_repo } },
        "base": { "ref": p.base },
    })
}

fn posting(body: &Value) -> Posting {
    let status = match (body["status"].as_str(), body["conclusion"].as_str()) {
        (Some("completed"), Some("success")) => Status::Success,
        (Some("completed"), _) => Status::Failure,
        (Some("in_progress"), _) => Status::InProgress,
        _ => Status::Queued,
    };
    let title = body["output"]["title"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let summary = body["output"]["summary"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    Posting {
        status,
        summary: if summary == title {
            String::new()
        } else {
            summary
        },
        title,
        text: body["output"]["text"].as_str().map(String::from),
    }
}

fn run_json(id: u64, name: &str, p: &Posting) -> Value {
    let (status, conclusion) = match p.status {
        Status::Queued => ("queued", Value::Null),
        Status::InProgress => ("in_progress", Value::Null),
        Status::Failure => ("completed", json!("failure")),
        Status::Success => ("completed", json!("success")),
    };
    let summary = if p.summary.is_empty() {
        &p.title
    } else {
        &p.summary
    };
    json!({
        "id": id, "name": name, "status": status, "conclusion": conclusion,
        "output": { "title": p.title, "summary": summary, "text": p.text }, "app": { "id": APP },
    })
}

fn not_found() -> Answer {
    json(404, json!({ "message": "Not Found" }))
}

fn answer(forge: &FileForge, bare: Option<&Path>, r: &Request) -> Answer {
    let (path, query) = r.path.split_once('?').unwrap_or((&r.path, ""));
    let param = |k: &str| {
        query
            .split('&')
            .find_map(|kv| kv.strip_prefix(&format!("{k}=")))
            .map(str::to_string)
    };
    // Every list fits on its first page.
    if param("page").is_some_and(|p| p != "1") {
        return if path.ends_with("/check-runs") {
            json(200, json!({ "check_runs": [] }))
        } else {
            json(200, json!([]))
        };
    }
    let body: Value = serde_json::from_str(&r.body).unwrap_or(Value::Null);
    let m = r.method.as_str();
    if path == format!("{REPO}/installation") {
        return json(
            200,
            json!({ "id": INSTALLATION, "app_id": APP,
                    "permissions": { "contents": "write", "checks": "write", "pull_requests": "write" } }),
        );
    }
    if path == format!("/app/installations/{INSTALLATION}/access_tokens") {
        let ends = chrono::Utc::now() + chrono::TimeDelta::hours(1);
        return json(
            201,
            json!({ "token": "token", "expires_at": ends.to_rfc3339() }),
        );
    }
    let Some(rest) = path.strip_prefix(REPO) else {
        return not_found();
    };
    let rest: Vec<&str> = rest.split('/').skip(1).collect();
    let repo = repo();
    match (m, rest.as_slice()) {
        ("GET", ["pulls"]) => {
            let base = param("base").unwrap_or_default();
            let ps = forge.pulls(&repo, &base).unwrap();
            json(200, Value::Array(ps.iter().map(pull_json).collect()))
        }
        ("POST", ["pulls"]) => {
            let p = forge
                .open(
                    &repo,
                    body["base"].as_str().unwrap(),
                    body["head"].as_str().unwrap(),
                    body["title"].as_str().unwrap_or_default(),
                    body["body"].as_str().unwrap_or_default(),
                )
                .unwrap();
            if let Some(b) = bare {
                sync(forge, b);
            }
            let p = forge.pull(&repo, p.number).unwrap().unwrap();
            json(201, pull_json(&p))
        }
        ("GET", ["pulls", n]) => match forge.pull(&repo, n.parse().unwrap()).unwrap() {
            Some(p) => json(200, pull_json(&p)),
            None => not_found(),
        },
        ("PATCH", ["pulls", n]) => {
            let n: u64 = n.parse().unwrap();
            forge
                .edit(&repo, n, body["body"].as_str().unwrap_or_default())
                .unwrap();
            json(200, pull_json(&forge.pull(&repo, n).unwrap().unwrap()))
        }
        ("GET", ["pulls", n, "reviews"]) => {
            let rs = forge.reviews(&repo, n.parse().unwrap()).unwrap();
            json(
                200,
                Value::Array(
                    rs.iter()
                        .map(|r| json!({ "user": { "login": r.user }, "state": r.state, "commit_id": r.commit }))
                        .collect(),
                ),
            )
        }
        ("GET", ["commits", sha, "check-runs"]) => {
            let mut runs: Vec<Value> = Vec::new();
            for p in forge.posted_all(&repo, sha).unwrap() {
                runs.push(run_json(p.id, &p.name, &p.posting));
            }
            if param("check_name").is_none() {
                for c in forge.checks(&repo, sha).unwrap() {
                    let (status, conclusion) = match c.standing {
                        Standing::Pending => ("in_progress", Value::Null),
                        Standing::Passed => ("completed", json!("success")),
                        Standing::Failed => ("completed", json!("failure")),
                    };
                    runs.push(json!({ "id": 1, "name": c.name, "status": status,
                                      "conclusion": conclusion, "app": { "id": 1 } }));
                }
            }
            json(
                200,
                json!({ "total_count": runs.len(), "check_runs": runs }),
            )
        }
        ("GET", ["commits", _, "status"]) => {
            json(200, json!({ "state": "pending", "statuses": [] }))
        }
        ("POST", ["check-runs"]) => {
            forge
                .post_named(
                    &repo,
                    body["head_sha"].as_str().unwrap(),
                    body["name"].as_str().unwrap(),
                    None,
                    &posting(&body),
                )
                .unwrap();
            json(201, json!({}))
        }
        ("PATCH", ["check-runs", id]) => {
            let id: u64 = id.parse().unwrap();
            let held = forge.held().unwrap();
            let Some((sha, name)) = held
                .posted
                .iter()
                .find_map(|(sha, ps)| ps.iter().find(|p| p.id == id).map(|p| (sha, &p.name)))
            else {
                return not_found();
            };
            let name = body["name"].as_str().unwrap_or(name);
            forge
                .post_named(&repo, sha, name, Some(id), &posting(&body))
                .unwrap();
            json(200, json!({}))
        }
        ("GET", ["issues", n, "timeline"]) => {
            let ts = forge.readied(&repo, n.parse().unwrap()).unwrap();
            json(
                200,
                Value::Array(
                    ts.iter()
                        .map(|t| json!({ "event": "ready_for_review", "created_at": t.to_rfc3339() }))
                        .collect(),
                ),
            )
        }
        ("GET", ["issues", n, "comments"]) => {
            let cs = forge.comment_list(&repo, n.parse().unwrap()).unwrap();
            json(
                200,
                Value::Array(
                    cs.iter()
                        .map(|c| json!({ "id": c.id, "body": c.body, "html_url": c.url }))
                        .collect(),
                ),
            )
        }
        ("POST", ["issues", n, "comments"]) => {
            forge
                .comment(
                    &repo,
                    n.parse().unwrap(),
                    body["body"].as_str().unwrap_or_default(),
                )
                .unwrap();
            json(201, json!({}))
        }
        ("PATCH", ["issues", "comments", id]) => {
            forge
                .edit_comment(
                    &repo,
                    id.parse().unwrap(),
                    body["body"].as_str().unwrap_or_default(),
                )
                .unwrap();
            json(200, json!({}))
        }
        _ => not_found(),
    }
}

/// Where a test keeps its fake GitHub's file.
pub fn file(dir: &Path) -> PathBuf {
    dir.join("github.json")
}
