# Design

## Context

See proposal.md for why. The probe lives in one profile and is reached
from one core line:

- `crates/sluice/src/profiles/checks.rs`: `READY_LIMIT` (sixty seconds);
  `Ready { argv, shown }`; the run's kept `Meta.ready`; in `start`, after
  the toolchain and before the runner's plan, `command_setting(tests,
  "ready")` run under the executor (its own `ready.sb` sandbox profile
  under `sandbox`) and a `bail!("the host is not ready to run the checks:
  …")` on failure, which is the line the signal names; and in `progress`,
  after a run in which a check did not pass and the limit did not end it,
  `ready_after`, which runs the command once more and, when it fails,
  moves every failed and missing check into `not_run` with "the host
  failed [tests] ready … after the run".
- `crates/sluice-core/src/session.rs`: a session's command run removes
  `ready` (and `not_run`) from the tests it hands the profile.
- `crates/sluice-fakes`: `not_ready` makes the fake refuse a start whose
  tests name `ready`.

A start error is handed back to the round, which releases the place and
tries the combination again next round with nothing recorded; with the
probe failing on every start that is the 20-second loop of the signal.

The statement now reads: clause 84, "sluice runs no readiness command
the repository writes, before a run or after one: a check brings up what
it needs and waits for it within its own limit (66), says by the
policy's not-run exit when it could not set up (114)"; clause 114, a run
has no outcome when the check could not be started, exited as the policy
names not run, its executor refused it something (139), or, after a
check did not pass, the executor finds a requirement the policy asks has
left the host; and rule core.37 rules out "a `ready` key in `[tests]`,
read, run or documented" and `READY_LIMIT`. Record 0066 as amended
2026-10-30 is the decision.

## Goals / Non-Goals

**Goals:**

- No code path runs `[tests] ready`, before or after a run, for any run
  sluice starts: a submission's, a session's `sluice exec`, a preview's.
- Every word of `ready` leaves the code, the docs and the skills, so
  nothing tells a reader the key exists.
- 114's remaining causes behave exactly as before: a program that could
  not start, a `not_run` exit, and (once built) the executor's report.

**Non-Goals:**

- The executor's own reading of a container engine, before claiming and
  after a run (record 0074; rule core.38): its own unit.
- Reading the sandbox's refusals (139): the change
  `a-checks-key-holds-its-executor`.
- What a round does with any other start error.
- Changing sluice's own stack test to exit with a `not_run` code: the
  engine it needs is record 0074's requirement, read by the executor.

## Decisions

**D1. A policy that names `ready` is read as if it did not.** `[tests]`
is the check profile's table, and the profile already ignores keys it
does not use; `ready` becomes one of those. Nothing reads the key, so
nothing can run it, and core.37's "read, run or documented" holds.

- *Refuse a policy naming it, as `[checkout] cache` is refused (ADR
  0049).* Not taken: the sources say the key leaves the policy and the
  code and rule out reading it, and they name no refusal. A refusal would
  also stop every branch whose revision still carries `ready` (sluice's
  own bolts cut before this lands among them) until its policy is edited,
  where ignoring it only runs tests that the probe used to hold back. If
  the design asks for a refusal, it is one `bail!` in `Policy::parse`
  beside the `[checkout] cache` one, and its own change.
- *Warn on it.* Not taken: no warning path for a policy key exists, and
  a warning is documentation of the key in the output.

**D2. Remove, don't disable.** `READY_LIMIT`, `Ready`, `Meta.ready`,
`ready_after`, the pre-run block in `start` and the after-run block in
`progress` go whole, with the module doc's `ready` paragraph, and
`run_to_end` in `runs.rs`, which was written for `ready` and has no
other caller. The
after-run block's `unpassed` reading stays only if something else reads
it (the change `a-checks-key-holds-its-executor` will); otherwise it goes
with the block.

**D3. A run in flight across the upgrade.** `Meta` is read with serde's
defaults and no `deny_unknown_fields`, so a run started by an older
sluice whose `meta.json` holds `ready` is read by the new one with the
field ignored: it gets no after-run ask, which is the new behaviour. A
kept `ready-after` file in such a run's directory is never read again
and goes with the run's directory. No migration.

**D4. The session's command run.** `session.rs` stops removing `ready`
from a command run's tests, since nothing reads it; it keeps removing
`not_run`, whose reason (a command's exit is its own) still holds.

**D5. Tests say what is gone, not what was.** In
`crates/sluice/tests/checks.rs`, `a_host_that_is_not_ready_runs_nothing`
and the after-run test are replaced by one test per run-outcome
scenario: a policy with `ready = "false"` whose passing test passes, one
whose `ready` would fail after a fail records the fail under its key with
nothing not run, and one whose `ready = "touch asked"` leaves no `asked`
behind. In the conformance suite, the fake's `not_ready` and the tests
that use it go; `a_command_run_is_held_to_neither_readiness_nor_not_run`
keeps its `not_run` half; and one test submits under a policy naming
`ready` on a fake that would have refused it and sees one start and a
landing. Sample reasons in `verdict_view.rs` and `github/render.rs`
become a `not_run` exit's words.

**D6. Docs.** The site's configuration table loses the `ready` row and
the `not_run` row's "as with `ready`"; the commands page lists three
causes of "could not run" and four things sluice tells apart without a
set-aside (no ready); the pull requests guide's sample row names a
`not_run` exit. The using skill's exec and verdicts references and the
setting-up skill's table and moon example say a test that needs docker
starts what it needs, waits for it, and exits with a `not_run` code when
it could not, with `[executor] docker = true` and a `[tests] release`.
sluice's own `.config/sluice.toml` loses `ready` and its comment. The
`_open.md` item for core.37 goes. Site pages pass Vale (`site:prose`).

## Risks / Trade-offs

- [A repository that leaned on `ready` to keep a test off a host without
  a container engine now records that test's fail and blames the
  submission] → record 0074's unit has the executor refuse such a run
  before claiming it; until it lands, the test can exit with a `not_run`
  code, and a wrongly blamed fail is set aside (116). The docs say so.
- [The change `a-checks-key-holds-its-executor` is written against
  `ready_after` (its task 2.3 reads the denials "after `ready_after` and
  only when it moved nothing")] → whichever lands second reads the
  denials where the after-run probe was, under the same `unpassed` and
  not-timed-out condition; this change keeps that condition's meaning in
  the code it leaves.
- [Other start errors still loop as the probe did] → named out of scope
  in the proposal for the planner.

## Migration Plan

Lands with sluice's own policy edited in the same change, so the gate
that reads the branch's policy after the landing (56) reads no `ready`.
A host on an older sluice reading the new policy runs its tests without
the probe, as the new sluice does. Rollback is reverting the change; the
policy line comes back with it.
