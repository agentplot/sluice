# Proposal

## Why

A container a preview's command starts is handed to the host's container
engine, outside the command's process group, so ending the group never
reaches it. Today only the repository's release command removes it, and
that command does not run in two of the four ways a preview is let go: a
preview whose checkout is gone has its stored release run once in its
scratch directory, where a script of the checkout cannot be found, and a
preview whose release failure the operator accepts runs nothing at all.
Either way sluice frees the preview and the container keeps running.

On 2026-11-11 on one host `floci-stand-15162`, an emulator of about 1 GB
with its TLS mounted from a preview's checkout
(`sluice/.previews/preview+2F2ec200ef1be6`), had run since 2026-11-09
while `sluice previews` answered that there were none (signal
2026-11-11-agentplot-operator-6799f524/01, from
2026-11-11-wldn-ops-d900f68d/01). The repository's release command,
`infra/ministack/release.sh`, is a path in the checkout.

A preview's release owes the end of everything started in it, what its
command handed to another process to keep included (34;
`agentplot/blueprints:design/sluice/proposals/previews-1.md` clause 101),
and rule core.10 says what a place's runs hand to another process is
removed when the place is released. No other tool of the family ends a
place's containers, so sluice owns this alone.

## What Changes

- Whichever way a preview is let go, released at its caller's asking,
  released after its command failed, freed with its checkout gone, or
  freed with its release's failure accepted, sluice asks the host's
  container engine for every container that is the preview's and removes
  each one. It does so after the release command, where one runs, and
  before the checkout is removed and the ports and names are let go.
- A container is the preview's when the engine says that it does one of
  these: mounts a path inside the preview's checkout or scratch
  directory, publishes one of the preview's ports on the host, or carries
  the label `sluice.place` set to the preview's `SLUICE_PLACE`. sluice
  leaves every other container alone.
- The engine is looked for where a run's engine is looked for before a run
  that asks for one: `SLUICE_ENGINE_SOCKETS`, else `DOCKER_HOST`, and the
  usual sockets (record 0074). Every endpoint that answers is asked. On a
  host where no engine answers there is nothing to remove, and the preview
  is released as it is today.
- If sluice cannot remove a container, the preview is kept, and what
  failed names the container and the engine's words. A release at the
  caller's asking or after a failed command fails as a failing release
  command does: it is tried again, then held, and `sluice release` exits
  1. A free with the checkout gone or with the failure accepted lets
  nothing go. The next listing or round tries the end again, and the
  gone checkout's release command does not run a second time.
- The site's previews guide and the plugins' previews references say that
  sluice removes what the release command left, and how a container with
  no mount or port of the preview is marked with `sluice.place`.
- The statement's clause 101 and the C.1 preview row of previews-1 say
  the same. Record 0112 and rule core.10's details hold the decision.

Out of scope:

- A candidate's place and a session's place. A candidate's release runs
  in its checkout as soon as its verdict is read, and no leak from it has
  been seen. A session's place may hold the session's own stack, which is
  its driver's to end (session-runs-1 94). Each waits on a unit of its own
  if a container is seen outliving one. The function built here takes any
  place.
- Named volumes and networks a preview's containers made. The engine
  removes a container's anonymous volumes with it, and nothing else.
- A container that does none of the three things above. It is the
  repository's to label.
- `wt step tether`'s stand outliving its removed worktree (the same
  signal's `ministack-14080`), which is worktrunk's, not sluice's.
- The sibling unit `an-unheld-worktrees-release-runs-its-release-command`,
  on the same bolt. It changes a session's release in a worktree, and
  this change touches nothing of it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `preview-release`: a preview's containers end with it, however it is
  let go. A container sluice cannot remove keeps the preview.

## Impact

- `crates/sluice/src/profiles/checks.rs` and a new
  `crates/sluice/src/profiles/checks/engine.rs`: the check profile asks
  the engine at each endpoint for its containers, picks the place's, and
  removes them.
- `crates/sluice/src/profiles/places.rs`: the place profile is handed the
  function that ends a place's containers and calls it for a preview in
  `release_preview` and `free`, so `release_gone` and an accepted failure
  reach it through `free`.
- `crates/sluice/src/profiles/mod.rs`: the binding hands that function to
  the place profile (rule core.2), as it hands the release command's
  builder.
- `crates/sluice/tests`: a fake engine that holds containers and records
  removals, and tests of the place profile and of the binary.
- Docs: the site's previews guide, the plugins' previews references,
  record 0112 and rule core.10's details. The statement in
  agentplot/blueprints.

## Touches

```
crates/sluice/src/profiles/checks.rs
crates/sluice/src/profiles/checks/engine.rs
crates/sluice/src/profiles/places.rs
crates/sluice/src/profiles/mod.rs
crates/sluice/tests/support/mod.rs
crates/sluice/tests/profiles/places.rs
crates/sluice/tests/previews_tools.rs
docs/adr/0112-a-previews-containers-end-with-it-found-by-its-checkout-its-ports-or-its-label.md
docs/architecture/core.md
site/src/content/docs/guides/previews.md
plugins/sluice/skills/using-sluice/references/previews.md
plugins/sluice/skills/setting-up-sluice/references/profiles/previews.md
```
