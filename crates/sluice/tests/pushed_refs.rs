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
use sluice_seams::{Branch, HostCopy, LandingCommit, Moved, Rev};
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
/// own: `token`, then `token-1`, `token-2` and on as it is renewed, unless
/// its forge refuses to make another, saying `refuses`.
struct FakeIdentity {
    url: String,
    token: String,
    renewed: Cell<usize>,
    refuses: Option<String>,
}

impl FakeIdentity {
    fn new(url: &str, token: &str) -> FakeIdentity {
        FakeIdentity {
            url: url.into(),
            token: token.into(),
            renewed: Cell::new(0),
            refuses: None,
        }
    }

    /// The identity whose forge makes no fresh token, saying `why`.
    fn refusing(self, why: &str) -> FakeIdentity {
        FakeIdentity {
            refuses: Some(why.into()),
            ..self
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
        if let Some(why) = &self.refuses {
            anyhow::bail!("{why}");
        }
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
    listing("git-upload-pack", &[])
}

/// `s` as a pkt-line.
fn pkt(s: &str) -> String {
    format!("{:04x}{s}", s.len() + 4)
}

/// The fake's refs, each a ref and its revision, as git's smart HTTP lists
/// them for `service`: `git-upload-pack` for a fetch, `git-receive-pack`
/// for a push.
fn listing(service: &str, refs: &[(&str, &str)]) -> support::http::Answer {
    let caps = match service {
        "git-receive-pack" => "report-status delete-refs side-band-64k ofs-delta",
        _ => "multi_ack ofs-delta",
    };
    let zero = "0".repeat(40);
    let mut lines = match refs {
        [] => vec![format!("{zero} capabilities^{{}}")],
        _ => refs.iter().map(|(r, at)| format!("{at} {r}")).collect(),
    };
    lines[0] = format!("{}\0{caps}", lines[0]);
    let body = format!(
        "{}0000{}0000",
        pkt(&format!("# service={service}\n")),
        lines
            .iter()
            .map(|l| pkt(&format!("{l}\n")))
            .collect::<String>()
    );
    (
        200,
        vec![(
            "Content-Type".into(),
            format!("application/x-{service}-advertisement"),
        )],
        body,
    )
}

/// A push's commands, `(old, new, ref)` each, as git sends them at the
/// head of a `POST /git-receive-pack`.
fn commands(r: &support::http::Request) -> Vec<(String, String, String)> {
    let body = r.body.as_bytes();
    let mut at = 0;
    let mut all = Vec::new();
    while let Some(len) = body
        .get(at..at + 4)
        .and_then(|h| usize::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok())
    {
        if len < 4 {
            break;
        }
        let line = String::from_utf8_lossy(&body[at + 4..at + len]);
        let line = line.split('\0').next().unwrap().trim_end();
        let mut words = line.split(' ').map(String::from);
        if let (Some(old), Some(new), Some(name)) = (words.next(), words.next(), words.next()) {
            all.push((old, new, name));
        }
        at += len;
    }
    all
}

/// The fake's answer to a push it takes: every ref it names updated.
fn received(r: &support::http::Request) -> support::http::Answer {
    let ok: Vec<String> = commands(r)
        .iter()
        .map(|(_, _, name)| format!("ok {name}"))
        .collect();
    report(&ok, None)
}

/// The fake's report of a push: `unpack ok`, then `lines`, one for each
/// ref, on side band 1, after `said` on side band 2, which git prints on
/// stderr as the remote's.
fn report(lines: &[String], said: Option<&str>) -> support::http::Answer {
    let status: String = std::iter::once("unpack ok")
        .chain(lines.iter().map(String::as_str))
        .map(|l| pkt(&format!("{l}\n")))
        .collect();
    let said = said.map_or(String::new(), |s| pkt(&format!("\x02{s}\n")));
    (
        200,
        vec![(
            "Content-Type".into(),
            "application/x-git-receive-pack-result".into(),
        )],
        format!("{said}{}0000", pkt(&format!("\x01{status}0000"))),
    )
}

/// Whether `r` lists the fake's refs for a push.
fn lists_for_push(r: &support::http::Request) -> bool {
    r.method == "GET" && r.path.ends_with("?service=git-receive-pack")
}

/// Whether `r` is a push: its commands and its objects.
fn is_push(r: &support::http::Request) -> bool {
    r.method == "POST" && r.path.ends_with("/git-receive-pack")
}

/// The pushes the fake was sent, each its `Authorization` and its
/// commands, in order.
fn pushes(fake: &Fake) -> Vec<(String, Vec<(String, String, String)>)> {
    fake.requests()
        .iter()
        .filter(|r| is_push(r))
        .map(|r| (r.header("Authorization").unwrap_or("").into(), commands(r)))
        .collect()
}

/// The `Authorization` of each listing the fake gave a push, in order.
fn listed_for_push(fake: &Fake) -> Vec<String> {
    fake.requests()
        .iter()
        .filter(|r| lists_for_push(r))
        .filter_map(|r| r.header("Authorization").map(String::from))
        .collect()
}

/// A fake remote, answering each token as `answer` does for it; one that
/// gives none is asked for one.
fn remote(
    answer: impl Fn(&str, &support::http::Request) -> support::http::Answer + Send + Sync + 'static,
) -> Fake {
    Fake::start(move |r| match r.header("Authorization") {
        None => ask(),
        Some(a) => answer(a, r),
    })
}

/// The answer of a fake remote holding `refs` and taking every push.
fn takes(r: &support::http::Request, refs: &[(&str, &str)]) -> support::http::Answer {
    if lists_for_push(r) {
        listing("git-receive-pack", refs)
    } else if is_push(r) {
        received(r)
    } else {
        listing("git-upload-pack", refs)
    }
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
    let err = format!("{:#}", p.read("main").unwrap_err());
    assert!(err.contains("Repository not found."), "{err}");
    assert!(
        err.contains("origin refused the fake's token number 0, then the fake's token number 1"),
        "{err}"
    );
    assert!(!err.contains(&token), "a report carries no token: {err}");
    assert_eq!(given(&fake).len(), 2, "run no third time");
}

/// 134, C.5: a session's branch the remote refuses on the identity's first
/// token, as GitHub refuses one it has just given, is pushed once more on
/// a fresh one.
#[test]
fn a_push_refused_on_its_token_is_made_once_more() {
    let repo = Repo::new("push-renewed");
    let _bare = repo.bare_origin();
    let base = repo.rev("main");
    let head = repo.branch("unit/u", "main", &[("a", Some("1"))]);
    let first = basic("fake-token");
    let fake = {
        let first = first.clone();
        remote(move |a, r| match a == first {
            true => not_found(),
            false => takes(r, &[("refs/heads/main", &base)]),
        })
    };
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, "fake-token"));
    let p = as_identity(&repo, &identity);
    p.push_branch(&rev(head.clone()), "unit/u").unwrap();
    let fresh = basic(&identity.token(1));
    assert_eq!(given(&fake), [first, fresh.clone(), fresh.clone()]);
    let zero = "0".repeat(40);
    assert_eq!(
        pushes(&fake),
        [(fresh, vec![(zero, head, "refs/heads/unit/u".into())])]
    );
    assert_eq!(identity.renewed.get(), 1);
}

