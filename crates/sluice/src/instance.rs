//! The host's own settings, never a policy's: how many checks run on it
//! at once and how many cores each is given (71, S60; ADR 0012;
//! `docs/plans/phase-1-integrations.md` section 4), and the identity it
//! pushes as on a forge (C.4 pushed refs).
//!
//! `[runs] max` is the host's bound, shared by every repository on it and
//! every instance of sluice serving them: a run of any of them starts only
//! while the host's count of running checks, kept in its state
//! (`crate::host_count`), is under it. Each run is given the cores divided
//! by the bound, so the bound's runs together use what sluice may use and
//! no more.
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
//!
//! [signing]
//! signer = "box"
//! key = "~/.config/sluice/signing"
//!
//! [peers]
//! lease = "5m"
//! alive = "2m"
//! ```
//!
//! `reserved` keeps that many runs of the bound for submissions, which
//! session runs never take, so submissions' runs start first (session-runs-1 clause 91).
//! `[runs] systems` names the systems this host runs, of those the
//! policy names (85, 87); `[signing]` the signer it signs its runs as
//! and its SSH key (78); `[peers]` the lease and the bound on a host's
//! sign of life, the host's and never a branch's (74).

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instance {
    /// The most checks running on the host at once, every repository's
    /// together (71).
    pub bound: usize,
    /// The runs of the bound kept for submissions, which session runs never
    /// take (91).
    pub reserved: usize,
    /// The cores sluice may use: the process's affinity and its
    /// cgroup's quota where the host has them.
    pub processors: usize,
    /// The app sluice pushes as, where the host names one.
    pub forge: Option<Forge>,
    /// The signer this host signs its runs as, where it names one (78).
    pub signing: Option<Signing>,
    /// The lease and the sign-of-life bound (74).
    pub peers: Peers,
    /// The systems this host runs, by the policy's names; `None` for the
    /// one system it is (87).
    pub systems: Option<Vec<String>>,
}

/// The host's `[signing]` table: the policy's name for this host's key,
/// and the path of its SSH private key (78).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signing {
    pub signer: String,
    pub key: PathBuf,
}

/// The host's `[peers]` table (72 to 74, 77).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Peers {
    /// How long a branch's records stand unwritten before another
    /// instance may take it (72, 73).
    pub lease: Duration,
    /// How often this host shows it is alive; a claim lapses after it
    /// passes with no sign (77).
    pub alive: Duration,
    /// What this host is called among the hosts of a remote; its host
    /// name when unset.
    pub name: Option<String>,
}

impl Default for Peers {
    fn default() -> Self {
        Peers {
            lease: Duration::from_secs(300),
            alive: Duration::from_secs(120),
            name: None,
        }
    }
}

