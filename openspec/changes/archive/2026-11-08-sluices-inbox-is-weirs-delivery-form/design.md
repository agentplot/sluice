# Design

## Context

A decided outcome reaches its submitter today by one of four carriers,
and the first to answer marks it delivered for good:

- the round's push, `Step::perform_outbox`, which asks
  `Driver::notify` for each outbox item `Line::push_outcome` queued and
  sets the submission's `delivered` and `delivered_to` on
  `Delivery::Delivered`, or logs one `pending` line on `Undelivered`;
- the pull request driver's `notify`, which posts the outcome on the
  request and answers `Delivered { to: "pull/<n>" }`;
- `sluice inbox`, through `Sluice::pending` and `Pending::mark`, which
  prints each pending outcome of the folder's work and marks it;
- a wait that reports its own submission's outcome, through
  `delivering` and `Sluice::pending_one`.

`Pending::mark` writes `delivered` into a finished submission's record,
which nothing else rewrites (core.41). Nothing shows an outcome nobody
collected.

agentplot/weir record 0006 gives Flume and sluice one form, and weir's
`weir-delivery` builds it at 52b9552: four records (`weir-delivery/1`,
`weir-handover/1`, `weir-confirmation/1`, `weir-known/1`) in recutils
and JSON, and `record`, `hand`, `confirm` and `owed`, functions over the
records a caller passes that return the records to add (weir 108 to
111). A deliverer implements `weir_delivery::Store`, `read` and `add`,
over its own store, and `weir-pack`'s `conform_deliveries` holds that
store to the pack's delivery cases (weir 112, 113). The statement's 51,
as amended at agentplot/blueprints 3bb8688, makes an entry's outcome
such a delivery.

### Looking across

- **weir** owns the form, the crate and the pack. sluice calls them and
  checks nothing of the form by a rule of its own (core.52).
- **Flume** moves onto the same crate in its change
  `flumes-inbox-is-weirs-delivery-form`, for a place's ends. Its
  merge-queue profile is also a carrier of sluice's outcomes: it runs
  `sluice inbox --json` in a place and takes each outcome into the
  place's records as an end, which Flume then owes its own recipients
  by the same form. To sluice it is a reader that confirms on receipt
  (D11).
- One design serves both: weir's. The way a tool hands a delivery over,
  when it hands one again, its span and who may confirm for a reader are
  each tool's (weir's non-goals); sluice's are below, and the confirm
  command's shape is offered to Flume to match (D7).

## Goals / Non-Goals

**Goals:** an outcome owed to its submitter's work until that work
confirms it; every hand-over recorded and none confirming by itself; an
outcome nobody collects shown expired with its reader; the branch's
store passing weir's pack.

**Non-Goals:** the overview showing what is owed; Flume's profile;
moving the records an earlier sluice wrote, which is an agent's step
(D12); a change to weir's form.

## Decisions

### D1. sluice takes weir's crates by a git revision

The workspace's `Cargo.toml` names `weir-delivery` by
`git = "https://github.com/agentplot/weir"` at `rev =
"52b95528a495100e34fca39109d563d8bf75673a"`, and `weir-pack` the same
way as a dev-dependency of `sluice-core`, for the pack's test (D4).
agentplot/weir is public, so CI and every host fetch it with no
credential. `flake.nix` gives `cargoLock.outputHashes` the source's
hash, which `nix:typecheck` fails without. The revision moves only by a
change that says so.

Not taken: a path to a sibling checkout, which no host holds at one
place; crates.io, where weir publishes nothing.

### D2. A decided outcome is one delivery, recorded where it is queued

`Line::push_outcome`, the one place every decided outcome is queued,
also records its delivery with `weir_delivery::record` over the
branch's deliveries:

- **key** `<id>:<outcome>`, the outbox item's id, `12:landed`,
  `13:failed`, `14:conflict`, unique in a repository since its
  submission numbers are;
- **reader** the submission's `from.worktree`, as recorded: a
  worktree's branch, or `pull/<n>` for a request intake took in;
- **message** the notice, the one line the round pushes, whose words
  move from the CLI's `words::notice` to `sluice_core::notice` so the
  round writes them, the CLI and the worktrunk profile calling them
  there;
- **when** the round's time, and the **span** the host's (D9).

