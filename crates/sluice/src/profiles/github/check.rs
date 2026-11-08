//! `sluice forge check` on GitHub: whether the app is set up so the
//! branch's rules let only sluice move it (C.4 pushed refs). The app's
//! token mints and the app may write the repository's contents and
//! workflows; every ruleset on the branch lets the app through and nobody
//! else.
//!
//! GitHub shows a ruleset's bypass list only to a caller with write access
//! to the ruleset, which the app is not given, so the rulesets are read
//! with the operator's own token.

use std::collections::BTreeSet;

use serde_json::Value;

use super::app::{App, Bearer, Repo};

/// One thing checked, and whether it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub ok: bool,
    pub what: String,
}

fn ok(what: String) -> Finding {
    Finding { ok: true, what }
}

fn failed(what: String) -> Finding {
    Finding { ok: false, what }
}

/// Check `app` on `repo` for `branch`; `operator` is the operator's own
/// token, which reads the rulesets' bypass lists.
pub fn check(app: &App, repo: &Repo, branch: &str, operator: Option<&str>) -> Vec<Finding> {
    let mut found = Vec::new();
    match app.token(repo) {
        Ok(_) => found.push(ok(format!("app {} makes a token for {repo}", app.id()))),
        Err(e) => {
            found.push(failed(format!(
                "app {} makes no token for {repo}: {e:#}",
                app.id()
            )));
            return found;
        }
    };
    match app.installation(repo) {
        Ok(i) => {
            // Contents to land at all; workflows because GitHub refuses an
            // app's push of any change under `.github/workflows`; checks
            // and pull requests to show a pull request its state and its
            // report (ADRs 0021, 0039).
            for (key, what, grant) in [
                ("contents", "contents", "Contents"),
                ("workflows", "workflows", "Workflows"),
                ("checks", "checks", "Checks"),
                ("pull_requests", "pull requests", "Pull requests"),
            ] {
                found.push(match i.permissions.get(key).map(String::as_str) {
                    Some("write") => ok(format!("the app may write {repo}'s {what}")),
                    other => failed(format!(
                        "the app may not write {repo}'s {what} ({key}: {}): give it \
                         {grant} read and write",
                        other.unwrap_or("none")
                    )),
                });
            }
        }
        Err(e) => found.push(failed(format!("{e:#}"))),
    }
    found.extend(rules(app, repo, branch, operator));
    found
}

/// Each ruleset on `branch`: the app among its bypass actors, and no one
/// else.
fn rules(app: &App, repo: &Repo, branch: &str, operator: Option<&str>) -> Vec<Finding> {
    let path = format!(
        "/repos/{}/{}/rules/branches/{branch}",
        repo.owner, repo.name
    );
    let rules = match app.get(&path, Bearer::Repo(repo)) {
        Ok(v) => v,
        Err(e) => return vec![failed(format!("reading {branch}'s rules: {e:#}"))],
    };
    let ids: BTreeSet<u64> = rules
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r["ruleset_id"].as_u64())
        .collect();
    if ids.is_empty() {
        return vec![failed(format!(
            "no ruleset on GitHub protects {branch}: anyone who may push to {repo} may move it"
        ))];
    }
    let Some(operator) = operator else {
        return vec![failed(format!(
            "no token of yours to read {branch}'s rulesets with: GitHub shows who may bypass \
             a ruleset only to its admins; set GH_TOKEN or sign in with gh"
        ))];
    };
    ids.into_iter()
        .map(|id| {
            let path = format!(
                "/repos/{}/{}/rulesets/{id}?includes_parents=true",
                repo.owner, repo.name
            );
            match app.get(&path, Bearer::Token(operator)) {
                Ok(r) => ruleset(app.id(), branch, &r),
                Err(e) => failed(format!("reading ruleset {id}: {e:#}")),
            }
        })
        .collect()
}

fn ruleset(app: u64, branch: &str, r: &Value) -> Finding {
    let name = r["name"].as_str().map_or_else(
        || format!("ruleset {}", r["id"]),
        |n| format!("ruleset {n:?}"),
    );
    let Some(actors) = r["bypass_actors"].as_array() else {
        return failed(format!(
            "your token cannot see who may bypass {name} on {branch}: it needs admin access \
             to the ruleset"
        ));
    };
    let is_app = |a: &Value| {
        a["actor_type"] == "Integration"
            && a["actor_id"].as_u64() == Some(app)
            && a["bypass_mode"] == "always"
    };
    let others: Vec<String> = actors
        .iter()
        .filter(|a| !is_app(a))
        .map(|a| {
            let kind = a["actor_type"].as_str().unwrap_or("an actor");
            match a["actor_id"].as_u64() {
                Some(id) => format!("{kind} {id}"),
                None => kind.to_string(),
            }
        })
        .collect();
    if !actors.iter().any(is_app) {
        return failed(format!(
            "{name} on {branch} does not let app {app} through: add it as the ruleset's \
             bypass, always"
        ));
    }
    if !others.is_empty() {
        return failed(format!(
            "{name} on {branch} lets more than the app through: {}",
            others.join(", ")
        ));
    }
    ok(format!("{name} on {branch} lets only app {app} through"))
}

