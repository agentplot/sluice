# Proposal

## Why

sluice runs moon in its CI mode (ADR 0044), so a task a repository marks
`runInCI: false` is never run, and a branch that requires one reads it
missing on every combination. Nothing says so when work is submitted:
the submission is queued, tested, and failed, and so is every one after
it. On 2026-10-21 and 2026-10-22 switchboard-kit's `main` required
`root:integration`, which its `moon.yml` kept out of CI, and sluice
0.36.4 failed every landing on `main` (#399, #402), so none of the kit's
bolts could land. The one warning sluice gives, in `sluice status` and a
waiting submit, comes only after a run has been tried and failed, and a
submit with `--no-wait` never shows it (madswan-design's signal
2026-10-23-madswan-design-8330b5b3/02, from wldn-ops's
2026-10-22-wldn-ops-68499b75/01).

## What Changes

- `sluice submit` refuses a submission when its branch's policy requires
  a check that the runner, in the submitting worktree, keeps out of CI.
  The refusal names each such task, the policy line that requires it
  (the line of `.config/sluice.toml` on the branch, and the tag when the
  task is required through one), and how to go on. Nothing is queued and
  nothing is published; it exits as any refusal does.
- The same refusal holds on a branch that takes pull requests: nothing
  is pushed and no request is opened or marked ready.
- Whether a task runs in CI is read from the submitting worktree, which
  holds exactly the head (2). A submission that lets the task run in CI
  is taken, so the fix can land through sluice. The gate itself, the
  required checks and their lines, is read from the branch's current
  revision, as always (56).
- A policy whose required checks all run is taken exactly as before. A
  branch whose runner is a command has nothing to ask.
- When the runner cannot be asked (its toolchain or its query fails),
  the submission is taken as before, with a warning saying why the check
  could not be made.
- The check seam gains one operation: given a worktree, the policy's
  `[tests]` and the required checks, the tasks among them the runner
  keeps out of CI, each with the required entry that names it. The moon
  profile answers it from `moon query tasks`; every other runner answers
  none.
- The policy keeps the line of each required entry, so a refusal can
  name it.
- The warning in `sluice status` and a waiting submit stays, for a
  combination whose tree keeps a task out though its submission did not
  (an entry ahead of it changed `moon.yml`).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `moon-runner`: a submission to a branch whose required checks include
  a task moon keeps out of CI is refused at submit, naming the task and
  the policy line.

## Impact

- Code: the check seam (`crates/sluice-seams`), the core's submit and
  its refusals (`crates/sluice-core/src/ops.rs`), the policy's parse
  (`policy.rs`), the moon and check profiles, the pull-request submit
  (`profiles/github/pulls.rs`), the CLI's words, the fakes and the
  conformance suite.
- Docs: ADR 0047; `docs/architecture/core.md` (core.15's details); the
  site's configuration and commands pages; the using-sluice skill's
  rules and submit reference.
- This repository's own policy requires no task kept out of CI (its one
  `runInCI: false` task, `root:screenshots`, carries no tag), so its own
  submissions are taken as before.

## Out of scope

- A request opened by hand on a branch that takes pull requests. Intake
  takes it in a round, with no worktree to ask the runner in, so it is
  tested and fails as today. Refusing it needs a place to read its head
  in, and waits on a change of its own.
- `sluice submit --check` and the forecast, which form a submission
  without writing one; they could name the same tasks, and wait on a
  change of their own.
- Several heads submitted as one act (125), which is not on `main` yet:
  when it lands, a head refused this way refuses the whole act, as 125
  says of any head that cannot be taken.
- Checking a policy when it is changed, before any submission meets it.
  A change to the policy is itself judged by the policy it changes (56),
  so it meets this refusal at its own submit.

## Touches

```
crates/sluice-seams/src/lib.rs
crates/sluice-core/src/ops.rs
crates/sluice-core/src/policy.rs
crates/sluice-core/src/lib.rs
crates/sluice/src/lib.rs
crates/sluice/src/words.rs
crates/sluice/src/profiles/checks.rs
crates/sluice/src/profiles/moon.rs
crates/sluice/src/profiles/github/pulls.rs
crates/sluice-fakes/src/lib.rs
crates/sluice-conformance/**
crates/sluice/tests/**
docs/adr/0047-*.md
docs/architecture/core.md
site/src/content/docs/architecture/rules.md
site/src/content/docs/reference/configuration.md
site/src/content/docs/reference/commands.md
plugins/sluice/skills/using-sluice/SKILL.md
plugins/sluice/skills/using-sluice/references/submit.md
```
