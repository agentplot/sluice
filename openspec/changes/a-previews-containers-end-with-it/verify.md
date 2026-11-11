# Verify: a-previews-containers-end-with-it

Branch `change/a-previews-containers-end-with-it` at `35d1b5ca`, cut from
`bolt/what-a-place-starts-ends-with-it` (merge base `49fcf7d3`; the bolt
stood at `5c4e580e` when the gate ran). Verified 2026-11-11.

Outcome: **not archived.** The bolt's gate fails on one test that this
change's release reaches. The work otherwise holds against its specs and
tasks.

## The whole checks

`sluice exec bolt/what-a-place-starts-ends-with-it` (sluice 0.78.1), the
bolt's gate `#check-fast`, run once: **failed**, exit 1.

```
running the checks 35d1b5c affects against bolt/what-a-place-starts-ends-with-it (5c4e580)
Tasks: 8 completed, 1 failed
failed: cli:test
recorded 9 results under their keys for 35d1b5c
```

The failure, in `cli:test`, `crates/sluice/tests/previews.rs`:

```
test a_preview_gets_the_hosts_cache_environment ... FAILED
thread 'a_preview_gets_the_hosts_cache_environment' panicked at crates/sluice/tests/previews.rs:113:5:
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.25s
```

Line 113 asserts that `sluice release <preview>` exits 0. It did not. The
assertion prints the release's standard error, which was empty: `sluice
release` prints a failed release on standard output (`could not release
<name>: …`, `crates/sluice/src/preview.rs:363`), which the test does not
show, so the gate's log does not hold the cause.

What ties it to this change: the release of a preview is the one path
this change alters (`Places::release_preview` now calls the check
profile's `end_containers` after the release command). This test, unlike
those in `previews_tools.rs`, sets no `SLUICE_ENGINE_SOCKETS`. Its
`sluice release` therefore asks the host's own engine at
`engine_endpoints()`, which the gate's sandbox lets it reach
(`[executor] docker = true`). That breaks the tasks' own condition, "No
test reaches the host's own container engine", and the release failed
there. The other preview tests in `previews.rs` and every test this
change added passed or are gated behind `tools`.

The other gate tasks passed: `root:format`, `root:lint`, `root:adrs`,
`root:changes`, `site:prose`, `site:rules`, `plugin-sluice:validate` and
the rest of `cli:test`.

For the code stage: point this test's `sluice` at an engine of the test's
own (`support::containers_engine`, or a `SLUICE_ENGINE_SOCKETS` naming no
socket, as `a_preview_on_a_host_with_no_engine_is_released_as_before`
does), and make the assertion show standard output too. Also find why the
release failed against the host's engine. A host engine that answers
`_ping` but fails a listing keeps every preview (design D5). That may be
the intended behaviour, but it should be a choice made knowingly, not
something this test happened to find.

## The specs

`openspec validate a-previews-containers-end-with-it --strict`: valid.

`preview-release` delta, against the code and tests (each named test
exists; the binary-level ones are `tools`-gated):

| Requirement / scenario | Met by |
|---|---|
| Containers end however a preview is let go, after its release command, before checkout, ports and names go | `places.rs` `release_preview` (`release_keeping` → `end_containers` → `runs.remove`, then worktree, scratch, block) and `free` for `Kind::Preview` (`end_keeping` → `end_containers` → `discard`, then worktree, `let_go`); `release_gone` reaches it through `free` |
| The release command leaves a container | `previews_tools::a_released_previews_container_ends_with_it`; ordering against the held block in `profiles/places::a_released_preview_ends_its_containers_before_its_ports_go` |
| A checkout removed by hand | `previews_tools::a_removed_checkouts_container_ends_with_its_preview`; `places::a_gone_previews_containers_end_after_its_stored_release` |
| A release failure accepted | `previews_tools::an_accepted_previews_container_ends_with_it`; `places::an_accepted_previews_containers_end_with_nothing_run` |
| A failed command | `previews_tools::a_failed_previews_labelled_container_ends_with_it` |
| A container is a preview's by mount, port or label; another checkout's is left | `engine::owned` (whole-component path match, given and resolved; `PublicPort` among `SLUICE_PORT_*`; `sluice.place` label); `engine::tests::a_container_is_the_places_by_a_mount_a_port_or_its_label`, `a_container_of_another_place_is_left`; `checks::the_places_containers_are_removed_and_no_other` |
| Every engine that answers is asked; forced removal with anonymous volumes; 404 counts as removed | `end_containers` over every `engine_endpoints()` entry, unix sockets deduplicated by resolved path, `DELETE …?force=true&v=true`, `204 \| 404`; `checks::a_tcp_engine_is_asked_as_a_socket_is` |
| A host with no container engine | `checks::a_host_with_no_engine_has_no_container_to_end`; `previews_tools::a_preview_on_a_host_with_no_engine_is_released_as_before` |
| A container sluice could not remove keeps its preview | `end_containers` goes on, then fails naming each container and the engine's `message`; `keep_failure` in `release_preview`, early return in `free`; `checks::a_removal_the_engine_refuses_is_named_and_the_rest_go_on`; `previews_tools::a_container_the_engine_will_not_remove_keeps_its_preview`; `places::a_container_left_keeps_the_preview_and_its_ports` |
| A preview kept by its container is tried again, its stored release never run twice | `release_gone`'s `gone_release` mark and `Runs::release_in`'s `released` mark; `previews_tools::a_gone_previews_refused_container_is_tried_again_by_the_next_listing`; `places::a_container_left_keeps_the_preview_and_its_ports` |

Every requirement and scenario has code and a test that names it. The
one shortfall is the gate failure above, which falls under the tasks'
condition that no test reaches the host's engine.

## The tasks

All 20 tasks in `tasks.md` are ticked. Checked against the tree:

- 1.1: the blueprints branch `design/sluice-a-previews-containers-end-with-it`
  holds `2d2bd69` naming clause 101 and record 0112. It is local and
  unpushed, as the task says, for the orchestrator to push before the code
  lands (rule core.12).
- 1.2, 1.3: record 0112 exists, proposed and dated 2026-11-11. Rule
  core.10's details carry the case and record 0112. `root:adrs` and
  `site:rules` passed in the gate.
- 2.1 to 2.5: `checks/engine.rs`, the fake engines in `tests/support`, and
  the five check-profile tests are present.
- 3.1 to 3.3: `ContainerEnd`, `with_containers`, and the calls in
  `release_preview` and `free` are present, with the four place-profile
  tests.
- 4.1 to 4.7: the binding passes `end_containers` through
  `with_containers` (rule core.2). `previews_tools.rs`'s `Setup` points
  every `sluice` at its own fake engine, and the six binary tests are
  present. 4.1's isolation covers `previews_tools.rs` only, not the
  release in `previews.rs`, which is the failure above.
- 5.1, 5.2: the site guide and both plugin references say it, naming no
  host or tool of ours. `site:prose` and `plugin-sluice:validate` passed.
