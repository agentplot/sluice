# Phase 1: questions the build met

Things the statement and the walkthrough don't settle: the rulings made
on 2026-10-11, then choices made in the build that stand unless
overruled. Clause numbers are at the end of each, for reference only.

In the examples, A, B and C are three agents' submissions to one bolt
branch, in that order.

---

## Ruled

- **One queue per branch.** Every submission is tested on top of every
  submission ahead of it, A, A+B, A+B+C, and lands in that order. A
  submission is never tested apart from the ones ahead, so a test is
  never thrown away because the queue regrouped. *(clauses 8, 10; S6)*
- **Who runs sluice's rounds.** A waiting `wt submit` runs rounds; the
  submit operation itself only adds to the queue. Rule core.4 reworded;
  ADR 0011. *(clause 1)*
- **Policy defaults are conservative.** A key a `[branches]` table leaves
  out reads: `parallel = 4`, `batch = false`, `flaky = "fail"`,
  `timeout = "1h"`. *(clauses 27, 66)*
- **The machine's test cap.** `parallel` stays per branch. Each machine
  also caps the runs it holds at once, across every branch: default a
  quarter of its cores, never fewer than one, overridable per machine,
  not in the policy file. The cap is enforced where a run starts, which
  refuses past it; sluice's `capacity` call is a hint. Each run's task
  runner is told its share of the cores, so the machine is never
  oversubscribed. Pass 2's test-runner and moon profiles build this.
  *(clause 71)*
- **A landing moved over by hand before it was recorded** still counts:
  the branch seam answers "is this commit in the branch's history?", and
  a landing found there is recorded as landed. *(clause 24)*
- **An `after` that will never land.** The submission waits; `sluice
  status` shows "waiting, after #B". *(clauses 5, 7)*
- **A resubmission** carries the `after` and priority it is submitted
  with; nothing is inherited. *(clause 67)*
- **Which combination a culprit is told.** The combination named on the
  branch as it is now: once the submissions ahead landed as tested, the
  combination is the same one on the new branch tip. Rule core.17.
  *(clauses 10, 20)*
- **Every log line carries its schema**, `sluice/log@1` in `log.jsonl`
  and `sluice/result@1` in `results.jsonl`. *(clause 53)*

---

## Choices made (skim, overrule if wrong)

- **Dirty worktree check.** The driver reports what the worktree holds
  beyond its commits; sluice's `submit` refuses on that report. The
  statement says "the driver refuses"; same outcome, different code.
  *(clause 2)*
- **A branch with no `[branches]` entry.** `submit` refuses it, since
  there is no gate to test it against. *(clause 27)*
- **Submission numbers** count across the whole repository, so
  `sluice status 12` needs no branch name.
- **Which `[branches]` entry wins.** An exact name beats a pattern; then
  the pattern with more literal characters. `bolt/*` matches `bolt/x` but
  not `bolt/x/y`.
- **When tests are skipped.** Only after sluice itself landed the
  current branch tip does it ask the runner for "just what changed".
  After a hand move, on a new branch, or after a landing that changed
  `.config/sluice.toml`, every required test runs. *(clauses 15, 70)*
- **Batch window.** With `batch = true`, `parallel` is how many
  submissions the one big combination may hold; after it fails, all the
  smaller combinations are tested at once. *(clauses 13, 14)*
- **Timeout result.** A timed-out run is recorded "timed out", counts as
  a failure, and the CLI says "timed out". *(clause 66)*
- **Results recorded twice.** Each test result goes into the branch's
  `log.jsonl` (as the walkthrough shows) and into one repository-wide
  `results.jsonl`, which any branch reads to recognise a test that
  already passed. *(clauses 16, 69)*
- **Batch landing early.** With `batch = true`, if A+B passes while A
  alone is still running, A and B land together and A's run is
  stopped. *(clauses 14, 22)*
- **An extra state file.** `branch.json` holds the queue's own state
  (combinations in flight, a landing in progress, messages not yet
  delivered). The walkthrough's file list doesn't show it; without it,
  a crash would lose decisions.
- **"testing ... · N tasks to run" line.** The counts are only known
  when the run finishes, so the CLI prints that line with the result,
  not when testing starts.
- **Checkout names** are `sluice/try/<branch>/<n>`, not the
  walkthrough's `sluice/try/<n>`, so two branches on one machine never
  pick the same name. *(clause 32)*
