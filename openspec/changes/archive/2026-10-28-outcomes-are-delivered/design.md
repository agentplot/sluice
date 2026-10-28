# Design

## Context

See proposal.md for why. The words are record 0070's: an outcome is
**delivered** once a carrier has put it before the session that
submitted it, **pending** until then; a session **collects** what is
pending by `sluice inbox`; a push carries a **notice**; whoever an
outcome goes to is its **recipient**. Statement clause 51, the C.5
worktrunk row and rule core.31 say it in those words; where an older
text on disk still says told, untold or asking, this design and the
words above are what to follow.

### What is built on this branch

The branch `outcomes-are-delivered` holds the slc-2 team's work merged,
under the former words. An implementer starts from it, not from main.

- **Seams** (`crates/sluice-seams/src/lib.rs`): `Submitter` has the
  optional `host` and `session`, skipped when absent. `Delivery {
  Delivered { to }, Undelivered { why } }`, and `Driver::notify ->
  Result<Delivery>`, whose doc says a carrier's no is `Undelivered` and
  `Err` is sluice's own failure. The outcome a notification carries is
  the enum `Told { Landed, Failed, Conflict }`, `Notification.told`.
- **Records** (`crates/sluice-core/src/records.rs`): `Submission.told`
  (a time) and `Submission.told_to`; the log events `Told { id,
  notification }` and `Untold { id, notification, why }`; the outbox
  item `OutboxItem { id, submission, told: ToldRecord }`;
  `Submission::told_outcome()` and `Submission::notification()`.
- **The round** (`crates/sluice-core/src/step.rs`, `perform_outbox`):
  one try per outcome; a submission already marked has its item removed
  and nothing notified; `Delivered` marks and logs, `Undelivered` removes
  the item, marks nothing and logs `untold` once; `Err` leaves the item.
- **The collect operation** (`crates/sluice-core/src/ops.rs`):
  `Sluice::untold(&Submitter) -> Untold`, which finds on every branch
  the landed, failed and conflicting submissions not marked whose
  `from.worktree` or `from.head` is the asking worktree's branch, from
  the live records and from the finished records of that branch's own
  landings alone (131); takes each such branch's lock in sorted order
  and reads again; `marked(to)` gives the records as they will be
  marked, and `mark(to)` writes them, one `update` per branch, removing
  any outbox item and logging under the item's id or
  `Submission::notification()`. `UntoldOutcome` is one found; a branch
  it could not read is in `unread`. `Line` rewrites a finished record
  it changed (`crates/sluice-core/src/line.rs`), and both records
  profiles replace a finished record put again (tests in
  `crates/sluice/tests/local_files.rs`, `remote_refs.rs`).
- **The command** (`crates/sluice/src/lib.rs`): `Command::Untold {
  worktree, json }`, `fn untold`, `UNTOLD_AFTER_HELP`. It writes and
  flushes its output before `mark`. `--json` prints `{"outcomes":
  [...]}` and `unread` when a branch could not be read; exit 3 then.
- **The words** (`crates/sluice/src/words.rs`): `told_line(id, branch,
  &Told)`, the one `[sluice]` line, which the worktrunk profile and the
  command both call (rule core.1).
- **The worktrunk profile**: `profiles/worktrunk/herdr.rs` names the
  session from `HERDR_PANE_ID` and `HERDR_SOCKET_PATH` and reads it back
  (`pane_of`); `Worktrunk` is given the host's name at binding in
  `profiles/mod.rs`; `Worktrunk::session` records both on the
  `Submitter`. Its `notify`, as merged, answers `Undelivered { "no
  session recorded" }` always.
- **The pull request driver** (`profiles/github/pulls.rs`): answers
  `Delivered { to: "pull/<n>" }` once posted, `Undelivered` where it
  holds no request.
- **Fakes and conformance**: `FakeDriver::notify` delivers to the
  submitter's session or answers `Undelivered` for an agent a scenario
  marks; `crates/sluice-conformance/tests/told.rs` and `untold.rs`.
- **Tests' isolation**: `support::isolated` removes every `HERDR_*`
  variable and every sluice command in `crates/sluice/tests` is built
  through it; `e2e/e2e.sh` unsets them.
- **CLI tests** (`crates/sluice/tests/profiles/worktrunk.rs`): `sluice
  untold` tells a landing once and marks it to the worktree's path, a
  blame as `sluice wait` reports it, `--json` then `{"outcomes":[]}`,
  nothing of another worktree's, nothing in a detached worktree.
