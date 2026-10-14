//! The instance's own settings, never a policy's: how many runs it has
//! under test at once and how many processors each is given (71; ADR
//! 0012; `docs/plans/phase-1-integrations.md` section 4), and the
//! identity it pushes as on a forge (C.4 pushed refs).
//!
//! `$XDG_CONFIG_HOME/sluice/config.toml` (default `~/.config`):
//!
//! ```toml
//! [runs]
//! max = 2
//! reserved = 1
//!
//! [forge]
//! app_id = 123456
//! key = "~/.config/sluice/app.pem"
//! ```
//!
//! `reserved` is how many of the bound's slots session runs never take,
//! so submissions' runs start first (session-runs-1 clause 91).

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instance {
    /// The most runs under test at once.
    pub bound: usize,
    /// The slots of the bound that session runs never take (91).
    pub reserved: usize,
    /// The processors sluice may use: the process's affinity and its
    /// cgroup's quota where the host has them.
    pub processors: usize,
    /// The app sluice pushes as, where the host names one.
    pub forge: Option<Forge>,
}

/// The host's `[forge]` table: the app sluice is on the forge, and the
/// path of its private key on this host, which no repository holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forge {
    pub app_id: u64,
    pub key: PathBuf,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ConfigForm {
    #[serde(default)]
    runs: RunsForm,
    forge: Option<ForgeForm>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ForgeForm {
    app_id: u64,
    key: String,
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
        let forge = form
            .forge
            .map(|f| forge(f, home().as_deref()))
            .transpose()?;
        Ok(Instance {
            bound,
            reserved,
            processors,
            forge,
        })
    }

    /// The processors each run is given, so `bound` runs at once use what
    /// sluice may use and no more.
    pub fn jobs(&self) -> usize {
        (self.processors / self.bound).max(1)
    }
}

/// The `[forge]` table, its key's path with `~` expanded. A key anyone
/// but its owner may read is refused.
fn forge(form: ForgeForm, home: Option<&Path>) -> Result<Forge> {
    let key = match (form.key.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => home.join(rest),
        (Some(_), None) => bail!("[forge] key {}: no home directory to expand ~ in", form.key),
        (None, _) => PathBuf::from(&form.key),
    };
    let meta = std::fs::metadata(&key).with_context(|| format!("[forge] key {}", key.display()))?;
    let mode = std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o777;
    if mode & 0o077 != 0 {
        bail!(
            "[forge] key {} has mode {mode:04o}, which lets others read it: \
             give it mode 0600 (chmod 600 {})",
            key.display(),
            key.display()
        );
    }
    Ok(Forge {
        app_id: form.app_id,
        key,
    })
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

/// The host's own state, shared by every repository on it:
/// `$XDG_STATE_HOME/sluice` (default `~/.local/state/sluice`).
pub fn state_dir() -> Result<PathBuf> {
    let state = match std::env::var_os("XDG_STATE_HOME") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?)
            .join(".local")
            .join("state"),
    };
    Ok(state.join("sluice"))
}

fn config_path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => home()?.join(".config"),
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

    /// The `[forge]` table: absent, present, and a key others may read
    /// refused with the file and the mode it needs.
    #[test]
    fn the_forge_table_and_its_key() {
        let dir = std::env::temp_dir().join(format!("sluice-forge-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let key = dir.join("app.pem");
        std::fs::write(&key, "key").unwrap();
        let set = |mode: u32| {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(mode)).unwrap();
        };
        let table = format!(
            "[forge]\napp_id = 42\nkey = {:?}\n",
            key.display().to_string()
        );

        assert_eq!(with("[runs]\nmax = 1\n", 4).unwrap().forge, None);

        set(0o600);
        assert_eq!(
            with(&table, 4).unwrap().forge,
            Some(Forge {
                app_id: 42,
                key: key.clone()
            })
        );

        set(0o644);
        let err = format!("{:#}", with(&table, 4).unwrap_err());
        assert!(err.contains(&key.display().to_string()), "{err}");
        assert!(err.contains("0600"), "{err}");

        set(0o640);
        assert!(with(&table, 4).is_err());

        set(0o400);
        let home = forge(
            ForgeForm {
                app_id: 1,
                key: "~/app.pem".into(),
            },
            Some(&dir),
        )
        .unwrap();
        assert_eq!(home.key, key);
        assert!(with("[forge]\napp_id = 1\n", 4).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
