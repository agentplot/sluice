# Tasks

## 1. The policy refuses a checkout cache

- [x] 1.1 In `crates/sluice-core/src/policy.rs`, make `Policy::parse` refuse a `[checkout]` table that holds the key `cache`, whatever its shape, before it builds the table's `Settings` (design D1). The message names ADR 0049, the host's `[[caches]]` in `~/.config/sluice/config.toml`, and says to remove `[[checkout.cache]]` from `.config/sluice.toml` (design D2). A comment at the refusal cites 35, 71 and ADR 0049 and says why the core reads this one word of `[checkout]`: the checkout cache was the statement's, not a profile's. Leave `trust = ["cache"]` as it is. Verify with a new unit test, `refuses_a_checkout_cache`: a `[[checkout.cache]]` with `key` and `paths`, and `cache = "target"`, each refused with those names in the message; a `[checkout]` with `method = "git-worktree"`, `ports` and `ports_from` and no `cache` read with no error and no warning, holding those three. Run it by `sluice exec main -- cargo test -p sluice-core --lib policy`.
- [x] 1.2 Add `status_refuses_a_checkout_cache` to `crates/sluice/tests/cli.rs`, modelled on `status_warns_of_cache_in_trust`: the branch's policy is the fixture's `POLICY` with a `[[checkout.cache]]` entry added; `sluice status <branch>` exits non-zero and prints the refusal, naming ADR 0049 and `[[caches]]`. Run it by `sluice exec main -- cargo test -p sluice --test cli checkout_cache`.

## 2. The places profile stops reading it quietly

- [x] 2.1 In `crates/sluice/src/profiles/places.rs`, change the module comment and the comment on the record's old `caches` field so neither says a `[[checkout.cache]]` is read as nothing; say instead that the policy refuses one (ADR 0049). In `crates/sluice/tests/profiles/places.rs`, remove `an_older_policys_checkout_cache_is_read_as_nothing` and its `cached()` helper (design D3). Verify with `sluice exec main -- cargo test -p sluice --test places` passing and `git grep -n 'read as nothing' -- crates` finding nothing about the checkout cache.

## 3. The configuration page

- [x] 3.1 In `site/src/content/docs/reference/configuration.md`, the `[checkout]` section: replace the sentence that a cache table an older revision declares is read as nothing with one saying a `[[checkout.cache]]` is refused when the policy is read, naming [ADR 0049](/sluice/decisions/0049/) and linking the host's [`[[caches]]`](#caches), and telling the reader to remove it. Verify with `moon run site:prose` clean.

## 4. The gate

- [x] 4.1 Commit, then run the gate's checks with `sluice exec main`; every task passes. The tests that parse sluice's own `.config/sluice.toml` and the setting-up-sluice examples pass with no change, which shows none of them declares a checkout cache.