/// 137, C.8; design D4, D13 of pull-requests-are-queued-on-the-forge: for
/// a branch the forge keeps, whether the repository's default branch, on
/// which the round job of every branch the forge keeps runs, holds the
/// round job `sluice init actions` writes at this version, whether the app
/// may dispatch it, and whether the `sluice` environment exists and admits
/// the default branch alone.
pub fn round_job(app: &App, repo: &Repo, branch: &str) -> Vec<Finding> {
    let mut found = Vec::new();
    let at = format!("/repos/{}/{}", repo.owner, repo.name);
    let default = match app.get(&at, Bearer::Repo(repo)) {
        Ok(v) => match v["default_branch"].as_str() {
            Some(d) => d.to_string(),
            None => {
                found.push(failed(format!("GitHub names no default branch for {repo}")));
                return found;
            }
        },
        Err(e) => {
            found.push(failed(format!("reading {repo}'s default branch: {e:#}")));
            return found;
        }
    };
    let path = format!("{at}/contents/{}?ref={default}", super::init::WORKFLOW_PATH);
    let wf = super::init::WORKFLOW_PATH;
    let runs = if default == branch {
        String::new()
    } else {
        format!(", where {branch}'s round job runs")
    };
    found.push(match app.get(&path, Bearer::Repo(repo)) {
        Ok(v) => match content(&v) {
            Some(text) if text == super::init::workflow() => ok(format!(
                "{default} holds the round job of sluice {}{runs}",
                crate::VERSION
            )),
            Some(_) => failed(format!(
                "{default}'s {wf} is not the round job sluice {} writes: write it again with \
                 `sluice init actions` and land it on {default}",
                crate::VERSION
            )),
            None => failed(format!("{default}'s {wf} could not be read as a file")),
        },
        Err(e) if format!("{e:#}").contains("404") => failed(format!(
            "{default} holds no {wf}, so nothing takes {branch}'s pull requests in: write it \
             with `sluice init actions` and land it on {default}"
        )),
        Err(e) => failed(format!("reading {default}'s {wf}: {e:#}")),
    });
    match app.installation(repo) {
        Ok(i) => found.push(match i.permissions.get("actions").map(String::as_str) {
            Some("write") => ok(format!("the app may dispatch {repo}'s round job")),
            other => failed(format!(
                "the app may not dispatch {repo}'s round job (actions: {}): give it Actions \
                 read and write, so a submit and a recorded result wake it",
                other.unwrap_or("none")
            )),
        }),
        Err(e) => found.push(failed(format!("{e:#}"))),
    }
    found.push(environment(app, repo, &default));
    found
}

/// A file's text as the contents API answers it, base64 in `content`.
fn content(v: &Value) -> Option<String> {
    if v["encoding"] != "base64" {
        return None;
    }
    let packed: String = v["content"].as_str()?.split_whitespace().collect();
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(packed)
        .ok()?;
    String::from_utf8(bytes).ok()
}

/// The `sluice` environment, which holds the app's key: it exists, and its
/// deployment branch rule admits `branch`, the default branch the round
/// job runs on, alone (C.8; design D4).
fn environment(app: &App, repo: &Repo, branch: &str) -> Finding {
    let at = format!("/repos/{}/{}/environments/sluice", repo.owner, repo.name);
    let env = match app.get(&at, Bearer::Repo(repo)) {
        Ok(v) => v,
        Err(e) if format!("{e:#}").contains("404") => {
            return failed(format!(
                "{repo} has no `sluice` environment to hold the app's key: make one whose \
                 deployment branch rule admits {branch} alone"
            ))
        }
        Err(e) => return failed(format!("reading the `sluice` environment: {e:#}")),
    };
    let rule = &env["deployment_branch_policy"];
    if rule.is_null() || rule["custom_branch_policies"] != true {
        return failed(format!(
            "any branch of {repo} may use the `sluice` environment, and with it the app's key: \
             give it a deployment branch rule admitting {branch} alone"
        ));
    }
    let path = format!("{at}/deployment-branch-policies");
    let names: Vec<String> = match app.get(&path, Bearer::Repo(repo)) {
        Ok(v) => v["branch_policies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|p| p["type"].as_str().is_none_or(|t| t == "branch"))
            .filter_map(|p| p["name"].as_str().map(str::to_string))
            .collect(),
        Err(e) => return failed(format!("reading the `sluice` environment's rule: {e:#}")),
    };
    if names == [branch] {
        ok(format!("the `sluice` environment admits {branch} alone"))
    } else {
        failed(format!(
            "the `sluice` environment admits {}, not {branch} alone: give it a deployment \
             branch rule admitting {branch} alone",
            if names.is_empty() {
                "no branch".to_string()
            } else {
                names.join(", ")
            }
        ))
    }
}