/// 134: a push refused on a fresh token too fails with the remote's words
/// and what each refused token was, and is made no third time.
#[test]
fn a_push_refused_twice_is_not_made_a_third_time() {
    let repo = Repo::new("refused-push");
    let _bare = repo.bare_origin();
    let fake = remote(|_, _| not_found());
    let token = format!("fake-token-{}", std::process::id());
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, &token));
    let p = as_identity(&repo, &identity);
    let head = rev(repo.rev("HEAD"));
    let err = format!("{:#}", p.push_branch(&head, "agent").unwrap_err());
    assert!(err.contains("Repository not found."), "{err}");
    assert!(
        err.ends_with("origin refused the fake's token number 0, then the fake's token number 1"),
        "{err}"
    );
    assert!(!err.contains(&token), "a report carries no token: {err}");
    assert_eq!(given(&fake).len(), 2, "two pushes, no third");
    assert_eq!(identity.renewed.get(), 1);
}

/// 134: a push refused on its token, when its forge makes no fresh one,
/// fails with the remote's words, what the refused token was and why none
/// could be made afresh, after one push.
#[test]
fn a_push_with_no_fresh_token_is_made_once() {
    let repo = Repo::new("push-unrenewed");
    let _bare = repo.bare_origin();
    let fake = remote(|_, _| not_found());
    let token = format!("fake-token-{}", std::process::id());
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, &token).refusing("the forge made no token"));
    let p = as_identity(&repo, &identity);
    let head = rev(repo.rev("HEAD"));
    let err = format!("{:#}", p.push_branch(&head, "agent").unwrap_err());
    assert!(err.contains("Repository not found."), "{err}");
    assert!(
        err.ends_with(
            "origin refused the fake's token number 0, \
             and none could be made afresh: the forge made no token"
        ),
        "{err}"
    );
    assert!(!err.contains(&token), "a report carries no token: {err}");
    assert_eq!(given(&fake).len(), 1, "one push");
    assert_eq!(identity.renewed.get(), 0);
}

