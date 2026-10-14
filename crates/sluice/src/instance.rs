//! The instance's own settings, never a policy's: how many runs it has
//! under test at once and how many processors each is given (71; ADR
//! 0012; `docs/plans/phase-1-integrations.md` section 4).
//!
//! `$XDG_CONFIG_HOME/sluice/config.toml` (default `~/.config`):
//!
//! ```toml
//! [runs]
//! max = 2
//! reserved = 1
//! ```
//!
//! `reserved` is how many of the bound's slots session runs never take,
//! so submissions' runs start first (session-runs-1 clause 91).

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instance {
    /// The most runs under test at once.
    pub bound: usize,
    /// The slots of the bound that session runs never take (91).
    pub reserved: usize,
    /// The processors sluice may use: the process's affinity and its
    /// cgroup's quota where the host has them.
    pub processors: usize,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ConfigForm {
    #[serde(default)]
    runs: RunsForm,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RunsForm {
    max: Option<i64>,
    reserved: Option<i64>,
}

impl Instance {
    /// The instance's settings from its configuration file, or the
    /// defaults where it has none: a quarter of the processors, never
    /// fewer than one.
    pub fn read() -> Result<Instance> {
        let processors = std::thread::available_parallelism().map_or(1, |n| n.get());
        Instance::from_file(config_path().as_deref(), processors)
    }

    pub fn from_file(path: Option<&Path>, processors: usize) -> Result<Instance> {
        let text = match path.map(std::fs::read_to_string) {
            Some(Ok(t)) => t,
            Some(Err(e)) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(e).with_context(|| format!("reading {}", path.unwrap().display()))
            }
            _ => String::new(),
        };
        let form: ConfigForm = toml::from_str(&text)
            .with_context(|| format!("reading {}", path.unwrap_or(Path::new("")).display()))?;
        let bound = match form.runs.max {
            None => (processors / 4).max(1),
            Some(n) if n >= 1 => n as usize,
            Some(n) => bail!("[runs] max must be at least 1, not {n}"),
        };
        let reserved = match form.runs.reserved {
            None => 0,
            Some(n) if n >= 0 && (n as usize) < bound => n as usize,
            Some(n) if n < 0 => bail!("[runs] reserved must be at least 0, not {n}"),
            Some(n) => bail!(
                "[runs] reserved = {n} leaves no slot of [runs] max = {bound} for session runs: \
                 it must be less than the bound"
            ),
        };
        Ok(Instance {
            bound,
            reserved,
            processors,
        })
    }

    /// The processors each run is given, so `bound` runs at once use what
    /// sluice may use and no more.
    pub fn jobs(&self) -> usize {
        (self.processors / self.bound).max(1)
    }
}

fn config_path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(base.join("sluice").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(text: &str, processors: usize) -> Result<Instance> {
        let dir = std::env::temp_dir().join(format!("sluice-instance-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{processors}-{}.toml", text.len()));
        std::fs::write(&path, text).unwrap();
        let out = Instance::from_file(Some(&path), processors);
        let _ = std::fs::remove_file(&path);
        out
    }

    /// 71: the bound is the instance's, a quarter of its processors unless
    /// its configuration says otherwise; each run gets an equal share.
    #[test]
    fn the_bound_and_each_runs_share() {
        let none = Instance::from_file(None, 16).unwrap();
        assert_eq!((none.bound, none.jobs()), (4, 4));
        assert_eq!(Instance::from_file(None, 2).unwrap().bound, 1);
        let two = with("[runs]\nmax = 2\n", 16).unwrap();
        assert_eq!((two.bound, two.jobs()), (2, 8));
        assert_eq!(with("[runs]\nmax = 32\n", 16).unwrap().jobs(), 1);
        assert!(with("[runs]\nmax = 0\n", 16).is_err());
        assert!(with("[runs]\nmin = 1\n", 16).is_err());
        assert_eq!(
            Instance::from_file(Some(Path::new("/nonexistent/sluice.toml")), 8)
                .unwrap()
                .bound,
            2
        );
    }
}
