//! `sluice exec` on a scratch repository (session-runs-1 clauses 89 to
//! 92): the binary, a real place and a real supervisor. Needs git, so it
//! is in the check-tools tier.

#![cfg(feature = "tools")]

#[path = "support/mod.rs"]
mod support;

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use support::{Repo, Scratch};

const POLICY: &str = "[tests]\nrunner = \"command\"\ncommand = \"sh check.sh\"\n\n\
[checkout]\nports = [\"web\"]\nports_from = \"41500\"\n\n\
[branches.\"bolt/*\"]\ntimeout = \"1m\"\n";

struct Setup {
    repo: Repo,
    worktree: PathBuf,
    config: Scratch,
    state: Scratch,
    _sessions: Scratch,
}

impl Setup {
    fn new(name: &str) -> Setup {
        Setup::at(name, "a")
    }

    /// A worktree at `rel` under the scratch directory for sessions.
    fn at(name: &str, rel: &str) -> Setup {
        let repo = Repo::new(name);
        repo.write(".config/sluice.toml", POLICY);
        repo.write("check.sh", "echo checked >> \"$SLUICE_TMP/checked\"\n");
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "sluice's policy"]);
        repo.git(&["branch", "bolt/x", "main"]);
        let sessions = Scratch::new(&format!("{name}-sessions"));
        let worktree = sessions.path().join(rel);
        repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            "unit/a",
            worktree.to_str().unwrap(),
            "bolt/x",
        ]);
        std::fs::write(worktree.join("a.txt"), "a").unwrap();
        let git = |args: &[&str]| {
            let mut a = vec!["-C", worktree.to_str().unwrap()];
            a.extend(args);
            repo.git(&a)
        };
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", "a"]);
        Setup {
            repo,
            worktree,
            config: Scratch::new(&format!("{name}-config")),
            state: Scratch::new(&format!("{name}-state")),
            _sessions: sessions,
        }
    }

    fn sluice(&self, args: &[&str]) -> Output {
        self.sluice_with(&[], args)
    }

    /// [`Setup::sluice`], with `env` set beside the rest.
    fn sluice_with(&self, env: &[(&str, &Path)], args: &[&str]) -> Output {
        support::isolated(&mut Command::new(env!("CARGO_BIN_EXE_sluice")))
            .envs(env.iter().copied())
            .args(args)
            .current_dir(&self.worktree)
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("XDG_STATE_HOME", self.state.path())
            .env_remove("SLUICE_AGENT")
            .env_remove("SLUICE_OTEL")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    /// `sluice` started in this setup's worktree on `host`'s host, its
    /// configuration and state, as another repository on that host runs
    /// it; its stdout piped, its stderr the test's.
    fn spawn_on(&self, host: &Setup, args: &[&str]) -> Child {
        support::isolated(&mut Command::new(env!("CARGO_BIN_EXE_sluice")))
            .args(args)
            .current_dir(&self.worktree)
            .env("XDG_CONFIG_HOME", host.config.path())
            .env("XDG_STATE_HOME", host.state.path())
            .env_remove("SLUICE_AGENT")
            .env_remove("SLUICE_OTEL")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
    }

    /// Commit `policy` on `bolt/x`, as the branch's policy.
    fn policy(&self, policy: &str) {
        self.repo.write(".config/sluice.toml", policy);
        self.repo.git(&["commit", "-q", "-am", "the policy"]);
        self.repo.git(&["branch", "-f", "bolt/x", "main"]);
    }

    fn results(&self) -> String {
        std::fs::read_to_string(self.repo.dir().join(".git/sluice/results.jsonl"))
            .unwrap_or_default()
    }

    fn sessions(&self) -> Vec<String> {
        match std::fs::read_dir(self.state.path().join("sluice/sessions")) {
            Ok(d) => d
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|n| !n.starts_with('.'))
                .collect(),
            Err(_) => Vec::new(),
        }
    }
}

/// Where the binary looks for the container engine in place of the
/// host's sockets (record 0074).
const ENGINE_SOCKETS: &str = "SLUICE_ENGINE_SOCKETS";

