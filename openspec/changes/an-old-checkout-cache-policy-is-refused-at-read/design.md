# Design

## Context

Every reader of a branch's policy goes through `Policy::parse` in
`crates/sluice-core/src/policy.rs`: `Sluice::policy_at` in the core
(status, submit, a round's steps, a landing, previews, `sluice exec`)
and `Branches::policy_of` and `named` in the branches profile (routing a
branch to its records and remote). A parse error propagates with `?`
from each, wrapped as `<branch>'s policy at <rev>`, so a refusal there
is shown by every command and round that reads the policy, with nothing
read further.

`[checkout]` reaches the core as `Settings`, which it passes to the
places profile without reading (core.1, core.2; the checkout spec's
"Only the places profile reads the method"). Since this bolt's
`a-run-gets-the-hosts-shared-caches` (its design D6), the places profile
reads a `cache` in `[checkout]` as nothing, citing core.15: refusing it
would refuse the policy a branch's own revision holds. The design agent
has since ruled the opposite for the checkout cache (clause 35 with
record 0049; proposal 91, approved by the user): silence makes such a
repository slower with no word, and a refusal names what replaced it.

The policy already handles one retired word itself: `trust = ["cache"]`
is read as absent with a warning (108), on the same core.15 reasoning.
That stays as it is; this change is about `[checkout] cache` only.

## Goals / Non-Goals

**Goals:**

- A policy holding `[checkout] cache` in any shape fails to read, with
  one message that says why and what to do.
- A policy holding no `cache` reads byte-for-byte as before.

**Non-Goals:**

- Changing how `trust = ["cache"]` or `method = "git-worktree"` are read.
- Any path that lets a refused branch land a fix to its own policy
  (see Risks).
- Removing `[[checkout.cache]]` from other repositories' policies.

## Decisions

### D1. The refusal is in `Policy::parse`, not in the places profile

`Policy::parse` refuses when the `[checkout]` table it has just read
holds the key `cache`, before building `Settings`, so every reader of
the policy refuses it the same way and nothing is made first.

Why here and not in the places profile, beside `one_method`: the
profile reads its settings only at `materialize` and `preview`, so a
submission would be taken, a round started and only the checkout
refused, which is later than "at policy read" and leaves `sluice
status` and `sluice submit` quiet. And this is no profile's setting the
core is interpreting (core.2): the checkout cache was the statement's
own (clause 35 before record 0049's last amendment), retired by the
statement, as `cache` in `trust` was. The core refuses a word the
statement no longer has; it still gives the settings `[checkout]` does
hold no meaning, and the places profile still reads `method`, `ports`
and `ports_from` alone. The conformance test that the core gives a
profile's settings no meaning of its own is untouched.

Considered: refusing in the places profile at `materialize` (too late,
as above); a warning in `Policy::warnings`, as `trust = ["cache"]`
gets (the ruling asks for a refusal, and a warning in `sluice status`
is what nobody reads).

### D2. The message

One `bail!` from `Policy::parse`, in the policy's own vocabulary and
the user words, for example:

```
[checkout] declares a cache, which sluice no longer keeps: a checkout starts
with nothing but its revision, and the caches its tests share are the host's
[[caches]], which the host's keeper declares in ~/.config/sluice/config.toml
(ADR 0049). Remove [[checkout.cache]] from .config/sluice.toml.
```

Readers already add `<branch>'s policy at <rev>` around it. The
exact wording is the coder's, holding: it names ADR 0049, `[[caches]]`,
`~/.config/sluice/config.toml` and the removal.

### D3. What goes from the places profile

With `cache` never reaching it, the places profile's module comment
and the comment on the record's old `caches` field stop saying a
`[[checkout.cache]]` is read quietly. The test
`an_older_policys_checkout_cache_is_read_as_nothing` and its `cached()`
helper go: the case it tested can no longer reach the profile, and the
refusal is tested where it happens. Removing what an earlier sluice
saved under `.git/sluice/.caches/` stays as it is.

### D4. Tests

- `refuses_a_checkout_cache` in `policy.rs`: `[[checkout.cache]]` with
  `key` and `paths`, and `cache = "target"`, each refused, the message
  naming ADR 0049, `[[caches]]` and `~/.config/sluice/config.toml`; and
  a `[checkout]` with `method = "git-worktree"`, `ports` and
  `ports_from` read with no error and no warning, holding those three.
- `status_refuses_a_checkout_cache` in `tests/cli.rs`, modelled on
  `status_warns_of_cache_in_trust`: a branch whose policy is the
  fixture's `POLICY` with a `[[checkout.cache]]` added; `sluice status`
  exits non-zero and prints the refusal.
- The existing tests that parse sluice's own `.config/sluice.toml` and
  the setting-up-sluice examples still pass, which shows none of them
  declares a checkout cache.

## Risks / Trade-offs

- [A branch whose own revision declares `[[checkout.cache]]` cannot land
  the removal through a sluice that refuses it: every round reads that
  branch's policy first, and branch protection keeps every other hand
  off the branch.] → Each repository removes the key before its host
  runs this sluice. On this host, tailrace, flywheel-next and
  flywheel-web still declare one on their `main`, and so does sluice's
  own `main` until this bolt lands there; the bolt must land on
  sluice's `main` through a sluice that does not yet refuse it. Named
  for the planner in the proposal's out-of-scope note.
- [A preview of an older landed revision that still declares the cache
  is refused.] → Accepted: the preview's command, ports and release are
  read from that revision (core.19), and the refusal says what to
  change.
- [`trust = ["cache"]` is warned of while `[checkout] cache` is
  refused, two treatments of retired words.] → Accepted: the ruling
  covers the checkout cache, whose silent loss slows every run; a
  replay no longer counting changes no speed.
