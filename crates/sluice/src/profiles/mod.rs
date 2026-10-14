//! Binding each seam to a profile (52; ADR 0003). The only directory of
//! this crate that may name a tool (rule core.1).
//!
//! The profiles are `docs/plans/phase-1-integrations.md`'s: the branch is
//! bound to [`local_refs`], or [`pushed_refs`] where its policy names a
//! remote, through [`branches`]; the records to [`local_files`], the check to
//! [`checks`], the place to [`places`] and the driver to [`worktrunk`].

pub mod branches;
pub mod checks;
pub mod local_files;
pub mod local_refs;
pub mod moon;
pub mod places;
pub mod pushed_refs;
pub mod worktrunk;

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::instance::Instance;

/// The profiles the walkthrough's setup names, for the repository holding
/// a directory.
pub struct Bound {
    pub checkout: places::Places,
    pub tests: checks::Checks,
    pub branch: branches::Branches,
    pub driver: worktrunk::Worktrunk,
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
        let branch = branches::Branches::open(dir)?;
        let common = branch.common_dir()?;
        let state = local_files::LocalFiles::new(&common);
        Ok(Bound {
            checkout: places::Places::new(
                &common,
                crate::runs::Runs::new(&common, program.clone()),
            ),
            tests: checks::Checks::new(&common, program, instance),
            branch,
            driver: worktrunk::Worktrunk,
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

impl crate::exec::Host for Bound {
    fn reserved(&self) -> usize {
        self.tests.instance().reserved
    }

    fn worktree(&self, dir: &str) -> Result<String> {
        let path = std::fs::canonicalize(dir)?;
        path.to_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("{} is not UTF-8", path.display()))
    }

    fn lock_sessions(&self) -> Result<Box<dyn std::any::Any>> {
        Ok(Box::new(self.tests.host_sessions()?.lock()?))
    }

    fn begin_session(
        &self,
        held: &dyn std::any::Any,
        run: &crate::sessions::SessionRun,
    ) -> Result<()> {
        let held = held
            .downcast_ref::<crate::sessions::SessionsLock>()
            .ok_or_else(|| anyhow::anyhow!("the host's session runs are not locked"))?;
        self.tests.host_sessions()?.begin(held, run)
    }

    fn end_session(&self, worktree: &str) -> Result<()> {
        self.tests.host_sessions()?.end(worktree)
    }

    fn run_dir(&self, run: &sluice_seams::RunHandle) -> Option<PathBuf> {
        Some(self.tests.runs().dir(&run.0))
    }

    fn end_run(&self, run: &sluice_seams::RunHandle) -> Result<()> {
        self.tests.runs().end(&run.0)
    }
}
