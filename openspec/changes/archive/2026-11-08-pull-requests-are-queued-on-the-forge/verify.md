# Verify: pull-requests-are-queued-on-the-forge

Verified on 2026-11-08 at `ff172daa`, on
`change/pull-requests-are-queued-on-the-forge`, cut from
`bolt/sluice-runs-one-host-process`. The bolt's head, `cd141bab`, where
the host-process change (#23) landed, is merged in. The branch merges
onto it cleanly.

**Outcome: ready.** The bolt's gate passes, and so do the tools suites
that task 10.1 names. Every task is ticked and holds. On a scratch copy,
the change archives with the gate's openspec tasks passing. The change
is archived with this report.

## The whole checks

`sluice exec bolt/sluice-runs-one-host-process` (sluice 0.73.1) ran the
bolt's gate, `#check-fast`, once against `cd141ba`. It exited 0 and
printed "passed: 13 tasks" and "recorded 13 results under their keys for
ff172da".

| Task | Outcome |
|---|---|
| `site:rules`, `site:prose`, `plugin-sluice:validate`, `root:adrs` | passed |
| `root:changes`, `root:specs`, `root:format`, `root:lint` | passed |
| `seams:test`, `core:test`, `fakes:test`, `conformance:test` | passed |
| `cli:test` | passed, in 2m 37s |

Across the test binaries, 1128 tests passed and none failed. Among them
are the library's unit tests, `cli.rs`, `forge` (37) and `host_process`
(30), and `plugin_docs`, where `every_command_is_named_in_the_skills`
passed. moon's warnings that it could not write its output archives and
shared cache blobs cost caching only. No task failed on them.

The gate does not run `--features tools`. Task 10.1 holds only when its
clippy and three tools suites pass, so I ran them once through sluice:

```
sluice exec bolt/sluice-runs-one-host-process -- sh -c \
  'cargo clippy -p sluice-core -p sluice --features tools --tests -- -D warnings &&
   cargo test -p sluice --features tools --test pull_requests --test host_process --test protect'
```

It exited 0. Clippy found nothing. `host_process` passed 30, `protect`
passed 25 and `pull_requests` passed 153. These included
`packing_under_sluices_own_fetch_waits_on_nothing`,
`wait_follows_a_pull_request` and
`a_landing_stands_when_its_release_then_fails`.

## Archive, tried on a scratch copy

With the pinned openspec (1.14.3, `.devenv/profile/bin/openspec`), on a
copy of `openspec/` outside the tree:

- Before the archive, `openspec validate --changes --strict` passes 9
  of 9.
- `openspec archive pull-requests-are-queued-on-the-forge --yes`
  applies + 22, ~ 20, - 18, → 6 to specs that now carry the
  host-process change's amendments. `reader-tick` and `submission-wait`
  take this change's deltas on top of them.
- After it, `openspec validate --specs --strict --no-interactive`
  passes 69 of 69 (`root:specs`), and `--changes` passes 8 of 8
  (`root:changes`).
- `forge-rounds` is created with the Purpose its delta gives.

## Tasks

All 41 tasks, 1.1 to 10.4, are ticked. Groups 1 to 9 hold as the
previous verify of this change found them, at `f8762658`. Their "True
when" tests ran again above, in the gate and in `pull_requests`, and
passed. Group 10 records the merge with the host process:

- **10.1.** `host_process::pass::pass` lists no pull request branch and
  calls no intake. `pass::step` takes the branch alone. No `Taker` and
  no `crate::intake` call is left in `crates/sluice/src`.
  - A waiting pull request submit, alone or as a group, follows its
    request by `pulls.follow`, which reads GitHub and sluice's check. It
    runs no round and reaches no host process.
  - `wait_on`, for a submission, follows the host process through
    `host_process::ask::Follower`.
  - Status and the overview give `room` and `host_process` beside
    `pull_requests` (`lib.rs`, `overview.rs`). No source file names
    `taken_in_by`. Two tests in `pull_requests` assert it is absent.
  - The conformance suite's `RecordedChecks` passes `hold` through to
    its inner check seam.
  - The protection hook now asks `pruned` before `needing`. The hook
    lets the same refs through as before, but it reads no records for
    git's prune of a branch it has just packed.
    `packing_under_sluices_own_fetch_waits_on_nothing` holds the fetch
    lock and shows that `pack-refs --prune` finishes under it.
- **10.2.** `using-sluice`'s `SKILL.md` and `references/submit.md`,
  `setting-up-sluice`'s `SKILL.md` and `profiles/pull-requests.md`, the
  commands reference, the state reference and the GitHub app guide name
  `sluice job round` for a branch that takes pull requests and the host
  process for every other branch. The state reference says nothing
  reads or writes `forge/intake/` now. `site:prose` and
  `plugin-sluice:validate` pass in the gate.
- **10.3.** The `submission-wait` delta modifies "A landing stands
  whatever fails after it" and keeps all six scenarios the host-process
  change gave it. Its last scenario is now the round job's: a check
  posting fails after the landing. `openspec validate --strict` passes.
- **10.4.** The bolt's gate passes on the merge (above).

## Scenarios met

Each scenario of the change's 14 delta specs has code and a test that
asserts its THEN clause. The previous verify read them scenario by
scenario. The merge added two things, and they hold:

- **"A landing stands whatever fails after it".** `follow_across` in
  `profiles/github/pulls.rs` reads a merged request as landed before it
  reads sluice's check at all. A submit following the request therefore
  reports it landed whatever the round job's round did after the
  landing.
- **The overview's and status's `room` and `host_process` beside
  `pull_requests`.** Tests in `cli.rs` and `pull_requests` assert them.

## Weaker evidence

- **No test fails the check posting after the landing.** "A pull
  request's submission" in `submission-wait` has the round job's
  posting of sluice's check fail after the landing.
  `a_landing_stands_when_its_release_then_fails` fails the round job's
  release instead. GitHub marks the request merged before the round job
  posts its check, and that test reads `passed · landed` there. The
  merged-first reading in `follow_across` is what the scenario rests on.
- **`sluice init actions` on a workflow that differs** exits 1 but
  still adds `[signers.forge]` and `[pools.forge]` where they are
  absent, and names what it added. D13 allows this.
- **The release binary.** Linux's binary is built static against musl.
  No pass ran the release workflow, or ran the binary on a runner
  without Nix.
- **No test asserts:**
  - the 50-minute bound and the `*/5` schedule;
  - the `0600` key file (the template sets it with `umask 077` and
    `chmod 600`);
  - a lead's requests taken in after a round job takes the lead from an
    older host;
  - a host claiming after the lease has passed.

## For the landing

**`a-landed-bolt-is-never-reported-withdrawn` stays open, and it cannot
be archived here.** It is built with every task ticked, and it is open
on `main`, on the bolt and on this branch.

- Its `request-intake-host` delta modifies "A wait whose pull request
  another host takes in runs nothing for it", which this change renames
  "A wait on a pull request runs nothing for it". After this archive,
  `root:changes` gives the info "Archive would refuse this delta" for
  it, and the gate still passes.
- If it is archived here first, this change's `root:changes` fails:
  this change's modification of that requirement would then lack that
  change's scenario "A wait on a submission handed over follows its
  pull request".
- `bolt/sluice-tells-each-submitter-its-own-end` archives that change in
  `d77303e6`. Its delta there adds "A wait on a submission this host
  handed over follows its pull request" to `request-intake-host`.

Where that bolt and this one meet, the merge needs two things:

1. Archive that change's directory as that bolt did.
2. Remove the added hand-over requirement through a change. Hand-overs
   are gone (D12, record 0101).
