# Verify

The work on `change/a-delivery-read-needs-no-work-tree` at 086dd186,
which holds its base `bolt/sluice-tells-each-submitter-its-own-end` at
68d68a9a, checked on 2026-11-07 against this change's proposal, design,
its delta specs `outcome-delivery` and `inbox`, and its tasks.

**The work holds, and the change is archived.** The code meets every
requirement and scenario of both delta specs, and every test the tasks
name passes through sluice against the base. All 8 tasks are ticked.
The base's gate, `#check-fast`, passes its 11 tasks on 086dd186.
Archived in a scratch copy of `openspec/`, the specs pass `root:specs`
under the pinned openspec, and every open change still passes
`root:changes`, a-landed-bolt-is-never-reported-withdrawn among them.

## Checks

| Check | Result |
|---|---|
| `sluice exec bolt/sluice-tells-each-submitter-its-own-end` on 086dd186, the base's gate `#check-fast` | 11 of 11 tasks passed in 1m 41s, exit 0, results recorded under their keys for 086dd18: root:format, root:lint (`cargo clippy --workspace --all-targets -- -D warnings`), root:adrs, root:changes, site:rules, site:prose (0 errors, 0 warnings), seams:test, core:test, fakes:test, conformance:test and cli:test; 1068 tests passed, none failed or ignored |
| `sluice exec bolt/sluice-tells-each-submitter-its-own-end -- cargo test -p sluice --features tools --test worktrunk` | 64 passed, none failed, exit 0. The worktrunk suite is behind the `tools` feature, which `#check-fast`'s cli:test does not turn on (it is main's `#check-tools`), so the gate ran none of it; this one run covers every test tasks 2.2 to 3.3 name, and the tests of the modified requirements' scenarios |
| `openspec validate a-delivery-read-needs-no-work-tree --strict` | valid |
| `openspec archive a-delivery-read-needs-no-work-tree --yes`, then `openspec validate --specs --strict --no-interactive` (`root:specs`) and `openspec validate --changes --strict --no-interactive` (`root:changes`), in a scratch copy of `openspec/` at 086dd186, openspec 1.14.3 as `devenv.nix` pins it | archive applies (+2 added, ~2 modified); 67 of 67 specs pass; 10 of 10 open changes pass |
| `git merge-base HEAD origin/bolt/sluice-tells-each-submitter-its-own-end` | 68d68a9a, the base's tip: the branch holds its base |
| Leftovers, by `grep` over `crates/`, `site/`, `plugins/` and `docs/architecture/` | no `Session::bare`, `Recipient::`, `no_work_tree`, nor the words "stays pending for its session's sluice inbox" or "no files checked out, so no session" |

## Requirements and scenarios

### outcome-delivery

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| A wait where no files are checked out reads as a worktree on its branch (added) | Another worktree's landing read in the git directory | `recipient` in `crates/sluice/src/lib.rs` asks `Driver::recipient`, which reads the git directory as the worktree on `main` with no path; `pending_one` takes nothing of `unit-a`'s, and `delivering` returns `None` saying nothing | `a_wait_with_no_files_checked_out_reads_as_a_worktree_on_its_branch` (worktrunk): exit 0, `landed as`, no `pending`, `git status` or `checked out` said, nothing `delivered` and no `delivered` line logged; `a`'s inbox then collects it |
| | The git directory's own landing | the worktrunk driver's `recipient` reads `a`'s own git directory as the branch its `HEAD` names, no path, `SLUICE_AGENT` as the agent; `to` falls to `me.agent` (D3) | `a_git_directory_delivers_its_branchs_own` (worktrunk): the wait marks it `delivered`, `delivered_to` `agent-a` |
| | A HEAD whose branch has no commit | `recipient` reads the branch by `git symbolic-ref -q --short HEAD` and never `rev-parse --verify HEAD`; `Drivers` forwards `recipient` to worktrunk's, so `session` is not called | `a_head_with_no_commit_fails_no_delivery_read` (worktrunk): exit 0, `landed as`, nothing of `could not read`, `pending` or a commit said, nothing marked |
| A wait that reports its own submission's outcome marks it delivered (modified) | A waiting submit delivers its landing | unchanged in a worktree: `me.path` is always given there, so `to` is the session, else the path | `a_waiting_submit_delivers_its_landing_and_inbox_then_prints_nothing` (worktrunk); the agent where no files are checked out in `a_git_directory_delivers_its_branchs_own` |

