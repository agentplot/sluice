# Verify

The work on `change/sluice-shows-the-writes-its-protection-refused` at
7025e107, cut from main at 3f5aed5f, checked on 2026-11-05 against this
change's proposal, design, its two delta specs, `refused-writes` and
`branch-protection`, and its tasks, with main still at 3f5aed5f.

**The work holds.** Every requirement and scenario of both specs is met
by the code and by the test the scenario names, and each of those tests
passed. Every task is ticked. Each task's condition holds, and the checks
the tasks name pass. Task 6.2's notes for the orchestrator were written
nowhere, so this report carries them, under "After the landing" below.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 7025e107, against main at 3f5aed5f | 14 of 14 tasks passed, each run and none restored from the cache: core:test 130 tests, fakes:test 7, conformance:test 268, cli:test 568 and cli:tools 524, none failed; every test the tables below name passed among them |
| `openspec validate sluice-shows-the-writes-its-protection-refused --strict` | valid |
| `git -C ~/Code/github_agentplot/blueprints/main log -1 --format=%s 539f745` (1.2) | names clause 148, with 124, C.4 and S73 |
| `git log --format=%s main..HEAD` | five `feat` commits, `feat(cli)` among them (3.5): 1b76dd5d, 08a1bce7, 49787928, 789fbabd, ece7890f |

The gate's run was not recorded under its keys. This report was being
written in the worktree while the run went, and sluice records a run
only from a worktree that holds exactly its head (rule core.18). The
run's results stand as a check of 7025e107: no file the gate reads
changed, since this report is the only file beyond the head.

moon warned 30 times across the run. 26 warnings say it could not store
a task's outputs or a blob under its caches. Three say it was asked to
hash a directory as an input, and one says it skipped an install for
want of a network. Every task still passed. None of these warnings comes
from this change.

`grep -rn refused_writes crates --include='*.rs'`, leaving out the
tests and `refused_writes.rs` itself, finds the field read
only by `line.rs`, `Sluice::carry`'s end, `Sluice::status`,
`Sluice::branch_view_of` and the CLI's writers of status and the
overview. No round, gate, landing or submission reads it (rule core.14).

## Requirements and scenarios

### refused-writes (new)

