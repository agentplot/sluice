# Tasks

The groups land in order, as one landing, from this change's place on
`fix/a-hand-overs-withdrawal-holds-no-request`, cut from main. Run
suites through sluice (`sluice exec main -- cargo test ...`). A quick
`cargo check` or one unit test may run directly. Tests use scratch
repositories, a bare clone standing for GitHub's git, other hosts'
marks pushed to it, and the forge double only. They never use a real
account or the host's own sluice state. Commit by pathspec, with
Conventional Commits scoped to the part of the tree, citing clauses by
number and rules by id.

## 1. The withdrawal names the host it handed over to

- [x] 1.1 `Withdrawal::handed_over_to` and `Event::Withdrawn`'s, optional and skipped when absent; `Requester::HandOver { to, words }` in place of `Requester::Intake`, and `Sluice::withdraw` writing `to` into both (D1). True when `cargo test -p sluice-core --lib a_hand_overs_withdrawal_names_its_host_and_an_older_one_reads_as_it_was` passes.

## 2. Every reader reads it

- [x] 2.1 `handed_over` reads the field, and `named_no_host` reads the words only in a record that names no host. `left_unlanded` and `withdrawn_by_another` pass a hand-over by, and `hand_over` withdraws as `Requester::HandOver` (D1, D2). True when `cargo test -p sluice --lib a_hand_overs_withdrawal_holds_no_request an_older_hand_overs_withdrawal_reads_as_it_was` passes.
- [x] 2.2 The wait that follows a hand-over says `handed over to <host>`, and `sluice status` shows it in the row's detail (D1). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- a_wait_on_a_hand_over_says_where_it_went` passes.
- [x] 2.3 The mark comes back to the host that withdrew, and the request is taken in again with no draft, for a record that names its host and for one written before (D1, D2). The next submit after a hand-over marks nothing, and a request closed is still withdrawn and held (D3). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- the_mark_comes_back_and_its_request_is_taken_in_again an_older_hand_overs_request_is_taken_in_again a_submit_after_a_hand_over_marks_no_draft a_request_closed_after_it_came_back_is_still_held handed_over handed_over_by_a_wait` passes. `a_submit_after_a_hand_over_marks_no_draft` fails with `pulls.rs` as main held it.

## 3. The records, the rule and the pages

- [x] 3.1 Record 0088 cites agentplot/blueprints 84c7a01 and names the field. Rule core.33 cites 84c7a01, and its details say how a record from before is read (D2). The site's rules page is written with `node site/scripts/rules.mjs`. True when `sluice exec main -- moon run root:adrs root:changes site:rules` passes.
- [x] 3.2 The pull requests guide says a hand-over marks nothing a draft and the request comes back with the mark. The state reference names `withdrawn.handed_over_to`, and the plugin's pull requests reference says a hand-over holds nothing. True when `sluice exec main -- moon run site:prose` passes.

## 4. The gate

- [x] 4.1 The whole gate on the committed branch, main merged. True when `sluice exec main` passes.
