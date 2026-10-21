# Design

## Context

See proposal.md, Why. What exists on `main` (12ae7f1), which this change
builds on:

- **The pull request driver** (`crates/sluice/src/profiles/github/pulls.rs`)
  posts one `sluice` check (`show`, `queue_shown`, `outcome_shown`) and
  one comment when a pull request is sent back (`comment_once`,
  `comment`).
  - `Posting { status, title, summary }` (`forge.rs`) has no text.
    `rest.rs` sends `output.title` and `output.summary` only.
  - The `Forge` trait can post a comment but cannot edit one.
  - Its `Outcome` drops `FailedTask.flaky`, `not_run` and `system`.
- **The records already hold most of a report.**
  - `ResultLine` (`records.rs`) carries kind (ran or replayed), task,
    hash, outcome, host, executor, combination, system and signer.
  - `FailedTask` carries log, flaky, system and not_run.
  - `set-aside.jsonl` holds set-asides.
  - The log holds `Testing`, `Ran`, `Replayed`, `Result`, `Blamed`,
    `Landed` and `Lost`, with times.
  - Finished submissions are read by `Sluice::latest_of` and `state`.
- **No duration outlives a run.**
  - `TaskResult` has none.
  - `Moon::progress` reads moon's checkpoints into `TaskProgress` while
    the run goes (ADR 0028), and that record is removed when the
    combination leaves testing.
  - `read_report` (`profiles/moon.rs`) reads `runReport.json` for hash,
    outcome, replay and exit only.
- **moon is run as** `moon exec <targets> --on-failure continue
  --concurrency N [--force]`, after `moon query tasks --affected
  --downstream deep` narrows the required targets. It runs without
  `--ci`.
- **The CLI's waiting `submit`** words flaky, could-not-run and
  set-aside (`crates/sluice/src/lib.rs`, `fn report` and the text near
  it).
- **Telemetry's scrub** (`telemetry.rs`, `Scrub`, ADR 0022) is private to
  the telemetry module.

## Goals / Non-Goals

**Goals:**

- One report form, `VerdictView`, read by the core from the records. A
  driver renders it; it names no runner and no forge (core.1, core.2).
- A pull request that reads like a team's CI report: a comment in the
  conversation, the same text in the `sluice` check, and a check per
  required check per system.
- moon run as moon's own CI runs it, as far as `runInCI` goes.

**Non-Goals:**

- Rendering for any driver but the pull request one. worktrunk's and
  the CLI's text stay as they are.
- Changing which targets the moon runner picks.
- Posting anything for a submission that is not a pull request.

## Decisions

### D1. The report is a view the core reads from the records, not a record

`Sluice::verdict_of(branch, id) -> Option<VerdictView>` sits beside
`status` in `ops.rs` and wraps a pure `verdict_view(...)` in a new
`crates/sluice-core/src/verdict_view.rs`. It reads, through the records
seam only (core.11):

- the submission, by `latest_of`, so a landed one is found among the
  finished;
- `State::results()`, for the lines of the branch whose combination is
  the submission's `Tested.combination`;
- `State::set_asides()`;
- the branch's log, for the `Testing`, `Result`, `Blamed` and `Landed`
  lines of that combination, and the run's detail.

Rows are built from the result lines, one per (task, system). A row is
filled in from `FailedTask` for flaky, could-not-run, missing and log
path, and from the set-asides for a set-aside. A required check with no
line and no `FailedTask` is missing.

`VerdictView` is defined in `sluice-seams`, beside `Notification` (53).
Its fields:

- `branch`;
- `standing`: queued with what is ahead, testing, passed and waiting
  ahead, landed as, blamed, or formed again;
- `combination`, `on`, and `submissions` in order;
- `run`: total, estimate saved, changed files, each optional;
- `rows`: task, system, key, result, ran or replayed, host, signer,
  executor, took, attempts, flaky, not_run, set-aside, log.

*Why a view and not a stored report?* Everything it shows is already a
record. A second copy would be state with two homes (core.14). It would
also need writing at every step that changes a submission.

