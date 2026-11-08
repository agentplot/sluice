# Verify: pull-requests-are-queued-on-the-forge

Verified on 2026-11-07 at `437f501a`, on
`change/pull-requests-are-queued-on-the-forge`, cut from
`bolt/sluice-runs-one-host-process` (`68d68a9a`).

**Outcome: not ready.** The bolt's gate fails, and the work falls short
of three requirements. Nothing is archived; the change goes back to
its code stage.

## The whole checks

`sluice exec bolt/sluice-runs-one-host-process` (sluice 0.73.1) ran the
bolt's gate, `#check-fast`, once. It exited 1: 12 tasks passed and 1
failed. "recorded 13 results under their keys for 437f501".

| Task | Outcome |
|---|---|
| `site:rules`, `site:prose`, `plugin-sluice:validate`, `root:adrs` | passed |
| `root:changes` (9 items), `root:specs` (67 items) | passed |
| `root:format`, `root:lint` | passed |
| `seams:test`, `core:test` (152), `fakes:test`, `conformance:test` | passed |
| `cli:test` | **failed** |

`cli:test` is `cargo test -p sluice -- --skip sandbox`. Its first
binary, the library's unit tests, failed with 227 passed and 1 failed:

```
---- profiles::github::pulls::tests::a_check_per_required_check_per_system stdout ----
thread '...' panicked at crates/sluice/src/profiles/github/pulls.rs:3759:9:
assertion `left == right` failed
  left: 5
  right: 3
```

Because cargo stopped at that binary, none of `cli:test`'s integration
test binaries ran in this run: `cli.rs`, `pull_requests`, `forge`,
`pushed_refs`, `telemetry_logs`. Nothing here confirms that the tests
the tasks name pass under the gate.

**Cause, by reading.** The test's body is the same as on the base.
Commit `67ccecab` changed `post_check`
(`crates/sluice/src/profiles/github/pulls.rs:1602`): a per-check run
that was completed and is now undecided is posted as a new check run,
not patched. So the test's "formed again" posting leaves five
per-check runs on the head where it expects three, each patched to
`formed again` and queued. Record 0096 weighed that new-run reading as
its option 4 and did not take it ("wants a design of its own"). This
change's design and records do not decide it. The code stage must
either keep record 0096's patch, or write the departure into a record
and update the test.

## Tasks

All 20 tasks, 1.1 to 9.2, are ticked. Task 4.1, and the integration
runs that 3.x, 5.x, 6.x, 7.1 and 8.x name, are not confirmed by this
gate run (see above).

## Requirements that fall short

