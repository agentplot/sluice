//! A full stack per candidate, side by side (31, 32, 34, 66; ADR 0016):
//! each candidate's check starts a container on one of its place's ports
//! and a server on another, then a browser test against both, while
//! another candidate's stack runs beside it. The release command removes
//! the containers; a candidate ended by its time limit leaves no container
//! and no process. Needs git, a container engine, bun and Playwright with
//! its browser (sluice's devenv shell), so it is in the check-tools tier;
//! the sandbox's own run of it is in the sandbox task, whose tests start
//! a sandbox.

#![cfg(feature = "tools")]

#[path = "support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use support::{Repo, Scratch};

/// The check: a container and a server on the place's ports, both up and
/// the other candidate's container up beside them, then the browser test.
/// A candidate holding `hang` starts its stack and never finishes.
const CHECK: &str = r#"set -eu
docker run -d --rm --label "sluice.fixture=$FIXTURE" --label "sluice.place=$SLUICE_PLACE" \
  -p "127.0.0.1:$SLUICE_PORT_WEB:80" nginx:alpine >/dev/null
bun --eval 'Bun.serve({ hostname: "127.0.0.1", port: Number(process.env.SLUICE_PORT_API),
  fetch: () => new Response("<h1>" + process.env.SLUICE_PLACE + "</h1>",
    { headers: { "content-type": "text/html" } }) })' "api-$FIXTURE" &
up() {
  n=0
  until curl -fs -o /dev/null "http://127.0.0.1:$1/"; do
    n=$((n + 1)); [ "$n" -lt 300 ] || { echo "nothing answers on $1"; exit 1; }
    sleep 0.2
  done
}
up "$SLUICE_PORT_WEB"
up "$SLUICE_PORT_API"
if [ -f hang ]; then sleep 600; fi
n=0
until [ "$(docker ps -q --filter "label=sluice.fixture=$FIXTURE" | wc -l)" -ge 2 ]; do
  n=$((n + 1)); [ "$n" -lt 450 ] || { echo "the other stack never came up"; exit 1; }
  sleep 0.2
done
playwright test
"#;

const RELEASE: &str = r#"ids=$(docker ps -aq --filter "label=sluice.place=$SLUICE_PLACE")
[ -z "$ids" ] || docker rm -f $ids >/dev/null
"#;

const SPEC: &str = r#"const { test, expect } = require('@playwright/test');
test('the stack', async ({ page }) => {
  await page.goto(`http://127.0.0.1:${process.env.SLUICE_PORT_WEB}/`);
  await expect(page).toHaveTitle('Welcome to nginx!');
  await page.goto(`http://127.0.0.1:${process.env.SLUICE_PORT_API}/`);
  await expect(page.locator('h1')).toHaveText(process.env.SLUICE_PLACE);
});
"#;

const CONFIG: &str = r#"module.exports = {
  use: { browserName: 'chromium', channel: 'chromium' },
  reporter: 'line',
  outputDir: process.env.SLUICE_TMP + '/playwright',
};
"#;

fn policy(executor: &str) -> String {
    format!(
        "[tests]\nrunner = \"command\"\ncommand = \"sh check.sh\"\nrelease = \"sh release.sh\"\n\n\
         [checkout]\nmethod = \"git-worktree\"\nports = [\"web\", \"api\"]\n\n\
         [executor]\nkind = \"{executor}\"\ndocker = true\n{network}\n\
         [branches.\"stack/*\"]\nparallel = 2\ntimeout = \"3m\"\n\n\
         [branches.\"hang/*\"]\ntimeout = \"20s\"\n",
        network = if executor == "sandbox" {
            "network = \"localhost\"\n"
        } else {
            ""
        },
    )
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

struct Setup {
    repo: Repo,
    sessions: Scratch,
    config: Scratch,
    state: Scratch,
}

impl Setup {
    fn command(&self, dir: &Path) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_sluice"));
        c.current_dir(dir)
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("XDG_STATE_HOME", self.state.path())
            .env_remove("SLUICE_AGENT")
            .env_remove("SLUICE_OTEL")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null());
        c
    }

    /// A session on `unit/<name>` off `branch`, with one commit adding
    /// `file`, submitted without waiting.
    fn submit(&self, branch: &str, name: &str, file: &str) {
        let dir: PathBuf = self.sessions.path().join(name);
        self.repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("unit/{name}"),
            dir.to_str().unwrap(),
            branch,
        ]);
        std::fs::write(dir.join(file), name).unwrap();
        let git = |args: &[&str]| {
            let mut a = vec!["-C", dir.to_str().unwrap()];
            a.extend(args);
            self.repo.git(&a)
        };
        git(&["add", "-A"]);
        git(&["commit", "-q", "-m", name]);
        let out = self
            .command(&dir)
            .args(["submit", branch, "--no-wait", "--json"])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
    }

    /// The branch's status, for a failed assertion's message.
    fn status(&self, branch: &str) -> String {
        let out = self
            .command(self.repo.dir())
            .args(["status", branch, "--json"])
            .output()
            .unwrap();
        let mut t = text(&out);
        let status: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
        for log in status
            .to_string()
            .split('"')
            .filter(|p| p.ends_with(".log"))
        {
            t.push_str(&format!(
                "\n--- {log}\n{}",
                std::fs::read_to_string(log).unwrap_or_default()
            ));
        }
        t
    }

    fn states(&self, branch: &str) -> Vec<String> {
        let out = self
            .command(self.repo.dir())
            .args(["status", branch, "--json"])
            .output()
            .unwrap();
        let status: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        status["submissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["state"].as_str().unwrap().to_string())
            .collect()
    }
}

