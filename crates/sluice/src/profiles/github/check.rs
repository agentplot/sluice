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
