# Verify

The work on `change/a-quiet-branch-reads-its-remotes-policy` at
9f15ec2a, cut from `bolt/sluice-takes-mended-work-again` at main's
0b15d250, checked on 2026-11-07 against this change's proposal, design,
its three delta specs (`branch-refusal`, `remote-branch`,
`branch-order`) and its tasks.

**The work does not hold yet, and the change is not archived.** The code
meets every requirement and scenario of the three delta specs, all 12
tasks are ticked and each one's condition holds, and the gate's checks
pass on 9f15ec2a. But archived, the specs fail `root:specs`:
`remote-branch`'s modified requirement "A policy refused on the
branch's remote refuses the branch until a push there clears it" grows
from 439 characters to 1111, over openspec's 500, and
`openspec validate --specs --strict` refuses it. The gate does not run
`root:specs` on this branch, since no file under `openspec/specs`
changes here, so only the archive's own commit would meet it. The
change goes back to its code stage for that one delta.

## What fails

| Check | Result |
|---|---|
| `openspec archive a-quiet-branch-reads-its-remotes-policy --yes`, then `openspec validate --specs --strict --no-interactive` (`root:specs`), in a scratch copy of `openspec/` at 9f15ec2a, under `sluice exec main` with the pinned openspec 1.14.3 | the archive applies `branch-order` ~1, `branch-refusal` +2, `remote-branch` ~1; 64 of 65 specs pass, and `remote-branch` fails: `requirements[15]: Requirement text is very long (>500 characters). Move examples and edge cases into scenarios, or split it into separate requirements that each state one behavior.` The same under openspec 1.16.0 |
| The requirement's text, in characters | 439 in `openspec/specs/remote-branch/spec.md` as it stands; 1111 in the change's delta and so once archived. Every other requirement of `remote-branch` is under 490 |

