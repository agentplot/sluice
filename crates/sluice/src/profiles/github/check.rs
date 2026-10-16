//! `sluice forge check` on GitHub: whether the app is set up so the
//! branch's rules let only sluice move it (C.4 pushed refs). The app's
//! token mints and the app may write the repository's contents and
//! workflows; every ruleset on the branch lets the app through and nobody
//! else. A ruleset whose only rule on the branch restricts its creation
//! may let others through: it lets them make the branch, as crew makes a
//! bolt, and never move it.
//!
//! GitHub shows a ruleset's bypass list only to a caller with write access
//! to the ruleset, which the app is not given, so the rulesets are read
//! with the operator's own token.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::app::{App, Repo};

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
    let token = match app.token(repo) {
        Ok(t) => {
            found.push(ok(format!("app {} makes a token for {repo}", app.id())));
            t
        }
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
            // app's push of any change under `.github/workflows`.
            for (key, what, grant) in [
                ("contents", "contents", "Contents"),
                ("workflows", "workflows", "Workflows"),
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
    found.extend(rules(app, repo, branch, &token, operator));
    found
}

/// Each ruleset on `branch`: the app among its bypass actors, and no one
/// else, unless all it restricts on `branch` is creating it.
fn rules(
    app: &App,
    repo: &Repo,
    branch: &str,
    token: &str,
    operator: Option<&str>,
) -> Vec<Finding> {
    let path = format!(
        "/repos/{}/{}/rules/branches/{branch}",
        repo.owner, repo.name
    );
    let rules = match app.get(&path, token) {
        Ok(v) => v,
        Err(e) => return vec![failed(format!("reading {branch}'s rules: {e:#}"))],
    };
    let mut kinds: BTreeMap<u64, BTreeSet<String>> = BTreeMap::new();
    for r in rules.as_array().into_iter().flatten() {
        if let Some(id) = r["ruleset_id"].as_u64() {
            let kind = r["type"].as_str().unwrap_or_default().to_string();
            kinds.entry(id).or_default().insert(kind);
        }
    }
    let ids: BTreeSet<u64> = kinds.keys().copied().collect();
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
            match app.get(&path, operator) {
                Ok(r) => {
                    let creation = kinds[&id].iter().all(|k| k == "creation");
                    ruleset(app.id(), branch, &r, creation)
                }
                Err(e) => failed(format!("reading ruleset {id}: {e:#}")),
            }
        })
        .collect()
}

/// `creation`: all the ruleset restricts on `branch` is creating it.
fn ruleset(app: u64, branch: &str, r: &Value, creation: bool) -> Finding {
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
    if creation {
        return ok(format!(
            "{name} on {branch} restricts only its creation: whoever it lets through may \
             make {branch}, never move it"
        ));
    }
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
