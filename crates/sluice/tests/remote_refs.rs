//! The records seam's `remote refs` profile against a bare repository as
//! the shared remote, from two hosts' clones of it (C.6; 62, 63, 80, 109;
//! ADR 0037). The check-tools tier: run with `--features tools`.

#![cfg(feature = "tools")]

mod support;

use std::collections::BTreeMap;
use std::rc::Rc;

use sluice::profiles::local_files::LocalFiles;
use sluice::profiles::pushed_refs::PushedRefs;
use sluice::profiles::remote_refs::{RemoteRefs, SignerKey, Signing};
use sluice::retry;
use sluice::signing::ssh;
use sluice_core::records::{parse_lines, to_line, ResultKind, ResultLine, RESULT_SCHEMA};
use sluice_seams::{
    read_refused, Changes, Enforced, Outcome, Rev, State, TaskLog, Written, KEPT_LOGS,
};
use ssh_key::{Algorithm, PrivateKey};
use support::http::{commands, is_push, listing, lists_for_push, Fake, CUT};
use support::{Repo, Scratch};

fn key() -> PrivateKey {
    PrivateKey::random(&mut rand_core::OsRng, Algorithm::Ed25519).unwrap()
}

fn allowed(principal: &str, k: &PrivateKey) -> SignerKey {
    SignerKey::Ssh(
        ssh::allowed(&format!(
            "{principal} {}",
            k.public_key().to_openssh().unwrap()
        ))
        .unwrap(),
    )
}

/// A host's records of `origin`, signing as `signer` with `k`, verifying
/// the signers in `signers`.
fn host(
    repo: &Repo,
    signers: BTreeMap<String, SignerKey>,
    signing: Option<(&str, PrivateKey)>,
) -> RemoteRefs {
    let pushed = Rc::new(PushedRefs::open_as(repo.dir(), "origin", None, recorded()).unwrap());
    RemoteRefs::new(
        &repo.dir().join(".git"),
        pushed,
        signers,
        signing.map(|(s, key)| Signing {
            signer: s.into(),
            key,
        }),
    )
}

/// A command's retry policy whose pauses are skipped, as if taken.
fn recorded() -> Rc<retry::Policy> {
    Rc::new(retry::Policy::command().recording())
}

fn put(files: &[(&str, &str)]) -> Changes {
    Changes {
        put: files
            .iter()
            .map(|(p, b)| (p.to_string(), b.as_bytes().to_vec()))
            .collect(),
        ..Changes::default()
    }
}

fn two_hosts(name: &str) -> (Repo, Scratch, Repo) {
    let a = Repo::new(name);
    let bare = a.bare_origin();
    let b = Repo::clone_of(bare.path(), &format!("{name}-b"));
    (a, bare, b)
}

/// 62, 63, 80: the state is one ref on the remote, read from any host,
/// and of two writes from one version exactly one is written.
#[test]
fn two_hosts_write_one_version_and_one_is_refused() {
    let (a, _bare, b) = two_hosts("rr-write");
    let (ha, hb) = (
        host(&a, BTreeMap::new(), None),
        host(&b, BTreeMap::new(), None),
    );
    let empty = ha.read("main").unwrap();
    assert_eq!((empty.version, empty.files.len()), (0, 0));
    {
        let _g = ha.lock("main").unwrap().unwrap();
        assert_eq!(
            ha.write("main", 0, put(&[("branch.json", "{}\n")]))
                .unwrap(),
            Written::Written(1)
        );
    }
    let seen = hb.read("main").unwrap();
    assert_eq!(seen.version, 1);
    assert_eq!(seen.files["branch.json"], b"{}\n");
    assert!(!seen.files.contains_key("version"));

    let at_a = ha.read("main").unwrap().version;
    let at_b = hb.read("main").unwrap().version;
    let wa = ha
        .write("main", at_a, put(&[("branch.json", "{\"a\":1}\n")]))
        .unwrap();
    let wb = hb
        .write("main", at_b, put(&[("branch.json", "{\"b\":1}\n")]))
        .unwrap();
    assert_eq!(wa, Written::Written(2));
    assert_eq!(wb, Written::Refused);
    assert_eq!(
        hb.read("main").unwrap().files["branch.json"],
        b"{\"a\":1}\n"
    );
    assert_eq!(ha.branches().unwrap(), ["main"]);
}