/// The operating system and architecture this host is, as the toolchain
/// names them (85): `macos`, `linux`; `aarch64`, `x86_64`.
pub fn this_system() -> (&'static str, &'static str) {
    (std::env::consts::OS, std::env::consts::ARCH)
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
    signing: Option<SigningForm>,
    #[serde(default)]
    peers: PeersForm,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SigningForm {
    signer: String,
    key: String,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct PeersForm {
    lease: Option<String>,
    alive: Option<String>,
    name: Option<String>,
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
    cores: Option<i64>,
    reserved: Option<i64>,
    systems: Option<Vec<String>>,
}

impl Instance {
    /// The instance's settings from its configuration file, or the
    /// defaults where it has none: a quarter of the cores sluice may use,
    /// never fewer than one.
    pub fn read() -> Result<Instance> {
        let (given, all) = host_cores();
        Instance::from_host(config_path().as_deref(), given, all)
    }

    /// The settings on a host that gives sluice all of its `processors`
    /// cores.
    pub fn from_file(path: Option<&Path>, processors: usize) -> Result<Instance> {
        Instance::from_host(path, processors, processors)
    }

    /// The settings on a host of `all` cores that gives sluice `given` of
    /// them: fewer where its affinity or quota limits sluice (71).
    pub fn from_host(path: Option<&Path>, given: usize, all: usize) -> Result<Instance> {
        let text = match path.map(std::fs::read_to_string) {
            Some(Ok(t)) => t,
            Some(Err(e)) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(e).with_context(|| format!("reading {}", path.unwrap().display()))
            }
            _ => String::new(),
        };
        let form: ConfigForm = toml::from_str(&text)
            .with_context(|| format!("reading {}", path.unwrap_or(Path::new("")).display()))?;
        // What sluice may use (71): the host's limit where it gives one, the
        // keeper's `cores` where it gives none, and all its cores otherwise.
        let processors = match form.runs.cores {
            None if given < all => given,
            None => all,
            Some(n) if n < 1 => bail!("[runs] cores must be at least 1, not {n}"),
            Some(n) if n as usize > all => {
                bail!("[runs] cores = {n} is more than this host's {all} cores")
            }
            Some(n) if given < all => bail!(
                "[runs] cores = {n}, but this host already gives sluice {given} of its {all} \
                 cores by its affinity or quota, and that is what sluice may use: leave cores out"
            ),
            Some(n) => n as usize,
        };
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
                "[runs] reserved = {n} leaves no run of [runs] max = {bound} for session runs: \
                 it must be less than the bound"
            ),
        };
        let forge = form
            .forge
            .map(|f| forge(f, home().as_deref()))
            .transpose()?;
        let signing = match form.signing {
            None => None,
            Some(s) => Some(Signing {
                key: private_key("[signing] key", &s.key, home().as_deref())?,
                signer: s.signer,
            }),
        };
        let d = Peers::default();
        let duration = |v: Option<String>, name: &str, default: Duration| -> Result<Duration> {
            match v {
                None => Ok(default),
                Some(t) => {
                    let d = sluice_core::policy::parse_duration(&t)
                        .with_context(|| format!("[peers] {name}"))?;
                    if d.is_zero() {
                        bail!("[peers] {name} must be longer than nothing");
                    }
                    Ok(d)
                }
            }
        };
        if let Some(n) = &form.peers.name {
            if n.is_empty() || n.contains(['/', ' ']) {
                bail!("[peers] name {n:?} must be one word with no /");
            }
        }
        let peers = Peers {
            lease: duration(form.peers.lease, "lease", d.lease)?,
            alive: duration(form.peers.alive, "alive", d.alive)?,
            name: form.peers.name,
        };
        if peers.alive >= peers.lease {
            bail!(
                "[peers] alive must be shorter than lease, so a lead that is alive keeps its branch"
            );
        }
        let systems = match form.runs.systems {
            Some(s) if s.is_empty() => {
                bail!("[runs] systems names no system; leave it out for the one this host is")
            }
            s => s,
        };
        Ok(Instance {
            bound,
            reserved,
            processors,
            forge,
            signing,
            peers,
            systems,
        })
    }

    /// The cores each run is given, so the host's `bound` runs at once,
    /// whatever repositories they are of, use what sluice may use and no
    /// more (71).
    pub fn jobs(&self) -> usize {
        (self.processors / self.bound).max(1)
    }
}

/// The `[forge]` table, its key's path with `~` expanded. A key anyone
/// but its owner may read is refused.
fn forge(form: ForgeForm, home: Option<&Path>) -> Result<Forge> {
    Ok(Forge {
        app_id: form.app_id,
        key: private_key("[forge] key", &form.key, home)?,
    })
}

/// A private key's path, `~` expanded; refused when anyone but its owner
/// may read it.
fn private_key(what: &str, path: &str, home: Option<&Path>) -> Result<PathBuf> {
    let key = match (path.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => home.join(rest),
        (Some(_), None) => bail!("{what} {path}: no home directory to expand ~ in"),
        (None, _) => PathBuf::from(path),
    };
    let meta = std::fs::metadata(&key).with_context(|| format!("{what} {}", key.display()))?;
    let mode = std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o777;
    if mode & 0o077 != 0 {
        bail!(
            "{what} {} has mode {mode:04o}, which lets others read it: \
             give it mode 0600 (chmod 600 {})",
            key.display(),
            key.display()
        );
    }
    Ok(key)
}

