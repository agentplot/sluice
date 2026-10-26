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

### Requirement: An older policy's checkout cache is read as nothing

A policy whose `[checkout]` still declares `[[checkout.cache]]`, as an
older revision of a branch or a previewed revision may, SHALL make a
checkout exactly as one that declares none, with no error, so a branch
whose own revision holds it is still tested and lands (core.15).

#### Scenario: A cache table in the policy

- **WHEN** a checkout is made from a `[checkout]` with a
  `[[checkout.cache]]` entry
- **THEN** it holds the revision tested, nothing is restored into it,
  and once released nothing is saved
  (`an_older_policys_checkout_cache_is_read_as_nothing` in
  `crates/sluice/tests/profiles/places.rs`)
