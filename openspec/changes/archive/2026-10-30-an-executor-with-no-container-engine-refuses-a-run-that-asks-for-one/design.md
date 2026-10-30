# Design

## Context

Record 0074 (accepted 2026-10-30, amended the same day on the user's
words) and clause 84 as amended, the C.3 sandbox row and rule core.38
govern this change; clauses 66, 87, 114 and 139 give the shapes it
reuses.

What exists on main today:

- `[executor] docker = true` parses into `Limits::docker`
  (`crates/sluice-core/src/policy.rs`). The check profile
  (`crates/sluice/src/profiles/checks.rs`) passes `DOCKER_HOST`,
  `DOCKER_CONTEXT` and `DOCKER_CONFIG` through to the run, and its
  Seatbelt profile opens the sockets `docker_sockets()` lists:
  `DOCKER_HOST`'s `unix://` path, `/var/run/docker.sock`, and
  `~/.docker/run/docker.sock`, `~/.orbstack/run/docker.sock`,
  `~/.colima/default/docker.sock`, each as given and as resolved. Nothing
  looks at whether an engine answers there.
- A host claims a run through `Step::claimable` in
  `crates/sluice-core/src/step.rs`, called from four places: the
  reopened runs and new candidates in `choose_starts`, the lead's
  standing-hosts-before-pools pass (107), and a peer's claim loop. A
  claim is the run's state set to `Starting` with its host and place,
  written to the branch's records; `perform_starts` then makes the place
  and calls the check seam's `start`.
- The lead reads an open run as timed out at the time limit only when it
  is `!ours(r, home)`, a run of a system the lead does not run (66, 87).
- `[tests] ready` runs a repository's command before and after a run;
  the after-run read (`ready_after` and the end of `progress`) turns
  fails and missing tests into `NotRun` with a reason. Unit
  `the-ready-probe-is-gone` removes all of it in parallel; this change
  adds nothing to it and reads nothing from it.
- Session runs: `Sluice::session_request` turns a `--` command into
  `runner = "command"` and drops `ready` and `not_run`, since a
  command's exit is its own.

The Docker Engine API (checked against its OpenAPI reference, v1.47):
`GET /_ping` (and `HEAD /_ping`) is "a dummy endpoint you can use to
test if the server is accessible", answering `200` with the body `OK`
and an `Api-Version` header, `500` on a server error. It is served on the
engine's unix socket, unversioned, by Docker Engine, Docker Desktop,
OrbStack and Colima alike.

## Goals / Non-Goals

**Goals:**

- One question on the check seam, asked by the core, so the engine's
  sockets stay the profile's (core.1, core.2) and the decisions, whether
  to claim, how to read a failed run, stay the core's (core.14).
- The look before a claim and the look after a failed run read the host
  the same way, through the same function.
- What an open run waits for is in the branch's records, so a status on
  any host that submits to the branch reads it (26, 80; core.11).

**Non-Goals:**

- Any change to `[tests] ready` (the other unit's).
- An engine over TCP, and a pool's job turning a run down (proposal, out
  of scope).
- A requirement other than the engine. The seam's question is general in
  shape, but the profile answers only for the engine.

## Decisions

### D1. The seam asks what of a run's requirements this host does not meet

`Tests` gains `fn unmet(&self, limits: &Limits) -> Result<Option<String>>`:
`None` when the host meets every requirement `limits` asks, otherwise the
reason in words, the sockets looked at and what each said. The profile
answers `None` at once when `limits.docker` is false, so no branch
without the engine pays anything. The submit-only seam in `ops.rs` keeps
its `unreachable!`; the fake check seam answers what a test set
(`FakeTests::engine(Option<String>)` or the like), `None` by default.

Alternative: the profile refuses in `start`, as `choose` refuses a limit
it cannot hold. Rejected: `start` runs after the claim is written and
the place is made, and the record asks for the refusal before claiming.

### D2. The profile reads the engine by `GET /_ping` on each socket it would open

`engine_answers(sockets)` connects to each path of `docker_sockets()`
that exists, with a short connect and read timeout (two seconds each),
sends `GET /_ping HTTP/1.0\r\nHost: docker\r\n\r\n`, and counts the
socket as an engine when the status line is `200`. The first that
answers ends the look. None answering gives the reason: each path that
exists with what it said (refused, timed out, the status it answered),
or, where none exists, the paths looked at. This is the executor reading
the host where it would open the run's socket: no command runs, nothing
of the repository's is read, and no test's words are read (core.37,
core.38).

The look runs as the process that would run the checks, so a gate user
that reaches no engine, OrbStack serving only the person's login
session, finds none, which is the case that started this.

`Checks` gains `with_engine_sockets(Vec<PathBuf>)`, like `with_running`,
so tests name the sockets looked at instead of the host's
`/var/run/docker.sock`; production uses `docker_sockets()`.

