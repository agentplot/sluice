# Proposal

## Why

sluice runs on one host. An agent can't submit and close its laptop:
the queue moves only while some sluice on that host is running, the
checks run only there, and a shared branch's gate can't trust a run
from any other machine, because nothing proves who made it. A
repository that ships for several systems can't require a check on
each of them. The statement in agentplot/blueprints,
`design/sluice/requirements.md`, now has sluice on many hosts as peers alone, with no
central server, runs counted only through signers the policy names,
systems per check, and the operator's own standing executors before
any pool such as GitHub Actions. This change builds it on the
operator's own hosts, signing with SSH keys, with the guides that make
it usable on day one. GitHub Actions as a pool, keyless signing and the
pull request's checks per system follow in the `actions-pool` change.

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
  envelope signed with an SSH key the policy lists (78, 79). The
  policy also accepts a `github` identity, whose records count once
  `actions-pool` verifies them. One host
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
  only when no standing executor able to take it has room (88, 107):
  the check seam gains a launched side, and the GitHub profile
  dispatches the pool's workflow and reads whether its job still runs.
  A submission to a branch whose records are on its remote wakes the
  first pool (111).
- **Guides, in this change.** A guide to executors and systems (one
  host, a warm executor on a box, signers and what each may claim,
  choosing systems per check, the overview of hosts); the parallel and
  batch guide's section on how systems multiply runs; the configuration
  and state references; the setting-up and using skills; and the
  README.
- ADRs: signers replace the cache's write credential as trust (amends
  0004); one deployment, peers (amends 0008); a branch with no remote
  trusts its own host's unsigned runs (amends 0015).

Out of scope, each its own later change: GitHub Actions as a pool, with
keyless signing, `sluice init actions` and the pull request's checks
(`actions-pool`); signing the verdict a landing
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
  places before it claims; the policy's pools, in order, after the
  standing hosts.
- `run-signing`: signers in the policy, signed run records and their
  claims, verifying them from the records alone, and replays counted
  through signed passes.
- `check-systems`: systems in the policy, systems per required check,
  and reading a candidate's verdict over every check on every system.
- `standing-executor`: a host's own settings making `sluice run` an
  executor of named systems that watches the records and claims runs.

### Modified Capabilities
- `remote-branch`: sluice pushes its records, submitted heads and open
  combinations to the remote under `refs/sluice/`, beside landing
  commits.

## Impact

- `crates/sluice-core`: the records seam gains the remote refs profile;
  step and gate read signed runs, systems and claims; policy parsing for
  signers, systems, pools, `confined`; `trust = ["cache"]` and kinds on
  remote branches refused with a message naming the change.
- `crates/sluice`: the moon profile records each run's system; a
  signing module (DSSE, SSH signatures) behind the executor; the
  GitHub profile gains the pool's dispatch and job status; `sluice run`
  gains standing-executor mode.
- Dependencies: `ssh-key` for SSH signatures; DSSE and in-toto are
  written in `crates/sluice`.
- Policies: switchboard-kit's and sluice's own `.config/sluice.toml`
  move from executor kinds to signers when they gain a remote.
- Site, README and both plugin skills, as listed under guides.