/// A log is appended to, and a finished record is kept beside the live
/// ones, never read with them (130).
#[test]
fn appends_and_finished_records_keep_their_places() {
    let (a, _bare, _b) = two_hosts("rr-append");
    let h = host(&a, BTreeMap::new(), None);
    let mut c = put(&[(".finished/7.json", "{\"id\":7}\n")]);
    c.append.insert("log.jsonl".into(), b"one\n".to_vec());
    assert_eq!(h.write("bolt/x", 0, c).unwrap(), Written::Written(1));
    let mut c = Changes::default();
    c.append.insert("log.jsonl".into(), b"two\n".to_vec());
    assert_eq!(h.write("bolt/x", 1, c).unwrap(), Written::Written(2));
    let snap = h.read("bolt/x").unwrap();
    assert_eq!(snap.files["log.jsonl"], b"one\ntwo\n");
    assert!(!snap.files.keys().any(|k| k.starts_with('.')));
    assert_eq!(
        h.read_finished("bolt/x", "7.json").unwrap().unwrap(),
        b"{\"id\":7}\n"
    );
}

/// 51, 130: a write putting a finished record that is there replaces it,
/// read back as written by name and with the rest, on another host too,
/// and the live index is left as it was.
#[test]
fn a_finished_record_put_again_replaces_it_and_leaves_the_index() {
    let (a, _bare, b) = two_hosts("rr-finished-again");
    let (ha, hb) = (
        host(&a, BTreeMap::new(), None),
        host(&b, BTreeMap::new(), None),
    );
    let index = "{\"id\":5,\"state\":\"landed\",\"worktree\":\"unit/a\"}\n";
    let mut c = put(&[(".finished/5.json", "{\"id\":5}\n")]);
    c.append
        .insert("finished.jsonl".into(), index.as_bytes().to_vec());
    assert_eq!(ha.write("bolt/x", 0, c).unwrap(), Written::Written(1));
    let delivered = "{\"id\":5,\"delivered\":\"2026-10-28T09:00:00Z\"}\n";
    let mut c = put(&[(".finished/5.json", delivered)]);
    c.append.insert("log.jsonl".into(), b"delivered\n".to_vec());
    assert_eq!(ha.write("bolt/x", 1, c).unwrap(), Written::Written(2));
    for h in [&ha, &hb] {
        assert_eq!(
            h.read_finished("bolt/x", "5.json").unwrap().unwrap(),
            delivered.as_bytes()
        );
        let all = h.read_all_finished("bolt/x").unwrap();
        assert_eq!(all.keys().collect::<Vec<_>>(), ["5.json"]);
        assert_eq!(all["5.json"], delivered.as_bytes());
        assert_eq!(
            h.read("bolt/x").unwrap().files["finished.jsonl"],
            index.as_bytes()
        );
    }
}

/// 80, 133: a state fetch the remote refuses reads the state last fetched,
/// and says why: the remote, the ref asked for and git's words; the
/// records' other fetches are refused reads too, and the next read that
/// reaches the remote is current again.
#[test]
fn a_refused_state_fetch_is_stale_with_why() {
    let (a, bare, _b) = two_hosts("rr-refused");
    let h = host(&a, BTreeMap::new(), None);
    {
        let _g = h.lock("main").unwrap().unwrap();
        h.write("main", 0, put(&[("branch.json", "{}\n")])).unwrap();
    }
    assert_eq!(h.read("main").unwrap().version, 1);
    assert_eq!(h.stale("main"), None);
    let aside = bare.path().with_extension("aside");
    std::fs::rename(bare.path(), &aside).unwrap();
    assert_eq!(h.read("main").unwrap().version, 1, "the state last fetched");
    let stale = h.stale("main").expect("a stale read");
    assert_eq!(stale.remote, "origin");
    assert_eq!(stale.asked, "refs/sluice/state/main");
    assert!(
        stale
            .said
            .contains("does not appear to be a git repository"),
        "{stale:?}"
    );
    for e in [
        h.fetch_heads("main").unwrap_err(),
        h.fetch_candidates("main").unwrap_err(),
    ] {
        let refused = read_refused(&e).expect("a refused read");
        assert_eq!(refused.remote, "origin");
        assert!(refused.asked.starts_with("refs/sluice/"), "{refused:?}");
    }
    std::fs::rename(&aside, bare.path()).unwrap();
    assert_eq!(h.read("main").unwrap().version, 1);
    assert_eq!(h.stale("main"), None);
}

