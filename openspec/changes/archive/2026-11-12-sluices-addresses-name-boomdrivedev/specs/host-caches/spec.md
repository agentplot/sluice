## MODIFIED Requirements

### Requirement: A sandboxed run may write the declared directories and reach the declared sockets

Under the sandbox, a run SHALL be able to write each declared `dir` and
connect to each declared `socket`, whatever the policy's `network`
says, and SHALL write nothing else outside its checkout, its scratch
directory, the store where moon keeps what it shares for the run and
what the sandbox already allows (35; C.2 sandbox row as amended at
boomdrivedev/blueprints 0c38f24).

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

### Requirement: A sandboxed moon run may write where moon keeps what it shares

Under the sandbox, a run whose runner is moon SHALL be able to write the
store where moon keeps that run's blobs and manifests, as sluice finds
it to read a key's manifest: the repository's base checkout's
`.moon/cache`, or `cache/shared` under moon's home, wherever the
repository moves moon's home and whatever the host's `[[caches]]` says
(108; C.2 moon and sandbox rows as amended at boomdrivedev/blueprints
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

