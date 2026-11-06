# Tasks

The groups land in order, as one landing, from this change's place on
`fix/a-hosts-intake-never-withdraws-anothers-submission`, cut from
main. Run suites through sluice (`sluice exec main -- cargo test ...`);
a quick `cargo check` or one unit test may run directly. Tests use
scratch repositories, a bare clone standing for GitHub's git, other
hosts' scratch clones and the forge double only, never a real account
or the host's own sluice state. Commit by pathspec, Conventional
Commits scoped to the part of the tree, citing clauses by number and
rules by id.

## 1. The decision recorded

- [x] 1.1 Write records 0088 (D1, D3, D4) and 0089 (D2, proposed with the amendment of 137), and rule core.33 and its details citing 122, 137 and both records; write the site's rules page with `node site/scripts/rules.mjs`. True when `sluice exec main -- moon run root:adrs root:changes site:rules` passes.
- [x] 1.2 The amendment of 137 for a host's waits, on agentplot/blueprints's branch `design/sluice-a-hosts-waits-hold-its-intake-mark-together` cut from main, committed and unpushed, for the user's yes. True when the branch holds one commit changing 137 alone.

## 2. A refused push is decided again

- [x] 2.1 `Pulls::takes` asks `decide` at most twice, reading the branch again between; `put_mark` answers `None` on a refusal and reads nothing; `Taking::Undecided` takes nothing in and hands nothing over, warning once (D1). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- a_mark_let_go_under_a_renewal_is_pushed_again a_mark_this_host_moved_under_a_renewal_stays_its_own` passes, and fails with `pulls.rs` as main held it.

## 3. A host's waits hold its mark together

- [x] 3.1 `Kept::hold` and `Kept::let_go` in `crates/sluice/src/profiles/github/kept.rs`, the holds under `forge/intake/<owner>/<repo>/<base>.waits/` made and read under their `.lock` (D2). True when `cargo test -p sluice --lib the_hosts_waits_hold_a_mark_together` passes.
- [x] 3.2 A wait's intake keeps its hold from its first pass on a branch whose records each host keeps; `let_go_marks` removes the mark, as the repository last read or wrote it, only where no other wait of the host holds it and it names this host and a wait (D2). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- a_hosts_waits_hold_its_mark_together a_wait_shows_itself_and_lets_go a_wait_stopped_by_ctrl_c_lets_go two_takers_at_once_one_holds` passes, the first failing with `pulls.rs` as main held it.

## 4. The hand-over passes by work the branch holds

- [x] 4.1 `hand_over` takes the pass's revision of the branch and passes by a submission whose head it holds (D3). True when `sluice exec main -- cargo test -p sluice --features tools --test pull_requests -- a_request_that_landed_elsewhere_is_not_handed_over handed_over handed_over_by_a_wait a_landing_is_not_handed_over` passes, the first failing with `pulls.rs` as main held it.

## 5. The gate

- [ ] 5.1 The whole gate on the committed branch. True when `sluice exec main` passes.