`record` holds a key once (weir 101), so a round that stopped after the
write and runs again records nothing more. `push_outcome` already skips
a submission by name, which records no delivery (124; core.50); a
withdrawal is queued nowhere and records none. The round passes
`push_outcome` its time and the span, which the core holds on `Peer`,
`expire_after`, beside the other host settings it is given (D9).

Not taken: the session or the path as the reader, which a restart
changes; the agent, which works in many worktrees (record 0102).

### D3. The deliveries are a file of the branch's live records

`deliveries.jsonl`, beside `branch.json` in the branch's live records,
holds the four records in C.8's JSON spelling, `weir_delivery::json`,
one to a line in the order added. `Line::load` reads it, and a write
of the line writes it back with what the write added, under the
branch's lock and through the records seam like every live record, so
it is on the remote where the branch's records are (core.11, core.14).
A line that does not read stops the branch's reads with the file's
name and weir's words, as `branch.json` would.

`Line` implements `weir_delivery::Store` over its deliveries: `read`
gives them in the order added; `add` appends its records, every one or
none, and never lets go of anything. A done delivery, every reader
confirmed (weir 107), is let go of whole, its hand-overs and
confirmations with it, by the write that moves its submission to the
finished records (130), or by the first write after it is done where
its submission has moved already. A delivery not done is never let go
of, whatever becomes of its submission (I9). `weir-known/1` records
stay.

Not taken: recutils, which C.8 gives weir's shared branch (sluice's
records are JSON throughout, core.41); the hand-overs on the
submission's record, which would rewrite a finished record; letting go
inside `add`, which the pack fails (weir 113).

### D4. The branch's store passes weir's pack

A test of sluice-core, `a_branchs_deliveries_pass_weirs_pack`, runs
`weir_pack::conform_deliveries(&Pack::embedded(), ...)` with a store
over a fresh branch's line on the fakes' records seam, and passes on
every delivery case (weir 113). It is the one check that sluice's store
reads back what it adds.

### D5. Every carrier records a hand-over, and confirms only on receipt

`sluice_seams::Delivery` becomes:

```rust
pub enum Delivery {
    /// Handed to the reader by `way`, reaching `to`, as the carrier names
    /// it; `confirms` where its reader confirms on receipt (weir 103).
    Handed { way: String, to: String, confirms: bool },
    /// Not handed over, and why, in the carrier's words.
    Undelivered { why: String },
}
```

The core makes the hand-over with `weir_delivery::hand`, `confirms`
passed through, and adds what it returns under the branch's lock. The
log gains a `handed` line, `{id, key, by, to}`, and a `confirmed` line,
`{id, key, reader, by}`, where `by` is the way, `confirm` or
`operator`; `to` keeps where the hand-over reached, the pane or the
path, which `delivered_to` held. The log's `delivered` and `pending`
lines already written stay as written and read as history (core.41),
and the round still logs `pending` for a push its carrier refused.

The ways:

| way | carrier | confirms on receipt |
|---|---|---|
| `notice` | the round's push through the multiplexer (worktrunk's herdr) | no |
| `pull-request` | the pull request driver's post on the request | yes: the request keeps the post for whoever reads it, and nothing else collects for `pull/<n>` |
| `inbox` | `sluice inbox` | only with `--confirm` |
| `wait` | `sluice wait`, a waiting `sluice submit`, `wt submit` | only with `--confirm`, and `wt submit` for a landing whose worktree it removes |

A hand-over is made only to a reader that has not confirmed (weir 103),
so a wait that reports an outcome its reader confirmed records nothing
and reports it as a read.

Not taken: a session's start confirming what its inbox prints by
default, which weir 0006 allows but which is the case the user asked
about (record 0102).

### D6. What is owed is read with `owed`, by reader

`Sluice::pending` and `pending_one` become `Sluice::owed`, which reads
each branch's deliveries and asks `weir_delivery::owed` at the time
given, taking the readers a folder reads as: its branch, from
`Driver::recipient`, and `pull/<n>` for each request whose `from.head`
is its branch where sluice lands that branch nowhere, by `Here::made`'s
reading (record 0085). A branch it cannot read is among the unread, and
the others are read still (core.11). It locks each branch holding
something owed, in the branches' order, as `pending` does, and writes
nothing until its answer is handed over (`Owed::hand`), when it records
each hand-over and, with `--confirm`, each confirmation, one write per
branch.

A reader's name is known where any branch's store holds its
`weir-known/1` record, so a worktree that confirmed on `main` is not
shown as never confirming on `bolt/x`.

### D7. `sluice confirm <key>...`

