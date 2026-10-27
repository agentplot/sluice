//! The `pushed refs` profile and the router that chooses it, against a
//! bare repository as the remote, which git treats as it treats a forge
//! (C.4, C.7 `remote`). The check-tools tier: run with `--features tools`.

#![cfg(feature = "tools")]

mod support;

use std::cell::Cell;
use std::path::PathBuf;
use std::process::Stdio;
use std::rc::Rc;

use sluice::identity::{Credentials, Identity};
use sluice::profiles::branches::Branches;
use sluice::profiles::pushed_refs::PushedRefs;
use sluice_seams::{read_refused, Branch, HostCopy, LandingCommit, Moved, ReadRefused, Rev};
use support::http::Fake;
use support::Repo;

fn rev(s: String) -> Rev {
    Rev::new(s)
}

/// A landing commit of `head` onto `onto`, as the core writes one.
fn landing(b: &impl Branch, onto: &Rev, head: &Rev) -> Rev {
    let Ok(sluice_seams::Formed::Revision(c)) = b.form(onto, head) else {
        panic!("formed no candidate");
    };
    b.commit(&LandingCommit {
        parent: onto.clone(),
        head: head.clone(),
        tree_of: c,
        message: "land\n".into(),
    })
    .unwrap()
}

/// 42: a read sees what another clone pushed, and moves no local branch
/// (rule core.4).
#[test]
fn a_read_sees_what_another_clone_pushed() {
    let repo = Repo::new("read");
    let bare = repo.bare_origin();
    let before = repo.rev("main");
    let other = Repo::clone_of(bare.path(), "other");
    let pushed = other.push_change("b", "2\n");
    let p = PushedRefs::open(repo.dir(), "origin").unwrap();
    assert_eq!(p.read("main").unwrap(), Some(rev(pushed.clone())));
    assert_eq!(repo.rev("main"), before);
    assert_eq!(repo.rev("refs/remotes/origin/main"), pushed);
    assert_eq!(p.read("none").unwrap(), None);
}

/// 45, 46, 124: a push the lease allows lands on the remote, and leaves
/// the host's copy where it was, for a round to level.
#[test]
fn a_push_lands_and_leaves_the_copy() {
    let repo = Repo::new("push");
    let bare = repo.bare_origin();
    let head = rev(repo.branch("unit/u", "main", &[("a", Some("1"))]));
    let p = PushedRefs::open(repo.dir(), "origin").unwrap();
    let base = p.read("main").unwrap().unwrap();
    let to = landing(&p, &base, &head);
    assert_eq!(p.move_to("main", &base, &to).unwrap(), Moved::Moved);
    let remote = Repo::clone_of(bare.path(), "check");
    assert_eq!(remote.rev("origin/main"), to.as_str());
    assert_eq!(repo.rev("main"), base.as_str());
}

/// 122, 124: a push accepted while git holds the copy's lock is a
/// landing all the same; the copy is left where it was.
#[test]
fn a_push_accepted_under_the_copys_lock_is_moved() {
    let repo = Repo::new("push-locked");
    let bare = repo.bare_origin();
    let head = rev(repo.branch("unit/u", "main", &[("a", Some("1"))]));
    let p = PushedRefs::open(repo.dir(), "origin").unwrap();
    let base = p.read("main").unwrap().unwrap();
    let to = landing(&p, &base, &head);
    let lock = repo.dir().join(".git/refs/heads/main.lock");
    std::fs::write(&lock, "").unwrap();
    assert_eq!(p.move_to("main", &base, &to).unwrap(), Moved::Moved);
    std::fs::remove_file(&lock).unwrap();
    let remote = Repo::clone_of(bare.path(), "check");
    assert_eq!(remote.rev("origin/main"), to.as_str());
    assert_eq!(repo.rev("main"), base.as_str());
}

