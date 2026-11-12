## Why

On 2026-11-08 sluice blamed Flume's submission 165 (pull request #122)
for `site:install`, and the runner's copy then withdrew its own entry
for the same request (signal
2026-11-08-agentplot-ops-f8f408fa/01-sluice-blames-a-dns-failure-on-the-submission).
Run c146's log of `site:install` holds 297 lines of `DNSResolveFailed
downloading tarball`. The change did not fail them: Flume's `main` runs
its checks under the sandbox with `[executor] network = "localhost"`,
whose Seatbelt profile refuses every name lookup, and the install
reached for npm's registry. sluice had no way to tell that the sandbox,
not the code, failed the check, so it recorded the fail under the
check's key and blamed the newest entry.

The same gap failed Flume's submission 45 on 2026-10-29, the incident
behind record 0072. A host's older sluice refused `/dev/ptmx`, 29 tests
failed on "failed to openpty", and their fails were weighed against the
queue's passes under a profile that allowed it. The two runs' limits
were the same, so `same_limits` read them as one key, though their
Seatbelt profiles differed and no record carried the difference.

The user decided on 2026-10-29 to do both: a check's key holds its
executor's grant, and a check that failed on what its executor refused
has no outcome. Clauses 138 and 139, record 0072 and rules core.34 and
core.35 carry the decision. Nothing of it is built: `Enforced` has no
grant, `verdict::same_limits` compares none, and no executor reports
what it refused.

## What Changes

- **A check's key holds its executor's grant.** Every run records its
  executor's grant with what it enforced. The sandbox's is a digest of
  the Seatbelt profile sluice writes, with every path named by its
  role, so places and hosts given one profile give one grant. The open
  executor's is one constant. sluice's version is no part of it, and
  moon's hash is left as it is (138).
- **The gate compares it.** A key's results are weighed for a flake,
  and a replay is proven, only among results under the run's grant, on
  a signed branch as on an unsigned one. A result recorded with no grant
  is weighed only with others recorded with none, so each check runs
  once more under its first grant (19, 108, 138; rules core.9,
  core.34).
- **A signed run carries its limits and grant** in its statement's
  predicate, read from the signed statement alone, so a signed branch
  weighs a key under the run's limits as a host's own branch does (19,
  core.22).
- **The sandbox's reports are kept as the run goes.** Each refusal rule
  of the run's profile carries the run's own mark. The run's supervisor
  streams the kernel's sandbox reports of that mark into the run's
  directory from before the command starts until it has ended. On the
  macOS probed, the persisted log keeps none of them (record 0072 as
  amended 2026-11-11).
- **A check that did not pass in a run the sandbox refused something
  has no outcome.** After such a run, and only then, sluice reads what
  the sandbox refused it. Each check that failed, was failed by a task
  it depends on, or was left without a result is not run, its reason
  naming what was refused. Nothing is recorded under its key, nobody is
  blamed, and a check that passed keeps its pass (114, 139, 149; rule
  core.35).
- **A report that cannot be read is said with the run**, which is read
  as one with no refusal.
- `sluice exec`, a submission's status and the guides name what the
  sandbox refused and say that sluice's sandbox, not the test, stopped
  it.

## Capabilities

### New Capabilities

- `check-grant`: what a run's executor let it reach, recorded with each
  result and read as part of its key.

### Modified Capabilities

- `run-outcome`: a check that did not pass in a run the sandbox refused
  something has no result, and the sandbox's own report is how sluice
  knows.

## Out of scope

- The Linux sandbox and the container executor, which are not built.
  Each gives its grant and reports its refusals when it is (C.2).
- Reading what a check printed, its exit or how it failed (38; rule
  core.35).
- A run's own command, a session's or a preview's, whose exit is its
  own and decides nothing (record 0074, design D6). Its supervisor
  streams the reports, and nothing reads them.
- Changing a repository's checks or policy so they need nothing the
  sandbox refuses. Flume's `site:install` reaching the network under
  `network = "localhost"` is Flume's to settle; this change makes sluice
  say so instead of blaming the change.
- The signed record as in-toto's Test Result (record 0077), whose
  change carries the grant in its annotations when it lands.

## Impact

- `crates/sluice-seams/src/lib.rs`: `Enforced::grant`, `Refusals`,
  `Tests::refused`, `RunReport::refusals_unread`.
- `crates/sluice/src/profiles/checks.rs`: the profile rendered by role
  and its grant; the run's mark on each refusal rule; the watch on a
  sandboxed run; `Tests::refused`.
- `crates/sluice/src/profiles/refusals.rs` (new): the kernel's reports
  of a run's mark read into what the sandbox refused, and its words.
- `crates/sluice/src/runs.rs`: a process the request asks to watch
  beside the command, ready before it and ended after it.
- `crates/sluice-core/src/verdict.rs`: the grant in `same_limits`, which
  the signed path compares too, and the why of a replay another grant's
  pass does not prove.
- `crates/sluice-core/src/step.rs`: `after_run` reads the refusals, and
  `finish_run` logs a report that could not be read.
- `crates/sluice-core/src/records.rs`: `Event::Detail::refusals_unread`.
- `crates/sluice/src/signing/statement.rs` and
  `crates/sluice/src/profiles/remote_refs.rs`: the limits and grant in a
  signed run's predicate.
- `crates/sluice/src/words.rs`, `crates/sluice/src/exec.rs`,
  `crates/sluice/src/lib.rs`: the words for a report that could not be
  read.
- `crates/sluice-fakes/src/lib.rs`, `crates/sluice-core/src/ops.rs`,
  `crates/sluice/tests/profiles/conformance.rs`: each `Tests` answers
  the grant and the refusals.
- Docs: `site/src/content/docs/reference/configuration.md` (`[executor]`,
  `flaky`), `site/src/content/docs/reference/state.md`,
  `plugins/sluice/skills/using-sluice/references/verdicts.md` and
  `exec.md`.
- Sources: clauses 19, 38, 83, 108, 114, 115, 138, 139 and 149, C.2's
  sandbox, open and moon rows, S66; records 0072 (as amended
  2026-11-11), 0074, 0084, 0097 and 0105; rules core.7, core.9,
  core.22, core.34 and core.35; the signal above.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/verdict.rs
crates/sluice-core/src/records.rs
crates/sluice-core/src/step.rs
crates/sluice-core/src/ops.rs
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/tests/**
crates/sluice/src/runs.rs
crates/sluice/src/exec.rs
crates/sluice/src/lib.rs
crates/sluice/src/words.rs
crates/sluice/src/profiles/checks.rs
crates/sluice/src/profiles/refusals.rs
crates/sluice/src/profiles/mod.rs
crates/sluice/src/profiles/remote_refs.rs
crates/sluice/src/signing/statement.rs
crates/sluice/tests/**
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/state.md
plugins/sluice/skills/using-sluice/references/*.md
```
