# Exploring sluice on many machines

A brief for a design session, written so it needs nothing from the
conversation that produced it. No code: the output is design documents and
drafted clauses for the operator to rule on.

## The question

sluice is a merge queue for AI agents in git worktrees. Phase 1 runs on one
machine and is being built now (`docs/plans/phase-1-core.md`). The operator
runs agents on many machines, ten servers or more, and sessions on any of
them submit to the same branch: a bolt's branch as much as `main`. Two ways
of running sluice across machines must both be possible from one design:

- **peers**: sluice runs on every machine, and they coordinate through state
  they share (refs on the shared git remote), with no server of sluice's
  own;
- **a central server**: one machine takes the queue and the test execution
  off every other; agents only submit and wait.

Settle how each works in enough detail to build, compare them honestly, and
say which to build first. The operator has said: no GitHub Actions as the
test executor; the task runner's cache (moon's) is an optimization, not the
design; a test checkout is a clean checkout, likely in a container, and the
tests run in it; sluice starts no services itself.

## Read first

All under `~/Code/github_agentplot/`:

1. `sluice/main/docs/examples/README.md`: what using sluice looks like on
   one machine.
2. `blueprints/main/design/sluice/requirements.md`: the statement. The
   multi-machine parts are A.8 (clauses 57 to 60), B.7 the records (61 to
   63), C.4 pushed refs, C.5 remote submit, C.6 records profiles, C.8 the
   two deployments compared, and the open questions in section 10.
3. `blueprints/main/design/sluice/models/reconstruction-1.md` section 2.4
   (the lead lease), 2.3 (claims), 4 (multi-host), and
   `reconstruction-1-answers.md`, "Left for several machines".
4. `blueprints/main/design/sluice/research/2026-10-08-remote-sluice.md`:
   remote execution (Bazel REAPI, BuildBuddy, NativeLink, Nx Agents) versus
   a central queue, and hosted queues compared.
5. `blueprints/main/design/sluice/research/2026-10-11-test-services.md`:
   why sluice starts no services, and what it owes a test run.
6. `sluice/main/docs/adr/0003`, `0005`, `0008`, `0009`, `0010`, and
   `docs/architecture/core.md`.
7. `blueprints/main/design/requirements-style.md`: the form drafted clauses
   must take (mechanism-free invariants in Parts A and B, tools only in
   Part C).

## What to work out

- **Shared state under peers.** The records as git refs on the shared
  remote: their layout, how a write conditional on the version read becomes
  a push with a lease, how often machines fetch, what a write costs, and how
  many machines and submissions per hour it sustains before the remote is
  the bottleneck.
- **Who advances a branch.** The lead lease: duration, renewal, takeover,
  and fencing so a machine that lost the lead writes nothing more (58, 59).
  The reconstruction argues a stopped machine and a quiet one need not be
  told apart; test that argument.
- **Who runs which tests.** Under peers, how a machine claims a
  combination's tests, how many it takes, and what happens when it dies
  mid-run (60). Under a central server, its executor pool and scheduling.
- **Checkouts on remote machines.** The container checkout profile: image,
  how the combination's commit reaches the executor, how results and
  failure logs come back before the result is read (clause 16).
- **Trust.** How a branch's rules name whose runs count, and how a run
  proves which machine made it when every machine can write shared state.
  Whether signed results are needed for "confined" to mean anything under
  peers.
- **The central server.** Its interface (what an agent's `wt submit` calls),
  how state stays readable when it is down (26), what restarting it costs.
- **Failure cases.** A partitioned machine, a dead executor, the remote
  unreachable, the server down, two machines both believing they lead.
- **Comparison and recommendation.** Cost to run, cost to build, failure
  behaviour, how each grows from phase 1's one-machine code, and which to
  build first.

## Deliverables

In `~/Code/github_agentplot/blueprints/main/design/sluice/`:

- `models/many-hosts-1.md`: the design of both deployments, the comparison
  and the recommendation, citing clauses by number.
- `proposals/many-hosts-2.md`: drafted clauses answering the section 10
  multi-machine questions, in the statement's form, with the proposals
  README row. The operator rules on them; do not edit `requirements.md`.
- Questions you could not settle, listed at the end of the model.

Commit in blueprints by explicit paths, Conventional Commits with scope
`sluice`, never push, ending each message with the attribution line your
session provides. Tell the operator when the model is ready to review.
