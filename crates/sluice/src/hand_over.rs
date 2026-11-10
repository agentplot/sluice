//! A `sluice run` that takes up a change of the host's settings no
//! binding takes up in place, `[telemetry]`, by starting itself afresh in
//! its own process (135 as amended 2026-11-09; record 0107 as amended;
//! rule core.54; design D2 to D5 of
//! `a-kept-run-hands-over-what-it-cannot-take-up`).
//!
//! The run replaces its own program image, so the process keeps its id,
//! its parent, its terminal, its output, its folder and its session, and
//! whatever runs it goes on following it. The image started afresh is the
//! run as it started: the executable it started as, resolved through every
//! link, the command line it was given, and the environment it started
//! with, before telemetry took the caller's OTLP variables out of it
//! (151; rule core.53). What the run carries, it writes as one JSON
//! document into [`VAR`]:
//!
//! - the settings file's text the run read and bound, and the tables that
//!   changed;
//! - the passes counted toward `--rounds`;
//! - the number of a descriptor that duplicates the driver's lock, left
//!   open across the start, so the repository has its driver throughout
//!   (rule core.31);
//! - the run supervisors it started and has not reaped;
//! - the holds and refusals it said, and whether it said another run
//!   drives the repository.
//!
//! The form needs no version: one executable writes it and reads it. The
//! image started afresh reads it first, before any thread or child
//! starts, and takes it out of its environment, so no child inherits it.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sluice_seams::Rev;

use crate::Carried;

/// The variable the document is handed over in.
pub const VAR: &str = "SLUICE_HANDED_OVER";

/// What a `sluice run` hands to itself started afresh (design D4).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandOver {
    /// The settings file's text the run read and bound.
    pub settings: String,
    /// The tables that changed, as `Instance::changed_tables` names them.
    pub tables: Vec<String>,
    /// The passes counted toward `--rounds`.
    pub passes: usize,
    /// The number of the descriptor that duplicates the driver's lock,
    /// open across the start, where the run held it.
    pub driver: Option<i32>,
    /// The run supervisors the run started and has not reaped (design D7).
    pub supervisors: Vec<u32>,
    /// 121: each branch's hold as last said.
    pub said: BTreeMap<String, Option<(Rev, Rev)>>,
    /// 26, 121: the words each branch passed by was last said with.
    pub passed: BTreeMap<String, String>,
    /// Whether the run said another `sluice run` drives the repository.
    pub said_beside: bool,
}

impl HandOver {
    /// The document handed over, where this process is a `sluice run`
    /// started afresh, and what it carries. [`VAR`] is taken out of the
    /// environment, so no child inherits it, and the driver's descriptor
    /// is made one closed on a start at once: a duplicate closed on a
    /// start, then the one inherited closed, so no run supervisor this
    /// process starts holds the lock past this run (design D4). A
    /// descriptor that is not open is no driver, and the first pass takes
    /// the lock as any start does. Called first in `main`, before any
    /// thread or child starts.
    pub fn take() -> Result<Option<(HandOver, Carried)>> {
        let Some(text) = std::env::var_os(VAR) else {
            return Ok(None);
        };
        std::env::remove_var(VAR);
        let text = text
            .into_string()
            .map_err(|_| anyhow!("{VAR} is not UTF-8"))?;
        let handed: HandOver =
            serde_json::from_str(&text).with_context(|| format!("reading {VAR}"))?;
        let driver = match handed.driver {
            Some(raw) if open(raw) => {
                // SAFETY: the descriptor is open, and it was left open
                // across the start by the run this process started afresh
                // from, which named it; nothing in this process has run
                // yet to own it.
                let inherited = unsafe { OwnedFd::from_raw_fd(raw) };
                let kept = inherited
                    .try_clone()
                    .context("holding the driver's lock handed over")?;
                drop(inherited);
                Some(kept)
            }
            _ => None,
        };
        let carried = Carried {
            passes: handed.passes,
            driver,
            said: handed.said.clone(),
            passed: handed.passed.clone(),
            said_beside: handed.said_beside,
        };
        Ok(Some((handed, carried)))
    }

