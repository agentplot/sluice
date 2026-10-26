# Spec Delta

## ADDED Requirements

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
