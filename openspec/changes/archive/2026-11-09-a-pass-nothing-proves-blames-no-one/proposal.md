## Why

On a branch whose records are on its remote, a check's pass counts only
where the task runner's own record proves the key ran: for moon, its
manifest of the key, read where moon keeps it (C.6, core.9). When a run
keeps no such record, sluice today reads every pass of it as a fail
under its key and blames the submission. The cause is said only in a
tracing warning that no reader of the queue sees.

That happened on 2026-11-09: submission 1 on
`bolt/sluice-runs-one-host-process`, run c11. Every one of its 28 tasks
passed. The sandbox refused moon's write to its shared store, so moon
kept no manifest, and every check was blamed on the submission. The
work was blamed for what the host did.

The statement (114 and the C.2 sandbox row, as amended at
agentplot/blueprints 0c38f24), record 0105 point 2 (accepted) and rules
core.9 and core.35 already say what holds: a check that passed in a run
that kept nothing proving it ran has no outcome. Nobody is blamed, the
check runs again within 115's bound, and why is read from the missing
proof itself. This change builds it.

## What Changes

- **The check profile says what a run kept no proof of.** Where a ran
  check passed and the manifest its key needs is missing where the
  profile reads it, the check is neither proven nor failed. It has no
  outcome, and its reason comes from that fact:
  `it passed, but nothing the run kept proves it ran: moon's manifest of
  it is not in <store>`. A manifest the tree does not bear out is still
  no pass, as before.
- **Such a check has no outcome (114).** Nothing is recorded under its
  key and nobody is blamed. The combination is tested again, and the
  run counts toward the runs in a row whose checks could not run, so
  the third such run blames with why (115). A check that failed on its
  own in the same run fails the combination as before, and the
  unproven pass is named as could not run.
- **The branch's log keeps the cause once per cause and host.** A
  `proof-missing` line names the host and why, once while the cause
  stands. When that host's next run proves its passes, a `proof-kept`
  line ends the cause, saying since when, for how many seconds and over
  how many runs. `branch.json` keeps the standing causes as
  `proof_missing`.
- **Status and wait say why a combination is tested again.** While a
  combination is tested again after a run that had no outcome,
  `sluice status <branch>`, `sluice status <n>` and `sluice wait` show
  why on its testing line. `sluice status <n> --json` and
  `sluice wait --json` carry the same under `lost`. This holds for every
  cause of a run with no outcome (114), not only a missing proof.
- The configuration reference and the state reference say so.

## Capabilities

### New Capabilities

- `pass-proof`: a pass that a run on a branch whose records are on its
  remote kept no proof of has no outcome. Why is read from the missing
  proof itself, it runs again within 115's bound, and the branch's log
  keeps its cause once per cause and host, with its end.

### Modified Capabilities

- `run-outcome`: a combination tested again after a run with no outcome
  says why in `sluice status` and `sluice wait`, in text and in JSON.

## Out of scope

- **Letting a sandboxed run write the store its check profile reads.**
  That is record 0105 point 1, built by the change
  `a-sandboxed-run-writes-its-runners-store`. The two changes are
  independent, and either may land first. Once both have landed, a
  missing proof is rare. This change makes sure it blames nobody when
  it happens.
- **A check that failed with no proof.** 114 gives no outcome only to a
  pass. A sandboxed run's fail reads the sandbox's own report of what it
  refused (139), which names a refused write to the store. Any other
  fail with no proof is read as before.
- **A replay, and a manifest the tree does not bear out.** A replay is
  proven by a signed pass for its key (108), never by the run's
  manifest. A manifest that contradicts the tree is a key the run lied
  about, and it is still no pass (C.6).
- **A branch kept in one host's own files, and a session's run.**
  Neither reads a proof: a one-host branch counts the runs its own
  sluice starts (78), and `sluice exec` signs nothing.
- **A launched job's proof.** A job on a pool proves its runs by its own
  signed record (C.2 launched row).
- **Showing a standing cause above a branch's queue.** The log and
  `branch.json` keep it, and each combination it touches says why on its
  own line. A branch-level line waits for a reader who needs one.

## Impact

- `crates/sluice-seams/src/lib.rs`: `Proof`, which `Tests::proven` now
  answers with the tasks it proved and the passes it kept no proof of,
  each with why.
- `crates/sluice/src/profiles/moon.rs`: `NoManifest`, the error
  `check_key` gives where a manifest the key needs is missing.
- `crates/sluice/src/profiles/checks.rs`: `proven` puts a ran pass whose
  manifest is missing among the unkept, naming the store.
- `crates/sluice-core/src/records.rs`: `SystemRun::lost`, `LostRun`,
  `BranchDoc::proof_missing`, `ProofMissing`, and the events
  `proof-missing` and `proof-kept`.
- `crates/sluice-core/src/proof.rs` (new): the standing causes, noted
  once per cause and host and ended with how long.
- `crates/sluice-core/src/step.rs`: `finish_run` reads an unkept pass as
  not run and notes its cause. `read_not_run` keeps on the run what its
  last run left not run, and on which host.
- `crates/sluice-core/src/verdict.rs`: its doc only.
- `crates/sluice-core/src/overview.rs`: `CandidateView::lost`.
- `crates/sluice/src/progress.rs`, `crates/sluice/src/lib.rs`: the
  testing line and the JSON documents say why.
- `crates/sluice-core/src/ops.rs` (its test double),
  `crates/sluice-fakes/src/lib.rs`,
  `crates/sluice/tests/profiles/conformance.rs` (its wrapper).
- Tests: `crates/sluice/tests/profiles/moon.rs`,
  `crates/sluice-conformance/tests/unkept_proof.rs` (new),
  `crates/sluice/tests/cli.rs`.
- Docs: `site/src/content/docs/reference/configuration.md`,
  `site/src/content/docs/reference/state.md`.
- Sources: clauses 15, 16, 20, 66, 108, 114 (as amended at
  agentplot/blueprints 0c38f24), 115, 133, 139, C.2 (its moon and
  sandbox rows, as amended at 0c38f24), C.6; record 0105 (accepted
  2026-11-09); rules core.1, core.7, core.9, core.11, core.14, core.22,
  core.26, core.28, core.35.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice/src/profiles/moon.rs
crates/sluice/src/profiles/checks.rs
crates/sluice-core/src/records.rs
crates/sluice-core/src/proof.rs
crates/sluice-core/src/lib.rs
crates/sluice-core/src/step.rs
crates/sluice-core/src/verdict.rs
crates/sluice-core/src/overview.rs
crates/sluice-core/src/ops.rs
crates/sluice-fakes/src/lib.rs
crates/sluice/src/progress.rs
crates/sluice/src/lib.rs
crates/sluice/tests/profiles/conformance.rs
crates/sluice/tests/profiles/moon.rs
crates/sluice-conformance/tests/unkept_proof.rs
crates/sluice/tests/cli.rs
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/state.md
openspec/changes/a-pass-nothing-proves-blames-no-one/**
```
