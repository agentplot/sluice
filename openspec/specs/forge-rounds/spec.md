# forge-rounds Specification

## Purpose
How the forge keeps a branch that takes pull requests: a GitHub
Actions workflow, the round job, takes the branch's requests in and
runs its rounds with every host off, woken by a submit, a recorded
result or its schedule, while the hosts whose results the branch
trusts run its checks (137, C.8; rule core.33; record 0101).

## Requirements

### Requirement: The forge's round job keeps a branch that takes pull requests
A branch whose records are on its remote and whose policy, at its
revision, takes pull requests and names a pool SHALL be kept by the
round job of the policy's first pool, named `pool/<pool>/round` in its
records, and by no host. The round job SHALL take the branch's
requests in and run its rounds, one at a time, with every host's
sluice stopped; its runs SHALL be claimed by standing executors as any
run is (72, 75, 76, 107, 137; C.8).

#### Scenario: Every host off
- **WHEN** `main` takes pull requests, keeps its records on `origin` and names `[pools.forge]`, a person opens a pull request into `main` with every condition holding, and no host runs sluice
- **THEN** `sluice job round --branch main` takes it in as a submission, forms its combination and posts its `sluice` check; once a host's `sluice step main` has claimed and passed the combination's run, the next `sluice job round --branch main` lands it and GitHub marks the pull request merged

#### Scenario: A host claims and never leads
- **WHEN** the round job last wrote `main`'s records longer ago than the host's lease, and host `a` runs `sluice step main` with a combination open to claims
- **THEN** `a` claims and runs the combination's run, and `main`'s records still name `pool/forge/round` as their lead

#### Scenario: A lead an older sluice took
- **WHEN** `main`'s records name host `a` as their lead, and round jobs have seen them unchanged for the lease from their first reading
- **THEN** the next round job takes `main` and takes its pull requests in

### Requirement: sluice job round runs a branch's rounds until nothing is its to do
`sluice job round --branch <b>` SHALL run rounds of `<b>` as its round
job, asking for the open pull requests at its first round and again
each `discovery` while it runs, and SHALL end, exiting 0, once a round
finds nothing its to do now, or after 50 minutes. Nothing is its to do
when no pull request is due to be asked for or taken in, nothing is to
be decided, landed, posted or released, and every open combination
waits on a run for another instance to claim (136, 137).

#### Scenario: It ends while a run goes on elsewhere
- **WHEN** host `a` holds the run of `main`'s only open combination and nothing else waits, and `sluice job round --branch main` runs
- **THEN** it exits 0 after one round, and `a`'s result, once recorded, is landed by the next round job

#### Scenario: A branch the forge does not keep
- **WHEN** `sluice job round --branch bolt/x` runs and `bolt/x` takes no pull requests
- **THEN** it exits 2, saying `bolt/x` takes no pull requests and is not the forge's to keep, and writes nothing

### Requirement: sluice job round with no branch wakes each branch the forge keeps
`sluice job round` with no `--branch` SHALL wake one round job for each
branch on the remote that the forge keeps, reading each branch's own
policy at its revision, whatever the default branch's policy names, and
exit 0 (111, 137; rule core.27).

#### Scenario: The schedule wakes each branch
- **WHEN** `main` and `release/1` take pull requests with their records on `origin`, `bolt/x` takes none, and `sluice job round` runs with no branch
- **THEN** it dispatches one round of the pool's workflow for `main` and one for `release/1`, none for `bolt/x`, and exits 0

#### Scenario: A branch its own policy names
- **WHEN** `hotfix/1`'s own revision takes pull requests with its records on `origin`, `main`'s policy names no `hotfix/1`, and `sluice job round` runs with no branch
- **THEN** it dispatches one round for `hotfix/1` beside `main`'s

### Requirement: The round job is woken when work arrives on a host
A submit that opens, updates or marks ready a pull request into a
branch the forge keeps, and a host that records a run's result on such
a branch, SHALL wake that branch's round job, starting the pool's
workflow on the branch its `ref` names, else on the repository's
default branch as GitHub names it, which holds the workflow whichever
branch the job keeps. A wake that fails SHALL be a warning and SHALL
lose nothing: the schedule's next round job takes the work (59, 111,
136).

