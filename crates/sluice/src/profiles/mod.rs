//! Binding each seam to a profile (52; ADR 0003). The only directory of
//! this crate that may name a tool (rule core.1).
//!
//! The real profiles are pass 2's (`docs/plans/phase-1-integrations.md`).
//! The branch is bound to [`local_refs`], the records to [`local_files`],
//! the check to [`checks`] and the place to [`places`]; a seam whose
//! profile is not built yet is bound to
//! [`Unbuilt`], which refuses every call by naming that profile, so the
//! CLI's arguments, output and exits are whole and only those seams are
//! missing.

pub mod checks;
pub mod local_files;
pub mod local_refs;
pub mod moon;
pub mod places;
pub mod worktrunk;

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use sluice_seams::{Driver, Notification, Session};

use crate::instance::Instance;

/// A seam whose profile is not built yet.
pub struct Unbuilt {
    pub seam: &'static str,
    pub profile: &'static str,
}

impl Unbuilt {
    fn refuse<T>(&self) -> Result<T> {
        bail!(
            "the {} profile of the {} seam is built in pass 2 (docs/plans/phase-1-integrations.md)",
            self.profile,
            self.seam
        )
    }
}

/// The profiles the walkthrough's setup names, for the repository holding
/// a directory.
pub struct Bound {
    pub checkout: places::Places,
    pub tests: checks::Checks,
    pub branch: local_refs::LocalRefs,
    pub driver: Unbuilt,
    pub state: local_files::LocalFiles,
}

impl Bound {
    /// Bind the repository holding `dir`, a worktree or a git directory,
    /// for this executable and this instance's settings.
    pub fn open(dir: &Path) -> Result<Self> {
        Bound::open_with(dir, std::env::current_exe()?, Instance::read()?)
    }

    /// `program` is the `sluice` executable that supervises check runs.
    pub fn open_with(dir: &Path, program: PathBuf, instance: Instance) -> Result<Self> {
        let branch = local_refs::LocalRefs::open(dir)?;
        let common = branch.common_dir()?;
        let state = local_files::LocalFiles::new(&common);
        Ok(Bound {
            checkout: places::Places::new(
                &common,
                crate::runs::Runs::new(&common, program.clone()),
            ),
            tests: checks::Checks::new(&common, program, instance),
            branch,
            driver: Unbuilt {
                seam: "driver",
                profile: "worktrunk",
            },
            state,
        })
    }

    pub fn seams(&self) -> sluice_core::Seams<'_> {
        sluice_core::Seams {
            checkout: &self.checkout,
            tests: &self.tests,
            branch: &self.branch,
            driver: &self.driver,
            state: &self.state,
        }
    }
}

impl Driver for Unbuilt {
    fn session(&self, _: &str) -> Result<Session> {
        self.refuse()
    }
    fn notify(&self, _: &Notification) -> Result<()> {
        self.refuse()
    }
}
