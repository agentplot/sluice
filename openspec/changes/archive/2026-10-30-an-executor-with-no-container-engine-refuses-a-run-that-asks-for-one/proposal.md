# Proposal

## Why

A branch whose policy says `[executor] docker = true` needs a container
engine for its tests, but nothing asks whether the host that takes the
run has one. A host without an engine runs the tests anyway, the test
that needs the engine fails, and the submission is blamed for the host's
fault. On 2026-10-30 mac-studio's gate, where sluice runs as a user that
reaches no engine, would have failed every combination of sluice's own
`main` this way. Record 0074, accepted and amended that day, decides it:
the engine is a requirement the host itself shows, so a host whose
engine does not answer leaves the run for one whose engine does, and an
engine that goes away during a run blames nobody (84, 114, 139; rule
core.38).

## What Changes

- Before it claims a run of a branch whose policy asks for a container
  engine, a host looks at the engine where the run would reach it:
  `DOCKER_HOST`'s socket and the usual sockets the sandbox opens. Where
  none answers as an engine, the host claims none of that branch's runs.
  The run stays open for a host whose engine answers, and nothing of it
  runs here: no checkout is made, nothing is recorded.
- The host that turned a run down says so in the branch's state, once
  for each host and reason: `sluice status` and `sluice wait` show the
  combination waiting for a host whose container engine answers, and
  which host turned it down and why.
- A run that every host turns down is read as not passed once the
  branch's time limit has gone, as a run for a system no host has is
  today, and the submission's session is told why (66, 87).
- After a run in which a test did not pass, the host looks at the engine
  again. Where it no longer answers, every test that did not pass has no
  result: nothing is recorded under its key, nobody is blamed, and the
  combination is tested again. A test that passed keeps its result.
- `sluice exec <branch>` on a host whose engine does not answer refuses
  to run the branch's tests, saying why; a command given after `--` runs
  as before, since it is the session's own and asks nothing of the
  policy.
- Whether a host has an engine is read from the host at each round,
  never written in a host's configuration, never named as a system, and
  never read from what a test printed.

Out of scope:

- Removing `[tests] ready`, its runs before and after a run, and its
  documentation: unit `the-ready-probe-is-gone`, in flight beside this
  one. This change adds the engine look beside `ready` and depends on
  nothing of it.
- An engine reached over TCP (`DOCKER_HOST=tcp://…`): the record names
  sockets the sandbox opens, and this change looks only there; a host
  that reaches its engine only over TCP waits on a design answer.
- A run started on a pool, whose job's start is its claim (88): its job
  turning the run down is left to a later change once the design says
  what a job that refuses at its start reports.

## Capabilities

### New Capabilities

- `host-requirements`: what a run needs from the host that the host
  itself shows, a container engine the policy asks for first: a host
  that cannot meet it turns the run down before claiming it, says so in
  the branch's state, and after a run in which a test did not pass reads
  it again, leaving those tests without a result where it has gone.

### Modified Capabilities

None. The claim rules (`run-claims`), the systems a run is taken on
(`check-systems`) and what a run with no result records (`run-outcome`)
hold as written; this change adds a reason a host claims nothing and a
reason a test has no result, under the new capability.

## Impact

- `crates/sluice-seams`: the check seam gains one question, what of a
  run's requirements this host does not meet, in words.
- `crates/sluice` check profile: the engine look at the sockets the
  sandbox profile already opens (`docker_sockets`), by the Docker Engine
  API's `GET /_ping`; a way for tests to name those sockets.
- `crates/sluice-core`: claims consult the seam once a round; a run's
  record gains the hosts that turned it down; the time limit reads such
  a run as not passed; the after-run look turns fails into tests with no
  result; a session run of the tests is refused.
- `crates/sluice` CLI: `sluice status`, `sluice wait` and their JSON say
  what an open run waits for; `sluice exec` says why it refused.
- `crates/sluice-fakes`: the fake check seam can be told the host lacks
  an engine.
- Docs: the `docker` row of the configuration reference, the sandbox
  profile reference and the verdicts reference of the plugin's skills.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/**
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/**
crates/sluice/src/profiles/checks.rs
crates/sluice/src/progress.rs
crates/sluice/src/exec.rs
crates/sluice/src/lib.rs
crates/sluice/tests/**
site/src/content/docs/reference/configuration.md
plugins/sluice/skills/setting-up-sluice/references/profiles/sandbox.md
plugins/sluice/skills/using-sluice/references/verdicts.md
```
