# Tasks

Read first: this change's proposal, specs and design; clauses 120 and
125 and C.5's pull request row of `design/sluice/requirements.md` in
agentplot/blueprints; ADRs 0048 and 0052; `docs/architecture/core.md`,
rules core.2, core.4, core.6 and core.14. Run suites through `sluice
exec main -- <command>`; before submitting, `sluice exec main`.

## 1. A group's new requests open as drafts

- [x] 1.1 Give `Forge::open` (`crates/sluice/src/profiles/github/forge.rs`) a `draft: bool`, sent as `"draft"` in `rest.rs`'s create call and kept by the fake (`fake.rs`) and the test server (`crates/sluice/tests/profiles/forge/github.rs`); every caller passes `false` but `open_group` for more than one worktree. Verify `rest.rs`'s and `fake.rs`'s unit tests (`sluice exec main -- cargo test -p sluice --lib profiles::github`) and `sluice exec main -- cargo test -p sluice --features tools --test forge`.

## 2. The act takes all or is refused

- [x] 2.1 In `crates/sluice/src/lib.rs`, give `PullRequests::open_group` a refusal that holds, beside a worktree's `SubmitRefusal`, a request short of its conditions: its worktree, number and what it lacks; and let `Opened` carry the submission the act queued for its request. No type of `sluice-core` changes (rule core.2). Verify `cargo check -p sluice --features tools`.
- [x] 2.2 In `open_group` (`pulls.rs`), for more than one worktree, after every branch is pushed and every request opened or updated: read each request's conditions on its head as `missing` does of a request out of draft. All met: mark each ready (`mark_ready`, as now), register each in `known`, and, still under the intake lock, queue the group with one `Sluice::submit_group` over the places `pull/<n>` in the order given, each accepted submission recorded in `entries`. Any short: mark none ready, mark each ready request a draft with `set_draft`, and answer the refusal for each short one. Verify one test per scenario of the group-submit delta's first requirement in `crates/sluice/tests/profiles/pull_requests.rs`: "Three pull requests" (three ready, queued in order, each `from.worktree` `pull/<n>`, in one combination); one short of a review (exit 2, the short one named with `needs 1 review`, all three drafts, none queued); refused, reviewed, submitted again; a request sluice held as a draft taken with a new act; a fork's request of the same branch name left alone; a waiting act landing all three. Run `sluice exec main -- cargo test -p sluice --features tools --test pull_requests group`.
- [ ] 2.3 In `submit_pull_group`, print each queued submission as `submission <id> to <branch> from #<n> · <ahead>` after its `pushed … · opened #<n>` line, and under `--json` add `id`, `ahead` and `pull_request` to each item; on the refusal print `refused: #<n> from <worktree> needs …` for each short request, then that every request of the act is a draft at its head and the same act run again takes them once they meet their conditions, exit 2, and under `--json` the refused error document with each short worktree in `worktrees`. Verify in `pull_requests.rs` the words and the `--json` documents of the "Three pull requests" and one-short tests, with the test run of 2.2.
- [ ] 2.4 Move `a_group_submit_opens_every_pull_request_marked`, `two_of_a_group_ready_submits_none` and `one_of_a_group_closed_holds_the_rest` to the specs: the first to the refusal under `reviews = 1` (all drafts, each named, marks written at each head), the other two reaching intake's group binding by a refused act and a person's ready marks (`set_draft(false)` on the fake), as the pull-request-intake delta's scenarios say. Verify the test run of 2.2.

## 3. One of a taken group on a new head

- [ ] 3.1 Rewrite the end of `one_of_a_group_on_a_new_head_is_withdrawn_alone` to the group-submit delta's second requirement: after the push to the second's branch, its submission is withdrawn and its request a draft, the first and third not withdrawn; rounds land the first and third, tested without the second; the second still has one submission and is a draft; then `sluice submit main --no-wait` from the second's worktree leaves it ready, with no group mark, and the next round submits it alone. Fix the driver only if the test shows it not holding (the design reads the hold as already in place). Verify `sluice exec main -- cargo test -p sluice --features tools --test pull_requests one_of_a_group_on_a_new_head_is_withdrawn_alone`.

## 4. Records and docs

- [ ] 4.1 Write `docs/adr/0054-a-group-submit-to-pull-requests-takes-its-requests-itself-or-leaves-them-drafts.md` in the form of ADR 0052, the design's decisions and their alternatives, citing 120, 125, C.5 and ADRs 0048 and 0052; take the next free number if the bolt has 0054 by then. Add to ADR 0048's consequences: narrowed by the new ADR under pull requests, the act marking each request ready and taking the group itself, since the forge now marks a request ready and a draft (ADR 0052), intake's group binding standing for requests a person marks ready. Verify `sluice exec main -- moon run root:adrs root:format`.
- [ ] 4.2 `site/src/content/docs/guides/pull-requests.md`, "Several pull requests as one act": the act takes every pull request at once, ready at its head, or is refused naming each one short and leaves all of them drafts; run it again once reviewed; a new head on one of a taken group makes it a draft until its worktree submits again, the others landing. A paragraph in `site/src/content/docs/reference/commands.md`'s "Submitting several worktrees as one act" saying the same in brief, linking the guide. Verify `sluice exec main -- moon run site:prose`.

## 5. Gate

- [ ] 5.1 Verify `openspec validate a-group-submit-meets-pull-requests --strict` (by hand: `root:specs` reads only `openspec/specs`) and `sluice exec main` with everything committed.