/// 134, 45, 104, rule core.5: a leased push the remote refuses on the
/// identity's first token is made once more on a fresh one with the same
/// lease, and moves the branch; a branch made as a landing is.
#[test]
fn a_leased_push_refused_on_its_token_is_made_once_more() {
    let repo = Repo::new("leased-renewed");
    let _bare = repo.bare_origin();
    let base = repo.rev("main");
    let head = rev(repo.branch("unit/u", "main", &[("a", Some("1"))]));
    let local = PushedRefs::open(repo.dir(), "origin").unwrap();
    let to = landing(&local, &rev(base.clone()), &head);
    let zero = "0".repeat(40);
    for (branch, old, new) in [
        ("bolt/x", zero, base.clone()),
        ("main", base.clone(), to.as_str().to_string()),
    ] {
        let first = basic("fake-token");
        let fake = {
            let (first, base) = (first.clone(), base.clone());
            remote(move |a, r| match is_push(r) && a == first {
                true => not_found(),
                false => takes(r, &[("refs/heads/main", &base)]),
            })
        };
        let url = format!("{}/o/r.git", fake.url());
        let identity = Rc::new(FakeIdentity::new(&url, "fake-token"));
        let p = as_identity(&repo, &identity);
        let moved = match branch {
            "main" => p.move_to("main", &rev(base.clone()), &to),
            _ => p.make(branch, &rev(base.clone())),
        };
        assert_eq!(moved.unwrap(), Moved::Moved, "{branch}");
        let command = vec![(old, new, format!("refs/heads/{branch}"))];
        assert_eq!(
            pushes(&fake),
            [
                (first, command.clone()),
                (basic(&identity.token(1)), command)
            ],
            "{branch}: the refused push, then the same on a fresh token"
        );
        assert_eq!(identity.renewed.get(), 1);
    }
}

