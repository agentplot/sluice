//! Binding each seam to a profile (52; ADR 0003). The only directory of
//! this crate that may name a tool (rule core.1).
//!
//! The profiles are `docs/plans/phase-1-integrations.md`'s: the branch is
//! bound to [`local_refs`], or [`pushed_refs`] where its policy names a
//! remote, through [`branches`]; the records to [`local_files`], the check to
//! [`checks`], the place to [`places`] and the driver to [`worktrunk`].

pub mod branches;
pub mod checks;
pub mod github;
pub mod local_files;
pub mod local_refs;
pub mod moon;
pub mod places;
pub mod pushed_refs;
pub mod worktrunk;

use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Result;

use crate::identity::Identity;
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
        // C.4: the app the host names is sluice's identity on GitHub.
        let identity = match &instance.forge {
            Some(f) => Some((
                Rc::new(github::App::open(f)?) as Rc<dyn Identity>,
                program.clone(),
            )),
            None => None,
        };
        let branch = branches::Branches::open_as(dir, identity)?;
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
