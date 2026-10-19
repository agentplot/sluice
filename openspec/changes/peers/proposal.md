# Proposal

## Why

sluice runs on one host. An agent can't submit and close its laptop:
the queue moves only while some sluice on that host is running, the
checks run only there, and a shared branch's gate can't trust a run
from any other machine, because nothing proves who made it. A
repository that ships for several systems can't require a check on
each of them. The design is drafted as `design/sluice/proposals/many-hosts-3.md`
in agentplot/blueprints: sluice on many hosts as peers alone, with no
central server, runs counted only through signers the policy names,
systems per check, and the operator's own standing executors before
any pool such as GitHub Actions. This change builds it, with the
guides that make it usable on day one.

The change waits on the operator's ruling on `many-hosts-3.md`: no code
lands before its clauses are in the statement (core.12).

## What Changes

- **Records on the remote.** A branch with a `remote`, in a policy
  with a `[signers]` table, keeps its
  decisions under `refs/sluice/state/<branch>`, written with a leased
  push, and each signer's run records under `refs/sluice/runs/<signer>`,
  added to only by that signer. Submitted heads, open combinations,
  run logs and each host's sign of life live beside them (C.6 remote
  refs; 80, 82, 109). A branch with no `remote` keeps local files and
  behaves exactly as today. **BREAKING** for the `remote-branch`
  requirement that sluice pushes nothing but landing commits.
- **Peers.** Any instance keeps a branch by writing its records and
  takes one after the deployment's lease (72 to 74). Runs are taken by
  claims, earliest first, up to each host's bound, and lapse with the
  holder's sign of life (75 to 77). A returning host releases its
  places before it claims (81). A waiting `sluice submit` runs rounds
  while its host is up; its host stopping loses nothing (111).
- **Signers.** The policy names signers and what each may claim:
  executor kind, confinement, systems. A run counts only when a signer
  the gate trusts signed its record, an in-toto statement in a DSSE
  envelope; a forge job signs keylessly with its workload identity,
  any other signer with an SSH key the policy lists (78, 79). One host
  with local files signs nothing. `trust` lists signer names, and
  `confined = true` replaces `trust = ["confined"]`. A replay counts only
  when a trusted signer signed a pass for its key and system;
  `trust = ["cache"]` goes (108). **BREAKING** for policies that list
  executor kinds under `trust` on a branch with a `remote`.
- **Systems.** The policy names systems; a branch names the systems
  each required check must pass on (`systems`, `systems_for`). A run
  records its system, the gate reads every check on every system named,
  and a check with none named takes one pass from any system (85 to 87).
- **Standing executors and pools.** `sluice run` on a host whose own
  settings make it a standing executor watches the records and claims
  runs of its systems. A pool the policy lists, in order, gets a run
  only when no standing executor able to take it has room (88, 107).
  The GitHub Actions pool starts one job per run on the runner its
  system names; a job may also run a round. Work no live instance will
  take is woken through the pool's dispatch, with a schedule as the
  backstop (111).
- **`sluice init actions`** writes the workflow, checks the app's
  permissions and the signer, and `sluice forge check` warns when the
  workflow and the policy disagree.
- **Pull requests.** A run on a pull request's head, by a trusted
  signer, is recorded under its keys and systems, and a candidate that
  shares a key reads it without running again (113). The `sluice`
  check run stays, and sluice posts one more per required check per
  system, with its outcome and signer. `pull_requests.checks` names
  other systems' checks only.
- **Guides, in this change.** A guide to executors and systems (one
  host, a warm executor on a box, a pool, signers, choosing systems per
  check); a guide to the GitHub Actions path (`sluice init actions`,
  the app, the signers, submit and close the laptop, reading the pull
  request's checks); the parallel and batch guide's section on how
  systems multiply runs; the configuration reference, the setting-up
  and using skills, and the README.
- ADRs: signers replace the cache's write credential as trust (amends
  0004); one deployment, peers (amends 0008); a branch with no remote
  trusts its own host's unsigned runs (amends 0015).

Out of scope, each its own later change: signing the verdict a landing
carries for releasers and deployers (waits on this change's signers);
a pool on a forge other than GitHub; reusing a key proven in another
repository; `sluice exec --remote` (session-runs-1, 96), which waits on
this change's remote submit.

## Capabilities

### New Capabilities
- `remote-records`: a branch's records on its remote: the decisions
  ref and its leased writes, each signer's run ref, heads, combinations,
  logs and hosts' signs of life, and reading them from any host.
- `peer-lead`: keeping and taking a branch by writing its records, the
  deployment's lease, and rounds run by any instance.
- `run-claims`: claims on runs, earliest first, up to a host's bound;
  lapsing with the holder's sign of life; releasing a returning host's
  places before it claims.
- `run-signing`: signers in the policy, signed run records and their
  claims, verifying them from the records alone, and replays counted
  through signed passes.
- `check-systems`: systems in the policy, systems per required check,
  and reading a candidate's verdict over every check on every system.
- `standing-executor`: a host's own settings making `sluice run` an
  executor of named systems that watches the records and claims runs.
- `actions-pool`: GitHub Actions as a pool: the workflow `sluice init
  actions` writes, one job per run on its system's runner, rounds run
  in a job, the wake and the schedule.

### Modified Capabilities
- `remote-branch`: sluice pushes its records, submitted heads and open
  combinations to the remote under `refs/sluice/`, beside landing
  commits.
- `pull-request-intake`: runs on a request's head count toward its
  candidate; `checks` names other systems' checks.
- `pull-request-status`: one check run per required check per system,
  beside the `sluice` check run.
- `forge-identity`: `sluice forge check` covers the pool's workflow,
  the app's permission to dispatch it and to write `refs/sluice/`, and
  the forge signer.

## Impact

- `crates/sluice-core`: the records seam gains the remote refs profile;
  step and gate read signed runs, systems and claims; policy parsing for
  signers, systems, pools, `confined`; `trust = ["cache"]` and kinds on
  remote branches refused with a message naming the change.
- `crates/sluice`: the moon profile records each run's system; a
  signing module (DSSE, SSH signatures, sigstore keyless) behind the
  executor; the GitHub profile gains the Actions pool, dispatch, check
  runs per check and system, and `sluice init actions`; `sluice run`
  gains standing-executor mode.
- Dependencies: a DSSE and in-toto implementation and sigstore's
  verification, both in Rust, chosen in the design.
- Policies: switchboard-kit's and sluice's own `.config/sluice.toml`
  move from executor kinds to signers when they gain a remote.
- Site, README and both plugin skills, as listed under guides.