- **The board** (`plugins/sluice-board`): no told list; `askedWords`;
  `ask` serialised by `asking` and `again`; runs `sluice untold` at the
  first overview and on a new outcome of the session's; its tests, its
  README, its `wake` option and the Claude Code guide. Complete under
  the former command's name.
- **Docs**: `reference/commands.md` has a `sluice untold` section;
  `reference/state.md` says `told` and `told_to`; the using-sluice skill
  names `sluice untold`.

### What is not merged

`unit/an-entry-records-its-submitting-session-and-the-round-pushes-its-outcome`
at 8a684af5 is a work-in-progress commit of an agent stopped mid-task,
unverified. Against the branch it adds:

- in `herdr.rs`: `BOUND` (ten seconds), `Answer { Took, Refused }`,
  `prompt(pane, socket, line, bound)`, which runs `herdr agent prompt`
  with every `HERDR_*` variable of its own removed and
  `HERDR_SOCKET_PATH` set to the recorded socket, and `refusal`, herdr's
  words from its JSON error or its stderr; a unit test of `refusal`;
- in `worktrunk/mod.rs`: `notify` as designed below, and the module's
  opening comment rewritten;
- in `tests/profiles/worktrunk.rs`: a fake `herdr` and six tests of the
  push;
- in `tests/profiles/pull_requests.rs`: a test that a landed request's
  submission names `pull/<n>`;
- in `site/src/content/docs/reference/state.md` and
  `docs/examples/state/bolt/concurrent-units/submissions/13.json`: the
  session, the host and the delivery.

Merging it conflicts in `herdr.rs`, where the WIP still holds the words
function that the inbox work moved to `words.rs`, and in `state.md`,
where both sides wrote the paragraph on the field. The words stay in
`words.rs` (core.1: code outside a profile names no profile, so the
command cannot call into the worktrunk profile); `herdr.rs` keeps the
pane, the prompt and the refusal; `notify` calls `words::told_line`.

### The herdr CLI (0.9.5), as read for this change

- Every pane herdr runs has `HERDR_PANE_ID` (for example `w1Z:p1`) and
  `HERDR_SOCKET_PATH`; a named server also sets `HERDR_SESSION`. Pane
  IDs belong to one server and are never reused; a pane moved to another
  workspace gets a new one.
- `herdr agent prompt <pane-id> <text>` types the text and Enter to the
  agent in that pane and exits 0 once both are written. It refuses with
  `agent_blocked` before typing when the agent waits on a question, and
  `agent_not_found` when the pane holds no agent or is gone; errors are
  JSON on stderr, exit 1.
- The CLI reaches the server `HERDR_SOCKET_PATH` names; unset, the
  default server, where another server's pane is `pane_not_found`.

### Flume's use

Flume submits a place with `sluice submit <base> --no-wait --json`, run
in the place with no multiplexer's variables in its environment; it
never runs `wt submit` and never waits. At each settle it runs `sluice
inbox --json` in the place, and `sluice step <base>` where no `sluice
run` is live on the host (flume record 0012; flume 133, flume C.6).

## Goals / Non-Goals

**Goals:**

- An outcome delivered once, by whichever carrier comes first, and
  recorded as delivered only on a carrier's word (51; core.31).
- One command by which a session, or a caller's place, collects what is
  pending for it, with a JSON document another tool can rely on.
- The former words gone from code, output and docs, with records
  written under them still read.

**Non-Goals:**

- Any multiplexer but herdr; pushing across hosts; delivering a
  withdrawal (proposal).
- A change to `sluice step`.

## Decisions

### The session is the profile's name for the pane, opaque to the core

`Submitter.session` is `herdr:<pane-id>@<socket-path>`, formed and read
by the worktrunk profile alone, recorded only when both variables are
set and not empty; `Submitter.host` is the instance's `[peers] name`,
else the hostname, given to `Worktrunk` at binding so it calls no other
profile (core.2). The socket names the server because every pane has
it and the default server has no session name. The core compares and
shows the string and never parses it (core.1). Built.

*Alternatives:* a structured `{multiplexer, server, pane}` puts the
multiplexer's shape in the core's types; `HERDR_SESSION` misses the
default server; the agent's own session id changes on every restart.

### notify answers whether it delivered

`Driver::notify -> Result<Delivery>`. `Err` stays sluice's own failure,
retried by the next round; a carrier that answered no is `Undelivered`,
never `Err`, so nothing loops on a closed pane. Built.

The worktrunk driver's `notify` (WIP, finished in group 2):

