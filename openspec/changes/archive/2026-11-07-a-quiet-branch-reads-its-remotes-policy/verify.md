# Verify

The work on `change/a-quiet-branch-reads-its-remotes-policy` at
9a9b7529, cut from `bolt/sluice-takes-mended-work-again` at main's
0b15d250, checked on 2026-11-07 against this change's proposal, design,
its three delta specs (`branch-refusal`, `remote-branch`,
`branch-order`) and its tasks.

**The work holds, and the change is archived with this report.** The
code meets every requirement and scenario of the three delta specs, all
13 tasks are ticked and each one's condition holds, the gate's checks
pass on 9a9b7529, and archived, all 65 specs pass `root:specs`' command
under the pinned openspec 1.14.3.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 9a9b7529 | 13 of 13 tasks passed in 10m 11s, recorded under their keys for 9a9b752: cli:test, cli:tools, conformance:test, core:test, fakes:test, root:adrs, root:changes, root:format, root:lint, root:screenshots-scene, site:prose and site:rules among them. Every test the tasks name ran there and passed: the four branch-refusal cases on the fakes (conformance:test) and on git (cli:tools), `quiet::a_quiet_branch_this_host_fetched_is_read_whole_by_status` and `quiet::a_quiet_branch_this_host_never_fetched_takes_a_submit` (cli:tools), and both `overview_orders_branches_*` cases (cli:test) |
| `openspec validate a-quiet-branch-reads-its-remotes-policy --strict --no-interactive`, under `sluice exec main` with the pinned openspec 1.14.3 | valid; one non-blocking warning, the proposal's Why over 1000 characters |
| `openspec archive a-quiet-branch-reads-its-remotes-policy --yes`, then `openspec validate --specs --strict --no-interactive` (`root:specs`), in a scratch copy of `openspec/` at 9a9b7529, with the pinned openspec 1.14.3's binary from the store path `sluice exec main` names (6.1) | the archive applies `branch-order` ~1, `branch-refusal` +2, `remote-branch` +1 ~1; 65 of 65 specs pass. Under `sluice exec` itself the archive could not write its lock in a scratch directory outside the worktree, which the executor's sandbox refuses, so it was run with the same binary outside it |
| `crates/sluice-core/src/ops.rs` and `overview.rs` as 0b15d250 holds them, then `cargo test -p sluice --features tools --test remote_branch -- a_quiet_branch_this_host` under `sluice exec main` (2.4) | both cases fail: `a_quiet_branch_this_host_fetched_is_read_whole_by_status` at `remote_branch.rs:212`, status refusing `main` at the records' `R1`, and `a_quiet_branch_this_host_never_fetched_takes_a_submit` at `remote_branch.rs:1048`, status after the submit refusing `bolt/x` at `R1`; the task asks the first to fail; restored, the tree clean |

The work on 9f15ec2a had been checked the same way for 2.1 to 5.3, and
no file outside `openspec/changes/a-quiet-branch-reads-its-remotes-policy`
has changed since: 2.2's four cases fail with the reads as 0b15d250
holds them (`shared.rs` 1823, 1856, 1878, 1906), as does 3.1's new case
(`cli.rs:2344`); clippy is clean for 2.1 and 5.1 to 5.3; and none of the
cases or helpers 5.1 to 5.3 take out is left.

## Requirements and scenarios

### branch-refusal (added)

| Requirement / scenario | Met by | Test, passed in the gate |
|---|---|---|
| A read outside a round reads the policy at the revision this host holds | `Sluice::status` reads it at `Branch::held`, then `base`, then `Branch::read`; `Sluice::branch_view_of`, through `view_read`, at `held` then `base`, whether or not the records' ask was refused. `held` is the host's own ref, or for a branch on a remote its tracking ref, read with no fetch (`pushed_refs.rs`, `held`) | |
| A quiet branch mended outside sluice | status reads the queue, `at` the records' `R1`, `parallel` from `R2`; the overview's view has no `unread`; no `moved`, the records unchanged | `a_quiet_branch_mended_outside_sluice_is_read_whole`, fakes and git |
| Status of a submission on the mended branch | `lookup` finds it landed; its branch's status has no warning and no refusal | `status_of_a_submission_on_a_mended_branch_answers_for_it`, fakes and git |
| A read outside a round fetches nothing, writes nothing and names its revision | neither read writes; `policy_at`'s refusal names the revision read | |
| The refusal names the revision this host holds | status and the overview refuse naming `R2` and `[[checkout.cache]]`, never `R1` | `a_refusal_names_the_revision_this_host_holds`, fakes and git |
| The state is left as it was | `at` is `R1`; the records, their version and the log unchanged after a status, an overview and a view | `a_read_of_a_mended_branch_leaves_its_state_as_it_was`, fakes and git |