Alternatives: `docker info` or `docker version`, a client that may not be
installed, reads contexts the sandbox does not open, and is a command
sluice runs, which core.37 rules out as a probe. Checking the socket
file exists: OrbStack and Docker Desktop leave a socket behind that
nothing listens on, and the record asks for one that answers.

### D3. A round looks once, before its first claim, and claims nothing while unmet

`Step` reads `unmet(&policy.limits)` at most once a round for a branch,
lazily, the first time `claimable` would return true for one of its
runs; the answer is kept for the rest of that round only. While it is
`Some`, `claimable` returns false for every run of the branch, so the
lead's 107 pass, `choose_starts` and the peer's claim loop all pass it
by, and the lead's 107 pass then offers the run to other live hosts and
pools as it does a run of another system. A branch with no open run this
host could claim never looks, so an idle reader's tick asks the host
nothing (core.31). Since every run of a branch is under one policy,
turning all of them down keeps 76's earliest-first order whole.

The claim and `perform_starts` share a round, so the engine is not
looked at again between them; an engine that stops in those seconds is
caught by D5 if a test fails, and costs nothing if none does.

### D4. The run's record names who turned it down, once per host and reason

`SystemRun` gains `declined: BTreeMap<String, String>`, host name to
reason, skipped when empty. When a round's look is `Some(why)` and this
host would otherwise have claimed an open run, the round writes
`declined[host] = why` where it differs from what is there, and logs
one `Event::Declined { combination, system, host, why }` with it; a
round whose reason is unchanged writes nothing. A host that later claims
the run removes its own entry in the same write as the claim. The host
name is the one claims use (`peer.name`).

`CandidateView` carries each open run's `declined`, so
`progress::testing_line`, which both `sluice status` and `sluice wait`
print, adds `waiting for a host whose container engine answers: <host>:
<why>` to a combination with an open run that some host turned down, and
the JSON of both carries `waiting` with the same hosts and reasons.

Alternative: the host's own state, as 133's read refusals keep theirs.
Rejected: a status on another host must see it (core.11), and the run's
record is where every reader already looks for the run.

### D5. After a failed run the core asks again and turns fails into tests with no result

When the core takes a finished `RunReport` of a branch whose limits ask
for the engine, in `Step::finish_run` for a queue run and in
`Sluice::finish_session` for a session run of the tests, and the run was not ended by its
time limit and has a test that did not pass (a `Fail` or a `missing`
test), it asks `unmet` again. Where it answers `Some(why)`, every
`Fail` task and every missing test moves into `not_run` with the reason
"the container engine no longer answers after the run: <why>", and the
report is read from there as 114's are: no key written, nobody blamed,
the combination tested again, 115's count of runs in a row applying. A
task that passed is left as it is. The look is made once, where the
report is first taken, so a report read again reads the same.

Doing this in the core rather than in the profile's `progress` keeps the
one seam question, lets the core skip it for a `--` command (D6), and
means a report read twice is decided once.

### D6. sluice exec asks before a session run of the tests, never before a command

Before `start_session` for the branch's tests, the session path asks
`unmet(&plan.policy.limits)`; on `Some(why)` `sluice exec` refuses,
printing `refused: this host's container engine does not answer: <why>;
<branch>'s tests ask for one ([executor] docker = true)` as a refusal
(`Kind::Refused`, exiting as `sluice exec`'s other refusals do), nothing
started or recorded. A command after `--` is not asked about, and
D5 is not applied to its run, as `ready` and `not_run` are dropped for
it today.

### D7. The time limit reads a declined run as not passed

The lead's time-limit pass reads an open run as timed out when it is
`!ours(r, home)` or this host's name is in its `declined`. The notice its
session gets says its tests waited for a host whose container engine
answers, naming the hosts and reasons from `declined`, in place of the
system wording.

## Risks / Trade-offs

- [A socket that answers `200` but is no Docker engine] → nothing else
  listens on those paths in practice, and the record asks for an engine
  that answers, which is what `/_ping` asks.
- [A two-second timeout per socket slowing a round] → only branches
  asking the engine with an open run this host could claim look, once a
  round, and the first answer ends the look.
- [Every host lacking an engine leaves the combination open until its
  time limit] → that is 87's shape the record chose; status says why
  meanwhile.
- [The other unit edits `checks.rs`, the configuration reference and
  the verdicts reference at the same time] → this change adds functions
  and a row of words and edits nothing of `ready`; the second to merge
  rebases.

## Migration Plan

A record without `declined` reads as empty. A host on an older sluice
claims as before and writes no `declined`; one on this sluice reads its
records unchanged. Nothing to undo: removing the change makes every host
claim as today.