*Alternative: render from the overview's `BranchView`.* That covers
only what is in the chain now. A submission's combination leaves
`branch.json` once it lands or is formed again, so the overview cannot
report a landed or blamed submission. The overview's `CandidateView`
does supply the live rows while a combination is tested (D5).

### D2. Not to blame means no rows

`verdict_view` returns `formed again` and no rows for a submission in
these three cases:

- it is queued;
- its latest `Tested` combination has a `Blamed` line for another
  submission;
- no `Testing` line names it since that blame.

The decision is made in the core, so no driver can show a failure clause
20 keeps from it (pull-requests-1 proposed clause 118).

### D3. A runner's detail rides on the result it belongs to

| Where | New fields |
|---|---|
| `TaskResult` (`sluice-seams`) | `took: Option<Duration>`, `attempts: Option<u32>` |
| `RunReport` (`sluice-seams`) | `detail: Option<RunDetail { took, saved, changed: Vec<String> }>` |
| `ResultLine` | `took_ms: Option<u64>`, `attempts: Option<u32>`, each `#[serde(default, skip_serializing_if = ...)]` |
| `Event::Result` | `detail: Option<RunDetail>` |

- `step.rs` `poll` writes all of these where it already writes the
  result lines and the `Result` line, before the verdict (core.7).
- Old lines read without them, so `RESULT_SCHEMA` stays `result@1`.
- `verdict::read` takes no new input. A test runs it on the same
  results with and without detail and asserts the same verdict (16, 19,
  38).
- On a branch whose records are refs on its remote, the signed
  statement's per-result fields gain the same two optional fields. A
  verifier ignores them, as they decide nothing. Otherwise a signed
  result would lose them.

*Alternative: keep the last `TaskProgress` snapshot.* It is the
checkpoint's time, to the second, not moon's own. It is also host-local
and removed by design (ADR 0028).

### D4. moon in CI mode, the targets unchanged

- `moon.rs` adds `--ci` to `moon exec`.
- The first task checks by hand, on moon 2.6, that `moon exec --ci <t>`
  skips a `<t>` marked `runInCI: false`, as `moon ci <t>` does. If it
  does not, the profile runs `moon ci <targets>` instead. sluice already
  narrows the targets, and `--force` keeps its meaning of no replay.
- **The warning.** `moon query tasks --json` gives each task's options.
  The runner's `plan` already calls it, so it reads `runInCI` there. The
  required targets marked `false` go back to the core as a new
  `RunReport.kept_out: Vec<String>`. The core logs them once per
  combination, and `status` and a waiting `submit` name them.
- **`read_report`** reads, per `RunTask` action:
  - `duration`, as `{secs, nanos}`;
  - the count of operations with `meta.type == "task-execution"`, as
    attempts.

  Per run, it reads `duration`, `comparisonEstimate.duration` (the
  estimate) and `context.changedFiles`. `saved` is the estimate minus
  the duration, never below zero.
- **The fixture** is a `runReport.json` from a real moon 2.6 run in
  sluice's own repository, trimmed. It replaces the hand-written one
  where they overlap.

### D5. The pull request shows the report three ways

```
              VerdictView (core)          CandidateView.tasks (overview)
                     |                              |
                     v                              v
              github/render.rs  <--------------------+
                |        |          |
   comment (edited)  `sluice` check text   `sluice / <check> (<system>)`
```

- **Comment.**
  - Marker `<!-- sluice-report: <branch> -->`. The first is created at
    the first result that touches the submission; after that sluice
    edits it.
  - `Forge::comments` returns `(id, body)`, and `Forge::edit_comment`
    PATCHes `/issues/comments/{id}`. `App::patch_api` exists.
  - The blamed comment, marked `told=<id>` as today, links to the
    report comment's URL.
- **`sluice` check.** `Posting` gains `text: Option<String>`.
  - `posting_json` sends `output.text` and `posted` reads it back.
  - The comparison that stops a repeated post includes it.
- **Checks per system.**
  - One check run per required check per system:
    `sluice / <task> (<system>)`, or `sluice / <task>` when the policy
    names no systems.
  - Read back by name and app before posting, as `sluice` is.
  - Each has its row's status, signer or host in the title, and the row
    and its log tail in the summary.
  - A row with no result yet is queued; one under test is in progress.
