## Why

Record 0093, accepted by the user on 2026-11-06 and amended on their
card of 2026-11-08, takes the intake mark away: its renewal, its lease,
hand-overs and their withdrawals all go, and whatever executes a
branch's queue takes its requests in, in one of four ways the machines
choose, nothing doing one part of that while another does the rest and
no request taken in twice. 137 says it at agentplot/blueprints
5f05fc6: a branch's requests are taken in only where its records are
on its remote, by the instance that keeps the branch there (72), and a
branch whose records each host keeps takes none in, a pull request
table on it refused for that branch alone (56, S64). Rule core.33 rules
out the mark, its renewal, its lease, a hand-over and a withdrawal for
one.

What `bolt/sluice-runs-one-host-process` does at 752db9a9, since #29
took the forge's round job out and put intake back as the host-process
landing had it:

- A branch whose records each host keeps is taken in by whichever host
  holds `refs/sluice/intake/<branch>` on its remote. The host process's
  pass pushes and renews it as `Taker::Driver`, takes it from a wait's
  holder, and takes another host's after the lease
  (`crates/sluice/src/profiles/github/pulls.rs`, `Pulls::takes`,
  `decide`, `put_mark`). A host that finds another's mark withdraws its
  own submissions of the requests as a hand-over (`hand_over`,
  `Requester::HandOver`), and `forge/intake/` in the host's state keeps
  the mark as seen and the waits' holds (`kept.rs`). Two copies of one
  repository, or two hosts each with a host process, can each hold the
  mark in turn and take one request in twice.
- `sluice status`, `sluice wait` and a submit's `--status` name the
  mark's host and holder as `taken_in_by`.
- Intake's warnings, of the branch's whole intake or of one request
  (128), are `telemetry::warn`, the host process's stderr alone. A
  `sluice step` asked of the host process prints what its round's
  `out` holds, which is only the spent allowance's line, so its caller
  never sees why a request was not taken. A failure that stands is
  said once per process, by a `thread_local` of `crate::intake`; a host
  process that restarts says it again, and nothing of it reaches the
  branch's log. Rule core.28 rules out a read of a remote reported only
  to the process that met it, and rules core.11 and core.14 a decision
  kept only in a process's memory.
- The two changes built on records 0088 and 0089's mark,
  `a-host-hands-over-only-to-the-host-its-mark-names` and
  `a-hand-overs-withdrawal-holds-no-request`, stand unarchived in
  `openspec/changes/`, with the tests they added.

## What Changes

- **No intake mark.** Nothing pushes, renews, leases, takes, holds,
  lets go or fetches `refs/sluice/intake/<branch>`, and nothing reads
  one an older sluice left on a remote, nor the `forge/intake/` it left
  in the host's state. `Taker`, `Holder`, `Mark`, the waits' holds and
  the mark's lease observation go.
- **The host that keeps the branch takes its requests in.** On a branch
  whose records are on its remote, intake runs only in the instance
  that keeps it (72, 73), any process of that host under intake's lock
  (rule core.14). Every other host asks GitHub nothing of the branch's
  open requests, withdraws nothing and follows its own requests by
  GitHub and sluice's check on their heads, as it does today (137).
- **BREAKING: a branch whose records each host keeps takes no pull
  requests.** A `pull_requests` table on a branch of a policy that
  names no `[signers]` refuses that branch alone, naming `[signers]`,
  as a `[[checkout.cache]]` does (56, S64, 137). A submit to it exits 2
  before anything is pushed or opened; a step, a pass, `status` and the
  overview show the refusal; every other branch reads as before, and
  routing still reads its remote (rule core.27). Its hosts' operators
  name `[signers]` before they install the release that carries this.
