# Tasks

Work in this change's place on `change/a-previews-containers-end-with-it`.
It was cut from `bolt/what-a-place-starts-ends-with-it`, the base every
`sluice exec` below names. A unit test, or a test of a target the
`tools` feature does not gate, runs directly. A test of the `tools`
feature and each moon task run through `sluice exec
bolt/what-a-place-starts-ends-with-it -- …`, as CLAUDE.md's Running
checks has it.

No test reaches the host's own container engine. The engine is
`support::containers_engine`, a fake on a unix socket in a
`Scratch::short` directory, or a TCP port on 127.0.0.1 for the TCP test.
Commit by pathspec, with Conventional Commits subjects scoped to the part
of the tree touched, citing clause 34, previews-1 clause 101, record 0112
and rule core.10.

## 1. The decision recorded

- [ ] 1.1 Amend the statement on the agentplot/blueprints branch `design/sluice-a-previews-containers-end-with-it`, in a worktree `~/Code/github_agentplot/blueprints/design-sluice-a-previews-containers-end-with-it` cut from `origin/main`, as design D7 gives it. Edit `design/sluice/proposals/previews-1.md` in four places:
  - Clause 101 gains the end of the preview's containers, by mounts, ports or `sluice.place` label, whichever way it is let go, after its release command and before its ports and names go. A preview whose container cannot be removed is kept, with what failed.
  - The C.1 preview row's release cell gains the removal between the release command and `git worktree remove`.
  - "What it changes elsewhere" gains that 123's free of a gone preview and of an accepted failure reaches the removal first.
  - One scenario after S35, numbered as the next S number free in `design/sluice/`. In it a checkout is removed by hand, its release command fails in the scratch directory, and its container is removed all the same before its ports go.

  Mark each one proposed, record 0112. Commit there and do not push: the orchestrator pushes it on the user's word, before the code lands (rule core.12). True when `git -C ~/Code/github_agentplot/blueprints/design-sluice-a-previews-containers-end-with-it log -1 --format=%s` names clause 101 and record 0112.
- [ ] 1.2 Write record 0112 as `docs/adr/0112-a-previews-containers-end-with-it-found-by-its-checkout-its-ports-or-its-label.md`, as design D7 gives it, in MADR form like record 0109.
  - Status: proposed, dated 2026-11-11. Its decision-makers are this change's construct stage, and its sources are signal 2026-11-11-agentplot-operator-6799f524/01, clause 34, previews-1 clauses 101 and 123, records 0016 and 0074, and rule core.10.
  - It holds D1 to D5 of the design, each with its path not taken.
  - If 0112 is taken when this task is built, use the next free number and change it everywhere this change names it.

  True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run root:adrs` passes.
- [ ] 1.3 In `docs/architecture/core.md`, add the case to rule core.10's details, as design D7 gives it, citing the signal and record 0112, and add 0112 to its ADR list. Leave the rule's own line as it is. Then run `node site/scripts/rules.mjs` to write the site's rules page again. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run site:rules` passes.

## 2. The check profile finds a place's containers and removes them

- [ ] 2.1 Add `crates/sluice/src/profiles/checks/engine.rs`, declared `mod engine;` from `checks.rs`, its doc naming it the check profile's own talk with the container engine (rule core.2). Add `owned(container: &serde_json::Value, place: &Place) -> bool`, as design D2 gives it: a container is the place's by a mount's `Source` inside `place.path` or `place.tmp`, as given or resolved, compared by whole components; by a `PublicPort` among the place's `SLUICE_PORT_*` values; or by its `sluice.place` label equal to the place's `SLUICE_PLACE`. Add the unit tests `a_container_is_the_places_by_a_mount_a_port_or_its_label` and `a_container_of_another_place_is_left`, the second covering a sibling directory whose name begins with the checkout's. True when `cargo test -p sluice --lib profiles::checks::engine::` passes.
- [ ] 2.2 In `crates/sluice/tests/support/mod.rs`, add `containers_engine(dir: &Path) -> Engine`, a fake engine at `e.sock` in `dir`, with these calls:
  - `GET /_ping` answers `200 OK`.
  - `GET /containers/json?all=true` answers the containers it holds, as `Content-Length` JSON. `Engine::hold(serde_json::Value)` sets them, and `Engine::hold_with(impl Fn() -> serde_json::Value + Send + 'static)` makes them at each listing.
  - `DELETE /containers/<id>?force=true&v=true` removes the container and answers `204`, `404` for one it does not hold, or a status `Engine::refuse(id, times)` gives for the next `times` asks.
  - `Engine::removed()` lists the ids removed, in order, and `Engine::socket()` gives its path.

  In `engine.rs`, add `end_containers(endpoints: &[Endpoint], place: &Place) -> Result<()>` over unix sockets, as design D4 gives it: ping, list, then remove each container `owned` takes. Re-export it from `checks`, make `ping` visible to `engine`, and trace each container removed. Add `the_places_containers_are_removed_and_no_other` to `crates/sluice/tests/checks.rs`, the preview-release spec's "Another checkout's container is left" at the profile's level, with one container each by mount, port and label and one of another place. True when `cargo test -p sluice --test checks the_places_containers_are_removed_and_no_other` passes.