/// 133: this host's refusal of a branch's remote is kept beside what it
/// observed, replaced and removed whole, by either profile; it is never
/// a branch's records.
#[test]
fn each_profile_keeps_a_refusal_beside_the_records() {
    let (a, _bare, _b) = two_hosts("rr-kept");
    let h = host(&a, BTreeMap::new(), None);
    let local = LocalFiles::new(&a.dir().join(".git"));
    {
        let _g = local.lock("bolt/x").unwrap().unwrap();
        local
            .write("bolt/x", 0, put(&[("branch.json", "{}\n")]))
            .unwrap();
    }
    let at = [
        a.dir()
            .join(".git/sluice-remote/origin/refused/bolt/x.json"),
        a.dir().join(".git/sluice/.refused/bolt/x.json"),
    ];
    for (s, at) in [(&h as &dyn State, &at[0]), (&local, &at[1])] {
        assert_eq!(s.refusal("bolt/x").unwrap(), None);
        s.keep_refusal("bolt/x", Some(b"{\"rounds\":1}\n")).unwrap();
        assert_eq!(std::fs::read(at).unwrap(), b"{\"rounds\":1}\n");
        s.keep_refusal("bolt/x", Some(b"{\"rounds\":2}\n")).unwrap();
        assert_eq!(
            s.refusal("bolt/x").unwrap().as_deref(),
            Some(&b"{\"rounds\":2}\n"[..])
        );
        s.keep_refusal("main", Some(b"{}\n")).unwrap();
        s.keep_refusal("bolt/x", None).unwrap();
        assert_eq!(s.refusal("bolt/x").unwrap(), None);
        assert!(!at.exists());
        s.keep_refusal("bolt/x", None).unwrap();
    }
    assert_eq!(local.branches().unwrap(), ["bolt/x"]);
    assert!(!local
        .read("bolt/x")
        .unwrap()
        .files
        .keys()
        .any(|k| k.contains("refused")));
}

/// Submission ids are numbered once across every host.
#[test]
fn ids_are_numbered_once_across_hosts() {
    let (a, _bare, b) = two_hosts("rr-ids");
    let (ha, hb) = (
        host(&a, BTreeMap::new(), None),
        host(&b, BTreeMap::new(), None),
    );
    assert_eq!(ha.allocate_id().unwrap(), 1);
    assert_eq!(hb.allocate_id().unwrap(), 2);
    assert_eq!(ha.allocate_id().unwrap(), 3);
}

fn ran(task: &str, outcome: Outcome) -> ResultLine {
    ResultLine {
        schema: RESULT_SCHEMA.into(),
        at: "2026-10-19T12:00:00Z".into(),
        kind: ResultKind::Ran,
        task: task.into(),
        hash: format!("{task}-key"),
        outcome,
        host: Some("box".into()),
        executor: Some(Enforced {
            kind: "container".into(),
            confined: true,
            processors: None,
            memory: None,
            network: None,
            docker: false,
        }),
        branch: "main".into(),
        combination: Rev::new("c0ffee"),
        worktree: None,
        system: Some("linux-x64".into()),
        signer: None,
        took_ms: None,
        attempts: None,
    }
}

fn lines(ls: &[ResultLine]) -> Vec<u8> {
    ls.iter().flat_map(to_line).collect()
}