/// 134, 46, rule core.5: a branch moved on the remote between a push
/// refused on its token and the second is the second's lease refused: the
/// move is elsewhere, and there is no third push.
#[test]
fn a_second_push_carries_the_first_ones_lease() {
    let repo = Repo::new("leased-stale");
    let _bare = repo.bare_origin();
    let base = repo.rev("main");
    let head = rev(repo.branch("unit/u", "main", &[("a", Some("1"))]));
    let moved = repo.branch("other", "main", &[("b", Some("2"))]);
    let local = PushedRefs::open(repo.dir(), "origin").unwrap();
    let to = landing(&local, &rev(base.clone()), &head);
    let first = basic("fake-token");
    let fake = {
        let (first, base, moved) = (first.clone(), base.clone(), moved.clone());
        remote(move |a, r| match (a == first, is_push(r)) {
            (true, true) => not_found(),
            (true, false) => takes(r, &[("refs/heads/main", &base)]),
            (false, _) => takes(r, &[("refs/heads/main", &moved)]),
        })
    };
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, "fake-token"));
    let p = as_identity(&repo, &identity);
    assert_eq!(
        p.move_to("main", &rev(base.clone()), &to).unwrap(),
        Moved::Elsewhere(Some(rev(moved)))
    );
    assert_eq!(
        pushes(&fake),
        [(
            first.clone(),
            vec![(base, to.as_str().into(), "refs/heads/main".into())]
        )],
        "the second push went no further than the lease"
    );
    assert_eq!(
        listed_for_push(&fake),
        [first, basic(&identity.token(1))],
        "a listing for each push, and no third"
    );
    assert_eq!(identity.renewed.get(), 1);
}

/// 134, 24, 45: a push the remote answered at the ref is the move refused
/// and made once, whatever the remote said besides; so is one whose lease
/// git found stale.
#[test]
fn a_push_refused_at_the_ref_is_made_once() {
    let repo = Repo::new("refused-at-the-ref");
    let _bare = repo.bare_origin();
    let base = repo.rev("main");
    let head = rev(repo.branch("unit/u", "main", &[("a", Some("1"))]));
    let moved = repo.branch("other", "main", &[("b", Some("2"))]);
    let local = PushedRefs::open(repo.dir(), "origin").unwrap();
    let to = landing(&local, &rev(base.clone()), &head);

    let fake = {
        let base = base.clone();
        remote(move |_, r| match is_push(r) {
            true => report(
                &["ng refs/heads/main protected branch hook declined".into()],
                Some("error: 403 Forbidden: refs/heads/main is protected"),
            ),
            false => takes(r, &[("refs/heads/main", &base)]),
        })
    };
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, "fake-token"));
    let p = as_identity(&repo, &identity);
    let err = format!(
        "{:#}",
        p.move_to("main", &rev(base.clone()), &to).unwrap_err()
    );
    assert!(err.contains("protected branch hook declined"), "{err}");
    assert!(err.contains("error: 403 Forbidden"), "{err}");
    assert_eq!(pushes(&fake).len(), 1, "one push: {err}");
    assert_eq!(identity.renewed.get(), 0);

    let fake = {
        let moved = moved.clone();
        remote(move |_, r| takes(r, &[("refs/heads/main", &moved)]))
    };
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, "fake-token"));
    let p = as_identity(&repo, &identity);
    assert_eq!(
        p.move_to("main", &rev(base), &to).unwrap(),
        Moved::Elsewhere(Some(rev(moved)))
    );
    assert_eq!(listed_for_push(&fake), [basic("fake-token")], "one push");
    assert!(pushes(&fake).is_empty(), "git found the lease stale");
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

/// C.5: a session's branch pushed through a remote that takes the
/// identity's token is one push, naming the branch and the revision.
#[test]
fn a_push_the_remote_takes_is_one_push() {
    let repo = Repo::new("push-taken");
    let _bare = repo.bare_origin();
    let base = repo.rev("main");
    let head = repo.branch("unit/u", "main", &[("a", Some("1"))]);
    let fake = {
        let base = base.clone();
        remote(move |_, r| takes(r, &[("refs/heads/main", &base)]))
    };
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, "fake-token"));
    let p = as_identity(&repo, &identity);
    p.push_branch(&rev(head.clone()), "unit/u").unwrap();
    let zero = "0".repeat(40);
    assert_eq!(
        pushes(&fake),
        [(
            basic("fake-token"),
            vec![(zero, head, "refs/heads/unit/u".into())]
        )]
    );
    assert_eq!(identity.renewed.get(), 0);
}