/// The cores the host gives sluice and all the host's cores: its affinity
/// and cgroup quota, which macOS has neither of, and its online logical
/// CPUs, taken as the first where the host cannot say or says fewer (71).
pub fn host_cores() -> (usize, usize) {
    let given = std::thread::available_parallelism().map_or(1, |n| n.get());
    // SAFETY: sysconf reads a value and touches no memory of ours.
    let online = unsafe { nix::libc::sysconf(nix::libc::_SC_NPROCESSORS_ONLN) };
    let all = usize::try_from(online).map_or(given, |n| n.max(given));
    (given, all)
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
        on(text, processors, processors)
    }

    /// The settings `text` gives on a host of `all` cores that gives sluice
    /// `given` of them.
    fn on(text: &str, given: usize, all: usize) -> Result<Instance> {
        static FILES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("sluice-instance-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let n = FILES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = dir.join(format!("{n}.toml"));
        std::fs::write(&path, text).unwrap();
        let out = Instance::from_host(Some(&path), given, all);
        let _ = std::fs::remove_file(&path);
        out
    }

    /// 71: the bound is the host's, a quarter of its cores unless its
    /// configuration says otherwise; each run gets an equal share.
    #[test]
    fn the_bound_and_each_runs_share() {
        let none = Instance::from_file(None, 16).unwrap();
        assert_eq!((none.bound, none.jobs()), (4, 4));
        assert_eq!(Instance::from_file(None, 2).unwrap().bound, 1);
        let two = with("[runs]\nmax = 2\n", 16).unwrap();
        assert_eq!((two.bound, two.jobs()), (2, 8));
        // S60: eighteen cores and a bound of three, six cores each.
        let s60 = with("[runs]\nmax = 3\n", 18).unwrap();
        assert_eq!((s60.bound, s60.jobs()), (3, 6));
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

    /// 71: what sluice may use is the host's limit where it gives one, the
    /// keeper's `cores` where it gives none, and all its cores otherwise;
    /// each run's share and the default bound follow from it.
    #[test]
    fn the_cores_sluice_may_use() {
        // A Mac keeps six cores for its agents.
        let mac = on("[runs]\nmax = 3\ncores = 12\n", 18, 18).unwrap();
        assert_eq!((mac.processors, mac.bound, mac.jobs()), (12, 3, 4));
        // Stated cores and no bound: a quarter of them.
        let eight = on("[runs]\ncores = 8\n", 18, 18).unwrap();
        assert_eq!((eight.bound, eight.jobs()), (2, 4));
        // A quota of six of sixteen cores, and nothing stated.
        let quota = on("[runs]\nmax = 2\n", 6, 16).unwrap();
        assert_eq!((quota.processors, quota.jobs()), (6, 3));
        assert_eq!(on("", 6, 16).unwrap().bound, 1);
        // A Mac with nothing stated, as before.
        let s60 = on("[runs]\nmax = 3\n", 18, 18).unwrap();
        assert_eq!((s60.processors, s60.jobs()), (18, 6));
    }

    /// 71: a `cores` sluice cannot honour is refused, naming the setting
    /// and the host's cores.
    #[test]
    fn stated_cores_sluice_cannot_honour() {
        let err = |text: &str, given, all| on(text, given, all).unwrap_err().to_string();
        assert_eq!(
            err("[runs]\ncores = 0\n", 18, 18),
            "[runs] cores must be at least 1, not 0"
        );
        assert_eq!(
            err("[runs]\ncores = 24\n", 18, 18),
            "[runs] cores = 24 is more than this host's 18 cores"
        );
        assert_eq!(
            err("[runs]\ncores = 4\n", 6, 16),
            "[runs] cores = 4, but this host already gives sluice 6 of its 16 cores by its \
             affinity or quota, and that is what sluice may use: leave cores out"
        );
    }

    /// The two counts `read` takes: the host never gives sluice more cores
    /// than it has, and a Mac gives it all of them (71).
    #[test]
    fn the_hosts_cores() {
        let (given, all) = host_cores();
        assert!(1 <= given && given <= all, "{given} of {all}");
        if cfg!(target_os = "macos") {
            assert_eq!(given, all);
        }
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

    /// `[signing]`, `[peers]` and `[runs] systems` (74, 78, 87).
    #[test]
    fn signing_peers_and_systems() {
        let dir = std::env::temp_dir().join(format!("sluice-signing-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let key = dir.join("signing");
        std::fs::write(&key, "key").unwrap();
        let set = |mode: u32| {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(mode)).unwrap();
        };
        let table = format!(
            "[signing]\nsigner = \"box\"\nkey = {:?}\n",
            key.display().to_string()
        );
        set(0o600);
        let i = with(&table, 4).unwrap();
        assert_eq!(
            i.signing,
            Some(Signing {
                signer: "box".into(),
                key: key.clone()
            })
        );
        set(0o644);
        let err = format!("{:#}", with(&table, 4).unwrap_err());
        assert!(
            err.contains("[signing] key") && err.contains("0600"),
            "{err}"
        );

        let none = Instance::from_file(None, 4).unwrap();
        assert_eq!(none.signing, None);
        assert_eq!(none.peers, Peers::default());
        assert_eq!(none.peers.lease, Duration::from_secs(300));
        assert_eq!(none.peers.alive, Duration::from_secs(120));
        assert_eq!(none.systems, None);
        let p = with("[peers]\nlease = \"10m\"\nalive = \"30s\"\n", 4).unwrap();
        assert_eq!(
            (p.peers.lease, p.peers.alive),
            (Duration::from_secs(600), Duration::from_secs(30))
        );
        assert!(with("[peers]\nlease = \"1m\"\nalive = \"2m\"\n", 4).is_err());
        assert!(with("[peers]\nlease = \"soon\"\n", 4).is_err());
        let s = with("[runs]\nsystems = [\"linux-x64\"]\n", 4).unwrap();
        assert_eq!(s.systems, Some(vec!["linux-x64".to_string()]));
        assert!(with("[runs]\nsystems = []\n", 4).is_err());
        let (os, arch) = this_system();
        assert!(["macos", "linux"].contains(&os), "{os}");
        assert!(["aarch64", "x86_64"].contains(&arch), "{arch}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