- no session recorded → `Undelivered { "no session recorded" }`, no call;
- a session recorded on another host → `Undelivered` naming both hosts,
  no call;
- a session that is no herdr pane → `Undelivered`, no call;
- otherwise `herdr agent prompt <pane> <notice>` under the recorded
  socket with the round's own `HERDR_*` removed, bounded to ten seconds:
  exit 0 → `Delivered { to: session }`; otherwise `Undelivered` in
  herdr's words, its JSON error's `message` or its stderr as written
  (core.28's rule of the remote's own words, applied to the carrier); no
  answer within the bound → killed, `Undelivered { "herdr did not answer
  within 10s" }`.

The pull request driver: posted → `Delivered { to: "pull/<n>" }`; no
request held → `Undelivered`. Built.

### The outbox: one try, delivered only on Delivered

For each outbox item: a submission already delivered has its item
removed and nothing notified; otherwise notify. `Delivered { to }`
removes the item, sets `delivered` and `delivered_to`, logs
`delivered`. `Undelivered { why }` removes the item, leaves both unset,
logs `pending { id, notification, why }` once. `Err` leaves the item.
Removing an undelivered item is what keeps later rounds from trying
again; the outcome is still in the records, decided and pending, for
`sluice inbox`. A step that stops between a delivery and its record
pushes once more under the same notification id (51: at least once,
named so a repeat is known). Built under the former names.

### The notice

One line, opening `[sluice]` so neither the agent nor a reader of its
transcript takes it for the user's typing, naming `#<id>`, the branch,
the outcome, and the command that shows the rest:

- `[sluice] #12 landed on main as abc1234.`
- `[sluice] #13 failed on main: cli:test, core:test. sluice wait 13 shows why.`
- `[sluice] #14 conflicts on main with #11 in src/lib.rs. sluice wait 14 shows it.`

A failure the time limit ended says `timed out` in place of the check
list. The same function words the push, each outcome `sluice inbox`
prints, and the `notice` field of its JSON, so a recipient reads the
same words however the outcome reaches it. Built as `told_line`;
renamed `notice` in group 2.

### `sluice inbox`: what it collects, and for whom

`sluice inbox [--worktree <path>] [--json]`, a one-shot read and write of
the records, as `sluice withdraw` is: it runs no round and follows no
reader (core.25 allows a one-shot read; core.4: it writes no branch's
ref).

- **Whose.** A submission is the worktree's when its `from.worktree` is
  the worktree's branch or its `from.head` is (a pull request opened
  from it). That is how `sluice wait <branch>` already finds a
  worktree's submissions, and it survives what the command exists for:
  a restarted agent comes back in a new pane, on the same branch. The
  recorded `session` is not a match key, since a pane's ID is never
  reused. A worktree on no branch has submitted nothing (3).
- **What.** `landed`, `failed` or `conflict`, and not delivered. A
  withdrawal is not delivered (proposal); a resubmission withdraws the
  failure it replaces, so a superseded failure is never delivered.
- **Where it reads.** The live records of every branch, and the finished
  records of the worktree's own landings alone, by name from the
  finished index (131). A failure or conflict stays live (130).
- **To whom it marks.** `delivered_to` is the `Submitter.session` the
  driver gives for the worktree where it runs in a multiplexer, else the
  worktree's path, else its branch.

### Delivered once: under each branch's lock, printed before marked

For each branch holding a pending outcome of the worktree's, in sorted
order, the command takes the branch's lock (`wait_lock`), reads again,
writes its whole output and flushes it, then marks each outcome in one
`update` per branch: `delivered` and `delivered_to` on the record, live
or finished; the outbox item removed if one stands; the `delivered` log
line under the item's id or `Submission::notification()`, the id the
round would have used. Then the locks go.

Holding the lock across printing and marking is what makes one outcome
delivered once: the round's outbox runs under the same lock, so it
either pushed and marked before the command read, or runs after and
finds it delivered. Printing before marking is what makes it delivered
at least once: a command stopped between them marks nothing, and the
next carrier delivers it again under the same notification. Taking the
locks in one order means two collectors never wait on each other. A
write refused because another host wrote the remote records is read
again by `update` (62); one found delivered there is not marked twice.
Built.

*Alternatives:* marking before printing loses an outcome to a stop,
which 51's at-least-once forbids; no lock lets the round push what the
command just printed.

### The text and the JSON document

Text: for each outcome, in branch then id order, the notice, then what
`sluice wait <id>` prints for it; a blank line between outcomes; nothing
at all when nothing is pending.

