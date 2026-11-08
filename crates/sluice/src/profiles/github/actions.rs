//! GitHub Actions as a pool (88, 107, 111; C.2 launched; ADR 0034): a run
//! no standing host can take starts one job of the workflow the policy's
//! pool names, by `workflow_dispatch`, on the runner its system names; a
//! wake starts a job that advances a branch. A job is named by its claim
//! in the run's title, `sluice <claim>`, which the workflow sets, so
//! whether it still runs is read from the workflow's queued and running
//! runs.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use sluice_seams::{Launch, Setting};

use super::app::{App, Bearer, Call, Repo};
use crate::profiles::checks::Launcher;
use crate::profiles::pushed_refs::remote_url;

pub struct Actions {
    app: Rc<App>,
    dir: PathBuf,
}

impl Actions {
    /// The pool of the repository holding `dir`, reached as `app`.
    pub fn new(app: Rc<App>, dir: &Path) -> Actions {
        Actions {
            app,
            dir: dir.to_path_buf(),
        }
    }

    fn repo(&self, remote: Option<&str>) -> Result<Repo> {
        let remote = remote.unwrap_or("origin");
        let url = remote_url(&self.dir, remote)?;
        Repo::from_url(&url).ok_or_else(|| anyhow!("{remote} ({url}) is no repository on GitHub"))
    }

    /// The workflow a `github` table names, as the API names it: its file.
    fn workflow(kind: &(String, Setting)) -> Result<String> {
        let path = field(kind, "workflow")?;
        Ok(path.rsplit('/').next().unwrap_or(&path).to_string())
    }

    /// Start a run of `workflow` on `git_ref` (134; rule core.29): a call
    /// that starts something, which nothing reads back, so one whose
    /// answer was not read is never sent again and fails
    /// [`sluice_seams::Unanswered`] for its caller to judge.
    fn dispatch(&self, repo: &Repo, workflow: &str, git_ref: &str, inputs: Value) -> Result<()> {
        let path = format!(
            "/repos/{}/{}/actions/workflows/{workflow}/dispatches",
            repo.owner, repo.name
        );
        let body = json!({ "ref": git_ref, "inputs": inputs }).to_string();
        self.app
            .post_api(Call::Make, &path, Bearer::Repo(repo), &body)?;
        Ok(())
    }
}

/// A key of a `github = { ... }` table.
fn field(kind: &(String, Setting), key: &str) -> Result<String> {
    if kind.0 != "github" {
        bail!("a {} pool is not GitHub Actions", kind.0);
    }
    match &kind.1 {
        Setting::Tables(t) => match t.first().and_then(|t| t.get(key)) {
            Some(Setting::Text(v)) => Ok(v.clone()),
            _ => bail!("the github table names no {key}"),
        },
        _ => bail!("github must be a table naming a workflow"),
    }
}

/// The branch a ref names: `refs/heads/main` is `main`.
fn branch_of(git_ref: &str) -> &str {
    git_ref.strip_prefix("refs/heads/").unwrap_or(git_ref)
}

impl Launcher for Actions {
    fn launch(&self, launch: &Launch) -> Result<()> {
        let repo = self.repo(launch.remote.as_deref())?;
        let workflow = Actions::workflow(&launch.kind)?;
        let git_ref = field(&launch.signer, "ref").unwrap_or_else(|_| "refs/heads/main".into());
        self.dispatch(
            &repo,
            &workflow,
            branch_of(&git_ref),
            json!({
                "mode": "run",
                "branch": launch.branch,
                "claim": launch.claim,
                "combination": launch.combination.as_str(),
                "system": launch.system.clone().unwrap_or_default(),
                "runner": launch.runner,
            }),
        )
    }

    fn job_alive(
        &self,
        kind: &(String, Setting),
        claim: &str,
        remote: Option<&str>,
    ) -> Result<bool> {
        let repo = self.repo(remote)?;
        let workflow = Actions::workflow(kind)?;
        let title = format!("sluice {claim}");
        for status in ["queued", "in_progress", "waiting", "pending", "requested"] {
            let path = format!(
                "/repos/{}/{}/actions/workflows/{workflow}/runs?status={status}&per_page=100",
                repo.owner, repo.name
            );
            let v = self.app.get(&path, Bearer::Repo(&repo))?;
            let found = v["workflow_runs"]
                .as_array()
                .is_some_and(|runs| runs.iter().any(|r| r["display_title"] == title.as_str()));
            if found {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn wake(&self, kind: &(String, Setting), branch: &str, remote: Option<&str>) -> Result<()> {
        let repo = self.repo(remote)?;
        let workflow = Actions::workflow(kind)?;
        let git_ref = field(kind, "ref").unwrap_or_else(|_| "main".into());
        self.dispatch(
            &repo,
            &workflow,
            branch_of(&git_ref),
            json!({ "mode": "round", "branch": branch, "claim": format!("round/{branch}") }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn github(pairs: &[(&str, &str)]) -> (String, Setting) {
        let t: BTreeMap<String, Setting> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), Setting::Text(v.to_string())))
            .collect();
        ("github".into(), Setting::Tables(vec![t]))
    }

    #[test]
    fn a_github_table_names_its_workflow_file() {
        let kind = github(&[("workflow", ".github/workflows/sluice.yml")]);
        assert_eq!(Actions::workflow(&kind).unwrap(), "sluice.yml");
        assert!(Actions::workflow(&("ssh".into(), Setting::Text("k".into()))).is_err());
        assert_eq!(branch_of("refs/heads/main"), "main");
    }
}
