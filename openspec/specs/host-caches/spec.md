# host-caches Specification

## Purpose
Lets every run sluice starts on a host use the caches the host keeps
for its builds, as the host's config declares them, without a run
reaching anything else outside its checkout and without sluice naming
or running any cache's tool (35, 71; record 0049 as amended).

## Requirements

### Requirement: The host's config declares the host's shared caches

The host's config SHALL declare the host's shared caches as a list of
`[[caches]]` entries beside `[runs]`. Each entry has `dir`, a directory
given as an absolute path or one under `~/`; an optional `socket`, the
path of the cache server's socket, written the same way; and an optional
`env`, a table of the variables the cache's clients read, each a string.
A repository's policy SHALL declare no shared cache.

#### Scenario: A compile cache with a server

- **WHEN** the host's config has a `[[caches]]` entry with `dir`,
  `socket` and `env`
- **THEN** sluice starts and reads that entry, `~` expanded in `dir`
  and `socket` and the `env` values taken exactly as written
  (`the_hosts_caches_are_read_from_its_config` in
  `crates/sluice/src/instance.rs`)

#### Scenario: No caches declared

- **WHEN** the host's config has no `[[caches]]`
- **THEN** every run is given no cache, exactly as before the table
  existed (`the_hosts_caches_are_read_from_its_config`)

### Requirement: A cache entry sluice cannot honour is refused

sluice SHALL refuse to start, naming the entry and the key, when a
`[[caches]]` entry has no `dir`, a key it does not know, a `dir` or
`socket` that is relative, an `env` value that is not a string, or an
`env` variable that sluice sets for a run itself (`PATH`, `HOME`,
`USER`, `LOGNAME`, `SHELL`, `TMPDIR`, `LANG`, `TERM`, `LC_*` or
`SLUICE_*`), or when two entries set the same variable.

#### Scenario: A relative directory

- **WHEN** a `[[caches]]` entry says `dir = "cache"`
- **THEN** sluice refuses to start and says the `dir` of that entry
  must be an absolute path (`a_cache_entry_sluice_cannot_honour_is_refused`
  in `crates/sluice/src/instance.rs`)

#### Scenario: A variable sluice sets itself

- **WHEN** an entry's `env` sets `TMPDIR` or `SLUICE_JOBS`
- **THEN** sluice refuses to start and names the variable
  (`a_cache_entry_sluice_cannot_honour_is_refused`)

#### Scenario: One variable declared twice

- **WHEN** two entries both set `RUSTC_WRAPPER`
- **THEN** sluice refuses to start and names the variable and both
  entries (`a_cache_entry_sluice_cannot_honour_is_refused`)

### Requirement: A sandboxed run may write the declared directories and reach the declared sockets

Under the sandbox, a run SHALL be able to write each declared `dir` and
connect to each declared `socket`, whatever the policy's `network`
says, and SHALL write nothing else outside its checkout, its scratch
directory and what the sandbox already allows (35; C.2 sandbox row).

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

### Requirement: A declared directory that holds a checkout is refused

A run SHALL be refused before anything of it runs when a declared `dir`
is its checkout, holds its checkout, or lies inside it, with a message
naming the `dir` and saying a shared cache lies outside every checkout
(35).

#### Scenario: The cache is the checkouts' parent

- **WHEN** a declared `dir` is a directory the run's checkout is inside
- **THEN** the run is refused, nothing of it runs, and the message
  names the `dir` (`a_cache_holding_the_checkout_is_refused` in
  `crates/sluice/tests/checks.rs`)

### Requirement: Every run is given the declared environment

Every run sluice starts SHALL be given each declared `env` variable
with its declared value, under any executor: a submission's run, an
agent's `sluice exec`, a preview's command, and the `release` command.
The repository's `toolchain` is sourced after them, as it is in an
agent's shell (35).

#### Scenario: An open run gets the variables

- **WHEN** a host declares `env = { SCCACHE_DIR = "/caches/sccache" }`
  and a run under the open executor prints its environment
- **THEN** it prints `SCCACHE_DIR=/caches/sccache`
  (`every_run_gets_the_hosts_cache_environment` in
  `crates/sluice/tests/checks.rs`)

#### Scenario: Only what is declared

- **WHEN** sluice's own environment has a variable the host's config
  does not declare, such as `RUSTC_WRAPPER`
- **THEN** the run does not get it
  (`every_run_gets_the_hosts_cache_environment`)

#### Scenario: An agent's run in its worktree

- **WHEN** an agent runs `sluice exec -- env` on a host that declares a
  cache's `env`
