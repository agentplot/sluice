# compile-cache Specification

## Purpose
How a run's compiles reach a compile cache its branch's policy
declares, under each executor, so checkouts and the agents' shells on
one host reuse each other's compiled crates without a cache changing
what a check reports (35; ADR 0049 and its amendment).

## Requirements

### Requirement: A policy declares the compile cache in `[checkout]`

A branch's policy SHALL turn the compile cache on with `compile_cache =
"sccache"` in `[checkout]`, read from the branch's revision (56). A
policy that sets no `compile_cache` SHALL compile as before, with no
compile-cache step in the run. Any other value SHALL refuse the
checkout with a message that names the setting and the one value sluice
serves.

#### Scenario: Declared

- **WHEN** a checkout is made from `[checkout] compile_cache = "sccache"`
- **THEN** the place it gives its runs carries the compile cache
  (`a_declared_compile_cache_reaches_the_place` in
  `crates/sluice/tests/profiles/places.rs`)

#### Scenario: Not declared

- **WHEN** a run's policy sets no `compile_cache`
- **THEN** its command runs as before, with no `RUSTC_WRAPPER` from
  sluice and nothing written outside its checkout and scratch directory
  (`a_run_with_no_compile_cache_is_as_before` in
  `crates/sluice/tests/checks.rs`)

#### Scenario: Another tool

- **WHEN** a checkout is made from `[checkout] compile_cache = "ccache"`
- **THEN** it is refused with a message naming `compile_cache` and
  `"sccache"` (`another_compile_cache_is_refused` in
  `crates/sluice/tests/profiles/places.rs`)

### Requirement: A policy names the variables the host's server is found by

A policy that declares the compile cache MAY name, in
`compile_cache_variables`, the tool's client's own variables the host
finds its compile server by. A name that is not the client's own, for
sccache one not starting `SCCACHE_`, SHALL refuse the checkout, naming
the variable, and so SHALL a name sluice sets for the run's compiles
itself.

#### Scenario: A variable that is not the client's

- **WHEN** a checkout is made from `compile_cache = "sccache"` and
  `compile_cache_variables = ["AWS_PROFILE"]`, or
  `["RUSTC_WRAPPER"]`
- **THEN** it is refused with a message naming that variable
  (`a_compile_cache_variable_not_the_clients_is_refused` in
  `crates/sluice/tests/profiles/places.rs`)

### Requirement: An open run compiles through the host's one server

Under the open executor, a run whose policy declares the compile cache
SHALL send its compiles to the host's one compile server, and SHALL
start, stop or restart nothing of that server. When the server does not
answer as the run starts, the run SHALL compile without the cache, and
its log SHALL say so and name the address.

#### Scenario: The host's server answers

- **WHEN** an open run with the cache declared and
  `compile_cache_variables = ["SCCACHE_SERVER_UDS"]` starts while
  sluice's environment sets `SCCACHE_SERVER_UDS` to a socket where a
  compile server answers
- **THEN** the command's compiles go through the tool, addressed to
  that server, and once the run ends that server is still up
  (`an_open_run_compiles_through_the_hosts_server` in
  `crates/sluice/tests/checks.rs`)

#### Scenario: No server answers

- **WHEN** an open run with the cache declared starts while nothing
  answers at the host's compile server's address
- **THEN** the command runs with no `RUSTC_WRAPPER` from sluice, its
  log says the host's compile server did not answer, and no server is
  left running after the run
  (`an_open_run_with_no_host_server_compiles_without_the_cache` in
  `crates/sluice/tests/checks.rs`)

### Requirement: An open run takes only the variables its policy names

An open run whose policy declares the compile cache SHALL find the
host's server through the variables `compile_cache_variables` names:
each one sluice's own environment sets SHALL reach the run with its
value, and no other of the tool's variables from sluice's environment
SHALL. With none of them set, the run SHALL address the tool's default.
A sandboxed run SHALL take none of them.

#### Scenario: Only the named variables reach the run

- **WHEN** an open run's policy names `SCCACHE_SERVER_UDS` alone, and
  sluice's environment sets `SCCACHE_SERVER_UDS` and `SCCACHE_DIR`
- **THEN** the command's environment holds `SCCACHE_SERVER_UDS` with
  sluice's value and no `SCCACHE_DIR`
  (`an_open_run_takes_only_the_named_compile_cache_variables` in
  `crates/sluice/src/profiles/checks.rs`)

#### Scenario: A sandboxed run takes none of them

- **WHEN** a sandboxed run's policy names `SCCACHE_SERVER_UDS` and
  sluice's environment sets it
- **THEN** the environment sluice gives the run holds no
  `SCCACHE_SERVER_UDS`, and the only address its command compiles
  through is its own server's
  (`a_sandbox_run_takes_no_compile_cache_variables` in
  `crates/sluice/src/profiles/checks.rs`)

### Requirement: A sandboxed run holds the cache's directory for its life

Under the sandbox executor, a run whose policy declares the compile
cache SHALL try, inside its sandbox and process group, to take the lock
of the host's compile-cache directory without waiting, and hold it
until its command ends. One run at a time SHALL hold it, across every
repository on the host. A run that finds the lock held SHALL compile
without the cache, and its log SHALL say so.

#### Scenario: The directory is free

- **WHEN** a sandboxed run with the cache declared starts and no other
  run holds the directory's lock
- **THEN** the run holds the lock until its command ends, and a second
  run started meanwhile does not get it
  (`a_sandbox_run_holds_the_compile_cache_for_its_life` in
  `crates/sluice/tests/checks.rs`)

#### Scenario: The directory is held

- **WHEN** a sandboxed run with the cache declared starts while another
  run holds the directory's lock
- **THEN** its command runs with no `RUSTC_WRAPPER` from sluice and its
  log says another run holds the compile cache, and it passes or fails
  as it would without the cache
  (`a_sandbox_run_finding_the_compile_cache_held_compiles_without_it`
  in `crates/sluice/tests/checks.rs`)

### Requirement: A sandboxed run's server is its own and ends with it

A sandboxed run that holds the directory SHALL start a server of its
own, inside its sandbox and process group, in the foreground, over
that directory and on a socket only it is told of, and SHALL send its
compiles to that server alone. The server SHALL end with the run,
whether the command exits, fails or is ended at its time limit.

#### Scenario: Compiled through its own server

- **WHEN** a sandboxed run holding the directory compiles a crate, and
  a later sandboxed run compiles the same crate in another checkout
- **THEN** the second run's compile is served from the cache, and no
  server of either run is left once each has ended
  (`a_sandbox_run_compiles_through_a_server_of_its_own` in
  `crates/sluice/tests/checks.rs`)

#### Scenario: Ended at the time limit

- **WHEN** a sandboxed run holding the directory runs past its time
  limit and is ended
- **THEN** its server has ended with it and the lock is free for the
  next run (`a_sandbox_runs_compile_server_ends_at_its_time_limit` in
  `crates/sluice/tests/checks.rs`)

### Requirement: The sandbox lets a run write the cache's directory and no more

When the policy declares the compile cache, the sandbox executor SHALL
let the run write the compile-cache directory, its lock included,
beyond its checkout and scratch directory, and nothing else it did not
before. The `ready` and `release` commands SHALL be held to the
profile they had before.

#### Scenario: The run's profile

- **WHEN** a sandboxed run's policy declares the compile cache
- **THEN** its Seatbelt profile allows writes to the checkout, the
  scratch directory and the compile-cache directory, and its `ready`
  and `release` profiles do not name that directory
  (`the_run_profile_adds_the_compile_cache_directory` in
  `crates/sluice/src/profiles/checks.rs`)

#### Scenario: A run that may open no socket

- **WHEN** a sandboxed run's policy declares the compile cache and
  `[executor] network = false`
- **THEN** its command runs with no compile-cache step and its profile
  does not name the compile-cache directory
  (`the_run_profile_under_no_network_has_no_compile_cache` in
  `crates/sluice/src/profiles/checks.rs`)

#### Scenario: Anything else is still refused

- **WHEN** a sandboxed run with the cache declared writes outside its
  checkout, its scratch directory and the compile-cache directory
- **THEN** the write is refused as before
  (`a_sandbox_run_with_a_compile_cache_writes_nowhere_else` in
  `crates/sluice/tests/checks.rs`)

### Requirement: A cache never changes what a check reports

A run SHALL report what its command reported, whether it compiled
through the cache or not: the command's exit code and the signal that
ended it reach the run's result as they would with no compile-cache
step. A compile the tool would refuse rather than serve, such as one
under `CARGO_INCREMENTAL=1`, SHALL run without the cache, and the log
SHALL say why (35).

#### Scenario: The exit is the command's

- **WHEN** a run with the cache declared runs a command that exits 3,
  and another whose command is killed by a signal
- **THEN** the first run's result has exit 3 and the second's that
  signal, as with no cache declared
  (`a_compile_cache_step_passes_the_commands_exit_through` in
  `crates/sluice/tests/checks.rs`)

#### Scenario: Incremental compiles

- **WHEN** a run with the cache declared has `CARGO_INCREMENTAL=1` in
  its environment
- **THEN** its command runs with no `RUSTC_WRAPPER` from sluice and its
  log says incremental compiles are not cached
  (`an_incremental_run_compiles_without_the_cache` in
  `crates/sluice/tests/checks.rs`)

### Requirement: Only check runs take the compile cache

A check run, a combination's or a session's (`sluice exec`), SHALL
take the compile cache its policy declares. A preview's command, and
the `ready` and `release` commands of any run, SHALL compile without
it and SHALL NOT take the directory's lock.

#### Scenario: A session run

- **WHEN** `sluice exec` runs in a worktree whose branch's policy
  declares the compile cache
- **THEN** its run takes the cache as a combination's run does
  (`a_session_run_takes_the_compile_cache` in
  `crates/sluice/tests/exec.rs`)

#### Scenario: A preview

- **WHEN** a preview is brought up from a revision whose policy
  declares the compile cache
- **THEN** its command runs with no compile-cache step and does not
  take the directory's lock (`a_preview_takes_no_compile_cache` in
  `crates/sluice/tests/previews.rs`)
