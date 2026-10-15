# Proposal

## Why

A person who wants to check a bolt's work running needs it running
somewhere. Today that is a worktree of the bolt's branch, kept checked
out and current, with its stack brought up by hand on whatever ports
are free. Keeping a worktree per branch is upkeep the fleet means to
drop, and a stack brought up by hand gets none of the ports, bound,
executor or release every other run on the host gets. The design is
drafted as `design/sluice/proposals/previews-1.md` in
agentplot/blueprints, clauses 98 to 103 and scenarios S30 to S35; this
change builds it.

## What Changes

- `sluice preview <branch> <revision>` makes a preview: a detached
  checkout at a revision the branch has landed, given its ports and
  caches as a candidate's checkout is, in which the policy's
  `[preview] command` runs once under the executor and the branch's
  limits, and hands off what it starts (98). A revision the branch has
  not landed is refused (S35).
- Each port name gets a name as well as a port: one flat DNS label, at
  most 63 characters, built from the port name, the revision, the
  branch and the repository, unique on the host. The policy's
  `[preview] addresses` are templates over the names and ports; sluice
  prints them filled in as soon as the preview is asked for (99). The
  command receives `SLUICE_NAME_<NAME>`, `SLUICE_BRANCH` and
  `SLUICE_REVISION` beside `SLUICE_PLACE` and `SLUICE_PORT_<NAME>`.
- Asking again for a revision with a preview returns it unchanged (100).
- `sluice previews` lists the host's previews with branch, revision,
  addresses, state and when each was made. `sluice release <preview>`
  releases one: the release command, everything ended, the checkout
  removed, ports and names let go. sluice releases a preview on its own
  only when its checkout is gone or its command failed or timed out
  (101).
- The preview command counts against the host's bound while it runs and
  never takes the reserve; a preview held after its command ended counts
  against nothing (102). Nothing of a preview is recorded as a result,
  read by a gate, or waited on by a submission (103).
- Site: a new guide, "Previews", for a caller such as crew or flywheel:
  what a preview is for, what it is not (a deployment with production
  parity), the lifecycle against landings, names under a `.localhost`
  proxy; the commands, configuration and state references; the home
  page's cards and the README gain previews.
- Rules and decisions: ADR 0021; core.16 and core.10 rewritten; a new
  rule that a preview decides nothing and is made only at a landed
  revision.
- The design book in agentplot/blueprints gains a chapter on previews,
  as `pull-requests.md` covers a proposal under review.

Out of scope: deploying a revision into a real environment, which is a
delivery tool's and not sluice's (the proposal's non-goal); previews on
other hosts or in a container place, which wait on the many-hosts work
(`many-hosts-2.md`, C.8); a cap on how many previews a host holds, left
to the caller; deciding when a preview is made or released, which is
the caller's.

## Capabilities

### New Capabilities

- `preview`: a running copy of a landed revision: making or returning
  it, its names and addresses, its command under the bound and the
  executor, its state, and the refusal of a revision the branch has not
  landed.
- `preview-release`: ending a preview at its caller's asking, listing
  the host's previews, and the cases sluice releases one on its own.

### Modified Capabilities

None.

## Impact

- `crates/sluice/src/profiles/places.rs`: a third place kind,
  `preview`, its record, names and addresses; `crates/sluice-seams`:
  `Checkout` gains preview methods; `crates/sluice-fakes` implements
  them.
- `crates/sluice/src/lib.rs`: `Command::Preview`, `Command::Previews`,
  `Release` takes a preview's name; a new `preview.rs` beside `exec.rs`.
- `crates/sluice/src/sessions.rs`: a preview's command counted with the
  host's session runs.
- Policy: a `[preview]` table, read from the previewed revision.
- `crates/sluice-conformance`: S30 to S35.
- Site, README, `docs/adr/`, `docs/architecture/core.md` and its site
  mirror; agentplot/blueprints `books/sluice/`.
