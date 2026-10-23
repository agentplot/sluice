# Tasks

Every test below runs in the gate: `sluice exec main` once committed,
or one suite while iterating, e.g. `sluice exec main -- cargo test -p
sluice --test cli group`.

## 1. The core takes a group in one write

- [x] 1.1 Add `GroupRequest` and `Sluice::submit_group` to `crates/sluice-core/src/ops.rs` (design decision 1): every worktree checked before anything is written, every refusal returned together with its worktree (a new refusal for a worktree named twice), every head published (82), ids allocated in order, one `update` under the branch's `wait_lock` withdrawing each worktree's earlier submission (67) and inserting all, `--after` and `--priority` on each, one pool wake; make `submit` call it with one worktree. Verify with unit tests in `ops.rs`: a dirty second worktree writes nothing and names it; three worktrees produce one records version with three submissions in order; no submission of the group names another in `after`.
- [x] 1.2 Add S59 to the conformance suite (`crates/sluice-conformance/src/shared.rs` and `tests/scenarios.rs`, `/// satisfies: S59, 125, 6, 12, 17, 18, 5, 50`): three heads submitted as one act on a `batch = true` branch form one combination of all three, tested once, landed in one move in order; then the middle one conflicting with the first is set aside alone, naming the first (65), and the other two land; and the middle one failing is blamed alone. Verify `cargo test -p sluice-conformance --test scenarios s59` passes in the gate.
- [ ] 1.3 Add a conformance test that a round reading the records between the group's allocation and its write sees none of the group, and after it all three (125). Verify it passes in the gate.

## 2. The command line

- [ ] 2.1 In `crates/sluice/src/lib.rs`, make `Submit`'s `--worktree` repeatable, default `.`; with more than one, refuse `--check` and `--status` with exit 64 (`--worktree` takes one with them), and route to a group submit: the refusal document naming each refused worktree (exit 2); the no-wait lines and `{"branch","submissions":[{"id","worktree","ahead"}]}`; the waiting form per design decision 3 (wait on each in order through `wait_on`, report each through `report`, exit 0 or the first non-landed one's `exit_for`, 130 on Ctrl-C with the document as it stands). Verify with tests in `crates/sluice/tests/cli.rs` for every scenario of the `group-submit` spec that does not need pull requests.
- [ ] 2.2 Update `site/src/content/docs/reference/commands.md` (the `sluice submit` row and a short section on submitting several worktrees as one act, its answer and exits) and `plugins/sluice/skills/using-sluice/SKILL.md` with `references/submit.md` (when to submit a set as one act, and that each is then waited on alone). Verify `site:prose` (Vale) and the plugin's checks pass in the gate.
- [ ] 2.3 Write `docs/adr/0047-a-group-submit-is-one-write-in-order-of-submission.md` (MADR, as 0046): the reading of "after the one before" as order of submission (design decision 2), the group mark and intake taking a group whole, and why a group's unreadable request holds only its own group (128). Verify the ADR's front matter matches its neighbours and `root:format` passes in the gate.

## 3. Under pull requests

- [ ] 3.1 In `crates/sluice/src/profiles/github/pulls.rs` and `lib.rs`, the group form of the pull-request submit (design decision 4): every worktree checked before any push; a group token; for each worktree in order the mark `<!-- sluice: group=<token> <k>/<n> head=<oid> -->` beside the agent mark, written to an open request's body before its branch is pushed, or to a new request's body when it is opened; a single submit writing the body with no group mark; the answer naming every request with what it waits for, and a waiting act following each request through `follow` to its outcome in order. Verify with tests in `crates/sluice/tests/profiles/pull_requests.rs` against the fake forge: three requests opened and marked 1, 2, 3 of 3; a dirty worktree pushes nothing.
- [ ] 3.2 Make intake (`take_in`, `consider`) take a bound group only whole, through `submit_group` in `k` order, in one write, once every request is present, open, out of draft, at its marked head and meeting its conditions; until then each bound request's state names the group's requests it waits for; the binding ends at a request's first submission at its marked head. Verify with tests in `pull_requests.rs` for every scenario of the `pull-request-intake` delta: two of three ready submits none; a round between the opens submits none until the third is open; one closed holds the rest; a new head on one after the group was taken withdraws only that one.
- [ ] 3.3 Update `site/src/content/docs/guides/pull-requests.md` with a group submit under pull requests: the mark, why the requests wait for one another, and how to free them. Verify `site:prose` passes in the gate.

## 4. Integration

- [ ] 4.1 Run the gate's checks with `sluice exec main` on the committed change and confirm every task passes, the S59 scenario among them.