1. **A host's wait and submit ask GitHub for the branch's open pull
   requests** (request-intake-host, "No host takes a branch's pull
   requests in": scenarios "A wait" and "Submitted here"). The spec
   says a host's wait and submit "SHALL ask GitHub for no branch's open
   pull requests". But `Pulls::find` (`pulls.rs:2915`) lists
   `GET /pulls?state=open&base=<branch>` for `sluice wait <branch>`,
   `--status` and a waiting submit, and `opening` (`pulls.rs:3273`)
   lists them at submit. The tests allow it:
   `assert_eq!(g.lists(), lists + 1, "its finding of its request alone")`
   in `a_wait_takes_nothing_in` (`pull_requests.rs:6631`) and at
   `pull_requests.rs:4909`. `submitted_here_is_followed`
   (`pull_requests.rs:4930`) takes its count after the submit. The
   design gives no exception. Either the code finds its request without
   the list (the number kept in `forge/follows/`, or `?head=`), or the
   spec gets an exception for a host finding its own request.
2. **A sent-back request a person opened answers with its old failure
   after a submit takes it again** (pull-request-submit, "A submit that
   does not wait answers with what sluice read": "Either answer SHALL
   exit 0"; worktrunk-submit, "Run again before the round job takes it
   in again"). For a request the round job blamed and left out of
   draft, `as_its_check_says` (`pulls.rs:2975`) reads its `failed:`
   check as `Told::SentBack` whatever the draft mark says. So after
   `sluice submit main --no-wait` marks it a draft and then ready,
   `submit_pull` returns `across(...)` (`lib.rs:4320`). That prints the
   earlier failure and exits 1, not 0 with `waiting to be taken in`. A
   later `--status` reads it `failed` too, so `wt submit` run again
   marks it again. `a_submit_marks_ready_a_request_the_round_job_sent_back_not_a_draft`
   (`pull_requests.rs:6066`) never checks the submit's exit or state.
   Found by reading, not run.
3. **Wakes go to the ref `main` whatever the default branch is**
   (forge-rounds, "The round job is woken when work arrives on a host",
   and "sluice job round with no branch wakes each branch the forge
   keeps"). `Actions::wake` falls back to `"main"` when the pool names
   no `ref` (`actions.rs:129`), and `sluice init actions` writes
   `[pools.forge]` with no `ref` (`init.rs`). On a repository whose
   default branch is not `main`, every submit's, result's and schedule's
   dispatch names a ref the workflow is not on. The GitHub double does
   not check the ref.

## Weaker evidence and smaller gaps

- **`sluice forge check` for a branch that is not the default**
  (forge-identity). The spec says the environment "admits only that
  branch", and `check::round_job` (`check.rs:162`) reads the workflow
  at `?ref=<branch>`. D4 says the environment admits the default
  branch, which the workflow runs on. For `release/1` the check fails,
  and its advice would cut the default branch off from the environment.
  The spec and the design disagree, so the design must settle it.
- **The release binary on the job's runner.** The template downloads
  `sluice-x86_64-linux.tar.gz` and runs it on `ubuntu-latest` with no
  Nix. That asset is `nix build .#default`, a `makeRustPlatform`
  `buildRustPackage` (`flake.nix`, `ci.yml` `binaries`), so its loader
  is probably in `/nix/store` and it will not start there. Not tested
  and not run here.
- **The no-branch job reads the default branch's policy first**
  (`Pulls::branches`, `pulls.rs`, then `job_wakes` in `lib.rs`). A
  branch whose own revision takes pull requests is missed where the
  default branch's policy does not route it. The spec says "reading
  each branch's policy at its revision".
- **When the job ends.** `job_round` ends once a round leaves the
  records unchanged (`lib.rs:976`), including while a run is open and
  no one has claimed it. The spec says "waits on a run another instance
  claimed". D2 and the "Every host off" test rely on the job ending
  there, so the spec's word should be "claims".
- **`sluice init actions` on a differing workflow** exits 1 but still
  adds `[signers.forge]` and `[pools.forge]`. It also says both were
  added when only one was.
- **A failed wake after a recorded result** is a `tracing::warn!`
  (`step.rs`), not the user-facing warning a submit gives, and no test
  makes it fail.
- **Asserted by no test:**
  - the 50-minute bound and the `*/5` schedule;
  - the `0600` key file (the template does it with `umask 077` and
    `chmod 600`);
  - `[signers.forge]`'s `ref` and `workflow`;
  - a lead's pull requests taken in after a round job takes it from an
    older host;
  - a host claiming after the lease has passed;
  - the three new `forge check` scenarios through the CLI (the tests
    call `check::round_job` directly);
  - `taken_in_by` absent from any document (it holds by grep alone);
  - `still waiting: sluice wait <branch>` in words;
  - a group's `--json` `queue` against a posted check;
  - "Still queued"'s check title;
  - "The queue ahead moves"'s exact title in the integration test.
- **Dead code and stale words:**
  - `Sluice::drop_obsolete` (`crates/sluice-core/src/step.rs:1276`) has
    no caller since the hand-over code went.
  - Doc comments still describe the intake mark, hand-overs or "another
    host takes ... in" as live: `pulls.rs`'s module doc and the docs on
    `follow_across`, `Followed`, `submit_pull`, `submit_pull_group`,
    `PullRequests::across` and `kept_across` in `lib.rs`, `kept.rs:23-40`
    (with a broken `[Observed]` link), `profiles/mod.rs`, `overview.rs`
    and `worktrunk/mod.rs`'s `Standing`.
  - Test doc comments still name host B, the mark or `taken_in_by`
    (`pull_requests.rs` near 1707, 2834, 5160, 5394, 5469, 5521, 5562
    and 5624).

## Scenarios met

These scenarios have code and a test that asserts their THEN clauses,
by reading:

- **forge-rounds:** Every host off; It ends while a run goes on
  elsewhere; A branch the forge does not keep; The schedule wakes each
  branch; A submit wakes it; A recorded result wakes it; A lost wake; A
  job after a job; A repository with signers and no pool; A workflow
  already there; The signer and the pool; No signers; The template's
  environment; A run waits for a host.
- **pull-request-intake:** the refusal of a branch nothing would take
  in, and the need for a pool.
- **pull-request-status:** each of its scenarios, `pool/forge/round` in
  the block.
- **group-submit:** each of its scenarios.
- **overview:** both scenarios.
- **submission-wait:** each of its scenarios.
- **obsolete-combinations:** both scenarios; `Requester::HandOver` is
  gone.
- **request-intake-host:** "A host's step", "Landed by", "Blamed by",
  "Merged", "Held, then marked ready" and the check-reading scenarios.

Read as partly met:

- **forge-rounds and peer-lead:** "A host claims and never leads" and
  "A lead an older sluice took".
- **reader-tick:** its "no more than one ask a tick" clause.

The REMOVED requirements' types are gone: `Taker`, `TakenInBy`,
`LetGo`, `Holder`, the mark's functions, `forge/intake/` and
`waits/<pid>.hold`. An older hand-over's withdrawal is still read as
written (D12).