/// 24, rule core.5: a push after another clone moved the branch is
/// refused and reports what the remote holds.
#[test]
fn a_push_after_the_branch_moved_is_elsewhere() {
    let repo = Repo::new("moved");
    let bare = repo.bare_origin();
    let head = rev(repo.branch("unit/u", "main", &[("a", Some("1"))]));
    let p = PushedRefs::open(repo.dir(), "origin").unwrap();
    let base = p.read("main").unwrap().unwrap();
    let to = landing(&p, &base, &head);
    let other = Repo::clone_of(bare.path(), "other");
    let moved = other.push_change("b", "2\n");
    assert_eq!(
        p.move_to("main", &base, &to).unwrap(),
        Moved::Elsewhere(Some(rev(moved.clone())))
    );
    let remote = Repo::clone_of(bare.path(), "check");
    assert_eq!(remote.rev("origin/main"), moved);
    assert_eq!(repo.rev("main"), base.as_str());
}

/// A remote that cannot be reached: the read and the push fail, and the
/// push moves nothing.
#[test]
fn an_unreachable_remote_fails_the_read_and_the_push() {
    let repo = Repo::new("unreachable");
    let bare = repo.bare_origin();
    let head = rev(repo.branch("unit/u", "main", &[("a", Some("1"))]));
    let p = PushedRefs::open(repo.dir(), "origin").unwrap();
    let base = p.read("main").unwrap().unwrap();
    let to = landing(&p, &base, &head);
    let aside = bare.path().with_extension("aside");
    std::fs::rename(bare.path(), &aside).unwrap();
    let read = p.read("main").unwrap_err().to_string();
    assert!(read.contains("fetching main from origin failed"), "{read}");
    let push = p.move_to("main", &base, &to).unwrap_err().to_string();
    assert!(push.contains("pushing main to origin failed"), "{push}");
    std::fs::rename(&aside, bare.path()).unwrap();
    assert_eq!(repo.rev("main"), base.as_str());
    assert_eq!(p.read("main").unwrap(), Some(base));
}

/// 133: a fetch from a remote whose path is not there fails as a refused
/// read naming the remote, the ref asked for and git's words unchanged, as
/// git prints them for the same fetch.
#[test]
fn a_fetch_from_a_remote_not_there_is_a_refused_read() {
    let repo = Repo::new("not-there");
    let bare = repo.bare_origin();
    let p = PushedRefs::open(repo.dir(), "origin").unwrap();
    std::fs::rename(bare.path(), bare.path().with_extension("aside")).unwrap();
    let err = p.read("main").unwrap_err();
    let refused = read_refused(&err).expect("a refused read");
    let git = repo.git_raw(&[
        "fetch",
        "--quiet",
        "--no-tags",
        "--no-write-fetch-head",
        "origin",
        "+refs/heads/main:refs/remotes/origin/main",
    ]);
    assert!(!git.status.success());
    assert_eq!(
        refused,
        &ReadRefused {
            remote: "origin".into(),
            asked: "refs/heads/main".into(),
            said: String::from_utf8_lossy(&git.stderr).trim_end().into(),
            note: None,
        }
    );
    assert!(refused
        .said
        .contains("does not appear to be a git repository"));
    assert_eq!(
        err.to_string(),
        format!("fetching main from origin failed: {}", refused.said)
    );
}

/// A push the remote refuses names the remote's reason, which git's own
/// error leaves out: a forge refusing an app's change to its workflows
/// says so only there.
#[test]
fn a_refused_push_names_the_remotes_reason() {
    use std::os::unix::fs::PermissionsExt;
    let repo = Repo::new("refused");
    let bare = repo.bare_origin();
    let hook = bare.path().join("hooks/pre-receive");
    std::fs::create_dir_all(hook.parent().unwrap()).unwrap();
    std::fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    let head = rev(repo.branch("unit/u", "main", &[("a", Some("1"))]));
    let p = PushedRefs::open(repo.dir(), "origin").unwrap();
    let base = p.read("main").unwrap().unwrap();
    let err = p.push_branch(&head, "unit/u").unwrap_err().to_string();
    assert!(err.contains("pre-receive hook declined"), "{err}");
    let to = landing(&p, &base, &head);
    let err = p.move_to("main", &base, &to).unwrap_err().to_string();
    assert!(err.contains("pre-receive hook declined"), "{err}");
    assert_eq!(repo.rev("main"), base.as_str());
}

