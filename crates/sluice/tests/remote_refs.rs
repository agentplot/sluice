//! The records seam's `remote refs` profile against a bare repository as
//! the shared remote, from two hosts' clones of it (C.6; 62, 63, 80, 109;
//! ADR 0031). The check-tools tier: run with `--features tools`.

#![cfg(feature = "tools")]

mod support;

use std::collections::BTreeMap;
use std::rc::Rc;

use sluice::profiles::pushed_refs::PushedRefs;
use sluice::profiles::remote_refs::{RemoteRefs, SignerKey, Signing};
use sluice::signing::ssh;
use sluice_core::records::{parse_lines, to_line, ResultKind, ResultLine, RESULT_SCHEMA};
use sluice_seams::{Changes, Enforced, Outcome, Rev, State, TaskLog, Written, KEPT_LOGS};
use ssh_key::{Algorithm, PrivateKey};
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
    let pushed = Rc::new(PushedRefs::open(repo.dir(), "origin").unwrap());
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
/// ones, never read with them (finished-records-1 clause 104).
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
    }
}

fn lines(ls: &[ResultLine]) -> Vec<u8> {
    ls.iter().flat_map(to_line).collect()
}

/// 78, 109: a host signs its runs and adds them to its signer's ref;
/// another host reads them verified, by signer, and a record no listed key
/// signed counts as nothing. A replay stays on the host that saw it.
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
    let read: Vec<ResultLine> = parse_lines(&hb.results().unwrap()).unwrap();
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
    // The replay is the recording host's alone (core.9).
    let mine: Vec<ResultLine> = parse_lines(&ha.results().unwrap()).unwrap();
    assert!(mine.iter().any(|l| l.kind == ResultKind::Replayed));
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