Run in a folder, it confirms each key for the readers the folder reads
as (D6), by `weir_delivery::confirm`, finding the key on whichever
branch holds a delivery under it. Its answers, per key:

- confirmed: `confirmed 12:landed (#12 landed on main)`, exit 0;
- confirmed before: `12:landed was confirmed at <time>`, exit 0, nothing
  written (weir 102);
- no branch holds it: not found, exit 1, naming the key, next `sluice
  inbox`;
- owed to a reader the folder is not: refused, exit 2, naming the
  delivery's reader, next `sluice confirm <key> --as operator`;
- a branch it could not read: said with the reader's words, exit 3, as
  the inbox does.

Several keys are answered in the order given, and the exit is the
worst of their exits by that order of severity, 3 over 2 over 1. With
`--json`, one document, `{"confirmed": [{key, branch, reader, at,
first}]}`, `first` true where this call confirmed it, and the error
document of `told` for a refusal. `--as operator` confirms for the
delivery's own reader, so a worktree removed before it confirmed can be
cleared (S77); the `confirmed` line names `by` `operator`. A confirmed
outcome done for every reader is let go of as D3 says.

Flume is offered the same verb, `flume confirm <key>`, so the family
types one word.

Not taken: a flag of `sluice inbox` alone, which a notice cannot name
in one short command for one outcome.

### D8. `sluice inbox`: handed over until confirmed

`sluice inbox` hands over, by `inbox`, every delivery owed or expired to
the folder's readers, and prints each as today, the notice and then
what `sluice wait <id>` prints, and then one line:

```
Confirm it once you have acted on it: sluice confirm 12:landed
```

An expired one's line opens `Expired at <expires_at>, still owed.`.
Run again, it prints the same outcomes until each is confirmed. With
`--confirm`, it confirms each on receipt in the same call and prints no
confirm line; with `--new`, it hands over only deliveries no way has
handed their reader yet. It prints nothing, and `outcomes` `[]`, where
nothing is owed. It holds each branch's lock from its read to its
write, so the round pushes no notice for what it hands over (D10).

Its document's element is the submission's record as `sluice wait <id>
--json` prints it, with `notice`, and `delivery` (D13) in place of
`delivered` and `delivered_to`.

### D9. The host sets the span

`~/.config/sluice/config.toml` gains `[deliveries] expire_after`, a
duration as sluice's settings write one (`parse_duration`: `"168h"`,
`"90m"`), `"168h"` where unset, a week as weir's example has it. A
value that does not read, or under one second, is refused naming the
setting, as every other setting is (`Instance`). The flake's module
declares `deliveries.expire_after` under the file's own name (core.51).

### D10. The round pushes once, and never what was handed over

`perform_outbox` pushes an outbox item's notice only where its delivery
is owed to its reader with no hand-over and no confirmation, then
removes the item whatever the answer, as today: a refused push is
logged `pending` and pushed again by no round. The notice the
worktrunk profile pushes is the delivery's message followed by `
sluice confirm <key> once you have read it.`, so the agent in the pane
knows the key. A step stopped between a push and its record may push
again under the same key, as today.

### D11. Flume's profile confirms on receipt, in Flume's change

Flume's merge-queue profile records each outcome `sluice inbox --json`
gives it as an end of the place, which Flume owes its recipients by
weir's form; for sluice it is a reader that confirms on receipt and runs
`sluice inbox --confirm --json`. It pins sluice 0.71.0, which has no
`--confirm`, so its change, `flumes-inbox-is-weirs-delivery-form`,
moves its pin to the release that carries this change and passes the
flag. Between this release reaching a host and Flume's, each settle
there reads its places' outcomes again, and Flume records an end it
holds already once more. So the release that carries this change is
cut when Flume's change is ready to take it, which ops is told.

### D12. The records sluice wrote before are moved by an agent

Under principle 12 (proposed, from the user's words of 2026-11-08),
no code carries the move. sluice no longer has `delivered` or
`delivered_to` on a submission, and serde passes over the fields where
an older record holds them, so they fall away at the next write of a
live record and stay unread in a finished one. Nothing records a
delivery for an outcome decided before the upgrade.

The step, an agent's on each host after this change is released, named
here and in no task:

1. Before the host takes the release, for each repository and each
   branch with records, list with the sluice it runs, `sluice status
   <branch> --all --json`, the landed, failed and conflicting
   submissions with no `delivered` that are a worktree's or a
   request's (not by name).
2. After it, for each, add a `weir-delivery/1` line to the branch's
   `deliveries.jsonl`, through the branch's records as the host keeps
   them, under the branch's lock: key `<id>:<state>`, reader its
   `from.worktree`, message its `notice`, `recorded_at` its decided
   time and `expires_at` that and the host's span, so an old one shows
   expired at once.
3. Read the branch's `sluice status` and see each shown.

### D13. A submission's `delivery`

`sluice status <id> --json`, `sluice wait --json` and each element of
`sluice inbox --json` give a decided submission's `delivery`:

```json
{"key": "12:landed", "reader": "unit-a", "state": "owed",
 "recorded_at": "...", "expires_at": "...",
 "handed": [{"by": "notice", "at": "..."}],
 "confirmed": null, "known": true}