- **No hand-over.** No withdrawal is written for one. A withdrawal an
  older sluice wrote for a hand-over, by `withdrawn.handed_over_to` or
  the words 0.60.0 to 0.70.2 gave it, is read as written: it holds no
  request, and a wait on it follows its request (record 0088 as
  amended; rule core.33's details), so a branch whose records move to
  its remote takes such a request in again with no draft.
- **A host that does not keep the branch shows each request it
  follows.** The rows record 0095 gave a host without the mark are every
  host's that does not keep the branch: `sluice status <branch>`, the
  overview and a wait show each request the host submitted or follows
  whose submission the branch's queue does not hold at its head, read
  from sluice's check on its head (137). `taken_in_by` names the host
  the branch's records name as keeping it, with no holder.
- **A step prints what its round met.** `sluice step` prints every
  warning its round's intake met, on stderr, as a step run in its own
  process did (rule core.14).
- **A standing failure is said once for each cause and host.** An
  intake failure, the branch's whole intake or one request's (128), is
  kept as this host's refusal of the branch's forge, beside its refusal
  of the remote, through the records seam: a `read-refused` line marked
  `intake` in the branch's log for each distinct cause and host while it
  stands, and a `read-again` line so marked at its end. Whether a cause
  was said is
  read from that record, never from a process's memory. `sluice status`
  shows it while it stands (rules core.28, core.47, core.11).
- **The tests and the unfinished changes built on the mark.** The
  pull request suite gives `main` its records on its remote, signed by
  the host that keeps it, and its tests of the mark, its lease, its
  holds and hand-overs are rewritten to the host that keeps the branch
  or taken out. The two changes built on records 0088 and 0089's mark
  are taken out of `openspec/changes/`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `request-intake-host`: one host takes a branch's requests in, the one
  that keeps it, where its records are on its remote; every mark,
  lease, hold and hand-over requirement goes; a host that does not keep
  the branch shows and follows the requests it follows from sluice's
  check; an older hand-over's withdrawal holds no request.
- `pull-request-intake`: a pull request table on a branch whose records
  each host keeps refuses that branch alone, a submit to it exiting 2
  before anything is pushed or opened; a request sluice cannot read, and
  a failure that is no one request's, is said once for each cause and
  host and logged on the branch, a step naming it each time.
- `remote-read-log`: intake's refusal of the forge is logged once per
  cause and host with its end, kept on the host until written, and
  shown by status.
- `host-process`: `sluice step` prints the warnings its round met.
- `submission-wait`: a wait across hosts names the host that keeps the
  branch, with no holder.
- `overview`: a branch another host keeps lists the requests this host
  follows, naming the keeper.
- `obsolete-combinations`: a hand-over is no longer among the
  withdrawals that make a combination obsolete.

## Impact

- `crates/sluice/src/profiles/pushed_refs.rs`: `intake_ref`,
  `intake_mark`, `intake_mark_at`, `put_intake_mark`,
  `drop_intake_mark`, `Mark`, `Holder` and the mark's refspec in the
  branch's fetch go.
- `crates/sluice/src/profiles/github/kept.rs`: `intake/` and its
  `.waits/` go (`Observed`, `observe`, `Hold`, `hold`, `let_go`).
- `crates/sluice/src/profiles/github/pulls.rs`: `Pulls::takes` is the
  instance that keeps the branch; `decide`, `put_mark`, `hold`,
  `hand_over`, `let_go_marks`, `Taking::Undecided` go; intake's
  warnings are returned, not printed; `across` and `kept_across` read
  every branch whose records are on its remote.
- `crates/sluice/src/lib.rs`: `Taker`, `Holder`, `PullRequests::let_go`
  go; `TakenInBy` loses `holder`; `crate::intake` gives its warnings to
  its caller and keeps no `thread_local` of failures.
- `crates/sluice/src/host_process/pass.rs`: a step's answer carries its
  warnings; a pass says a cause on the pass that logs it.
- `crates/sluice-core`: `Requester::HandOver` and `Sluice::drop_obsolete`
  go, `Withdrawal`'s `handed_over_to` is read and never written; the
  policy's refusal of a
  pull request table on a branch whose records each host keeps
  (`Policy::rules_for`, `Policy::remote_of`); intake's refusal kept and
  logged as 133's is (`Refusal`, `Event::ReadRefused` and
  `Event::ReadAgain` gaining `intake`, `Sluice::note_intake_refused`,
  `Sluice::note_intake_read`); the records seam keeps the second
  document beside the first.
- Records: `read-refused` and `read-again` lines of `sluice/log@1` gain
  an optional `intake`, which an older sluice passes over; no field is
  removed from `sluice/submission@1`.
- Tests: `crates/sluice/tests/profiles/pull_requests.rs`'s `OnGitHub`
  gives `main` signers and its records on its remote;
  `crates/sluice/tests/pushed_refs.rs`'s and `kept.rs`'s tests of the
  mark and holds go.
- Docs: a record for this change's choices, records 0095 and 0096
  marked amended in part, rule core.33's details and core.28's; the
  pull requests guide, the state and commands references, and the
  plugin's pull request reference and GitHub examples; the release
  notes say first that such a branch is refused.
- The orchestrator is told: sluice's own `main` and
  WilldanGroup/switchboard-kit's `main` take pull requests with no
  `[signers]`, so each is refused on the release that carries this
  until its policy names signers; crew reads `taken_in_by` from a
  wait's document and loses `holder`.
- Out of scope, each its own unit from record 0093's amendment of
  2026-11-08: a machine whose signature a branch does not accept
  leading none of its rounds (137, 78); `sluice daemon`; the GitHub
  Action way (`actions-pool`), whose workflow the refusal names once
  that change builds it.
