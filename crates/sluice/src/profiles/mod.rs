//! Binding each seam to a profile (52; ADR 0003). The only directory of
//! this crate that may name a tool (rule core.1).
//!
//! Phase 1 builds the core against fakes; the real profiles are pass 2
//! (`docs/plans/phase-1-integrations.md`). Until then every seam is bound
//! to [`Unbuilt`], which refuses every call by naming the profile the
//! policy asked for, so the CLI's arguments, output and exits are whole
//! and only the seams are missing.

pub mod worktrunk;

use anyhow::{bail, Result};
use sluice_seams::{
    Affected, Branch, Changes, Checkout, Driver, Formed, Guard, LandingCommit, Moved,
    Notification, Place, Progress, Rev, RunHandle, RunRequest, Session, Snapshot, State, Tests,
    Written,
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

/// The phase-1 binding: the profiles the walkthrough's setup names, none
/// built yet.
pub struct Bound {
    pub checkout: Unbuilt,
    pub tests: Unbuilt,
    pub branch: Unbuilt,
    pub driver: Unbuilt,
    pub state: Unbuilt,
}

impl Default for Bound {
    fn default() -> Self {
        Bound {
            checkout: Unbuilt { seam: "place", profile: "worktrunk" },
            tests: Unbuilt { seam: "check", profile: "moon" },
            branch: Unbuilt { seam: "line", profile: "local refs (gix and git)" },
            driver: Unbuilt { seam: "driver", profile: "worktrunk" },
            state: Unbuilt { seam: "records", profile: "local files" },
        }
    }
}

impl Bound {
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
    fn affected(&self, _: &Rev, _: &Rev) -> Result<Affected> {
        self.refuse()
    }
    fn start(&self, _: &RunRequest) -> Result<RunHandle> {
        self.refuse()
    }
    fn progress(&self, _: &RunHandle) -> Result<Progress> {
        self.refuse()
    }
    fn capacity(&self) -> Result<usize> {
        self.refuse()
    }
}

impl Branch for Unbuilt {
    fn read(&self, _: &str) -> Result<Option<Rev>> {
        self.refuse()
    }
    fn read_file(&self, _: &Rev, _: &str) -> Result<Option<Vec<u8>>> {
        self.refuse()
    }
    fn form(&self, _: &Rev, _: &Rev) -> Result<Formed> {
        self.refuse()
    }
    fn commit(&self, _: &LandingCommit) -> Result<Rev> {
        self.refuse()
    }
    fn move_to(&self, _: &str, _: &Rev, _: &Rev) -> Result<Moved> {
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

impl State for Unbuilt {
    fn lock<'a>(&'a self, _: &str) -> Result<Option<Box<dyn Guard + 'a>>> {
        self.refuse()
    }
    fn read(&self, _: &str) -> Result<Snapshot> {
        self.refuse()
    }
    fn write(&self, _: &str, _: u64, _: Changes) -> Result<Written> {
        self.refuse()
    }
    fn branches(&self) -> Result<Vec<String>> {
        self.refuse()
    }
    fn allocate_id(&self) -> Result<u64> {
        self.refuse()
    }
    fn results(&self) -> Result<Vec<u8>> {
        self.refuse()
    }
    fn record_results(&self, _: &[u8]) -> Result<()> {
        self.refuse()
    }
}
