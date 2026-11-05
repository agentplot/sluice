//! A write the protection refused: logged once per cause and host, counted
//! in the branch's state while it stands, shown by status and the overview
//! until a day passes with none, and its end logged by the next write that
//! changes something (124, 148; rule core.47), over the fakes, each refusal
//! recorded as the hook records it.

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use sluice_conformance::{policy, Harness, Rig, BRANCH};
use sluice_core::records::{Event, LogLine, RefusedWrite, State};
use sluice_core::{Clock, Peer};
use sluice_seams::{Changes, Rev, Written};

/// The name the harness's sluice records its own refusals under.
const HOST: &str = "this-host";

/// A full revision of 40 digits, the `n`th.
fn full(n: u64) -> Rev {
    Rev::new(format!("{n:040x}"))
}

/// The timer's command, its `n`th run naming revisions of its own.
fn timer(n: u64) -> String {
    format!(
        "git -C /home/me/app update-ref refs/heads/{BRANCH} {} {}",
        full(2 * n),
        full(2 * n + 1)
    )
}

/// The timer's cause's command.
fn timers_command() -> String {
    format!("git -C /home/me/app update-ref refs/heads/{BRANCH} <revision> <revision>")
}

/// `by`'s write to the branch, from `from` to `to`, refused on `host` and
/// recorded as the hook records it: under the branch's lock, by that
/// host's own sluice.
fn refuse_on(h: &Harness, host: &str, by: &str, from: &Rev, to: Option<&Rev>) {
    let s = h.sluice().as_peer(Peer {
        name: host.into(),
        ..Peer::default()
    });
    let state = s.seams().state;
    let _lock = state
        .lock(BRANCH)
        .expect("the lock is asked")
        .expect("the lock is free");
    s.record_protected(BRANCH, from, to, by)
        .expect("the refusal is recorded");
}

/// The timer's `n`th run refused on the harness's own host.
fn refuse(h: &Harness, n: u64) {
    refuse_on(h, HOST, &timer(n), &full(2 * n + 1), Some(&full(2 * n)));
}

/// The log's `protected` and `protected-ended` lines.
fn protected_lines(h: &Harness) -> Vec<LogLine> {
    h.log()
        .into_iter()
        .filter(|l| {
            matches!(
                l.event,
                Event::Protected { .. } | Event::ProtectedEnded { .. }
            )
        })
        .collect()
}

/// Every cause the branch's state holds, standing or not.
fn causes(h: &Harness) -> Vec<RefusedWrite> {
    h.doc().refused_writes
}

/// The causes status gives.
fn status(h: &Harness) -> Vec<RefusedWrite> {
    h.sluice()
        .status(BRANCH)
        .expect("status reads")
        .refused_writes
}

/// The causes the overview gives the branch.
fn overview(h: &Harness) -> Vec<RefusedWrite> {
    h.sluice()
        .overview(None)
        .expect("the overview reads")
        .into_iter()
        .find(|b| b.branch == BRANCH)
        .expect("the branch has records")
        .refused_writes
}

/// What the world's clock says now; it moves a second on each ask.
fn clock(h: &Harness) -> DateTime<Utc> {
    h.f.clock.now()
}

fn time(at: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(at)
        .expect("a time sluice wrote")
        .with_timezone(&Utc)
}

