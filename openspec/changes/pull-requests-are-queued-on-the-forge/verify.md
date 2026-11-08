# Verify: pull-requests-are-queued-on-the-forge

Verified on 2026-11-08 at `3b884f93`, on
`change/pull-requests-are-queued-on-the-forge`, cut from
`bolt/sluice-runs-one-host-process` (`68d68a9a`).

**Outcome: not ready.** The bolt's gate fails on a command the plugin's
skills do not name. Archived as it stands, the change would also fail
`root:specs` and `root:changes`. Nothing is archived. The change goes
back to its code stage.

## The whole checks

`sluice exec bolt/sluice-runs-one-host-process` (sluice 0.73.1) ran the
bolt's gate, `#check-fast`, once. It exited 1, with 12 tasks passed and
1 failed, and printed "recorded 13 results under their keys for
3b884f9".

| Task | Outcome |
|---|---|
| `site:rules`, `site:prose`, `plugin-sluice:validate`, `root:adrs` | passed |
| `root:changes`, `root:specs` | passed |
| `root:format`, `root:lint` | passed |
| `seams:test` (4), `core:test` (152), `fakes:test` (8), `conformance:test` | passed |
| `cli:test` | **failed** |

`cli:test` runs `cargo test -p sluice -- --skip sandbox`. The library's
unit tests passed (228), as did `cli.rs` (218) and `forge` (37). The
`plugin_docs` binary failed, with 1 test passed and 1 failed:

```
---- every_command_is_named_in_the_skills stdout ----
thread 'every_command_is_named_in_the_skills' panicked at crates/sluice/tests/plugin_docs.rs:43:5:
commands no skill names (as `sluice <command>`): ["job"]
```

**Cause.** `1dd0949b` adds `sluice job` (`Command::Job`,
`crates/sluice/src/lib.rs:532`) as a visible subcommand. No file under
`plugins/sluice/skills/` names `sluice job`. The commands reference
names it (`site/src/content/docs/reference/commands.md:32`, `:1014`).
ADR 0023's test requires every visible command to be named in a skill.
There are two ways to fix it. One is to name `sluice job round` in
`setting-up-sluice`, which writes the workflow that runs it, or in
`using-sluice`. The other is to hide the command from `--help`, since
the forge's workflow runs it and no person does. Tasks 9.2 and 9.3 name
only `plugin-sluice:validate`, and that task does not run this test.

Cargo stopped at `plugin_docs`. The test binaries after it did not run
in this gate: `previews`, `protect`, `pushed_refs`, `reader_tick`,
`remote_branch`, `remote_refs`, `runs`, `sessions`, `stack`,
`telemetry`, `telemetry_logs`, `usage` and `worktrunk`.
`pull_requests` needs `--features tools` and runs in `#check-tools`,
main's gate, not the bolt's. No integration run that tasks 3.x to 8.x
name was confirmed by this gate.

The gate also printed that `3b884f9` would conflict with #23
(`change/a-hosts-queues-move-in-sluices-own-host-process`) on the bolt,
in `crates/sluice/src/lib.rs`, `crates/sluice/src/overview.rs`, both
skills' `SKILL.md`, `using-sluice/references/submit.md`, and the site's
`reference/commands.md` and `reference/state.md`.

## Archive, tried on a scratch copy

`openspec validate pull-requests-are-queued-on-the-forge --strict`
passes, and so does `root:changes`. I archived a copy of `openspec/`
with the pinned openspec (1.14.3, `.devenv/profile/bin/openspec`). It
applied + 21, ~ 20, - 18 and → 6, and the gate's two openspec tasks
then failed:

1. **`root:specs` fails on 5 specs** (63 passed, 5 failed). Four
   requirements are over 500 characters after the change modifies
   them. Under `--strict`, that warning fails the check:
   - `forge-identity`, "forge check verifies the setup" (684);
   - `group-submit`, "Under pull requests the act marks them all ready
     when every condition holds" (518);
   - `pull-request-intake`, "A branch turns pull requests on in its
     policy" (711);
   - `pull-request-status`, "sluice's check carries the pull request's
     queue as fields" (527).

   Each is a MODIFIED block of this change's delta. The main spec
   passes today, so the change's text pushes them over the limit.
   Validating a change does not catch this, so only an archive shows
   it. Each needs its examples moved into scenarios, or a split into
   requirements that each state one behaviour.

   The fifth is `forge-rounds`, which the archive creates with
   openspec's placeholder Purpose. The archive commit writes that
   Purpose, as `1a1e0e9e` did for `submit-by-name`.
