# Design

## Context

sluice already keeps a pull request workflow whole: a branch whose policy
has a `pull_requests` table takes requests opened by a submit or by hand,
leaves drafts waiting, waits for the reviews and other systems' checks the
table names, tests a ready request with what is ahead of it and lands it
with its own push (pull-request-intake, pull-request-submit; 113, 120,
C.5). What the design now asks of setup (127, S62) is that it ask
whether the team works this way and write the table, so the workflow is
kept from the first day, and that the guide say so in one place.

The setup skill on this branch already interviews the user per branch
("which branches agents will submit to, and what must pass") and reads
the policy back in plain words before committing it (`references/read-back.md`).
This change adds one question to that interview, and one line to the
read-back. The pull requests guide on main already describes every
behaviour; it lacks an entry point for a team that already uses pull
requests and a plain statement that no mode leaves the merge button in
charge.

## Goals / Non-Goals

**Goals:**

- One question, asked per gated branch on GitHub, whose yes leads to a
  written `pull_requests` table seeded from what GitHub requires today.
- The team told, before the policy is committed, what stays and what
  changes: only who lands.
- One section of the pull requests guide a team lead can read alone.

**Non-Goals:**

- Any new policy key, CLI behaviour or check. No code changes.
- Translating a GitHub requirement the policy has no key for.

## Decisions

### Ask per branch, after the checks question

The skill already asks per branch what must pass. The pull request
question joins it there, asked only for a branch whose policy names a
remote (the table needs one; pull-request-intake refuses it otherwise).
Asking once for the whole repository was considered and set aside: a team
commonly reviews `main` and lets agents land on `bolt/*` straight from
their worktrees, and 127 speaks of "that branch's conditions".

### Seed the conditions from GitHub, read with `gh`

A team that lands by pull request today already has its conditions in
GitHub: a ruleset's `pull_request` rule
(`required_approving_review_count`, `require_code_owner_review`,
`dismiss_stale_reviews_on_push`), its `required_status_checks` rule
(each check's `context`) and a `merge_queue` rule, read for a branch with
`gh api repos/{owner}/{repo}/rules/branches/{branch}`; or classic branch
protection, `gh api repos/{owner}/{repo}/branches/{branch}/protection`
(`required_pull_request_reviews.required_approving_review_count`,
`required_status_checks.checks[].context`), which answers 404 for a
branch with none and needs admin rights to read. The skill reads both,
offers the highest review count as `reviews` and the union of check
names as `checks`, leaving out any named `sluice` or starting
`sluice / `, which are sluice's own (113), and shows the user what it read
before writing. Asking the user to recall the settings was the
alternative; reading them is quicker and right more often, and the user
still confirms. Where `gh` is missing or can't read them, the skill asks.

### Move the requirements, never duplicate them

The guide already says to require no approvals or status checks in the
ruleset and to leave GitHub's merge queue off: the conditions belong in
the policy, read from the branch itself so a request cannot loosen them
(56), and a merge queue of GitHub's would be a second queue landing what
sluice never tested. The skill proposes the ruleset edits after the policy
holds the conditions and before the app becomes the only bypass, in the
existing GitHub order of step 5, and makes none unasked, as the skill's
opening rule already says of GitHub settings.

### Say what has no condition

`require_code_owner_review`, required review per path, required
deployments and signed commits have no key in C.7. The skill says sluice
will not wait for them; it does not approximate one (for example by
counting code owners into `reviews`). Approximating would land a request
the team's own rule would have held.

### No test-only mode, stated where it would be asked

127 forbids offering a way in which GitHub's button lands and sluice only
tests. Nothing builds one today; the risk is a setup or a reader
improvising one (sluice's checks required in a ruleset, the button left
on). The skill, the profile and the guide each say there is none, with
the reason, in the place a person would look for it: the skill's pull
request question, the profile's "GitHub settings", the guide's opening
section and its "Merged with GitHub's button" line, and the GitHub guide's
"Set up GitHub".

### The guide's entry point is a section, not a page

The intent asks the guide to say it "in one page". The guide is that
page; its new first section, "Keeping your team's pull requests", answers
the team's four questions (do we keep them, when does sluice open one,
when is one a draft and waits, what else changes) in a few short
paragraphs and links down to the detail. A separate page was the
alternative; it would repeat the guide and drift from it.

## Risks / Trade-offs

- [The bolt's copy of the guide is older than main's] → The code stage
  writes the new section against the guide as it stands on the branch;
  the section names only behaviour both copies describe (drafts, waiting,
  conditions, sluice's push), so it holds when main is taken in.
- [`gh` reads of rules need rights the user may lack] → the skill falls
  back to asking, and says which read failed.
- [A team relies on code-owner review] → stated before commit, so the
  user decides with the gap in view; giving it a condition is a design
  question for C.7.

## Open Questions

None that change what is built.