- **THEN** the command prints the declared variables
  (`sluice_exec_gets_the_hosts_cache_environment` in
  `crates/sluice/tests/exec.rs`)

### Requirement: A test the policy marks cold is given no cache

A branch's policy SHALL be able to mark tests cold with `[tests] cold`,
a list of tests named as the gate's `required` names them. A run that
holds a test so marked SHALL be given none of the host's caches: no
declared directory or socket opened, no declared variable set (35).

#### Scenario: A cold test under moon

- **WHEN** `[tests] cold = ["#bench"]` and a combination's run holds a
  moon task tagged `bench`
- **THEN** the run's command gets none of the declared variables, and
  under the sandbox cannot write the declared `dir`
  (`a_cold_test_gets_no_cache` in `crates/sluice/tests/profiles/moon.rs`)

#### Scenario: A run with no cold test

- **WHEN** `[tests] cold` names tests none of which the run holds
- **THEN** the run is given every declared cache (`a_cold_test_gets_no_cache`)

#### Scenario: A cold command

- **WHEN** the runner is `"command"` and `[tests] cold` lists its
  command
- **THEN** the command gets none of the declared variables
  (`a_cold_command_gets_no_cache` in `crates/sluice/tests/checks.rs`)

### Requirement: sluice runs no cache's server

sluice SHALL start, end and lock nothing of a cache: no server, no lock
file, no directory created or removed. It SHALL name no cache's tool. A
cache whose server is down changes nothing sluice does; what the run's
tools do without it is theirs (35, core.1, core.10).

#### Scenario: A socket with nothing behind it

- **WHEN** a host declares a `socket` that no server listens on and a
  run starts
- **THEN** the run starts and ends as it would with no cache, and
  nothing is left listening on the socket or holding the `dir`
  (`the_sandbox_opens_the_hosts_caches`)

### Requirement: A shared cache never changes a result

A test's result SHALL be the same with the host's caches as without
them; what the caches hold supplies outputs and decides nothing (35,
108). A result moon replays still counts only through a pass recorded
for its key.

#### Scenario: A session's run is replayed through the runner's shared cache

- **WHEN** the host declares moon's shared worktree cache as a host
  cache, an agent runs the suite in a clean worktree with `sluice exec`
  and submits
- **THEN** the combination replays the tests the agent's run recorded
  and lands without running them again
  (`s26_a_clean_session_run_is_replayed_by_its_submission` in
  `crates/sluice/tests/profiles/conformance.rs`)

### Requirement: The moon profile proves a key by moon's manifest where moon keeps it

When it proves a key a moon run reported, the moon profile SHALL read
moon's manifest of that key where moon keeps it: in the checkout's
`.moon/cache`, or, when moon's shared worktree setting is on for the
run, in the store moon shares among the repository's worktrees, found
from the checkout and the run's environment as moon 2.6.1 finds it. It
SHALL NOT read it from where the host's `[[caches]]` says (C.2 moon
row, 35).

#### Scenario: A repository with a base checkout

- **WHEN** the host declares moon's shared worktree cache, and a
  combination's run in a worktree of a repository whose base checkout
  holds `.moon` reports a key
- **THEN** the profile reads the key's manifest from the base
  checkout's `.moon/cache`, and the key is signed when the tree bears
  it out (`a_key_is_read_where_moon_keeps_it` in
  `crates/sluice/src/profiles/moon.rs`; the real rig's signed
  scenarios in `crates/sluice/tests/profiles/conformance.rs`)

#### Scenario: A bare repository with worktrees beside it

- **WHEN** the repository is a bare `<repo>.git` with its worktrees
  beside it, and a run with moon's shared worktree setting on reports a
  key
- **THEN** the profile reads the key's manifest from `cache/shared`
  under moon's home, which is where moon wrote it, and not from the
  worktree's `.moon/cache`
  (`a_key_run_in_a_bare_repositorys_worktree_is_proven` in
  `crates/sluice/tests/profiles/moon.rs`)

#### Scenario: A run with the setting off

- **WHEN** a run is cold, or the host declares no moon cache, so moon's
  shared worktree setting is off for it
- **THEN** the profile reads the key's manifest from the checkout's own
  `.moon/cache` (`a_key_is_read_where_moon_keeps_it`)

#### Scenario: The host's table names another directory

- **WHEN** the host's `[[caches]]` declares a `dir` that is not where
  moon keeps its store
- **THEN** the profile still reads the manifest where moon keeps it
  (`a_key_is_read_where_moon_keeps_it`)
