//! `sluice init actions` (137, C.8; design D13 of
//! pull-requests-are-queued-on-the-forge): the forge's round job written
//! into a repository whose policy names its hosts' signers, the policy's
//! `[signers.forge]` and `[pools.forge]` added where absent, and what to
//! set on GitHub said.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use sluice_core::policy::{Policy, POLICY_PATH};

use crate::profiles::local_refs::LocalRefs;

/// The round job's workflow as this version writes it, `{{version}}` its
/// version.
pub const TEMPLATE: &str = include_str!("workflows/sluice.yml");

/// Where a repository holds the round job's workflow.
pub const WORKFLOW_PATH: &str = ".github/workflows/sluice.yml";

/// The round job's workflow as this version of sluice writes it.
pub fn workflow() -> String {
    TEMPLATE.replace("{{version}}", crate::VERSION)
}

/// The exit of a policy `sluice init actions` refuses, changing nothing.
const REFUSED: i32 = 2;
/// The exit of a workflow left as it differs.
const DIFFERS: i32 = 1;

/// `sluice init actions [--force]` in the repository holding `dir`, its
/// working copy's policy edited where it stands. A policy that names no
/// `[signers]` is refused with exit 2, changing nothing: the first signer
/// moves every branch with a remote onto it, so the hosts that run its
/// checks need signers of their own first. A workflow there that differs
/// from what this version writes is left, saying how, with exit 1, unless
/// `force`.
pub fn actions(dir: &Path, force: bool, out: &mut dyn Write) -> Result<i32> {
    let local = LocalRefs::open(dir)?;
    let top = crate::profiles::worktree_top(dir)?
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| dir.to_path_buf());
    let policy_path = top.join(POLICY_PATH);
    let text = std::fs::read_to_string(&policy_path)
        .with_context(|| format!("reading {}", policy_path.display()))?;
    let policy = Policy::parse(&text).context("reading .config/sluice.toml")?;
    if policy.signers.is_empty() {
        writeln!(
            out,
            "sluice init actions: the policy names no [signers]. The hosts that run the checks \
             need signers before a branch's records move to its remote, which the first signer \
             does for every branch with a remote: name [signers] for them first, then run \
             `sluice init actions` again. Nothing was written."
        )?;
        return Ok(REFUSED);
    }
    let default = local.default_branch()?.unwrap_or_else(|| "main".into());

    // The workflow, at this version.
    let written = workflow();
    let path = top.join(WORKFLOW_PATH);
    let mut code = 0;
    match std::fs::read_to_string(&path) {
        Ok(there) if there == written => {
            writeln!(
                out,
                "{WORKFLOW_PATH} is the round job of sluice {}",
                crate::VERSION
            )?;
        }
        Ok(there) if !force => {
            writeln!(
                out,
                "{WORKFLOW_PATH} differs from the round job sluice {} writes, and is left as it \
                 is; `sluice init actions --force` writes it:",
                crate::VERSION
            )?;
            for line in differs(&there, &written) {
                writeln!(out, "  {line}")?;
            }
            code = DIFFERS;
        }
        _ => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("making {}", parent.display()))?;
            }
            std::fs::write(&path, &written)
                .with_context(|| format!("writing {}", path.display()))?;
            writeln!(
                out,
                "wrote {WORKFLOW_PATH}, the round job of sluice {}",
                crate::VERSION
            )?;
        }
    }

    // The policy's signer and pool, where absent.
    let mut added = String::new();
    if !policy.signers.contains_key("forge") {
        added.push_str(&format!(
            "\n# The forge's round job, which keeps each branch that takes pull requests\n\
             # (137, C.8); `sluice init actions` wrote it.\n\
             [signers.forge]\n\
             github = {{ workflow = \"{WORKFLOW_PATH}\", ref = \"refs/heads/{default}\" }}\n"
        ));
    }
    if !policy.pools.iter().any(|p| p.name == "forge") {
        added.push_str(&format!(
            "\n# The round job's pool: it names no runners, so it runs rounds alone.\n\
             [pools.forge]\n\
             github = {{ workflow = \"{WORKFLOW_PATH}\" }}\n\
             signer = \"forge\"\n"
        ));
    }
    if !added.is_empty() {
        let mut edited = text.clone();
        if !edited.ends_with('\n') {
            edited.push('\n');
        }
        edited.push_str(&added);
        Policy::parse(&edited).context("the policy with the round job's signer and pool")?;
        std::fs::write(&policy_path, &edited)
            .with_context(|| format!("writing {}", policy_path.display()))?;
        writeln!(
            out,
            "added [signers.forge] and [pools.forge] to {POLICY_PATH}"
        )?;
    }

    writeln!(
        out,
        "\nOn GitHub, for the round job:\n\
         - an environment named `sluice`, whose deployment branch rule admits {default} alone;\n\
         - in it, the variable SLUICE_APP_ID, the app's id, and the secret SLUICE_APP_KEY, its \
         private key;\n\
         - the app's permission to write Actions, so a submit can wake the round job.\n\
         Then land {WORKFLOW_PATH} and {POLICY_PATH} on {default}, and `sluice forge check \
         {default}` says whether all of it is set."
    )?;
    Ok(code)
}

/// How `there` differs from `written`, line by line: each line of the one
/// the other lacks, `-` for what is there, `+` for what would be written.
fn differs(there: &str, written: &str) -> Vec<String> {
    let ours: Vec<&str> = written.lines().collect();
    let theirs: Vec<&str> = there.lines().collect();
    let mut lines: Vec<String> = theirs
        .iter()
        .filter(|l| !ours.contains(l))
        .map(|l| format!("- {l}"))
        .collect();
    lines.extend(
        ours.iter()
            .filter(|l| !theirs.contains(l))
            .map(|l| format!("+ {l}")),
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C.8, design D4: the template's round job is its only job, and the
    /// only one that names the `sluice` environment.
    #[test]
    fn the_template_names_its_version_and_one_environment() {
        let w = workflow();
        assert!(!w.contains("{{version}}"));
        assert!(w.contains(&format!("SLUICE_VERSION: \"{}\"", crate::VERSION)));
        assert_eq!(w.matches("environment:").count(), 1);
    }

    #[test]
    fn a_difference_names_each_line_either_lacks() {
        assert_eq!(
            differs("a\nb\nc\n", "a\nc\nd\n"),
            ["- b".to_string(), "+ d".to_string()]
        );
    }
}