/// 78, 109: a host signs its runs and adds them to its signer's ref;
/// another host reads them verified, by signer, and a record no listed key
/// signed counts as nothing.
#[test]
fn signed_runs_are_read_verified_on_another_host() {
    let (a, _bare, b) = two_hosts("rr-signed");
    let (box_key, stranger) = (key(), key());
    let ha = host(&a, BTreeMap::new(), Some(("box", box_key.clone())));
    let mut replay = ran("c:test", Outcome::Pass);
    replay.kind = ResultKind::Replayed;
    replay.host = None;
    replay.executor = None;
    ha.record_results(&lines(&[
        ran("a:test", Outcome::Pass),
        ran("b:test", Outcome::Fail),
        replay,
    ]))
    .unwrap();

    let hb = host(
        &b,
        BTreeMap::from([("box".into(), allowed("exec@build-1", &box_key))]),
        None,
    );
    let all: Vec<ResultLine> = parse_lines(&hb.results().unwrap()).unwrap();
    // The replay is signed with its run, as a replay (108; core.9).
    assert!(all.iter().any(|l| l.kind == ResultKind::Replayed
        && l.task == "c:test"
        && l.signer.as_deref() == Some("box")));
    let read: Vec<ResultLine> = all
        .into_iter()
        .filter(|l| l.kind == ResultKind::Ran)
        .collect();
    assert_eq!(read.len(), 2);
    assert!(read.iter().all(|l| l.signer.as_deref() == Some("box")
        && l.host.as_deref() == Some("exec@build-1")
        && l.system.as_deref() == Some("linux-x64")
        && l.executor.as_ref().is_some_and(|e| e.confined)));
    let b_test = read.iter().find(|l| l.task == "b:test").unwrap();
    assert_eq!(b_test.outcome, Outcome::Fail);
    // A key the policy does not list proves nothing (S18).
    let hc = host(
        &b,
        BTreeMap::from([("box".into(), allowed("other", &stranger))]),
        None,
    );
    assert!(parse_lines::<ResultLine>(&hc.results().unwrap())
        .unwrap()
        .is_empty());
}

/// Could-not-run-1 116, rule core.11: a result set aside on one host is
/// read set aside on every host, and two set-asides at once both stay.
#[test]
fn a_set_aside_is_read_on_another_host() {
    let (a, _bare, b) = two_hosts("rr-aside");
    let ha = host(&a, BTreeMap::new(), None);
    let hb = host(&b, BTreeMap::new(), None);
    ha.set_aside(b"{\"task\":\"a:test\"}").unwrap();
    hb.set_aside(b"{\"task\":\"b:test\"}").unwrap();
    for h in [&ha, &hb] {
        let text = String::from_utf8(h.set_asides().unwrap()).unwrap();
        assert_eq!(text.lines().count(), 2, "{text}");
        assert!(text.contains("a:test") && text.contains("b:test"), "{text}");
    }
}

/// 109: two hosts of one signer each add their runs, and neither loses
/// the other's.
#[test]
fn two_hosts_of_one_signer_keep_both_adds() {
    let (a, _bare, b) = two_hosts("rr-adds");
    let (ka, kb) = (key(), key());
    host(&a, BTreeMap::new(), Some(("sessions", ka.clone())))
        .record_results(&lines(&[ran("a:test", Outcome::Pass)]))
        .unwrap();
    host(&b, BTreeMap::new(), Some(("sessions", kb.clone())))
        .record_results(&lines(&[ran("b:test", Outcome::Pass)]))
        .unwrap();
    let both = SignerKey::Ssh(
        ssh::allowed(&format!(
            "a {}\nb {}\n",
            ka.public_key().to_openssh().unwrap(),
            kb.public_key().to_openssh().unwrap()
        ))
        .unwrap(),
    );
    let reader = host(&a, BTreeMap::from([("sessions".into(), both)]), None);
    let mut tasks: Vec<String> = parse_lines::<ResultLine>(&reader.results().unwrap())
        .unwrap()
        .into_iter()
        .map(|l| l.task)
        .collect();
    tasks.sort();
    assert_eq!(tasks, ["a:test", "b:test"]);
}

