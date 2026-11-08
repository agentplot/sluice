## Why

Record 0093, accepted by the user on 2026-11-06 ("I approved 93."),
queues pull requests on the forge: a branch's requests are taken in
only where its records are on its remote, by the instance that keeps
the branch there, on a forge its jobs, one round at a time; a branch
whose records each host keeps takes no requests in; a host's submit
opens or updates its request, marks it ready and follows it there; and
the intake mark, its renewal and lease, hand-overs and their
withdrawals all go. The statement carries it in clauses 137, 120, 140,
C.5's pull request row and C.8's rows (agentplot/blueprints ab5d788,
7c2fbf0), and 137 as amended at 75c5ff4 refuses a pull request table on
a branch whose records each host keeps. Rule core.33 binds it.

The planner's ruling of 2026-11-07, on this change's question, sets its
scope: this change builds what takes a branch's requests in on the
forge, `sluice job round`, the round part of `sluice.yml`, of `sluice
init actions` and of `sluice forge check`, taken out of `actions-pool`;
such a pull request table is refused for that branch alone; and moving
sluice's own `main` and WilldanGroup/switchboard-kit's `main` is not
this change's work. Why not leave intake to whichever host leads the
branch: "once the host process ends when idle (135), a request a person
opens by hand waits on nothing until some command on some host happens
to start one. switchboard-kit's requests are opened by people."

What sluice does at the bolt's base, 68d68a9a:

- A branch whose records each host keeps is taken in by whichever host
  holds `refs/sluice/intake/<branch>` on its remote
  (`crates/sluice/src/profiles/pushed_refs.rs` `intake_ref`,
  `put_intake_mark`, `drop_intake_mark`). `Pulls::takes`, `decide`,
  `put_mark`, `hold`, `hand_over` and `let_go_marks` in
  `crates/sluice/src/profiles/github/pulls.rs` push, renew, take, hand
  over and let it go; `Taker`, `TakenInBy`, `Holder` and `LetGo` in
  `crates/sluice/src/lib.rs` carry it through `sluice run`, `sluice
  step`, waits and submits; `forge/intake/` and `waits/<pid>.hold` keep
  it in the host's state (`kept.rs`).
- A branch whose records are on its remote is taken in by whichever
  host leads it (`Pulls::takes` answering `s.keeps(branch)`), so its
  requests move only while some host's sluice runs.
- `Actions::wake` dispatches a round of the pool's workflow
  (`crates/sluice/src/profiles/github/actions.rs`), but no workflow
  sluice writes takes it: there is no `sluice job` and no `sluice init
  actions`. `actions-pool` designs them (its D2) with none of its tasks
  done.
- A `pull_requests` table needs only `remote`
  (`crates/sluice-core/src/policy.rs`), so a branch whose records each
  host keeps takes requests today.

## What Changes

- **The forge's round job.** `sluice job round --branch <b>` runs a
  branch's rounds as its pool's round job, the instance
  `pool/<pool>/round`, taking its requests in, posting on them and
  landing them, and ends once nothing is its to do now. With no branch
  it wakes one round job for each branch the forge keeps. A branch
  whose records are on its remote and that takes pull requests is kept
  by that job alone, with every host off; its checks are still claimed
  by standing executors (72, 75, 76, 107, 137; C.8).
- **`sluice.yml`'s round job.** On `workflow_dispatch` and a schedule,
  one per branch at a time, in the `sluice` environment that alone
  holds the app's key, running the sluice release that wrote it, its
  state kept in the forge's cache from one job to the next.
- **`sluice init actions`**, its round part: writes that workflow, adds
  `[signers.forge]` and `[pools.forge]` with no runners where absent,
  and prints what to set on GitHub. **`sluice forge check`**, its round
  part: the workflow on the branch, the app's permission to dispatch
  it, and the environment that admits only that branch.
- **A pool with no runners** runs rounds and takes no run.
- **Wakes.** A submit that opened, updated or marked ready a request,
  and a host that recorded a run's result on a branch the forge keeps,
  wake its round job (111, 136).
- **No host takes requests in.** The intake mark, its renewal, its
  lease, hand-overs and their withdrawals, `Taker`, `TakenInBy`,
  `LetGo`, `forge/intake/` and `waits/<pid>.hold` go; a host's pass,
  round, step, wait and submit take nothing in and run no round of a
  branch the forge keeps beyond claiming its runs. A group submit
  queues nothing on a host: its requests are marked ready as one group
  and the round job takes them in one pass (125).
