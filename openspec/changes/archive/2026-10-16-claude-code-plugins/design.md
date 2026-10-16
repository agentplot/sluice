# Design

## Context

ADR 0023 decides the parts this change rests on: the plugins live under
`plugins/`, outside the core, versioned with the commands they describe;
they read the queue only through `sluice overview` (`sluice/overview@1`,
`--json --watch`); a submission's `from.path` ties it to its worktree. The
plan behind this change ranked the ideas and chose this scope.

## Decisions

### D1. Two plugins, one marketplace

`plugins/sluice` holds skills and hooks written as plain commands, which
work on any Claude Code. `plugins/sluice-board` holds the mod, whose API is
early access: a mod that fails to load must not take the skills with it.

### D2. Skills point at the site

Each skill is short and points at the site's reference pages for depth. A
setup reference per profile has the same parts: when to choose it, its
keys, a complete example, how to check it, its traps. A new profile is one
file and one row of the setup skill's table.

### D3. The binary is installed by the setup skill, asking first

nix where present, the release tarball for the platform, else `cargo
install`. A sluice from the user's own flake or Home Manager is theirs to
update; the skill says so and changes nothing.

### D4. The guard reads protected branches from sluice

The hook acts only on a Bash command that pushes, merges, updates a ref or
merges a pull request. Then it asks `sluice overview --json` for the
branches with a queue and the policy for the branches it protects, and
refuses a command aimed at one, naming `sluice submit`. Elsewhere it costs
one pattern match. `sluice protect` already refuses local updates; the
guard explains before git refuses, and covers pushes to a remote and
`gh pr merge`, which the local hook cannot see.

### D5. The board reads one stream

On session start, in a repository with records, the mod runs `sluice
overview --json --watch` for the session's life and keeps the last document
in the session's state; new events are found by comparing `recent`. It falls
back to polling `sluice overview --json`. The wake matches the session's
submissions on `from.path` against the session's worktree root.

## Risks / Trade-offs

- The mod API moves between Claude Code releases; the board is versioned on
  its own and may need updates the skills do not.
- Skills restate some of the site; the example-policy test catches drift in
  the examples, and the drift tests in the tasks cover keys and commands.
