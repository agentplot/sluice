//! The policy: `.config/sluice.toml`, read from the branch's current
//! revision (56, 27; ADR 0007; rule core.15).
//!
//! The core gives `[tests]` and `[checkout]` no meaning: they name the
//! profiles that bind the check and place seams and their settings, which
//! the core passes on (52; rule core.1). It reads the `[branches]` tables,
//! the gate of each branch, and `[executor]`, the limits a run is held to
//! (83).

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use sluice_seams::{Limits, Setting, Settings};

/// Where the policy lives in a branch's tree.
pub const POLICY_PATH: &str = ".config/sluice.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    /// `[tests]`, for the check seam's profile.
    pub tests: Settings,
    /// `[checkout]`, for the place seam's profile.
    pub checkout: Settings,
    /// `[executor]`: what every run of the repository may use and reach.
    pub limits: Limits,
    /// Patterns of branch names and their gates, in the order written.
    pub branches: Vec<(String, Rules)>,
}

/// How a flaky key reads (19).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flaky {
    Pass,
    Fail,
}

/// One branch's gate (27).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rules {
    pub required: Vec<String>,
    /// The window's upper bound; its lower bound is one (13).
    pub parallel: usize,
    /// Deepest first, or every candidate (14).
    pub batch: bool,
    pub flaky: Flaky,
    /// How long one candidate's checks may run (66).
    pub timeout: Option<Duration>,
}

