---
title: GitHub Actions
description: The hybrid for a repository that stays on GitHub, as an adoption path.
sidebar:
  order: 5
  badge: { text: 'design', variant: 'caution' }
---

:::caution[A design under review]
Under review in [agentplot/blueprints](https://github.com/agentplot/blueprints),
`design/sluice/models/many-hosts-1.md` and
`design/sluice/proposals/many-hosts-2.md`. Not ratified, not built. This
is an adoption path for repositories on GitHub, not the direction: a
fleet of sluice's own hosts is.
:::

sluice stays the lead and forms the matrix; GitHub Actions is the pool
it pushes runs to, the [launched shape](/sluice/many-hosts/architectures/)
of executor. A developer sees a GitHub App and two workflows, and keeps
the pull request flow they already have.

## What a developer installs

- **A GitHub App, "sluice".** Installing it is the setup. It holds the
  bypass on protected branches, posts the status check on each PR head,
  and comments the queue position. The App is stateless: it holds its
  key and the webhook, and no queue.
- **Two workflows, written by `sluice init github`.** `check.yml` runs
  the tests of one combination on the systems the policy names.
  `step.yml` runs one round of the queue.
- **The policy file**, the same `.config/sluice.toml` a laptop reads. A
  bolt's branch is gated on the agent's machine; `main` is gated by the
  same file on GitHub.

## The pull request flow

sluice replaces GitHub's merge, not GitHub's pull request.

- **A PR is the submission.** Opening it submits the head to the
  branch; closing it withdraws. Reviews and comments are untouched, and
  whether work may be submitted at all, approval included, is decided
  before it reaches sluice.
- **The status check is sluice's result.** The checks run on the
  combination sluice will land, not on the PR head alone.
- **The merge button is replaced.** sluice lands by pushing the branch
  with the App's identity, from the revision it tested on, and GitHub
  marks the PR merged because its head is reachable. Branch protection
  restricts pushes to the App, so nobody else can press it. A hand merge
  that slips through is refused as a landing, not tested.
- **Incompatible with GitHub's own merge queue, and with squash or
  rebase merges**, on the same branch: sluice is the queue, and a landing
  is a merge commit carrying its result.

## `check.yml`: a launched executor

One dispatch per combination, carrying the combination's ref and run
id; the workflow's matrix expands the systems. The lead reads the
required systems from the policy and passes them, so the matrix is
still sluice's.

```yaml
name: check
on:
  workflow_dispatch:
    inputs:
      ref:     { required: true }   # refs/sluice/combinations/main/<id>
      run:     { required: true }   # sluice's run id
      systems: { required: true }   # JSON list from the policy
      tasks:   { required: true }   # JSON list of moon targets
permissions:
  contents: write                   # to push the result ref only
jobs:
  run:
    strategy:
      fail-fast: false
      matrix:
        system: ${{ fromJSON(inputs.systems) }}
    runs-on: ${{ matrix.system == 'aarch64-darwin' && 'macos-latest'
             || matrix.system == 'aarch64-linux'  && 'ubuntu-24.04-arm'
             || 'ubuntu-latest' }}
    steps:
      - uses: actions/checkout@v5
        with: { ref: ${{ inputs.ref }} }
      - uses: cachix/install-nix-action@v31
      - run: nix profile add nixpkgs#devenv
      - uses: actions/cache@v4
        with: { path: target, key: "${{ matrix.system }}-${{ hashFiles('Cargo.lock', '.prototools') }}" }
      - id: moon
        run: devenv shell -- moon run ${{ join(fromJSON(inputs.tasks), ' ') }}
        continue-on-error: true
      - run: |                      # the result, from moon's run report
          sluice report --run "${{ inputs.run }}" --system "${{ matrix.system }}" \
            --executor "gha:${{ github.run_id }}" --outcome "${{ steps.moon.outcome }}" \
            > result.json
          blob="$(git hash-object -w result.json)"
          tree="$(printf '100644 blob %s\tresult.json' "$blob" | git mktree)"
          git push origin "$(git commit-tree "$tree" -m result):refs/sluice/results/${{ inputs.run }}/${{ matrix.system }}"
```

- **The combination reaches the runner** through
  `refs/sluice/combinations/<branch>/<id>`, which the lead pushed. The
  job checks out that ref, not a branch.
- **The job is the claimant.** Its start is the claim, its exit the
  result. A job GitHub cancels or loses is a lapsed claim.
- **The result comes back as a ref**, a small JSON per task with moon's
  hash as the key, the outcome, the system and the executor
  `gha:<run id>`, under `refs/sluice/results/<run>/<system>`. The run's
  log stays on GitHub, reachable by the run id.
- **Vouching is the API.** Before the lead reads a result it confirms it
  against `gh run view <id> --json conclusion,headSha`. A result ref no
  run of GitHub's vouches for is ignored. That makes `gha` an executor
  identity the policy can name, confined by construction: a fresh VM per
  job.

The policy on the lead's side:

```toml
[executor]
kind = "launched"
launch = ["gh", "workflow", "run", "check.yml", "-f", "ref={ref}", "-f", "run={run}",
          "-f", "systems={systems}", "-f", "tasks={tasks}"]
results = "refs/sluice/results/{run}/"
vouch = ["gh", "run", "view", "{executor}", "--json", "conclusion,headSha"]

[branches.main]
required = ["#check-fast", "#check-tools", "#check-e2e"]
systems = ["aarch64-darwin", "x86_64-linux", "aarch64-linux"]
trust = ["gha"]
```

## `step.yml`: the lead on GitHub

A workflow whose only step is `sluice step main`, run on
`workflow_dispatch` from a submit, on `workflow_run` when a check
finishes, and on a schedule as a sweeper, under a concurrency group so
one round runs at a time. Because all state is in refs, a round needs no
memory between jobs, which is what `sluice step` was built for. With the
lead on GitHub, nothing of sluice runs anywhere else; with the lead on
an operator's host, `step.yml` is not needed.

## The state is in the repository

Every other hosted queue holds the queue in its own database, or in
temporary branches nobody can inspect. Here the queue, every claim,
every run's result and the log are commits under `refs/sluice/` in the
repository itself, and the landing commit carries its result as
trailers.

```sh
git log refs/sluice/state/main      # every decision the queue made, in order
```

- Nothing to trust blindly: a dispute about why something landed or was
  blamed is a diff.
- Nothing to lose: uninstall the App and the whole history is still in
  the repository; move the repository and the state moves with it.
- The same state the laptop reads: `sluice status main` run locally
  against the remote refs shows exactly what the App sees.

`sluice init` writes a fetch refspec for `refs/sluice/*`, since clones do
not fetch it by default and GitHub's pages do not show it.

## The costs, honestly

A round costs a job start, so the queue moves in half-minute steps, and
nix plus devenv add a minute or two per job before the cache helps. A
landing is minutes behind the last green check. That is fine for `main`
on a big project and wrong for a bolt's branch, which gates in seconds
on one machine and stays there. A fair mix is the standing shape for the
system the operator's own machine is, and the launched shape only for
the systems the fleet has no host for.

## To verify before promising it

- Push events do not fire for refs outside `refs/heads` and `refs/tags`,
  so the stepper is driven by dispatch and `workflow_run`, never by
  pushes to the state ref.
- Whether a ruleset can grant bypass to the Actions token, or only to an
  App's. If only an App's, the App is required rather than merely nicer.
