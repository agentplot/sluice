# Proposal

## Why

A result that a check failed is recorded under the check's key whatever
caused it, and a branch reads every result under a key from then on (19,
69). When the host fails rather than the check, for example a container
engine that is down, a launch that stalls past a test's own timeout, or
a program that never starts, that fail stays under the key. Every later
pass makes the key flaky, and `main`'s policy reads a flaky key as
failed, so the head can never land again without a change to the task's
inputs.

This happened on 2026-10-17. A `sluice exec main` recorded fails for
`cli:e2e` and `cli:tools`, and eight minutes later the gate passed both
on the same tree. The submission still failed, because its keys held a
pass and a fail. sluice cannot know what a repository's checks need, so
the fix is a way for the repository and the check to say "could not
run", and a reading that keeps such a run off the key. It rests on the
unratified proposal `design/sluice/proposals/could-not-run-1.md` in
agentplot/blueprints (clauses 114 to 116, amended 84).

## What Changes

- A run of a check can end with no outcome (60, could-not-run-1 114) in
  three ways:
  - the check's program could not be started;
  - it exited with a code the policy's new `[tests] not_run` names;
  - for a check that did not pass, the host failed the policy's new
    `[tests] ready` command after the run.
- Such a check is listed with its reason and recorded under no key. Its
  candidate runs again. No submission is blamed until the third run in
  a row in which its checks could not run, and then the culprit is told
  each check that could not run, and why (could-not-run-1 115).
- `[tests] ready` also runs before a run of the checks. A host that
  fails it is refused the run, which records nothing (84, amended). A
  command run, `sluice exec <branch> -- <cmd>`, runs neither `ready`
  nor `not_run`.
- `sluice exec` says which checks could not run and why. A session
  run's last result reads `could not run`.
- `sluice set-aside`: the operator sets aside one recorded result with
  a reason (could-not-run-1 116). It is kept in `set-aside.jsonl` beside
  `results.jsonl`. The result's line stays, and no gate reads it from
  then on. The Claude Code plugin's guard refuses it from an agent's
  session.
- sluice's own policy declares `ready = ["docker", "info"]`. `cli:e2e`
  and `root:screenshots-scene` get explicit inputs.
- `CLAUDE.md` gains "Running checks". The `using-sluice` skill says to
  run suites through `sluice exec` so parallel sessions share the host,
  and that a small call may run directly.
- ADR 0031, core.7's Details, and the site's pages on results and the
  policy.

Out of scope:

- Recognising a host fault that neither `ready` nor `not_run` declares.
  That is what set-aside is for.
- Signing a set-aside on a signer's ref. That waits on the records
  across hosts being built (many-hosts-3).
- A bound the host's own setting can change. The bound is three runs in
  a row for now.
