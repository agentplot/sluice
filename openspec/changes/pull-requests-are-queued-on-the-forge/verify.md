# Verify: pull-requests-are-queued-on-the-forge

Verified on 2026-11-07 at `25629a7a`, on
`change/pull-requests-are-queued-on-the-forge`, cut from
`bolt/sluice-runs-one-host-process` (`68d68a9a`).

**Outcome: not ready.** The bolt's gate fails, and one requirement
would be lost at archive. Nothing is archived; the change goes back to
its code stage.

## The whole checks

`sluice exec bolt/sluice-runs-one-host-process` (sluice 0.73.1) ran the
bolt's gate, `#check-fast`, once. It exited 1: 12 tasks passed and 1
failed. "recorded 13 results under their keys for 25629a7".

| Task | Outcome |
|---|---|
| `site:rules`, `site:prose`, `plugin-sluice:validate`, `root:adrs` | passed |
| `root:changes`, `root:specs` | passed |
| `root:format`, `root:lint` | passed |
| `seams:test`, `core:test` (152), `fakes:test`, `conformance:test` | passed |
| `cli:test` | **failed** |

`cli:test` is `cargo test -p sluice -- --skip sandbox`. The library's
unit tests passed (228, `a_check_per_required_check_per_system` among
them), as did `cli.rs` (40) and `conformance` (218). The `forge` binary
failed with 36 passed and 1 failed:

```
---- the_pools_calls stdout ----
thread 'the_pools_calls' panicked at crates/sluice/tests/profiles/forge.rs:977:46:
called `Result::unwrap()` on an `Err` value: GET /repos/o/r: 404 Not Found
```

**Cause, by reading.** Since `1b0f3b5b`, `Actions::wake` asks
`GET /repos/<owner>/<repo>` for the default branch when the pool names
no `ref` (`round_ref`, `crates/sluice/src/profiles/github/actions.rs:50`).
`the_pools_calls` wakes with a `kind` that names only `workflow`
(`forge.rs:942`, `pool.wake(&kind, "main", ...)` at `:977`), and its
fake does not answer `/repos/o/r`. Task 3.4's check names only
`--test pull_requests`, so this test was never run on the fix. The test
needs its fake to answer the repository with a default branch, and its
wake's `ref` checked, or the pool given a `ref`.

Because cargo stopped at `forge`, the test binaries after it did not run
in this gate: `pull_requests`, `pushed_refs`, `telemetry_logs`,
`reader_tick`, `peers`, `moon`, `places`, `worktrunk` and the rest.
Nothing here confirms that the tests tasks 3.x, 4.1, 5.x, 6.x, 7.1 and
8.x name pass under the gate.

## Tasks

All 31 tasks, 1.1 to 9.3, are ticked. The integration runs they name
are not confirmed by this gate run (see above).

## A requirement that falls short

1. **"A pull request taken again waits to be taken in" is a removal**
   (`specs/pull-request-submit/spec.md`). The requirement, added by task
   1.3 for record 0096's fix, sits after the `## REMOVED Requirements`
   heading, below the two removals. openspec parses it as REMOVED:
   `openspec show --json --deltas-only` lists it with `operation`
   `REMOVED`. `openspec validate --strict` passes all the same. An
   archive of a scratch copy succeeds, but
   `openspec/specs/pull-request-submit/spec.md` then holds no
   requirement of that name. It belongs under `## ADDED Requirements`.
   The code meets it by reading (`taken_again`, `pulls.rs:1759` and
   `:3524`), and `a_submit_marks_ready_a_request_the_round_job_sent_back_not_a_draft`
   (`pull_requests.rs:6088`) asserts its exit 0 and its words, but that
   binary did not run in this gate.

## Weaker evidence and smaller gaps

- **A create whose answer was not read lists the open pull requests.**
  `App::open`'s settle path (`rest.rs:661`) calls `self.pulls(repo,
  base)`, the branch's list, to find whether GitHub made the request.
  request-intake-host says a submit "SHALL ask GitHub for no branch's
  open pull requests". `Forge::pull_from` would settle it by its head.
  This path runs only after a POST whose answer was lost (core.29).
- **`sluice init actions` on a differing workflow** exits 1 but still
  adds `[signers.forge]` and `[pools.forge]` where absent. It now names
  only what it added.
- **The release binary.** `6c087f23` builds each with the pinned Rust,
  Linux's against musl. Neither the workflow nor the binary on a runner
  with no Nix was run here.
- **Asserted by no test:**
  - the 50-minute bound and the `*/5` schedule;
  - the `0600` key file (the template does it with `umask 077` and
    `chmod 600`);
  - a lead's pull requests taken in after a round job takes it from an
    older host;
  - a host claiming after the lease has passed.

## Scenarios met

These scenarios have code and a test that asserts their THEN clauses, by
reading. The tests outside the library, `cli.rs` and `forge` did not run
in this gate.

- **forge-rounds:** Every host off; It ends while a run goes on
  elsewhere; A branch the forge does not keep; The schedule wakes each
  branch; A branch its own policy names (`held_branches`,
  `the_schedule_reads_each_branchs_own_policy`); A submit wakes it; A
  default branch that is not main
  (`a_wake_starts_the_round_job_on_the_default_branch`); A recorded result
  wakes it; A lost wake; A job after a job; A repository with signers and
  no pool; A workflow already there; The signer and the pool (with the
  default branch as `ref`); No signers; The template's environment; A
  run waits for a host.
- **forge-identity:** No round job on the branch; The app may not
  dispatch it; An environment any branch may use; Another branch the
  forge keeps. Each passed in `forge` in this gate.
- **request-intake-host:** A host's step; A wait and Submitted here,
  found by head through `Forge::pull_from`
  (`GET /pulls?state=open&head=<owner>:<branch>&base=<b>`), the
  double filtering by `head` and the tests asserting no list; Landed by,
  Blamed by, Merged, Held, then marked ready, and the check-reading
  scenarios.
- **pull-request-submit:** No wait, in words; No wait, in one document;
  Sent back by the round job; Sent back as a draft; Sent back, not a
  draft; Still queued; Taken again, its check still failed (see above
  for where its requirement sits).
- **worktrunk-submit:** Run again before the round job takes it in
  again, by `taken_again`.
- **pull-request-intake, pull-request-status, group-submit, overview,
  submission-wait, obsolete-combinations, reader-tick, peer-lead:** each
  of their scenarios.

`a_check_per_required_check_per_system` holds record 0101's new-run
posting (task 6.4) and passed in the library's tests. `Sluice::drop_obsolete`
is gone, and the doc comments the earlier pass named no longer describe
the intake mark or a hand-over as live.