- [ ] 2.3 In `end_containers`, on an answering engine whose listing or removal fails:
  - Go on through every other container.
  - Then fail starting `could not remove the containers <place> left:`, followed by each container's first name and the engine's `message`, or the listing's failure.
  - Count `404` as removed.

  Add `a_removal_the_engine_refuses_is_named_and_the_rest_go_on` to `crates/sluice/tests/checks.rs`. True when `cargo test -p sluice --test checks a_removal_the_engine_refuses_is_named_and_the_rest_go_on` passes.
- [ ] 2.4 Make `end_containers` return `Ok(())` with nothing asked where no endpoint exists or none answers `GET /_ping` with `200`. Pass over a unix socket whose resolved path was already asked, as `engine_unmet` does. Add `a_host_with_no_engine_has_no_container_to_end` to `crates/sluice/tests/checks.rs`, using a socket path with nothing at it and one with a 500 engine from `support::engine`. True when `cargo test -p sluice --test checks a_host_with_no_engine_has_no_container_to_end` passes.
- [ ] 2.5 Make `end_containers` reach a TCP endpoint through `ureq`, with the TLS `engine_tls_config` gives, as `ping_tcp` does. Bound the listing to ten seconds and a removal to thirty, as design D4 gives it. Add a TCP form of the fake, `support::tcp_containers_engine() -> Engine` with `Engine::port()`, and the test `a_tcp_engine_is_asked_as_a_socket_is` to `crates/sluice/tests/checks.rs`. True when `cargo test -p sluice --test checks a_tcp_engine_is_asked_as_a_socket_is` passes.

## 3. The place profile ends a preview's containers however it is let go

- [ ] 3.1 In `crates/sluice/src/profiles/places.rs`, add `pub type ContainerEnd = Box<dyn Fn(&Place) -> Result<()>>`, a `containers: Option<ContainerEnd>` field, and `Places::with_containers`, as design D1 gives it. In `release_preview`, after `self.runs.remove(name)` and before `remove_worktree`, call it with `preview_run_place(name, &made)`. On its error, call `keep_failure(name, &e, |tries| tries > retries)` and return the error (design D3, D5). Make the module doc say a preview's containers end between its release command and its checkout's removal. Add `a_released_preview_ends_its_containers_before_its_ports_go` to `crates/sluice/tests/profiles/places.rs`. Its `ContainerEnd` asserts the preview's block is still held, then calls `checks::end_containers` on a `support::containers_engine`. The test checks that the preview's containers are removed and the other place's is not. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test places a_released_preview_ends_its_containers_before_its_ports_go` passes.
- [ ] 3.2 In `free`, for a record of kind `Preview`, call the end after `self.runs.discard(name)` and before `remove_worktree` and `let_go`, and return its error before anything is let go (design D3, D5). Add these tests to `crates/sluice/tests/profiles/places.rs`:
  - `a_gone_previews_containers_end_after_its_stored_release`: `release_gone` on a preview whose checkout was removed, with a release command that fails in the scratch directory.
  - `an_accepted_previews_containers_end_with_nothing_run`: `free` on a preview with a release command that would log.

  True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test places -- a_gone_previews_containers_end_after_its_stored_release an_accepted_previews_containers_end_with_nothing_run` passes.
- [ ] 3.3 Add `a_container_left_keeps_the_preview_and_its_ports` to `crates/sluice/tests/profiles/places.rs`, with an engine refusing one removal at a time:
  - `release_preview` fails, and the record keeps `release_failed` with one try, its block and its checkout.
  - A `release_gone` of the same preview with its checkout removed fails and lets nothing go.
  - A second `release_gone` once the engine removes the container frees it, and the stored release command has run once in all.

  True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test places a_container_left_keeps_the_preview_and_its_ports` passes.

## 4. The binary

