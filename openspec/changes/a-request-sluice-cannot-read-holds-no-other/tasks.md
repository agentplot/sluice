# Tasks

While iterating, run the pull request tests through sluice:
`sluice exec main -- cargo test -p sluice --features tools --test pull_requests <name>`.
The gate's `cli:tools` and `cli:test` tasks are what verify each
group.

## 1. One pull request's failure is its own

- [x] 1.1 In `take_in` (`crates/sluice/src/profiles/github/pulls.rs`), run each pull request's `consider`, `verdict_of` and `show_report` as one fallible step. On an error, print `sluice: warning: taking in <branch>'s pull requests: pull/<n>: <error:#>` with `crate::telemetry::warn` and go on to the next pull request; return `Ok(true)` after the loop. Leave every step before the loop as it is. Add `with_context` naming what was read to the `forge.readied`, `forge.reviews` and `forge.checks` calls in `consider` (`reading pull/<n>'s timeline`, `reviews`, `checks`). Verify with the e2e tests of 2.2 to 2.5.
- [x] 1.2 In `consider`, catch a `fetch_head` failure. If the pull request is a draft or `missing` is not empty, hand back `waiting(&missing, None)` to post together with the failure to report; otherwise hand back nothing to post and the failure. Change `consider`'s return so the loop gets both (what to post, if anything, and a failure, if any), posts what it is given through `show_report`, and prints the failure as in 1.1. Verify with a unit test beside `what_a_waiting_pull_request_shows` that `waiting(&["a draft".into()], None)` reads `waiting: a draft` with no `Forecast:` in its summary and no line, under `cli:test`, and the e2e tests of 2.3.

## 2. The fake GitHub and the scenarios

- [x] 2.1 In `crates/sluice/tests/profiles/forge/github.rs`, give `GitHub` a switch that makes `GET /repos/o/r/pulls/<n>/reviews` answer an error status for the pull requests it names, and one that makes `GET /repos/o/r/pulls` answer an error, both held in shared state the answer closure reads. Add to `OnGitHub` in `tests/profiles/pull_requests.rs` a step that returns the round's output without asserting it has no warning, and a helper that makes GitHub serve no head for a pull request: delete its branch and `refs/pull/<n>/head` in the bare repository. Verify with `cli:tools`, through the tests below.
- [ ] 2.2 Add `s63_a_request_sluice_cannot_fetch_holds_no_other`, citing S63 and 128: two hand-opened pull requests, out of draft, `reviews = 0`, the one GitHub serves no head for listed first by the fake. One step: its output names that pull request and `has no head`, the other is submitted and its `sluice` check reads queued or testing, the first has no submission. Two more steps: the first is named each time. Push its branch again: the next step submits it, with no warning. Then a third pull request with no head, closed: the next step names nothing. Verify with `cli:tools`.
- [ ] 2.3 Add `a_draft_whose_head_cannot_be_fetched_is_only_a_draft`, citing 128: a draft with no head on GitHub and a ready pull request. One step: its output names the draft, the ready one is submitted, and the draft's `sluice` check reads `waiting: a draft` with no `Forecast:` in its summary. Verify with `cli:tools`.
- [ ] 2.4 Add `reviews_that_cannot_be_read_hold_no_other`, citing 128: `reviews = 1`, two pull requests each with an approving review on its head, the fake failing the first's reviews. One step: its output names the first and its reviews, the first is not submitted, the second is. Turn the failure off: the next step submits the first. Verify with `cli:tools`.
- [ ] 2.5 Add `a_failed_list_ends_intake_and_the_queue_still_lands`: a pull request submitted and queued, then the fake failing the list of pull requests. One step: its output warns that taking in `main`'s pull requests failed, and the queued submission is tested and lands. Verify with `cli:tools`.
- [ ] 2.6 Keep every existing test in `tests/profiles/pull_requests.rs` passing; its `step` still asserts no warning, so a pull request that reads cleanly prints none. Verify with `cli:tools`.

## 3. Docs

- [ ] 3.1 In `site/src/content/docs/guides/pull-requests.md`, under "When things change", add a bullet: when sluice cannot fetch a pull request's head, or read its reviews or checks, each round prints a warning naming it and what failed and goes on with the others; it is read again each round until it can be, or it is closed; a draft shows `waiting: a draft` either way. Verify with `site:prose`, every warning cleared.

## 4. Integration

- [ ] 4.1 With the work committed, run `sluice exec main` and see every check-fast and tools task pass.
