# checkout Specification

## Purpose

How a branch's policy decides the way a checkout is made for a
combination or a preview: there is one way, a git worktree (ADR 0013),
so the policy need not name it.

## Requirements

### Requirement: A policy need not name how a checkout is made

A checkout SHALL be made from a policy whose `[checkout]` table names no
`method`, or that has no `[checkout]` table, as a detached git worktree
at the revision tested, for a combination and for a preview alike.

#### Scenario: No method named

- **WHEN** a checkout is made from a `[checkout]` with no `method`
- **THEN** it holds the revision tested, and once released its
  directory is gone (`a_policy_need_not_name_the_method` in
  `crates/sluice/tests/profiles/places.rs`)

#### Scenario: A submission lands with no method named

- **WHEN** the walkthrough's policy, which names no `method`, gates a
  submission end to end over moon and git
- **THEN** the submission is tested in a checkout and lands
  (`e2e/e2e.sh`, run by `moon run ':#check-e2e'`)

### Requirement: An older policy's method is read as before

A policy that says `method = "git-worktree"`, as an older revision of a
branch or a previewed revision may, SHALL make a checkout exactly as one
that names no method (core.15).

#### Scenario: The one method named

- **WHEN** a checkout is made from a `[checkout]` with
  `method = "git-worktree"`
- **THEN** it holds the revision tested, and once released its
  directory is gone (`a_policy_need_not_name_the_method` in
  `crates/sluice/tests/profiles/places.rs`)

### Requirement: Any other method is refused

A `method` other than `"git-worktree"` SHALL be refused before anything
is made, with a message naming the method and saying a checkout is a git
worktree and `[checkout]` names no method (ADR 0013). A `method` that is
not a string SHALL be refused.

#### Scenario: Another method

- **WHEN** a checkout is asked for with `method` `"worktrunk"`,
  `"archive"` or `"rsync"`
- **THEN** it is refused and no checkout directory is made
  (`an_unknown_method_is_refused` in
  `crates/sluice/tests/profiles/places.rs`)

### Requirement: Only the places profile reads the method

sluice's core SHALL give `[checkout]` no meaning of its own: what a
round decides SHALL be the same whatever `method` the policy passes to
the places profile (core.2).

#### Scenario: The core ignores the method

- **WHEN** the same submissions are run under policies that differ only
  in their runner and `[checkout] method`, over the fakes
- **THEN** the submission ends in the same state and the same
  combinations are tested (`the_core_gives_the_profiles_a_policy_names_no_meaning_of_its_own` in
  `crates/sluice-conformance/tests/records.rs`)

### Requirement: A checkout starts with nothing but its revision

A checkout made for a combination, a preview or an agent's worktree
SHALL hold its revision and nothing restored from another checkout, and
its release SHALL save nothing of it. A checkout shares only the host's
caches, which its runs are given (35; record 0049 as amended).

#### Scenario: A passed combination's build is not kept

- **WHEN** a combination's checkout builds `target`, passes and is
  released, and a later combination's checkout is made from the same
  lock file
- **THEN** the later checkout has no `target`
  (`a_checkout_starts_with_nothing_but_its_revision` in
  `crates/sluice/tests/profiles/places.rs`)

#### Scenario: The saved caches go

- **WHEN** a checkout is made in a repository whose
  `.git/sluice/.caches/` holds caches an earlier sluice saved
- **THEN** that directory is gone, and nothing else under
  `.git/sluice/` is removed
  (`a_checkout_starts_with_nothing_but_its_revision`)

### Requirement: A policy that still declares the checkout cache is refused

A branch's policy whose `[checkout]` still declares a `cache`, as
`[[checkout.cache]]` tables or as a `cache` key of any other shape,
SHALL be refused when sluice reads it, before anything is made, tested
or landed under it, so a repository that relied on the checkout cache
learns it is gone rather than getting slower with no word (35, 71; ADR
0049 as amended). The refusal SHALL name ADR 0049 and the host's
`[[caches]]` in `~/.config/sluice/config.toml` that replace it, and say
to remove `[[checkout.cache]]` from `.config/sluice.toml`. Every command
and round that reads the policy SHALL show the same refusal.

#### Scenario: The policy is read

- **WHEN** a policy whose `[checkout]` holds a `[[checkout.cache]]`
  entry, or `cache = "target"`, is read
- **THEN** the read is refused, and its message names ADR 0049,
  `[[caches]]` and `~/.config/sluice/config.toml`
  (`refuses_a_checkout_cache` in `crates/sluice-core/src/policy.rs`)

#### Scenario: A branch's status

- **WHEN** `sluice status` is run for a branch whose policy holds a
  `[[checkout.cache]]` entry
- **THEN** it exits with an error and prints the refusal naming ADR
  0049 and the host's `[[caches]]`
  (`status_refuses_a_checkout_cache` in `crates/sluice/tests/cli.rs`)

### Requirement: A policy that declares no checkout cache reads as before

A policy whose `[checkout]` declares no `cache`, or that has no
`[checkout]` table, SHALL be read exactly as before: its `ports`,
`ports_from` and an older `method = "git-worktree"` reach the places
profile unchanged, and nothing is refused or warned of for the cache.

#### Scenario: Ports and the one method

- **WHEN** a policy whose `[checkout]` holds `method = "git-worktree"`,
  `ports` and `ports_from`, and no `cache`, is read
- **THEN** it is read with no error and no warning, and its `[checkout]`
  holds those three settings (`refuses_a_checkout_cache` in
  `crates/sluice-core/src/policy.rs`)
