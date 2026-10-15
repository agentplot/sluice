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
use sluice_seams::{Limits, Network, Setting, Settings};

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
    /// `[landing] subject`: a landing commit's first line, with `{id}`,
    /// `{from}`, `{agent}` and `{to}` filled in (23).
    pub landing_subject: String,
    /// `[preview]`: how a preview of this revision is brought up
    /// (previews-1 clauses 98, 99); `None` when the table is absent.
    pub preview: Option<Preview>,
}

/// `[preview]`: what brings a preview of the revision up, and the
/// addresses a person opens (previews-1 clauses 98, 99).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    /// The command, first word first: a list, or a string run by `sh -c`.
    pub command: Vec<String>,
    /// Each address by its label, a template over `{name.<port>}` and
    /// `{port.<port>}`; the place seam fills them in and refuses a port
    /// `[checkout] ports` does not list.
    pub addresses: BTreeMap<String, String>,
    /// How long the command may run; the branch's time limit when unset.
    pub timeout: Option<Duration>,
}

/// A conventional commit's subject by default.
pub const LANDING_SUBJECT: &str = "chore(land): #{id} {from}";

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
    /// Which results satisfy the gate (27 to 29): `confined`, executor
    /// kinds, and `cache` for replays.
    pub trust: Vec<String>,
    /// The git remote the branch lives on (C.4, C.7): read by fetch and
    /// landed by a leased push. `None` for a local branch.
    pub remote: Option<String>,
    /// C.7 `pull_requests`: the branch takes its work as pull requests on
    /// its remote, submitted once these conditions hold. `None` for a
    /// branch whose work is submitted straight from a session's place.
    pub pull_requests: Option<PullRequests>,
}

/// C.7 `pull_requests`: what a pull request into the branch meets before
/// its driver submits it (27). Read from the branch's revision (56), so a
/// pull request cannot loosen its own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PullRequests {
    /// Approving reviews on the pull request's current head.
    pub reviews: u32,
    /// Checks other systems post on its head, by name, each to have passed.
    pub checks: Vec<String>,
}

/// What `trust` may list.
const TRUSTS: [&str; 5] = ["confined", "open", "sandbox", "container", "cache"];

impl Rules {
    /// 28: whether a ran result's executor satisfies the gate.
    pub fn trusts_run(&self, kind: &str, confined: bool) -> bool {
        self.trust.iter().any(|t| t == kind) || (confined && self.trusts("confined"))
    }

    /// Whether `trust` lists `what`.
    pub fn trusts(&self, what: &str) -> bool {
        self.trust.iter().any(|t| t == what)
    }

    /// 29: whether a run of the executor `kind` can ever satisfy the gate:
    /// only a container is confined today.
    pub fn satisfiable_by(&self, kind: &str) -> bool {
        self.trusts(kind) || (kind == "container" && self.trusts("confined"))
    }
}

