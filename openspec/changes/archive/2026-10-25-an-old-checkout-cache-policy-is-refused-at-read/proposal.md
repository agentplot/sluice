# Proposal

## Why

This bolt takes sluice's checkout cache away: a checkout now starts with
nothing but its revision, and what a run builds is shared only through
the caches the host's keeper declares (35, 71; ADR 0049 as amended). A
repository whose policy still declares `[[checkout.cache]]` is today
read as if it declared nothing, so its checks quietly start cold and get
slower with no word to anyone. The design agent ruled, and the user
approved, that such a policy is refused when it is read instead, with a
message that says why and what replaces it.

## What Changes

- **BREAKING**: a branch's policy whose `[checkout]` still declares a
  `cache` (the `[[checkout.cache]]` tables, or a `cache` key of any
  shape) is refused when sluice reads it. The refusal names ADR 0049,
  says a checkout starts with nothing but its revision, and points to
  the host's `[[caches]]` in `~/.config/sluice/config.toml` as what
  replaces it, and to removing `[[checkout.cache]]` from
  `.config/sluice.toml`. Everything that reads the policy (`sluice
  status`, `sluice submit`, a round, `sluice exec`, a preview) shows the
  same refusal.
- A policy that declares no checkout cache reads exactly as before,
  including an older `method = "git-worktree"` and `ports`.
- The places profile no longer has to read a `[[checkout.cache]]`
  quietly, since one never reaches it; its comments and its test of
  that go.
- The configuration page says a `[[checkout.cache]]` is refused, and
  what to do, in place of saying it is read as nothing.

Out of scope, waiting on no later change of this bolt: removing
`[[checkout.cache]]` from the policies of the other repositories that
still declare it. A branch whose own revision declares it cannot land
the removal through a sluice that refuses it, so each such repository
removes the key before its host runs this sluice (see the design's
risks). That is work in those repositories, not here.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `checkout`: the requirement that an older policy's checkout cache is
  read as nothing is replaced by one that refuses it at policy read,
  naming ADR 0049 and the host's caches, while a policy without one
  reads as before.

## Impact

- `crates/sluice-core/src/policy.rs`: `Policy::parse` refuses a
  `[checkout]` holding `cache`, beside the other retired policy words it
  already handles.
- `crates/sluice/src/profiles/places.rs` and its tests: the quiet read
  of `[[checkout.cache]]` goes from the comments and the tests.
- `crates/sluice/tests/cli.rs`: `sluice status` on such a branch shows
  the refusal.
- `site/src/content/docs/reference/configuration.md`: the `[checkout]`
  section.
- Repositories still declaring `[[checkout.cache]]` stop being read by
  sluice until they remove it.

## Touches

```
crates/sluice-core/src/policy.rs
crates/sluice/src/profiles/places.rs
crates/sluice/tests/profiles/places.rs
crates/sluice/tests/cli.rs
site/src/content/docs/reference/configuration.md
```
