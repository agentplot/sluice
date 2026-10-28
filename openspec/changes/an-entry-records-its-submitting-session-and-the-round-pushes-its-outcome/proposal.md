# Proposal

## Why

When a submission is decided, sluice marks it told, but under worktrunk
nobody was told: the driver's notify delivers nothing and returns, and
the round records the outcome as told all the same (submission 234 on
2026-10-28 reads `told` with nobody told). A session that submitted
with `--no-wait` learns its outcome only by asking. ADR 0068 settles
that the round which decides an outcome pushes it into the submitter's
pane through the host's multiplexer, and that the record says told only
when it was (clauses 20, 50, 51, the C.5 worktrunk row; rule core.31).

## What Changes

- A submission records the session that submitted it, as the host's
  multiplexer names it: under herdr, the herdr server and the pane the
  submit ran in, with the host. A submit run outside any multiplexer
  records no session.
- The round that decides an outcome pushes it, in one line of sluice's
  words, into the recorded pane as a prompt to the agent there, on the
  host that recorded it. herdr's acceptance of the prompt is the
  delivery.
- The submission is marked told only on a delivery, and the record says
  when and to which session. A notify that delivered nothing marks
  nothing: the outcome stands decided and untold, and the branch's log
  says why it was not delivered.
- An outcome is pushed once. A round that cannot deliver does not try
  again, and nothing is pushed for a submission already told.
- The pull request driver's notify says the same: posted on the request
  is a delivery to that request; a submission it holds no request for is
  not delivered.
- A submission's record gains `told_to`, the session its outcome was
  delivered to, beside `told`, the time. Both are absent on an untold
  outcome. Records written before read unchanged.
- Every test that starts sluice runs with herdr's variables cleared, so
  no test records or pushes into a real pane.

Out of scope, each its own unit of the bolt or a later one:

- The one command a starting session runs to be told its untold
  outcomes, and the Claude Code board carrying it where there is no
  multiplexer (51; ADR 0068). Until it lands, an untold outcome is read
  as today, by `sluice wait`.
- A `sluice wait` that reports an outcome marking it told (51).
- The board giving up its own told list (ADR 0068's consequences). Until
  it does, a Claude Code session in a herdr pane is told its outcome by
  both the push and the board's own wake.
- A wait that follows the host's driver and runs no round (135), and the
  README's sentence on it.

## Capabilities

### New Capabilities

- `outcome-push`: a submission's session recorded at submit, its
  outcome pushed into that session's pane by the round that decides it,
  and told recorded only on a delivery.

### Modified Capabilities

None.

## Impact

- `sluice-seams`: `Submitter` gains the submitting session and its host;
  `Driver::notify` answers whether it delivered, and to which session.
- `sluice-core`: the outbox marks told only on a delivery, skips a
  submission already told, logs an outcome not delivered, and leaves it
  untold; a told submission names when (`told`) and to which session
  (`told_to`).
- `sluice` worktrunk profile: records the herdr pane at submit and
  pushes the outcome through `herdr agent prompt`. herdr is named only
  in this profile (core.1).
- `sluice` pull request profile and the binding in `profiles/mod.rs`.
- The fakes, the conformance suite and the profiles' tests.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/records.rs
crates/sluice-core/src/step.rs
crates/sluice-core/src/ops.rs
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/**
crates/sluice/src/profiles/mod.rs
crates/sluice/src/profiles/worktrunk/**
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/src/lib.rs
crates/sluice/tests/**
crates/sluice/Cargo.toml
Cargo.lock
e2e/e2e.sh
site/src/content/docs/reference/state.md
docs/examples/state/**
```
