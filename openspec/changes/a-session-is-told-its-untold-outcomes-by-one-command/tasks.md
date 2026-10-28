# Tasks

This change builds on the push change
(`an-entry-records-its-submitting-session-and-the-round-pushes-its-outcome`).
Before group 1, read it as built: `Submitter.session`, `Submission.told_to`,
the outbox skipping a submission already told, and where its one-line
words function lives. Run suites through sluice (`sluice exec main --
cargo test -p <crate>`); the gate's checks are `sluice exec main` once
the work is committed.

## 1. The records

- [x] 1.1 In `crates/sluice-core/src/records.rs`, add `Submission::told_outcome() -> Option<Told>`, the outcome a round would tell for a landed, failed or conflicting submission, from its record (`landed_as`; failed checks, tested combination and its submissions, timed out from `result`, logs; conflict paths and submissions), and `Submission::notification()`, `<id>:landed|failed|conflict`, the id the round's outbox gives it (design, "The words"; "Marking told"); verify unit tests for each of the three states, a timed-out failure, and `None` for queued and withdrawn.
- [x] 1.2 In `crates/sluice-core/src/line.rs`, let a `Line` carry finished submissions changed in this write, which `changes` puts at `finished_file(id)` with no index line and no removal; verify a unit test that a finished submission marked told is one `put` of its finished file and that the index and live files are untouched.

## 2. The core tells a session its untold outcomes

- [x] 2.1 In `crates/sluice-core/src/ops.rs`, add the operation of design "Where it reads" and "Marking told": given the asking session's `Submitter`, find on every branch the untold landed, failed and conflicting submissions whose `from.worktree` or `from.head` is its branch, from the live records and the finished records of that branch's own landings alone; take each such branch's lock in sorted order and read again; return a value that holds the locks and the outcomes found, with a `mark(told_to)` that, per branch in one `update`, sets `told` and `told_to` (live or finished), removes any outbox item for it, and logs `told` with the item's id or `Submission::notification()`; a branch whose records cannot be read is returned among the unread with the reader's words, the others still read. Dropped without `mark`, it marks nothing. Verify unit tests in `crates/sluice-core` that: another worktree's submission is not found; a withdrawn, queued or told one is not found; a finished untold landing is found and marked in its finished record; an outbox item standing for a found submission is removed by `mark` and its id logged; dropped without `mark` the submission stays untold.
- [x] 2.2 Add conformance scenarios in `crates/sluice-conformance` for the spec: a landing told by the ask is marked told with the asker's session and a second ask finds nothing; an outcome the round delivered is not found by the ask; an outcome the ask marked is not notified by the next round (no notification counted for it); an ask on `main` and `bolt/x` finds both; a failure withdrawn by a resubmission is not found. Verify `cargo test -p sluice-conformance` and `sluice exec main -- cargo test -p sluice --features tools --test conformance`.

## 3. The records profiles rewrite a finished record

- [x] 3.1 Add tests in `crates/sluice/tests/local_files.rs` and `crates/sluice/tests/remote_refs.rs` that a write putting an existing `.finished/<id>.json` replaces it, reads back as written through `read_finished` and `read_all_finished`, and leaves `finished.jsonl` as it was; change the profile only if a test fails. Verify `sluice exec main -- cargo test -p sluice --features tools --test local_files --test remote_refs`.

## 4. The command

- [ ] 4.1 Move the push's one-line words function from the worktrunk profile to `crates/sluice/src/words.rs`, taking a submission's id, branch and `Told`, and have the worktrunk profile call it there (core.1); verify its unit tests move with it and pass, and `rg -n 'worktrunk' crates/sluice/src/lib.rs crates/sluice/src/words.rs` finds no new mention.
- [x] 4.2 Add `Command::Untold { worktree, json }` in `crates/sluice/src/lib.rs`, documented "Tell this worktree's session every outcome of its submissions not yet told to it, and mark each told", with examples `sluice untold` and `sluice untold --json` in its help: a worktree on no branch prints nothing and exits 0; otherwise the core's operation with the driver's `Submitter` for the worktree, `told_to` its `session`, else its `path`, else its branch; the output written and flushed before `mark`; text per outcome the words line then `report` as `sluice wait <id>` prints it, a blank line between, nothing at all when there are none; `--json` one document `{"outcomes":[...]}` of `record_json` with `told` and `told_to` as marked, `unread` added only when a branch could not be read; each unread branch said on stderr as `sluice: could not read <branch>'s records: <words>`, exit 3; exit 0 otherwise. Verify `cargo check -p sluice` and the tests of 4.3.
- [ ] 4.3 In `crates/sluice/tests/profiles/worktrunk.rs`, with herdr's variables cleared as the push change's `support::isolated` does, add tests that: a `--no-wait` submit that lands, then `sluice untold` in its worktree, prints a first line naming `#<id>`, `landed` and the revision and then `passed · landed as`, and `sluice status <id> --json` shows `told` and `told_to` the worktree's path; a second `sluice untold` prints nothing and exits 0; `--json` prints `{"outcomes":[...]}` with that record, and `{"outcomes":[]}` the second time; a blamed submit's ask names `failed`, its check and the log line `sluice wait` prints; under `HERDR_PANE_ID=w1:p2` and `HERDR_SOCKET_PATH=<scratch>` with the push change's fake `herdr` refusing the push, the ask's `told_to` names that pane and the round after it calls the fake no more; another worktree's submission is not told; a worktree on no branch prints nothing and exits 0. Verify `sluice exec main -- cargo test -p sluice --features tools --test worktrunk`.

## 5. Documentation

- [x] 5.1 Add a `sluice untold` section to `site/src/content/docs/reference/commands.md` beside `sluice wait`: what it tells, that it marks each told so it is told once whichever of the pane's push or the command comes first, its text and `--json` with an example, its exits, and that a session starting again runs it once in place of keeping a wait; say in `reference/state.md` that `told` and `told_to` are also set by `sluice untold`, `told_to` then the worktree's path where no multiplexer named the session; verify `sluice exec main -- moon run site:prose` passes with no Vale warning.
- [x] 5.2 In `plugins/sluice/skills/using-sluice/SKILL.md`, under Submit, say that a session starting again, after a restart or on another host, runs `sluice untold` once to be told the outcomes of its submissions it missed, in place of keeping a wait for each, and that it answers nothing for an outcome already told; verify `sluice exec main -- cargo test -p sluice --test plugin_docs` passes (every command is named in the skills), and `sluice exec main -- moon run plugin-sluice:validate`.

## 6. Integration

- [ ] 6.1 Run the gate's checks on the committed work with `sluice exec main`, and `openspec validate a-session-is-told-its-untold-outcomes-by-one-command`; verify both pass.
