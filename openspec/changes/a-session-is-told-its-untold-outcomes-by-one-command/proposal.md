# Proposal

## Why

A session that restarts between its submit and the outcome has lost its
wait, and an outcome nobody could push into its pane (a submit run
outside any multiplexer, a pane gone, a round on another host) stands
in the records decided and untold. Today such a session learns its
outcome only by asking `sluice wait` or `sluice status` about each
submission it remembers. ADR 0068 and clause 51 settle that a session
that starts asks sluice by one command for every untold outcome of the
submissions it made, is told them, and has each marked told, so no wait
need stand for an outcome to arrive (51, 135; rule core.31).

## What Changes

- A new command, `sluice untold`, run in a worktree, tells the session
  working there every outcome of its submissions not yet told: each one
  landed, blamed or conflicting, on any branch, whose recorded submitter
  is this worktree's work (its branch, or the branch a pull request was
  opened from).
- Each outcome is told in the words `sluice wait <id>` reports for it,
  opened by the one line the round pushes into a pane, so a session is
  told the same way however the outcome reaches it. Under `--json`, one
  document listing each submission's record as `sluice wait <id>
  --json` prints it.
- Each outcome told is marked told on its submission, with when and to
  which session: the multiplexer's name for the asking session where it
  runs in one, otherwise the worktree's path. The log says it was told,
  under the same notification the round's push would use.
- An outcome already told is answered with nothing: run again, or after
  the round's push delivered it, the command prints nothing and exits 0
  (`{"outcomes":[]}` under `--json`). The round pushes nothing for an
  outcome the command told first.
- The command takes the branch's lock while it tells and marks, so the
  round's push and the command never both deliver one outcome; one
  stopped between telling and marking tells it again, under the same
  notification.
- A branch whose records cannot be read does not stop the others: the
  command tells what it could read, names the branch it could not with
  the reader's words, and exits 3.
- The command is named in the using-sluice skill and the commands
  reference, and the state reference says `told` is also set by it.

Out of scope, each its own unit of this bolt or a later one:

- The round recording the submitting session, pushing into its pane and
  marking told only on a delivery, which this change builds on
  (`an-entry-records-its-submitting-session-and-the-round-pushes-its-outcome`).
- A `sluice wait` that reports an outcome marking it told (51). Until it
  lands, an outcome a wait reported is still told by `sluice untold`.
- The Claude Code board giving up its own told list and carrying sluice's
  telling by this command where there is no multiplexer, and anything
  that runs the command at a session's start (the plugin's session-start
  hook, crew); ADR 0068's consequences.
- A withdrawal told to the session: the round tells landings, blames and
  conflicts, and so does this command.

## Capabilities

### New Capabilities

- `untold-outcomes`: a session asking by one command for every untold
  outcome of its own submissions, told them in a wait's words, and each
  marked told to it, once.

### Modified Capabilities

None.

## Impact

- `sluice-core`: an operation that finds a session's untold outcomes
  across branches, from the live records and the finished records of the
  session's own submissions alone (131), and marks them told under each
  branch's lock; a finished submission's record rewritten with `told`
  and `told_to` in the branch's write.
- `sluice`: the `untold` command, its text and JSON, and the push's
  one-line wording kept outside the worktrunk profile so the command
  shares it (core.1).
- The conformance suite, the records profiles' tests, the CLI tests.
- The using-sluice skill, `reference/commands.md`, `reference/state.md`.

## Touches

```
crates/sluice-core/src/lib.rs
crates/sluice-core/src/ops.rs
crates/sluice-core/src/line.rs
crates/sluice-core/src/records.rs
crates/sluice-conformance/**
crates/sluice/src/lib.rs
crates/sluice/src/words.rs
crates/sluice/src/profiles/worktrunk/**
crates/sluice/tests/**
plugins/sluice/skills/using-sluice/**
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/state.md
```