impl Default for Rules {
    /// What a table leaves out: every candidate, a window of one to four,
    /// a flaky key read as failed, an hour per candidate, and every run
    /// sluice starts trusted, no replay (ADR 0015).
    fn default() -> Self {
        Rules {
            required: Vec::new(),
            parallel: 4,
            batch: false,
            flaky: Flaky::Fail,
            timeout: Some(Duration::from_secs(3600)),
            trust: vec!["open".into(), "sandbox".into(), "container".into()],
            remote: None,
            pull_requests: None,
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
    #[serde(default)]
    landing: LandingForm,
    preview: Option<PreviewForm>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviewForm {
    command: Option<toml::Value>,
    #[serde(default)]
    addresses: BTreeMap<String, String>,
    timeout: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct LandingForm {
    subject: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ExecutorForm {
    kind: Option<String>,
    memory: Option<String>,
    network: Option<Network>,
    docker: Option<bool>,
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
    trust: Option<Vec<String>>,
    remote: Option<String>,
    pull_requests: Option<PullRequestsForm>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PullRequestsForm {
    reviews: Option<i64>,
    #[serde(default)]
    checks: Vec<String>,
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
            docker: form.executor.docker.unwrap_or(false),
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
            let trust = r.trust.unwrap_or(d.trust);
            if let Some(t) = trust.iter().find(|t| !TRUSTS.contains(&t.as_str())) {
                bail!(
                    "[branches.\"{pattern}\"] trust lists {t:?}; it may list {}",
                    TRUSTS.join(", ")
                );
            }
            let pull_requests = match r.pull_requests {
                None => None,
                Some(_) if r.remote.is_none() => bail!(
                    "[branches.\"{pattern}\".pull_requests] needs remote: pull requests \
                     are on the remote the branch lives on, and {pattern} names none"
                ),
                Some(p) => Some(PullRequests {
                    reviews: match p.reviews {
                        None => 0,
                        Some(n) if n >= 0 => n as u32,
                        Some(n) => bail!(
                            "[branches.\"{pattern}\".pull_requests] reviews must be at least 0, not {n}"
                        ),
                    },
                    checks: p.checks,
                }),
            };
            branches.push((
                pattern,
                Rules {
                    required: r.required,
                    parallel,
                    batch: r.batch.unwrap_or(d.batch),
                    flaky,
                    timeout,
                    trust,
                    remote: r.remote,
                    pull_requests,
                },
            ));
        }
        let landing_subject = form
            .landing
            .subject
            .unwrap_or_else(|| LANDING_SUBJECT.into());
        let mut rest = landing_subject.as_str();
        while let Some(i) = rest.find('{') {
            let close = rest[i..]
                .find('}')
                .ok_or_else(|| anyhow!("[landing] subject {landing_subject:?} leaves a {{ open"))?;
            let name = &rest[i + 1..i + close];
            if !["id", "from", "agent", "to"].contains(&name) {
                bail!("[landing] subject names {{{name}}}; it may name {{id}}, {{from}}, {{agent}} and {{to}}");
            }
            rest = &rest[i + close + 1..];
        }
        if landing_subject.trim().is_empty() || landing_subject.contains('\n') {
            bail!("[landing] subject must be one line");
        }
        let preview = form.preview.map(preview).transpose()?;
        Ok(Policy {
            tests,
            checkout,
            limits,
            branches,
            landing_subject,
            preview,
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

/// `[preview]`, its command and the shape of its addresses checked.
fn preview(form: PreviewForm) -> Result<Preview> {
    let command = match form.command {
        None => bail!("[preview] names no command"),
        Some(toml::Value::String(s)) if !s.trim().is_empty() => {
            vec!["/bin/sh".into(), "-c".into(), s]
        }
        Some(toml::Value::Array(xs)) if !xs.is_empty() => xs
            .into_iter()
            .map(|x| match x {
                toml::Value::String(s) => Ok(s),
                other => bail!("[preview] command must list strings, not {other}"),
            })
            .collect::<Result<_>>()?,
        Some(_) => bail!("[preview] command must be a string or a list, the command first"),
    };
    for (label, template) in &form.addresses {
        let mut rest = template.as_str();
        while let Some(i) = rest.find('{') {
            let close = rest[i..].find('}').ok_or_else(|| {
                anyhow!("[preview] addresses {label} = {template:?} leaves a {{ open")
            })?;
            let inner = &rest[i + 1..i + close];
            match inner.split_once('.') {
                Some(("name" | "port", port)) if !port.is_empty() => {}
                _ => bail!(
                    "[preview] addresses {label} names {{{inner}}}; it may name {{name.<port>}} and {{port.<port>}}"
                ),
            }
            rest = &rest[i + close + 1..];
        }
    }
    let timeout = match form.timeout.as_deref() {
        None => None,
        Some(t) => Some(parse_duration(t).context("[preview] timeout")?),
    };
    Ok(Preview {
        command,
        addresses: form.addresses,
        timeout,
    })
}

/// A profile's table: each value a string, a number, a list of strings,
/// or an array of tables whose values are strings or lists of strings.
fn settings(table: BTreeMap<String, toml::Value>, section: &str) -> Result<Settings> {
    let mut out = Settings::new();
    for (k, v) in table {
        let setting = match v {
            toml::Value::String(s) => Setting::Text(s),
            // A number is its digits; the profile that reads it parses them.
            toml::Value::Integer(n) => Setting::Text(n.to_string()),
            toml::Value::Array(xs) if !xs.is_empty() && xs.iter().all(|x| x.is_table()) => {
                Setting::Tables(
                    xs.into_iter()
                        .map(|x| {
                            let toml::Value::Table(t) = x else {
                                unreachable!("every entry is a table")
                            };
                            let t: BTreeMap<String, toml::Value> = t.into_iter().collect();
                            if let Some((n, v)) = t.iter().find(|(_, v)| v.is_integer()) {
                                bail!("{section} [[{k}]] {n} must be a string or a list of strings, not {v}");
                            }
                            let inner = settings(t, &format!("{section} [[{k}]]"))?;
                            if inner.values().any(|s| matches!(s, Setting::Tables(_))) {
                                bail!("{section} [[{k}]] holds strings and lists of strings only");
                            }
                            Ok(inner)
                        })
                        .collect::<Result<_>>()?,
                )
            }
            toml::Value::Array(xs) => Setting::List(
                xs.into_iter()
                    .map(|x| match x {
                        toml::Value::String(s) => Ok(s),
                        other => bail!("{section} {k} must list strings, not {other}"),
                    })
                    .collect::<Result<_>>()?,
            ),
            other => bail!(
                "{section} {k} must be a string, a number, a list of strings or an array of tables, not {other}"
            ),
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
        assert_eq!(main.trust, ["open", "sandbox", "container"]);
    }

    /// sluice's own policy reads, and gates as section 9 says: bolt
    /// branches on the fast tier from any run or the cache, `main` on
    /// every tier from sandboxed or confined runs only, deepest first,
    /// landed on `origin`, taking its work as pull requests that wait on
    /// nothing beyond the gate.
    #[test]
    fn sluices_own_policy_reads() {
        let p = Policy::parse(include_str!("../../../.config/sluice.toml")).unwrap();
        assert_eq!(p.limits.executor.as_deref(), Some("sandbox"));
        assert_eq!(p.limits.network, Some(Network::Localhost));
        assert!(matches!(&p.checkout["cache"], Setting::Tables(t) if t.len() == 1));
        let bolt = p.rules_for("bolt/x").unwrap().unwrap();
        assert_eq!(bolt.required, ["#check-fast"]);
        assert!(bolt.satisfiable_by("sandbox") && bolt.trusts("cache"));
        let main = p.rules_for("main").unwrap().unwrap();
        assert_eq!(main.required, ["#check-fast", "#check-tools", "#check-e2e"]);
        assert!(main.batch && main.satisfiable_by("sandbox") && !main.trusts("cache"));
        assert_eq!(main.remote.as_deref(), Some("origin"));
        assert_eq!(bolt.remote, None);
        assert_eq!(main.pull_requests, Some(PullRequests::default()));
        assert_eq!(bolt.pull_requests, None);
        assert!(p.rules_for("unit/x").unwrap().is_none());
    }

    /// 23: a landing commit's subject is the policy's, conventional by
    /// default.
    #[test]
    fn reads_the_landing_subject() {
        assert_eq!(Policy::parse("").unwrap().landing_subject, LANDING_SUBJECT);
        let p = Policy::parse("[landing]\nsubject = \"land: #{id} from {agent}\"\n").unwrap();
        assert_eq!(p.landing_subject, "land: #{id} from {agent}");
        assert!(Policy::parse("[landing]\nsubject = \"#{nope}\"\n").is_err());
        assert!(Policy::parse("[landing]\nsubject = \"#{id\"\n").is_err());
    }

    /// 27 to 29: a gate names the results it trusts.
    #[test]
    fn reads_what_a_gate_trusts() {
        let p = Policy::parse(
            "[branches.main]\ntrust = [\"confined\", \"sandbox\"]\n\
             [branches.\"bolt/*\"]\ntrust = [\"open\", \"cache\"]\n",
        )
        .unwrap();
        let main = p.rules_for("main").unwrap().unwrap();
        assert!(main.trusts_run("sandbox", false));
        assert!(main.trusts_run("container", true));
        assert!(!main.trusts_run("open", false));
        assert!(!main.trusts("cache"));
        assert!(main.satisfiable_by("sandbox"));
        assert!(!main.satisfiable_by("open"));
        let bolt = p.rules_for("bolt/x").unwrap().unwrap();
        assert!(bolt.trusts("cache") && bolt.trusts_run("open", false));
        assert!(!bolt.satisfiable_by("sandbox"));
        // ADR 0015: a table without `trust` takes every run sluice starts,
        // and no replay.
        let default = Policy::parse("[branches.main]\n").unwrap();
        let default = default.rules_for("main").unwrap().unwrap();
        assert!(["open", "sandbox", "container"]
            .iter()
            .all(|k| default.satisfiable_by(k) && default.trusts_run(k, false)));
        assert!(!default.trusts("cache"));
        assert!(Policy::parse("[branches.main]\ntrust = [\"anyone\"]\n").is_err());
    }

    /// 35: the caches a place may hold reach the place's profile as an
    /// array of tables, given no meaning here.
    #[test]
    fn reads_an_array_of_tables_for_a_profile() {
        let p = Policy::parse(
            "[checkout]\nmethod = \"git-worktree\"\n\
             [[checkout.cache]]\nkey = [\"Cargo.lock\"]\npaths = [\"target\"]\n",
        )
        .unwrap();
        assert_eq!(
            p.checkout["cache"],
            Setting::Tables(vec![Settings::from([
                ("key".into(), Setting::List(vec!["Cargo.lock".into()])),
                ("paths".into(), Setting::List(vec!["target".into()])),
            ])])
        );
        assert!(Policy::parse("[[checkout.cache]]\nkey = 1\n").is_err());
        assert!(
            Policy::parse("[[checkout.cache]]\n[[checkout.cache.inner]]\nx = \"y\"\n").is_err()
        );
    }

    /// C.7: a branch may name the remote it lives on.
    #[test]
    fn reads_a_branchs_remote() {
        let p =
            Policy::parse("[branches.main]\nremote = \"origin\"\n[branches.\"bolt/*\"]\n").unwrap();
        assert_eq!(
            p.rules_for("main").unwrap().unwrap().remote.as_deref(),
            Some("origin")
        );
        assert_eq!(p.rules_for("bolt/x").unwrap().unwrap().remote, None);
        assert!(Policy::parse("[branches.main]\nremote = 1\n").is_err());
        assert!(Policy::parse("[branches.main]\nremote = [\"origin\"]\n").is_err());
    }

    /// C.7: a branch may turn pull requests on, with the conditions a pull
    /// request meets before it is submitted; only on a remote.
    #[test]
    fn reads_a_branchs_pull_requests() {
        let p = Policy::parse(
            "[branches.main]\nremote = \"origin\"\n\
             [branches.main.pull_requests]\nreviews = 1\nchecks = [\"qa/approved\"]\n\
             [branches.\"bolt/*\"]\n",
        )
        .unwrap();
        assert_eq!(
            p.rules_for("main").unwrap().unwrap().pull_requests,
            Some(PullRequests {
                reviews: 1,
                checks: vec!["qa/approved".into()],
            })
        );
        // Absent: work is submitted straight from a session's place.
        assert_eq!(p.rules_for("bolt/x").unwrap().unwrap().pull_requests, None);
        // An empty table takes every pull request out of draft.
        let p =
            Policy::parse("[branches.main]\nremote = \"origin\"\n[branches.main.pull_requests]\n")
                .unwrap();
        assert_eq!(
            p.rules_for("main").unwrap().unwrap().pull_requests,
            Some(PullRequests::default())
        );
        // Pull requests need the remote they are on.
        let err = Policy::parse("[branches.main]\n[branches.main.pull_requests]\nreviews = 1\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("main") && err.contains("remote"), "{err}");
        assert!(Policy::parse(
            "[branches.main]\nremote = \"origin\"\n[branches.main.pull_requests]\nreviews = -1\n"
        )
        .is_err());
        assert!(Policy::parse(
            "[branches.main]\nremote = \"origin\"\n[branches.main.pull_requests]\nlabels = [\"x\"]\n"
        )
        .is_err());
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
                network: Some(Network::Denied),
                docker: false,
            }
        );
        let network = |v: &str| {
            Policy::parse(&format!("[executor]\nnetwork = {v}\n")).map(|p| p.limits.network)
        };
        assert_eq!(network("true").unwrap(), Some(Network::Allowed));
        assert_eq!(network("\"localhost\"").unwrap(), Some(Network::Localhost));
        assert!(network("\"lan\"").is_err());
        assert!(network("1").is_err());
        assert!(
            Policy::parse("[executor]\ndocker = true\n")
                .unwrap()
                .limits
                .docker
        );
        assert!(Policy::parse("[executor]\ncpus = 2\n").is_err());
        assert!(Policy::parse("[executor]\nmemory = \"lots\"\n").is_err());
        assert!(Policy::parse("[executor]\nmemory = \"8\"\n").is_err());
        assert!(Policy::parse("[tests]\nshell = [1]\n").is_err());
        assert!(Policy::parse("[tests]\njobs = true\n").is_err());
        assert_eq!(
            Policy::parse("[checkout]\nports_from = 20000\n")
                .unwrap()
                .checkout["ports_from"],
            Setting::Text("20000".into())
        );
    }

    /// Previews-1 clauses 98, 99: `[preview]`'s command, addresses and
    /// time limit.
    #[test]
    fn reads_the_preview_table() {
        assert_eq!(Policy::parse("").unwrap().preview, None);
        let p = Policy::parse(
            "[preview]\ncommand = [\"moon\", \"run\", \"app:preview\"]\n\
             addresses = { web = \"https://{name.web}.localhost\", api = \"http://localhost:{port.api}/docs\" }\n\
             timeout = \"10m\"\n",
        )
        .unwrap()
        .preview
        .unwrap();
        assert_eq!(p.command, ["moon", "run", "app:preview"]);
        assert_eq!(p.addresses["web"], "https://{name.web}.localhost");
        assert_eq!(p.timeout, Some(Duration::from_secs(600)));
        let shell = Policy::parse("[preview]\ncommand = \"make preview\"\n").unwrap();
        assert_eq!(
            shell.preview.unwrap().command,
            ["/bin/sh", "-c", "make preview"]
        );
        let refused = |text: &str| format!("{:#}", Policy::parse(text).unwrap_err());
        assert!(refused("[preview]\n").contains("[preview] names no command"));
        assert!(refused("[preview]\ncommand = []\n").contains("a string or a list"));
        assert!(
            refused("[preview]\ncommand = \"x\"\naddresses = { web = \"{host}\" }\n")
                .contains("[preview] addresses web names {host}")
        );
        assert!(
            refused("[preview]\ncommand = \"x\"\naddresses = { web = \"{name.web\" }\n")
                .contains("leaves a { open")
        );
        assert!(refused("[preview]\ncommand = \"x\"\ntimeout = \"soon\"\n")
            .contains("[preview] timeout"));
        assert!(refused("[preview]\ncommand = \"x\"\nurls = {}\n").contains("urls"));
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
