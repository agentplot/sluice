# Verify: a-previews-containers-end-with-it

The branch `change/a-previews-containers-end-with-it` was verified at
`f714a527` on 2026-11-12. It was cut from
`bolt/what-a-place-starts-ends-with-it` and holds that bolt at `8306248`,
the bolt's revision when the gate ran, and `main` at `5cf91de`.

Outcome: **archived.** The bolt's gate passed all 23 of its tasks.
`cli:test` ran the tests this change adds that the gate reaches. Every
requirement and scenario has code and a test, all 28 tasks are ticked,
and the record is 0121 everywhere this change names it.

## The whole checks

The gate is `sluice exec bolt/what-a-place-starts-ends-with-it`
(sluice 0.91.4), the bolt's `#check-fast`. It ran once on `f714a52` and
**passed** with exit 0.

```
running the checks f714a52 affects against bolt/what-a-place-starts-ends-with-it (8306248)
waiting: the host is at its bound
...
Tasks: 23 completed
 Time: 6m 7s 67ms

passed: 23 tasks
recorded 23 results under their keys for f714a52
```

| Task | Outcome |
|---|---|
| `root:format` | passed |
| `root:lint` | passed |
| `root:adrs` | passed |
| `root:changes` | passed |
| `root:specs` | passed |
| `root:versions` | passed |
| `nix:lint` | passed |
| `nix:typecheck` | passed |
| `nix:test` | passed |
| `site:prose` | passed |
| `site:rules` | passed |
| `core:test` | passed |
| `seams:test` | passed |
| `fakes:test` | passed |
| `conformance:test` | passed |
| `cli:test` | passed (37 s) |
| `plugin-sluice:validate` | passed |
| `plugin-board:validate` | passed |
| `plugin-board:typecheck` | passed |
| `plugin-board:test` | passed |
| `plugin-herdr:manifest` | passed |
| `plugin-herdr:test-sh` | passed (138 passed, 3 skipped) |
| `plugin-herdr:test-dash` | passed |

The Rust suites ran 1218 tests in 71 binaries, none failed. `cli:test`
ran these tests of this change, each `ok`:

- in `engine.rs`: `profiles::checks::engine::tests::a_container_is_the_places_by_a_mount_a_port_or_its_label` and `a_container_of_another_place_is_left`;
- in `tests/checks.rs`: `the_places_containers_are_removed_and_no_other`, `a_removal_the_engine_refuses_is_named_and_the_rest_go_on`, `a_host_with_no_engine_has_no_container_to_end`, `a_tcp_engine_is_asked_as_a_socket_is` and `an_engine_that_keeps_its_connections_open_is_read_to_each_answers_end`;
- in `tests/previews.rs`: `a_preview_gets_the_hosts_cache_environment`, against a fake engine of its own.

The linker was not ended in this run. The run printed 23 warnings,
`Failed to store blobs` and `Failed to package task outputs`, moon cache
writes the sandbox refused, the case record 0105 names. No task failed
on them.

The `tools`-gated tests are in `tests/profiles/places.rs`,
`tests/previews_tools.rs` and `tests/equip_tools.rs`. They belong to
main's `#check-tools` and not to the bolt's gate, so this stage did not
run them. Their tasks, 3.1 to 3.3, 4.1 to 4.7 and 4.9, name the
`sluice exec … --features tools` run that proves each one. They run
when the bolt merges onto main.

## The merges

The branch holds the bolt at `8306248` (`c67570e8`) and `main` at
`5cf91de` (`47f533e2`). The gate ran on both with no conflict.

- **Records.** `docs/adr/` holds main's records 0114, 0116 and 0117,
  the bolt's 0120 and 0115, and this change's 0121 (frontmatter
  `number: 121`, status proposed, 2026-11-11). Record 0118 is on
  `fix/main--a-claimed-runs-place-is-made-at-its-combination`. No code,
  test, doc or change file of this change still names 0114 for its
  record; the 0114s left in `crates/sluice-fakes` and `tests/cli.rs` are
  main's record. `root:adrs` passed.
- **Rules.** Rule core.9's and core.11's details are main's word for
  word. Rule core.10's details hold the unheld worktree's case under
  record 0120 and this change's case under record 0121, marked
  proposed; its source names previews-1 clause 101 as amended at
  agentplot/blueprints `0f84603` and both records. The rule's own line
  is unchanged. `site:rules` passed.
- **Statement.** The agentplot/blueprints branch
  `design/sluice-a-previews-containers-end-with-it` is at `0f84603`, in
  the worktree
  `~/Code/github_agentplot/blueprints/design-sluice-a-previews-containers-end-with-it`,
  three commits past its base (`2d2bd69`, `e3433e8`, `0f84603`). Its last
  subject names clause 101 and record 0121. Clause 101, the C.1 preview
  row, the frees under 123 and S78 in `previews-1.md` cite record 0121
  as proposed. S78 is still free on `origin/main`, the branch is 14
  commits behind it and merges with it cleanly, and it is **unpushed**:
  the orchestrator pushes it on the user's word before the code lands
  (rule core.12).

## The specs

`openspec validate a-previews-containers-end-with-it --strict` reports
the change valid. `openspec/specs/preview-release/spec.md` holds none of
the five requirements this delta adds, so it archives with no clash.

The `preview-release` delta, checked against the code and tests on the
branch:

