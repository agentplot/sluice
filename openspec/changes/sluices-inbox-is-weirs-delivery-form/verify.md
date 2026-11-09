# Verify: sluices-inbox-is-weirs-delivery-form

Verified on `change/sluices-inbox-is-weirs-delivery-form` at `57a9fbe1`,
cut from `bolt/sluices-outcomes-are-owed-until-confirmed` at `4dbe34df`,
on 2026-11-08.

**Verdict: ready to archive.** The bolt's gate passes. Every task is
ticked. Every requirement is built, and each scenario's THEN clause is
asserted by a test.

## The whole checks

`sluice exec bolt/sluices-outcomes-are-owed-until-confirmed` (sluice
0.73.1), the bolt's gate `#check-fast`, run once: **passed, exit 0**.
All 20 tasks passed, and sluice recorded 20 results under their keys for
`57a9fbe`.

| Task | Result |
|---|---|
| `cli:test` | passed: 613 tests |
| `core:test` | passed: 148 tests |
| `conformance:test` | passed: 331 tests |
| `fakes:test`, `seams:test` | passed: 8 and 4 tests |
| `nix:lint`, `nix:test`, `nix:typecheck` | passed |
| `plugin-board:test` | passed: 156 tests |
| `plugin-board:typecheck`, `plugin-board:validate` | passed |
| `plugin-sluice:hooks`, `plugin-sluice:lay-out`, `plugin-sluice:validate` | passed |
| `root:adrs`, `root:changes`, `root:format`, `root:lint` | passed |
| `site:prose` | passed: 0 errors, 0 warnings |
| `site:rules` | passed |

`#check-fast` does not run the `tools` suites, which hold most of this
change's CLI tests. Main's `#check-tools` runs them when the bolt lands
on main. To see each scenario's test pass, the four suites the change
writes were run whole, once:

`sluice exec bolt/sluices-outcomes-are-owed-until-confirmed -- cargo
test -p sluice --features tools --test worktrunk --test peers --test
pull_requests --test conformance -- --skip sandbox`: **passed, exit 0**.
These tests passed: `worktrunk` 77, `peers` 10, `pull_requests` 167 and
`conformance` 59.

## Tasks

All 38 tasks are ticked. Every test a task names exists under its name.
None of the tests that tasks rename or replace remain. `openspec
validate --strict` passes. Every MODIFIED and REMOVED requirement's
header matches its main spec, and no ADDED header clashes with one, so
the archive applies.

## Specs

- **outcome-deliveries.** These hold:
  - One delivery per decided outcome, owed to the worktree or the
    request.
  - None for a submission by name or a withdrawal.
  - A resubmission confirms the outcome it withdraws.
  - The store passes weir's pack.
  - Only a done delivery leaves.
  - The deliveries are on the branch's remote for another host to
    collect (`peers.rs`).
  - A line that does not read stops a branch's status.
  - A host's span of a day.
- **outcome-delivery.** Each hand-over is recorded on its carrier's word
  and confirms nothing. An outcome is pushed once. The log names where
  each hand-over reached. A request's post confirms on receipt. A wait
  hands over its own outcome, confirming only under `--confirm`. `wt
  submit` confirms a landing whose worktree it removes. A wait where no
  files are checked out reads as a worktree on its branch. The REMOVED
  requirements are gone: `Submission` holds no `delivered`,
  `delivered_to`, `told` or `told_to`, and an older record is written
  back without them (`records.rs:1913`).
- **inbox.** Each collect hands over and the outcome stays owed until it
  is confirmed. `--confirm` and `--new` work through the CLI. An expired
  outcome says `Expired at <t>, still owed.` with the confirm line. The
  git directory hands over nothing of a worktree's, and the outcome is
  still owed with no hand-over after its inbox.
- **outcome-confirm.**
  - A first confirmation's `--json` gives `first` true with its
    `branch` and `reader`.
  - Confirming a second time writes nothing.
  - A key not found gives `not_found` naming `sluice inbox`, exit 1.
  - Another reader's key is refused, exit 2.
  - Exit 3 names a branch whose records do not read.
  - Mixed keys exit by the worst code.
  - The operator confirms with `by` `operator`.
- **owed-outcomes.**
  - `sluice status <branch>` shows the `OWED` section and `--json`
    gives `owed`.
  - `sluice status <n> --json` gives a pushed landing's whole
    `delivery`, and only `key` and `done` once the delivery is let go
    of.
  - A queued or withdrawn submission has no `delivery`.
  - The two `cli.rs` status tests take in the `OWED` section.
- **submit-by-name.** No delivery, no `handed` or `confirmed` line, and
  `sluice confirm` finds nothing under the key, both in the branch's
  worktree and as the operator. A request's blame is shown in `main`'s
  `OWED` under `pull/<n>`, handed over by nothing.
- **plugin-board.**
  - Every later `session.start` is a load (D14). The engine fires one at
    a reload, an enable or a respawn, never at `/clear`.
  - A reload's first overview runs `sluice inbox` and starts a turn only
    for what is still owed.
  - The first overview after the repository had no sluice collects
    everything owed, with no toast.
  - An outcome decided while no watch was open is collected by `sluice
    inbox --new`.
  - `from` is `0.79.0`. The latest tag is `v0.78.1`.
- **plugin-usage, submission-wait, host-settings-module.** Held by
  `plugin-sluice:hooks`, the wait tests in `worktrunk.rs`, `nix:test`
  and `instance::tests::the_modules_file_is_read_as_written`.
- **site.** `reference/state.md`'s example notice ends with `sluice
  confirm 13:failed once you have read it.`.

### Seen, not blocking

- Doc comments in `crates/sluice/src/lib.rs` still describe the removed
  `delivered` mark, at 2989, 4453 and 4667-4670 ("marked delivered").
  The code under them hands over and confirms as the specs say.
- `wt submit` decides to pass `--confirm-landing` before the wait, from
  `wt`'s `merge.remove` setting (`wt_submit.rs:45`). If that removal
  then fails and the worktree stays, its landing is confirmed already.
  Record 0102 chose this ("where it will remove the worktree").
- The weir `outputHashes` value in `flake.nix` is evaluated by
  `nix:typecheck`, but nothing builds it until the release.
- The gate's moon logged 21 `Failed to store blobs` warnings, because
  it could not write the shared cache under the repository's
  `moon-home` from inside the sandbox. Caching was skipped, and no task
  failed.

## Archive

Archived with this report.
