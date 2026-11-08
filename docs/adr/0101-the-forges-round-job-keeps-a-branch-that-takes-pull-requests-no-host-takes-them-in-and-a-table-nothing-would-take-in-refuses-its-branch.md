---
number: 101
title: The forge's round job keeps a branch that takes pull requests, no host takes them in, and a pull request table nothing would take in refuses its branch alone
status: accepted
date: 2026-11-07
decision-makers:
- agentplot-design, for 137 as amended at agentplot/blueprints 75c5ff4
- the planner, for the change's scope, ruling on 2026-11-07
- the design of change/pull-requests-are-queued-on-the-forge, for D1 to D14
---

# The forge's round job keeps a branch that takes pull requests, no host takes them in, and a pull request table nothing would take in refuses its branch alone

Status: accepted. Record 0093, accepted by the user on 2026-11-06 ("I approved 93."), queues pull requests on the forge, and the statement carries it in 137, 120, 140, C.5's pull request row and C.8's rows (agentplot/blueprints ab5d788, 7c2fbf0). 137 as amended at 75c5ff4 is agentplot-design's, on this change's question. The scope is the planner's ruling of 2026-11-07. D1 to D14 are the change `pull-requests-are-queued-on-the-forge`'s own, under the statement so amended and rules core.1, core.14, core.31, core.32 and core.33, which they keep; cheap to change later.

Amends in part records 0095 and 0096, each marked so where it stands.

Deciders: agentplot-design, for the statement; the planner, for the scope; the change's design, for D1 to D14.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 59, 72, 73, 75, 76, 87, 107, 111, 120, 125, 129, 136, 137 and 140, S64, C.5's pull request row and C.8's rows at 7c2fbf0, and 137 at 75c5ff4; records 0071, 0088, 0089, 0093, 0095 and 0096; rules core.1, core.14, core.31, core.32, core.33; the change `actions-pool`'s D2; `crates/sluice/src/profiles/pushed_refs.rs` (`intake_ref`, `put_intake_mark`, `drop_intake_mark`), `crates/sluice/src/profiles/github/pulls.rs` (`Pulls::takes`, `decide`, `put_mark`, `hold`, `hand_over`, `let_go_marks`), `crates/sluice/src/profiles/github/actions.rs` (`Actions::wake`), `crates/sluice/src/lib.rs` (`Taker`, `TakenInBy`, `Holder`, `LetGo`), `crates/sluice/src/profiles/github/kept.rs` and `crates/sluice-core/src/policy.rs` as sluice held them at 68d68a9a.

## Context and problem statement

137 as rewritten at 7c2fbf0 has a branch's requests taken in "only where its records are on its remote, by the instance that keeps the branch there, on a forge its jobs, one round at a time". 137 as amended at 75c5ff4 adds that a pull request table on a branch whose records each host keeps is refused for that branch alone, since nothing would take its requests in.

At 68d68a9a a branch whose records each host keeps was taken in by whichever host held `refs/sluice/intake/<branch>` on its remote, with the mark's renewal, lease, hand-overs and their withdrawals; a branch whose records are on its remote was taken in by whichever host led it, so its requests moved only while some host's sluice ran. `Actions::wake` dispatched a round of the pool's workflow, but no workflow sluice wrote took it: there was no `sluice job` and no `sluice init actions`. A `pull_requests` table needed only `remote`.

The planner's ruling of 2026-11-07, on this change's question, sets its scope: this change builds what takes a branch's requests in on the forge, `sluice job round`, the round part of `sluice.yml`, of `sluice init actions` and of `sluice forge check`, taken out of `actions-pool`; such a pull request table is refused for that branch alone; and moving sluice's own `main` and WilldanGroup/switchboard-kit's `main` is not this change's work. Why not leave intake to whichever host leads the branch: "once the host process ends when idle (135), a request a person opens by hand waits on nothing until some command on some host happens to start one. switchboard-kit's requests are opened by people."

## Considered options

**The statement (137 as amended at 75c5ff4).**