#### Scenario: A submit wakes it
- **WHEN** an agent runs `wt submit main --no-wait` and `main` is the forge's
- **THEN** GitHub records one dispatch of the pool's workflow with `mode` `round` and `branch` `main`

#### Scenario: A default branch that is not main
- **WHEN** the repository's default branch on GitHub is `trunk`, `[pools.forge]` names no `ref`, and an agent runs `wt submit main --no-wait`
- **THEN** GitHub records one dispatch of a round of `main` started on `trunk`

#### Scenario: A recorded result wakes it
- **WHEN** host `a`'s `sluice step main` records the result of a run of `main`'s combination
- **THEN** GitHub records one dispatch of a round of `main`

#### Scenario: A lost wake
- **WHEN** GitHub refuses the dispatch after a submit
- **THEN** the submit warns that the round job was not woken, exits as it would have, and the next `sluice job round` with no branch wakes `main`'s round

### Requirement: The round job's state carries from one job to the next
The round job's state folder SHALL be restored at its start and kept
at its end in the forge's cache for its branch, so each job reads the
forge conditionally on the answers the last kept, and measures the
lease from the first reading any job of the branch made (73, 136; rule
core.32).

#### Scenario: A job after a job
- **WHEN** two round jobs of `main` run one after the other with the state folder the first kept, and nothing on GitHub changed between them
- **THEN** the second's list of `main`'s open pull requests is sent with what GitHub answered the first, and GitHub answers it unchanged

### Requirement: sluice init actions writes the round job
`sluice init actions` SHALL write `.github/workflows/sluice.yml` with
its round job: on `workflow_dispatch` and on a schedule every five
minutes, one job per branch at a time, in the `sluice` environment,
running the sluice release that wrote it. A workflow there that differs
from what it would write SHALL be left, saying how it differs, with
exit 1, unless `--force` is given (C.8).

#### Scenario: A repository with signers and no pool
- **WHEN** the policy names `[signers.box]` and no pool, and `sluice init actions` runs
- **THEN** it writes `.github/workflows/sluice.yml` with its round job and exits 0

#### Scenario: A workflow already there
- **WHEN** `.github/workflows/sluice.yml` exists and differs from what it would write
- **THEN** it leaves the file, says how it differs, exits 1, and writes it only with `--force`

### Requirement: sluice init actions names the round job in the policy
`sluice init actions` SHALL add `[signers.forge]`, naming the workflow
on the default branch, and `[pools.forge]` with no runners, naming the
workflow and the default branch as its `ref`, where absent, saying
which it added, and print what to set on GitHub: the `sluice` environment, its
`SLUICE_APP_ID` variable and `SLUICE_APP_KEY` secret, its branch rule,
and the app's Actions write permission. A policy that names no
`[signers]` SHALL be refused, changing nothing, with exit 2.

#### Scenario: The signer and the pool
- **WHEN** the policy names `[signers.box]` and no pool, and `sluice init actions` runs
- **THEN** `.config/sluice.toml` gains `[signers.forge]` and `[pools.forge]` with no runners, and the output names the environment, its variable and secret, its branch rule and the Actions permission

#### Scenario: No signers
- **WHEN** the policy names no `[signers]`
- **THEN** it exits 2, saying the hosts that run the checks need signers before the branch's records move to its remote, and writes nothing

### Requirement: Only the round job holds the app's key
The workflow SHALL give the `sluice` environment to its round job
alone. The job SHALL write the app's key to a file of mode `0600` under
the runner's temporary directory and name it in the host config it
writes, so the key reaches only sluice's own git calls and forge calls
(C.8; forge-identity).

#### Scenario: The template's environment
- **WHEN** `sluice init actions` has written the workflow
- **THEN** its round job is its only job that names the `sluice` environment, and the run of a pull request's own workflow is given no secret of it

### Requirement: A pool with no runners runs rounds alone
A `[pools.<name>]` table with no `runners` SHALL be accepted. No run
SHALL be launched on it, so a run no standing executor has room for
stays open, and its jobs SHALL only run rounds (87, 107).

#### Scenario: A run waits for a host
- **WHEN** `[pools.forge]` names no runners and no standing executor has room for an open combination's run
- **THEN** the run stays open, GitHub records no dispatch with `mode` `run`, and the combination is not passed
