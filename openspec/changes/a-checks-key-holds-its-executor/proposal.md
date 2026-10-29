## Why

On 2026-10-29 Flume's fix branch was run with `sluice exec main` by a
host's sluice 0.56.0, whose sandbox let no command open `/dev/ptmx`. All
29 of `flume:tools`' herdr tests failed with "failed to openpty", and
sluice recorded each fail under the test's key, moon's hash of its
inputs. The queue's run of the same head, under sluice 0.57.1, passed
45 of 45. Each key held a pass and a fail, Flume's main reads that as
failed (`flaky = "fail"`), and submission 45 failed with no way forward
but a set-aside the user will not make or a commit that changes the
tests' inputs.

The gate already weighs a key's results only among runs held to the
same limits (19, `same_limits`), but the two runs' limits were the same:
sandbox, same network, no container engine. What differed was the
Seatbelt profile, which no record carries. And the run never judged its
tests, which record 0031 says leaves them with no outcome, but sluice
had no way to see that the sandbox, not the code, failed them.

The user decided on 2026-10-29 to do both: a check's key holds its
executor's grant, and a check that failed on what the executor refused
has no outcome. Clauses 138 and 139 (blueprints 50a1c94), record 0072
and rules core.34 and core.35 carry the decision.

## What Changes

- **A check's key holds its executor's grant.** Every recorded
  executor carries `grant`: for the sandbox, a digest of the Seatbelt
  profile sluice writes, rendered with every path named by its role
  (`<place>`, `<scratch>`, `<cache>`, the sockets, `<user-tmp>`), so
  places and hosts with one profile give one grant; for the open
  executor, one constant. The gate compares it wherever it compares
  limits, for a flake and for the pass that proves a replay, on signed
  branches as on unsigned ones. sluice's version is not in it, and
  moon's hash is left as it is.
- **Results with no grant.** A line written before this change is read
  as run under an unknown grant, weighed only with other lines with
  none. Every check runs once more under its first grant.
- **Signed runs carry the grant** in the statement's predicate, read
  from the signed statement alone.
- **A check that failed on what the sandbox refused has no outcome.**
  The run's profile marks its denials as the run's; after a run in
  which a check did not pass, sluice reads the unified log for the
  run's denials, once, and keeps what it read. A denial leaves each
  check that did not pass not run, with what was refused as the
  reason; checks that passed keep their passes. A log that cannot be
  read is said with the run, which reads as one with no denial.
- `sluice exec`, a submission's failure and the guides say what the
  sandbox refused and that sluice's profile stopped the run.

## Capabilities

### New Capabilities

- `check-grant`: what a run's executor let it reach, recorded with each
  result and read as part of its key.

### Modified Capabilities

- `run-outcome`: a check that did not pass in a run whose sandbox
  reported a refusal has no result.

## Out of scope

- The Linux sandbox and the container executor, not built; each gives
  its grant and reports its refusals when it is.
- Reading what a test printed or how it failed.
- A host's failures outside the sandbox, which stay record 0031's
  `not_run` and `[tests] ready`.
- A canary run before a check, or a list of what the executor allows.

## Impact

- `crates/sluice-seams/src/lib.rs`: `Enforced::grant`.
- `crates/sluice/src/profiles/checks.rs`: the grant of each executor,
  the profile's mark on its denials, the denials read after a run in
  which a check did not pass, the checks they leave not run.
- `crates/sluice/src/profiles/denials.rs` (new): reading the unified
  log's sandbox reports for a run.
- `crates/sluice-core/src/verdict.rs`: `same_limits` compares the
  grant, and the signed path compares limits and grant.
- `crates/sluice/src/signing/statement.rs`: the grant in the predicate.
- `crates/sluice/src/lib.rs` and the text views: the words for a check
  the sandbox refused.
- Tests: `crates/sluice-core/src/verdict.rs`, `crates/sluice/tests/checks.rs`,
  `crates/sluice/tests/exec.rs`, `crates/sluice/tests/remote_refs.rs`,
  the conformance suite's fake executor.
- Docs: `site/src/content/docs/reference/configuration.md` (`flaky`,
  `[executor]`), `plugins/sluice/skills/using-sluice/references/verdicts.md`
  and `exec.md`.
- Sources: clauses 19, 83, 108, 114, 115, 138 and 139, C.2, S66
  (blueprints 50a1c94); record 0072; rules core.7, core.9, core.22,
  core.34 and core.35.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/verdict.rs
crates/sluice-core/src/records.rs
crates/sluice-fakes/src/**
crates/sluice/src/lib.rs
crates/sluice/src/profiles/checks.rs
crates/sluice/src/profiles/denials.rs
crates/sluice/src/signing/statement.rs
crates/sluice/tests/**
site/src/content/docs/reference/configuration.md
plugins/sluice/skills/using-sluice/references/*.md
```