/// 78: a host with no signing key records no run on the remote, and says
/// why.
#[test]
fn a_host_with_no_signing_key_records_nothing_and_says_so() {
    let (a, _bare, _b) = two_hosts("rr-unsigned");
    let err = host(&a, BTreeMap::new(), None)
        .record_results(&lines(&[ran("a:test", Outcome::Pass)]))
        .unwrap_err();
    assert!(format!("{err:#}").contains("[signing]"), "{err:#}");
}

/// 20: a failed candidate's outputs are kept on the remote too, for the
/// last `KEPT_LOGS` candidates of the branch.
#[test]
fn logs_are_kept_on_the_remote_for_the_last_candidates() {
    let (a, bare, _b) = two_hosts("rr-logs");
    let h = host(&a, BTreeMap::new(), None);
    for n in 1..=(KEPT_LOGS + 2) {
        let kept = h
            .keep_logs(
                "main",
                &format!("c{n}"),
                &[TaskLog {
                    task: "a:test".into(),
                    text: format!("run {n}\n"),
                }],
            )
            .unwrap();
        assert!(std::fs::read_to_string(&kept[0].text)
            .unwrap()
            .contains(&format!("run {n}")));
    }
    let out = support::git_command()
        .arg("--git-dir")
        .arg(bare.path())
        .args([
            "for-each-ref",
            "--format=%(refname)",
            "refs/sluice/logs/main/",
        ])
        .output()
        .unwrap();
    let refs = String::from_utf8(out.stdout).unwrap();
    assert_eq!(refs.lines().count(), KEPT_LOGS);
    assert!(!refs.contains("refs/sluice/logs/main/c1\n"));
    assert!(refs.contains(&format!("refs/sluice/logs/main/c{}\n", KEPT_LOGS + 2)));
}

/// Design D10: the first host to take a branch whose records moved to its
/// remote pushes its own queue there as the starting state; another host
/// whose queue was not first leaves its queued submissions behind, for its
/// round to submit again.
#[test]
fn the_first_hosts_queue_moves_and_the_others_is_left_behind() {
    use sluice::profiles::branches::Branches;
    use sluice::profiles::local_files::LocalFiles;
    use sluice::profiles::records::Records;

    let k = key();
    let a = Repo::new("rr-move");
    a.write(
        ".config/sluice.toml",
        &format!(
            "[signers.box]\nssh = \"box {}\"\n\n[branches.main]\nremote = \"origin\"\ntrust = [\"box\"]\n",
            k.public_key().to_openssh().unwrap()
        ),
    );
    a.git(&["add", "-A"]);
    a.git(&["commit", "-q", "-m", "signers"]);
    let bare = a.bare_origin();
    let b = Repo::clone_of(bare.path(), "rr-move-b");
    // Each host's own queue, as its sluice wrote it before the move.
    let own = |r: &Repo, id: u64| {
        let s = LocalFiles::new(&r.dir().join(".git"));
        let _g = s.lock("main").unwrap().unwrap();
        let doc = format!(
            "{{\"schema\":\"sluice/submission@1\",\"id\":{id},\"branch\":\"main\",\"state\":\"queued\"}}\n"
        );
        s.write("main", 0, put(&[(&format!("submissions/{id}.json"), &doc)]))
            .unwrap();
    };
    own(&a, 5);
    own(&b, 9);

    let records = |r: &Repo| {
        Records::new(
            &r.dir().join(".git"),
            Branches::open_as(r.dir(), None, recorded()).unwrap(),
            None,
        )
    };
    let ra = records(&a);
    drop(ra.lock("main").unwrap().unwrap());
    let moved = ra.read("main").unwrap();
    assert!(moved.files.contains_key("submissions/5.json"));
    assert!(ra.left_behind("main").unwrap().is_empty());

    let rb = records(&b);
    drop(rb.lock("main").unwrap().unwrap());
    assert!(!rb
        .read("main")
        .unwrap()
        .files
        .contains_key("submissions/9.json"));
    let left = rb.left_behind("main").unwrap();
    assert_eq!(left.len(), 1);
    assert!(String::from_utf8_lossy(&left[0]).contains("\"id\":9"));
    rb.forget_left_behind("main", &[9]).unwrap();
    assert!(rb.left_behind("main").unwrap().is_empty());
}

