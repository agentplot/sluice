//! Binding each seam to a profile (52; ADR 0003). The only directory of
//! this crate that may name a tool (rule core.1).
//!
//! The real profiles are pass 2's (`docs/plans/phase-1-integrations.md`).
//! The branch is bound to [`local_refs`] and the records to
//! [`local_files`]; a seam whose profile is not built yet is bound to
//! [`Unbuilt`], which refuses every call by naming that profile, so the
//! CLI's arguments, output and exits are whole and only those seams are
//! missing.

pub mod local_files;
pub mod local_refs;
pub mod worktrunk;

use std::path::Path;

use anyhow::{bail, Result};
use sluice_seams::{
    Checkout, Driver, Notification, Place, Progress, Rev, RunHandle, RunRequest, Session, Tests,
};

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
    pub checkout: Unbuilt,
    pub tests: Unbuilt,
    pub branch: local_refs::LocalRefs,
    pub driver: Unbuilt,
    pub state: local_files::LocalFiles,
}

impl Bound {
    /// Bind the repository holding `dir`, a worktree or a git directory.
    pub fn open(dir: &Path) -> Result<Self> {
        let branch = local_refs::LocalRefs::open(dir)?;
        let state = local_files::LocalFiles::new(&branch.common_dir()?);
        Ok(Bound {
            checkout: Unbuilt {
                seam: "place",
                profile: "worktrunk",
            },
            tests: Unbuilt {
                seam: "check",
                profile: "moon",
            },
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

impl Checkout for Unbuilt {
    fn materialize(&self, _: &str, _: &Rev) -> Result<Place> {
        self.refuse()
    }
    fn release(&self, _: &str) -> Result<()> {
        self.refuse()
    }
}

impl Tests for Unbuilt {
    fn start(&self, _: &RunRequest) -> Result<RunHandle> {
        self.refuse()
    }
    fn progress(&self, _: &RunHandle) -> Result<Progress> {
        self.refuse()
    }
    fn bound(&self) -> Result<usize> {
        self.refuse()
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
