# Verify: a-previews-containers-end-with-it

Branch `change/a-previews-containers-end-with-it` at `b2e84e3a`, cut from
`bolt/what-a-place-starts-ends-with-it`, which it holds at `5c4e580e`, the
bolt's revision when the gate ran. Verified 2026-11-11.

Outcome: **not archived.** The bolt's gate failed on `cli:test`, which
never ran a test: the linker building the `runs` test binary was ended
by SIGTERM. The other eight tasks passed. The branch merges onto the bolt
with no conflict, and every requirement, scenario and task holds in the
code and tests on the branch. The tests that prove the scenarios have not
run on this tree, because `cli:test` stopped before them.

## The whole checks

`sluice exec bolt/what-a-place-starts-ends-with-it` (sluice 0.78.1), the
bolt's gate `#check-fast`, run once on `b2e84e3`: **failed**, exit 1.

```
running the checks b2e84e3 affects against bolt/what-a-place-starts-ends-with-it (5c4e580)
...
cli:test | error: linking with `cc` failed: exit status: 1
cli:test |   = note: Terminated: 15  /nix/store/67105gg3dxn344vi8v8bf5y44v1rmqxf-cctools-binutils-darwin-1010.6/bin/ld "@$responseFile"
cli:test |           clang: error: linker command failed with exit code 143 (use -v to see invocation)
cli:test | error: could not compile `sluice` (test "runs") due to 1 previous error
...
Tasks: 8 completed, 1 failed
failed: cli:test
recorded 9 results under their keys for b2e84e3
```

| Task | Outcome |
|---|---|
| `root:format` | passed (390 files, 0 changed) |
| `root:lint` | passed |
| `root:adrs` | passed |
| `root:changes` | passed |
| `nix:typecheck` | passed |
| `site:prose` | passed |
| `site:rules` | passed ("the page matches docs/architecture/core.md") |
| `plugin-sluice:validate` | passed |
| `cli:test` | **failed**, 12.7 s in, linking the `runs` test binary |

What failed:

- `ld` exited 143, which is 128 + 15: something outside the build sent
  it SIGTERM. No compiler error came first. The crates this change
  touches had already compiled, and the failing binary, `tests/runs.rs`,
  is not one this change adds or edits.
- `cli:test` had waited on cargo's build-directory lock while
  `root:lint`'s clippy ran in the same `target/`, which is normal under
  moon. When the stage looked afterwards, no other process held this
  place's `target/`. The system log held nothing for the window, and the
  sender of the signal was not found.
- sluice recorded the fail under `cli:test`'s key for `b2e84e3`. It was
  not read as a run with no outcome.

Seen in the same run, and blocking nothing: every task warned
`Failed to store blobs` under
`sluice.git/moon-home/cache/shared/blobs/temp/`, and `Failed to package
task outputs` under this place's `.moon/cache/outputs/`. These are writes
to moon's caches that the sandbox refused, the case record 0105 names.
The tasks that passed still passed.

Since no test ran, none of the tests this change adds has run on
`b2e84e3`: the six in `checks.rs`, `engine.rs`'s two unit tests, and
`previews.rs`'s `a_preview_gets_the_hosts_cache_environment`.
`cli:test` is the task that runs them. The `tools`-gated tests in
`profiles/places.rs`, `previews_tools.rs` and `equip_tools.rs` are not in
the bolt's gate.

## The merge with the bolt

The branch holds the bolt's `5c4e580e` (`d8b310f0`), and sluice printed no
conflict before the run.

