# Pull requests

**Choose it when** work should get a pull request on GitHub before it lands:
for review, for other systems' checks (QA, CI, a deployment), or so each
landing has a page. Needs [remote-branches](remote-branches.md) and the
[github-app](github-app.md).

## Keys

```toml
[branches.main]
remote = "origin"
required = ["#check-fast", "#check-tools"]   # what sluice tests

[branches.main.pull_requests]
reviews = 1                  # approving reviews on the latest commit
checks = ["qa/approved"]     # checks other systems post, by the name GitHub shows
```

- `reviews` and `checks` decide when sluice takes a pull request, never what
  it tests: that is `required`.
- They are read from the branch itself, so a pull request cannot loosen its
  own.
- The table needs `remote`; sluice refuses it without one.

## How it behaves

- `sluice submit` / `wt submit` push the worktree's branch and open or update
  its pull request instead of queueing the head.
- A pull request waits while it is a draft or misses a review or check, then
  joins the queue. It never waits inside the queue. New commits send it back
  to waiting.
- sluice posts one check named `sluice` with the state (`waiting: …`,
  `queued behind …`, `testing …`, `failed: …`, `conflict: …`, `landed as …`)
  and comments when it sends one back.
- Pull requests from forks are never taken.

## Keep `sluice run` going

A pull request opened by hand has nobody waiting on it. Keep `sluice run`
running (a terminal, a login item, a service manager) so each round reads
open pull requests and moves the queue. sluice starts no service itself.

## GitHub settings

- Leave GitHub's merge queue off.
- Require no approvals or status checks in the ruleset; put them in
  `[branches.main.pull_requests]`.
- Turn on "dismiss stale pull request approvals" if new commits need a new
  review.
- The repository's own CI may keep running on pull requests; name its job in
  `checks` if it must pass first.

Reference: <https://agentplot.github.io/sluice/guides/pull-requests/>.
