---
number: 119
title: A queue's runs go to a pool, and a CodeBuild fleet runs as many at once as it holds
status: accepted
date: 2026-11-12
decision-makers:
- the user
- agentplot-design
---

# A queue's runs go to a pool, and a CodeBuild fleet runs as many at once as it holds

Status: accepted by the user on 2026-11-12, in agentplot-design's pane: "Did my AWS code build work get put in the backlog? We need to be fucking working on this shit".

Sources: switchboard-kit's `.config/sluice.toml` on its main (agentplot-wldn/switchboard-kit); sluice's guide *Parallel and batch*; the statement in agentplot/blueprints, `design/sluice/requirements.md`, clauses 71, 77, 78, 79, 88, 107, 109, 141, C.3's launched executor, C.7 and C.8; the change `actions-pool` on main; record 0016; record 0117; AWS's CodeBuild guide, *Run builds on reserved capacity fleets* and *Reserved capacity fleet properties* (docs.aws.amazon.com/codebuild/latest/userguide/fleets.html, fleets.reserved-capacity-fleets.html), and CodeBuild's pricing page.

## Context and problem statement

Switchboard's queue runs every check on the EC2 box that holds it. Its main gate ends in Docker end-to-end tests that take 20 to 40 minutes a run. Its main tests one combination at a time (`batch = true`, at most three submissions to a batch), its bolts take the box's other runs, and the box's bound holds all of them together to about four. Submissions have waited days for their turn.

sluice's statement already has pools: a job service that starts a machine for each run the lead gives it, after the standing executors (88, 107). It names one pool, GitHub Actions, whose change `actions-pool` is written and not built. No pool is built, and the guides document none.

## Considered options

1. **Each run is a build on a CodeBuild reserved-capacity fleet, a pool beside GitHub Actions** (chosen).
2. **Standing executors on EC2 instances in an auto scaling group, each claiming runs.** Each would be a host sluice keeps running, with its own bound, leases and caches, and scaling them would fall to sluice or to ops. CodeBuild scales a fleet, keeps its instances warm and starts one build per run.
3. **A larger box.** It runs more at once on one machine, with every stack sharing one Docker engine and one memory, beside the box's sessions. It grows to a size and stops there.
4. **CodeBuild on demand, with no fleet.** Every build starts on a cold machine that installs the toolchain and pulls every image again, which a 20-minute gate pays on every run.

## Decision outcome

### The user's

- **Every run of the queue goes to the pool.** "No, no standing machine has room." "It's configured to run on Code Build." "It's for my Willdan project, which is running Docker end-to-end tests".
- **The pool sets how many run at once, and it holds about ten.** "Our queues are capped: we only run about four at a time". "If we could have ten different machines all running compiles and tests at the same time, we could expand the capacity of the merge queue itself." "I know that any given merge might still take 20 to 40 minutes".
- **The guide and the function come first, for both pools, after the rebuild.** "We would start by just writing the documentation into the doc site and making sure that the functionality exists. We cover GitHub Actions and Code Build. Probably it should be done after the refactor based on the code review."
- **A trial, then a fleet for the box, then Switchboard.** "Then ad hoc, I would have my EC2 box set up Code Build temporarily so we could be testing it. Maybe even we would do that from this box. Sluice would do it just to test that it works." "Then we would set up a one-off fleet for the box so that we can improve our capacity. Once we know it's working, it's something we could build in as a switchboard feature later."

### agentplot-design's

- **CodeBuild is a pool profile.** The policy names a CodeBuild project and its fleet. The lead starts one build per run, the build's start is its claim and its end its report, and a build CodeBuild loses or stops is a lapsed claim (77, 88). Which instance takes a build is CodeBuild's.
- **A gate that trusts only a pool's signer runs every check on the pool.** No standing executor signs as a signer that gate trusts, so none can take its runs (107 as written). A pool's runs take no part of a host's bound (71). How many run at once is the branch's own `parallel`, which the guide tells a branch on a fleet to set to the fleet's maximum.
- **A branch on a fleet tests its combinations side by side.** Under `batch = true` a branch tests one combination at a time, so ten machines would run one of main's runs at a time. The guide tells a branch on a fleet to test with `batch = false` and `parallel` at the fleet's maximum. Each submission then lands about one run after it was submitted while the runs pass. Switchboard's policy is Switchboard's to change.
- **No candidate's code and no session can reach the signing key.** A build that runs a candidate's checks holds no key. A step that runs none of the candidate's code signs only the keys the candidate's tree bears out, as the GitHub Actions pool's run and sign jobs do (`actions-pool`). It signs with a key in AWS KMS that only that step's role may use. The policy lists the key's public half, `[signers.<name>] kms`, beside `ssh` and `github`, so every host verifies a run from the records alone and calls no AWS service (78, 141). The lead starts builds as an identity no session can use, and that identity cannot change what a build runs: no buildspec, image or environment of its own (C.8).
- **A warm instance starts the checks at once.** The fleet keeps a base of instances running. Each keeps, between builds, the Docker image layers, the toolchain's shell and the task runner's cache, so a build on a warm instance installs nothing and pulls no image it already holds. The fleet scales out to its maximum when builds wait. Builds past the maximum queue for an instance, since on-demand machines start cold and the branch's `parallel` already holds its runs to the fleet's size.
- **A build leaves its instance clean.** It runs the queue's release before it ends, whatever its outcome (record 0016), since the fleet's next build runs on the same instance. A run that reaches a container engine on a reused machine is never confined (79, C.8), as the box's open runs are not today.
- **Pools are built after the rebuild and the queue's home.** A launched job's records reach the branch through its signer's ref (109), and record 0117 redesigns where a queue's records live before anything more changes how they move. So `actions-pool`, the CodeBuild pool and the guide that covers both are built after `bolt/sluice-runs-one-host-process` and that redesign. The trial runs sluice's own CodeBuild tests against a project made by hand in an account of Willdan's that the box or deneb reaches.
- **The fleet's size waits on measurements.** Each instance has the memory one integration stack needs, as the box's runs show it. The base and the maximum come from the trial's run times and the queue's measured waiting (principle 15). They go to the user before the box's fleet is made, since a fleet bills Willdan by the instance-minute, from each instance's start, with an hour's minimum.

### Consequences

- The queue's capacity grows with the fleet, not with the box, and the box's sessions keep their machine.
- A failed run on a branch tested side by side throws away the combinations stacked on it, which costs fleet minutes and no one's wait beyond that run.
- A warm instance's caches serve every build the fleet runs, and every project in the account that shares the fleet. The task runner's cache decides no verdict (108), and image layers are fetched by digest.
- The capacity arrives after the rebuild and the queue-home redesign, not before.
