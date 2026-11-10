# Spec Delta

## MODIFIED Requirements

### Requirement: A sandboxed run may write the declared directories and reach the declared sockets

Under the sandbox, a run SHALL be able to write each declared `dir` and
connect to each declared `socket`, whatever the policy's `network`
says, and SHALL write nothing else outside its checkout, its scratch
directory, the store where moon keeps what it shares for the run and
what the sandbox already allows (35; C.2 sandbox row as amended at
agentplot/blueprints 0c38f24).

#### Scenario: The declared directory is written

- **WHEN** a host declares a cache `dir` and a sandboxed run writes a
  file in it
- **THEN** the write succeeds and the run passes
  (`the_sandbox_opens_the_hosts_caches` in `crates/sluice/tests/checks.rs`)

#### Scenario: A directory beside it is not

- **WHEN** the same run writes a file in a directory beside the
  declared `dir`
- **THEN** the write is refused and the run fails
  (`the_sandbox_opens_the_hosts_caches`)

#### Scenario: The declared socket is reached with no network

- **WHEN** a host declares a cache `socket` with a server listening on
  it, and a sandboxed run whose policy says `network = false` connects
  to it
- **THEN** the connection succeeds (`the_sandbox_opens_the_hosts_caches`)

## ADDED Requirements

### Requirement: A sandboxed moon run may write where moon keeps what it shares

Under the sandbox, a run whose runner is moon SHALL be able to write the
store where moon keeps that run's blobs and manifests, as sluice finds
it to read a key's manifest: the repository's base checkout's
`.moon/cache`, or `cache/shared` under moon's home, wherever the
repository moves moon's home and whatever the host's `[[caches]]` says
(108; C.2 moon and sandbox rows as amended at agentplot/blueprints
0c38f24; record 0105; rules core.9, core.35).

#### Scenario: moon's home moved by the toolchain, on a host that shares moon's cache

- **WHEN** a host declares a cache whose `dir` is not moon's home and
  whose `env` turns moon's shared worktree setting on, and a sandboxed
  run with `network = false`, in a worktree of a bare repository, whose
  `[tests] toolchain` sets `MOON_HOME` to a directory under `/tmp` where
  moon has not made `cache/shared` yet, runs a task that passes
- **THEN** the task passes, its manifest is in `cache/shared` under that
  home and not in the worktree, and its key is proven reading it there
  (`a_key_run_in_the_sandbox_is_proven_under_the_home_the_toolchain_gives_moon`
  in `crates/sluice/tests/profiles/moon.rs`)

### Requirement: The sandbox opens the store alone, where it resolves

The sandbox SHALL open a moon run's store by its path and where the
path resolves, whether moon has made it yet or not, and sluice SHALL
make nothing of it. It SHALL open no other directory of moon's home. A
run whose store is its own checkout's `.moon/cache` SHALL be given the
sandbox it is given with no store (C.2 sandbox row as amended; record
0105; rule core.34).

#### Scenario: Only the store is opened

- **WHEN** a sandboxed moon run's store lies under moon's home, outside
  its checkout
- **THEN** its sandbox opens the store by its path and where the path
  resolves, and opens neither moon's home nor a directory beside the
  store
  (`the_profile_opens_the_runners_store_outside_the_place` in
  `crates/sluice/src/profiles/checks.rs`)

#### Scenario: A store moon has not made yet

- **WHEN** a sandboxed moon run's store does not exist yet, and a
  directory above it is reached through a link
- **THEN** its sandbox opens the store where it will resolve once made,
  and nothing of the store is made
  (`the_profile_opens_the_runners_store_outside_the_place`)

#### Scenario: A store in the checkout

- **WHEN** a sandboxed moon run's store is its own checkout's
  `.moon/cache`, as a cold run's is where the repository's own moon
  workspace does not share moon's cache
- **THEN** its sandbox is the one it is given with no store
  (`the_profile_opens_the_runners_store_outside_the_place`;
  `a_cold_test_gets_no_cache_in_the_sandbox` in
  `crates/sluice/tests/profiles/moon.rs`)