What the code stage has to do: bring `remote-branch`'s delta under 500
characters a requirement, with nothing it says lost. The added text, a
submit reading the branch fetched and a read outside a round reading the
remote's revision as this host last fetched it, fetching nothing, is
one behavior of its own: it can be an ADDED requirement of
`remote-branch`, holding the two scenarios this change adds ("A quiet
branch this host fetched is read whole by status", "A quiet branch this
host never fetched takes a submit"), with the modified requirement's
text kept near its own and naming the next sluice command that reads
the branch. The tests of task 2.4 name those scenarios by title, so
their titles stay. Then `openspec validate --strict` on the change, and
an archive in a scratch copy passing `root:specs`, before the verify
stage runs again.

## Checks

| Check | Result |
|---|---|
| `sluice exec main` on 9f15ec2a | 13 of 13 tasks passed in 10m 13s, recorded under their keys for 9f15ec2: cli:test, cli:tools, conformance:test, core:test, fakes:test, root:adrs, root:changes, root:format, root:lint, root:screenshots-scene, site:prose and site:rules among them; every test the tasks name passed there |
| `openspec validate a-quiet-branch-reads-its-remotes-policy --strict` | valid |
| `cargo clippy -p sluice --features tools --test pull_requests --test remote_branch -- -D warnings`, `cargo clippy -p sluice --test reader_tick -- -D warnings`, `cargo clippy -p sluice-core --all-targets -- -D warnings`, under `sluice exec main` (2.1, 5.1 to 5.3; `root:lint` runs clippy without `--features tools`) | clean |
| `crates/sluice-core/src/ops.rs` and `overview.rs` as 0b15d250 holds them, then `cargo test -p sluice-conformance --test refused_alone -- mended refusal_names_the_revision` under `sluice exec main` (2.2) | the four new cases fail, at `shared.rs` 1823, 1856, 1878 and 1906; restored, the tree clean |
| `overview.rs` as 0b15d250 holds it, then `cargo test -p sluice --test cli -- overview_orders_branches` under `sluice exec main` (3.1) | `overview_orders_branches_by_the_revision_this_host_holds` fails at `cli.rs:2344`, `overview_orders_branches_as_the_policy_names_them` passes; restored, the tree clean |
| `git grep` for the cases 5.1 to 5.3 take out, and their helpers | none of `a_quiet_branch_that_takes_pull_requests_takes_them_in_after_intakes_fetch`, `a_refusal_ends_at_the_tick_after_the_hosts_revision_moved`, `a_quiet_branch_whose_state_is_on_its_remote_clears_at_the_next_tick`, a `mended` module in `reader_tick.rs`, `on_origin`, `last_saw_on_origin`, `stood_on_origin` or `Watch` in `remote_branch.rs`; the `CACHE` left in `remote_branch.rs` is the file's own, used by its earlier cases |

Record 0093, which the design, record 0046's amendment and the tasks
cite as accepted on main, is on `origin/main`
(`docs/adr/0093-a-hosts-queues-move-in-sluices-own-host-process-started-when-work-arrives-and-pull-requests-are-queued-on-the-forge.md`)
and not on this branch's base. The citations name it as main words it,
and it reaches the branch when the bolt takes main.

## Requirements and scenarios

### branch-refusal (added)

| Requirement / scenario | Met by | Test, passed in the gate |
|---|---|---|
| A read outside a round reads the policy at the revision this host holds | `Sluice::status` reads at `Branch::held`, then `base`, then `Branch::read`; `Sluice::branch_view_of` through `view_read`, `held` then `base`, whether or not the records' ask was refused | |
| A quiet branch mended outside sluice | status reads the queue, `at` the records' `R1`, `parallel` from `R2`; the overview's view has no `unread` | `a_quiet_branch_mended_outside_sluice_is_read_whole`, on the fakes (conformance:test) and on git (cli:tools) |
| Status of a submission on the mended branch | `lookup` finds it landed; its branch's status has no warning and no refusal | `status_of_a_submission_on_a_mended_branch_answers_for_it`, fakes and git |
| A read outside a round fetches nothing, writes nothing and names its revision | `held` reads the host's ref with no fetch; neither read writes; `policy_at`'s refusal names the revision read | |
| The refusal names the revision this host holds | status and the overview refuse naming `R2` and `[[checkout.cache]]`, never `R1` | `a_refusal_names_the_revision_this_host_holds`, fakes and git |
| The state is left as it was | `at` is `R1`; the records, their version and the log unchanged; no `moved` | `a_read_of_a_mended_branch_leaves_its_state_as_it_was`, fakes and git |

### remote-branch (modified)

| Requirement / scenario | Met by | Test, passed in the gate |
|---|---|---|
| A policy refused on the branch's remote refuses the branch until a push there clears it | as above, and a submit's read fetched (124), unchanged. **Its text is over openspec's 500 characters, above** | |
| The bolt's own round runs past a copy that lags; A refusal on the remote names the remote's revision; A push to the remote clears the refusal | unchanged by this change | the suite's existing cases in `remote_branch.rs` |
| A quiet branch this host fetched is read whole by status | status and the overview read `main` whole, `at` `R1`, `revision` `R2`; `origin/main`'s reflog, the records and `base` unchanged, no `moved` | `quiet::a_quiet_branch_this_host_fetched_is_read_whole_by_status` (cli:tools) |
| A quiet branch this host never fetched takes a submit | status refuses naming `R1`, not `R2`; the submit fetches `R2` and queues; status after it reads `landed, queued`; `base` and `moved` unchanged | `quiet::a_quiet_branch_this_host_never_fetched_takes_a_submit` (cli:tools) |

### branch-order (modified)

| Requirement / scenario | Met by | Test, passed in the gate |
|---|---|---|
| The order is read from the default branch | `overview()` takes the order from the default branch's `view_read`'s `policy_at`, then its local revision where it has no records | |
| No records on the default branch | unchanged | `overview_orders_branches_as_the_policy_names_them` (cli:test) |
| No policy | unchanged | the suite's existing case |
| The order follows the revision this host holds | the order read at `main`'s held revision, not its records' `base` | `overview_orders_branches_by_the_revision_this_host_holds` (cli:test) |

## Tasks

All 12 are ticked, and each one's condition holds on 9f15ec2a:
1.1 (`root:adrs` in the gate; record 0046's amendment and record 0069's
More information say what the task asks), 1.2 (`site:rules` in the
gate; rule core.15's details as the task words them), 2.1 (clippy
clean), 2.2 to 2.4 (the cases pass in the gate; 2.2 and 3.1 seen
failing with the reads as 0b15d250 holds them, above), 3.1, 4.1 and 4.2
(`site:prose` in the gate; the commands reference's `sluice status` row
and the GitHub guide's paragraph as the tasks word them, naming no
watch, wait or `sluice run` as what fetches), and 5.1 to 5.3 (clippy
clean, the cases and helpers gone).