fn written(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// The log's lines moved aside, as a part's close moves them (132): the
/// live records keep no line of the log.
fn close_log(h: &Harness) {
    let s = h.sluice();
    let state = s.seams().state;
    let _lock = state
        .lock(BRANCH)
        .expect("the lock is asked")
        .expect("the lock is free");
    let mut files = h.w().files(BRANCH);
    let log = files.remove("log.jsonl").expect("the branch has a log");
    let mut changes = Changes::default();
    changes.put.insert("log/1.jsonl".into(), log);
    changes.remove.insert("log.jsonl".into());
    let written = state
        .write(BRANCH, h.version(), changes)
        .expect("the log is moved");
    assert!(matches!(written, Written::Written(_)));
    assert!(
        h.log().is_empty(),
        "the live records hold no line of the log"
    );
}

/// The bolt policy, its branches kept on `origin`.
fn on_origin() -> Harness {
    let h = Harness::new(&policy(false, "fail", 4).replace(
        "trust = [\"open\", \"sandbox\", \"container\"]\n",
        "trust = [\"open\", \"sandbox\", \"container\"]\nremote = \"origin\"\n",
    ));
    h.w().on_remote(BRANCH);
    h
}

/// satisfies: 148; rule core.47: a timer's command with new full
/// revisions each run is one cause, its revisions read as `<revision>`.
#[test]
fn the_same_command_with_other_revisions_is_one_cause() {
    let h = Harness::bolt();
    refuse(&h, 1);
    h.w().advance(Duration::minutes(5));
    refuse(&h, 2);
    let causes = causes(&h);
    assert_eq!(causes.len(), 1, "{causes:#?}");
    assert_eq!(causes[0].command, timers_command());
    assert_eq!(
        causes[0].key(),
        (HOST, "refs/heads/bolt/x", timers_command().as_str())
    );
    assert_eq!(causes[0].times, 2);
    let lines = protected_lines(&h);
    assert_eq!(lines.len(), 1);
    assert_eq!(
        lines[0].event,
        Event::Protected {
            reference: format!("refs/heads/{BRANCH}"),
            from: full(3),
            to: Some(full(2)),
            by: timer(1),
            host: Some(HOST.into()),
        },
        "the first refusal is logged with the command as git ran it"
    );
}

/// satisfies: 148; rule core.47: the same command refused on two hosts of
/// a branch whose records are on its remote is a cause for each.
#[test]
fn two_hosts_are_two_causes() {
    let h = on_origin();
    refuse_on(&h, "build-1", &timer(1), &full(3), Some(&full(2)));
    refuse_on(&h, "build-2", &timer(1), &full(3), Some(&full(2)));
    refuse_on(&h, "build-1", &timer(2), &full(5), Some(&full(4)));
    let hosts: Vec<(String, u64)> = causes(&h).into_iter().map(|c| (c.host, c.times)).collect();
    assert_eq!(hosts, [("build-1".into(), 2), ("build-2".into(), 1)]);
    let logged: Vec<Option<String>> = protected_lines(&h)
        .into_iter()
        .map(|l| match l.event {
            Event::Protected { host, .. } => host,
            e => panic!("no end is due: {e:?}"),
        })
        .collect();
    assert_eq!(
        logged,
        [Some("build-1".to_string()), Some("build-2".to_string())]
    );
}

/// satisfies: 148; rule core.47: a cause refused three times is one line,
/// its count, its first and last, and its last write's revisions.
#[test]
fn one_line_for_a_cause_refused_three_times() {
    let h = Harness::bolt();
    refuse(&h, 1);
    h.w().advance(Duration::minutes(20));
    refuse(&h, 2);
    h.w().advance(Duration::minutes(20));
    let before = clock(&h);
    refuse(&h, 3);
    let after = clock(&h);
    let lines = protected_lines(&h);
    assert_eq!(lines.len(), 1, "no line from the second refusal on");
    let c = &causes(&h)[0];
    assert_eq!(c.times, 3);
    assert_eq!(c.first, lines[0].at);
    let last = time(&c.last);
    assert!(before < last && last < after, "{c:?}");
    assert_eq!((&c.from, c.to.as_ref()), (&full(7), Some(&full(6))));
    assert_eq!(status(&h), std::slice::from_ref(c));
    assert_eq!(overview(&h), std::slice::from_ref(c));
}

/// satisfies: 148; rule core.47: a cause refused again 23 hours after its
/// last still stands, counted with no line.
#[test]
fn a_cause_refused_within_its_day_still_stands() {
    let h = Harness::bolt();
    refuse(&h, 1);
    h.w().advance(Duration::hours(23));
    refuse(&h, 2);
    assert_eq!(protected_lines(&h).len(), 1);
    assert_eq!(causes(&h)[0].times, 2);
    assert_eq!(status(&h).len(), 1);
}

/// satisfies: 148; rule core.47: a caller that stops is shown no more a
/// day after its last refusal; a write that changes nothing logs no end,
/// and the next write that changes something logs it at its last and a
/// day, with its count.
#[test]
fn a_fixed_caller_ends_a_day_after_its_last() {
    let h = Harness::bolt();
    h.step();
    refuse(&h, 1);
    h.w().advance(Duration::hours(1));
    refuse(&h, 2);
    let cause = causes(&h)[0].clone();
    h.w().advance(Duration::hours(23));
    assert_eq!(status(&h).len(), 1, "it stands until its day is out");
    h.w().advance(Duration::hours(1));
    assert!(status(&h).is_empty());
    assert!(overview(&h).is_empty());
    let version = h.version();
    h.step();
    assert_eq!(
        h.version(),
        version,
        "a round with nothing to do writes nothing"
    );
    assert_eq!(protected_lines(&h).len(), 1, "and logs no end");
    assert_eq!(
        causes(&h),
        std::slice::from_ref(&cause),
        "the ended cause is kept, unshown"
    );
    h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
    h.submit("unit/a");
    let lines = protected_lines(&h);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1].at, written(time(&cause.last) + Duration::days(1)));
    assert_eq!(
        lines[1].event,
        Event::ProtectedEnded {
            host: HOST.into(),
            reference: format!("refs/heads/{BRANCH}"),
            command: timers_command(),
            times: 2,
            first: cause.first,
            last: cause.last,
        }
    );
    assert!(causes(&h).is_empty());
}