/// `bolt/x` holding a moon workspace of two projects whose `test` tasks,
/// tagged `gate`, the policy requires of runs that ask for the container
/// engine: `a`'s runs `a_check`, `b`'s passes. The session's head changes
/// both. Needs moon on the path.
fn moon_setup(name: &str, a_check: &str) -> Setup {
    let repo = Repo::new(name);
    repo.write(
        ".config/sluice.toml",
        "[tests]\nrunner = \"moon\"\n\n[executor]\ndocker = true\n\n\
         [branches.\"bolt/*\"]\nrequired = [\"#gate\"]\ntimeout = \"5m\"\n",
    );
    repo.write(
        ".moon/workspace.yml",
        "projects:\n  a: 'a'\n  b: 'b'\n\
         vcs:\n  client: 'git'\n  defaultBranch: 'main'\ntelemetry: false\n",
    );
    repo.write(".gitignore", ".moon/cache/\n");
    repo.write("a/check.sh", a_check);
    for (p, command) in [("a", "sh a/check.sh"), ("b", "true")] {
        repo.write(&format!("{p}/lib"), "1");
        repo.write(
            &format!("{p}/moon.yml"),
            &format!(
                "tasks:\n  test:\n    command: '{command}'\n    inputs: ['**/*']\n    \
                 tags: [gate]\n    options: {{runFromWorkspaceRoot: true}}\n"
            ),
        );
    }
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "two projects"]);
    repo.git(&["branch", "bolt/x", "main"]);
    let sessions = Scratch::new(&format!("{name}-sessions"));
    let worktree = sessions.path().join("a");
    repo.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "unit/a",
        worktree.to_str().unwrap(),
        "bolt/x",
    ]);
    for p in ["a", "b"] {
        std::fs::write(worktree.join(p).join("lib"), "2").unwrap();
    }
    let git = |args: &[&str]| {
        let mut a = vec!["-C", worktree.to_str().unwrap()];
        a.extend(args);
        repo.git(&a)
    };
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "a and b"]);
    Setup {
        repo,
        worktree,
        config: Scratch::new(&format!("{name}-config")),
        state: Scratch::new(&format!("{name}-state")),
        _sessions: sessions,
    }
}

