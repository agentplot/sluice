# Pull requests

**Choose it when** the team lands on the branch through pull requests
today, so it keeps them, or when work should get a pull request on GitHub
before it lands: for review, for other systems' checks (QA, CI, a
deployment), or so each landing has a page. Needs
[remote-branches](remote-branches.md), [peers](peers.md) for the branch's
signers and the [github-app](github-app.md).

## Keys

```toml
[branches.main]
remote = "origin"
required = ["#check-fast", "#check-tools"]   # what sluice tests

[branches.main.pull_requests]
reviews = 1                  # approving reviews on the latest commit
checks = ["qa/approved"]     # checks other systems post, by the name GitHub shows
log_lines = 40               # optional: post the end of a failed test's output
```

- `reviews` and `checks` decide when sluice takes a pull request, never what
  it tests: that is `required`.
- They are read from the branch itself, so a pull request cannot loosen its
  own.
- The table needs `remote`; sluice refuses it without one.
- It also needs the branch's queue on its remote and the forge's round job:
  `[signers]` for the hosts that run its tests, with `trust` naming them,
  and `[pools.forge]`. Without them sluice refuses that branch alone,
  saying what to add, and opens no pull request for it.
- `log_lines` posts that many last lines of a failed test's output in the
  pull request's report, scrubbed of what looks like a secret. Leave it out
  unless the tests' output is safe for every reader of the repository: the
  report then says where the output is kept on the host.

## How it behaves

- `sluice submit` / `wt submit` push the worktree's branch and open or update
  its pull request instead of queueing the head.
- A pull request waits while it is a draft or misses a review or check, then
  joins the queue. It never waits inside the queue.
- One a submit opened names its agent in its description. Sent back
  (blamed, timed out, in conflict, withdrawn), or moved by commits no submit
  pushed, sluice marks it a draft, and it waits for the next submit from its
  branch or a person's ready mark: `waiting: the next submit`. A submit
  marks it ready for review at the commits it pushes, the same ones
  included.
- One a person opened waits for its conditions again on new commits; sent
  back, or withdrawn by the operator or a planner, it is taken again on new
  commits, or once marked a draft and then ready for review. Withdrawn, its
  check names who withdrew it: `waiting: withdrawn by the operator`.
- No host takes the pull requests in: the forge's round job does, and runs
  the branch's rounds, with every host off. A submit opens or updates the
  pull request, marks it ready, wakes the round job and follows it by
  sluice's check; hosts whose results the branch trusts run its tests.
- sluice posts one check named `sluice` with the state (`waiting: …`,
  `queued behind …`, `testing …`, `failed: …`, `conflict: …`, `landed as …`)
  and comments when it sends one back.
- Once a combination has a result, sluice posts its report: one comment in
  the conversation, edited as it changes, the same text as the `sluice`
  check's details, and a check per required test per system
  (`sluice / app:test (linux-x64)`). Don't require these checks in a
  ruleset.
- The app needs Checks and Pull requests read and write; `sluice forge
  check` says so.
- Pull requests from forks are never taken.

## Set up the round job

The round job is a GitHub Actions workflow in the repository. A team that
lands by pull request today gets it as part of setup:

1. Write `[signers]` for the hosts that run the branch's tests, as
   [peers](peers.md) says, with `trust` naming them, so the branch's queue
   is kept on its remote. Each host needs its `[signing]` key first.
2. Run `sluice init actions`. It writes `.github/workflows/sluice.yml` and
   adds `[signers.forge]` and `[pools.forge]` to the policy; it refuses a
   policy with no `[signers]`.
3. Print what it says to set on GitHub for the user: the `sluice`
   environment admitting the default branch alone, its `SLUICE_APP_ID`
   variable and `SLUICE_APP_KEY` secret, and the app's Actions write
   permission. These are the user's to set.
4. Land the workflow and the policy, then `sluice forge check <branch>`
   says whether each is set.

The round job runs on each submit, on each recorded result and every five
minutes, which finds a pull request a person opened. An intake mark an
older sluice left on the remote is read by nothing; `git push origin
:refs/sluice/intake/<branch>` removes it.

## GitHub settings

- Leave GitHub's merge queue off.
- Required approvals and status checks move from the ruleset, or branch
  protection, into `[branches.main.pull_requests]`: read them from GitHub,
  write them as `reviews` and `checks`, then remove them there on the
  user's word.
- No setting lets GitHub's merge button land while sluice only tests:
  what the button lands was never tested with what landed just before it.
  Asked for one, say there is none, and why, and offer this table.
- Turn on "dismiss stale pull request approvals" if new commits need a new
  review.
- The repository's own CI may keep running on pull requests; name its job in
  `checks` if it must pass first.

Reference: <https://agentplot.github.io/sluice/guides/pull-requests/>.
