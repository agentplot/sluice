# Proposal

Written after the fact: the change landed on main on 2026-10-16 as
submission #68 (commit `6b97938`) without an OpenSpec change. This
records what was built, read from that commit. No capability in
`openspec/specs/` covered how a checkout is made, so this adds a small
one, `checkout`.

## Why

ADR 0013 chose one way to make a checkout, a plain git worktree, and no
setting for it; yet a policy that named no `[checkout] method` was
refused with "[checkout] names no method", so every policy had to carry
a key with one possible value. The key cannot simply be refused now:
a branch's gate is read from its current revision, and a preview's
`[checkout]` from the revision it previews (core.15, ADR 0021), so an
older revision that still says `method = "git-worktree"` must keep
working.

## What Changes

- `[checkout]` names no method. A policy without `method` makes a
  checkout, for a combination and for a preview alike.
- A policy that says `method = "git-worktree"` is read as before.
- Any other `method` is refused before anything is made, with the
  reason: `[checkout] method = "<m>": a place is a git worktree, and
  [checkout] names no method (ADR 0013)`; a `method` that is not a
  string is refused as before.
- The key leaves the repository's own policy (`.config/sluice.toml`),
  the README, `docs/examples/`, every test fixture, the conformance
  policy, `e2e/e2e.sh`, and the site's configuration reference, home
  page, walkthrough and previews guide. The configuration reference
  says why no key names the method and that an older one is still
  read. ADR 0013's consequence says the same.

Out of scope: removing the reading of `method = "git-worktree"`, which
waits until no branch's current revision and no previewed revision
still names it; there is no change planned for it. A second way of
making a checkout (a container place, C.1) is not proposed and would
bring the key back under a new ADR (core.3).

## Capabilities

### New Capabilities

- `checkout`: how a policy's `[checkout]` table chooses the way a
  checkout is made: it need not, the one way is read when named, and
  any other is refused.

### Modified Capabilities

None.

## Impact

- `crates/sluice/src/profiles/places.rs`: `one_method` and `METHOD`,
  used by `materialize` and `preview`.
- `crates/sluice-core/src/policy.rs`: a test fixture.
- `crates/sluice-conformance/src/lib.rs`: the shared policy has an empty
  `[checkout]`; `tests/records.rs` adds a method only to show the core
  gives it no meaning.
- Fixtures in `crates/sluice/tests/`, `e2e/e2e.sh`, `.config/sluice.toml`,
  `README.md`, `docs/examples/`, `docs/plans/phase-1-integrations.md`,
  `docs/adr/0013-…`, and `site/src/content/docs/`.