/// What is left of the fixture `id`: its containers, and its servers.
fn leftovers(id: &str) -> (String, String) {
    let containers = Command::new("docker")
        .args([
            "ps",
            "-aq",
            "--filter",
            &format!("label=sluice.fixture={id}"),
        ])
        .output()
        .expect("docker on PATH");
    let servers = Command::new("pgrep")
        .args(["-f", &format!("api-{id}")])
        .output()
        .expect("pgrep");
    (
        String::from_utf8_lossy(&containers.stdout)
            .trim()
            .to_string(),
        String::from_utf8_lossy(&servers.stdout).trim().to_string(),
    )
}

fn stacks(executor: &str) {
    let id = format!("{}-{executor}", std::process::id());
    let repo = Repo::new("stack");
    let check = format!(
        "export FIXTURE={id}\nexport PLAYWRIGHT_BROWSERS_PATH={}\n{CHECK}",
        std::env::var("PLAYWRIGHT_BROWSERS_PATH")
            .expect("PLAYWRIGHT_BROWSERS_PATH, from sluice's devenv shell"),
    );
    repo.write(".config/sluice.toml", &policy(executor));
    repo.write("check.sh", &check);
    repo.write("release.sh", RELEASE);
    repo.write("stack.spec.js", SPEC);
    repo.write("playwright.config.js", CONFIG);
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "a stack per candidate"]);
    for b in ["stack/a", "stack/b", "hang/c"] {
        repo.git(&["branch", b, "main"]);
    }
    let config = Scratch::new("stack-config");
    std::fs::create_dir_all(config.path().join("sluice")).unwrap();
    std::fs::write(
        config.path().join("sluice/config.toml"),
        "[runs]\nmax = 3\n",
    )
    .unwrap();
    let s = Setup {
        repo,
        sessions: Scratch::new("stack-sessions"),
        config,
        state: Scratch::new("stack-state"),
    };
    s.submit("stack/a", "a", "a.txt");
    s.submit("stack/b", "b", "b.txt");
    s.submit("hang/c", "c", "hang");

    let out = s
        .command(s.repo.dir())
        .args(["run", "--rounds", "40"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));

    // 31: two stacks on their own ports, up at once, each test reaching
    // its own; both land.
    assert_eq!(s.states("stack/a"), ["landed"], "{}", s.status("stack/a"));
    assert_eq!(s.states("stack/b"), ["landed"], "{}", s.status("stack/b"));
    // 66: the hung candidate was ended by its time limit.
    assert_eq!(s.states("hang/c"), ["failed"], "{}", text(&out));
    // 32, 34: nothing any of them started is left: no container, no
    // server; and every slot of ports is free again.
    assert_eq!(leftovers(&id), (String::new(), String::new()));
    let slots = s.state.path().join("sluice/slots");
    let held: Vec<String> = std::fs::read_dir(&slots)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.'))
        .collect();
    assert!(held.is_empty(), "slots still held: {held:?}");
}

#[test]
fn two_stacks_side_by_side_and_a_time_limit_leaves_nothing() {
    stacks("open");
}

/// The same under the sandbox, which lets the run reach the container
/// engine and start a browser, and holds its network to localhost.
#[test]
#[cfg(target_os = "macos")]
fn two_stacks_side_by_side_in_the_sandbox() {
    stacks("sandbox");
}