/// 124: the host's copy is its own `refs/heads/<branch>`, read with no
/// fetch, so with the remote unreachable.
#[test]
fn the_copy_is_read_with_the_remote_unreachable() {
    let repo = Repo::new("copy-read");
    let bare = repo.bare_origin();
    let base = rev(repo.rev("main"));
    let other = Repo::clone_of(bare.path(), "other");
    other.push_change("b", "2\n");
    let p = PushedRefs::open(repo.dir(), "origin").unwrap();
    let aside = bare.path().with_extension("aside");
    std::fs::rename(bare.path(), &aside).unwrap();
    assert_eq!(p.copy("main").unwrap(), HostCopy::At(Some(base)));
    assert_eq!(p.copy("none").unwrap(), HostCopy::At(None));
    std::fs::rename(&aside, bare.path()).unwrap();
}

/// 124, rule core.5: a levelling from the value the copy holds sets it,
/// with sluice's reflog line; one from any other value, or from none
/// while the copy exists, is reported with what it holds and moves
/// nothing.
#[test]
fn a_copy_is_levelled_only_from_the_value_it_holds() {
    let repo = Repo::new("copy-level");
    let bare = repo.bare_origin();
    let base = rev(repo.rev("main"));
    let stale = rev(repo.branch("unit/u", "main", &[("a", Some("1"))]));
    let other = Repo::clone_of(bare.path(), "other");
    let pushed = rev(other.push_change("b", "2\n"));
    let p = PushedRefs::open(repo.dir(), "origin").unwrap();
    assert_eq!(p.read("main").unwrap(), Some(pushed.clone()));
    assert_eq!(
        p.level("main", Some(&stale), &pushed).unwrap(),
        Moved::Elsewhere(Some(base.clone()))
    );
    assert_eq!(
        p.level("main", None, &pushed).unwrap(),
        Moved::Elsewhere(Some(base.clone()))
    );
    assert_eq!(repo.rev("main"), base.as_str());
    assert_eq!(p.level("main", Some(&base), &pushed).unwrap(), Moved::Moved);
    assert_eq!(repo.rev("main"), pushed.as_str());
    assert_eq!(
        repo.git(&["reflog", "show", "-1", "--format=%gs", "refs/heads/main"]),
        "sluice: level main to its landing"
    );
    assert_eq!(p.level("bolt/y", None, &pushed).unwrap(), Moved::Moved);
    assert_eq!(repo.rev("bolt/y"), pushed.as_str());
}

/// 124: a levelling while git holds the copy's lock fails, saying so, and
/// moves nothing.
#[test]
fn a_levelling_fails_on_the_copys_lock() {
    let repo = Repo::new("copy-lock");
    let bare = repo.bare_origin();
    let base = rev(repo.rev("main"));
    let other = Repo::clone_of(bare.path(), "other");
    let pushed = rev(other.push_change("b", "2\n"));
    let p = PushedRefs::open(repo.dir(), "origin").unwrap();
    p.read("main").unwrap();
    let lock = repo.dir().join(".git/refs/heads/main.lock");
    std::fs::write(&lock, "").unwrap();
    let err = format!("{:#}", p.level("main", Some(&base), &pushed).unwrap_err());
    assert!(
        err.contains("git holds the lock on refs/heads/main"),
        "{err}"
    );
    assert_eq!(repo.rev("main"), base.as_str());
    std::fs::remove_file(&lock).unwrap();
    assert_eq!(p.level("main", Some(&base), &pushed).unwrap(), Moved::Moved);
}