/// satisfies: 148; rule core.47: a command refused after its cause ended
/// opens it anew, logged as its first was.
#[test]
fn a_cause_refused_after_its_end_opens_anew() {
    let h = Harness::bolt();
    refuse(&h, 1);
    let ended = causes(&h)[0].clone();
    h.w().advance(Duration::hours(25));
    refuse(&h, 2);
    let events: Vec<Event> = protected_lines(&h).into_iter().map(|l| l.event).collect();
    assert!(
        matches!(
            &events[..],
            [
                Event::Protected { .. },
                Event::ProtectedEnded { times: 1, .. },
                Event::Protected { by, .. },
            ] if *by == timer(2)
        ),
        "{events:#?}"
    );
    let c = &causes(&h)[0];
    assert_eq!(c.times, 1);
    assert_ne!(c.first, ended.first);
    assert_eq!(c.first, c.last);
}

/// satisfies: 148; rule core.47: a host's twenty-first cause on a branch
/// ends at once its cause refused longest ago.
#[test]
fn a_twenty_first_cause_ends_the_oldest() {
    let h = Harness::bolt();
    let commit = |i: u64| format!("git commit -m 'views: refresh {i}'");
    for i in 0..21 {
        refuse_on(&h, HOST, &commit(i), &full(1), Some(&full(i + 2)));
        h.w().advance(Duration::minutes(1));
    }
    let standing = causes(&h);
    assert_eq!(standing.len(), 20);
    assert!(standing.iter().all(|c| c.command != commit(0)));
    let lines = protected_lines(&h);
    let opened = lines
        .iter()
        .filter(|l| matches!(l.event, Event::Protected { .. }))
        .count();
    let ended: Vec<&LogLine> = lines
        .iter()
        .filter(|l| matches!(l.event, Event::ProtectedEnded { .. }))
        .collect();
    assert_eq!(opened, 21);
    assert_eq!(ended.len(), 1);
    assert!(
        matches!(&ended[0].event, Event::ProtectedEnded { command, times: 1, .. } if *command == commit(0))
    );
    assert_eq!(
        ended[0].at,
        lines.last().unwrap().at,
        "it ends when the twenty-first is refused"
    );
    assert_eq!(status(&h).len(), 20);
}

/// satisfies: 148, 131, 132; rule core.47: the count is the state's, so a
/// part's close that takes the log's lines away changes nothing status or
/// the overview give, and the count goes on with no line.
#[test]
fn the_count_is_read_with_the_logs_lines_taken_away() {
    let h = Harness::bolt();
    for n in 1..=3 {
        refuse(&h, n);
        h.w().advance(Duration::minutes(5));
    }
    let before = (causes(&h), status(&h), overview(&h));
    close_log(&h);
    assert_eq!((causes(&h), status(&h), overview(&h)), before);
    assert_eq!(status(&h)[0].times, 3);
    refuse(&h, 4);
    assert_eq!(status(&h)[0].times, 4);
    assert!(
        h.log().is_empty(),
        "a standing cause's refusal writes no line"
    );
}

/// satisfies: 148; rules core.47, core.14: a round beside a standing cause
/// forms, tests and lands as it would with none, and the cause is left
/// as it stood.
#[test]
fn a_round_beside_a_standing_cause_is_unchanged() {
    let land = |refused: bool| {
        let h = Harness::bolt();
        if refused {
            refuse(&h, 1);
            refuse(&h, 2);
            assert!(
                !h.sluice().in_play(BRANCH).unwrap(),
                "a cause puts nothing in play"
            );
        }
        h.work("unit/a", "agent-a", &[("a/lib", Some("a2"))]);
        let id = h.submit("unit/a");
        h.settle();
        assert_eq!(h.sub(id).state, State::Landed);
        let kinds: Vec<String> = h
            .log()
            .into_iter()
            .filter(|l| !matches!(l.event, Event::Protected { .. }))
            .map(|l| serde_json::to_value(&l).unwrap()["event"].to_string())
            .collect();
        (kinds, h.tested(), h.started().len(), h.moves(), h)
    };
    let (kinds, tested, started, moves, h) = land(true);
    let none = land(false);
    assert_eq!(
        (kinds, tested, started, moves),
        (none.0, none.1, none.2, none.3)
    );
    let c = &causes(&h)[0];
    assert_eq!(
        (c.times, c.command.as_str()),
        (2, timers_command().as_str())
    );
    assert_eq!(status(&h).len(), 1);
}
