# Design

## Context

Record 0093 and clauses 137, 120, 140, C.5 and C.8, with 137 as amended
at agentplot/blueprints 75c5ff4, are what this change builds; the
planner's ruling of 2026-11-07 sets its scope. See proposal.md for why
and for what sluice does at 68d68a9a.

What stands and is built on:

- Records on the remote (`remote-records`, `peer-lead`): a branch whose
  policy names a `remote` in a policy with `[signers]` keeps its
  records under `refs/sluice/`, led by one instance under the
  deployment's lease (72, 73), its runs claimed by any instance with
  room (75), its results counted through the signers it names (78).
- `Actions::wake` dispatches `{"mode":"round","branch","claim":"round/<b>"}`
  to the first pool's workflow (`profiles/github/actions.rs`), called
  after a submit on a branch whose records are on its remote
  (`ops.rs`).
- Records 0095 and 0096: sluice's check on a request's head carries its
  entry as fields (`<!-- sluice:queue {…} -->`); a host keeps the
  requests it follows under `forge/follows/` and reads each once in a
  quarter of `discovery`; the fields decide over the words; a held
  request is read by its draft mark; a submit marks ready a request its
  check says was sent back or held. Each record names what of it stays
  and what goes once record 0093 is built.
- A failed affected reading takes the work into the queue
  (`step.rs`, "the affected reading failed; the work takes the queue").

## Goals / Non-Goals

**Goals:**

- A request opened by a person, or by a submit on a host that then
  switches off, is taken in, tested on standing executors and landed
  with every host's sluice stopped.
- No host takes a request in, holds a mark, or hands anything over.
- Every host reads where its request stands from the forge and
  sluice's check alone.
- No request is opened where nothing would take it in.

**Non-Goals:**

- Moving any repository's branches (proposal, Out of scope).
- Runs on the pool, keyless signing, `sluice-pr.yml` (`actions-pool`).
- The host process (`a-hosts-queues-move-in-sluices-own-host-process`).

## Decisions

### D1. The forge keeps a branch that takes pull requests, as `pool/<pool>/round`

A branch the forge keeps is one whose records are on its remote, whose
policy at its revision takes pull requests and names a pool. Its lead
is the round job of the policy's first pool, the instance named
`pool/<pool>/round`, as `actions-pool`'s D2 named it: every round job
of the pool leads under that one name, so a job goes on from the last
with no lease to wait out, and the workflow's concurrency keeps one per
branch at a time. A leased write of the records refuses a second
writer whatever its name (62).

`Step::keeps` answers false, for such a branch, in any instance not so
named: a host never takes its lead, however long the records stood
unchanged, and runs `execute`, claiming and running its runs (75). A
host that led it under an older sluice is a lead that went quiet: the
round job takes the branch once it has seen the records unchanged for
the lease by its own observation (73), carried between jobs (D6).

*Alternative:* a host's instance keeps the branch while one is up and
the job when none is. core.33 rules out a host taking a branch's
requests in, and a host process that ends when idle (135) would leave a
request opened by hand waiting on whichever command next starts one;
the planner's ruling says so.

### D2. `sluice job round`

`sluice job round --branch <b>` refuses a branch the forge does not
keep, naming why, and exits 2. Otherwise it runs rounds of `<b>` under
the branch's lock, each a step as `sluice step` runs one (core.14):
intake asks for the open requests at its first round and again each
`discovery` while it runs (136), takes in each request whose
conditions hold, posts, forms, lands and releases. It ends, exiting 0,
once a round finds nothing its to do now: no request due to be asked
or taken in, nothing to decide, land, post or release, every open
combination waiting on a run another instance claims. A run's result
wakes the next job (D5). It ends after 50 minutes whatever is left,
short of the job's limit; what it left is a lead gone quiet (73).

With no `--branch`, it lists the branches on the remote whose policy at
their revision the forge keeps, wakes one round job for each through
`Actions::wake`, and exits 0. The schedule runs it so.

It reads the host config the workflow writes (D4) as any command does,
so its forge client, lease and discovery are a host's. It needs no
toolchain: where the policy's toolchain is absent from the runner, the
affected reading fails and the work takes the queue, as it does today
(140); the workflow installs nothing a policy's toolchain names
(core.1), and the guide says to add the step where landing beside the
queue is wanted.

*Alternative:* `sluice step` in the job. It would take the lead under
the host's name the job's config gives, and stop after one round.

### D3. The workflow's round job

`crates/sluice/src/profiles/github/workflows/sluice.yml` holds the
round part of `actions-pool`'s `sluice.yml`:

- `on: workflow_dispatch` with the inputs `Actions::wake` sends
  (`mode`, `branch`, `claim`), and `schedule` every five minutes,
  GitHub's shortest, which is the forge's discovery for a request
  opened by hand.
