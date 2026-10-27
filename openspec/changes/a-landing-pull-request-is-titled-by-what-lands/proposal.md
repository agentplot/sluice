## Why

A pull request `sluice submit` opens is titled by the first line of the
first commit its branch carries beyond the branch it lands on. A bolt's
branch begins with its proposal's commit, so a request that lands
thousands of lines of code reads in GitHub's list as a documentation
change. The title should name what lands: the subject the submitter
gives, or the branch's own name, never a commit's message (118, C.5
pull request; record 0064).

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
- `--subject` with several `--worktree`s, `--check` or `--status` is a
  usage error. On a branch that takes no pull requests the submit goes
  ahead and says the subject titles nothing there.
- Requests already open keep their titles; the next submit with a
  subject retitles one sluice opened.

Out of scope: a subject for each request of a group submit (125), and
any use of the subject on a branch that takes no pull requests, such as
a landing commit's message. Both wait on a ruling from the design (see
design.md, Open Questions); no later change is planned for them yet.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pull-request-submit`: a submit titles the pull request it opens by
  its subject or its branch's name, retitles one sluice opened when it
  gives a subject, and never retitles one a person opened.

## Impact

- `crates/sluice/src/lib.rs`: the `--subject` argument, its usage
  errors, and passing it to the pull request driver.
- `crates/sluice/src/profiles/github/pulls.rs`: the title on opening
  and on updating; telling a request sluice opened from one a person
  opened.
- `crates/sluice/src/profiles/github/forge.rs`, `rest.rs`, `fake.rs`:
  the forge's edit carries a title; the forge names its own login.
- `crates/sluice/src/profiles/worktrunk/mod.rs`: `wt submit` forwards
  `--subject` as it forwards every argument (no change expected; a test
  holds it).
- Tests under `crates/sluice/tests/`, the pull requests guide on the
  site, the using-sluice skill and the worktrunk reference.

## Touches

```
crates/sluice/src/lib.rs
crates/sluice/src/profiles/github/pulls.rs
crates/sluice/src/profiles/github/forge.rs
crates/sluice/src/profiles/github/rest.rs
crates/sluice/src/profiles/github/fake.rs
crates/sluice/src/profiles/github/app.rs
crates/sluice/src/profiles/worktrunk/mod.rs
crates/sluice/tests/**
site/src/content/docs/guides/pull-requests.md
plugins/sluice/skills/using-sluice/SKILL.md
plugins/sluice/skills/setting-up-sluice/references/profiles/worktrunk.md
```