/// Each result `sluice exec` recorded: `(task, outcome)`.
fn recorded(s: &Setup) -> Vec<(String, String)> {
    s.results()
        .lines()
        .map(|l| {
            let v: serde_json::Value = serde_json::from_str(l).unwrap();
            (
                v["task"].as_str().unwrap().to_string(),
                v["outcome"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// The host's configuration in `config`: a bound of `max`.
fn host_bound(config: &Scratch, max: usize) {
    std::fs::create_dir_all(config.path().join("sluice")).unwrap();
    std::fs::write(
        config.path().join("sluice/config.toml"),
        format!("[runs]\nmax = {max}\n"),
    )
    .unwrap();
}

/// The host's configuration in `config`: one cache whose clients read
/// `SCCACHE_DIR`.
fn host_cache(config: &Scratch) {
    std::fs::create_dir_all(config.path().join("sluice")).unwrap();
    std::fs::write(
        config.path().join("sluice/config.toml"),
        format!(
            "[[caches]]\ndir = {:?}\nenv = {{ SCCACHE_DIR = \"/caches/sccache\" }}\n",
            config.path().join("cache").display().to_string()
        ),
    )
    .unwrap();
}

/// Wait until `done` holds, for at most a minute.
fn eventually(what: &str, mut done: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(60);
    while !done() {
        assert!(Instant::now() < until, "{what} never happened");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The lines `child` prints, as they come.
fn lines_of(child: &mut Child) -> std::sync::mpsc::Receiver<String> {
    let out = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(out).lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    rx
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// 89, 90, 92: a command gets the worktree as its checkout, its scratch
/// directory and its port, the same port on its next run; its exit is the
/// command's, and nothing is recorded.
#[test]
fn a_command_gets_the_worktrees_environment_and_its_exit() {
    let s = Setup::new("exec-command");
    let seen = |n: &str| -> String {
        let out = s.sluice(&[
            "exec",
            "bolt/x",
            "--",
            "sh",
            "-c",
            &format!("echo \"$SLUICE_CHECKOUT $SLUICE_PORT_WEB $SLUICE_PLACE\" > seen-{n}; exit 7"),
        ]);
        assert_eq!(out.status.code(), Some(7), "{}", text(&out));
        std::fs::read_to_string(s.worktree.join(format!("seen-{n}"))).unwrap()
    };
    let one = seen("1");
    let two = seen("2");
    let canon = std::fs::canonicalize(&s.worktree).unwrap();
    let words: Vec<&str> = one.split_whitespace().collect();
    assert_eq!(words[0], canon.to_str().unwrap());
    assert!(words[1].parse::<u16>().unwrap() >= 41500, "{one}");
    assert!(words[2].starts_with("sluice-"), "{one}");
    assert_eq!(one, two, "the same place and port on the second run");
    assert_eq!(s.results(), "", "a command records nothing");
    assert!(s.sessions().is_empty(), "the run is no longer counted");
}

/// 35, 89: an agent's run in its worktree is given the host's caches as
/// a submission's run is: the variables the host's config declares.
#[test]
fn sluice_exec_gets_the_hosts_cache_environment() {
    let s = Setup::new("exec-cache-env");
    host_cache(&s.config);
    let out = s.sluice(&["exec", "bolt/x", "--", "sh", "-c", "env > seen"]);
    assert!(out.status.success(), "{}", text(&out));
    let seen = std::fs::read_to_string(s.worktree.join("seen")).unwrap();
    assert!(
        seen.lines().any(|l| l == "SCCACHE_DIR=/caches/sccache"),
        "{seen}"
    );
}

/// Every file name kept under `dir`, walked whole.
/// 64: before it runs, exec says whether the head would form if
/// submitted now; a conflict is told and the run goes on, and `--json`
/// carries the forecast.
#[test]
fn exec_tells_a_conflict_before_it_runs_and_runs_anyway() {
    let s = Setup::new("exec-forecast");
    let forecast = |out: &Output| -> serde_json::Value {
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        v["forecast"].clone()
    };
    let out = s.sluice(&["exec", "bolt/x", "--json", "--", "true"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(forecast(&out)["conflict"], serde_json::Value::Null);
    assert!(!text(&out).contains("would conflict"), "{}", text(&out));

    // The branch moves on, changing what the head changed.
    s.repo.git(&["switch", "-q", "bolt/x"]);
    s.repo.write("a.txt", "b");
    s.repo.git(&["add", "-A"]);
    s.repo.git(&["commit", "-q", "-m", "b"]);
    s.repo.git(&["switch", "-q", "main"]);

    let out = s.sluice(&["exec", "bolt/x", "--json", "--", "true"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert_eq!(
        forecast(&out)["conflict"]["paths"],
        serde_json::json!(["a.txt"])
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("would conflict: a.txt with bolt/x"),
        "{}",
        text(&out)
    );
    let out = s.sluice(&["exec", "bolt/x", "--", "true"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    assert!(
        text(&out).starts_with("would conflict: a.txt with bolt/x\n"),
        "{}",
        text(&out)
    );
}

fn file_names(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries {
        let e = e.unwrap();
        out.push(e.file_name().to_string_lossy().into_owned());
        if e.file_type().unwrap().is_dir() {
            out.extend(file_names(&e.path()));
        }
    }
    out
}

/// 89, 26, rule core.11: a worktree whose path, written as one file name,
/// is longer than any file name may be runs its checks, is recorded and
/// is listed by its whole path, and every file name sluice keeps for it
/// fits.
#[test]
fn exec_runs_from_a_worktree_at_a_long_path() {
    let deep = ["a-rather-long-directory-name"; 10].join("/");
    let s = Setup::at("exec-long", &deep);
    let canon = std::fs::canonicalize(&s.worktree).unwrap();
    assert!(canon.as_os_str().len() > 255, "{}", canon.display());
    // Ports of its own, so the tests beside it that count theirs see none.
    s.repo.write(
        ".config/sluice.toml",
        &POLICY.replace("\"41500\"", "\"43100\""),
    );
    s.repo.git(&["commit", "-q", "-am", "ports of its own"]);
    s.repo.git(&["branch", "-f", "bolt/x", "main"]);

    let out = s.sluice(&["exec", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        s.results().contains("\"outcome\":\"pass\""),
        "{}",
        s.results()
    );
    let out = s.sluice(&["exec", "bolt/x", "--", "sh", "-c", "echo $SLUICE_PORT_WEB"]);
    assert!(out.status.success(), "{}", text(&out));
    // 35, rule core.30: its scratch is the repository's own, as short as
    // any worktree's, and holds nothing of the worktree's path.
    let out = s.sluice(&["exec", "bolt/x", "--", "sh", "-c", "echo $SLUICE_TMP"]);
    assert!(out.status.success(), "{}", text(&out));
    let printed = String::from_utf8_lossy(&out.stdout);
    let tmp = printed
        .lines()
        .find(|l| l.contains("/sluice/.tmp/"))
        .unwrap_or_else(|| panic!("no scratch printed: {}", text(&out)));
    let root = std::fs::canonicalize(s.repo.dir().join(".git")).unwrap();
    let hex = std::fs::canonicalize(tmp)
        .unwrap()
        .strip_prefix(root.join("sluice/.tmp"))
        .unwrap_or_else(|_| panic!("{tmp} is not under {}", root.display()))
        .to_string_lossy()
        .into_owned();
    assert!(
        hex.len() == 10 && hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
        "{tmp}"
    );
    assert!(!tmp.contains("a-rather-long-directory-name"), "{tmp}");

    let out = s.sluice(&["status", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        doc["worktrees"][0]["worktree"],
        canon.to_str().unwrap(),
        "{doc}"
    );
    assert_eq!(doc["worktrees"][0]["removed"], false, "{doc}");

    let mut names = file_names(&s.repo.dir().join(".git/sluice"));
    names.extend(file_names(s.state.path()));
    assert!(names.iter().any(|n| n.contains("%~")), "{names:?}");
    for n in &names {
        assert!(n.len() <= 255, "{n}");
    }

    let out = s.sluice(&["release"]);
    assert!(out.status.success(), "{}", text(&out));
    let out = s.sluice(&["status", "--json"]);
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["worktrees"].as_array().map_or(0, Vec::len), 0, "{doc}");
}

/// 92: a clean worktree's run of the checks is recorded under its keys
/// with its executor and host, as a run on its head from its worktree; a
/// dirty one runs and records nothing.
#[test]
fn a_clean_run_of_the_checks_is_recorded_and_a_dirty_one_is_not() {
    let s = Setup::new("exec-checks");
    let out = s.sluice(&["exec", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    let results = s.results();
    let lines: Vec<serde_json::Value> = results
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 1, "{results}");
    let head = s.repo.git(&["rev-parse", "unit/a"]);
    let canon = std::fs::canonicalize(&s.worktree).unwrap();
    let l = &lines[0];
    assert_eq!(l["kind"], "ran");
    assert_eq!(l["task"], "sh check.sh");
    assert_eq!(l["outcome"], "pass");
    assert_eq!(l["branch"], "bolt/x");
    assert_eq!(l["combination"], head.trim());
    assert_eq!(l["worktree"], canon.to_str().unwrap());
    assert_eq!(l["executor"]["kind"], "open");
    assert_eq!(l["executor"]["confined"], false);
    assert!(l["host"].is_string());

    std::fs::write(s.worktree.join("a.txt"), "changed").unwrap();
    let out = s.sluice(&["exec", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("not recorded: worktree is not clean"),
        "{}",
        text(&out)
    );
    assert_eq!(s.results(), results, "a dirty run records nothing");
    let tmp = Path::new(&s.repo.dir().join(".git/sluice/.tmp"))
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(
        std::fs::read_to_string(tmp.join("checked")).unwrap(),
        "checked\nchecked\n",
        "both runs ran, with one scratch directory"
    );
}

/// Session-runs-1 clause 97, 123: a worktree removed with `rm -rf`, with
/// nothing released, is no session's. The next `sluice exec` on the host,
/// from another worktree, runs the release its runs stored, once, in its
/// scratch directory with its port, the policy's toolchain not run again
/// in its name, and frees its place: its block of ports goes to the next
/// worktree, and the branch's log names it `gone` with its release run. A
/// third `sluice exec` runs its release no more. `sluice status` lists
/// every worktree held, and the removed one only until it is freed.
#[test]
fn the_next_exec_runs_a_removed_worktrees_stored_release_once() {
    let s = Setup::new("exec-removed");
    let marks = Scratch::new("exec-removed-marks");
    // Each says where it ran, so a run in the removed worktree's name
    // shows, and the release the port it was given.
    let write = |name: &str| {
        let script = marks.path().join(format!("{name}.sh"));
        let log = marks.path().join(name);
        std::fs::write(
            &script,
            format!("echo \"$(pwd -P) $SLUICE_PORT_WEB\" >> {}\n", log.display()),
        )
        .unwrap();
        (script, log)
    };
    let (release, released) = write("released");
    let (toolchain, tooled) = write("toolchain");
    s.repo.write(
        ".config/sluice.toml",
        // Ports of its own: a block it frees is free only if no test
        // beside it probes the same ports at that moment.
        &POLICY
            .replace(
                "[tests]\n",
                &format!(
                    "[tests]\nrelease = \"sh {}\"\ntoolchain = [\"/bin/sh\", \"{}\"]\n",
                    release.display(),
                    toolchain.display()
                ),
            )
            .replace("\"41500\"", "\"43200\""),
    );
    s.repo.git(&["commit", "-q", "-am", "a release command"]);
    s.repo.git(&["branch", "-f", "bolt/x", "main"]);
    let other = s._sessions.path().join("b");
    s.repo.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "unit/b",
        other.to_str().unwrap(),
        "bolt/x",
    ]);
    let sluice_in = |dir: &Path, args: &[&str]| -> Output {
        support::isolated(&mut Command::new(env!("CARGO_BIN_EXE_sluice")))
            .args(args)
            .current_dir(dir)
            .env("XDG_CONFIG_HOME", s.config.path())
            .env("XDG_STATE_HOME", s.state.path())
            .env_remove("SLUICE_AGENT")
            .env_remove("SLUICE_OTEL")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let port = |dir: &Path| -> String {
        let out = sluice_in(
            dir,
            &["exec", "bolt/x", "--", "sh", "-c", "echo $SLUICE_PORT_WEB"],
        );
        assert!(out.status.success(), "{}", text(&out));
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find(|l| l.parse::<u16>().is_ok())
            .unwrap()
            .to_string()
    };
    let a_port = port(&s.worktree);
    let canon_a = std::fs::canonicalize(&s.worktree).unwrap();

    // Held, however long since its last run, while the worktree exists.
    let out = sluice_in(s.repo.dir(), &["status"]);
    assert!(out.status.success(), "{}", text(&out));
    let listed = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(listed.contains(canon_a.to_str().unwrap()), "{listed}");
    assert!(
        listed.contains(&format!("SLUICE_PORT_WEB={a_port}")),
        "{listed}"
    );
    assert!(listed.contains("last run 20"), "{listed}");

    // Its runs ran the toolchain in it, as every run does.
    let ran_in = |log: &Path| std::fs::read_to_string(log).unwrap_or_default();
    assert!(ran_in(&tooled).contains(canon_a.to_str().unwrap()));
    std::fs::remove_file(&tooled).unwrap();

    // Removed with nothing released: still held, and listed as removed.
    std::fs::remove_dir_all(&s.worktree).unwrap();
    let out = sluice_in(s.repo.dir(), &["status", "--json"]);
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["worktrees"][0]["removed"], true, "{doc}");
    assert!(!released.exists());

    // The next run, in another worktree, runs its stored release first,
    // once, from the scratch directory its record names with its port, and
    // frees it (123, rule core.30).
    let record: serde_json::Value = serde_json::from_slice(
        &std::fs::read(s.repo.dir().join(format!(
            ".git/sluice/.places/{}.json",
            sluice::runs::flat_name(&format!("session/{}", canon_a.display()))
        )))
        .unwrap(),
    )
    .unwrap();
    let scratch = std::fs::canonicalize(record["scratch"].as_str().unwrap()).unwrap();
    let out = sluice_in(&other, &["exec", "bolt/x", "--", "true"]);
    assert!(out.status.success(), "{}", text(&out));
    let said = text(&out);
    assert!(
        said.contains(&format!(
            "freed {}: it no longer exists; its release ran once from its scratch directory; its runs ended, its ports free",
            canon_a.display()
        )),
        "{said}"
    );
    let once = format!("{} {a_port}\n", scratch.display());
    assert_eq!(ran_in(&released), once);
    assert!(!scratch.exists(), "{} left", scratch.display());
    assert!(
        !ran_in(&tooled).contains(canon_a.to_str().unwrap())
            && !ran_in(&tooled).contains(scratch.to_str().unwrap()),
        "the toolchain ran in its name: {}",
        ran_in(&tooled)
    );

    // Its block of ports is free: the next worktree takes it.
    assert_eq!(port(&other), a_port);
    let out = sluice_in(s.repo.dir(), &["status", "--json"]);
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let held = doc["worktrees"].as_array().unwrap();
    assert_eq!(held.len(), 1, "{doc}");
    assert_eq!(
        held[0]["worktree"],
        std::fs::canonicalize(&other).unwrap().to_str().unwrap()
    );
    assert_eq!(held[0]["removed"], false);
    // Logged gone where the branch's releases are, naming the worktree,
    // and never named again.
    let log = std::fs::read_to_string(s.repo.dir().join(".git/sluice/bolt/x/log.jsonl")).unwrap();
    let named: Vec<&str> = log
        .lines()
        .filter(|l| l.contains(&format!("\"worktree\":\"{}\"", canon_a.display())))
        .collect();
    assert_eq!(named.len(), 1, "{log}");
    assert!(named[0].contains("\"event\":\"gone\""), "{log}");
    assert!(named[0].contains("\"ran\":true"), "{log}");
    let out = sluice_in(&other, &["exec", "bolt/x", "--", "true"]);
    assert!(
        !text(&out).contains(canon_a.to_str().unwrap()),
        "{}",
        text(&out)
    );
    assert_eq!(ran_in(&released), once, "its release ran again");
}

/// Session-runs-1 clauses 92, 97, rule core.18: each run leaves on the
/// worktree's entry in `sluice status --json` and `sluice overview --json`
/// the head and outcome of the last run there, recorded only for a clean
/// run of the checks; the next run replaces it and the release removes it.
#[test]
fn a_worktrees_last_result_says_its_head_outcome_and_whether_it_was_recorded() {
    let s = Setup::new("exec-last-result");
    let head = |s: &Setup| s.repo.git(&["rev-parse", "unit/a"]).trim().to_string();
    let git = |args: &[&str]| {
        let mut a = vec!["-C", s.worktree.to_str().unwrap()];
        a.extend(args);
        s.repo.git(&a)
    };
    let held = |s: &Setup| -> serde_json::Value {
        let out = s.sluice(&["status", "--json"]);
        assert!(out.status.success(), "{}", text(&out));
        let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        doc["worktrees"][0].clone()
    };
    let last = |s: &Setup| held(s)["last_result"].clone();
    let expect = |s: &Setup, head: &str, kind: &str, outcome: &str, recorded: bool| {
        let l = last(s);
        assert_eq!(l["head"], head, "{l}");
        assert_eq!(l["kind"], kind, "{l}");
        assert_eq!(l["outcome"], outcome, "{l}");
        assert_eq!(l["recorded"], recorded, "{l}");
        assert!(l["at"].as_str().unwrap().starts_with("20"), "{l}");
    };

    // A clean pass, recorded; the overview lists the same.
    let out = s.sluice(&["exec", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    let passed = head(&s);
    expect(&s, &passed, "checks", "pass", true);
    let entry = held(&s);
    for field in ["last_run", "ports", "removed", "worktree", "branch"] {
        assert!(!entry[field].is_null(), "{field}: {entry}");
    }
    let out = s.sluice(&["overview", "--json"]);
    assert!(out.status.success(), "{}", text(&out));
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(doc["sessions"][0], entry, "{doc}");
    let out = s.sluice(&["status"]);
    let said = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        said.contains(&format!("last result passed on {}, recorded", &passed[..7])),
        "{said}"
    );

    // A clean fail, recorded, replaces it.
    std::fs::write(s.worktree.join("check.sh"), "exit 1\n").unwrap();
    git(&["commit", "-q", "-am", "a failing check"]);
    let out = s.sluice(&["exec", "bolt/x"]);
    assert!(!out.status.success(), "{}", text(&out));
    let failed = head(&s);
    expect(&s, &failed, "checks", "fail", true);

    // An unclean pass is not recorded.
    std::fs::write(s.worktree.join("check.sh"), "true\n").unwrap();
    git(&["commit", "-q", "-am", "a passing check"]);
    std::fs::write(s.worktree.join("a.txt"), "changed").unwrap();
    let before = s.results();
    let out = s.sluice(&["exec", "bolt/x"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(s.results(), before);
    expect(&s, &head(&s), "checks", "pass", false);

    // Nor is a command that exits 0.
    let out = s.sluice(&["exec", "bolt/x", "--", "true"]);
    assert!(out.status.success(), "{}", text(&out));
    expect(&s, &head(&s), "command", "pass", false);
    let out = s.sluice(&["status"]);
    let said = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(said.contains("last result command passed on"), "{said}");

    // A run its time limit ends.
    s.repo.write(
        ".config/sluice.toml",
        &POLICY.replace("timeout = \"1m\"", "timeout = \"1s\""),
    );
    s.repo.git(&["commit", "-q", "-am", "a short time limit"]);
    s.repo.git(&["branch", "-f", "bolt/x", "main"]);
    let out = s.sluice(&["exec", "bolt/x", "--", "sleep", "30"]);
    assert!(!out.status.success(), "{}", text(&out));
    expect(&s, &head(&s), "command", "timed_out", false);

    // The release removes it.
    let out = s.sluice(&["release"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(held(&s).is_null(), "released");
}

/// The policy with no ports, so a test of the host's bound probes none.
fn portless() -> String {
    POLICY.replace("ports = [\"web\"]\nports_from = \"41500\"\n", "")
}

/// 71, 89; the exec-checks spec's "An agent waits on another repository's
/// queue" and the run-bound spec's "Another repository's agent run holds
/// the bound": on a host whose bound is one, another repository's `sluice
/// exec` holds the one run; this repository's waits, saying so, and runs
/// once that one ends.
#[test]
fn an_exec_waits_for_another_repositorys_run_on_the_host() {
    let a = Setup::at("exec-host-a", "a");
    let b = Setup::at("exec-host-b", "b");
    a.policy(&portless());
    b.policy(&portless());
    host_bound(&a.config, 1);

    let mut holding = b.spawn_on(
        &a,
        &[
            "exec",
            "bolt/x",
            "--",
            "sh",
            "-c",
            "touch started; while [ ! -e go ]; do sleep 0.05; done",
        ],
    );
    eventually("the other repository's run starting", || {
        b.worktree.join("started").exists()
    });

    let mut waiting = a.spawn_on(&a, &["exec", "bolt/x", "--", "sh", "-c", "echo ran > ran"]);
    let said = lines_of(&mut waiting);
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        let line = said
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .expect("exec says it waits");
        if line == "waiting: the host is at its bound" {
            break;
        }
    }
    assert!(!a.worktree.join("ran").exists(), "it waits before it runs");

    std::fs::write(b.worktree.join("go"), "").unwrap();
    assert!(holding.wait().unwrap().success());
    assert!(waiting.wait().unwrap().success());
    assert_eq!(
        std::fs::read_to_string(a.worktree.join("ran")).unwrap(),
        "ran\n"
    );
}

/// 71; the exec-checks spec's "Another repository's submission": another
/// repository's combination running on the host counts in this
/// repository's `room.queue`, and no branch here shows it under test.
#[test]
fn the_overview_counts_another_repositorys_combination_in_its_room() {
    let a = Setup::at("exec-room-a", "a");
    let b = Setup::at("exec-room-b", "b");
    let marks = Scratch::new("exec-room-marks");
    let (started, go) = (marks.path().join("started"), marks.path().join("go"));
    a.policy(&portless());
    b.policy(&portless());
    b.repo.write(
        "check.sh",
        &format!(
            "touch '{}'; while [ ! -e '{}' ]; do sleep 0.05; done\n",
            started.display(),
            go.display()
        ),
    );
    b.repo.git(&["commit", "-q", "-am", "a check that holds"]);
    b.repo.git(&["branch", "-f", "bolt/x", "main"]);
    host_bound(&a.config, 3);

    let on_host = |s: &Setup, args: &[&str]| {
        let out = s.spawn_on(&a, args).wait_with_output().unwrap();
        assert!(out.status.success(), "{}", text(&out));
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    on_host(&b, &["submit", "bolt/x", "--no-wait", "--json"]);
    on_host(&b, &["step", "bolt/x"]);
    eventually("the other repository's combination running", || {
        started.exists()
    });

    let doc: serde_json::Value =
        serde_json::from_str(&on_host(&a, &["overview", "--json"])).unwrap();
    std::fs::write(&go, "").unwrap();
    assert_eq!(doc["room"]["bound"], 3, "{doc}");
    assert_eq!(doc["room"]["queue"], 1, "{doc}");
    assert_eq!(doc["room"]["sessions"], 0, "{doc}");
    for branch in doc["branches"].as_array().unwrap() {
        assert_eq!(branch["candidates"], serde_json::json!([]), "{doc}");
    }
}

/// 114, 139; record 0074 (design D5); the host-requirements spec's "The
/// engine stopped during the run": a check that takes the engine away and
/// fails is not run, with why, and nothing is recorded under its key;
/// the check that passed beside it keeps its pass.
#[test]
fn a_check_that_fails_after_the_engine_went_is_not_run_and_a_pass_is_recorded() {
    let dir = Scratch::short();
    let socket = support::engine(dir.path(), "HTTP/1.0 200 OK");
    let s = moon_setup(
        "exec-engine-gone",
        &format!("rm -f '{}'\nexit 1\n", socket.display()),
    );
    let out = s.sluice_with(&[(ENGINE_SOCKETS, socket.as_path())], &["exec", "bolt/x"]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(1), "{said}");
    assert!(
        said.contains(&format!(
            "could not run: a:test: the container engine no longer answers after the run: \
             no socket at {}",
            socket.display()
        )),
        "{said}"
    );
    assert_eq!(
        recorded(&s),
        [("b:test".to_string(), "pass".to_string())],
        "{said}"
    );
}

/// Record 0074; the host-requirements spec's "A test that says docker is
/// down": what a check prints is never read as the engine gone; while
/// the engine answers after the run, its fail is recorded under its key.
#[test]
fn a_check_that_says_the_engine_is_down_while_it_answers_is_recorded_as_failed() {
    let dir = Scratch::short();
    let socket = support::engine(dir.path(), "HTTP/1.0 200 OK");
    let s = moon_setup(
        "exec-engine-says",
        "echo 'Cannot connect to the Docker daemon at unix:///var/run/docker.sock. \
         Is the docker daemon running?'\nexit 1\n",
    );
    let out = s.sluice_with(&[(ENGINE_SOCKETS, socket.as_path())], &["exec", "bolt/x"]);
    let said = text(&out);
    assert_eq!(out.status.code(), Some(1), "{said}");
    assert!(said.contains("failed: a:test"), "{said}");
    let mut got = recorded(&s);
    got.sort();
    assert_eq!(
        got,
        [
            ("a:test".to_string(), "fail".to_string()),
            ("b:test".to_string(), "pass".to_string())
        ],
        "{said}"
    );
}
