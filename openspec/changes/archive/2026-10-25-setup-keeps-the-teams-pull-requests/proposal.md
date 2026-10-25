# Proposal

## Why

A team that lands its work through pull requests today can keep doing so
under sluice, but nothing tells it so, and the setup never asks. A
newcomer on an onboarding call could not say when sluice opens a pull
request, when one is a draft, or how the team's own reviews and checks
carry over, and asked for a way to keep "the traditional way" (signal
2026-10-23-madswan-operator-bea2f287/03). The design answers that there
is one way and it keeps the workflow whole: requests are opened as
always, by a submit or by hand, drafts wait, the reviews and other
systems' checks the policy names must hold, and only who lands it
changes, sluice's push instead of the host's merge button (127, S62).
sluice already behaves so (113, 120, C.5); what is missing is the setup
asking for it and a page that says it.

## What Changes

- The setup skill asks, for each branch it gates that lives on GitHub,
  whether the team lands on it through pull requests today. Where it
  does, the skill reads what GitHub requires of the branch now
  (approving reviews, required status checks) and offers them as the
  branch's `[branches.<b>.pull_requests]` conditions, writes the table,
  and moves the requirements out of GitHub's settings into the policy,
  asking before it changes any GitHub setting (127, C.7).
- The setup says what changes for the team and what does not: requests,
  drafts, reviews and other systems' checks go on as before; nobody
  presses merge, uses GitHub's merge queue, or merges by hand on that
  branch; sluice tests each request with what is ahead of it and lands
  it, and GitHub marks it merged (127, S62).
- Asked for a way in which GitHub's button lands and sluice only tests,
  the setup says sluice has none, and why: what the button lands was
  never tested with what landed just before it (127, I1).
- The setup's read-back of the policy names a branch's pull request
  conditions beside its checks.
- The pull requests guide opens with one section a team that already
  works through pull requests can read alone: it keeps them, when sluice
  opens a request (a submit) and when a person does (by hand, as
  always), when a request is a draft and waits and when it waits for its
  conditions, that sluice's push lands it, and that nothing else about
  a developer's day changes. The guide and the GitHub guide say plainly
  that there is no mode in which the button lands.
- The pull-requests profile the setup reads carries the same.

Out of scope, each waiting on a later change:

- Any change to how sluice takes, tests or lands a pull request. The
  behaviour is built (pull-request-intake, pull-request-submit); this
  change only asks for it at setup and documents it.
- A condition the policy cannot name today, such as GitHub's "review
  from code owners" or a required review count per path. The setup says
  plainly that such a requirement has no condition in sluice's policy;
  giving it one waits on the design (C.7).
- Forges other than GitHub, which sluice does not speak (C.5).
- `sluice forge check` reading the ruleset for required reviews, status
  checks or a merge queue. The setup reads them with `gh`; a check that
  refuses them waits on its own change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plugin-setup`: the setup asks whether a gated branch lands through
  pull requests today and, where it does, writes its pull request
  conditions from what the team requires now; it says what changes and
  what does not, and that no mode lets the host's button land; the
  read-back names a branch's pull request conditions.

## Impact

- `plugins/sluice/skills/setting-up-sluice/SKILL.md` and its
  `references/profiles/pull-requests.md` and `references/read-back.md`.
- `site/src/content/docs/guides/pull-requests.md` and
  `site/src/content/docs/guides/github.md`.
- No code, no policy key, no CLI output changes.

## Touches

```
plugins/sluice/skills/setting-up-sluice/SKILL.md
plugins/sluice/skills/setting-up-sluice/references/profiles/pull-requests.md
plugins/sluice/skills/setting-up-sluice/references/read-back.md
site/src/content/docs/guides/pull-requests.md
site/src/content/docs/guides/github.md
```
