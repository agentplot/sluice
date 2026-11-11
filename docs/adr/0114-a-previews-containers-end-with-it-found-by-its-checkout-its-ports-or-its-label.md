---
number: 114
title: A preview's containers end with it, found by its checkout, its ports or its label
status: proposed
date: 2026-11-11
decision-makers:
- the construct stage of change/a-previews-containers-end-with-it
---

# A preview's containers end with it, found by its checkout, its ports or its label

Status: proposed. This is the construct stage's own decision within clause 34, previews-1 clause 101 and rule core.10, which it keeps. The statement's amendment is on the agentplot/blueprints branch `design/sluice-a-previews-containers-end-with-it` at e3433e8, marked proposed under this record.

Deciders: the construct stage of `change/a-previews-containers-end-with-it`, cut from `bolt/what-a-place-starts-ends-with-it`.
Sources: signal `2026-11-11-agentplot-operator-6799f524/01`, from `2026-11-11-wldn-ops-d900f68d/01`; `agentplot/blueprints:design/sluice/requirements.md` clauses 31, 34 and 123; `design/sluice/proposals/previews-1.md` clause 101 and its C.1 preview row; records 0016 and 0074; rule core.10; the OpenSpec change `a-previews-containers-end-with-it` and its `preview-release` delta; the Docker Engine API v1.47.

## Context and problem statement

A container a preview's command starts is handed to the host's container engine, outside the command's process group, so ending the group never reaches it. Record 0016 gave the end of what a checkout's checks hand off to the repository's release command alone. A preview is let go in four ways: released at its caller's asking, released after its command failed, freed because its checkout is gone, and freed because the operator accepted its release's failure (123). In the third, the stored release command runs once in the scratch directory, where a script of the checkout cannot be found. In the fourth, nothing runs. Either way sluice frees the preview and the container keeps running.

On 2026-11-11, on one host, the container `floci-stand-15162`, an emulator of about 1 GB with its TLS mounted from a preview's checkout under `sluice/.previews/`, had run since 2026-11-09 while `sluice previews` answered that there were none. The repository's release command was a script in the checkout.

Clause 34 has a release end everything started in the checkout, what its runs handed to another process to keep included, and rule core.10 has that removed when the checkout is released. No other tool of the family ends a checkout's containers: Flume, Boomdrive, weir, timber and boom name none, and worktrunk's tether is a session's, outside sluice. So sluice owns it. What should end a preview's containers, and how does sluice know which are its?

## Decision outcome

### 1. The check profile ends a checkout's containers, handed to the `Places` profile by the binding

The engine is the check profile's, which already reaches it for record 0074's look before a run. The `Places` profile knows when a preview is let go and still holds its ports. So the check profile builds `end_containers(endpoints, place)`, in `crates/sluice/src/profiles/checks/engine.rs`, and the binding hands it to the `Places` profile as a `ContainerEnd` through `Places::with_containers`, as it hands the release command's builder through `with_release`. Neither profile calls the other (rule core.2). A `Places` given none ends nothing.

Not taken: a method on the `Tests` or `Checkout` seam that the core calls around the `Checkout` seam's release. The core would call it either before the release command, which the repository's own teardown must precede, or after the ports were let go, when another checkout may already hold one of them. Only the `Places` profile stands between the two.

### 2. A container is the preview's by its mounts, its ports or its label

A container is the preview's when the engine reports one of these, and for nothing else:

1. A mount's `Source` is the preview's checkout or scratch directory, or lies beneath one, compared by whole path components, each path as given and as it resolves where it still exists, so `/tmp` and `/private/tmp` on macOS are one.
2. One of its ports is published on the host at one of the preview's `SLUICE_PORT_<NAME>` values. While the preview holds its block, no other checkout on the host holds those ports (31).
3. Its label `sluice.place` equals the preview's `SLUICE_PLACE`.

No name, image, creation time or compose project is read. The signal's container matches on the first and the second. The label is for a container that mounts nothing of the preview's and publishes no port, such as one reached only on a network of the engine's. sluice's own stack tests already label their containers so.

