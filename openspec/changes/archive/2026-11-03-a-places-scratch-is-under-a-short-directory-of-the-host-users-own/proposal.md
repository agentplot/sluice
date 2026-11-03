## Why

A checkout's scratch, the `SLUICE_TMP` its checks may write and the
sandbox gives them as `TMPDIR`, is
`<the repository's git common dir>/sluice/.tmp/<10 hex>`. Its name no
longer grows with the checkout's (record 0063), but its root grows with
where the repository lives: 73 bytes on one host. On 2026-11-03 another
repository's tests of a terminal multiplexer could not pass under
`sluice exec` there: the multiplexer's named session keeps its socket at
`<home>/<tool>/sessions/<name>/<tool>-client.sock`, the test's home is
under `SLUICE_TMP`, and the socket's path came to 108 bytes, over the
104 macOS allows. Nothing the check's author does from inside the check
shortens where its repository is.

Clause 35 already meant the scratch to fit the tightest limit a check
meets. As amended at agentplot/blueprints f15896b, it puts the scratch
under a short directory of the host user's own, apart from every
repository, at a path that grows with neither the checkout's name, its
worktree's location nor the repository's, and names macOS's 104 bytes
for a socket among the limits; scratch under that directory that no
record of the repository names, among the scratch the repository's
checkouts made there, is an abandoned checkout's (123). Record 0063 and
rule core.30 are amended with this change's first commit.

## What Changes

- **The scratch is under the host user's own directory.** A checkout
  made from now on is given `<real path of /tmp>/sluice-<uid>/<10 hex of
  the repository>/<10 hex of the checkout>`: on macOS
  `/private/tmp/sluice-501/…`, 45 bytes, on Linux `/tmp/sluice-1000/…`,
  38, whatever the repository, its worktree or the checkout is called or
  where it lives (35 as amended; record 0063 as amended). Its record names
  the path, as since record 0063.
- **The directory is the user's own.** `/tmp/sluice-<uid>` and the
  repository's directory under it are made readable, writable and
  searchable by the user alone. One that already exists and is a link,
  another user's, or open to others, is refused with its path and what
  is wrong; sluice makes nothing under it and changes nothing of it.
- **A scratch the system emptied is made again.** A restart or the
  system's own cleaning may empty `/tmp`; a run, or a release run for a
  checkout whose worktree is gone, makes the scratch again at the path
  its record names, with the user's and the repository's directories.
- **The sweep reaches only the repository's own.** The removal of
  scratch no record names (123) lists the repository's directory under
  the user's, never the user's directory itself, so another repository's
  scratch is never weighed; and it sweeps the repository's former root,
  `sluice/.tmp/`, by the rules it had, while places made before keep
  their scratch there.
- **A checkout made before keeps its scratch until it is released.** A
  record naming a scratch under `sluice/.tmp/`, or naming none, is used
  and released where it names, a worktree's included; a worktree takes
  the short path at its first run after `sluice release`.
- **The sandbox writes the new scratch as it did the old.** The record,
  `SLUICE_TMP`, `TMPDIR` and the Seatbelt profile's writable rule name
  the scratch's resolved path, so a check binds a socket anywhere under
  it. Nothing of the sandbox profile changes; a test under the sandbox
  proves it.
- The state and configuration reference pages say where a checkout's
  scratch is, that it is the host user's own, and that it may be emptied
  when the host restarts.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `place-scratch`: where a checkout's scratch is, who owns the directory
  above it, what the sweep weighs, and where a checkout made before is
  released.

## Out of scope

- **The container executor** is not built (`choose` refuses
  `kind = "container"`); when a change builds it, it gives the container
  the scratch its checkout's record names (C.4 container row). Nothing
  of it changes here.
- **The Linux sandbox** is not built either; it writes the scratch the
  record names when a change builds it.
- **Moving a worktree's scratch made before onto the short path without
  `sluice release`.** A worktree's scratch keeps what its checks leave
  between runs; moving it would discard that. A later change may move it
  if a host needs it.
- **A setting naming another directory for a host whose `/tmp` is small
  or mounted with no programs allowed to run.** No host has asked; it
  waits on one that does.
- **A container engine that shares only the user's home with its
  machine.** A check that mounts a folder under its scratch into a
  container there sees an empty folder; the host shares `/tmp` with its
  engine. Said on the configuration page; sluice changes nothing for it.

## Impact

- `crates/sluice/src/profiles/places.rs`: the user's directory and the
  repository's key, `new_scratch`, the scratch made through one function
  that makes and checks the directories above it, the sweep's two roots,
  the module's doc comment; a builder and a variable for tests to name
  the directory standing for the user's, since the gate runs the tests
  under the sandbox, which lets them write their own directories and not
  `/tmp/sluice-<uid>`.
- `crates/sluice/src/profiles/mod.rs`: the binding hands the tests'
  variable to the place seam.
- `Cargo.toml`: nix's `user` feature, for the effective user's id.
- `crates/sluice-seams/src/lib.rs`: the doc comments of `Place::tmp` and
  `Checkout::remove_unnamed_scratch`.
- `crates/sluice/moon.yml`: the `sandbox` task runs `--test exec` too, so
  the sandbox scenario through `sluice exec` is run where the sandbox
  tests are.
- Tests: `crates/sluice/tests/profiles/places.rs`,
  `crates/sluice/tests/exec.rs`, `crates/sluice/tests/support/mod.rs`,
  `crates/sluice/tests/stack.rs`,
  `crates/sluice/tests/profiles/conformance.rs`, and the unit tests in
  `places.rs`.
- Docs: `site/src/content/docs/reference/state.md`,
  `site/src/content/docs/reference/configuration.md`.
- Records and rules: record 0063 and rule core.30 as amended,
  `docs/architecture/_open.md`'s item for them.
- Sources: clauses 34, 35 (as amended at agentplot/blueprints f15896b),
  123, 129, C.4 sandbox and container rows; record 0063 as amended;
  rules core.10, core.16, core.26, core.30, core.31.

## Touches

```
crates/sluice/src/profiles/places.rs
crates/sluice/src/profiles/mod.rs
crates/sluice-seams/src/lib.rs
Cargo.toml
crates/sluice/moon.yml
crates/sluice/tests/profiles/places.rs
crates/sluice/tests/exec.rs
crates/sluice/tests/support/mod.rs
crates/sluice/tests/stack.rs
crates/sluice/tests/profiles/conformance.rs
site/src/content/docs/reference/state.md
site/src/content/docs/reference/configuration.md
docs/adr/0063-a-places-scratch-is-at-a-short-path-named-by-its-record.md
docs/architecture/core.md
docs/architecture/_open.md
openspec/changes/a-places-scratch-is-under-a-short-directory-of-the-host-users-own/**
```
