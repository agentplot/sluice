# Proposal

## Why

A pull request into a branch sluice lands shows almost nothing of its
verdict. PR #55's `sluice` check reads only "landed as 0cef58d", and
nothing appears in its conversation. A team used to moon's CI expects
a report there: each task, its time, whether its cache answered it, and
what failed. sluice should have a report of its own, the same whatever
task runner a repository uses, built from its records, so that a moon
repository today, and an nx or turbo one later, reads the same page. It
should also show what no runner's report can: what was tested together,
who is blamed, and where each check ran.

## What Changes

- **A verdict report, built from the records.** The core reads a
  submission's verdict from its records into one form: the combination
  and the submissions tested in it, and for each required check its key,
  result, executor, whether it ran or was replayed, a flaky reading
  (19), a check that could not run and why (114, 115), and a set-aside
  (116). It names no task runner and no forge (core.1, core.2) and
  decides nothing (27, 30).
- **A task runner's detail, recorded with each result.** The check seam
  hands back, beside each result, how long the task took and how often
  it was tried. Per run, it hands back the time the run took, the time
  the runner estimates it saved, and the changed files. sluice records
  this with the result before the verdict (core.7). No verdict reads it
  (pull-requests-1, proposed clause 119).
- **moon runs as in CI.** The moon profile runs its checks with moon's
  CI mode on, so a task marked `runInCI: false` is not run. sluice warns
  when a required check is such a task. The targets sluice picks stay
  as they are: the branch's required checks, narrowed to what is
  affected.
- **The pull request shows the report.** One comment in the pull
  request's conversation carries the report. It is created at the first
  result that touches the submission and edited in place after. The
  `sluice` check carries the same report as its text. While a
  combination is tested, both show its tasks as they end. The comment
  sent when a pull request is blamed links to the report.
- **A pull request tested with a blamed one shows no outcome of that
  combination.** It goes back to queued, and its report says its
  combination is formed again (20; pull-requests-1, proposed clause
  118).
- **A check per required check per system.** Beside `sluice`, each
  required check on each system the branch names gets its own check on
  the pull request's head, `sluice / <check> (<system>)`, or
  `sluice / <check>` when the policy names no systems. It carries the
  check's result, its signer or host, and its row of the report. This
  is the pull request row's addition in many-hosts-3.
- **Failure output, only as asked.** The end of a failed check's
  output is posted only when the branch's policy sets
  `pull_requests.log_lines`, scrubbed as telemetry scrubs it (ADR 0022).
- **`sluice forge check` verifies two more permissions**: Checks and
  Pull requests, read and write.

## Capabilities

### New Capabilities

- `verdict-report`: what a submission's report holds, read from the
  records, the same for every task runner; how a task runner's detail
  is recorded with a result; what a submission tested with a blamed one
  is shown.
- `moon-runner`: how the moon profile runs a combination's checks (CI
  mode, `runInCI`, the targets) and what it reads back from moon's run
  report.

### Modified Capabilities

- `pull-request-status`: the report in the pull request's conversation
  and in the `sluice` check's text, edited in place; the blamed
  comment links to it; failure output by policy.
- `forge-identity`: `forge check` verifies the Checks and Pull requests
  permissions.

## Out of scope

- **Head runs from GitHub Actions** (`sluice-pr.yml`, S40) stay with
  `actions-pool`. Once they are recorded, they show in this change's
  checks per system as any result does.
- **Task runners other than moon.** nx and turbo come with their own
  profiles. Each will fill the same detail the moon profile fills.
- **Posting moon's own run report.** It describes one run of moon, not
  the combination that lands. Its changed files would include other
  pull requests' changes, and its "cached" is not a replay the gate
  trusts. A repository may still run moon's own reporting in its own CI
  workflow, outside sluice.
- **Statement.** Clauses 118 and 119 and scenarios S50 and S51 go into
  `design/sluice/proposals/pull-requests-1.md`, for the operator's
  ruling. This change cites them as proposed.

## Impact

- **Seams:** `crates/sluice-seams` gains `TaskResult.took`/`attempts`,
  `RunReport.detail` and `VerdictView`.
- **Core:** `crates/sluice-core` gains `ResultLine` and `Event::Result`
  optional fields, `verdict_view` and `Sluice::verdict_of`, and the
  `pull_requests.log_lines` policy key. It depends on
  `finished-submissions` task 2.2's whole-log reader if that lands
  first.
- **Profiles:**
  - `profiles/moon.rs`: `--ci`, reading duration, attempts and
    estimate, and the `runInCI` warning.
  - `profiles/checks.rs`: the command runner's `took`.
  - `profiles/github/`: `Posting.text`, comment edit, `render.rs`,
    `pulls.rs` and `check.rs`.
  - `telemetry.rs`'s scrub, lifted to a module of its own.
- **Docs:**
  - the pull-requests and GitHub App guides;
  - a "sluice and moon's CI" section;
  - the `using-sluice` skill's verdicts reference;
  - ADR 0039.
- **No new language, file type or plugin directory.**