    /// The command that starts the run afresh: `program`, with `args`,
    /// its environment cleared and set to `env`, but for [`VAR`], which
    /// holds this document (design D3, D4).
    pub fn command(
        &self,
        program: &Path,
        args: &[OsString],
        env: &[(OsString, OsString)],
    ) -> Result<Command> {
        let doc = serde_json::to_string(self).context("writing the hand-over")?;
        let mut cmd = Command::new(program);
        cmd.args(args)
            .env_clear()
            .envs(env.iter().filter(|(name, _)| name != VAR).cloned())
            .env(VAR, doc);
        Ok(cmd)
    }

    /// Replace this process's image with the run started afresh, handing
    /// it `driver`, the duplicate of the driver's lock, by its number
    /// (design D2, D5). It returns only where the start failed: `driver`
    /// is closed, so the driver the run keeps is the only hold on the
    /// lock, and the error names `program`.
    pub fn start(
        mut self,
        program: &Path,
        args: &[OsString],
        env: &[(OsString, OsString)],
        driver: Option<OwnedFd>,
    ) -> anyhow::Error {
        self.driver = driver.as_ref().map(AsRawFd::as_raw_fd);
        let failed = match self.command(program, args, env) {
            Ok(mut cmd) => anyhow::Error::from(cmd.exec()),
            Err(e) => e,
        };
        drop(driver);
        failed.context(format!("starting {} afresh", program.display()))
    }
}

/// Whether `raw` names a descriptor open in this process.
fn open(raw: i32) -> bool {
    // SAFETY: F_GETFD reads a descriptor's flags and touches no memory.
    raw >= 0 && unsafe { nix::libc::fcntl(raw, nix::libc::F_GETFD) } != -1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A document with every field set.
    fn every() -> HandOver {
        HandOver {
            settings: "[telemetry]\nsend = true\ncollector = \"http://localhost:4318\"\n".into(),
            tables: vec!["[telemetry]".into(), "[[caches]]".into()],
            passes: 4,
            driver: Some(7),
            supervisors: vec![101, 202],
            said: BTreeMap::from([(
                "main".into(),
                Some((Rev("a".repeat(40)), Rev("b".repeat(40)))),
            )]),
            passed: BTreeMap::from([("bolt/a".into(), "its policy does not read".into())]),
            said_beside: true,
        }
    }

    /// The settings-take-up spec's "The hand-over's command": the run
    /// started afresh is the executable, the command line and the
    /// environment the run started with, its OTLP variable among it, and
    /// the document, which reads back as written (151; core.53; design
    /// D3, D4).
    #[test]
    fn the_run_started_afresh_is_the_one_that_started() {
        let handed = every();
        let args: Vec<OsString> = ["run", "--rounds", "12"].map(OsString::from).into();
        let env: Vec<(OsString, OsString)> = vec![
            ("HOME".into(), "/home/someone".into()),
            (
                "OTEL_EXPORTER_OTLP_ENDPOINT".into(),
                "http://localhost:9999".into(),
            ),
        ];
        let cmd = handed.command(Path::new("/x/sluice"), &args, &env).unwrap();
        assert_eq!(cmd.get_program(), "/x/sluice");
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            ["run", "--rounds", "12"]
        );
        let mut set: BTreeMap<OsString, Option<OsString>> = cmd
            .get_envs()
            .map(|(k, v)| (k.to_owned(), v.map(ToOwned::to_owned)))
            .collect();
        let doc = set
            .remove(&OsString::from(VAR))
            .flatten()
            .expect("the document");
        let expected: BTreeMap<OsString, Option<OsString>> =
            env.into_iter().map(|(k, v)| (k, Some(v))).collect();
        assert_eq!(set, expected);
        assert!(
            format!("{cmd:?}").starts_with("env -i "),
            "every other variable is cleared: {cmd:?}"
        );
        let read: HandOver = serde_json::from_str(doc.to_str().unwrap()).unwrap();
        assert_eq!(read, handed);
    }
}