/// C.7, 124: the router sends `copy` and `level` where it sends `read`.
#[test]
fn the_router_sends_the_copy_to_the_branchs_profile() {
    let repo = Repo::new("router-copy");
    repo.write(
        ".config/sluice.toml",
        "[branches.main]\nremote = \"origin\"\n[branches.\"bolt/*\"]\n",
    );
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "policy"]);
    repo.git(&["branch", "bolt/x"]);
    let _bare = repo.bare_origin();
    let base = rev(repo.rev("main"));
    let b = Branches::open(repo.dir()).unwrap();
    assert_eq!(b.copy("main").unwrap(), HostCopy::At(Some(base.clone())));
    assert_eq!(b.copy("bolt/x").unwrap(), HostCopy::Itself);
    assert!(b.level("bolt/x", Some(&base), &base).is_err());
}

/// C.7: a branch with no `remote` is local refs, exactly as before.
#[test]
fn the_router_keeps_a_branch_without_a_remote_local() {
    let repo = Repo::new("router-local");
    repo.write(".config/sluice.toml", "[branches.main]\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "policy"]);
    let bare = repo.bare_origin();
    let base = rev(repo.rev("main"));
    let other = Repo::clone_of(bare.path(), "other");
    other.push_change("b", "2\n");
    let head = rev(repo.branch("unit/u", "main", &[("a", Some("1"))]));
    let b = Branches::open(repo.dir()).unwrap();
    assert_eq!(b.read("main").unwrap(), Some(base.clone()));
    let to = landing(&b, &base, &head);
    assert_eq!(b.move_to("main", &base, &to).unwrap(), Moved::Moved);
    assert_eq!(repo.rev("main"), to.as_str());
    assert_ne!(
        Repo::clone_of(bare.path(), "check").rev("origin/main"),
        to.as_str()
    );
}

/// C.7: a branch whose `remote` names a remote reads and lands through
/// it; one naming a remote the repository lacks is refused by name.
#[test]
fn the_router_sends_a_branch_to_its_remote() {
    let repo = Repo::new("router-remote");
    repo.write(
        ".config/sluice.toml",
        "[branches.main]\nremote = \"origin\"\n[branches.\"bolt/*\"]\nremote = \"nowhere\"\n",
    );
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "policy"]);
    repo.git(&["branch", "bolt/x"]);
    let bare = repo.bare_origin();
    let other = Repo::clone_of(bare.path(), "other");
    let pushed = other.push_change("b", "2\n");
    let b = Branches::open(repo.dir()).unwrap();
    assert_eq!(b.read("main").unwrap(), Some(rev(pushed)));
    let err = format!("{:#}", b.read("bolt/x").unwrap_err());
    assert!(err.contains("\"nowhere\""), "{err}");
}

/// An identity whose remote is a fake HTTP server, with a token of its
/// own: `token`, then `token-1`, `token-2` and on as it is renewed.
struct FakeIdentity {
    url: String,
    token: String,
    renewed: Cell<usize>,
}

impl FakeIdentity {
    fn new(url: &str, token: &str) -> FakeIdentity {
        FakeIdentity {
            url: url.into(),
            token: token.into(),
            renewed: Cell::new(0),
        }
    }

    /// The token it gives after `renewed` renewals.
    fn token(&self, renewed: usize) -> String {
        match renewed {
            0 => self.token.clone(),
            n => format!("{}-{n}", self.token),
        }
    }
}

impl Identity for FakeIdentity {
    fn name(&self) -> String {
        "a fake identity".into()
    }

    fn url(&self, _remote_url: &str) -> Option<String> {
        Some(self.url.clone())
    }

    fn credentials(&self, url: &str) -> anyhow::Result<Credentials> {
        assert_eq!(url, self.url);
        let n = self.renewed.get();
        Ok(Credentials {
            username: "x-fake".into(),
            password: self.token(n),
            about: format!("the fake's token number {n}"),
        })
    }

    fn renew(&self, url: &str) -> anyhow::Result<Credentials> {
        self.renewed.set(self.renewed.get() + 1);
        self.credentials(url)
    }
}

/// The `Authorization` header git sends with the fake's `token`.
fn basic(token: &str) -> String {
    format!("Basic {}", base64(format!("x-fake:{token}").as_bytes()))
}

