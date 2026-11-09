# Verify: a-sandboxed-run-writes-its-runners-store

Verified on `change/a-sandboxed-run-writes-its-runners-store` at
`72a85c24`, cut from `bolt/sluices-sandbox-keeps-its-proof` at
`6500ff66`, on 2026-11-09.

**Verdict: ready to archive.** The bolt's gate passes. Every task is
ticked. Every requirement is built, and each scenario's THEN clause is
asserted by a test that passes.

## The whole checks

`sluice exec bolt/sluices-sandbox-keeps-its-proof` (sluice 0.73.1), the
bolt's gate `#check-fast`, run once: **passed, exit 0**. The 8 tasks
`72a85c2` affects all passed, and sluice recorded 8 results under their
keys for `72a85c2`.

| Task | Result |
|---|---|
| `cli:test` | passed, `profiles::checks::tests::the_profile_opens_the_runners_store_outside_the_place` among them |
| `nix:typecheck` | passed |
| `plugin-sluice:validate` | passed |
| `root:adrs`, `root:changes`, `root:format` | passed |
| `root:lint` | passed: `cargo clippy --workspace --all-targets -- -D warnings` clean |
| `site:prose` | passed: 0 errors, 0 warnings |

The installed sluice 0.73.1 does not carry this change. Under its
sandbox, moon warned on every task "Failed to store blobs, will skip
caching the manifest storage=shared-local-cache". The write it was
refused is to `sluice.git/moon-home/cache/shared`, the fault this change
fixes (record 0105, Context). The run still passed and recorded its
results. moon also warned that it could not archive outputs through the
worktree's `.moon/cache/outputs` link into `sluice.git/moon-outputs`.
That link is the repository's own layout for output archives, not the
manifest store, and is outside this change (proposal, Out of scope:
other directories of moon's home).

`#check-fast` runs no test whose name holds `sandbox`, since macOS
refuses a sandbox inside another (`crates/sluice/moon.yml`, task
`sandbox`). As the tasks say, the sandbox scenarios were run once
outside any sandbox:

`devenv shell -- cargo test -p sluice --features tools --test moon
--test checks -- sandbox`: **passed**. `checks` 4 passed, `moon` 3
passed:

- `a_key_run_in_the_sandbox_is_proven_under_the_home_the_toolchain_gives_moon`
- `a_cold_test_gets_no_cache_in_the_sandbox`
- `a_sandboxed_run_reads_as_an_open_one`
- `the_sandbox_opens_the_hosts_caches`
- `the_sandbox_holds_writes_and_the_network`
- `the_sandbox_opens_the_engines_tcp_address`
- `the_sandbox_lets_a_command_open_a_pseudo_terminal`

## Tasks

All 6 tasks are ticked, and each one's condition holds:

- **1.1** `root:adrs` passes. Record 0105's status names the amendment
  as the construct stage's own choice. Point 1 grants a cold run the
  store its own moon workspace puts outside its checkout.
- **2.1** The unit test passes in `cli:test`, and clippy is clean in
  `root:lint`.
- **2.2** The sandbox scenario passes, run outside any sandbox.
- **3.1** The `[[caches]]` doc comment in `crates/sluice/src/instance.rs`
  says it. Clippy is clean.
- **3.2** `site:prose` passes with no warning.
- **3.3** `plugin-sluice:validate` passes.

`openspec validate a-sandboxed-run-writes-its-runners-store --strict`
passes. The MODIFIED requirement's header matches the one in
`openspec/specs/host-caches/spec.md`, and neither ADDED header clashes
with one there, so the archive applies.

## Specs

### host-caches

- **A sandboxed run may write the declared directories and reach the
  declared sockets** (modified). The list of what a run may write now
  includes the store. All three scenarios still pass in
  `the_sandbox_opens_the_hosts_caches`: the declared directory written,
  the one beside it refused, the socket reached with no network.
- **A sandboxed moon run may write where moon keeps what it shares**
  (added). `Checks::start` resolves `moon::store` once, from the
  checkout and the run's command environment with the toolchain
  sourced. It passes that path to `sandbox_profile` and records the same
  value as `Meta.store`, so where moon writes and where `proven` reads
  are one path. The scenario "moon's home moved by the toolchain, on a
  host that shares moon's cache" is
  `a_key_run_in_the_sandbox_is_proven_under_the_home_the_toolchain_gives_moon`,
  and its setup is the one the scenario describes:
  - a bare repository with a worktree;
  - the host's cache at a directory that is not moon's home, its `env`
    turning on `MOON_CACHE_SHARED_WORKTREE_CACHE` and
    `MOON_EXPERIMENT_CAS_OUTPUTS_CACHE`;
  - `MOON_HOME` set by the toolchain under `/tmp`, reached through the
    link to `/private/tmp`, with `cache/shared` not made;
  - `network = false`.

  It asserts that the task passes, that its manifest is under the home's
  `cache/shared` and not in the worktree, and that `proven` names it.
  The sandbox opens no path under `/tmp` but this one, so the test fails
  without the change.
- **The sandbox opens the store alone, where it resolves** (added).
  `resolved` canonicalizes the nearest ancestor that exists and joins
  the rest of the path to it. `given_and_resolved` gives the path given
  and, where it differs, that path. The store is added only where its
  resolved path lies outside the resolved checkout and scratch.
  `release_request` passes `None`. `the_profile_opens_the_runners_store_outside_the_place`
  asserts all three scenarios:
  - **Only the store is opened.** It adds exactly the store's given and
    resolved paths, and not moon's home, `cache` or `cache/beside`,
    whether given or resolved.
  - **A store moon has not made yet.** Under a link, the store is opened
    at its given path and where it will resolve, and nothing of it is
    made.
  - **A store in the checkout.** For a store in the checkout, under the
    checkout's link or in the scratch, the profile is byte-for-byte the
    one written with no store. `a_cold_test_gets_no_cache_in_the_sandbox`
    still passes.

## Design and rules

- **The decisions are built as the design states them:**
  - one resolution of the store, shared by the profile and the record;
  - nothing added for a store inside the place, so the grant is
    unchanged there (rule core.34);
  - one resolver for every opened path;
  - nothing of the store made (rule core.46);
  - a cold run opened the store its own environment resolves, with
    record 0105 amended to say so;
  - a stored release run under the run's profile.
- **Rule core.1.** The core names no tool outside the check profile.
  The store is passed in as a path.
- **Rule core.26.** The site page and the setup skill's reference name
  no host, team or tool of ours.
- **Rule core.12.** The code and docs cite the C.2 sandbox row as
  amended at agentplot/blueprints 0c38f24, record 0105 and rules core.9,
  core.34 and core.35.
