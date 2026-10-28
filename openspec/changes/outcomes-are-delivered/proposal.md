# Proposal

## Why

A submission's outcome reaches the session that made it only when that
session waits for it. The worktrunk driver's notify delivers nothing,
yet the round marks every outcome as given to its session (submission
234 on 2026-10-28 reads `told` with nobody told), and a session that
restarts between its submit and its outcome has lost its wait. Records
0068 and 0070 settle that the round which decides an outcome pushes a
notice into the submitter's session through the host's multiplexer,
that sluice alone marks an outcome delivered and only on a carrier's
word, that an outcome not delivered stays pending, and that a session
that starts collects what is pending for it by `sluice inbox` (20, 50,
51, 135, the C.5 worktrunk row; rule core.31).

Flume needs the same command for a place it submitted with no session:
it submits with `sluice submit <base> --no-wait --json`, never waits,
and collects the place's outcomes at each settle by `sluice inbox
--json`, whose document it reads as a contract (flume record 0012; flume
133, flume C.6).

This change folds the three in-flight changes that began the work,
`an-entry-records-its-submitting-session-and-the-round-pushes-its-outcome`,
`a-session-is-told-its-untold-outcomes-by-one-command` and
`the-board-carries-sluices-telling-and-keeps-no-told-list`, into one,
under the words of record 0070, and builds the rest.

## What Changes

- A submission records the session that submitted it, as the host's
  multiplexer names it: under herdr, `herdr:<pane>@<socket>`, with the
  host. A submit run outside any multiplexer records no session.
- The round that decides a landing, a blame or a conflict pushes one
  line of sluice's words, a notice, into the recorded pane, on the host
  that recorded it. herdr taking the line is the delivery. The pull
  request driver's post on the request is the delivery there.
- An outcome is marked delivered only on a delivery: the submission's
  `delivered` (when) and `delivered_to` (to which session). An outcome
  not delivered stays pending, with one `pending` line in the branch's
  log saying why, and no round tries it again.
- `sluice inbox`, run in a worktree, collects every outcome pending for
  its submissions on any branch: it prints each in the notice's line and
  what `sluice wait <id>` prints, then marks each delivered, to the
  multiplexer's name for the session or else the worktree's path. Run
  again, it prints nothing. With `--json`, one document, `{"outcomes":
  [...]}`, each the submission's record with its `notice`, whose shape
  another tool may read and which only gains fields.
- A `sluice wait`, a waiting `sluice submit` or `wt submit` that reports
  the outcome of a submission from its own worktree marks it delivered
  the same way; one that reads an outcome already delivered reports it
  and marks nothing.
- Whichever of the push, `sluice inbox` and a reporting wait comes first
  delivers an outcome; the others deliver nothing for it.
- **BREAKING** for a reader of the state files: the submission's `told`
  and `told_to` are written as `delivered` and `delivered_to`, and the
  log's `told` event as `delivered`. sluice reads the former names, so
  records written by an earlier sluice are read as they were.
- The Claude Code board keeps no list of what it told: it runs `sluice
  inbox` at its first overview and when an overview shows a new outcome
  of the session's, and starts a turn with sluice's words, none when
  sluice prints nothing. Built; renamed here.
- The using-sluice skill and the plugin's session-start note say a
  session starting again runs `sluice inbox` once; the site's command
  and state references say what the command does and what the fields
  mean.
- Code identifiers move to the new words (`Told`, `ToldRecord`,
  `told_outcome`, `Untold` and their kin).
- Every test that starts sluice runs with herdr's variables cleared, so
  no test records or prompts a real pane.

`sluice step <branch>` is the one round a caller runs where no `sluice
run` is live, Flume's settle among them (135; flume C.6). It needs no
change: its round runs the outbox as every round does, and a branch
whose lock is held answers `busy`.

Out of scope:

- Any multiplexer but herdr; a second is a second arm of the worktrunk
  profile, in a change of its own when one is asked for.
- Pushing across hosts: a round on one host does not prompt a pane on
  another, and the outcome stays pending for that session's `sluice
  inbox`.
- Delivering a withdrawal. Flume's statement lists `withdrawn` among
  what its inbox answers (flume 133); sluice 51 and 20 deliver landings,
  blames and conflicts. Flume reads a withdrawal from `sluice status`
  until the two statements agree; that waits on the statement's owners.
- The herdr plugin's list behind its toasts, which are the host's user's
  and deliver nothing (51).
- Flume's own units that call `sluice inbox` and `sluice step`, which are
  Flume's (flume record 0012).

## Capabilities

### New Capabilities

- `outcome-delivery`: a submission's session recorded at submit, its
  outcome pushed into that session as a notice by the round that decides
  it, delivered recorded only on a carrier's word, and a reporting wait
  marking delivered.
- `inbox`: `sluice inbox`, by which a session or a caller's place
  collects every outcome pending for its submissions, each delivered
  once, with a `--json` document another tool reads.

### Modified Capabilities

- `plugin-board`: a session's outcome is delivered in sluice's words,
  collected by `sluice inbox` at the board's first overview and when the
  overview shows one of the session's outcomes, with no list of the
  board's own.

## Impact

- `sluice-seams`: `Submitter` gains `host` and `session`; `Driver::notify`
  answers `Delivery`. Built.
- `sluice-core`: the outbox marks delivered only on `Delivered`; the
  pending outcomes of a session found and marked under each branch's
  lock; `delivered` and `delivered_to` on a submission, read under their
  former names; the log's `delivered` and `pending` events.
- `sluice`: the worktrunk profile's herdr pane and push; the pull request
  driver's delivery; `sluice inbox`; the waits marking delivered; the
  notice's words in `words.rs`.
- The fakes, the conformance suite, the CLI and profile tests, `e2e/e2e.sh`.
- `plugins/sluice-board` (the command's new name), `plugins/sluice`
  (skill and session-start note), the site's references and Claude Code
  guide, the state examples.
- On a branch whose records are on its remote, every host that submits
  to it runs this sluice before any round writes a `delivered` or
  `pending` line (design, Migration Plan).

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/**
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/**
crates/sluice/src/lib.rs
crates/sluice/src/words.rs
crates/sluice/src/profiles/mod.rs
crates/sluice/src/profiles/worktrunk/**
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/tests/**
e2e/e2e.sh
plugins/sluice-board/**
plugins/sluice/skills/using-sluice/**
plugins/sluice/scripts/session-context.sh
plugins/sluice/tests/hooks.sh
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/state.md
site/src/content/docs/guides/claude-code.md
docs/examples/state/**
openspec/changes/**
```
