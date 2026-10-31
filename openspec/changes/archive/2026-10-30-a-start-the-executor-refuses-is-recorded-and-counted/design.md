# Design

## Context

Clauses 16, 84, 87, 88, 114, 115 and 139 of the statement, and record
0074 as amended 2026-10-30, govern this change; rules core.7, core.14,
core.35, core.37 and core.38 bind it. The archived change
`an-executor-with-no-container-engine-refuses-a-run-that-asks-for-one`
built the engine look this change extends; its decisions D1 to D7 stand
where this one does not say otherwise.

What main does today (the round is `crates/sluice-core/src/step.rs`):

- A run is claimed by the lead's `decide` (`choose_starts`) or a peer's
  `execute`, which set it `Starting` with a place and write the records;
  `perform_starts` (631-736) then calls `checkout.materialize` and
  `tests.start`. `start` (`crates/sluice/src/profiles/checks.rs`
  983-1165) runs `choose` (84's refusal of a limit), `[tests]
  toolchain`, the runner's plan (`moon.rs` 51-146: install, version,
  `moon query tasks`, `moon setup`), writes the sandbox profile and
  spawns the supervisor.
- Every error from `materialize` or `start`, `AtBound` aside, returns
  out of `perform_starts` with `?`. Nothing of it is written. The lead's
  next `recover` (295-335) releases the place and reopens the run but
  leaves `host` set, so on a signed branch `may_claim`'s
  `run.host.is_none()` (2472) keeps every host off it; on an unsigned
  branch the next round claims it again and makes `<n+1>`, without end.
  A peer's run stays `Starting` and its place is retried under the same
  name each round.
- 114 and 115 are built for a report that comes back: `finish_run`
  (1345-1380) counts `not_run_runs` against `NOT_RUN_BOUND = 3`, reopens
  the run with `host = None` and logs `Event::Lost { not_run }` under the
  bound, and past it adds each not-run check to `failed` with its
  reason.
- The look before a claim is `Unmet` (97-118) asked once a round through
  `meets` (2407-2422), gating `claimable`; `Tests::unmet` answers only
  when `limits.asks_of_host()`, which is `docker` alone. A host that
  declines writes `SystemRun.declined` and `Event::Declined` once per
  host and reason (`note_declined`, 2427-2465); `poll` reads a declined
  run as timed out at the time limit with `waited_for` (3137).
- `docker_sockets` (checks.rs 630-662) takes `DOCKER_HOST` only when it
  is `unix://`; a `tcp://` address is passed to the run's environment
  and neither looked at nor opened in the Seatbelt profile.
- A pool's job is dispatched and polled (`profiles/github/actions.rs`);
  nothing in this crate runs its checks yet. The change `actions-pool`
  plans `sluice job run --claim <c> --branch <b>`, which makes the
  claimed run's checkout and runs its checks, and `sluice job record`,
  which reports the run under its claim as a standing host reports its
  own.

Vendor references read for this design:

- Docker Engine API v1.47, `GET /_ping`: answers `200` with body `OK`
  and an `Api-Version` header on any transport the engine serves.
- Docker CLI environment variables
  (docs.docker.com/reference/cli/docker/#environment-variables):
  `DOCKER_HOST` is `unix://path`, `tcp://host[:port]`, `ssh://…` or
  `npipe://…`; `DOCKER_TLS_VERIFY`, when set to anything, makes the
  client use TLS and verify the engine; `DOCKER_TLS` makes it use TLS;
  `DOCKER_CERT_PATH` (default `~/.docker`) holds `ca.pem`, `cert.pem`
  and `key.pem`. A `tcp://` address with no port is 2376 under TLS and
  2375 without.
- ureq 3 (the workspace's HTTP client, built with rustls):
  `tls::TlsConfig::builder().client_cert(..).root_certs(..)`,
  `ClientCert::new_with_certs`, `RootCerts::new_with_certs` and
  `tls::parse_pem` give TLS with a client certificate and a named CA.
  The lockfile holds 3.4.2; the coder confirms these on it.
- Seatbelt: `(remote tcp "<host>:<port>")` accepts `*` or `localhost`
  as its host and nothing else; a name is resolved through
  `/private/var/run/mDNSResponder`, a unix socket.

## Goals / Non-Goals

**Goals:**

- One path from "a claimed run" to "its tests running or its tests not
  run, with why", so a standing host's round, a peer's claim and a
  pool's job end the same way and are counted the same way.
- The look before a claim answers for everything the executor reads of
  the host: its kind and limits as `choose` reads them, and the engine.
- The engine is read where the run would reach it, `DOCKER_HOST`'s TCP
  address included.

**Non-Goals:**

- `sluice job run` itself (the change `actions-pool`).
- The sandbox's refusals read from the unified log and the grant in a
  signed run (the change `a-checks-key-holds-its-executor`).
- A session's `sluice exec` whose tests could not be started.
- `ssh://` engines and Docker contexts.

## Decisions

### D1. A start that fails is a report with every test not run

`perform_starts` keeps the bound's refusal (`AtBound`) as today: the run
is reopened, nothing counts. Every other error from making the checkout
through `tests.start` becomes a `RunReport` with no tasks, no missing
tests, `executor` as `choose` would have named it (`Enforced::default()`
where `choose` itself refused), and `not_run` holding each of the run's
checks (`r.checks`, or the gate's required checks where the run names
none) with one reason: `could not start: <error>`, the error written
with its whole chain (`{:#}`), what a failing program printed included,
its last 2 KiB where it printed more. The place, where one was made, is
released as `AtBound`'s path releases it, a failed release kept on
`line.doc.release` for later rounds.

That report is read where every report is read: the not-run reading of
`finish_run` moves into a function both call, taking the line, the
candidate, the run's index, the report and the time, and no run handle.
So under the bound the run is open with `host = None`, its place
released and `Event::Lost { combination, not_run }` logged; past it,
each check is in `failed` with its reason and the verdict is not
passed, attributed as any failure (17, 18, 20). Past the bound with no
run handle, no logs are read: the reason is what the session is told.

*Alternative:* a new event for a start that failed. Rejected: 114 makes
a check that could not be started the same as one that could not run,
and every reader of `Event::Lost` and of `FailedTask.not_run` already
shows it.

*Alternative:* leave the claim with the host and retry. Rejected: that
is what loops today, and it keeps a signed branch's run from every
other host.

### D2. `unmet` answers for the executor's limits first

`Tests::unmet(&Limits)` stays the seam's one question. The check
profile answers `choose(limits)`'s refusal, in its words, before it
looks at the engine, so a branch whose limits the executor cannot hold
costs no socket. `Limits::asks_of_host` goes: every branch is asked,
and for one with no engine the answer is `choose`'s, which reads
nothing of the host beyond the platform sluice was built for. `Unmet`
still asks at most once a round per branch, the first time `claimable`
would be true, so an idle tick asks nothing (core.31).

`choose` stays in `start` as well, so a run started without the look
(D4) is refused by the same words.

*Alternative:* a second seam question for limits. Rejected: the core
does the same with either answer, and one question keeps the round's
one look.

### D3. The engine's words say it is the engine

Since a declined run now waits for an engine or a limit, the engine's
reasons begin with what is missing: `no container engine answers: <what
each socket or address said>`. The waiting line, which `sluice status`
and `sluice wait` print (`progress::waiting`), becomes `waiting for a
host that can run its tests as its policy asks: <host>: <why>; …`, and
`waited_for`'s notice `no host that could run its tests as its policy
asks took them: <host>: <why>; …`. The JSON keeps its `waiting` field
and shape.

### D4. A start looks where the round has not

`perform_starts` asks the round's `Unmet` for the branch before
`materialize`, asking the seam if this round has not yet. A run this
host claimed this round was claimed on a met answer, so the cached
answer costs nothing. A run left `Starting` from an earlier round, or
one started under a claim another process took, a pool's job's above
all, is read afresh. On `Some(why)` no checkout is made and D1's report
is built with the reason `<why>` (no `could not start:` prefix: the
executor's own report of what it cannot meet, 139), counted as D1's.

The core exposes this start of a claimed run, the look, the checkout,
the tests' start and D1's report, as the one function a pool's job
starts its run by, so `sluice job run` (the change `actions-pool`) needs
nothing of its own to refuse at its start; until that command is built,
the conformance suite drives the function under a pool's claim with the
fake check seam.

*Alternative:* the job reports a refusal in its signed record. Rejected
for this change: no test result is recorded for a run with no outcome,
so there is nothing to sign, and `sluice job record` reports under its
claim through the core as a standing host does.

### D5. The engine at a TCP address, as the run's client reaches it

`docker_sockets` becomes `engine_endpoints`, a list of `Endpoint::Unix
(PathBuf)` and `Endpoint::Tcp { host, port, tls }`. `DOCKER_HOST`
parsed as `tcp://host[:port]` gives a `Tcp` endpoint, `tls` set when
`DOCKER_TLS_VERIFY` or `DOCKER_TLS` is set, the port defaulted as the
client does; `unix://` gives a `Unix` endpoint as today; any other
scheme gives nothing, as today. The usual unix sockets follow, as
today. `SLUICE_ENGINE_SOCKETS` still replaces the list for tests, and
gains `tcp://` entries.

The look pings each endpoint in order, the first `200` ending it:
unix sockets as today; a `Tcp` endpoint with `GET /_ping` through ureq,
plain HTTP without `tls` and HTTPS with it, the client certificate and
key from `DOCKER_CERT_PATH`'s `cert.pem` and `key.pem` where they are,
and the engine verified against its `ca.pem` under `DOCKER_TLS_VERIFY`
and not verified under `DOCKER_TLS` alone, as the client does. The two
seconds to connect and four to answer hold for TCP as for unix sockets.
Each endpoint's answer goes into the reason: refused, timed out, a
TLS error, or the status it gave. The look is one try: it asks the
host's engine, not a remote, so the retry policy (core.29) does not
apply, as for the unix sockets.

The sandbox profile opens a `Tcp` endpoint for a run of `docker = true`
whatever the network: `(allow network-outbound (remote tcp
"localhost:<port>"))` where the host is `localhost`, `127.0.0.1` or
`::1`, otherwise `(remote tcp "*:<port>")`, the narrowest Seatbelt
takes; where the host is a name and not an address, the resolver's
socket `/private/var/run/mDNSResponder` is opened too. A run that
reaches an engine is never confined (C.3), so the wider rule changes no
confinement.

*Checked 2026-10-30.* ureq 3.4.2, the version `Cargo.lock` holds, has
every TLS call this decision names: `TlsConfig::builder`, `client_cert`
(taking an `Option<ClientCert>`), `root_certs`,
`ClientCert::new_with_certs`, `RootCerts::new_with_certs` and
`parse_pem`; `disable_verification` serves `DOCKER_TLS` alone. On macOS
27.0.1 (26A434), `sandbox-exec` accepts `(remote tcp "*:<port>")` and
`(remote tcp "localhost:<port>")`, and refuses both an address
(`127.0.0.1:2375`) and a name (`build-engine:2376`) with "host must be *
or localhost in network address". Under a profile that denies all
other outbound traffic, a `localhost:<port>` rule lets a run reach an
engine on that port at `127.0.0.1` and at `::1` alike, and neither on
any other port, so an engine at `::1` takes the `localhost` rule as
above.

*Alternative:* a TCP connect alone. Rejected: record 0074 asks for an
engine that answers, and a port something else holds is no engine.

## Risks / Trade-offs

- [A host-wide fault counted against a combination] A host whose disk
  is full fails every start; three runs in a row read the combination
  as not passed. → That is 115's bound; another host may take each
  reopened run, and the reasons say it was the host.
- [A combination that breaks its own toolchain gets three runs, not
  one] → 114 makes a start that failed no outcome; two more starts cost
  seconds, and the session is told the toolchain's words.
- [`*:<port>` lets a run reach any host on the engine's port] → only for
  `docker = true`, which is never confined; a run that reaches an engine
  can already reach anything the engine can.
- [The other in-flight changes edit `checks.rs`, `step.rs` and the
  verdicts reference] → this change edits the start path, the look and
  the engine's endpoints, none of the grant or the denials; the second
  to merge rebases.

## Migration Plan

Records are unchanged in shape: `Event::Lost`, `FailedTask.not_run`,
`not_run_runs` and `declined` are read as they are. A host on an older
sluice keeps failing a start without a record; one on this sluice
records it. The waiting words change, the JSON does not. Removing the
change returns to today's behaviour.