- **Records.** `docs/adr/` holds
  `0112-a-worktree-sluice-holds-nothing-for-is-released-by-its-policys-release-command.md`
  (the bolt's) and
  `0114-a-previews-containers-end-with-it-found-by-its-checkout-its-ports-or-its-label.md`
  (this change's, frontmatter `number: 114`). There is no 0113 here,
  since it is taken on `bolt/sluices-setup-skill-adds-its-herdr-lines`.
  `root:adrs` passed.
- **Rule core.10's details.** They keep the unheld worktree's case under
  record 0112 and the preview containers' case under record 0114. They
  cite previews-1 clause 101 as amended at agentplot/blueprints
  `e3433e8`, and the ADR list names both 0112 and 0114. The site's
  rules page matches (`site:rules`).
- **Code and tests.** Every comment this change wrote names record 0114.
  The 0112 citations left in `places.rs` (lines 940, 987, 1031 and 1764),
  `exec.rs`, `sluice-seams`, `sluice-fakes`, `cli.rs` and `exec.rs`'s
  test are the sibling's. The lines in `places.rs` come from the bolt's
  commit `5f93d35c`, and the other files are as the bolt has them.
  After the merge, `free` still calls `end_keeping` and then
  `end_containers` for a preview before it discards the run, removes the
  worktree and lets the preview go. `release_preview` still chains
  `release_keeping`, `end_containers` and `runs.remove` under
  `keep_failure`.
- **Statement.** The agentplot/blueprints branch
  `design/sluice-a-previews-containers-end-with-it`, in the worktree
  `~/Code/github_agentplot/blueprints/design-sluice-a-previews-containers-end-with-it`,
  is at `e3433e8`, one commit after `2d2bd69`. Its subject names clause
  101 and record 0114. `previews-1.md` names record 0114 at lines 186,
  222, 337 and 348, and nowhere names 0112. It is unpushed.

## The specs

`openspec validate a-previews-containers-end-with-it --strict`: valid.

The `preview-release` delta, checked against the code and tests on the
branch:

| Requirement / scenario | Met by |
|---|---|
| Containers end however a preview is let go: after its release command, before the checkout, ports and names go | `places.rs`: `release_preview` runs `release_keeping`, then `end_containers`, then `runs.remove`, and only after that the worktree, the scratch and the block. `free` for `Kind::Preview` runs `end_keeping` and `end_containers`, then `discard`, the worktree and `let_go`. `release_gone` reaches the end through `free`. `profiles/mod.rs` hands over `checks::end_containers` on `engine_endpoints()` through `with_containers` (rule core.2). |
| The release command leaves a container | `previews_tools::a_released_previews_container_ends_with_it`; the ordering against the held block in `places::a_released_preview_ends_its_containers_before_its_ports_go` |
| A checkout removed by hand | `previews_tools::a_removed_checkouts_container_ends_with_its_preview`; `places::a_gone_previews_containers_end_after_its_stored_release` |
| A release failure accepted | `previews_tools::an_accepted_previews_container_ends_with_it`; `places::an_accepted_previews_containers_end_with_nothing_run` |
| A failed command | `previews_tools::a_failed_previews_labelled_container_ends_with_it` |
| A container is a preview's by its mount, port or label; another checkout's is left | `engine::owned`; `engine::tests::a_container_is_the_places_by_a_mount_a_port_or_its_label` and `a_container_of_another_place_is_left`; `checks::the_places_containers_are_removed_and_no_other` |
| Every engine that answers is asked; forced removal with anonymous volumes; 404 counts as removed | `end_containers` asks every endpoint, each unix socket once by its resolved path, with `DELETE …?force=true&v=true`, and takes `204` or `404`. `read_answer` reads each answer to the end it gives itself. Tests: `checks::a_tcp_engine_is_asked_as_a_socket_is` and `checks::an_engine_that_keeps_its_connections_open_is_read_to_each_answers_end`. |
| A host with no container engine | `checks::a_host_with_no_engine_has_no_container_to_end`; `previews_tools::a_preview_on_a_host_with_no_engine_is_released_as_before` |
| A container sluice could not remove keeps its preview | `end_containers` goes on through the rest, then fails naming each container left and the engine's `message`. `release_preview` keeps the failure (`keep_failure`), and `free` returns early. Tests: `checks::a_removal_the_engine_refuses_is_named_and_the_rest_go_on`, `previews_tools::a_container_the_engine_will_not_remove_keeps_its_preview` and `places::a_container_left_keeps_the_preview_and_its_ports`. |
| A preview kept by its container is tried again, and its stored release never runs twice | `release_gone`'s `gone_release` mark and `Runs::release_in`'s `released` mark; `previews_tools::a_gone_previews_refused_container_is_tried_again_by_the_next_listing`; `places::a_container_left_keeps_the_preview_and_its_ports` |

Every requirement and scenario has code and a test that names it. None of
those tests ran on `b2e84e3` (above).

## The tasks

All 25 tasks in `tasks.md` are ticked:

- 1.1 and 1.5's statement part: `e3433e8` on the blueprints branch, its
  subject naming clause 101 and record 0114, unpushed.
- 1.2: record 0114 is proposed and dated 2026-11-11, with D1 to D5 and
  their paths not taken. `root:adrs` passed.
- 1.3 and 1.5: core.10's details keep both cases and both records.
  `site:rules` passed.
- 1.4: decision 4 reads each unix-socket answer to the end it gives
  itself, citing the host whose engine held its connection open, with
  reading to the close as a path not taken.
- 2.1 to 2.6: `checks/engine.rs` (`owned`, `end_containers`,
  `read_answer`), the fake engines (`hold`, `hold_with`, `refuse`,
  `removed`, `socket`, `port`, `keep_open`) and the six check-profile
  tests are present. They are not run on this tree.
- 3.1 to 3.3: `ContainerEnd`, `with_containers`, and the calls in
  `release_preview` and `free` are present, with the four place-profile
  tests. These are `tools`-gated and outside the bolt's gate.
- 4.1 to 4.9: the binding is present. `previews_tools.rs`,
  `equip_tools.rs` and `previews.rs` each run against a fake engine of
  their own. The seven binary tests are present.
- 5.1 and 5.2: the site guide and both plugin references say it, naming
  no host or tool of ours. `site:prose` and `plugin-sluice:validate`
  passed.

## For the code stage

1. Run the bolt's gate on the branch again:
   `sluice exec bolt/what-a-place-starts-ends-with-it`. `cli:test` must
   pass, with this change's tests among those it runs.
2. If the linker is ended again, find what sends it SIGTERM under
   `sluice exec` and fix it or report it, as CLAUDE.md's Running checks
   asks. Do not route around it.
