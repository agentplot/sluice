# Spec Delta

## ADDED Requirements

### Requirement: A claimed run's checkout is made at its combination

A host SHALL make a claimed run's checkout at its combination however
the claim reached it: a round's own start, a claim it read after its
round fetched the branch's combinations, or a pool's job. Where the host
lacks the combination, it SHALL first fetch the combination's own ref,
`refs/sluice/combinations/<branch>/<id>`, from the branch's remote, and
no other; a host that holds it SHALL fetch nothing for it (82, C.6;
record 0117).

#### Scenario: A pool's job on a machine that lacks the combination

- **WHEN** the lead launches a run of `bolt/x` on the forge's pool, and
  the pool's machine starts the claimed run holding none of its
  combination
- **THEN** the machine fetches the combination by its own ref, makes the
  checkout at it, and starts its tests
  (`a_pools_job_makes_its_place_at_a_combination_only_the_remote_holds`
  in `crates/sluice-conformance/tests/could_not_start.rs`)

#### Scenario: A combination only the remote holds

- **WHEN** host B opened combinations `c7` and `c8` of `main`, and host
  A, holding neither, fetches `c7`
- **THEN** host A holds `c7` alone, and a checkout made at it holds it
  (`a_combination_only_the_remote_holds_is_fetched_and_a_place_made_at_it`
  in `crates/sluice/tests/remote_refs.rs`)

#### Scenario: A host that holds the combination

- **WHEN** host `a` claims a run of a combination its round fetched
- **THEN** it starts the run's tests with no fetch of the combination
  of its own
  (`a_host_that_holds_the_combination_fetches_nothing_more` in
  `crates/sluice-conformance/tests/could_not_start.rs`)
