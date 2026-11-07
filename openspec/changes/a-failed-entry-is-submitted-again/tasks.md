# Tasks

Each group is one session's work for an implementer who reads this
change (proposal, design, specs) and its own group. The groups land in
order, as one landing. Work in this change's place on
`change/a-failed-entry-is-submitted-again`, cut from
`bolt/sluice-takes-mended-work-again`. Run suites through sluice
(`sluice exec main -- cargo test ...`); a quick `cargo check`, `cargo
clippy` or one unit test may run directly. No test reaches a credential
or the host's own sluice state: tests use temporary repositories, the
GitHub double and `support::isolated`, as the pull request suite does
now. Commit by pathspec (`git add <paths>; git commit -m "<subject>" --
<paths>`), Conventional Commits, scoped to the part of the tree, citing
clauses by number and rules by id. The code's commits are `fix`, so the
release CI makes from the landing is a patch. Every group ends with its
check task passing.

## 1. The decision recorded

- [x] 1.1 Write record 0093 with the options weighed (how a host that does not take the requests in reads a request's state, what `--status` answers there, what `wt submit` reads, what a submit marks); amend rule core.33 and its details to say a wait elsewhere follows its request by what sluice's check says; write the site's rules page with `node site/scripts/rules.mjs`. Done in the commit that writes this change. True when `sluice exec main -- moon run root:adrs root:changes site:rules` passes.

## 2. The GitHub double keeps a completed run completed

- [x] 2.1 In `crates/sluice/src/profiles/github/fake.rs`, make `post_named`, given an existing run whose posting is `Failure` or `Success` and a posting that is `Queued` or `InProgress`, keep the run's status and take the posting's title, summary and text; any other posting replaces the run whole, as now (D5). Its doc comment cites switchboard-kit #268 and record 0093. Add `a_completed_check_run_stays_completed` beside the fake's tests: a run posted `Failure` and patched `Queued` reads `Failure` with the queued posting's title. True when `cargo test -p sluice --lib fake` passes.
- [x] 2.2 Run the pull request and worktrunk suites and mend each test that read a reopened check's status: it reads the title instead, as any host now reads it (D3), and says so in its comment. True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests --test worktrunk` passes, and the only failures before the mend were tests that read a reopened run's status.
- [x] 2.3 Check the group: commit, then run `sluice exec main -- moon run cli:test`. True when it passes.

## 3. A request's state across hosts is what sluice's check says

- [x] 3.1 In `crates/sluice/src/lib.rs`, give `Followed::Landed` and `Followed::SentBack` `head: String` and `draft: bool`, the pull request's as `follow_across` read it (D2); update every match on them. True when `cargo clippy -p sluice --all-targets -- -D warnings` is clean.
- [x] 3.2 In `crates/sluice/src/profiles/github/pulls.rs`, add the function that reads an open request's state from the title of sluice's check on its head and its draft mark, D3's table, with the host the intake mark names: `failed: ` sent back; `conflict: ` sent back as a conflict; `waiting: the next submit` (the title `waiting` makes for `NEXT_SUBMIT`) sent back as a draft and, out of draft, open as `waiting to be taken in by <host>`; anything else, or no check, open with its words or `waiting`. Have `follow_across` read an open request through it, never through the posting's status; merged and closed stay as they are. Its doc comment cites 120, 137, C.5 and record 0093. Add `a_request_across_hosts_reads_as_its_check_says`, asserting the function over the postings `waiting`, `queued`, `queue_shown` and the outcome's posting make for each state, each with GitHub's sticky `Failure` and with its own status, draft and not. True when `cargo test -p sluice --lib pulls` passes.
- [x] 3.3 In `crates/sluice/tests/profiles/pull_requests.rs`, add `held_then_marked_ready_waits_to_be_taken_in` (request-intake-host, "Held, then marked ready": B blames and drafts `#n`; a `sluice submit main --no-wait` on A marks it ready; `sluice wait main --timeout 5s --json` on A, before B's next pass, exits 5 with `state` `waiting to be taken in by b`) and `taken_in_again_while_github_keeps_the_check_failed` (request-intake-host, "Taken in again while GitHub keeps the check failed": after that, B takes `#n` in again and lands it while the double keeps its check failed; A's wait reports `queued`, then `testing`, and exits 0 at the landing). Keep `blamed_on_another_host` passing as it stands. True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- held_then_marked_ready taken_in_again_while blamed_on_another_host landed_on_another_host` passes.
- [x] 3.4 Check the group: commit, then run `sluice exec main -- moon run cli:test cli:tools`. True when both pass.

## 4. `--status` across hosts, and `wt submit` reads it

- [x] 4.1 In `crates/sluice/src/lib.rs`, in `submit_pull` under `PullMode::Status`, answer `Followed::Landed` and `Followed::SentBack` with `state` `landed`, `failed` or `conflict`, `title`, `summary`, `pull_request`, `branch`, `head`, `draft` and `sends` (`with_sends`, and `with_copy` as now) under `--json`, the check's title and summary in words otherwise, and exit 0 (D2); a wait and a waiting submit keep `across` and its exits. Comments cite 124, 137 and record 0093. True when `cargo clippy -p sluice --all-targets -- -D warnings` is clean.
- [x] 4.2 In `crates/sluice/src/profiles/worktrunk/mod.rs`, make `Standing::PullRequest` `PullRequest { number, landed }`; have `standing` read a pull request document's `state` `closed`, `failed` or `conflict` as `Standing::Nothing`, `landed` at `sends` as `PullRequest { landed: true }`, and any other as now; have `landed` take a landed pull request as it takes a landed submission (D1). Extend `what_stands_at_what_a_submit_sends` with `failed` and `conflict` documents at `sends`, draft and not (nothing stands), a `landed` one at `sends` and at another revision, and a `failed` one with no `sends` (no answer). True when `cargo test -p sluice --lib worktrunk` passes.
- [x] 4.3 In `crates/sluice/src/profiles/worktrunk/wt_submit.rs`, say `following pull request #<n>, landed at this revision` for a landed pull request, and keep `already open` for the rest; update the module's doc comment to say a pull request sent back stands nowhere on any host (record 0093). True when `cargo clippy -p sluice --all-targets -- -D warnings` is clean.
- [x] 4.4 In `crates/sluice/tests/profiles/pull_requests.rs`, add `status_of_a_request_another_host_sent_back` (pull-request-submit, "Sent back on another host": `wt submit main --status --json` on A after B blamed and drafted `#n` exits 0 with `state` `failed`, the check's `title` and `summary`, `head`, `draft` `true` and `sends` equal to `head`) and `run_again_marks_ready_a_draft_another_host_sent_back` (worktrunk-submit, "Sent back as a draft on another host", then "Run again before the other host takes it in again": the across twin of `run_again_marks_a_draft_ready`, asserting the `--no-wait --json` document, `#n` out of draft, a second `--timeout 2s --json` run that pushes and marks nothing and exits 5, and B's next pass taking `#n` in again at the same head). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- status_of_a_request_another_host run_again_marks` passes.
- [x] 4.5 Check the group: commit, then run `sluice exec main -- moon run cli:test cli:tools`. True when both pass.

## 5. A submit takes again a request another host sent back

- [x] 5.1 In `crates/sluice/src/profiles/github/pulls.rs`'s `opening`, where `left` is none and the existing request is open and not a draft, read sluice's check on its head before the push; where 3.2's function reads it sent back or held, mark the request ready as `mark_ready` does with no `left`, a draft first, and say `marked ready for review` (D4). Read nothing for a new request, a draft, or one whose `left` this host's records give. Comments cite 120, C.5, rule core.32 and record 0093. True when `cargo clippy -p sluice --all-targets -- -D warnings` is clean.
- [ ] 5.2 In `crates/sluice/tests/profiles/pull_requests.rs`, add `a_submit_marks_ready_a_request_another_host_sent_back_not_a_draft` (pull-request-submit, "Sent back, not a draft": a request opened by hand from `unit/review`, blamed by B and left out of draft; `sluice submit main --no-wait` on A from a worktree on `unit/review` marks it a draft and ready, and B's next pass takes it in again) and `a_submit_leaves_a_request_queued_on_another_host` ("Still queued": no draft mark reaches the double, and B's submission stays queued). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- a_submit_marks_ready a_submit_leaves` passes.
- [ ] 5.3 Check the group: commit, then run `sluice exec main -- moon run cli:test cli:tools`. True when both pass.

## 6. Say it to the operator

- [ ] 6.1 In `site/src/content/docs/reference/commands.md`, in the paragraph on a wait whose pull request another host takes in, say the wait reads sluice's check's words, not the colour GitHub keeps for it; that a pull request held for the next submit is sent back while a draft and, once marked ready, waits for that host to take it in again; and that `sluice submit --status --json` there prints a landed or sent-back request with `head`, `draft` and `sends` and exits `0`. In `site/src/content/docs/guides/worktrunk.md`, say a pull request blamed or in conflict goes into the queue again on `wt submit` whichever host takes the branch's pull requests in. In `site/src/content/docs/guides/pull-requests.md`, where a submit marks a draft sluice made ready, say a submit from any host does, and marks ready a request sent back that is no draft. Name no host, team or tool of ours (rule core.26). True when `sluice exec main -- moon run site:prose` passes with no warning on the pages.
- [ ] 6.2 Check the group: commit, then run `sluice exec main -- moon run site:prose`. True when it passes.

## 7. The gate

- [ ] 7.1 Run the gate's checks on the committed work with `sluice exec main`, and `openspec validate a-failed-entry-is-submitted-again --strict`. True when every task passes and the change is valid.