- One job, `round`, `if: inputs.mode == 'round' || github.event_name
  == 'schedule'`, on `ubuntu-latest`, `environment: sluice`,
  `concurrency: { group: sluice-round-${{ inputs.branch || 'schedule'
  }}, cancel-in-progress: false }`, `permissions: contents: read`.
- Its steps: an empty repository, `git init` with `origin` the
  repository's address, which the round's own fetch fills, since the
  round reads the branch and its records from the remote (124, C.6);
  download the release of sluice whose version the template was
  written at, from the project's releases; restore the state folder
  from the cache (D6); write the host config (D4); run `sluice job
  round` with `--branch ${{ inputs.branch }}` where one was given; save
  the state folder.

A dispatch with `mode: run` runs no job: a pool with no runners is
launched no run (D7), and `actions-pool` adds the run and sign jobs.

*Alternative:* `cargo install` or a Nix build in the job: minutes per
round, and a toolchain the round needs nowhere else.

### D4. The job's credential is its environment's

The `sluice` environment holds the app's id as the variable
`SLUICE_APP_ID` and its key as the secret `SLUICE_APP_KEY`, and admits
only the branch the workflow runs on, the default branch. The job's
step writes the key to `$RUNNER_TEMP/sluice/app.pem`, mode `0600`, and
a host config `[forge] app_id`, `key` beside it, with `XDG_CONFIG_HOME`
pointing there, so sluice's git calls carry the app's token as any
host's do (forge-identity). No other job holds the environment; a
pull request's workflow run gets neither (C.8).

### D5. Wakes