| Requirement | Scenario | Where it is met | Test, and the task it passes in |
|---|---|---|---|
| A refused write's cause is its command on its host | A timer's update-ref with new revisions each run | `command_of` in `crates/sluice-core/src/refused_writes.rs`; `RefusedWrite::key`, `(host, ref, command)`; `Line::refuse_write` matches on it | `the_same_command_with_other_revisions_is_one_cause`, conformance:test |
| | An abbreviated revision is kept as written | `command_of` rewrites only a whole word of 40 or 64 hexadecimal digits | `anything_but_a_whole_full_revision_is_left_as_written` and `a_full_revision_is_written_as_revision`, core:test |
| | Two hosts | the cause's key holds `host`, `Sluice::peer.name` from `record_protected` | `two_hosts_are_two_causes`, on a branch whose policy names `origin`, conformance:test |
| A cause's first refusal is logged, and the rest counted | Three refusals of one command | `Line::refuse_write`: a standing cause gets `times += 1`, `last`, `from` and `to`, and no line | `one_line_for_a_cause_refused_three_times`, conformance:test |
| The count is kept in the branch's state | The log moved aside | `BranchDoc::refused_writes`; status and the overview read the document alone | `a_refused_writes_count_is_in_the_state_not_the_log`, cli:tools; `the_count_is_read_with_the_logs_lines_taken_away` and S73, conformance:test |
| | Records written before | `serde(default)` on `refused_writes`, and on `Event::Protected`'s `host` | `records_written_before_refused_writes_still_read`, core:test |
| A cause stands until a day passes with none of its writes refused | Refused again within the day | `stands`: `now < last + SHOWN`, a day | `a_cause_refused_within_its_day_still_stands`, conformance:test; `a_cause_stands_until_a_day_after_its_last`, core:test |
| | A fixed caller | `standing` for status and the overview; `Sluice::carry` calls `Line::end_refused_writes` only where the line's own changes are not empty, logging `protected-ended` at `last` and a day | `a_fixed_caller_ends_a_day_after_its_last`, conformance:test, which also asserts that a round that writes nothing logs no end; `neither_status_nor_the_overview_shows_a_cause_a_day_after_its_last`, cli:test |
| | Refused after its end | `refuse_write` ends past causes first, then logs the command anew | `a_cause_refused_after_its_end_opens_anew`, conformance:test |
| At most twenty causes stand per host and branch | Twenty-one commits on main | `STANDING`, 20; `refuse_write` puts out the host's cause whose `last` is oldest, logged at `now` | `a_twenty_first_cause_ends_the_oldest`, conformance:test |
| Status shows each standing cause above the queue | The timer's cause in status | `Status::refused_writes`, newest first; `write_refused_writes` in `crates/sluice/src/lib.rs`, after the copy behind and 133's refusal; `command_line` cuts after 100 characters | `status_shows_the_writes_its_protection_refused`, cli:test |
| | None standing | `write_refused_writes` writes nothing; `status_json` carries `[]` | `status_says_nothing_of_no_refused_writes`, cli:test |
| Status says what to do and carries the causes in `--json` | A branch kept in this host's repository | `Status::on_remote`, from the branch seam's `copy`, which the routing answers by the policy's remote; read only while a cause stands | `status_tells_a_local_branchs_refused_writes_what_to_do`, cli:test |
| | A deletion | `, deleting it` where `to` is `None`; `to` serialized as `null` | `status_names_a_refused_deletion`, cli:test; `a_refused_write_and_its_end_are_the_documents_and_the_logs`, core:test |
| The overview shows them under the branch's line | Five causes standing | `write_refused_writes` in `crates/sluice/src/overview.rs`, after the held places, at most three, then `and <n> more: sluice status <branch>` | `the_overview_shows_three_and_how_many_more`, cli:test |
| The overview's document carries them | Five causes in the document | `BranchView::refused_writes`, kept by `Sluice::branch_view_of` to those standing, newest first | `the_overviews_document_carries_every_cause`, cli:test |
| | A follower | the reader builds its views through `Sluice::overview` at every tick, so its document drops a cause at the first tick past its day | `a_followers_overview_carries_refused_writes`, cli:tools |
| Nothing is decided from a refused write | A round beside a standing cause | nothing outside `line.rs`, `ops.rs`'s status, `overview.rs` and `carry`'s end reads `refused_writes`; `in_play` is unchanged | `a_round_beside_a_standing_cause_is_unchanged`, conformance:test |
| A tool that watches the host reads them from sluice | The pages say where | `branches[].refused_writes[]` in the overview's fields on `site/src/content/docs/reference/commands.md`; the trap in `plugins/sluice/skills/setting-up-sluice/references/profiles/remote-branches.md`; each says never `log.jsonl` or `branch.json` | site:prose and plugin-sluice:validate |

S73,
`s73_a_timers_refused_writes_are_one_command_counted_and_shown_until_a_day_after_the_last`,
passes in conformance:test. It gives one `protected` line for 25
refusals five minutes apart, `times` 25 in status and the overview, the
same count with the log's lines moved aside, neither showing it a day
after, and the next write logging `protected-ended` with `times` 25.

### branch-protection (modified)

| Requirement | Scenario | Where it is met | Test, and the task it passes in |
|---|---|---|---|
| A refusal is recorded with the command and both revisions | The log line of a refused move | `Line::refuse_write` logs `protected` with `ref`, `from`, `to`, `by` as git ran it, and `host`, whatever the command | `every_way_git_moves_a_protected_branch_is_refused`, cli:tools: a line and a cause for each of nine commands, `git branch -f bolt/x unit/a` among them. `a_timers_writes_are_one_cause`, cli:tools, asserts `host` on the line the hook writes for a refusal by git |
| | The same command refused again | as above, through the cause | `a_timers_writes_are_one_cause`, cli:tools |

The scenario "The log line of a refused move" asks for `host` on a
`git branch -f` line. No test reads the host on that command's line.
`protected` lines are written by one path, `Line::refuse_write`,
whatever the command. Two tests read `host` on a line from that path:
`a_timers_writes_are_one_cause` against git, and
`the_same_command_with_other_revisions_is_one_cause` over the fakes,
which compares the whole event.

