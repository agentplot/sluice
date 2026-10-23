# Tasks

Read first: this change's proposal, specs and design; clauses 121, 22,
24, 45, 104 to 106, C.4, C.5's branch row and S8, S57 in
`agentplot/blueprints:design/sluice/requirements.md`; ADR 0046;
`docs/architecture/core.md`. Run suites through
`sluice exec main -- <command>`; before submitting, `sluice exec main`.

## 1. Decisions

- [x] 1.1 Write the ADR (the next free number after 0046) for design D1 to D6: one revision the branch must contain, kept in the branch's document; the hold as a field and a log line; runs left alone; `sluice branch accept`; `sluice branch create` setting the revision. Name clause 121 in core.6's Details ("Rules out: … a candidate formed on a revision that lacks the branch's last landing (121)") in `docs/architecture/core.md` and `site/src/content/docs/architecture/rules.md`; verify `site:prose` and `openspec validate a-branch-read-past-its-landings-is-refused --strict` (by hand: no gate task runs openspec).

## 2. The records

- [x] 2.1 `BranchDoc` gains `landed: Option<Rev>` and `held: Option<Held>` (`read`, `lacks`, `since`), both skipped when absent; `Event` gains `Held { read, lacks }` and `Accepted { revision, lacked }`; verify unit tests in `records.rs` that a document without the fields reads as before and one with them round-trips.
- [ ] 2.2 The landing confirmation in `perform_landing` sets `landed` to the landing's `to` in the write that logs `landed`; verify the conformance landing tests and a case that `landed` equals the tip after two landings.
- [ ] 2.3 A document with no `landed` takes it on the round's first read and writes it: the log's last `landed` line's `to` when `base` is absent or contains it, otherwise `base`; verify conformance cases with records written without the field: a read lacking the last landing holds; a branch removed and made again, `base` on the new branch, is not held and takes `base` (spec: records written before the hold existed).

## 3. The hold

- [ ] 3.1 The local-refs profile's `contains` answers `false` when the revision asked of is not a commit on this host, instead of failing; verify a case in `tests/local_refs.rs` with an unknown object id.
- [ ] 3.2 `decide` checks the read against `landed` before touching `base`: held, it records `held` (one log line per revision read), writes, and returns `Step::Held` without polling, forming, starting or landing; otherwise it clears `held` and goes on as today; `step_locked` skips `perform`'s new work but still tells the outbox and releases decided places; verify conformance cases over the fakes, using `set_branch` to put the branch back: behind the landing holds; beside it holds; a hand commit on top forms (S8 as today); three held rounds write one `held` line; a run under test on the landing finishes and lands once the landing is back, with no new run.
- [ ] 3.3 S57 as a conformance scenario in `crates/sluice-conformance/tests/scenarios.rs`, `/// satisfies: S57, 121, 22, 45`: two landings, the branch then read at an older revision, nothing formed or landed and both revisions recorded; then each way out, the landing back and the operator's acceptance, forms the queue and lands it; verify `cargo test -p sluice-conformance`.

## 4. Accepting a revision

- [ ] 4.1 `Sluice::accept_branch(branch, revision)` in `branches.rs` under the branch's lock: absent → exit 1; not held, or the read or the hold's `read` not the revision given → refusal naming what the branch reads; otherwise set `landed`, clear `held`, log `accepted`; verify conformance cases for each outcome, and that the next round logs `moved` and forms on the accepted revision.
- [ ] 4.2 `make_branch` writes `landed` as the made revision and clears `held` once the move succeeds, nothing on a refusal; `remove_branch` clears `held` once its move succeeds; verify conformance cases for a bolt removed and made again from a revision lacking its old landing, which is not held, and for a held branch removed, whose records then hold no hold.
- [ ] 4.3 `sluice branch accept <branch> <revision> [--json]` in `lib.rs` and `branch.rs`, exits 0, 1, 2 as create and delete, help with an example; verify `tests/cli.rs` cases for each exit and the JSON document, and `tests/usage.rs` for the help.

## 5. Saying it

- [ ] 5.1 `sluice step` text and `--json` (`"step": "held"`, `held`), and `sluice run` printing the line once per hold; verify `tests/cli.rs` over the fakes.
- [ ] 5.2 `sluice status <branch>` prints the hold above the queue and carries `held` under `--json`; the overview's branch view and text the same; verify `tests/cli.rs` cases with `status_lists_the_queue` and the overview's tests unchanged for a branch not held.
- [ ] 5.3 A waiting `sluice submit` and `sluice wait` print the hold's line once in text; their `--json` record is unchanged; verify `tests/cli.rs` cases for both, with a timeout.

## 6. Over real git

- [ ] 6.1 S57 end to end in `tests/remote_branch.rs`: two submissions land on `bolt/x` in local refs, the second adding `remote = "origin"`, origin holding an older `bolt/x`; a third submission is queued; `sluice step` holds and names both revisions, origin and the local branch untouched; then (a) pushing the second landing to origin, and (b) in a second test, `sluice branch accept`, each lands the third submission; verify `sluice exec main -- cargo test -p sluice --test remote_branch`.
- [ ] 6.2 `a_push_from_elsewhere_refuses_the_landing_and_the_next_round_lands` and the other `remote_branch.rs` cases still pass unchanged, a push on top of the landing being formed on as before; verify the same command.

## 7. Docs

- [ ] 7.1 `reference/commands.md`: `sluice branch accept` in the table and the `sluice branch` section, `held` in step's outputs and exits; `reference/state.md`: `landed` and `held` in `branch.json`, the `held` and `accepted` log lines; `README.md`'s `sluice branch` row; verify `site:prose`.
- [ ] 7.2 `guides/github.md`: where it says to push `main` by hand once after adding `remote`, say what sluice does if the remote's copy is older than what landed locally, and the two ways out; verify `site:prose`.

## 8. Gate

- [ ] 8.1 Verify `sluice exec main` passes with everything committed.