impl Default for Rules {
    /// What a table leaves out, always the conservative reading: every
    /// candidate, a window of one to four, a flaky key read as failed, an
    /// hour per candidate.
    fn default() -> Self {
        Rules {
            required: Vec::new(),
            parallel: 4,
            batch: false,
            flaky: Flaky::Fail,
            timeout: Some(Duration::from_secs(3600)),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileForm {
    #[serde(default)]
    tests: BTreeMap<String, toml::Value>,
    #[serde(default)]
    checkout: BTreeMap<String, toml::Value>,
    #[serde(default)]
    executor: ExecutorForm,
    #[serde(default)]
    branches: BTreeMap<String, RulesForm>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ExecutorForm {
    kind: Option<String>,
    memory: Option<String>,
    network: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesForm {
    #[serde(default)]
    required: Vec<String>,
    parallel: Option<i64>,
    batch: Option<bool>,
    flaky: Option<String>,
    timeout: Option<String>,
}

impl Policy {
    pub fn parse(text: &str) -> Result<Policy> {
        let form: FileForm = toml::from_str(text).context("reading .config/sluice.toml")?;
        let tests = settings(form.tests, "[tests]")?;
        let checkout = settings(form.checkout, "[checkout]")?;
        let limits = Limits {
            executor: form.executor.kind,
            memory: match form.executor.memory.as_deref() {
                None => None,
                Some(m) => Some(parse_size(m).context("[executor] memory")?),
            },
            network: form.executor.network,
        };
        let mut branches = Vec::new();
        for (pattern, r) in form.branches {
            let d = Rules::default();
            let parallel = match r.parallel {
                None => d.parallel,
                Some(n) if n >= 1 => n as usize,
                Some(n) => bail!("[branches.\"{pattern}\"] parallel must be at least 1, not {n}"),
            };
            let flaky = match r.flaky.as_deref() {
                None => d.flaky,
                Some("pass") => Flaky::Pass,
                Some("fail") => Flaky::Fail,
                Some(other) => {
                    bail!("[branches.\"{pattern}\"] flaky must be \"pass\" or \"fail\", not {other:?}")
                }
            };
            let timeout = match r.timeout.as_deref() {
                None => d.timeout,
                Some(t) => Some(
                    parse_duration(t)
                        .with_context(|| format!("[branches.\"{pattern}\"] timeout"))?,
                ),
            };
            branches.push((
                pattern,
                Rules {
                    required: r.required,
                    parallel,
                    batch: r.batch.unwrap_or(d.batch),
                    flaky,
                    timeout,
                },
            ));
        }
        Ok(Policy {
            tests,
            checkout,
            limits,
            branches,
        })
    }

    /// The gate for `branch`: an exact name before any pattern, then the
    /// pattern with the most literal characters. `None` when no table
    /// names the branch, which is then not sluice's to land on.
    pub fn rules_for(&self, branch: &str) -> Result<Option<&Rules>> {
        if let Some((_, r)) = self.branches.iter().find(|(p, _)| p == branch) {
            return Ok(Some(r));
        }
        let mut best: Option<(usize, &String, &Rules)> = None;
        let mut tied = false;
        for (p, r) in &self.branches {
            if !p.contains('*') || !glob(p, branch) {
                continue;
            }
            let weight = p.chars().filter(|c| *c != '*').count();
            match best {
                Some((w, _, _)) if w > weight => {}
                Some((w, _, _)) if w == weight => tied = true,
                _ => {
                    best = Some((weight, p, r));
                    tied = false;
                }
            }
        }
        if tied {
            let (_, p, _) = best.expect("a tie has a best");
            bail!("{branch} matches more than one [branches] pattern as closely as {p:?}");
        }
        Ok(best.map(|(_, _, r)| r))
    }
}

/// A profile's table: each value a string or a list of strings.
fn settings(table: BTreeMap<String, toml::Value>, section: &str) -> Result<Settings> {
    let mut out = Settings::new();
    for (k, v) in table {
        let setting = match v {
            toml::Value::String(s) => Setting::Text(s),
            toml::Value::Array(xs) => Setting::List(
                xs.into_iter()
                    .map(|x| match x {
                        toml::Value::String(s) => Ok(s),
                        other => bail!("{section} {k} must list strings, not {other}"),
                    })
                    .collect::<Result<_>>()?,
            ),
            other => bail!("{section} {k} must be a string or a list of strings, not {other}"),
        };
        out.insert(k, setting);
    }
    Ok(out)
}

/// `"512M"`, `"8G"`: bytes, in powers of 1024.
pub fn parse_size(text: &str) -> Result<u64> {
    let t = text.trim();
    let (digits, unit) = t.split_at(t.find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len()));
    let n: u64 = digits
        .parse()
        .map_err(|_| anyhow!("{text:?} is not a size such as \"8G\""))?;
    let scale: u64 = match unit {
        "K" => 1 << 10,
        "M" => 1 << 20,
        "G" => 1 << 30,
        "T" => 1 << 40,
        _ => bail!("{text:?} is not a size such as \"8G\""),
    };
    n.checked_mul(scale)
        .ok_or_else(|| anyhow!("{text:?} is too large"))
}

/// `*` matches any run of characters but `/`.
fn glob(pattern: &str, name: &str) -> bool {
    fn go(p: &[u8], n: &[u8]) -> bool {
        match p.split_first() {
            None => n.is_empty(),
            Some((b'*', rest)) => {
                let mut i = 0;
                loop {
                    if go(rest, &n[i..]) {
                        return true;
                    }
                    if i == n.len() || n[i] == b'/' {
                        return false;
                    }
                    i += 1;
                }
            }
            Some((c, rest)) => n.first() == Some(c) && go(rest, &n[1..]),
        }
    }
    go(pattern.as_bytes(), name.as_bytes())
}

/// `"20m"`, `"90s"`, `"1h30m"`.
pub fn parse_duration(text: &str) -> Result<Duration> {
    let mut total = 0u64;
    let mut digits = String::new();
    let mut any = false;
    for c in text.trim().chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        let n: u64 = digits
            .parse()
            .map_err(|_| anyhow!("{text:?} is not a duration such as \"20m\""))?;
        digits.clear();
        total += n * match c {
            's' => 1,
            'm' => 60,
            'h' => 3600,
            _ => bail!("{text:?} is not a duration such as \"20m\""),
        };
        any = true;
    }
    if !digits.is_empty() || !any {
        bail!("{text:?} is not a duration such as \"20m\"");
    }
    Ok(Duration::from_secs(total))
}

#[cfg(test)]
mod tests {
    use super::*;

    const POLICY: &str = r##"
[tests]
runner = "script"
command = "tests/run"

[checkout]
method = "scratch"

[branches."bolt/*"]
required = ["#gate"]
parallel = 4
batch = false
flaky = "fail"
timeout = "20m"

[branches.main]
required = ["#gate", "#live"]
parallel = 2
batch = true
"##;

    #[test]
    fn reads_the_walkthrough_shape() {
        let p = Policy::parse(POLICY).unwrap();
        assert_eq!(p.tests["runner"], Setting::Text("script".into()));
        assert_eq!(p.tests["command"], Setting::Text("tests/run".into()));
        assert_eq!(p.checkout["method"], Setting::Text("scratch".into()));
        assert_eq!(p.limits, Limits::default());
        let bolt = p.rules_for("bolt/concurrent-units").unwrap().unwrap();
        assert_eq!(bolt.required, vec!["#gate"]);
        assert_eq!(bolt.parallel, 4);
        assert!(!bolt.batch);
        assert_eq!(bolt.flaky, Flaky::Fail);
        assert_eq!(bolt.timeout, Some(Duration::from_secs(1200)));
        let main = p.rules_for("main").unwrap().unwrap();
        assert!(main.batch);
        assert_eq!(main.parallel, 2);
        assert_eq!(main.flaky, Flaky::Fail);
        assert_eq!(main.timeout, Some(Duration::from_secs(3600)));
        assert!(p.rules_for("unit/x").unwrap().is_none());
        assert!(p.rules_for("bolt/a/b").unwrap().is_none());
    }

    #[test]
    fn refuses_what_it_does_not_know() {
        assert!(Policy::parse("[branches.main]\nwindow = 3\n").is_err());
        assert!(Policy::parse("[branches.main]\nparallel = 0\n").is_err());
        assert!(Policy::parse("[branches.main]\nflaky = \"retry\"\n").is_err());
        assert!(Policy::parse("[branches.main]\ntimeout = \"soon\"\n").is_err());
    }

    /// 83: the limits a run is held to, and the settings passed on.
    #[test]
    fn reads_the_executor_and_the_shell() {
        let p = Policy::parse(
            "[tests]\nrunner = \"tasks\"\nshell = [\"env-shell\", \"--\"]\n\
             [executor]\nkind = \"sandbox\"\nnetwork = false\nmemory = \"8G\"\n",
        )
        .unwrap();
        assert_eq!(
            p.tests["shell"],
            Setting::List(vec!["env-shell".into(), "--".into()])
        );
        assert_eq!(
            p.limits,
            Limits {
                executor: Some("sandbox".into()),
                memory: Some(8 << 30),
                network: Some(false),
            }
        );
        assert!(Policy::parse("[executor]\ncpus = 2\n").is_err());
        assert!(Policy::parse("[executor]\nmemory = \"lots\"\n").is_err());
        assert!(Policy::parse("[executor]\nmemory = \"8\"\n").is_err());
        assert!(Policy::parse("[tests]\nshell = [1]\n").is_err());
        assert!(Policy::parse("[tests]\njobs = 2\n").is_err());
    }

    #[test]
    fn sizes() {
        assert_eq!(parse_size("512M").unwrap(), 512 << 20);
        assert_eq!(parse_size("1G").unwrap(), 1 << 30);
        assert!(parse_size("1GB").is_err());
        assert!(parse_size("G").is_err());
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("20m").unwrap(), Duration::from_secs(1200));
        assert_eq!(parse_duration("1h30m").unwrap(), Duration::from_secs(5400));
        assert_eq!(parse_duration("45s").unwrap(), Duration::from_secs(45));
        assert!(parse_duration("20").is_err());
    }

    #[test]
    fn an_exact_name_wins_over_a_pattern() {
        let p = Policy::parse(
            "[branches.\"bolt/*\"]\nparallel = 3\n[branches.\"bolt/x\"]\nparallel = 1\n",
        )
        .unwrap();
        assert_eq!(p.rules_for("bolt/x").unwrap().unwrap().parallel, 1);
        assert_eq!(p.rules_for("bolt/y").unwrap().unwrap().parallel, 3);
    }
}