A submit that opened, updated or marked ready a request into a branch
the forge keeps wakes its round job, once for the act, after the
`.moved` touch (136's "at once after a submit on the host opened or
moved a request"). A host that records a run's result on such a branch
wakes it, once a round, since the verdict is work no running instance
will take (111). A wake that fails is a warning, never a failure: the
schedule takes the work at its next run (59). The wake after a
submission written to a branch whose records are on its remote stays
as it is.

### D6. The job's state carries between jobs

The job restores `$XDG_STATE_HOME/sluice` from `actions/cache` keyed
`sluice-round-<branch>-<run_id>` with the prefix
`sluice-round-<branch>-` to restore from, and saves it at the end, so
a job sends the forge's answers kept for each address (136; core.32)
and measures the lease from its own first reading (73). A job that
finds no cache asks unconditionally once and keeps what it got, as a
host's first pass does.

### D7. A pool with no runners

`[pools.<name>] runners` becomes optional. The lead launches no run on
a pool with none (107 finds no runner for the run's system), so a run
no standing executor has room for waits open, as it does where no pool
is named (87). `signer` stays required: it is what the pool's run jobs
will sign as once `actions-pool` adds them, and what the round job
signs is nothing.

### D8. The refusal, for one branch

`Policy::rules_for(<b>)` answers a refusal for `<b>` alone where its
table has `pull_requests` and either its records are each host's (no
`[signers]`, 137 as amended at 75c5ff4) or the policy names no pool
(this change's choice: nothing else would take its requests in). The
words name the branch, `[signers]` or `[pools]` as the case is, and
`sluice init actions`, and say no request is opened for it:

```
main takes pull requests, which only the forge's round job takes in, and
its records are each host's: name [signers] so they are kept on origin,
and add the round job with `sluice init actions` (137)
```

Every other branch of the policy reads as before, as a
`[[checkout.cache]]` refuses its branch alone (56, S64). A submit to
the refused branch is refused with those words before anything is
pushed or opened; a status and the overview show the refusal (26). The
policy's whole refusal of a `pull_requests` table with no `remote`
stays.

*Alternative:* accept the table and open requests that wait. Nothing
would take them in, and nothing would say so.

### D9. What a host keeps and reads

Every host is one that does not take the requests in, so what records
0095 and 0096 decided for such a host becomes every host's:

- `opening` keeps each request it opens, updates or marks ready in
  `forge/follows/`, whoever keeps the branch; a request leaves once a
  read finds it merged or closed and a wait or status has said so.
- `follow_across`, `across` and `kept_across` read every followed
  request, of a branch whose records are on its remote too.
- A wait, a waiting submit, `--status`, `sluice status` and the
  overview read a request's state from sluice's check's block, else its
  words (`as_its_check_says`), never the conclusion GitHub keeps.
- `taken_in_by` goes from every document. `queue` stays.
- A request waiting with no check on its head yet, or whose check says
  it waits to be taken in, reads `waiting to be taken in`.

A status of a branch the forge keeps shows its queue from the branch's
records on the remote, as any such branch's (26, 80), and each request
the host follows beside it, from the check.

### D10. sluice's check names the round job

The block's `host`, and the summary's `Submission <n> on <host>.` and
`Waiting to be taken in on <host>.`, name the instance that keeps the
branch: `pool/<pool>/round`. A check an older sluice posted, naming a
host, reads as before.

### D11. A submit answers with its request

A `--no-wait` submit to a branch the forge keeps answers with the
request and the state its check says, or `waiting to be taken in`,
and never with an entry it wrote, since it writes none; under `--json`,
`pull_request`, `branch`, `state` and `queue`. A waiting submit and
`sluice wait <branch>` follow the request as `follow_across` does, with
the exits it gives today: 0 landed, 1 sent back or closed, 5 out of
time. A request the round job sent back, failed, in conflict or held,
is marked ready at the head pushed by the next submit (record 0096).

A group submit opens its drafts, reads their conditions, and where all
hold marks them ready as one group and wakes the round job, which
takes the group in one pass and one write (125;
`pull-request-intake`'s group requirement); where one falls short it
refuses as today. Its answer names every request with what it waits
for.

### D12. What goes, and what an older record keeps

Removed: `intake_ref`, `Mark`, the mark's fetch, `fetched_mark`,
`intake_mark`, `intake_mark_at`, `put_intake_mark`, `drop_intake_mark`
(`pushed_refs.rs`); `Holder`, `TakenInBy`, `Taker`, `LetGo` and the
trait's `intake`, `elsewhere`, `let_go`, `handed_over` (`lib.rs`);
`take_in`'s mark, `takes`'s mark rules, `decide`, `put_mark`, `hold`,
`hand_over`, `let_go_marks`, `taker_elsewhere`, `taken_in_by`
(`pulls.rs`); `Observed`, `observe`, `hold`, `let_go` and their files
(`kept.rs`); `Requester::HandOver`.

`Withdrawal.handed_over_to` stays readable and is never written; a
record that holds it reads as written (26, 109). `forge/intake/` and
`waits/` are no longer read or written, and the state reference says
they may be removed. The intake mark on a remote stays until someone
deletes it; the guide's recipe does.

### D13. `sluice init actions` and `sluice forge check`, their round parts

`sluice init actions` writes `.github/workflows/sluice.yml` from the
template at the version that writes it; leaves a differing file, saying
how it differs, unless `--force`; adds `[signers.forge] github = {
workflow = ".github/workflows/sluice.yml", ref = "refs/heads/<default>" }`
and `[pools.forge] signer = "forge"` where absent; and prints the
environment, its variable, secret and branch rule, and the app's
Actions write permission. It refuses a policy that names no
`[signers]`, changing nothing: adding the first signer moves every
branch with a remote onto it, and the hosts that run their checks need
signers of their own first (the guide).

`sluice forge check <b>`, for a branch the forge keeps, also reports
whether the workflow on `<b>` is the round job `sluice init actions`
would write at this version, whether the app may dispatch it (Actions
write), and whether the `sluice` environment exists and admits only
`<b>`, exiting non-zero naming each that fails. The forge signer's own
checks stay `actions-pool`'s.

### D14. The suite takes its requests in by the round job

`OnGitHub` in `crates/sluice/tests/profiles/pull_requests.rs` gives
`main` its records on the bare remote: `[signers.a]`, the host's SSH
key, `trust = ["a"]`, `[signers.forge]` and `[pools.forge]` with no
runners. `OnGitHub::job()` runs `sluice job round --branch main` in a
clone of its own with the job's config (D4) and a state folder kept
across its calls (D6). `round()` is the job's round, then the host's
`sluice step main`, which claims and runs the checks, then the job's
round again. The GitHub double records dispatches as it does. A test
of the intake mark or a hand-over goes with it; a test of a host
following its request keeps its name where its scenario stands.

## Risks / Trade-offs

- [Discovery is five minutes, not one] A request opened by hand waits
  up to a schedule's interval, and GitHub delays scheduled runs under
  load. → A submit's wake starts a job at once; a person's request is
  the one that waits. 136's minute stays a host's and a running job's.
- [Every host off, checks open] With no standing executor up, a
  request is taken in and its combination opened, and its runs wait
  for a host. → `actions-pool`'s run jobs; until then the check says it
  is under test and the run is claimed when a host returns (75).
- [The job's minutes] A job per wake and per schedule run, each a
  fetch and a few calls. → It ends when nothing is its to do; idle
  schedules cost a job start and conditional reads.
- [A release the job downloads] The job runs the version that wrote
  the workflow; hosts may run another. → The records' form is read by
  either, as today across hosts; `sluice forge check` names a workflow
  older than the host's sluice.
- [Repositories that take pull requests today] Each stops taking
  requests in on the release. → The refusal says what to add, the
  release notes say it first, and ops and wldn hold the installs
  (proposal, Out of scope).

## Migration Plan

Per repository whose branch takes pull requests: land `[signers]` for
the hosts that run its checks, with `trust` naming them; run `sluice
init actions` and land the workflow and `[pools.forge]`; set the
environment as it prints; then upgrade its hosts. Rollback: install the
earlier release on every host serving the branch; its records on the
remote read as before, and the earlier sluice takes its requests in by
the lead.

## Open Questions

- The schedule's five minutes and the job's 50 are first guesses, as
  `actions-pool`'s were.