- **While testing**, the rows come from the overview's
  `CandidateView.tasks` for the submission's combination: each task
  `waiting`, `running`, `passed`, `failed` or `replayed`, with its times.
  - A post happens only when a task's state changes, so a long run
    costs one edit per task that ends.
  - Rounds run every few seconds, so time alone never triggers a post.
- **Rendering is deterministic.** It has no relative times, and the row
  order is: failed, flaky, could not run, missing, then the rest by
  name.
- **Layout**, kept close to the CI reports teams already read:
  - a head line;
  - the combination line;
  - `Total time | Comparison time | Estimated savings` when recorded;
  - a table `| | Check | Time | Result | Info |`, with one colored
    square per result (green passed, purple replayed, red failed, yellow
    flaky, black could not run or missing);
  - 20 rows, then a collapsed "Every check";
  - collapsed "Executor", "Changed files" and, by policy, "Output of
    <check>";
  - one section per system when the policy names more than one.
- **Size.** GitHub caps a comment and a check's text at 65,536
  characters. The renderer cuts the collapsed sections first, then the
  rows past 20, and says how many it left out.

### D6. Output posted only as asked, scrubbed

- `pull_requests.log_lines: Option<usize>` is added to the policy's
  `pull_requests` table and read from the branch's revision (56,
  core.15).
- With it, the last that many lines of the kept output (`FailedTask.log`)
  are posted, scrubbed. Without it, the kept output's path is posted, as
  today.
- `Scrub` and `end` move from `telemetry.rs` to `crates/sluice/src/scrub.rs`.
  Telemetry and `render.rs` both use them.
- On a remote branch, the output may be in `refs/sluice/logs/...`. The
  renderer reads what the records seam hands it and never fetches.

### D7. The wording is shared

The text for flaky, could not run, set aside and nothing run moves out
of `lib.rs` into `crates/sluice/src/words.rs`, one function per case.
The waiting `submit` and the renderer both call it, so the CLI and the
pull request say the same thing.

### D8. `forge check` asks for two more permissions

`github/check.rs` adds `("checks", "Checks")` and
`("pull_requests", "Pull requests")` to the write permissions it reads
from the installation.

## Risks / Trade-offs

- **[Risk]** `moon exec --ci` may not filter by `runInCI` for explicit
  targets. → Task 1.1 checks it first. D4's fallback is `moon ci
  <targets>`.
- **[Risk]** `--ci` changes moon's output (log groups) and may set
  `CI=true` for tasks. A test that behaves differently under `CI` would
  start failing in sluice. → Say so in the moon guide section. It is
  what the team's own CI already does.
- **[Risk]** GitHub's secondary rate limits on edits. → One edit per
  ended task per pull request. Live posts pause when a round's posts
  exceed 30, and the final report is always posted.
- **[Risk]** A posted output tail leaks a secret that scrubbing misses.
  → Off by default, and the guide repeats ADR 0022's warning.
- **[Trade-off]** Checks per system multiply posts on a large matrix.
  45 checks on 3 systems is 135 check runs per head. GitHub allows 1000
  per name per suite. Accepted, because many-hosts-3 asks for them.
- **[Trade-off]** The log read for the run's detail and times needs the
  whole log once `finished-submissions` 2.1 splits it. → Use 2.2's
  whole-log reader if it has landed. Otherwise read the current part,
  where a recent combination's lines are.

## Migration Plan

- Lines and events written before this change read without the new
  fields. Their reports show no times.
- A pull request open when this lands gets its report comment at its
  next result.
- `log_lines` lands with its parser before sluice's own
  `.config/sluice.toml` sets it. `~/.cargo/bin/sluice` is reinstalled
  first, because the installed binary parses the policy.
- The GitHub App gains Checks and Pull requests write, which ADR 0021
  already lists. `forge check` now fails until they are granted.

## Open Questions

- Whether nx and turbo report attempts. Their profiles leave the field
  empty if not, and the report leaves the note out.
