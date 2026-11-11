## MODIFIED Requirements

### Requirement: Why is read from the missing proof itself

The reason SHALL be read from the fact itself, the task runner's record
missing where the check profile reads it, and from what the store that
should hold it says of itself. It SHALL name where the record was looked
for and whose record is missing, never what the task runner printed
(record 0105; rule core.35). For moon, the record is moon's manifest of
the test's key and of each key it depends on (C.2 moon row). The reason
SHALL read `it passed, but nothing the run kept proves it ran: moon's
manifest of <whose> is not in <store>, <fact>`. `<whose>` is `it`, or
`<task>, which it depends on`. `<store>` is the store the run's record
names. `<fact>` is the first that holds of: `which does not exist`;
`which <user> cannot write`; `a directory of which <user> cannot
write`; `where moon kept none of the run's manifests`; `where moon kept
others of the run's manifests`. `<user>` is the user sluice runs as.
The facts are read from the store itself after the run, with nothing
run (114).

#### Scenario: A manifest gone from the store

- **WHEN** a moon run passes `alpha:test` and `beta:test`, and
  `alpha:test`'s manifest is in neither the store the run's record
  names nor the checkout's own
- **THEN** `beta:test` is proven, and `alpha:test` is among the passes
  with no proof, its key that of `alpha:test`, saying
  `it passed, but nothing the run kept proves it ran: moon's manifest of
  it is not in <store>, where moon kept others of the run's manifests`
  (`a_pass_whose_manifest_is_gone_is_unkept_naming_the_store` in
  `crates/sluice/tests/profiles/moon.rs`)

#### Scenario: What the store says of itself

- **WHEN** a manifest is missing from a store that does not exist; from
  one whose `blobs` its user cannot write; from one whose directory for
  the key its user cannot write; from one holding none of the run's
  manifests; and from one holding others of them
- **THEN** the fact read is, in turn, `which does not exist`, `which
  <user> cannot write`, `a directory of which <user> cannot write`,
  `where moon kept none of the run's manifests` and `where moon kept
  others of the run's manifests`
  (`the_store_says_why_a_manifest_is_missing` in
  `crates/sluice/src/profiles/moon.rs`)

#### Scenario: A dependency's manifest missing

- **WHEN** a passing task's own manifest is in the store and the
  manifest of a task it depends on is not
- **THEN** the task's proof is read as not kept, not as a key the tree
  does not bear out, naming the task whose manifest is missing and its
  key
  (`a_missing_manifest_is_told_from_one_the_tree_does_not_bear_out` in
  `crates/sluice/src/profiles/moon.rs`)

## ADDED Requirements

### Requirement: A manifest is read where the run's moon kept it

The check profile SHALL read a key's manifest, and each manifest the
key depends on, in the store the run's record names. Where that store
is not the checkout's own `.moon/cache` and lacks a manifest, it SHALL
read the checkout's own store, before the checkout is released. A
manifest read from either SHALL be held to the same proof: it hashes to
its key, names its task, and names inputs the combination's tree holds
(C.2 moon row; rule core.9).

#### Scenario: moon kept the manifest in the checkout

- **WHEN** a moon run passes `alpha:test`, the run's record names a
  shared store, and `alpha:test`'s manifest is in the checkout's
  `.moon/cache` and not in the shared store
- **THEN** `alpha:test` is proven
  (`a_manifest_moon_kept_in_the_checkout_proves_its_key` in
  `crates/sluice/tests/profiles/moon.rs`)

#### Scenario: Neither store holds it

- **WHEN** neither store holds `alpha:test`'s manifest
- **THEN** `alpha:test` is among the passes with no proof, naming the
  store the run's record names
  (`a_manifest_moon_kept_in_the_checkout_proves_its_key`)

### Requirement: moon's dependency marks are no key

Where a manifest names a dependency `ignored`, `passthrough`, `outputs`
or `outputs-passthrough`, the marks moon 2.6.1 writes for a dependency
that gives the task no key, the check profile SHALL walk nothing for it
and SHALL NOT read the key as one the tree does not bear out. Any other
value that is not a key SHALL be refused as before (C.2 moon row, C.6).

#### Scenario: Each mark

- **WHEN** `b:test`'s manifest names `a:test` as a key, and `c:lint`,
  `d:gen`, `e:build` and `f:setup` as `ignored`, `passthrough`,
  `outputs` and `outputs-passthrough`
- **THEN** `b:test`'s key is borne out, walking `a:test` alone; and a
  dependency named `stale` is refused as no key
  (`moons_dependency_marks_are_no_key` in
  `crates/sluice/src/profiles/moon.rs`)

### Requirement: A task moon does not cache is proven

A task that moon runs and does not cache, `cache: false`, and each task
depending on it SHALL be proven by their manifests as any task is, on a
run that shares moon's cache in a linked worktree (C.2 moon row).

#### Scenario: An install and a check depending on it

- **WHEN** a run in a linked worktree with moon's shared worktree cache
  on runs `root:install`, with `cache: false` and no outputs, and
  `alpha:test`, which depends on it, with `--force`
- **THEN** both manifests are in the store the run's record names,
  `alpha:test`'s naming `root:install` as `ignored`, and both are proven
  (`a_task_moon_does_not_cache_and_its_dependent_are_proven` in
  `crates/sluice/tests/profiles/moon.rs`)

### Requirement: Each test left without proof names the missing key

A test that passed in a run that kept no proof of it SHALL be listed
not run with the key of the record that is missing, its own or, as its
reason names, a task's it depends on. The `lost` line and the JSON
under `lost` SHALL carry it as `key`. The reason SHALL carry no key, so
that its cause is logged once for each host and reason (record 0105;
rule core.28).

#### Scenario: The lost line

- **WHEN** a signed run passes `a:test` and `b:test` and keeps no proof
  of either
- **THEN** its `lost` line lists each with its key, the key the run
  gave it, and the log holds one `proof-missing` line
  (`a_lost_line_names_each_unkept_key` in
  `crates/sluice-conformance/tests/unkept_proof.rs`)

### Requirement: A key the tree does not bear out is blamed with why

Where a check's key is not borne out by the combination's tree for any
reason but a missing manifest, the submission blamed for it SHALL name
the check failed under that key with the check profile's words for
why, as `unsigned` in its record. A waiting submit and `sluice wait`
SHALL say, under the failed line,
`<task>: the combination's tree does not bear its key out: <why>`
(C.6, 20).

#### Scenario: A forged key

- **WHEN** a signed run passes `a:test` under a key whose record names
  an input the tree does not hold
- **THEN** the submission is blamed on `a:test` under its key, and its
  record says why under `unsigned`
  (`a_key_the_tree_does_not_bear_out_is_blamed_with_why` in
  `crates/sluice-conformance/tests/unkept_proof.rs`)

#### Scenario: The words

- **WHEN** `a:test` is blamed with the why `a:test: a/x is not in the
  tree`
- **THEN** the line reads `a:test: the combination's tree does not bear
  its key out: a:test: a/x is not in the tree`
  (`a_key_not_borne_out_is_said_with_why` in
  `crates/sluice/src/words.rs`)

#### Scenario: The check profile says why

- **WHEN** a moon run passes `alpha:test` under a key whose manifest
  names an input the tree does not hold
- **THEN** `proven` leaves `alpha:test` out and answers it among the
  refused with its key and the words `check_key` gave
  (`proven_drops_a_key_the_tree_does_not_bear_out` in
  `crates/sluice/tests/profiles/moon.rs`)
