# Verify: pull-requests-are-queued-on-the-forge

Verified on 2026-11-08 at `f8762658`, on
`change/pull-requests-are-queued-on-the-forge`, cut from
`bolt/sluice-runs-one-host-process` (`68d68a9a`).

**Outcome: ready.** The bolt's gate passes, every task holds, and the
change archives with the gate's openspec tasks passing on a scratch
copy. The change is archived with this report. Two points for its
landing are at the end: the branch conflicts with the bolt's head, and
another bolt archives a change this one leaves open.

## The whole checks

`sluice exec bolt/sluice-runs-one-host-process` (sluice 0.73.1) ran the
bolt's gate, `#check-fast`, once. It exited 0 and printed "passed: 13
tasks" and "recorded 13 results under their keys for f876265".

| Task | Outcome |
|---|---|
| `site:rules`, `site:prose`, `plugin-sluice:validate`, `root:adrs` | passed |
| `root:changes`, `root:specs`, `root:format`, `root:lint` | passed |
| `seams:test`, `core:test`, `fakes:test`, `conformance:test` | passed |
| `cli:test` | passed |

`cli:test` ran every test binary of `cargo test -p sluice -- --skip
sandbox`: the library's unit tests (228), `cli.rs` (218), `forge` (37),
and `plugin_docs`, where `every_command_is_named_in_the_skills` passed,
and every binary after it.

The gate does not run `--features tools`. Most tasks of groups 3 to 8
are true only when `pull_requests` passes, so I ran it, with `forge` and
task 6.7's clippy, once through sluice:

```
sluice exec bolt/sluice-runs-one-host-process -- sh -c \
  'cargo test -p sluice --features tools --test pull_requests --test forge &&
   cargo clippy -p sluice-core -p sluice --features tools --tests -- -D warnings'
```

It exited 0. `forge` passed 39 tests, `pull_requests` passed 152, and
clippy found nothing.

## Archive, tried on a scratch copy

With the pinned openspec (1.14.3, `.devenv/profile/bin/openspec`), on a
copy of `openspec/` outside the tree:

- `openspec validate pull-requests-are-queued-on-the-forge --strict`
  and `openspec validate actions-pool --strict` pass before the
  archive.
- `openspec archive pull-requests-are-queued-on-the-forge --yes`
  applies + 22, ~ 19, - 18, → 6.
- After it, `openspec validate --specs --strict --no-interactive`
  passes 68 of 68 (`root:specs`), and `openspec validate --changes
  --strict --no-interactive` passes 8 of 8 (`root:changes`), with
  `actions-pool` valid.
- `forge-rounds` is created with the Purpose its delta gives. No
  archive commit has to write it.

## Tasks

All 37 tasks, 1.1 to 9.4, are ticked. These hold since the last verify:

- **1.5.** The four MODIFIED requirements are now 500 characters or
  fewer, and they say what they said before. `pull-request-intake`
  leaves "A branch turns pull requests on in its policy" as the main
  spec has it. Its refusal is the ADDED "A pull request table nothing
  would take in refuses its branch alone", with both scenarios.
  `pull_requests_need_the_branchs_records_on_its_remote`,
  `pull_requests_need_a_pool` and
  `a_branch_nothing_would_take_in_is_refused_alone` assert them.
  `root:specs` passes after the scratch archive.
- **1.6.** `actions-pool` validates before and after the archive.
- **9.4.** `setting-up-sluice`'s `profiles/pull-requests.md` names
  `sluice job round`. Record 0101's construction choices keep `sluice
  job` visible and name hiding it as the path not taken.
  `plugin_docs` passes in the gate.

The test names in the "True when" clauses of groups 2 to 8 ran in the
library's tests, `cli.rs`, `forge` and `pull_requests` above. All
passed.

## Scenarios met

Each scenario of the change's 14 delta specs has code and a test that
asserts its THEN clause. They are forge-rounds, forge-identity,
request-intake-host, pull-request-submit, worktrunk-submit,
pull-request-intake, pull-request-status, group-submit, overview,
submission-wait, obsolete-combinations, reader-tick, peer-lead and
landing-beside. The last verify read them scenario by scenario. Since
then the code has gained only words: the skill's line and record 0101's
choice. The integration tests behind those scenarios now ran and passed
(above).

## Weaker evidence

- **`sluice init actions` on a differing workflow** exits 1 but still
  adds `[signers.forge]` and `[pools.forge]` where they are absent,
  naming what it added. D13 allows this.
- **The release binary.** `6c087f23` builds Linux's binary static
  against musl. No pass ran the release workflow, or the binary on a
  runner without Nix.
- **No test asserts:**
  - the 50-minute bound and the `*/5` schedule;
  - the `0600` key file (the template does it with `umask 077` and
    `chmod 600`);
  - a lead's requests taken in after a round job takes it from an older
    host;
  - a host claiming after the lease has passed.

## For the landing

1. **The branch conflicts with the bolt's head.** #23
   (`change/a-hosts-queues-move-in-sluices-own-host-process`, landed as
   `cd141bab`) conflicts with this branch in:
   - `crates/sluice/src/lib.rs`, `crates/sluice/src/overview.rs`;
   - both skills' `SKILL.md`, `using-sluice/references/submit.md`;
   - the site's `reference/commands.md` and `reference/state.md`.

   The gate printed this and so does `git merge-tree`. The bolt has to
   be merged into the branch before a submit can land it. The tasks
   expect this: `Command::Run`'s pass becomes
   `host_process::pass::pass`. The checks above ran on this branch's
   tree, not on the merge.
2. **`a-landed-bolt-is-never-reported-withdrawn` stays open and cannot
   archive here.** It is built, with every task ticked, and open on
   `main`, on the bolt and on this branch. Its `request-intake-host`
   delta MODIFIES "A wait whose pull request another host takes in runs
   nothing for it". This change renames that requirement "A wait on a
   pull request runs nothing for it". After this archive, `root:changes`
   gives the info "Archive would refuse this delta" for it. The gate
   still passes.

   `bolt/sluice-tells-each-submitter-its-own-end` archives that change
   in `d77303e6`. There its delta ADDS "A wait on a submission this host
   handed over follows its pull request" to `request-intake-host`. Where
   that bolt and this one meet, the merge needs two things:
   - archive the change's directory here as that bolt did;
   - remove the added hand-over requirement through a change. Hand-overs
     are gone (D12, record 0101), so the requirement no longer holds.

   Archiving it here first instead fails this change's `root:changes`.
   Its MODIFIED block would then omit that change's scenario "A wait on
   a submission handed over follows its pull request", which I tried on
   a scratch copy.
