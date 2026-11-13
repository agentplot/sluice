# Design

## Context

Clause 34 (a place's release ends everything started in it, what its
checks handed to another process to keep included), previews-1 clause
101 and its C.1 preview row, clause 123 (a held place, a gone place, an
accepted failure) and rule core.10 govern this change. Record 0016 gave
the end of what checks hand off to the repository's release command
alone. Record 0074 gave the check profile its way of reaching the host's
container engine.

What exists on the bolt today:

- The place profile, `crates/sluice/src/profiles/places.rs`, lets a
  preview go in two functions. `release_preview(name, failed, retries)`
  ends its run and runs its release command (`self.runs.remove`), removes
  its worktree, its scratch, its block of ports and its record. It runs
  at the caller's asking (`Sluice::release_preview`) and for a failed
  command (`Sluice::release_unusable_previews`, with `failed`). `free(name)`
  ends its run with nothing run in its name (`self.runs.discard`), then
  removes the worktree and lets go of the scratch, block and record
  (`let_go`). It runs for an accepted failure (`Sluice::accept_preview`)
  and at the end of `release_gone`, which first runs the release command
  its run stored, once, in the scratch directory.
- The release command is the repository's. A script in the checkout,
  such as switchboard-kit's `infra/ministack/release.sh`, cannot be found
  from the scratch directory, so a gone preview's release fails and the
  preview is freed all the same (123). An accepted failure runs nothing.
  Neither reaches a container.
- The check profile, `crates/sluice/src/profiles/checks.rs`, reaches the
  engine already: `engine_endpoints()` (`SLUICE_ENGINE_SOCKETS`, else
  `DOCKER_HOST`, `/var/run/docker.sock` and the usual sockets of Docker
  Desktop, OrbStack and Colima), `docker_host` for a TCP address with its
  TLS, and `engine_unmet`, which asks `GET /_ping` over HTTP/1.0 on a
  unix socket (`ping`) or over `ureq` on TCP (`ping_tcp`).
- The binding, `crates/sluice/src/profiles/mod.rs`, hands the place
  profile the check profile's builder of a session's release command
  (`Places::with_release`, `checks::release_request`), so neither
  profile calls the other (rule core.2).
- A preview's place, as its command was given it, is
  `Places::preview_run_place(name, made)`: its checkout path, its scratch
  (`tmp`), and an environment holding each `SLUICE_PORT_<NAME>`, each
  `SLUICE_NAME_<NAME>` and its `SLUICE_PLACE`, `sluice-` and ten hex
  digits of its record's path.

The Docker Engine API (v1.47, served the same by Docker Engine, Docker
Desktop, OrbStack and Colima) has these calls. `GET
/containers/json?all=true` lists every container, running or not, each
with `Id`, `Names`, `Labels`, `Ports` (`PublicPort` where one is
published on the host) and `Mounts` (`Source`, the host path of a bind
mount). `DELETE /containers/{id}?force=true&v=true` kills a running
container, removes it with its anonymous volumes, and answers `204`, or
`404` when there is no such container. Other answers carry a JSON
`message`.

## Goals / Non-Goals

**Goals:**

- One function of the check profile ends a place's containers. The place
  profile calls it on every way a preview is let go, at the one moment
  the preview still holds its ports.
- A container is attributed to a preview only by what the engine reports
  of it and the preview's own record holds, never by a name, an image or
  a time.
- A container that stays running keeps its preview, and what failed says
  so wherever a failing release is said today.

**Non-Goals:**

- A candidate's place and a session's place (proposal, out of scope).
- Naming the containers ended in `sluice release`'s answer. That would
  change the place seam's release and free answers for every caller. The
  profile traces them, and the answer is left for when a caller asks.
- Volumes and networks beyond a container's anonymous volumes.

## Decisions

### D1. sluice owns the end, through the check profile, handed to the place profile

No tool of the family ends a place's containers. Flume, Boomdrive,
weir, timber and boom name none in their statements. worktrunk's
`wt step tether` is a session's, outside sluice. So sluice owns it.
The engine is the check profile's: it already reaches it for record
0074's look. The place profile knows when a preview is let go and still
holds its ports. So the check profile builds the function, and the
binding hands it to the place profile:

```rust
/// Ends what the engine holds for a place: the check seam's, handed to
/// this profile by the binding (rule core.2).
pub type ContainerEnd = Box<dyn Fn(&Place) -> Result<()>>;

impl Places {
    pub fn with_containers(mut self, end: ContainerEnd) -> Self;
}
```

The binding passes `Box::new(|place| checks::end_containers(&checks::engine_endpoints(), place))`.
A `Places` given none ends nothing, so tests of the place profile that
have no engine are unchanged.

