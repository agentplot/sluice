## Why

A pull request `sluice submit` opens is titled by the first line of the
first commit its branch carries beyond the branch it lands on. A bolt's
branch begins with its proposal's commit, so a request that lands
thousands of lines of code reads in GitHub's list as a documentation
change. The title should name what lands: the subject the submitter
gives, or the branch's own name, never a commit's message (118, C.5
pull request; record 0064).

A subject is the submitter's word for what lands wherever it lands. On
a branch with no pull requests it names the landing commit's work, and
in a group submit each head has its own (23, 125).

## What Changes

- `sluice submit` and `wt submit` take `--subject <text>`: one line, the
  submitter's words for what lands.
- A pull request a submit opens is titled by that subject, as written,
  or by the submitted branch's name where the submit gives none. No
  commit's message titles a request any more.
- A submit that gives a subject and updates a pull request sluice opened
  sets its title to the subject. A submit that gives none leaves the
  title as it is.
- A pull request a person opened keeps the title they gave it: no
  submit changes it, with a subject or without.
- The description is unchanged: it still carries the commits' messages
  (C.5).
- A group submit takes one `--subject` per `--worktree`, in the same
  order, or none. One subject for several worktrees, or any count but
  none or one each, is a usage error; under pull requests each request
  is titled by its own subject or its branch.
- Each submission keeps its subject in the branch's records, and its
  landing commit names its work by it: the policy's landing subject
  fills `{from}` with the subject, or with the branch where the submit
  gave none. The branch stays in the `Sluice-From` trailer. A
  submission taken from a pull request keeps the request's title as its
  subject.
- A subject that is empty or more than one line, or given with
  `--check` or `--status`, is a usage error.
- Requests already open keep their titles; the next submit with a
  subject retitles one sluice opened. Submissions queued before this
  change land named by their branch, as today.

Out of scope, with no later change planned for them yet: showing a
submission's subject in `sluice status`, the overview or the home
screen; and a policy whose landing subject names no `{from}`, which
names no subject either (design.md, Open Questions).

## Capabilities

### New Capabilities

- `landing-subject`: a landing commit names its entry's work by the
  subject the submit gave, or by its branch where it gave none.

### Modified Capabilities

- `pull-request-submit`: a submit titles the pull request it opens by
  its subject or its branch's name, retitles one sluice opened when it
  gives a subject, and never retitles one a person opened.
- `group-submit`: the act takes one subject per worktree, in order, or
  none, and titles each pull request by its own.

## Impact

- `crates/sluice-core/src/records.rs`, `ops.rs`, `text.rs`,
  `policy.rs`: the submission's subject, the submit and group requests
  that carry it, and the landing commit's `{from}`.
- `crates/sluice/src/lib.rs`: the `--subject` argument, repeated for a
  group, its usage errors, and passing it to the submit and the pull
  request driver.
- `crates/sluice/src/profiles/github/pulls.rs`: the title on opening
  and on updating, each head's own in a group; telling a request sluice
  opened from one a person opened; a request's title kept on the
  submission taken from it.
- `crates/sluice/src/profiles/github/forge.rs`, `rest.rs`, `fake.rs`,
  `app.rs`: the forge's edit carries a title; the forge names its own
  login.
- `crates/sluice/src/profiles/worktrunk/mod.rs`: `wt submit` forwards
  `--subject` as it forwards every argument (a test holds it).
- Tests under `crates/`, the site's pull requests guide, commands and
  configuration references, the using-sluice skill and the worktrunk
  reference.

## Touches

```
crates/sluice-core/src/records.rs
crates/sluice-core/src/ops.rs
crates/sluice-core/src/text.rs
crates/sluice-core/src/policy.rs
crates/sluice-conformance/src/**
crates/sluice-conformance/tests/copies.rs
crates/sluice-conformance/tests/peers.rs
crates/sluice-conformance/tests/places.rs
crates/sluice-conformance/tests/scenarios.rs
crates/sluice-conformance/tests/verdicts.rs
crates/sluice/src/lib.rs
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/src/profiles/github/forge.rs
crates/sluice/src/profiles/github/rest.rs
crates/sluice/src/profiles/github/fake.rs
crates/sluice/src/profiles/github/app.rs
crates/sluice/src/profiles/worktrunk/mod.rs
crates/sluice/tests/**
site/src/content/docs/guides/pull-requests.md
site/src/content/docs/reference/commands.md
site/src/content/docs/reference/configuration.md
plugins/sluice/skills/using-sluice/SKILL.md
plugins/sluice/skills/using-sluice/references/submit.md
plugins/sluice/skills/setting-up-sluice/references/profiles/worktrunk.md
```