2. **`root:changes` fails on `actions-pool`.** Its MODIFIED "forge
   check verifies the setup" (`openspec/changes/actions-pool/specs/forge-identity/spec.md`)
   leaves out the scenario "Another branch the forge keeps", which this
   change adds to the main spec. Once this change is archived, archiving
   `actions-pool` would drop that scenario, so openspec refuses it. This
   branch already edits `actions-pool` (`f046d228`). Its block needs the
   scenario copied in, which validates before and after this archive.

The openspec on PATH (1.16.0) gives the same 5 spec failures. The gate
uses the pinned one.

## Tasks

All 34 tasks, 1.1 to 9.3, are ticked. Tasks 9.2 and 9.3 do not hold
under the gate, since a skill names no `sluice job` (above). The
integration runs that tasks 3.x, 4.1, 5.x, 6.x, 7.1 and 8.x name were
not confirmed by this gate.

## The fixes since the last verify hold

- **1.4.** "A pull request taken again waits to be taken in" sits under
  `## ADDED Requirements`. `openspec show --json --deltas-only` gives it
  `ADDED`, and the scratch archive adds it to `pull-request-submit`.
- **3.6.** `the_pools_calls` (`crates/sluice/tests/profiles/forge.rs`)
  answers `/repos/o/r` with `default_branch` `trunk`. It asserts a wake
  on a pool with no `ref` dispatched on `trunk`, and one whose pool
  names `refs/heads/release` dispatched on `release`. It passed in this
  gate.
- **6.8.** `App::open`'s settle path (`rest.rs:662`) now calls
  `pull_from` and never lists the branch.
  `a_write_that_makes_something_is_read_back_before_it_is_made_again`
  asserts one ask by head and no list. It passed in this gate.

## Weaker evidence and smaller gaps

- **`sluice init actions` on a differing workflow** exits 1 but still
  adds `[signers.forge]` and `[pools.forge]` where they are absent. It
  names only what it added.
- **The release binary.** `6c087f23` builds each binary with the pinned
  Rust, and Linux's against musl. This pass ran neither the workflow
  nor the binary on a runner without Nix.
- **No test asserts:**
  - the 50-minute bound and the `*/5` schedule;
  - the `0600` key file (the template does it with `umask 077` and
    `chmod 600`);
  - a lead's pull requests taken in after a round job takes it from an
    older host;
  - a host claiming after the lease has passed.

## Scenarios met, by reading

These scenarios have code and a test that asserts their THEN clauses.
Only the library's tests, `cli.rs` and `forge` ran in this gate.

- **forge-rounds:** Every host off; It ends while a run goes on
  elsewhere; A branch the forge does not keep; The schedule wakes each
  branch; A branch its own policy names; A submit wakes it; A default
  branch that is not main; A recorded result wakes it; A lost wake; A
  job after a job; A repository with signers and no pool; A workflow
  already there; The signer and the pool; No signers; The template's
  environment; A run waits for a host.
- **forge-identity:** No round job on the branch; The app may not
  dispatch it; An environment any branch may use; Another branch the
  forge keeps.
- **request-intake-host:** A host's step; A wait; Submitted here, found
  by head through `Forge::pull_from`, both on following and on settling
  a lost create; Landed by; Blamed by; Merged; Held, then marked ready;
  and the scenarios that read the check.
- **pull-request-submit:** No wait, in words; No wait, in one document;
  Sent back by the round job; Sent back as a draft; Sent back, not a
  draft; Still queued; Taken again, its check still failed.
- **worktrunk-submit:** Run again before the round job takes it in
  again, by `taken_again`.
- **pull-request-intake, pull-request-status, group-submit, overview,
  submission-wait, obsolete-combinations, reader-tick, peer-lead:** each
  of their scenarios.

## What the code stage needs to do

1. Name `sluice job round` in a skill, or hide `sluice job` from
   `--help`, so `every_command_is_named_in_the_skills` passes. Then
   check that the binaries after `plugin_docs` pass.
2. Bring the four MODIFIED requirements above to 500 characters or
   fewer, each, in this change's deltas.
3. Copy "Another branch the forge keeps" into `actions-pool`'s MODIFIED
   "forge check verifies the setup".