`--json` is a contract another tool reads (Flume reads it, flume C.6),
so its shape is pinned here and in the spec, and it only gains fields:

```json
{
  "outcomes": [
    {
      "schema": "sluice/submission@1",
      "id": 13,
      "branch": "main",
      "state": "failed",
      "from": { "agent": "unit-a", "worktree": "unit/a", "path": "/repo/places/unit-a" },
      "failed": [ { "task": "cli:test", "log": "…/cli:test.log" } ],
      "delivered": "2026-10-28T10:02:41Z",
      "delivered_to": "/repo/places/unit-a",
      "notice": "[sluice] #13 failed on main: cli:test. sluice wait 13 shows why."
    }
  ],
  "unread": [ { "branch": "bolt/x", "error": "<the reader's words>" } ]
}
```

- `outcomes` is always present, `[]` when nothing is pending; each
  element is the submission's record as `sluice wait <id> --json` prints
  it, with `delivered` and `delivered_to` as this run marks them, and
  `notice`, the one line.
- The fields a reader may rely on: `id`, `branch`, `state` (`landed`,
  `failed` or `conflict`); for `landed`, `landed_as`; for `failed`,
  `failed[].task`, `failed[].log` where kept, `result` (`timed out` when
  the time limit ended it, else `failed`); for `conflict`,
  `conflict.paths` and `conflict.with[].id`; `delivered`, `delivered_to`,
  `notice`; `from.worktree`, `from.path`.
- `unread` is present only when a branch could not be read.
- sluice's own failure is the error document `{"error": {"kind",
  "message", "next"}}`, exit 3, as every `--json` command's.

Exits: 0 whether anything was pending or not; 3 when a branch could not
be read, after delivering what it could, or when sluice itself failed.

*Alternative:* a document of its own shape, made for Flume. It would
be a second description of a submission to keep in step with the
record; the record with one added field serves both.

### A branch it cannot read

Said on stderr as `sluice: could not read <branch>'s records: <the
reader's words>`, kept in `unread` under `--json`; the other branches
are still collected (core.11), exit 3. Built.

### A reporting wait marks delivered

51 has a reporting wait deliver as a push and `sluice inbox` do. When
`sluice wait`, a waiting `sluice submit` or a `wt submit` (which waits
through the same `await_submission`) reports a landed, failed or
conflicting submission as its outcome, and the submission is the
waiting worktree's by the rule `sluice inbox` uses, the wait marks it
delivered to the same recipient name, under the branch's lock, after its
report is written and flushed, through the same core operation
narrowed to that one submission. An outcome already delivered is
reported as it is and marks nothing; a wait on another worktree's
submission, the operator's say, marks nothing, since it is not the
submitter's session (20). Under `--json` the record printed carries the
`delivered` and `delivered_to` the wait marks. A wait that ends
undecided, by its timeout or Ctrl-C, marks nothing.

A waiting pull-request submit, and `sluice wait <branch>` on a branch
that takes pull requests, mark the same way: the request was opened
from the waiting worktree's branch, which the submission records as its
`from.head`. The pull request driver has usually delivered the outcome
to the request already, and then the wait reports that mark unchanged.

A group submit's wait (125) marks nothing: it waits on several
worktrees' submissions from the directory it runs in, which is no one
of their sessions, so each outcome stays pending for its own worktree's
`sluice inbox`, the push or the pull request.

*Alternative:* leave the wait out, as the earlier changes did. Then an
agent that waited on its own submission is delivered it twice, once by
its wait and again by `sluice inbox` or the push, which 51 forbids.

### The record's fields: new names, former ones read

`Submission.delivered` and `Submission.delivered_to` are written under
those names and read under `told` and `told_to` too (`#[serde(alias)]`),
so a record written by an earlier sluice reads as it was, and nothing
on disk is rewritten (record 0070). The log event is written `delivered`
and read as `told` too; the event `pending` replaces `untold`, which no
released sluice wrote, so it needs no alias. `sluice/submission@1`
stands: a field that only an earlier name of reads is absent, which a
reader already handles for a record written before.

The outbox item's on-disk key stays `told`: the outbox is what an older
sluice on another host reads to run its rounds, and renaming that key
would make an older host's round fail to read the branch rather than
miss a mark. In Rust the field becomes `outcome: DecidedRecord` with
`#[serde(rename = "told")]`.

### The rename

One mechanical group, done second, so every test and doc written after
it is in the new words once (the merged work and its finished tests are
renamed with it):