/// The `Authorization` headers the fake remote was sent, in order.
fn given(fake: &Fake) -> Vec<String> {
    fake.requests()
        .iter()
        .filter_map(|r| r.header("Authorization").map(String::from))
        .collect()
}

/// What git is answered with no credentials: a 401 asking for some.
fn ask() -> support::http::Answer {
    (
        401,
        vec![("WWW-Authenticate".into(), "Basic realm=\"fake\"".into())],
        String::new(),
    )
}

/// GitHub's answer to git with a token that may not read the repository.
fn not_found() -> support::http::Answer {
    (
        404,
        vec![("Content-Type".into(), "text/plain".into())],
        "Repository not found.\n".into(),
    )
}

/// A repository with nothing in it, as git's smart HTTP lists one.
fn empty_repository() -> support::http::Answer {
    let pkt = |s: &str| format!("{:04x}{s}", s.len() + 4);
    let zero = "0".repeat(40);
    let body = format!(
        "{}0000{}0000",
        pkt("# service=git-upload-pack\n"),
        pkt(&format!("{zero} capabilities^{{}}\0multi_ack\n"))
    );
    (
        200,
        vec![(
            "Content-Type".into(),
            "application/x-git-upload-pack-advertisement".into(),
        )],
        body,
    )
}

/// The pushed refs of `repo` reached through `fake` as `identity`.
fn as_identity(repo: &Repo, identity: &Rc<FakeIdentity>) -> PushedRefs {
    PushedRefs::open_as(
        repo.dir(),
        "origin",
        Some((
            identity.clone() as Rc<dyn Identity>,
            PathBuf::from(env!("CARGO_BIN_EXE_sluice")),
        )),
    )
    .unwrap()
}

fn base64(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

/// C.4: sluice's git calls get the identity's token through the helper
/// named on their own command line; nothing holds it afterwards, and git
/// run directly in a worktree gets nothing of sluice's.
#[test]
fn an_identity_reaches_only_sluices_git() {
    let repo = Repo::new("identity");
    let _bare = repo.bare_origin();
    // A remote that asks for credentials and refuses whatever it gets.
    let fake = Fake::start(|r| match r.header("Authorization") {
        Some(_) => (403, vec![], "refused\n".into()),
        None => ask(),
    });
    let token = format!("fake-token-{}", std::process::id());
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, &token));
    let p = as_identity(&repo, &identity);
    assert_eq!(p.identity().as_deref(), Some("a fake identity"));
    assert!(p.read("main").is_err(), "the fake refuses every fetch");
    let basic = basic(&token);
    assert_eq!(
        given(&fake),
        [basic.clone(), self::basic(&identity.token(1))],
        "git gave the fake identity's credentials, then fresh ones"
    );

    let config = std::fs::read_to_string(repo.dir().join(".git/config")).unwrap();
    assert!(!config.contains(&token), "{config}");
    assert!(std::env::vars().all(|(_, v)| !v.contains(&token)));

    let worktree = repo.dir().join("agent");
    repo.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "agent",
        worktree.to_str().unwrap(),
    ]);
    let before = fake.requests().len();
    let push = support::git_command()
        .current_dir(&worktree)
        .args(["push", &url, "HEAD:main"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!push.status.success());
    let seen = fake.requests();
    assert!(seen.len() > before, "the agent's push reached the remote");
    for r in &seen[before..] {
        let auth = r.header("Authorization").unwrap_or("");
        assert_ne!(auth, basic, "an agent's push carried sluice's token");
    }
}

