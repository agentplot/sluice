# Spec Delta

## Purpose

What a run needs from the host that the host itself shows, a container
engine the branch's policy asks for first: a host that cannot give it
leaves the run for one that can, and a host that loses it during a run
blames nobody for what failed (84, 114, 139; record 0074).

## ADDED Requirements

### Requirement: A host whose container engine does not answer claims no run that asks for one

When a branch's policy says `[executor] docker = true`, a host SHALL look
for a container engine before it claims any of that branch's runs, at
`DOCKER_HOST`'s socket and the usual sockets the sandbox opens. Where no
socket answers as an engine, the host SHALL claim none of the branch's
runs, make no checkout for them and record nothing of them.

#### Scenario: No engine

- **WHEN** `main`'s policy says `docker = true`, a combination of `main`
  is open, and no engine answers at any socket the host looks at
- **THEN** the host claims no run of it, makes no checkout and records
  no result, and the run stays open

#### Scenario: An engine that answers

- **WHEN** the same combination is open and an engine answers at
  `DOCKER_HOST`'s socket
- **THEN** the host claims the run as before

#### Scenario: A policy that asks no engine

- **WHEN** `main`'s policy does not say `docker = true` and no engine
  answers on the host
- **THEN** the host claims `main`'s runs as before, looking at no engine

#### Scenario: An engine read afresh

- **WHEN** a host turned `main`'s run down, and its engine answers by a
  later round
- **THEN** that round's host claims the run

### Requirement: A run turned down for its engine waits for a host that can take it

A run every host turned down for its engine SHALL stay open for a host
whose engine answers. Each host that turned it down SHALL be written in
the branch's state with its reason, once for each host and reason, so
`sluice status` and `sluice wait` on any host that submits to the branch
show the combination waiting for a host whose container engine answers,
naming each such host and why.

#### Scenario: Another host takes it

- **WHEN** host A's engine does not answer and host B's does, both
  serving `main`
- **THEN** host B claims the run and A claims nothing of it

#### Scenario: Status says what it waits for

- **WHEN** host A turned down `main`'s combination #12 because no engine
  answers there, and no host has claimed it
- **THEN** `sluice status main` shows #12's combination waiting for a
  host whose container engine answers, naming host A and the sockets it
  found no engine at, and `sluice status main --json` carries the same

#### Scenario: Said once

- **WHEN** host A turns the same run down in ten rounds for the same
  reason
- **THEN** the branch's state names host A's reason once

### Requirement: A run no host takes for its engine is not passed at its time limit

When a run turned down for its engine is still unclaimed once the
branch's time limit has passed since its combination was opened, the
combination SHALL be read as not passed, as one whose run its time limit ended, and its
submission's session SHALL be told its tests waited for a host whose
container engine answers (66, 87).

#### Scenario: No host has an engine

- **WHEN** `main`'s time limit is 30 minutes, its policy says
  `docker = true`, and the only host serving it has no engine
- **THEN** 30 minutes after the combination opened it is not passed, and
  the submission's notice says no host whose container engine answers
  took its tests

### Requirement: An engine gone after a run leaves its failed tests without a result

After a run of a branch whose policy says `docker = true` in which a test
did not pass, the host SHALL look at the engine again. Where it no longer
answers, each test that did not pass SHALL have no result, nothing
recorded under its key and nobody blamed, with the reason, and the
combination SHALL be tested again. A test that passed SHALL keep its
result (114, 139).

#### Scenario: The engine stopped during the run

- **WHEN** a run has one test that passes and one that stops the
  engine and fails
- **THEN** the passing test's result is recorded under its key, the
  failing test is listed as not run, saying the container engine no
  longer answers, with no result, and no submission is blamed

#### Scenario: The engine still answers

- **WHEN** a test fails and the engine still answers after the run
- **THEN** the fail is recorded under its key and blamed as before

#### Scenario: Every test passed

- **WHEN** every test of the run passed and the engine stopped
  afterwards
- **THEN** every result is recorded under its key, and the engine is not
  looked at

#### Scenario: The time limit ended the run

- **WHEN** the time limit ended a run of `docker = true`
- **THEN** the run is read as the time limit says, and the engine is not
  looked at

### Requirement: sluice exec refuses the branch's tests on a host whose engine does not answer

`sluice exec <branch>` SHALL refuse to run the branch's tests when the
branch's policy says `docker = true` and no engine answers on the host,
running and recording nothing and saying where it looked. A command
given after `--` SHALL run as before, its engine not looked at.

#### Scenario: The tests are refused

- **WHEN** `main`'s policy says `docker = true` and no engine answers,
  and a session runs `sluice exec main`
- **THEN** nothing runs or is recorded, and the output says the host's
  container engine does not answer and names the sockets looked at

#### Scenario: A command runs

- **WHEN** the same session runs `sluice exec main -- true`
- **THEN** the command runs and passes

### Requirement: Whether a host has an engine is read only from the host

A host's having a container engine SHALL be read only by the host
looking at its own sockets for an engine that answers. It SHALL never be
written in a host's configuration, never be a system a branch names, and
never be read from what a test printed or how it failed.

#### Scenario: A test that says docker is down

- **WHEN** a test prints "Cannot connect to the Docker daemon" and fails,
  and the engine answers when the host looks after the run
- **THEN** the fail is recorded under its key and blamed as any fail is

#### Scenario: No system for an engine

- **WHEN** two hosts are both `macos-arm64`, one with an engine and one
  without
- **THEN** both serve `main` under the one system, and only the one
  whose engine answers claims `main`'s runs