- [ ] 4.1 In `crates/sluice/src/profiles/mod.rs`, hand the place profile `Box::new(|place| checks::end_containers(&checks::engine_endpoints(), place))` through `with_containers`, beside `with_release`, with a comment citing rule core.2 and record 0112. In `crates/sluice/tests/previews_tools.rs`, make `Setup` start its own `support::containers_engine` in a `Scratch::short` directory and pass `SLUICE_ENGINE_SOCKETS` with its socket to every `sluice` it runs, so no test reaches the host's engine. Add `a_released_previews_container_ends_with_it`, the preview-release spec's "The release command leaves a container" and "Another checkout's container is left". Once the preview's checkout and port are known, the engine holds one container each by mount, port and label and one of another checkout. The release command logs, and the test checks that it ran, that the three were removed, and that the fourth was left. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test previews_tools a_released_previews_container_ends_with_it` passes.
- [ ] 4.2 Add `a_removed_checkouts_container_ends_with_its_preview` to `crates/sluice/tests/previews_tools.rs`, the spec's "A checkout removed by hand". Its release command is a script committed in the checkout, logging when it runs. After `git worktree remove`, `sluice previews` fails that command once in the scratch directory, removes the container that mounts a file of the checkout, and frees the preview. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test previews_tools a_removed_checkouts_container_ends_with_its_preview` passes.
- [ ] 4.3 Add `an_accepted_previews_container_ends_with_it` to `crates/sluice/tests/previews_tools.rs`, the spec's "A release failure accepted": a release command that exits 1, `sluice release` exiting 1, then `sluice release <preview> --accept-failure` exiting 0, the container that publishes its port removed and no further release logged. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test previews_tools an_accepted_previews_container_ends_with_it` passes.
- [ ] 4.4 Add `a_failed_previews_labelled_container_ends_with_it` to `crates/sluice/tests/previews_tools.rs`, the spec's "A failed command". The preview command writes its `SLUICE_PLACE` to a file of the test's and exits 1. The engine, through `hold_with`, holds a container labelled `sluice.place` with what that file says. `sluice preview --wait` reports `failed`, and the container is removed. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test previews_tools a_failed_previews_labelled_container_ends_with_it` passes.
- [ ] 4.5 Add `a_preview_on_a_host_with_no_engine_is_released_as_before` to `crates/sluice/tests/previews_tools.rs`, the spec's "A host with no container engine": `SLUICE_ENGINE_SOCKETS` naming a path with no socket, and `sluice release` exiting 0 with the checkout removed and its port free. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test previews_tools a_preview_on_a_host_with_no_engine_is_released_as_before` passes.
- [ ] 4.6 Add `a_container_the_engine_will_not_remove_keeps_its_preview` to `crates/sluice/tests/previews_tools.rs`, the spec's "The engine refuses a removal". `sluice release` exits 1 naming the container and the engine's words, `sluice previews --json` still lists the preview with its release failure, and its port stays held. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test previews_tools a_container_the_engine_will_not_remove_keeps_its_preview` passes.
- [ ] 4.7 Add `a_gone_previews_refused_container_is_tried_again_by_the_next_listing` to `crates/sluice/tests/previews_tools.rs`, the spec's "The engine removes it on a later try". The checkout is removed, and the engine refuses the first removal. The first `sluice previews` keeps the preview naming what failed. The second frees it. The release log shows one run. True when `sluice exec bolt/what-a-place-starts-ends-with-it -- cargo test -p sluice --features tools --test previews_tools a_gone_previews_refused_container_is_tried_again_by_the_next_listing` passes.

## 5. What a reader is told

- [ ] 5.1 In `site/src/content/docs/guides/previews.md`, where it gives `[tests] release` and `sluice release`, add three points:
  - However a preview is released or freed, sluice then removes every container that mounts the checkout or `SLUICE_TMP`, publishes one of its ports, or carries the label `sluice.place=$SLUICE_PLACE`.
  - The label is how to mark a container with neither a mount nor a port.
  - A container sluice cannot remove keeps the preview, and `sluice release` says which.

  Name no host or tool of ours (rule core.26). True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run site:prose` passes with no warning on the page.
- [ ] 5.2 Say the same, in the references' own words, in `plugins/sluice/skills/using-sluice/references/previews.md` and `plugins/sluice/skills/setting-up-sluice/references/profiles/previews.md`. In the second, keep that the release command must remove what the command handed off, and add that sluice removes what it left by those three marks. Propose no branch name (rule core.26). True when `sluice exec bolt/what-a-place-starts-ends-with-it -- moon run plugin-sluice:validate` passes.
