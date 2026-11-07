//! What a reader's tick reads and starts, through the binary on a
//! repository on disk with git counting its starts (129). A test binary
//! of its own: a child a test starts holds a copy of every lock the test
//! process holds until it execs, which a test of the fakes in `cli.rs`
//! handing a lock from one holder to another would find taken.

mod support;

use support::OnDisk;

const POLICY: &str =
    "[tests]\nrunner = \"command\"\ncommand = \"true\"\n\n[branches.main]\ntimeout = \"1m\"\n";

fn text(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// How many times git started for `args`, run once.
fn starts(d: &OnDisk, args: &[&str]) -> usize {
    let before = d.git.count();
    let out = d.sluice(args);
    assert!(out.status.success(), "{args:?}: {}", text(&out));
    d.git.count() - before
}

/// The counting git counts what sluice starts.
#[test]
fn a_counting_git_counts_what_status_starts() {
    let d = OnDisk::new("counted-status", POLICY);
    assert!(starts(&d, &["status", "main"]) > 0, "{:?}", d.git.starts());
}

/// reader-tick, "sluice run in a set-up repository with nothing
/// queued": its passes after the first start no git (129).
#[test]
fn sluice_run_with_nothing_queued_starts_git_at_its_first_pass_alone() {
    let d = OnDisk::new("idle-run", POLICY);
    d.land("a");
    let one = starts(&d, &["run", "--rounds", "1"]);
    let before = d.git.starts().len();
    let six = starts(&d, &["run", "--rounds", "6"]);
    assert_eq!(six, one, "{:?}", &d.git.starts()[before..]);
}

/// A quiet branch whose policy is mended on its remote and fetched by
/// hand: what a watch, the reader, shows and starts once this host holds
/// the mended revision (56, 124, 129; rules core.15, core.31).
mod mended {
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Stdio};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::support::{OnDisk, Repo};

    /// `main` on `origin`, its records in this host's files.
    const ON_ORIGIN: &str = "[tests]\nrunner = \"command\"\ncommand = \"true\"\n\n\
                             [branches.main]\ntimeout = \"1m\"\nremote = \"origin\"\n";

    /// A `[[checkout.cache]]` table, which this sluice refuses (record
    /// 0049).
    const CACHE: &str = "\n[[checkout.cache]]\nkey = [\"Cargo.lock\"]\npaths = [\"target\"]\n";

    /// How long the watch is watched once its line shows `main` whole:
    /// two ticks at least, the watch pausing a second between them.
    const TICKS: Duration = Duration::from_millis(2500);

    /// `other`'s commit of `policy` on `main`, pushed to origin: its
    /// revision.
    fn push_policy(other: &Repo, policy: &str) -> String {
        other.write(".config/sluice.toml", policy);
        other.git(&["commit", "-q", "-am", "the policy, by hand"]);
        other.git(&["push", "-q", "origin", "HEAD:main"]);
        other.rev("HEAD")
    }

    /// `main`'s records in this host's files: their `base`, and how many
    /// `moved` lines their log holds.
    fn stood(d: &OnDisk) -> (String, usize) {
        let dir = d.repo.dir().join(".git/sluice/main");
        let doc: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("branch.json")).unwrap()).unwrap();
        let moved = std::fs::read_to_string(dir.join("log.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter(|l| l.contains("\"event\":\"moved\""))
            .count();
        (doc["base"].as_str().unwrap_or_default().to_string(), moved)
    }

    /// `main`'s records in this host's files as a sluice that read `at`'s
    /// policy left them: a round stood on `at` last.
    fn last_saw(d: &OnDisk, at: &str) {
        let dir = d.repo.dir().join(".git/sluice/main");
        let mut doc: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join("branch.json")).unwrap()).unwrap();
        doc["base"] = at.into();
        std::fs::write(
            dir.join("branch.json"),
            serde_json::to_string_pretty(&doc).unwrap(),
        )
        .unwrap();
        let version: u64 = std::fs::read_to_string(dir.join("version"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        std::fs::write(dir.join("version"), format!("{}\n", version + 1)).unwrap();
    }

    /// `sluice overview --json --watch` in `d`'s repository: ended when
    /// dropped.
    struct Watch {
        child: Child,
        lines: mpsc::Receiver<String>,
    }

    impl Watch {
        fn start(d: &OnDisk) -> Watch {
            let mut child = d
                .command(d.repo.dir())
                .args(["overview", "--json", "--watch"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let out = BufReader::new(child.stdout.take().unwrap());
            let (send, lines) = mpsc::channel();
            std::thread::spawn(move || {
                for line in out.lines().map_while(Result::ok) {
                    if send.send(line).is_err() {
                        break;
                    }
                }
            });
            Watch { child, lines }
        }

        /// `main` as the next line within `left` shows it.
        fn next(&self, left: Duration) -> Option<serde_json::Value> {
            let line = self.lines.recv_timeout(left).ok()?;
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            let main = v["branches"]
                .as_array()
                .and_then(|b| b.iter().find(|b| b["branch"] == "main"))
                .cloned();
            Some(main.unwrap_or_else(|| panic!("no main in {v}")))
        }

        /// `main` as each line within `within` shows it.
        fn mains_within(&self, within: Duration) -> Vec<serde_json::Value> {
            let until = Instant::now() + within;
            let mut out = Vec::new();
            while let Some(left) = until.checked_duration_since(Instant::now()) {
                match self.next(left) {
                    Some(main) => out.push(main),
                    None => break,
                }
            }
            out
        }

        /// `main` as the first line within `within` that `holds` shows it.
        fn until(
            &self,
            within: Duration,
            holds: impl Fn(&serde_json::Value) -> bool,
        ) -> serde_json::Value {
            let until = Instant::now() + within;
            let mut seen = Vec::new();
            while let Some(left) = until.checked_duration_since(Instant::now()) {
                let Some(main) = self.next(left) else {
                    break;
                };
                if holds(&main) {
                    return main;
                }
                seen.push(main);
            }
            panic!("no such line within {within:?}: {seen:#?}");
        }
    }

    impl Drop for Watch {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    /// reader-tick, "A refusal ends at the tick after the host's revision
    /// moved" (129; rules core.15, core.31): `main` lands on origin, its
    /// records in this host's files last seeing `r1`, whose policy holds a
    /// `[[checkout.cache]]` table, and nothing is queued. Another host
    /// pushes `r2`, which takes the table out, and the watch, the reader,
    /// still shows `main` refused naming `r1`, the revision this host
    /// holds. `origin/main` fetched by hand to `r2`, the watch's next line
    /// shows `main` whole, its `at` still `r1`. The policy is read by gix,
    /// with no process: the watch starts no git from the fetch on, so the
    /// mended policy is read without one and no round runs, which would
    /// fetch `main`; and the records still stand on `r1`.
    #[test]
    fn a_refusal_ends_at_the_tick_after_the_hosts_revision_moved() {
        let d = OnDisk::new("mended-tick", ON_ORIGIN);
        let origin = d.repo.bare_origin();
        d.land("a");
        let other = Repo::clone_of(origin.path(), "mended-tick-b");
        let clean = std::fs::read_to_string(other.dir().join(".config/sluice.toml")).unwrap();
        let r1 = push_policy(&other, &format!("{clean}{CACHE}"));
        d.repo.git(&["fetch", "-q", "origin"]);
        assert_eq!(d.repo.rev("origin/main"), r1);
        last_saw(&d, &r1);
        let r2 = push_policy(&other, &clean);
        assert_eq!(d.repo.rev("origin/main"), r1, "this host fetched r2");
        let before = stood(&d);
        assert_eq!(before.0, r1);

        let watch = Watch::start(&d);
        let refused = watch.until(Duration::from_secs(60), |m| m["unread"].is_string());
        let words = refused["unread"].as_str().unwrap();
        assert!(
            words.contains(&r1) && words.contains("[[checkout.cache]]"),
            "{refused}"
        );
        let started = d.git.count();

        d.repo.git(&["fetch", "-q", "origin"]);
        assert_eq!(d.repo.rev("origin/main"), r2);
        let read = watch.until(Duration::from_secs(10), |m| m.get("unread").is_none());
        assert_eq!(read["at"], r1.as_str(), "{read}");
        assert_eq!(read["revision"], r2.as_str(), "{read}");
        let after = watch.mains_within(TICKS);
        assert!(
            after.iter().all(|m| m.get("unread").is_none()),
            "{after:#?}"
        );

        assert_eq!(d.git.count(), started, "{:?}", &d.git.starts()[started..]);
        assert_eq!(stood(&d), before, "a round ran");
    }
}