```

`state` is `owed`, `expired` or `done`; `confirmed` the time the reader
confirmed, once it has; `known` whether any reader has confirmed under
the name. A submission whose delivery was let go of, done, gives
`{"key": ..., "state": "done"}` and nothing else, since nothing else is
kept. One by name, a withdrawn one and one undecided give none.

`sluice status <branch>` gains an `OWED` section after the queue's,
each reader with its deliveries, `--json` an `owed` list of `{reader,
known, deliveries: [{key, id, state, expires_at, handed}]}`. A reader
no reader has confirmed under is marked `never confirmed`. Nothing is
shown where nothing is owed.

### D14. The board collects new outcomes with `--new`

The board runs `sluice inbox` at its first overview, so a restarted
session is handed again what it never confirmed, and `sluice inbox
--new` on a new outcome of the session's, so a notice the round pushed
is not handed over twice. It starts its turn with what sluice printed,
the confirm line with it, and keeps no list of its own (core.31). A
reload of the mod is a first overview and repeats what is still owed.
The board takes every later `session.start` as such a load, which the
engine fires at a reload, an enable and a respawn, so its next overview
collects with `sluice inbox` whether or not the module's own state was
made afresh, and a test of the board exercises a reload by a second
start. Its `.claude-plugin/sluice.json` names as `from` the first
release with `--new` and `sluice confirm`, so the board says on an older
sluice that it needs that release and collects nothing there.

Not taken: reading a reload from the module's state alone, which a
second start in one module never resets, so no test reaches it.

### D15. The agent is told to confirm

The session note and the using-sluice skill say: run `sluice inbox`
once at the start; act on each outcome; then run `sluice confirm <key>`
for it; an outcome not confirmed is handed over again by the next
inbox.

### D16. A resubmission confirms the outcome it withdraws

`admit` withdraws a worktree's earlier submission that was blamed or in
conflict when the worktree submits again (67). That is the reader
acting on the outcome, so the same write confirms that submission's
delivery for its reader with `weir_delivery::confirm`, the `confirmed`
line's `by` `resubmission`. Without it, every blame an agent fixed and
resubmitted would be handed over again by its next inbox, against
today's "a failure resubmitted is not delivered". A landing is never
withdrawn, so a resubmission confirms none.

Not taken: leaving the blame owed, which repeats what the reader has
already acted on.

## Risks / Trade-offs

- [An agent that never confirms is handed the same outcomes at every
  inbox] → that is the form's point; the notice, the inbox's line and
  the session note each name the command, and an expired one is shown
  in the branch's status for the operator.
- [Flume's settles repeat outcomes until its profile passes
  `--confirm`] → the release waits on Flume's change (D11).
- [`deliveries.jsonl` grows with outcomes nobody confirms] → each is a
  line of a few hundred bytes; an expired one is shown, so the operator
  confirms it, and a done one is let go of.
- [The git dependency's hash in `flake.nix` must move with the
  revision] → `nix:typecheck` fails while they differ.
- [The inbox's document loses `delivered` and `delivered_to`] → nothing
  in the family reads them: Flume's profile reads `notice`, `state`,
  `landed_as`, `failed`, `conflict`, `tested`, `from`, `id` and
  `branch`, and the board reads the text.

## Migration Plan

Lands on `bolt/sluices-outcomes-are-owed-until-confirmed`, then main.
The release is cut once Flume's change is ready to take it (D11). The
move of the records an earlier sluice wrote is D12's agent step, after
the release. Rollback: an earlier sluice ignores `deliveries.jsonl` and
reads every outcome decided meanwhile as pending, since none carries
`delivered`, so its inbox hands each over once more.