Path not taken: a method on the `Checkout` or `Tests` seam that the core
calls around the place seam's release. The core would call it either
before the release command, which the repository's own teardown must
precede, or after the ports were let go, when another place may already
hold one of them. Only the place profile stands between the two.

### D2. A container is the place's by its mounts, its ports or its label

`end_containers(endpoints, place)` lists the containers at each endpoint
and takes a container as the place's when one of these holds:

1. A mount's `Source` is `place.path` or `place.tmp`, or lies beneath
   one, compared whole component by component. Each path is compared as
   it is given and as it resolves where it still exists, so `/tmp` and
   `/private/tmp` on macOS are one.
2. One of its `Ports` has a `PublicPort` equal to one of the place's
   `SLUICE_PORT_<NAME>` values. While the place holds its block, no
   other place on the host holds those ports (31).
3. Its label `sluice.place` equals the place's `SLUICE_PLACE`.

Nothing else is read: no name, image, creation time or compose project.
A pure function, `owned(container, place) -> bool`, over the engine's
JSON decides it, so it is tested with no engine.

The signal's container matches on 1 (its TLS mounted from the checkout)
and on 2 (its `floci-stand` port). The label is for a container that
mounts nothing of the place and publishes no port, such as one reached
only on a network of the engine's. It is the label sluice's own stack
tests already give their containers and remove them by
(`crates/sluice/tests/stack.rs`), so a repository that follows them
needs no change.

Paths not taken:
- The release command alone, record 0016's way. It cannot run in two of
  the four ways a preview is let go, and those two are where the signal's
  container came from.
- A socket of sluice's own between the run and the engine, which would
  label each container as it is created. That is a service sluice would
  run (31; rule core.10), and a process the command handed off would
  outlive it.
- The containers the engine holds after the command less those it held
  before. Other places' runs start containers at the same time, so the
  difference would take theirs.
- A compose project name set for the run. It names one tool and covers
  only what that tool starts.

### D3. The end runs after the release command and before the ports go

In `release_preview` the order becomes: `self.runs.release_keeping(name)`
(the run ended and the release command run, marked done in the run's
directory, which stays), then the end, then `self.runs.remove(name)`
(which runs no finished release again), `remove_worktree`, the scratch,
the block and the record. In `free` for a record of kind `Preview`:
`self.runs.end_keeping(name)`, then the end, then `self.runs.discard(name)`,
the worktree and `let_go`. The run's directory stays until the end has
succeeded, so a preview a container keeps still reads its run's state.
`release_gone` reaches the end through `free`. The release command thus
has its turn first, which matters where it tears down a deployment on an
emulator before the emulator goes. And no other place can hold one of
the preview's ports while the end looks for them. The place passed is `preview_run_place(name, &made)`, read
from the record before anything is let go. A place of any other kind is
not passed (proposal, out of scope).

### D4. Each engine that answers is asked, once, and its answers are bounded

`end_containers` takes every endpoint `engine_endpoints()` gives. That
is the same set the executor looks at before a run that asks for an
engine (record 0074), and the variable the tests set. The engine is
asked whatever the branch's `[executor] docker` says, since an `open`
executor reaches the engine without it, as switchboard-kit's does. Like
`engine_unmet`, it passes over a unix socket that does not exist and one
whose resolved path was already asked. An endpoint that does not answer
`GET /_ping` with `200` is passed over. An endpoint that answers is
asked `GET /containers/json?all=true`, and each container of the
place's is then removed with `DELETE /containers/{id}?force=true&v=true`.
The removal is forced because the release command already had its
graceful turn. `204` and `404` count as removed. Anything else is a
failure, kept with the container's first name and the engine's
`message`.

Unix sockets are spoken to in HTTP/1.0, as `ping` speaks, asking the
engine to close the connection, through a thread so that a socket that
never takes the connection holds nothing. A response is read to the end
it gives itself: its `Content-Length`, its last chunk, or a status that
carries no body. It is read to the connection's close only where it
gives none, since an engine may hold the connection open once it has
answered, whatever was asked (one host's did, 2026-11-11, answering in
HTTP/1.1). TCP endpoints go through `ureq` with the TLS
`engine_tls_config` gives, as `ping_tcp` does. Each call is one try,
since the engine is the host's and no remote (rule core.29). Connecting
is bounded by `ENGINE_WAIT`. A listing must answer within ten seconds,
and a removal within thirty. A listing is read to at most 16 MiB. The
code lives in `crates/sluice/src/profiles/checks/engine.rs`, a module of
the check profile declared from `checks.rs`, which makes `ping`,
`ping_tcp` and `engine_tls_config` visible to it within the crate.
`end_containers` is re-exported from `checks`.