Not taken:
- The release command alone, record 0016's way. It cannot run in two of the four ways a preview is let go, and those two are where the signal's container came from.
- A socket of sluice's own between the run and the engine, labelling each container as it is created. That is a service sluice would run (31; rule core.10), and a process the command handed off would outlive it.
- The engine's containers after the command less those before it. Other checkouts' runs start containers at the same time, so the difference would take theirs.
- A compose project name set for the run. It names one tool and covers only what that tool starts.

### 3. The end runs after the release command and before the ports go

In a release, the run is ended and its release command run first, then the containers are ended, then the run's directory and the checkout are removed and the scratch, the block of ports, the names and the record are let go. In a free, the run is ended with nothing run in its name, then the containers are ended, then the run's directory, the checkout and the rest go. The run's directory stays until the containers have ended, so a preview a container keeps still reads its run, and a release command that finished is marked so there and never runs again. A gone preview's free follows its stored release command's one run. The release command has its turn first, which matters where it tears down a deployment on an emulator before the emulator goes, and no other checkout can hold one of the preview's ports while the end looks for them. Only a preview is passed. A combination's checkout and a session's are left to their release commands.

Not taken: the end before the release command. The repository's teardown may need the containers it tears down.

### 4. Each engine that answers is asked, once, and each call is bounded

The end asks every endpoint where the executor looks for an engine before a run that asks for one (record 0074): `SLUICE_ENGINE_SOCKETS`, else `DOCKER_HOST`, and the usual sockets. It asks whatever the branch's `[executor] docker` says, since an open executor reaches the engine without it. It passes over a unix socket that does not exist, one whose resolved path it already asked, and one that does not answer `GET /_ping` with `200`. An engine that answers is asked `GET /containers/json?all=true`, and each container of the preview's is removed with `DELETE /containers/{id}?force=true&v=true`. The removal is forced, since the release command already had its turn to stop it, and takes the container's anonymous volumes with it. `204` and `404` count as removed. Each call is one try, since the engine is the host's and no remote (rule core.29). Connecting waits as the executor's look waits, a listing answers within ten seconds and a removal within thirty, and a listing is read to at most 16 MiB. A unix socket is asked in HTTP/1.0 to close the connection, and each answer is read to the end it gives itself, its `Content-Length`, its last chunk or a status that carries no body, and to the connection's close only where it gives none. An engine may hold the connection open once it has answered whatever was asked: one host's did, 2026-11-11, answering in HTTP/1.1, and every listing there waited out its ten seconds and kept its preview. A host where no engine answers has nothing to end.

Not taken: asking only when the branch's policy says `[executor] docker = true`. The signal's repository runs an open executor and never says it. Not taken either: reading each answer to the connection's close, as HTTP/1.0 lets a server end one. The executor's look reads only the status line and so never meets a held connection, but a listing's body must be read whole.

### 5. A container left keeps the preview

Where an engine answers and a listing or a removal fails, the end goes on through every other container, then fails with `could not remove the containers <preview> left:`, each container's first name and the engine's words. The preview keeps its checkout, its ports, its names and its record. A release at the caller's asking, or after a failed command, keeps the failure as a failing release command's is: it is tried again each round up to the branch's release tries, then held (123), and `sluice release` exits 1. Its release command has already run, marked so in its run's directory, which stays, so a later try runs the end alone and the preview's state reads as before. A free, with the checkout gone or the failure accepted, fails before it lets anything go. The next listing or round frees the gone preview again, which tries the end again, and its stored release command never runs a second time.

Not taken: freeing the preview whatever the end did, as 123 frees a gone checkout whatever its release command did. A port a container still publishes would then be free for the next checkout's block, and the container would run on with nothing listing it, which is the leak this record ends.

### Consequences

- However a preview is let go, no container that mounts its checkout or scratch, publishes its port, or carries its `SLUICE_PLACE` outlives it.
- A container that does none of the three is the repository's to label with `sluice.place`. The site's previews guide and the plugins' previews references say so.
- A host with no engine, or whose engine is reached only through a `DOCKER_HOST` the releasing process lacks, releases as before.
- Named volumes and networks a preview's containers made stay. The engine removes a container's anonymous volumes with it, and nothing else.
- `sluice release` does not name the containers it ended. The profile traces each one.
