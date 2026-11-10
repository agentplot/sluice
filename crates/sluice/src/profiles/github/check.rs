//! `sluice forge check` on GitHub: whether the app is set up so the
//! branch's rules let only sluice move it (C.4 pushed refs). The app's
//! token mints and the app may write the repository's contents and
//! workflows; every ruleset on the branch lets the app through and nobody
//! else.
//!
//! GitHub shows a ruleset's bypass list only to a caller with write access
//! to the ruleset, which the app is not given, so the rulesets are read
//! with the operator's own token.
//!
//! Under a credential command, which mints nothing, the check reads what
//! GitHub shows a token's holder: that the command gives a token for the
//! repository, that the token is the app's `[forge] app_id` names, and
//! that it may push, from the receive-pack advertisement git's push asks
//! for first; then the rulesets. GitHub shows a token's other grants to no
//! holder of the token alone, so they are left to where the app's key is
//! kept (record 0108).

use std::collections::BTreeSet;

use serde_json::Value;

use super::app::{App, Bearer, Repo};
use crate::identity::CredentialCommand;

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
    if let Some(command) = app.command() {
        return commanded(app, command, repo, branch, operator);
    }
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

/// [`check`] under a credential command: a token comes, it is the app's,
/// it may push; then the rulesets (record 0108).
fn commanded(
    app: &App,
    command: &CredentialCommand,
    repo: &Repo,
    branch: &str,
    operator: Option<&str>,
) -> Vec<Finding> {
    let named = format!("[forge] credential {command}");
    let mut found = Vec::new();
    match app.token(repo) {
        Ok(_) => found.push(ok(format!("{named} gives a token for {repo}"))),
        Err(e) => {
            let words = format!("{e:#}");
            found.push(failed(if words.starts_with(&named) {
                words
            } else {
                format!("{named} gives no token for {repo}: {words}")
            }));
            return found;
        }
    }
    let id = app.id();
    found.push(match app.app_of_token(repo) {
        Ok((of, _)) if of == id => ok(format!("the token is app {id}'s")),
        Ok((of, slug)) => failed(format!(
            "the token {named} gives is app {of}'s ({slug}), not app {id}'s: name in [forge] \
             app_id the app whose token it gives"
        )),
        Err(e) => failed(format!("reading whose token {named} gives: {e:#}")),
    });
    found.push(match app.may_push(repo) {
        Ok(200..=299) => ok(format!("the token may push to {repo}")),
        Ok(status @ (401 | 403 | 404)) => failed(format!(
            "the token may not push to {repo}: GitHub answered git's push {status}: give the \
             app Contents read and write"
        )),
        Ok(status) => failed(format!(
            "GitHub answered git's push to {repo} {status}, which says nothing of whether the \
             token may push"
        )),
        Err(e) => failed(format!(
            "reading whether the token may push to {repo}: {e:#}"
        )),
    });
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