/// 134, 122, 77: a host's sign of life whose push was cut after it was
/// sent is pushed once, and stands when the remote's ref holds the commit
/// it pushed.
#[test]
fn a_sign_of_life_cut_after_it_was_sent_stands() {
    let repo = Repo::new("rr-beat-cut");
    let held = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
    let fake = {
        let held = held.clone();
        Fake::start(move |r| {
            if lists_for_push(r) {
                listing("git-receive-pack", &[])
            } else if is_push(r) {
                let (_, new, _) = commands(r).remove(0);
                *held.lock().unwrap() = Some(new);
                (CUT, vec![], String::new())
            } else {
                let held = held.lock().unwrap().clone();
                match &held {
                    Some(at) => listing("git-upload-pack", &[("refs/sluice/hosts/h", at)]),
                    None => listing("git-upload-pack", &[]),
                }
            }
        })
    };
    repo.git(&[
        "remote",
        "add",
        "origin",
        &format!("{}/o/r.git", fake.url()),
    ]);
    let h = host(&repo, BTreeMap::new(), None);
    h.beat("h", b"{}").unwrap();
    let pushes = fake.requests().iter().filter(|r| is_push(r)).count();
    assert_eq!(pushes, 1, "one push");
    assert!(held.lock().unwrap().is_some(), "the remote holds the beat");
}

