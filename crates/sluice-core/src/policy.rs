//! The policy: `.config/sluice.toml`, read from the line's current
//! revision (56, 27; ADR 0007; rule core.15).
//!
//! The core gives `[tests]` and `[checkout]` no meaning: they name the
//! profiles that bind the check and place seams, which the binary chooses
//! (52; rule core.1). The core reads only the `[branches]` tables, the
//! gate of each line.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

/// Where the policy lives in a line's tree.
pub const POLICY_PATH: &str = ".config/sluice.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    pub tests: Profile,
    pub checkout: Profile,
    /// Patterns of line names and their gates, in the order written.
    pub branches: Vec<(String, Rules)>,
}

/// A profile's name and its own settings, uninterpreted by the core.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Profile {
    pub name: Option<String>,
    pub settings: BTreeMap<String, String>,
}

/// How a flaky key reads (19).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flaky {
    Pass,
    Fail,
}

/// One line's gate (27).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rules {
    pub required: Vec<String>,
    /// The window's upper bound; its lower bound is one (13).
    pub parallel: usize,
    /// Deepest first, or every candidate (14).
    pub batch: bool,
    pub flaky: Flaky,
    /// How long one candidate's checks may run (66). `None` when the
    /// policy states no limit.
    pub timeout: Option<Duration>,
}

impl Default for Rules {
    /// The bolt line's defaults of ADR 0007: every candidate, a window of
    /// one to four, a flaky key read as passed.
    fn default() -> Self {
        Rules {
            required: Vec::new(),
            parallel: 4,
            batch: false,
            flaky: Flaky::Pass,
            timeout: None,
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
    branches: BTreeMap<String, RulesForm>,
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
        let tests = profile(form.tests, "runner", "[tests]")?;
        let checkout = profile(form.checkout, "method", "[checkout]")?;
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
                None => None,
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
            branches,
        })
    }

    /// The gate for `branch`: an exact name before any pattern, then the
    /// pattern with the most literal characters. `None` when no table
    /// names the line, which is then not sluice's to land on.
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

fn profile(table: BTreeMap<String, toml::Value>, key: &str, section: &str) -> Result<Profile> {
    let mut p = Profile::default();
    for (k, v) in table {
        let s = match v {
            toml::Value::String(s) => s,
            other => bail!("{section} {k} must be a string, not {other}"),
        };
        if k == key {
            p.name = Some(s);
        } else {
            p.settings.insert(k, s);
        }
    }
    Ok(p)
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
        assert_eq!(p.tests.name.as_deref(), Some("script"));
        assert_eq!(p.tests.settings["command"], "tests/run");
        assert_eq!(p.checkout.name.as_deref(), Some("scratch"));
        let bolt = p.rules_for("bolt/concurrent-units").unwrap().unwrap();
        assert_eq!(bolt.required, vec!["#gate"]);
        assert_eq!(bolt.parallel, 4);
        assert!(!bolt.batch);
        assert_eq!(bolt.flaky, Flaky::Fail);
        assert_eq!(bolt.timeout, Some(Duration::from_secs(1200)));
        let main = p.rules_for("main").unwrap().unwrap();
        assert!(main.batch);
        assert_eq!(main.parallel, 2);
        assert_eq!(main.flaky, Flaky::Pass);
        assert_eq!(main.timeout, None);
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