## Tasks

21 of 21 are ticked, and each one's condition holds:

- 1.1, 1.3: record 0082 is accepted on the user's words of 2026-11-05,
  citing agentplot/blueprints main at 539f745. Rule core.47 stands under
  "Branches and landings" with its details, which cite 539f745. The
  site's rules page carries core.47. `root:adrs`, `root:changes` and
  `site:rules` pass in the gate.
- 1.2: `git -C ~/Code/github_agentplot/blueprints/main log -1
  --format=%s 539f745` reads `design(sluice): a write the protection
  refused is logged once per cause and host, … (124, 148, C.4, S73; …)`.
- 2.1 to 2.6: `refused_writes.rs` with `command_of`, `SHOWN`,
  `STANDING`, `stands`, `ends`, `standing` and `RefusedWrite::key`, and
  its four unit tests. `RefusedWrite`, `BranchDoc::refused_writes`,
  `Event::Protected`'s `host` and `Event::ProtectedEnded` are in
  `records.rs`, with two unit tests. `Line::end_refused_writes` and
  `Line::refuse_write` are in `line.rs`. `Sluice::carry` ends causes
  only where the line's own changes are not empty. `record_protected`
  calls `refuse_write` with `self.peer.name`. The nine conformance tests
  and S73 pass in conformance:test.
- 3.1 to 3.5: `Status::refused_writes` and `Status::on_remote`;
  `BranchView::refused_writes` with `serde(default)`, none on
  `BranchView::unread`. `write_status`, `status_json` and
  `write_overview` write them. The seven CLI tests pass in cli:test, and
  `a_followers_overview_carries_refused_writes` passes in cli:tools.
  `789fbabd` and `ece7890f` are `feat(cli)`.
- 4.1 to 4.3: `protect.rs`'s module and `hook` doc comments, and no
  other change to the hook. `a_timers_writes_are_one_cause` and
  `a_refused_writes_count_is_in_the_state_not_the_log` are new.
  `every_way_git_moves_a_protected_branch_is_refused` asserts a line and
  a cause of `times` 1 for each command. `packing_a_protected_branch_is_allowed`
  asserts a deletion tried twice is one cause of `times` 2. All of them
  pass in cli:tools.
- 5.1 to 5.3: the status row and `branches[].refused_writes[]` on the
  commands page; the paragraph and D5's console block in
  `guides/worktrees.md`; `refused_writes` under `branch.json`, and
  `protected` and `protected-ended` after `read-again`, on the state
  page; the trap in `remote-branches.md`. No host, tool or team of ours
  is named (rule core.26): the pages' `build-1` and `/home/me/app` are
  placeholders. `site:prose` and `plugin-sluice:validate` pass in the
  gate.
- 6.1: as the checks above.
- 6.2: the code stage's last commit says the orchestrator's work is "in
  the stage's notes". No file of the change holds it, and flume's record
  of the code stage's end carries no report or words. It is written
  below. No task waits on the landing.

## After the landing

The orchestrator's, once the submission lands on main, since the code,
verify and submission stages all come before it:

1. Confirm the release CI makes from the landing's commit is a new minor
   version: `gh release view <tag> --repo agentplot/sluice` names that
   commit. The change's `feat` commits (1b76dd5d, 08a1bce7, 49787928,
   789fbabd, ece7890f) make it a minor.
2. From that release's archive for the host, `sluice status <branch>
   --json` and `sluice overview --json` carry `refused_writes`, `[]` on a
   branch with none. Task 6.2 names `sluice status --help` for this.
   Neither the help nor any requirement of this change names the field
   there, as the help names no other field of status, so the check is
   made on the two documents.
3. Give the release's tag to swancloud for its doctor's change. On
   swancloud's branch `fix/the-views-timer-moves-anchors-only`,
   `packages/swancloud-doctor/sluice-refused.sh` reads `log.jsonl`. It
   is to read `branches[].refused_writes` from `sluice overview --json`
   in each repository, judging its own window on `last`.
4. Tell flume's owner that flume's `refused_acts` lists a refusal once
   for each time it happens. sluice now counts one per cause (148).