| Requirement / scenario | Met by |
|---|---|
| A preview's containers end with it, however it is let go: after its release command and before its checkout, ports and names go | `places.rs`: `release_preview` runs `runs.release_keeping`, then `end_containers`, then `runs.remove`, and only after those the worktree, the scratch, the block and the record. `free`, for `Kind::Preview`, runs `runs.end_keeping` and `end_containers` before `discard`, the worktree and `let_go`. `release_gone` reaches the end through `free`. `profiles/mod.rs` hands over `checks::end_containers(&checks::engine_endpoints(), place)` through `with_containers` (rule core.2). |
| Scenario: the release command leaves a container | `previews_tools::a_released_previews_container_ends_with_it`. `places::a_released_preview_ends_its_containers_before_its_ports_go` checks the order against the held block. |
| Scenario: a checkout removed by hand | `previews_tools::a_removed_checkouts_container_ends_with_its_preview`; `places::a_gone_previews_containers_end_after_its_stored_release` |
| Scenario: a release failure accepted | `previews_tools::an_accepted_previews_container_ends_with_it`; `places::an_accepted_previews_containers_end_with_nothing_run` |
| Scenario: a failed command | `previews_tools::a_failed_previews_labelled_container_ends_with_it` (`release_unusable_previews` → `release_preview` with `failed`) |
| A container is a preview's by its mount, its port or its label | `engine::owned` compares mounts by whole components, as given and as resolved. It reads `PublicPort` against `SLUICE_PORT_*` and the `sluice.place` label against `SLUICE_PLACE`, and nothing else. Tests: `engine::tests::a_container_is_the_places_by_a_mount_a_port_or_its_label` and `a_container_of_another_place_is_left` (passed in the gate). |
| Scenario: another checkout's container is left | `checks::the_places_containers_are_removed_and_no_other` (passed in the gate); `previews_tools::a_released_previews_container_ends_with_it` |
| Every engine that answers is asked: stopped and removed with its anonymous volumes, a container already gone counting as removed | `end_containers` asks every endpoint and each unix socket once by its resolved path. It passes over any endpoint that does not answer `_ping` with `200`, and sends `DELETE /containers/{id}?force=true&v=true`, taking `204` or `404`. `read_answer` reads each unix-socket answer to the end the answer gives itself. The listing is bounded to ten seconds and a removal to thirty. Tests: `checks::a_tcp_engine_is_asked_as_a_socket_is` and `checks::an_engine_that_keeps_its_connections_open_is_read_to_each_answers_end` (passed in the gate). |
| Scenario: a host with no container engine | `checks::a_host_with_no_engine_has_no_container_to_end` (passed in the gate); `previews_tools::a_preview_on_a_host_with_no_engine_is_released_as_before` |
| A container sluice could not remove keeps its preview | `end_containers` goes on through the rest, then fails with `could not remove the containers <place> left:`, naming each container and the engine's `message`. `release_preview` keeps the failure (`keep_failure`), and `free` returns before anything is let go. Test: `checks::a_removal_the_engine_refuses_is_named_and_the_rest_go_on` (passed in the gate). |
| Scenario: the engine refuses a removal | `previews_tools::a_container_the_engine_will_not_remove_keeps_its_preview`; `places::a_container_left_keeps_the_preview_and_its_ports` |
| A preview kept by its container is tried again, and its stored release never runs twice | `release_gone` writes its `gone_release` mark before it starts, and `Runs::release` writes a `released` mark that `runs.remove` honours after `release_keeping`. A later free tries the end alone. |
| Scenario: the engine removes it on a later try | `previews_tools::a_gone_previews_refused_container_is_tried_again_by_the_next_listing`; `places::a_container_left_keeps_the_preview_and_its_ports` |

Every requirement and scenario has code and a test that names it.

## The tasks

All 28 tasks in `tasks.md` are ticked, and none is open.

- **1.1, 1.5 and 4.12 (the statement).** The blueprints branch is at
  `0f84603`, whose subject names clause 101 and record 0121. It is
  unpushed.
- **1.2 and 1.4.** Record 0121 holds D1 to D5, each with its path not
  taken. Its decision 4 reads a unix-socket answer to the end it gives
  itself, and reading to the close is a path not taken. `root:adrs`
  passed.
- **1.3, 1.5 and 4.12 (the rule).** Rule core.10's details keep both
  cases and both records, and core.9's and core.11's are main's.
  `site:rules` passed.
- **2.1 to 2.6.** The code is in `checks/engine.rs` (`owned`,
  `end_containers`, `read_answer`, `unix_call`, `tcp_call`), declared
  from `checks.rs` and re-exported with `PLACE_LABEL`. The fake engines
  in `tests/support` provide `hold`, `hold_with`, `refuse`, `removed`,
  `socket`, `port` and `keep_open`. Their seven tests passed in the gate.
- **3.1 to 3.3.** `ContainerEnd`, `with_containers`, and the calls in
  `release_preview` and `free` are present, with the four place-profile
  tests. These tests are `tools`-gated.
- **4.1 to 4.9.** The binding is present. `previews_tools.rs`,
  `equip_tools.rs` and `previews.rs` each run against a fake engine of
  their own. `previews.rs` passed in the gate.
- **4.10 and 4.11.** The bolt's gate passes on the branch with the bolt
  at `8306248` taken in, and its `cli:test` ran this change's tests in
  `engine.rs`, `checks.rs` and `previews.rs`, judged under its new key.
  Nothing was set aside.
- **5.1 and 5.2.** The site guide and both plugin references describe
  the three marks and the `sluice.place` label. They name no host or
  tool of ours. `site:prose` and `plugin-sluice:validate` passed.

## Left for others

- The agentplot/blueprints branch
  `design/sluice-a-previews-containers-end-with-it` at `0f84603` is
  unpushed. The orchestrator pushes it on the user's word, before this
  change lands on main (rule core.12).
- Record 0121 is proposed until the user has seen it as a record.
- The `tools`-gated tests run under main's `#check-tools` when the bolt
  merges onto main.