/// 134, 62: the state's leased push refused at its lease, another host
/// having written first, is one push, and the write is refused as before.
#[test]
fn a_state_write_refused_at_the_lease_is_one_push() {
    use std::os::unix::fs::PermissionsExt;
    let (a, _bare, b) = two_hosts("rr-lease-once");
    let (ha, hb) = (
        host(&a, BTreeMap::new(), None),
        host(&b, BTreeMap::new(), None),
    );
    let at_a = ha.read("main").unwrap().version;
    let at_b = hb.read("main").unwrap().version;
    // Every push of b's runs this, once.
    let log = b.dir().join(".git/pushes.log");
    let pack = b.dir().join(".git/receive-pack");
    std::fs::write(
        &pack,
        format!(
            "#!/bin/sh\necho push >> '{}'\nexec git receive-pack \"$@\"\n",
            log.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&pack, std::fs::Permissions::from_mode(0o755)).unwrap();
    b.git(&[
        "config",
        "remote.origin.receivepack",
        pack.to_str().unwrap(),
    ]);
    let wa = ha
        .write("main", at_a, put(&[("branch.json", "{\"a\":1}\n")]))
        .unwrap();
    let wb = hb
        .write("main", at_b, put(&[("branch.json", "{\"b\":1}\n")]))
        .unwrap();
    assert_eq!(wa, Written::Written(1));
    assert_eq!(wb, Written::Refused);
    let pushes = std::fs::read_to_string(&log).unwrap_or_default();
    assert_eq!(pushes.lines().count(), 1, "one push: {pushes}");
}

/// Count each fetch `repo` makes from `origin` in a file, one line a
/// fetch: git runs this in place of `git upload-pack` on the remote.
fn count_fetches(repo: &Repo) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let log = repo.dir().join(".git/fetches.log");
    let pack = repo.dir().join(".git/upload-pack");
    std::fs::write(
        &pack,
        format!(
            "#!/bin/sh\necho fetch >> '{}'\nexec git upload-pack \"$@\"\n",
            log.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&pack, std::fs::Permissions::from_mode(0o755)).unwrap();
    repo.git(&["config", "remote.origin.uploadpack", pack.to_str().unwrap()]);
    log
}

/// `rev` is an object `repo` holds.
fn holds(repo: &Repo, rev: &str) -> bool {
    repo.git_raw(&["cat-file", "-e", &format!("{rev}^{{commit}}")])
        .status
        .success()
}

/// The revision `name` holds in `repo`, or `None` when it has none.
fn ref_at(repo: &Repo, name: &str) -> Option<String> {
    let out = repo.git_raw(&["rev-parse", "--verify", "-q", name]);
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// 64, 129: a read of the records with the branch brings, in one fetch,
/// the branch into its tracking ref, the work another host submitted and
/// the combinations formed on the branch.
#[test]
fn a_read_with_the_branch_brings_the_branch_and_the_work_in_one_fetch() {
    let (a, _bare, b) = two_hosts("rr-with-branch");
    let (ha, hb) = (
        host(&a, BTreeMap::new(), None),
        host(&b, BTreeMap::new(), None),
    );
    {
        let _g = hb.lock("main").unwrap().unwrap();
        hb.write("main", 0, put(&[("branch.json", "{}\n")]))
            .unwrap();
    }
    let moved = b.push_change("moved", "on the remote\n");
    let head = b.branch("unit/b", "main", &[("work", Some("b's\n"))]);
    hb.publish_head("main", &Rev::new(head.clone())).unwrap();
    let combination = b.branch("combined", "main", &[("both", Some("1 and 2\n"))]);
    hb.publish_candidate("main", "c1", &Rev::new(combination.clone()))
        .unwrap();
    assert!(!holds(&a, &head) && !holds(&a, &combination));

    let fetches = count_fetches(&a);
    let read = ha.read_with_branch("main").unwrap();
    assert_eq!(read.version, 1);
    assert_eq!(read.files["branch.json"], b"{}\n");
    assert_eq!(ha.stale("main"), None);
    let fetched = std::fs::read_to_string(&fetches).unwrap();
    assert_eq!(fetched.lines().count(), 1, "one fetch: {fetched}");
    assert_eq!(
        ref_at(&a, "refs/remotes/origin/main").as_deref(),
        Some(moved.as_str())
    );
    assert!(holds(&a, &head), "the head submitted from b");
    assert!(holds(&a, &combination), "the combination b formed");
}

/// 64, 129: where the remote no longer holds the branch, its records are
/// read all the same, and no revision of the branch is left to name as
/// the remote's (124).
#[test]
fn a_branch_the_remote_lacks_still_gives_the_records() {
    let (a, bare, _b) = two_hosts("rr-no-branch");
    let h = host(&a, BTreeMap::new(), None);
    {
        let _g = h.lock("main").unwrap().unwrap();
        h.write("main", 0, put(&[("branch.json", "{}\n")])).unwrap();
    }
    assert!(ref_at(&a, "refs/remotes/origin/main").is_some());
    let out = support::git_command()
        .arg("--git-dir")
        .arg(bare.path())
        .args(["update-ref", "-d", "refs/heads/main"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let read = h.read_with_branch("main").unwrap();
    assert_eq!(read.version, 1);
    assert_eq!(read.files["branch.json"], b"{}\n");
    assert_eq!(h.stale("main"), None);
    assert_eq!(ref_at(&a, "refs/remotes/origin/main"), None);
}

/// 64, 80, 129: a remote out of reach gives the records as last fetched,
/// and the read says it is stale, in the remote's words; the branch's
/// tracking ref stays as it was.
#[test]
fn a_remote_out_of_reach_gives_the_records_last_fetched() {
    let (a, bare, _b) = two_hosts("rr-with-branch-stale");
    let h = host(&a, BTreeMap::new(), None);
    {
        let _g = h.lock("main").unwrap().unwrap();
        h.write("main", 0, put(&[("branch.json", "{}\n")])).unwrap();
    }
    assert_eq!(h.read_with_branch("main").unwrap().version, 1);
    let tracked = ref_at(&a, "refs/remotes/origin/main");
    let aside = bare.path().with_extension("aside");
    std::fs::rename(bare.path(), &aside).unwrap();
    let read = h.read_with_branch("main").unwrap();
    assert_eq!(read.version, 1, "the records last fetched");
    let stale = h.stale("main").expect("a stale read");
    assert_eq!(stale.remote, "origin");
    assert_eq!(stale.asked, "refs/sluice/state/main");
    assert!(
        stale
            .said
            .contains("does not appear to be a git repository"),
        "{stale:?}"
    );
    assert_eq!(ref_at(&a, "refs/remotes/origin/main"), tracked);
    std::fs::rename(&aside, bare.path()).unwrap();
}
