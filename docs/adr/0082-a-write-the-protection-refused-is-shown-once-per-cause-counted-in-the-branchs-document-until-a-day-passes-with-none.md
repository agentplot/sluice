---
number: 82
title: A write the protection refused is shown once per cause, counted in the branch's document, until a day passes with none
status: accepted
date: 2026-11-05
decision-makers:
- Chuck Swanberg
---

# A write the protection refused is shown once per cause, counted in the branch's document, until a day passes with none

Status: accepted by the user on 2026-11-05, in the words "Approve: push the amendment and start its code", given on the statement's amendment of that day, now on boomdrivedev/blueprints main at 539f745 (124, 148, C.4, S73), after the user chose, reviewing swancloud's fix for its views timer, "that sluice shows refused writes itself and that swancloud's doctor reads them from sluice, not from sluice's log file", as the brief of the change `sluice-shows-the-writes-its-protection-refused` relays it. The choices within it below are the construct stage's own for that change: what a cause is, where its count is kept, how long it is shown, the bound on causes, the end's line, and the words and fields. Each is cheap to change until a release carries it, and the user sees them at that change's review.

Deciders: the user, choosing and approving as above on 2026-11-05; the construct stage of `sluice-shows-the-writes-its-protection-refused`, for the choices within it.
Sources: `boomdrivedev/blueprints:design/sluice/requirements.md` clauses 26, 124, 131, 132, 133, C.4, and clause 148 and S73, with 124's and C.4's amendments, as boomdrivedev/blueprints main words them at 539f745; ADRs 0053, 0059, 0061, 0078; rules core.11, core.14, core.28, core.41; `crates/sluice/src/protect.rs` `hook` and `crates/sluice-core/src/ops.rs` `record_protected` as main held them at 3f5aed5f; swancloud's `packages/swancloud-doctor/sluice-refused.sh` on its branch `fix/the-views-timer-moves-anchors-only` (7011324e); Boomdrive's ADR 0019; flume's `refused_acts` (`crates/flume-core/src/answer.rs`).

## Context and problem statement

sluice's branch protection refuses any other hand's write to a branch it guards, and to its copy of a branch on a remote (124). Each refusal tells the command that ran git, and writes one `protected` line in the branch's log with the ref, both revisions and the command. Nothing of sluice's shows these lines: no overview row, no status line, no field in `--json`. On deneb, by 2026-11-05, swancloud's views timer, which ran `git -C <repository> update-ref refs/heads/main <new> <old>` every few minutes to level the host's copy itself, had been refused 5920 times in swancloud's records, 2787 in strategy's and 443 in weir's. crew's records held 6467 lines, most from git's own `pack-refs` before the protection let packing pass. The Boomdrive runner had been refused many more times. No one saw it for days. swancloud's fix reads sluice's `log.jsonl` with `grep`, drops the 40-digit revisions from `by`, and keeps the last line within fifteen minutes.

A line per refusal also fills the log. A timer refused every few minutes closes a thousand-line part (132) in a few days. The live records carry only the current part and the newest closed one, so a count read from the lines would lose what the closed parts hold.

What does sluice show of the writes its protection refused, where is the count kept so that a part's close leaves it whole, how long is a refusal shown, and what does a host's doctor read?

## Considered options

Where the count is kept:

1. **In the branch's document, one entry per cause and host**, beside `copies` (124), which each host already writes for itself. The hook already writes the branch's records under its lock when it refuses. The overview and status read the document with the rest of the live records (131).
2. **In the host's own state, as 133's refusal is kept**, and carried into the log by the first write after.
3. **Counted from the log's `protected` lines** at each read.

How long a cause is shown:

1. **Until a day passes with no write of it refused**, a constant of sluice's, as the log part's thousand lines is (132).
2. **An hour.**
3. **A week.**
4. **A setting of the host's or the policy's.**

## Decision outcome

The count is kept in the branch's document (option 1), and a cause is shown until a day passes with none of its writes refused (option 1).