When no endpoint answers, `end_containers` returns `Ok(())`: a host with
no engine has no container to end. It traces each container it removed,
`tracing::info!(sluice.place = %place.name, container = %name, "ended a
container the place left")`.

Path not taken: asking only when the branch's policy says `[executor]
docker = true`. The signal's repository runs `open` and never says it.

### D5. A container left keeps the preview

`end_containers` returns an error when an engine answered and a listing
or a removal failed. The error starts `could not remove the containers
<place> left:` and is followed by each container and what the engine
said, or the listing's failure. It goes on through every other container
before it fails, so one refusal does not leave the rest running.

- In `release_preview` the error is kept as a failing release command's
  is: `keep_failure(name, &e, |tries| tries > retries)` and `Err(e)`. The
  preview keeps its checkout, block, names and record. `sluice release`
  exits 1 with its existing words around the error, and a round tries
  again up to its branch's release tries, then holds it (123). Its
  release command finished and is marked so in the run's directory, so
  a later try runs the end alone.
- In `free` the error is returned before the worktree is removed or
  anything is let go. `accept_preview` fails with it, and `sluice release
  --accept-failure` exits 1. `release_gone` has already noted its start
  on the record, so a later `release_unusable_previews` calls `free`
  again with no command run, and that tries the end again. The listing
  that met the error reports it as it reports any failed release of an
  unusable preview.

Path not taken: freeing the preview whatever the end did, as 123 frees a
gone place whatever its release command did. A port a container still
publishes would then be free for the next place's block. That block's
check would pass over it, but the container would run on with nothing
listing it, which is the leak this change ends.

### D6. Tests reach a fake engine, never the host's

`support::containers_engine` is a fake engine on a unix socket, and a
TCP form of it. It answers `_ping`, lists the containers a test gives it
(fixed, or made at each listing where they depend on what the run
wrote), and removes them, refusing a removal as often as a test asks.
The check profile's tests hand it to `end_containers` directly. The
place profile's tests hand it through `with_containers`. The binary's
tests in `previews_tools.rs` name it with `SLUICE_ENGINE_SOCKETS` on
every `sluice` they run, so no preview test lists or removes a container
of the host's. `support::isolated` is left as it is: `stack.rs` needs
the host's real engine.

### D7. The statement, the record and the rule

The statement first, on the agentplot/blueprints branch
`design/sluice-a-previews-containers-end-with-it`. Previews-1 clause 101
gains the end and its three tests, and that a preview whose container
cannot be removed is kept. The C.1 preview row's release cell gains the
removal between the release command and `git worktree remove`.
"What it changes elsewhere" gains 123's two frees reaching it. One
scenario is added. Each is marked proposed, record 0121. The orchestrator
pushes the branch on the user's word, before the code lands (rule
core.12).

Record 0121, `docs/adr/0121-a-previews-containers-end-with-it-found-by-its-checkout-its-ports-or-its-label.md`,
status proposed, decided by this change's construct stage within 34,
101 and core.10. It holds D1 to D5, each with its path not taken. If
0121 is taken when the record is written, the next free number is used,
everywhere this change names it.

Rule core.10's details gain the case. Ruled out: a preview let go, by
any of its four ways, leaving a container the engine reports mounting its
checkout or scratch, publishing its port, or labelled with its
`SLUICE_PLACE` (one host, 2026-11-11: `floci-stand-15162`, signal
2026-11-11-agentplot-operator-6799f524/01). Also ruled out: its ports let
go before that end, and a container matched by name, image or time. Not
ruled out: a container matching none of the three, left alone; a host
whose engine does not answer releasing as before; a candidate's and a
session's place, whose release command alone ends what they hand off.
The rule's own line stays as it is.

## Risks / Trade-offs

- [A container another tool started that mounts a path under a preview's
  checkout] → only the preview's own stack has reason to. The checkout
  is sluice's, under `sluice/.previews/`, and the scratch is unique to
  the place.
- [An engine reached only through a `DOCKER_HOST` the releasing process
  lacks, while the preview's command had it] → that engine is not asked.
  The host process and a caller's command normally share the user's
  environment. Reading the run's own `DOCKER_HOST` from its request was
  left out to keep the end's input to the place alone.
- [An engine that hangs on a listing] → each call is bounded (D4), and
  the preview is kept with what failed rather than freed.
- [A removal that kills a container mid-write] → the release command has
  had its turn first, and a preview is the repository's stack on its
  fakes (previews-1, For the caller).
- [The sibling unit edits `places.rs` on the same bolt] → it touches a
  session's release in a worktree. This change touches `release_preview`
  and `free`'s preview branch. The second to merge rebases.

## Migration Plan

Nothing to migrate. A preview made by an older sluice has the same
record, and its containers are ended at its first release by this one.
Rolling back leaves the release command as the only end, as before.