| former | new |
|---|---|
| `sluice untold`, `Command::Untold`, `UNTOLD_AFTER_HELP`, `fn untold` | `sluice inbox`, `Command::Inbox`, `INBOX_AFTER_HELP`, `fn inbox` |
| `Told` (seams), `Notification.told` | `Decided`, `Notification.decided` |
| `ToldRecord`, `OutboxItem.told`, `to_told` | `DecidedRecord`, `OutboxItem.outcome` (on disk `told`), `to_decided` |
| `Submission.told`, `told_to` | `delivered`, `delivered_to` (aliases `told`, `told_to`) |
| `Submission::told_outcome` | `Submission::decided` |
| `Event::Told`, `Event::Untold` | `Event::Delivered` (alias `told`), `Event::Pending` |
| `Sluice::untold`, `untold_on`, `Untold`, `UntoldOutcome` | `Sluice::pending`, `pending_on`, `Pending`, `PendingOutcome` |
| `words::told_line` | `words::notice` |
| conformance `tests/told.rs`, `tests/untold.rs` | `tests/delivered.rs`, `tests/inbox.rs` |
| board `untoldRun`, `untoldAfresh`, `askedWords`, `ask` | `inboxRun`, `inboxAfresh`, `collectedWords`, `collect` |
| tests and comments saying told, untold, ask | delivered, pending, collect |

The `told` module in `crates/sluice/src/told.rs`, which writes a
command's error document, is another sense of the word and keeps its
name. A test that names a herdr plugin's toast list keeps its words: the
herdr plugin's list is the user's toasts, which deliver nothing (51).

### Flume's case

A place submitted by `sluice submit <base> --no-wait --json` with no
`HERDR_*` in its environment records `from.path` (the place) and no
`session`. The round that decides it calls `notify`, which answers
`Undelivered { "no session recorded" }` with no call: the outbox item
goes, one `pending` line is logged, and no later round tries again. The
next settle's `sluice inbox --json` in the place answers the outcome
once, marks it delivered to the place's path, and a second `sluice
inbox --json` answers `{"outcomes":[]}`. Nothing here is Flume's:
sluice names no caller (core.1); this is the case of any submit with no
multiplexer, which the tests exercise as Flume runs it.

`sluice step <base>` needs no change: one round, its outbox run as any
round's, `busy` when another holds the branch's lock (core.14), and its
`--json` answer as now.

### Tests never reach a real pane

`support::isolated` clears every `HERDR_*` variable (built). Tests of
the push put a fake `herdr` first on `PATH` that records its arguments
and `HERDR_SOCKET_PATH` and answers as the test sets, and set
`HERDR_PANE_ID` and `HERDR_SOCKET_PATH` to scratch values (WIP).

## Risks / Trade-offs

- [A transient refusal, an agent at a question for a moment, leaves the
  outcome pending] → The record is truthful; the next `sluice inbox`, a
  wait or the board delivers it.
- [A pane moved to another workspace changes its ID, so the push misses]
  → Pending, never delivered to another pane: a closed or moved ID is
  never reused.
- [A herdr call that hangs holds the round] → Ten seconds, one push per
  decided submission per round.
- [An older sluice on another host rewrites a submission and drops
  `delivered`, which it does not know] → The outcome reads pending again
  and is delivered once more, a repeat its notice names. The Migration
  Plan asks every host on such a branch to upgrade.
- [Records an earlier sluice marked `told` with nobody told] → Read as
  delivered and left: they name no recipient, and rewriting history is
  worse.
- [The board sees a landing before the round's push and collects it
  first] → Delivered once either way; the session reads sluice's words.
- [A long-lived worktree reads one finished record per landing on each
  collect] → Bounded by its own submissions.
- [Flume's statement lists `withdrawn` among what inbox answers, and
  `conflict` not] → sluice delivers landings, blames and conflicts (20,
  51); Flume reads a withdrawal by `sluice status`. Raised with the
  statements' owners (proposal).

## Migration Plan

Records written before read unchanged: the new fields' former names are
aliases, the outbox keeps its key. The log's `delivered` and `pending`
events are new to an older sluice, which refuses a log line it does not
know, as with each event added before: on a branch whose records are on
its remote, every host that submits to it runs this change before any
round writes one. On one host, nothing to do. The board and the
using-sluice skill ship with the sluice that has `sluice inbox`; a board
against an older sluice toasts that it could not collect. Rollback is a
revert, after removing any `delivered` and `pending` lines from the
branches' logs.