### remote-branch (modified, added)

| Requirement / scenario | Met by | Test, passed in the gate |
|---|---|---|
| A policy refused on the branch's remote refuses the branch until a push there clears it (modified; 484 characters archived, under openspec's 500) | the round's read fetched, unchanged; a pushed policy clears the refusal at the next command that reads the branch, by the added requirement below | |
| The bolt's own round runs past a copy that lags; A refusal on the remote names the remote's revision; A push to the remote clears the refusal | unchanged by this change | the suite's existing cases in `remote_branch.rs` |
| A submit reads the branch fetched, and a read outside a round as this host last fetched it (added; 494 characters) | a submit's `Branch::read` fetches (124), unchanged; status and the view read at `held` | |
| A quiet branch this host fetched is read whole by status | status and the overview read `main` whole, `at` `R1`, `revision` `R2`; `origin/main`'s reflog, the records and `base` unchanged, no `moved` | `quiet::a_quiet_branch_this_host_fetched_is_read_whole_by_status` (cli:tools) |
| A quiet branch this host never fetched takes a submit | status refuses naming `R1` and `[[checkout.cache]]`, not `R2`; the submit fetches `R2` and queues; status after it reads `landed, queued`; `base` and `moved` unchanged | `quiet::a_quiet_branch_this_host_never_fetched_takes_a_submit` (cli:tools) |

### branch-order (modified)

| Requirement / scenario | Met by | Test, passed in the gate |
|---|---|---|
| The order is read from the default branch | `overview()` takes the order from the default branch's `view_read`'s `policy_at`, then its local revision where it has no records | |
| No records on the default branch | unchanged | `overview_orders_branches_as_the_policy_names_them` (cli:test) |
| No policy | unchanged | the suite's existing case |
| The order follows the revision this host holds | the order read at `main`'s held revision, not its records' `base` | `overview_orders_branches_by_the_revision_this_host_holds` (cli:test) |

## Tasks

All 13 are ticked, and each one's condition holds on 9a9b7529: 1.1
(`root:adrs` in the gate; record 0046's amendment and record 0069's
More information say what the task asks), 1.2 (`site:rules` in the
gate; rule core.15's details as the task words them, naming no intake),
2.1 (clippy clean), 2.2 to 2.4 and 3.1 (the cases pass in the gate, and
fail with the reads as 0b15d250 holds them, above), 4.1 and 4.2
(`site:prose` in the gate; the commands reference's `sluice status` row
and the GitHub guide's paragraph as the tasks word them, naming no
watch, wait or `sluice run` as what fetches), 5.1 to 5.3 (clippy clean,
the cases and helpers gone), and 6.1 (the change valid, and archived in
a scratch copy all 65 specs pass under 1.14.3).

## For the landing

- Record 0093, which the design, record 0046's amendment and the tasks
  cite as accepted on main, is on `origin/main` and not on this branch's
  base. It reaches the branch when the bolt takes main.
- `sluice exec main` says the branch would conflict with main, now at
  36eb1497, in `crates/sluice-core/src/ops.rs`,
  `crates/sluice/tests/profiles/pull_requests.rs` and
  `site/src/content/docs/reference/commands.md`. Each is textual: in
  `status`, main's record 0091 adds a doc paragraph and a `recorded`
  line beside this change's doc paragraph and policy read, and both
  stand together; in `pull_requests.rs` main and this change make the
  same clippy fix, main's naming the parse error; and the commands
  reference's `sluice status` row is changed on both sides. They are the
  bolt's to resolve when it takes main, and none changes what this
  change requires.