/// A fetch the remote refuses the identity's credentials on is run once
/// more with credentials made afresh, and reads what the second run
/// finds.
#[test]
fn a_refused_fetch_is_run_again_with_fresh_credentials() {
    let repo = Repo::new("refused-once");
    let _bare = repo.bare_origin();
    let token = format!("fake-token-{}", std::process::id());
    let (first, fresh) = (basic(&token), basic(&format!("{token}-1")));
    let fake = Fake::start(move |r| match r.header("Authorization") {
        None => ask(),
        Some(a) if a == first => not_found(),
        Some(a) if a == fresh => empty_repository(),
        Some(_) => (403, vec![], "an unknown token\n".into()),
    });
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, &token));
    let p = as_identity(&repo, &identity);
    assert_eq!(p.read("main").unwrap(), None, "the fresh token reads");
    assert_eq!(
        given(&fake),
        [basic(&token), basic(&identity.token(1))],
        "the refused token, then a fresh one, once"
    );
}

/// A fetch refused on fresh credentials too fails with the remote's words
/// and what each refused credential was; it is run no third time.
#[test]
fn a_refusal_that_stands_names_both_credentials() {
    let repo = Repo::new("refused-twice");
    let _bare = repo.bare_origin();
    let fake = Fake::start(|r| match r.header("Authorization") {
        None => ask(),
        Some(_) => not_found(),
    });
    let token = format!("fake-token-{}", std::process::id());
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, &token));
    let p = as_identity(&repo, &identity);
    let read = p.read("main").unwrap_err();
    let err = format!("{read:#}");
    assert!(err.contains("Repository not found."), "{err}");
    let note = "origin refused the fake's token number 0, then the fake's token number 1";
    assert!(err.contains(note), "{err}");
    assert!(!err.contains(&token), "a report carries no token: {err}");
    assert_eq!(given(&fake).len(), 2, "run no third time");
    // 133: the note is sluice's, apart from what git said.
    let refused = read_refused(&read).expect("a refused read");
    assert_eq!(refused.asked, "refs/heads/main");
    assert!(
        refused.said.contains("Repository not found."),
        "{refused:?}"
    );
    assert!(!refused.said.contains("refused the fake's"), "{refused:?}");
    assert_eq!(refused.note.as_deref(), Some(note));
}

/// A push the remote refuses is never run again (rule core.5), and fails
/// with the remote's words and what the refused credential was.
#[test]
fn a_refused_push_is_not_run_again() {
    let repo = Repo::new("refused-push");
    let _bare = repo.bare_origin();
    let fake = Fake::start(|r| match r.header("Authorization") {
        None => ask(),
        Some(_) => not_found(),
    });
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, "fake-token"));
    let p = as_identity(&repo, &identity);
    let head = rev(repo.rev("HEAD"));
    let err = format!("{:#}", p.push_branch(&head, "agent").unwrap_err());
    assert!(err.contains("Repository not found."), "{err}");
    assert!(
        err.ends_with("origin refused the fake's token number 0"),
        "{err}"
    );
    assert_eq!(given(&fake).len(), 1, "one push, never again");
    assert_eq!(identity.renewed.get(), 0);
}

/// sluice runs unattended: a push to a remote that wants a credential the
/// host cannot give fails at once with an error, never a prompt or a wait.
#[test]
fn a_push_with_no_credential_fails_rather_than_waits() {
    let repo = Repo::new("no-credential");
    let fake = Fake::start(|_| {
        (
            401,
            vec![("WWW-Authenticate".into(), "Basic realm=\"fake\"".into())],
            String::new(),
        )
    });
    repo.git(&[
        "remote",
        "add",
        "origin",
        &format!("{}/o/r.git", fake.url()),
    ]);
    let head = rev(repo.rev("main"));
    let dir = repo.dir().to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let p = PushedRefs::open(&dir, "origin").unwrap();
        let read = p.read("main").map_err(|e| e.to_string());
        let push = p.move_to("main", &head, &head).map_err(|e| e.to_string());
        let _ = tx.send((read, push));
    });
    let (read, push) = rx
        .recv_timeout(std::time::Duration::from_secs(60))
        .expect("git waited instead of failing");
    let read = read.unwrap_err();
    assert!(read.contains("fetching main from origin failed"), "{read}");
    let push = push.unwrap_err();
    assert!(push.contains("pushing main to origin failed"), "{push}");
}