/// 134, 122, 104: a branch made whose push git read no answer to, the
/// remote failing on it, is not pushed again; what the remote holds then
/// decides: made when it holds the branch at the revision pushed, failed
/// naming its absence otherwise.
#[test]
fn a_branch_made_with_no_answer_is_what_the_remote_holds() {
    let repo = Repo::new("made-unanswered");
    let _bare = repo.bare_origin();
    let base = repo.rev("main");
    let at = repo.branch("unit/u", "main", &[("a", Some("1"))]);
    for made in [true, false] {
        let fake = {
            let (base, at) = (base.clone(), at.clone());
            remote(move |_, r| {
                if lists_for_push(r) {
                    listing("git-receive-pack", &[("refs/heads/main", &base)])
                } else if is_push(r) {
                    (500, vec![], "the server failed\n".into())
                } else if made {
                    let refs = [
                        ("refs/heads/bolt/x", at.as_str()),
                        ("refs/heads/main", &base),
                    ];
                    listing("git-upload-pack", &refs)
                } else {
                    listing("git-upload-pack", &[("refs/heads/main", &base)])
                }
            })
        };
        let url = format!("{}/o/r.git", fake.url());
        let identity = Rc::new(FakeIdentity::new(&url, "fake-token"));
        let p = as_identity(&repo, &identity);
        let moved = p.make("bolt/x", &rev(at.clone()));
        if made {
            assert_eq!(moved.unwrap(), Moved::Moved);
        } else {
            let err = format!("{:#}", moved.unwrap_err());
            assert!(err.contains("pushing bolt/x to origin failed"), "{err}");
            assert!(
                err.ends_with("the push went unanswered, and origin has no bolt/x"),
                "{err}"
            );
        }
        assert_eq!(pushes(&fake).len(), 1, "made: {made}, one push");
        assert_eq!(identity.renewed.get(), 0);
    }
}

/// 134, 122, 105: a branch removed whose push's answer was lost, the
/// connection cut, and which the remote no longer holds is removed: its
/// local branch and remote-tracking ref go as an answered removal's do.
#[test]
fn a_branch_removed_with_no_answer_is_what_the_remote_holds() {
    let repo = Repo::new("removed-unanswered");
    let _bare = repo.bare_origin();
    let base = repo.rev("main");
    let at = repo.branch("bolt/x", "main", &[("a", Some("1"))]);
    repo.git(&["update-ref", "refs/remotes/origin/bolt/x", &at]);
    let fake = {
        let (base, at) = (base.clone(), at.clone());
        remote(move |_, r| {
            if lists_for_push(r) {
                let refs = [
                    ("refs/heads/bolt/x", at.as_str()),
                    ("refs/heads/main", &base),
                ];
                listing("git-receive-pack", &refs)
            } else if is_push(r) {
                (support::http::CUT, vec![], String::new())
            } else {
                listing("git-upload-pack", &[("refs/heads/main", &base)])
            }
        })
    };
    let url = format!("{}/o/r.git", fake.url());
    let identity = Rc::new(FakeIdentity::new(&url, "fake-token"));
    let p = as_identity(&repo, &identity);
    assert_eq!(p.remove("bolt/x", &rev(at.clone())).unwrap(), Moved::Moved);
    let zero = "0".repeat(40);
    assert_eq!(
        pushes(&fake),
        [(
            basic("fake-token"),
            vec![(at, zero, "refs/heads/bolt/x".into())]
        )],
        "one push"
    );
    for gone in ["refs/heads/bolt/x", "refs/remotes/origin/bolt/x"] {
        let out = repo.git_raw(&["rev-parse", "--verify", "-q", gone]);
        assert!(!out.status.success(), "{gone} is left");
    }
}