### inbox

| Requirement | Scenario | Where it is met | Test |
|---|---|---|---|
| A folder with no files checked out collects as a worktree on its branch (added) | The git directory collects nothing of a worktree's | `inbox` takes `pending(&me)` for the worktree on `main`, which holds nothing of `unit-a`'s; nothing on stderr | `a_wait_with_no_files_checked_out_reads_as_a_worktree_on_its_branch` (worktrunk): `{"outcomes":[]}`, stderr empty, exit 0, and `a`'s inbox collects the outcome after |
| | A git directory collects its branch's own | as the wait's, with `delivered_to` the agent | `a_git_directory_delivers_its_branchs_own` (worktrunk): `a`'s next submission, failed, collected with `delivered_to` `agent-a`, and its record says the same |
| | A HEAD whose branch has no commit | `recipient` reads the branch with no commit; nothing pending for it | `a_head_with_no_commit_fails_no_delivery_read` (worktrunk): `{"outcomes":[]}`, stderr empty, exit 0 |
| | (where `HEAD` names no branch, nothing) | `recipient` gives `None`; `inbox` prints `{"outcomes":[]}` under `--json` and exits 0 | `a_folder_with_no_files_checked_out_is_read_with_no_status_and_no_commit` (a detached bare clone read as `None`); `inbox_in_a_detached_worktree_delivers_nothing` |
| Each outcome collected is marked delivered to the collecting session (modified) | A landing collected outside any multiplexer names the worktree; A session in a herdr pane is named by its pane | `to` is `me.session`, else `me.path`, else `me.agent` | `inbox_delivers_a_landing_once_and_marks_it_delivered_to_the_worktree`, `inbox_in_a_herdr_pane_delivers_to_that_pane_and_no_round_pushes_it` (worktrunk); conformance `delivered.rs` |

### The driver's read (task 2.2)

`a_folder_with_no_files_checked_out_is_read_with_no_status_and_no_commit`
(worktrunk) reads a bare clone and the checkout's git directory, with an
untracked file in the checkout, as `main` with no path, no head, the
agent and host `h`, and by `session` with nothing held beyond the
commits; a bare clone whose `HEAD` names `refs/heads/nothing` as
`nothing`; a detached bare clone as `None`; the checkout with its path.
`Session::bare` is gone from the seam and from every `Session` built:
`sluice-fakes`, `sluice-core`'s `named_session` and test driver,
`sluice-conformance`'s inbox driver, and the pull requests profile.

## Tasks

| Task | Condition | Result |
|---|---|---|
| 1.1 Record 0099, core.48 and its details, record 0085's amendment line, the rules page | `moon run root:adrs site:rules` | both passed in the gate |
| 1.2 a-landed-bolt-is-never-reported-withdrawn's delta drops the replaced requirement | `moon run root:changes` | passed in the gate; the delta keeps only "A repository with no files checked out is read with no git status" |
| 2.1 `Driver::recipient` default; `Session::bare` removed | clippy clean | root:lint passed in the gate |
| 2.2 the worktrunk driver's `recipient`; `Drivers` forwards it | its named test | passed |
| 3.1 `recipient`, `delivering`, `inbox` read `Driver::recipient` | its named test | passed |
| 3.2 a git directory delivers its branch's own | its named test | passed |
| 3.3 a `HEAD` with no commit fails no delivery read | its named test | passed |
| 4.1 the commands and state references | `moon run site:prose` clears every warning | 0 errors, 0 warnings in the gate |

## Notes

- The inbox's scenario "The git directory collects nothing of a
  worktree's" names a blamed submission; its test reads a landing. The
  inbox takes landed and failed outcomes through the one `pending`, by
  `Here::made`, so the reading is the same; a failure's own collection
  where no files are checked out is shown by `a_git_directory_delivers_its_branchs_own`.
- The worktrunk suite runs in main's `#check-tools`, so the bolt's
  landing onto main runs it in its gate as well.
- moon warned that it could not archive cli:test's outputs to its cache;
  the task passed, and its result was recorded.
