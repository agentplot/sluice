# Tasks

While iterating, run the pull request tests through sluice:
`sluice exec main -- cargo test -p sluice --features tools --test pull_requests <name>`.
The gate's `cli:tools` and `cli:test` tasks are what verify each
group.

## 1. GitHub can mark a pull request a draft or ready

- [x] 1.1 Add `node_id` to `Pull` (`profiles/github/forge.rs`), read from REST's `node_id` in `rest.rs` and defaulting to empty in the file fake's serde. Add `Forge::set_draft(repo, number, draft)`. Verify with `cargo check -p sluice` and the existing `rest.rs` unit tests under `cli:test`.
- [x] 1.2 Implement `set_draft` on the app as one GraphQL mutation (`convertPullRequestToDraft` or `markPullRequestReadyForReview`, keyed by `pullRequestId`) posted to `{api}/graphql` with the installation token. An `errors` array is an error even on HTTP 200. Verify with a unit test in `rest.rs` that builds both request bodies and reads an error reply as an error, under `cli:test`.
- [x] 1.3 Make `FileForge` implement `set_draft` with its existing `set_draft`, which records the ready time on the timeline. Make the test fake in `tests/profiles/forge/github.rs` answer `POST /graphql` for both mutations from it, and give it a switch that refuses them. Verify with a fake.rs unit test that a convert then a ready leaves one new ready time, under `cli:test`.

## 2. Intake holds a submit's pull request until its submitter acts

- [x] 2.1 In `pulls.rs`, tell a submit's pull request by its agent marker (`marked_agent`). Widen `removal` to the `Withdrawn` line. In `consider`, hold a submit's pull request whose latest submission left without landing and that has no `ready_for_review` at or after that second. While it is held, submit nothing for it, mark it a draft if it is not one, and post `waiting: the next submit`. A draft call that fails holds it just the same, and the next pass tries again. Hand-opened pull requests keep today's path. Verify with the e2e tests of 2.3.
- [x] 2.2 Add the `waiting: the next submit` posting to `waiting()`, with its summary naming the next `sluice submit` from the branch and marking it ready for review. In `comment()`, which `notify` and `consider` call, give a submit's pull request the "sluice marked it a draft; the next submit from its branch, or marking it ready for review, submits it again" text. Hand-opened ones keep today's text. Verify with unit tests in `pulls.rs` beside `what_a_waiting_pull_request_shows`, under `cli:test`.
- [x] 2.3 Add an e2e test in `tests/profiles/pull_requests.rs` for each intake and status scenario of this change: a new head no submit pushed (the crew #127/#128 case: queued, head moved, withdrawn, a draft, never resubmitted over several steps); withdrawn by `sluice withdraw --as operator`; marked ready by a person; held when the fake refuses the draft; and the `waiting: the next submit` check with the drafted comment. Keep `a_new_commit_while_queued`, `blamed_then_retried` and `blamed_and_left_alone` passing as the hand-opened cases. Verify with `cli:tools`.

## 3. A submit is the act that brings it back

- [x] 3.1 In `Pulls::open`, take the branch's intake lock, blocking. Under it: push and open or update as today; withdraw the place's submission if it is queued and not landing, as its agent; then, unless the place has no submission and the pull request is out of draft, mark it ready for review, marking it a draft first if it is not one. Read the timeline back, and if the newest ready mark is before the leaving's second, wait for the next second and mark it again, up to three tries, then fail naming both times. Verify with the e2e tests of 3.3.
- [x] 3.2 In `lib.rs`, remove `Reached::Before`, the `sent_back` lookup in `submit_pull` and the `same_commits` suffix from `report`. In `words.rs`, drop the suffix parameter of `timed_out_way_on` and the advice lines that take it. A same-head submit is followed like any other. Verify with `words.rs` unit tests that the time-out and blamed advice name no draft, under `cli:test`.
- [ ] 3.3 Replace `submit_again_on_a_head_sent_back` with e2e tests for the submit's scenarios: blamed, then `wt submit` of the same head runs its checks again; timed out (give `OnGitHub` a way to set a short `timeout` with the `slow` file), then `wt submit` of the same head is taken and never prints "nothing was queued or run again"; a submit of a new head on a queued pull request withdraws the old submission and is taken without a draft; and `sluice submit` from a second worktree on the same branch, standing for ops, marks it ready and names that agent. Verify with `cli:tools`.
- [ ] 3.4 Add `s56_a_refused_request_waits_for_its_submitter` to `tests/profiles/pull_requests.rs`, citing S56 and 120. A pull request `wt submit` opened is removed for a conflict. A commit standing for another submission's fix is pushed to its branch, so its head moves. Over several steps it is a draft and nothing is submitted or landed in its agent's name. The agent's next `wt submit` marks it ready at the new head, and it lands. Then a hand-opened pull request whose head moves the same way is submitted again once its review holds. Verify with `cli:tools`.

## 4. Docs and the decision

- [ ] 4.1 Write the next-numbered ADR in `docs/adr/`: a pull request a submit opened is a draft until its submitter acts. Cover the marker as ownership, the hold read from the log and the timeline, the submit under intake's lock, and GraphQL for draft state. It narrows ADR 0021's sent-back sentence to pull requests opened by hand. Cite clauses 120, 54, 67, C.5 and S56. Verify with `root:adrs`.
- [ ] 4.2 Update `site/src/content/docs/guides/pull-requests.md` ("When things change" and the check table: `waiting: the next submit`) and `site/src/content/docs/reference/commands.md` (replace the same-head paragraph and its console example with a same-head submit that is taken). Verify with `site:prose`, every warning cleared.
- [ ] 4.3 Update the plugins' references: `plugins/sluice/skills/using-sluice/references/verdicts.md` (the sent-back paragraph), `references/submit.md` and `plugins/sluice/skills/setting-up-sluice/references/profiles/pull-requests.md` (a submit's pull request sent back is a draft until the next submit). Verify with `plugin-sluice:validate` and the `plugin_docs` test under `cli:test`.

## 5. Integration

- [ ] 5.1 With the work committed, run `sluice exec main` and see every check-fast and tools task pass.
