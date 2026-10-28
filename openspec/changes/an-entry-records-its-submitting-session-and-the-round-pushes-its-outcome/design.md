# Design

## Context

See proposal.md for why. What is built on main at 2026-10-28:

- `Submitter` (`crates/sluice-seams/src/lib.rs`) holds `agent`,
  `worktree`, `path` and `head`. The worktrunk profile fills it in
  `Worktrunk::session` from git and `SLUICE_AGENT`. Nothing records a
  pane or a host.
- `Driver::notify(&Notification) -> Result<()>`. The worktrunk profile's
  returns `Ok(())` and does nothing. The pull request profile's posts the
  outcome on the request, and returns `Ok(())` with nothing posted when it
  holds no request for the submission.
- `Step::perform_outbox` (`crates/sluice-core/src/step.rs`) calls
  `notify` for each outbox item and, on `Ok`, removes the item, sets the
  submission's `told` to the time and logs `Event::Told`. An `Err` leaves
  the item for the next round.
- `Submission.told` is `Option<String>`, a time. No reader in the tree
  reads it but the JSON a wait or status prints.

The herdr CLI as installed (0.9.5), read for this change:

- Every pane herdr runs has `HERDR_PANE_ID` (for example `w1Z:p1`) and
  `HERDR_SOCKET_PATH` in its environment; a named server also sets
  `HERDR_SESSION`. Pane IDs are scoped to one server and never reused
  once closed. A pane moved to another workspace gets a new ID.
- `herdr agent prompt <pane-id> <text>` sends the text and Enter to the
  agent in that pane as one submission and exits 0 once both are
  written. It refuses with `agent_blocked` before typing anything when
  the agent waits on a question, and `agent_not_found` when the pane
  holds no agent or is gone. Server errors are JSON on stderr, exit 1.
- The CLI reaches the server named by `HERDR_SOCKET_PATH` in its
  environment (`--session <name>` overrides it). Checked: with
  `HERDR_SOCKET_PATH` set to a missing socket it reports
  `server_not_running` at that path; with it unset it reaches the
  default server, where another server's pane is `pane_not_found`.

## Goals / Non-Goals

**Goals:**

- The submitting pane recorded at submit, the outcome pushed into it by
  the deciding round, told recorded only on a delivery.
- No test records or prompts a real pane.

**Non-Goals:**

- The command a starting session runs for its untold outcomes, a
  reporting wait marking told, and the board's carrying: later units.
- Any multiplexer but herdr. A second one is a second arm in the same
  profile.
- Pushing across hosts. A round on the gate box does not reach a pane on
  a laptop; that outcome stands untold for the session's own ask.

## Decisions

### The session is the profile's name for the pane, kept opaque to the core

`Submitter` gains two optional fields, both skipped when absent so old
records read unchanged:

- `host`: the host the submit ran on, as its peers name it: the
  instance's `[peers] name`, else the hostname. This is the name a
  host's lease already goes by. `profiles/mod.rs` hands it to
  `Worktrunk` at binding, so the worktrunk profile calls no other
  profile (core.2).
- `session`: the multiplexer's name for the pane, formed and read by the
  worktrunk profile alone: `herdr:<pane-id>@<socket-path>`, from
  `HERDR_PANE_ID` and `HERDR_SOCKET_PATH`, recorded only when both are
  set and not empty.

The socket path names the server rather than `HERDR_SESSION`, because
every herdr pane has it and the default server has no session name. The
same string is what a starting session will compute from its own
environment to find its entries (the later ask), so it is formed by one
function the profile exposes. The core compares and shows the string
and never parses it (core.1).

*Alternatives:* a structured `{multiplexer, server, pane}` in the seams
puts the multiplexer's shape in the core's types; `HERDR_SESSION` alone
misses the default server; the agent's own session id (herdr reports
Claude Code's) changes on every restart, and the pane is what a
restarted session comes back in.

### notify answers whether it delivered

`Driver::notify` returns `Result<Delivery>`:

```rust
pub enum Delivery {
    /// Delivered to this session, as the driver names it.
    Delivered { to: String },
    /// Not delivered, and why, in the carrier's words.
    Undelivered { why: String },
}
```

`Err` keeps its meaning, sluice's own failure (a forge write the retry
policy gave up on): the item stays in the outbox for the next round, as
built. A carrier that was asked and answered no is `Undelivered`, never
`Err`, so nothing loops on a closed pane.

