# Design

## Context

Written after the fact, from commit `6b97938`. The decision is ADR 0013
(a place is a plain git worktree), whose consequence this change
rewrote; it is not restated here.

## Decisions

### D1. One check, in the places profile

`one_method(checkout)` in `crates/sluice/src/profiles/places.rs`
accepts no `method` or `method = "git-worktree"`, refuses another
string with the reason and a non-string as before. `materialize` and
`preview` call it before anything is made, replacing the two copies of
the old check. The core reads nothing of `[checkout]` (core.2): the
conformance test `the_core_gives_the_profiles_a_policy_names_no_meaning_of_its_own`
still passes a method through to show it.

### D2. A place's record still names its method

`Made::new` is given the constant `METHOD` (`"git-worktree"`), not the
policy's value, so every record says what made it and `release` still
refuses a record made by a method this sluice cannot remove.

### D3. Why the old key is still read

A gate is read from the branch's current revision and a preview's
`[checkout]` from the previewed revision (core.15, ADR 0021). Refusing
the key would fail every branch, and every preview of a revision, whose
policy predates this change.

## Risks / Trade-offs

- [A policy keeps the dead key forever] → harmless; the configuration
  reference says it is no longer needed.
