# Spec Delta

## ADDED Requirements

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

## REMOVED Requirements

### Requirement: An older policy's checkout cache is read as nothing

**Reason**: Read as nothing, a policy that relied on the checkout cache
gave its checks cold checkouts with no word to anyone. Such a policy is
now refused when it is read, naming ADR 0049 and the host's caches.

**Migration**: Remove `[[checkout.cache]]` from `.config/sluice.toml`,
and have the host's keeper declare the caches the builds share as
`[[caches]]` in `~/.config/sluice/config.toml`.