**A cause is the command, on its host.** A cause is the branch's ref, the host whose ref was tried, and the command that tried it, with each full object name in it (40 or 64 hexadecimal digits, a whole word) read as `<revision>`. A timer that runs `update-ref refs/heads/main <new> <old>` names new revisions every time, and is one caller all the same. swancloud's doctor already drops the revisions from `by` to name its caller. Its first refusal is logged as today, with the command as it ran and both revisions, and now the host. Each later refusal of a standing cause adds no line. It counts one more in the document and moves the cause's last time and last revisions. This is 133's shape for a refused read (rule core.28): one line opens a cause and one closes it, and a host's causes are its own.

**The count is in the document, so a part's close cannot touch it.** The document is in the live records, and every read the overview and a status make takes the live records (131; rule core.41). The count never comes from a log line, so closing the log's parts leaves it whole, and nothing reads a closed part to show it. Option 3 fails here: it would need every part since the cause opened, and 131 forbids the overview and status that read. Option 2 keeps the count where only one host's status sees it. The overview, which another host's reader may serve, would see nothing, and the hook would keep a state apart from the branch's, which it already writes.

**A day.** A cause stands while its writes keep being refused, and ends a day after its last. A caller run every few minutes, or every hour, stays one cause for as long as it runs. A caller that is fixed drops out of the overview and status the next day, and the operator sees its last refusal grow old from the moment they fix it. An hour would end a caller run less often than hourly and open it again at each run, two lines a run, more than today's one. A week would show a fixed caller for a week. A setting would be a word a host must choose with nothing to choose it by. A caller run once a day may end and open again each day. That is two lines a day, and it is shown all the while.

**Its end is logged by the next write.** When a cause has stood a day without a refusal, the first write of the branch's records after takes it out of the document. It logs `protected-ended` with the host, the ref, the command, the count, the first and the last, at the time it ended: its last refusal and a day. This is the same way 133's kept lines ride the next write (`Sluice::carry`). Until then the cause stays in the document unshown, since every reader judges its day by its last refusal. A branch nothing writes keeps an ended cause in its document that no one is shown, and its log has the end only once something writes it.

**At most twenty causes per host.** A command that names something new each time it runs, such as a commit's message, is a new cause each time. Twenty standing causes per host per branch keeps the document small, since every round reads it. A twenty-first ends at once the cause whose last refusal is oldest, its end logged. The log keeps the opening line of every cause, so nothing is lost but the count of the cause put out early, and its end line holds that.

**The doctor reads sluice.** A tool that watches a host, swancloud's doctor among them, reads `sluice overview --json`, each branch's `refused_writes`, or `sluice status <branch> --json`'s, never `log.jsonl` and never `branch.json`, whose form is sluice's own. The fields are those of the cause: `host`, `ref`, `command`, `times`, `first`, `last`, `from` and `to`. `to` is `null` for a deletion, and `from` and `to` are the last refused write's revisions. A doctor judges its own window, such as fifteen minutes, on `last`. sluice's day bounds only what it shows.

### Consequences

- `branch.json` gains `refused_writes`, left out when empty. A `protected` line gains `host`. The log gains `protected-ended`. An older sluice reads the document as before and ignores the new field. It fails to read a log that holds a `protected-ended` line, as it fails on any event it does not know. Every host's sluice is the release that writes it, the hook being re-installed by the next command in each repository (124).
- The tests that refuse one command several times and count its lines change. Those that refuse different commands, one line each, still hold.
- On a branch whose records are on its remote, the hook's write is a push, as today. A refusal the hook cannot record within its second, the branch's lock held, is told to its caller and counted nowhere, as today.
- Flume's status lists each refused act of an agent's once per refusal, with no time, for as long as its start lives (`refused_acts`, flume 108). A start's life bounds it, so it needs no day, and it is told to the orchestrator as the one other tool that shows its own refusals. The Boomdrive runner no longer writes a branch sluice guards (its ADR 0019) and shows no refusals of its own.
- Rule core.47 is drawn from this record.