1. **A pull request table on a branch whose records each host keeps refuses that branch alone** (chosen, agentplot-design's). Nothing would take its requests in once no host does.
2. Accept the table and open requests that wait. Nothing would take them in, and nothing would say so.

**The scope (the planner's ruling).**

1. **This change builds the round job, the round part of the workflow, `sluice init actions` and `sluice forge check`, and the refusal; moving the two `main`s is not its work** (chosen).
2. Leave intake to whichever host leads a branch whose records are on its remote. The planner's words above say why not.

**D1. Who keeps a branch that takes pull requests.**

1. **The forge, as `pool/<pool>/round`** (chosen). A branch whose records are on its remote, whose policy at its revision takes pull requests and names a pool, is led by the round job of the policy's first pool, every job of the pool under that one name, so a job goes on from the last with no lease to wait out; the workflow's concurrency keeps one per branch at a time, and a leased write refuses a second writer whatever its name (62). `Step::keeps` answers false for such a branch in any other instance: a host claims and runs its runs (75) and never takes its lead. A host that led it under an older sluice is a lead gone quiet, which the round job takes after the lease by its own observation (73), carried between jobs (D6).
2. A host's instance keeps the branch while one is up, the job when none is. core.33 rules out a host taking a branch's requests in, and the planner's ruling says why.

**D2. `sluice job round`.**

1. **Rounds under the branch's lock until a round finds nothing its to do now, or 50 minutes** (chosen). Each round is a step as `sluice step` runs one (core.14); intake asks at its first round and each `discovery` while it runs (136). Nothing is its to do when no request is due to be asked or taken in, nothing is to be decided, landed, posted or released, and every open combination waits on a run another instance claims. It exits 2 naming why for a branch the forge does not keep. With no `--branch` it wakes one round job per branch on the remote the forge keeps, which the schedule runs. It reads the host config the workflow writes, needs no toolchain, and where the policy's toolchain is absent the affected reading fails and the work takes the queue (140; core.1).
2. `sluice step` in the job. It would take the lead under the host name the job's config gives, and stop after one round.

**D3. The workflow's round job.**

1. **`profiles/github/workflows/sluice.yml`, `workflow_dispatch` with the inputs `Actions::wake` sends and a schedule every five minutes, one job `round` in the `sluice` environment, one per branch at a time, downloading the release that wrote it** (chosen). Its steps: an empty repository whose `origin` is the repository's address, its branches fetched into `origin`'s tracking branches with the job's own read-only token, which nothing keeps, since sluice reads which remote a branch is on from the revisions the host holds and never by a fetch (124, C.6; rule core.27), and its own fetches and pushes carry the app's token; the release; the state folder restored (D6); the host config written (D4); `sluice job round`; the state folder saved. Construction's own: the fetch of the branches, the path not taken a job that reads nothing until sluice fetches, which would find no branch to route. A dispatch with `mode: run` runs no job until `actions-pool` adds the run jobs.
2. `cargo install` or a Nix build in the job: minutes per round, and a toolchain the round needs nowhere else.

**D4. The job's credential.**

1. **The `sluice` environment's `SLUICE_APP_ID` variable and `SLUICE_APP_KEY` secret, admitted only on the default branch, written to `$RUNNER_TEMP/sluice/app.pem` of mode `0600` and named in a host config there** (chosen), so sluice's own git and forge calls carry the app's token as any host's do. No other job holds the environment (C.8).
2. The repository's secrets, which every workflow of the repository reads.

**D5. Wakes.**

1. **A submit that opened, updated or marked ready a request wakes the round job once for the act, and a host that records a run's result wakes it once a round; a wake that fails is a warning** (chosen), since the schedule takes the work at its next run (59, 111, 136).
2. The schedule alone: a request waits up to five minutes after every submit and every result.

**D6. State between jobs.**

1. **The state folder restored from and saved to the forge's cache, keyed by branch and run** (chosen), so a job reads conditionally on what the last kept (136; core.32) and measures the lease from the first reading any job made (73). Construction's own: the cache holds the repository's `.git/sluice-remote` beside the state folder, since that is where a host keeps its first reading of a branch's records; the path not taken, moving that reading into the state folder, would move every host's.
2. A fresh state per job: every read unconditional, and the lease measured from nothing.

**D7. A pool with no runners.**

1. **`runners` optional; no run is launched on a pool with none** (chosen), so a run no standing executor has room for waits open, as where no pool is named (87, 107). `signer` stays required.
2. A separate table for the round job: a second name for one pool.

**D8. The refusal, for one branch.**

1. **`Policy::rules_for(<b>)` refuses `<b>` alone where its table has `pull_requests` and its records are each host's or the policy names no pool, naming `[signers]` or `[pools]` and `sluice init actions`** (chosen); every other branch reads as before (56, S64). A submit to it is refused before anything is pushed or opened; a status and the overview show the refusal (26). The no-pool half is this change's own: nothing else would take the requests in.
2. Refuse the whole policy, stopping every branch while one is moved.

**D9. What a host keeps and reads.**

1. **What records 0095 and 0096 decided for a host without the mark becomes every host's** (chosen): `opening` keeps each request it opens, updates or marks ready under `forge/follows/` whoever keeps the branch; `follow_across` reads every followed request; a request's state is read from sluice's check's block, else its words; `taken_in_by` goes and `queue` stays; a request with no check on its head, or whose check says it waits to be taken in, reads `waiting to be taken in`.
2. A host reads the round job's state for its request: that state is on the remote, but a host that follows one request would fetch every branch's records for it, and core.33 has the host follow its request on the forge.

**D10. sluice's check names the round job.**

1. **Its block's `host` and its first line name `pool/<pool>/round`** (chosen); an older check naming a host reads as before.
2. Name no instance: a reader could not tell the forge's queue from an older host's.

**D11. A submit answers with its request.**

1. **A `--no-wait` submit answers with the request and its check's state, or `waiting to be taken in`, and writes no submission; a waiting submit and `sluice wait` follow it by GitHub and its check** (chosen), exits 0, 1 and 5 as before. A group submit marks its requests ready as one group and wakes the round job, which takes the group in one pass and one write (125).
2. A submit writes the submission itself into the branch's state on the remote: a host advancing a branch the forge keeps, which core.33 rules out.

**D12. What goes, and what an older record keeps.**

1. **The intake mark and everything that keeps, renews, takes, hands over or lets it go, `Requester::HandOver` and the host state under `forge/intake/` and `waits/`, go; `Withdrawal.handed_over_to` is read and never written** (chosen), so a record that holds it reads as written (26, 109). The mark left on a remote is deleted by hand, as the guide says.
2. A migration that deletes every remote's mark: a push sluice makes to a ref it no longer owns.

**D13. `sluice init actions` and `sluice forge check`, their round parts.**

1. **`sluice init actions` writes the workflow at its own version, adds `[signers.forge]` and `[pools.forge]` with no runners where absent, leaves a differing file unless `--force`, refuses a policy with no `[signers]`, and prints what to set on GitHub; `sluice forge check <b>` reports the workflow, the app's Actions write permission and the environment's branch rule** (chosen). Adding the first signer moves every branch with a remote onto it, so the hosts that run its checks need signers of their own first.
2. `sluice init actions` adds the first signer itself: every host's runs stop counting until each has a key the policy names.

**D14. The suite.**

1. **`OnGitHub` gives `main` its records on the bare remote and a round job run in a clone of its own with the job's config and a state folder kept across its calls** (chosen); `round()` is the job's round, the host's step that claims and runs the checks, and the job's round again.
2. A fake forge job inside the test process: it would test a path no workflow runs.

**Construction's own, made building D1 to D14, each cheap to change later.**

- *A combination is pushed to the remote as it is formed* on a branch whose records are on its remote, and a combination dropped before it started is removed there too: a round job's repository is made afresh, so the job that starts a combination, or forms the next on it, fetches what an earlier job formed (82). The path not taken, a job that forms again what it cannot fetch, would form a combination twice under one name.
- *A held request's check says what sent it back*, `Sent back as \`failed: app:test\`.` above its words, since a wait on any host now reads its outcome from the check and the hold's title alone names no test (120; record 0096). The path not taken, a wait reading the branch's records for the outcome, is D9's option 2.
- *A report comment already made is edited to what its request's view says now*, a combination formed again and started included, and *a check of one required test completed on a combination since dropped is posted as a new run when it is undecided again*, which GitHub shows in its place: the round job can read a result before the blame of an entry ahead of it, and a report or a check of that result would otherwise stand (17, 21; ADR 0044). The path not taken, holding every report until the candidates ahead have verdicts, would hold a request's report for a run that passed.
- *`sluice job round` with no branch reads the branches its repository holds as its remote's tracking branches*, as the workflow's empty repository holds them (core.27).

## Decision outcome

The statement's option 1, the planner's scope, and option 1 of each of D1 to D14. Rule core.33 holds what they rule out.

### Consequences

- A request opened by a person, or by a submit on a host that then switches off, is taken in and landed with every host's sluice stopped, its runs claimed by standing executors until `actions-pool` adds the pool's run jobs.
- Each repository whose branch takes pull requests stops taking requests in on the release that carries this, until it names `[signers]`, runs `sluice init actions` and sets the environment; the release's notes say so first, and ops and wldn hold the installs serving sluice's own `main` and switchboard-kit's.
- Record 0095's `taken_in_by`, its words naming the mark's host and holder, and the hand-over as a way a request enters the follows go; its fields, follows, bound, rows and reading stay, the check posted by the round job.
- Record 0096's reading of the check and a submit taking again a request sent back stay, every host's now and the round job's doing the sending back.
- `actions-pool` keeps keyless signing, the run, sign and record jobs, `sluice-pr.yml` and head runs.