- **Every host follows its requests on the forge.** What records 0095
  and 0096 decided for a host without the mark becomes every host's:
  each request a submit on it opened, updated or marked ready is kept
  under `forge/follows/`, read once in a quarter of `discovery` across
  the host, its state read from sluice's check's fields over its words.
  `taken_in_by` goes from status, the overview and waits; `queue`
  stays, its `host` naming the round job.
- **A pull request table on a branch nothing would take in is refused
  for that branch alone**, as `[[checkout.cache]]` is: on a branch whose
  records each host keeps (137 as amended at 75c5ff4), or whose policy
  names no pool to take them in, naming `[signers]` and `sluice init
  actions` (56, S64).
- **Release notes.** The commit that brings the refusal is a breaking
  one, so the release's notes say what such a branch adds before its
  hosts upgrade.
- **`actions-pool`** keeps keyless signing, the run, sign and record
  jobs, `sluice-pr.yml` and head runs; its tasks, design and specs no
  longer build the round job, written so in this change.

## Capabilities

### New Capabilities

- `forge-rounds`: the forge's round job, which keeps a branch that
  takes pull requests, the workflow that runs it, its wakes, its state
  between jobs, `sluice init actions`'s round part and a pool with no
  runners.

### Modified Capabilities

- `request-intake-host`: no host takes a branch's requests in; the
  intake mark and its hand-overs go; every host follows its requests by
  the forge and sluice's check.
- `pull-request-intake`: the refusal; requests submitted by the round
  job; no hand-over clause.
- `pull-request-submit`: a submit wakes the round job, answers with its
  request and follows it, and takes again a request the round job sent
  back.
- `pull-request-status`: sluice's check names the round job.
- `group-submit`: the act marks its requests ready as one group and
  queues nothing on the host.
- `worktrunk-submit`: a request stands and is sent back as the round
  job's check says.
- `overview`: a branch lists the requests the host follows, with no
  `taken_in_by`.
- `submission-wait`: a wait on a request follows it by its check, with
  no `taken_in_by`.
- `obsolete-combinations`: no hand-over makes a combination obsolete.
- `forge-identity`: `sluice forge check`'s round part.
- `reader-tick`: a host's tick fetches a branch the forge keeps only to
  claim its runs.
- `landing-beside`: a request's entry lands beside the queue by the
  round job alone (140).
- `peer-lead`: a branch the forge keeps is led by its round job alone.

## Out of scope

- **Moving the two `main`s.** sluice's own `main` is the planner's, as
  a chore after this change lands; switchboard-kit's is wldn's. No host
  serving either installs a release carrying this change until that
  branch names `[signers]` and carries the round workflow; ops and wldn
  hold those installs. This change owes them the refusal's words and
  the release notes' line.
- **What `actions-pool` keeps**: keyless signatures and their
  verification, the run, sign and record jobs, `sluice-pr.yml`, runs on
  a request's head, and a pool that takes runs. Until it lands, a
  branch the forge keeps runs every check on standing executors.
- **The host process.** `a-hosts-queues-move-in-sluices-own-host-process`,
  on the same bolt, moves `sluice run`'s pass into sluice's host
  process; this change takes intake out of whichever of the two passes
  stands when it is built.
- **A forge other than GitHub.**
- **The intake mark left on a remote.** `refs/sluice/intake/<branch>`
  is read by nothing once this lands; the pull requests guide says how
  to delete it.

## Impact

- `crates/sluice-core`: `policy.rs` (the refusal for one branch, a pool
  with no runners), `step.rs` (`keeps` for a branch the forge keeps),
  `ops.rs` (wakes).
- `crates/sluice`: `lib.rs` (`Command::Job`, `Command::Init`, the
  intake mark's types, `submit_pull`, `submit_pull_group`, `wait_on`,
  status and `--status`), `profiles/github/pulls.rs`, `kept.rs`,
  `actions.rs`, `forge.rs`, `profiles/pushed_refs.rs`, `overview.rs`;
  the workflow template under `profiles/github/workflows/`.
- Tests: the pull requests suite takes its requests in by the round
  job, with `main`'s records on its bare remote; the intake mark's
  tests and the hand-over tests go with it.
- The site's pull requests guide, state, commands and configuration
  references, the `using-sluice` and `setting-up-sluice` skills, and
  sluice's own records: a new record for this change's choices, and
  records 0095 and 0096 amended.
- Open changes: `a-hand-overs-withdrawal-holds-no-request` and
  `a-host-hands-over-only-to-the-host-its-mark-names`, built and landed,
  are archived first, so this change's deltas remove what they added;
  `actions-pool`'s tasks, design and deltas are written here without
  the round job.
