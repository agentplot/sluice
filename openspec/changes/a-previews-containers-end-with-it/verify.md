# Verify: a-previews-containers-end-with-it

Branch `change/a-previews-containers-end-with-it` at `0f975da6`, cut from
`bolt/what-a-place-starts-ends-with-it` (merge base `49fcf7d3`; the bolt
stood at `5c4e580e` when the gate ran). Verified 2026-11-11.

Outcome: **not archived.** The branch's own gate passes, and every
requirement, scenario and task holds on the branch. But the branch
cannot land on its bolt. Since this change was cut, the bolt landed the
sibling unit `an-unheld-worktrees-release-runs-its-release-command`
(#93, `5c4e580e`), which also took record 0112:

- `docs/adr/0112-a-worktree-sluice-holds-nothing-for-is-released-by-its-policys-release-command.md`
  is on the bolt.
- `docs/adr/0112-a-previews-containers-end-with-it-found-by-its-checkout-its-ports-or-its-label.md`
  is on this branch.

Merged, the two trees fail the bolt's gate, and they do not merge
cleanly.

## The whole checks

`sluice exec bolt/what-a-place-starts-ends-with-it` (sluice 0.78.1), the
bolt's gate `#check-fast`, run once on `0f975da`: **passed**, exit 0.

```
would conflict: docs/architecture/core.md with bolt/what-a-place-starts-ends-with-it
running the checks 0f975da affects against bolt/what-a-place-starts-ends-with-it (5c4e580)
Tasks: 9 completed
passed: 9 tasks
recorded 9 results under their keys for 0f975da
```

The tasks were `root:format`, `root:lint`, `root:adrs`, `root:changes`,
`nix:typecheck`, `site:prose`, `site:rules`, `plugin-sluice:validate` and
`cli:test`. Each ran, and none was replayed. In `cli:test`, every test
this change added that the `tools` feature does not gate passed:
`a_preview_gets_the_hosts_cache_environment` (task 4.8),
`an_engine_that_keeps_its_connections_open_is_read_to_each_answers_end`
(task 2.6), the five check-profile tests, and `engine.rs`'s two unit
tests.

## What stops it landing

The gate ran on this branch's tree, not on the tree the branch would
land as. sluice printed the conflict before the run. Against the bolt
as it stands:

1. **Two records numbered 0112.** `git merge-tree` of the bolt and
   `0f975da` merges `docs/adr/` with both files in it. `sh
   check-adrs.sh` over that merged `docs/adr/` exits 1:

   ```
   check-adrs: 0112 is the number of docs/adr/0112-a-previews-containers-end-with-it-found-by-its-checkout-its-ports-or-its-label.md docs/adr/0112-a-worktree-sluice-holds-nothing-for-is-released-by-its-policys-release-command.md
   check-adrs: the later of each takes the next free number, from 0113
   ```

   `root:adrs` runs that script and is in `#check-fast`, so the
   candidate this branch forms would fail the bolt's gate. Task 1.2
   already gives the remedy: "If 0112 is taken when this task is built,
   use the next free number and change it everywhere this change names
   it". This change is the later of the two, since the bolt landed its
   sibling first. 0113 is free on the bolt but is taken on
   `bolt/sluices-setup-skill-adds-its-herdr-lines`. Choose the number
   knowing that.
2. **`docs/architecture/core.md` conflicts.** Both units rewrote rule
   core.10's details line from the same base. The bolt's version cites
   the sibling's signal and record 0112 for an unheld worktree's release.
   This branch's version adds the preview containers' case under record
   0112. The merged line must keep both cases, each citing its own
   record, with this change's under its new number.

Where this change names record 0112, all of it to be renumbered:

- Its record file and its frontmatter `number`.
- `docs/architecture/core.md`, core.10's details, then `node
  site/scripts/rules.mjs` to write the site's rules page again.
- Code and test comments: `crates/sluice/src/profiles/checks/engine.rs`,
  `profiles/mod.rs`, `profiles/places.rs`, `runs.rs`, and
  `crates/sluice/tests/{checks,equip_tools,previews,previews_tools,support/mod,profiles/places}.rs`.
- The change's own `proposal.md`, `design.md` and `tasks.md`. The spec
  delta names no record.
- The statement branch `design/sluice-a-previews-containers-end-with-it`
  of agentplot/blueprints (`2d2bd69`), which names record 0112 at
  `design/sluice/proposals/previews-1.md` lines 186, 337 and 348. It is
  unpushed. Before it is pushed, check whether the sibling's statement
  amendment also names 0112.

## The specs

`openspec validate a-previews-containers-end-with-it --strict`: valid.

`preview-release` delta, against the code and tests on the branch:

| Requirement / scenario | Met by |
|---|---|
| Containers end however a preview is let go, after its release command, before checkout, ports and names go | `places.rs` `release_preview` (`release_keeping` → `end_containers` → `runs.remove`, then worktree, scratch, block) and `free` for `Kind::Preview` (`end_keeping` → `end_containers` → `discard`, then worktree, `let_go`); `release_gone` reaches it through `free`; the binding in `profiles/mod.rs` hands `checks::end_containers` over `engine_endpoints()` through `with_containers` (rule core.2) |
| The release command leaves a container | `previews_tools::a_released_previews_container_ends_with_it`; ordering against the held block in `profiles/places::a_released_preview_ends_its_containers_before_its_ports_go` |
| A checkout removed by hand | `previews_tools::a_removed_checkouts_container_ends_with_its_preview`; `places::a_gone_previews_containers_end_after_its_stored_release` |
| A release failure accepted | `previews_tools::an_accepted_previews_container_ends_with_it`; `places::an_accepted_previews_containers_end_with_nothing_run` |
| A failed command | `previews_tools::a_failed_previews_labelled_container_ends_with_it` |
| A container is a preview's by mount, port or label; another checkout's is left | `engine::owned`; `engine::tests::a_container_is_the_places_by_a_mount_a_port_or_its_label`, `a_container_of_another_place_is_left`; `checks::the_places_containers_are_removed_and_no_other` |
| Every engine that answers is asked; forced removal with anonymous volumes; 404 counts as removed | `end_containers` over every endpoint, unix sockets deduplicated by resolved path, `DELETE …?force=true&v=true`, `204 \| 404`; `checks::a_tcp_engine_is_asked_as_a_socket_is`; each unix answer read to the end it gives itself (`read_answer`: `Content-Length`, last chunk, `1xx`/`204`/`304`, else the close), `checks::an_engine_that_keeps_its_connections_open_is_read_to_each_answers_end` |
| A host with no container engine | `checks::a_host_with_no_engine_has_no_container_to_end`; `previews_tools::a_preview_on_a_host_with_no_engine_is_released_as_before` |
| A container sluice could not remove keeps its preview | `end_containers` goes on, then fails naming each container and the engine's `message`; `keep_failure` in `release_preview`, early return in `free`; `checks::a_removal_the_engine_refuses_is_named_and_the_rest_go_on`; `previews_tools::a_container_the_engine_will_not_remove_keeps_its_preview`; `places::a_container_left_keeps_the_preview_and_its_ports` |
| A preview kept by its container is tried again, its stored release never run twice | `release_gone`'s `gone_release` mark and `Runs::release_in`'s `released` mark; `previews_tools::a_gone_previews_refused_container_is_tried_again_by_the_next_listing`; `places::a_container_left_keeps_the_preview_and_its_ports` |

Every requirement and scenario has code and a test that names it.

## The tasks

All 24 tasks in `tasks.md` are ticked, and each holds on the branch:

- 1.1: the blueprints branch `design/sluice-a-previews-containers-end-with-it`
  holds `2d2bd69`, which names clause 101 and record 0112. It is local
  and unpushed, as the task says. Its record number must change with the
  rest (above).
- 1.2: the record exists, proposed and dated 2026-11-11, with D1 to D5
  and their paths not taken. It passes `root:adrs` on the branch, but
  its number is now taken on the bolt. The task's own clause for a
  taken number is unmet (above).
- 1.3: rule core.10's details carry the case. `site:rules` passed on the
  branch. The line conflicts with the bolt's (above).
- 1.4: decision 4 says a unix socket is asked in HTTP/1.0 to close the
  connection and each answer is read to the end it gives itself, citing
  the host whose engine held its connection open (2026-11-11). Reading
  to the close is added as a path not taken.
- 2.1 to 2.6: `checks/engine.rs` (`owned`, `end_containers`,
  `read_answer`, `Connection: close`), the fake engines with `hold`,
  `hold_with`, `refuse`, `removed`, `socket`, `port` and `keep_open`, and
  the six check-profile tests are present. Those the bolt's gate runs
  passed.
- 3.1 to 3.3: `ContainerEnd`, `with_containers`, and the calls in
  `release_preview` and `free` are present, with the four place-profile
  tests.
- 4.1 to 4.7: the binding and `previews_tools.rs`'s `Setup` on its own
  fake engine, with the six binary tests.
- 4.8: `previews.rs`'s `a_preview_gets_the_hosts_cache_environment` points
  `SLUICE_ENGINE_SOCKETS` at its own `support::containers_engine`,
  asserts one listing, and prints standard output beside standard error.
  It passed in the gate.
- 4.9: `equip_tools.rs`'s `Setup` starts its own engine in a
  `Scratch::short` directory and passes it to every `sluice`, and
  `a_preview_is_equipped_before_its_command` asserts one listing.
- 5.1, 5.2: the site guide and both plugin references say it, naming no
  host or tool of ours. `site:prose` and `plugin-sluice:validate` passed.

No other binary test reaches the host's engine through this change's
release. `cli.rs` and `telemetry_logs.rs` release previews through fake
profiles. `stack.rs` is `tools`-gated, is in main's `#check-tools`, and
uses the real engine on purpose.

## For the code stage

1. Merge `bolt/what-a-place-starts-ends-with-it` into this branch.
2. Resolve core.10's details so the line keeps both cases.
3. Renumber this change's record to the next free number, everywhere
   listed above, including the statement branch in agentplot/blueprints.
4. Run the bolt's gate again.
