# Proposal

## Why

A branch on GitHub usually takes its work as pull requests, which wait
for review, for a QA team and for other systems' checks before they may
merge. sluice has no pull requests: an agent's work goes straight into
the queue, nobody can review it there, and a pull request opened by hand
never reaches sluice. The design is in agentplot/blueprints,
`design/sluice/proposals/pull-requests-1.md` (the C.5 pull request
driver, C.7 `pull_requests`, S21 to S24), and in the site guide
`site/src/content/docs/guides/pull-requests.md`. It needs the branch to
live on its remote, which `main-on-github` builds, and sluice's own
identity on GitHub, which `github-app` builds.

## What Changes

- A branch's policy may turn pull requests on with a
  `[branches."<pattern>".pull_requests]` table holding the conditions a
  pull request meets before sluice takes it: `reviews`, approving reviews
  on its current head, and `checks`, names of checks other systems post
  on its head. The table needs `remote` on the same branch.
- `sluice submit` and `wt submit` to such a branch push the agent's
  branch and open a pull request into it, or update the one already
  open, and do not submit it: the pull request waits.
- Each round, for such a branch, finds the open pull requests into it
  from branches of the repository, including ones opened by hand.
  A pull request is submitted when it is out of draft and every
  condition holds on its head; until then it waits. Pull requests from
  forks are never submitted.
- A new head on a submitted pull request withdraws its submission and it
  waits again; moved back to draft or closed, it is withdrawn.
- sluice posts the pull request's state on its head as a check run
  named `sluice`, as its GitHub App: what it waits for and its forecast while waiting, then
  queued, testing, failed, conflict or landed; it comments when a
  submission is blamed or conflicts. A blamed or conflicting pull
  request is submitted again only on a new head or after it is moved to
  draft and back.
- The landing is the push of the branch from `main-on-github`; GitHub
  marks the pull request merged because its head is in the branch.
- A waiting `wt submit` reports the pull request's states and returns
  when it lands or is sent back.
- Site: the configuration reference gains `pull_requests`; the commands
  and state references show pull requests; the guide
  `guides/pull-requests.md` loses its "designed, not built" banner and
  matches what was built. An ADR records the decisions.

Out of scope, each its own later change:
- GitHub Actions as sluice's executor and stepper, with the queue kept
  in the repository (`many-hosts-2.md`, the forge as the pool).
- Webhooks: rounds ask GitHub; sluice runs no listener.
- Pull requests from forks, and checks run on pull requests that wait.

## Capabilities

### New Capabilities
- `pull-request-intake`: which pull requests sluice takes into a
  branch's queue and when: the conditions, drafts, new heads, closing,
  forks, and resubmission after being sent back.
- `pull-request-status`: what sluice shows on a pull request: its posted
  state, its forecast while waiting, its comments, and its merge by
  landing.
- `pull-request-submit`: `sluice submit` and `wt submit` opening or
  updating a pull request for an agent's work, and a waiting submit
  following it to its outcome.

### Modified Capabilities

## Impact

- `crates/sluice/src/profiles/github/` (core.1: the forge named only
  there): the forge's API as the app, with the token from `github-app`,
  the pull request driver, behind a small interface with a fake for
  tests.
- The sluice crate's runner: an intake pass before each round of a
  branch with `pull_requests`; `sluice run` serves those branches even
  with no records yet.
- `crates/sluice-core/src/policy.rs`: the `pull_requests` table.
- Depends on `main-on-github` (the `remote` key and the pushed landing)
  and `github-app` (the token, and check runs, which only an app may
  post).
- `site/src/content/docs/reference/{configuration,commands,state}.md`,
  `site/src/content/docs/guides/pull-requests.md`, `docs/adr/`,
  `docs/plans/phase-1-pass-2.md` (piece 24's row).