- Worktrunk: no session recorded → `Undelivered { "no session recorded" }`
  with no call. A session recorded on another host → `Undelivered`,
  naming that host, with no call. Otherwise `herdr agent prompt` with the
  recorded pane, `HERDR_SOCKET_PATH` set to the recorded socket and the
  round's own `HERDR_*` variables removed. Exit 0 → `Delivered { to:
  session }`. Anything else → `Undelivered` with herdr's error message
  from its JSON, or its stderr as written when that is not JSON (core.28's
  rule of the remote's own words, applied to the carrier). The call is
  bounded to ten seconds; one that does not answer is killed and is
  `Undelivered { "herdr did not answer within 10s" }`.
- Pull request: posted → `Delivered { to: "pull/<n>" }`; no request held
  for the submission → `Undelivered`.

*Alternative:* keep `Result<()>` and map `Err` to untold. That cannot
tell a carrier's no from sluice's own failure, and the forge's writes
need the second to stay retried.

### The outbox: one try, told only on Delivered

`perform_outbox`, for each item:

1. A submission already told: remove the item, push nothing (51: no push
   to a session already told).
2. Otherwise notify. `Delivered { to }`: remove the item, set `told`,
   log `Event::Told` as today. `Undelivered { why }`: remove the item,
   leave `told` unset, log `Event::Untold { id, notification, why }` once.

Removing an undelivered item is what keeps a round from trying again:
an item left would make `has_work` true for ever on a branch whose
submitter's pane is gone. The outcome is still in the records, decided
and untold, which is what the later ask reads. A step that stops between
a delivery and its record pushes once more under the same notification
id, as built (51: at least once, named so a repeat is known).

### told names when and to which session

`Submission.told` stays the time, and `Submission.told_to:
Option<String>` is added beside it, skipped when absent. Clause 51 and
ADR 0068 ask that the record name when and to which session; the two
fields do, and an older sluice on another host submitting to the same
branch, which ignores fields it does not know, reads the record
unharmed (core.11). Records written before keep `told` with no
`told_to`. Those were marked told by a notify that may have delivered
nothing; they are left as they read rather than rewritten, since they
name no session and no ask could find them.

*Alternative:* `told` as an object `{at, to}`. It reads more naturally,
but a sluice from before this change fails to parse the submission, so
every host on a branch whose records are on its remote would have to
upgrade at once.

### The words pushed

One line, opening `[sluice]` so the agent and a reader of its transcript
never take it for the user's typing, naming the submission by `#<id>`,
the branch and the outcome, and the one command that shows the rest:

- `[sluice] #12 landed on main as abc1234.`
- `[sluice] #13 failed on main: cli:test, core:test. sluice wait 13 shows why.`
- `[sluice] #14 conflicts on main with #11 in src/lib.rs. sluice wait 14 shows it.`

A failure that the time limit ended says `timed out` in place of the
check list. Formed in the worktrunk profile from the `Notification`'s
`Told`; the later ask uses the same function so a session is told in the
same words however the outcome reaches it.

### Tests never reach a real pane

`support::isolated` removes every `HERDR_*` variable, and the five tests
that start sluice without it (`instance.rs`, `peers.rs`,
`remote_branch.rs`, `stack.rs`, `usage.rs`) go through it; `e2e/e2e.sh`
unsets them. Tests that exercise the push put a fake `herdr` first on
`PATH` that records its arguments and environment and answers as told,
and set `HERDR_PANE_ID` and `HERDR_SOCKET_PATH` to scratch values.

## Risks / Trade-offs

- [A transient refusal, an agent at a question for a moment, leaves the
  outcome untold until the ask lands] → The record is truthful, and
  `sluice wait` still reports it. The ask, the next unit, closes it.
- [Until the board gives up its own told list, a Claude Code session in
  herdr is told by both the push and the board] → Named in the proposal;
  the board's unit removes it.
- [A pane moved to another workspace changes its ID, so the push misses]
  → Untold, not mis-told: a closed or moved ID is never reused, so no
  other pane is prompted.
- [A herdr call that hangs holds the round] → Bounded at ten seconds;
  a round has at most one push per decided submission.
- [Old records keep `told` from a notify that delivered nothing] →
  Accepted: they name no session, and rewriting history is worse.
- [`Submitter` gains fields an older sluice ignores, so an older host
  running a round pushes nothing and marks told as it always did] →
  Until every host on a branch runs this change, an older host's round
  may still mark an outcome told that nobody was told.

## Migration Plan

The fields added are optional and skipped when absent, and an older
sluice ignores them. The `untold` log event is new, and an older sluice
refuses a log line it does not know, as with each event added before
(`read_refused`, `gone`): on a branch whose records are on its remote,
every host submitting to it runs this change before any round writes
one. On one host, nothing to do. Rollback is a revert, after removing
any `untold` lines from the branch's log.
