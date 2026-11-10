---
number: 105
title: A sandboxed run writes the store its check profile reads, and a pass it cannot prove has no outcome
status: accepted
date: 2026-11-09
decision-makers:
- the design agent
---

# A sandboxed run writes the store its check profile reads, and a pass it cannot prove has no outcome

Status: accepted on 2026-11-09 by agentplot-design. It is the design agent's own choice, made on the statement and the rules written, and cheap to change. Amended 2026-11-09 by the construct stage of the change `a-sandboxed-run-writes-its-runners-store`, its own choice under the C.2 sandbox row, which grants the store a run's environment resolves with no exception for a cold run: a cold run whose repository's own moon workspace shares moon's cache is granted the store that workspace puts outside its checkout. agentplot-design took the amendment as its own that day.

Deciders: agentplot-design, on the question of the fix place `fix--sluice-runs-one-host-process--passes-mains-checks-954d21e`, 2026-11-09: "Needed: one family design for where moon's shared worktree cache lives in a run. Either the host stops declaring MOON_CACHE_SHARED_WORKTREE_CACHE, or the repositories stop moving MOON_HOME [...], or sluice equips moon's store as a declared cache. Also needed: whether a run whose every check passed but whose manifests moon couldn't store should blame the submission for every check."

Sources: the statement's clauses 108, 114, 115 and 139 and its C.2 moon and sandbox rows, as amended at agentplot/blueprints 0c38f24; rules core.9, core.24 and core.35; record 0072; `crates/sluice/src/profiles/moon.rs` `store`; this repository's `devenv.nix` (`MOON_HOME`) and the configuration reference's `[[caches]]`; the place's reading: submission 1 on `bolt/sluice-runs-one-host-process`, run c11, after #51 named `[signers.sessions]`, every one of its 28 tasks passed, moon said "Failed to store blobs, will skip caching the manifest storage=shared-local-cache", and every check was blamed.

## Context and problem statement

A signed run counts a key only where moon's own manifest proves the key ran, read where moon keeps it, as moon resolves its settings from the run's environment (core.9). With moon's shared worktree setting on, which the host's `[[caches]]` turns on, moon keeps its manifests in `cache/shared` under its home. Each repository of the family moves moon's home into its git common directory (`MOON_HOME=<common>/moon-home`, in `devenv.nix`), so that no moon of the person's own, in `~/.moon/bin`, runs in a task (core.24). The host declares `~/.moon/cache/shared` as the cache a sandboxed run may write, which is not where moon writes. The sandbox refused moon's write, moon kept no manifest, and sluice read every passing check as failed and blamed the submission for it. The cause was said only in a tracing warning. Every family repository meets this once its branch is signed.

## Considered options

1. **sluice lets a sandboxed run write the store its check profile resolves (chosen).** The store moon writes in a run is the store sluice reads the manifests from (core.9), resolved the same way from the run's environment.
2. **The host stops turning moon's shared worktree cache on.** Each place would build moon's cache alone, and the configuration reference's own advice to share moon's cache that way would no longer hold.
3. **The repositories stop moving moon's home.** A moon of the person's own in `~/.moon/bin` would then be on every task's path, which core.24 rules out, on any host where a person installed moon by moon's own installer.

## Decision outcome

1. **A run writes where its check profile reads.** Under the sandbox, a run may write the task runner's shared store as the check profile resolves it from that run's environment: for moon, the store `moon::store` names, its base checkout's `.moon/cache` or `cache/shared` under moon's home. It is granted beside the place, its scratch and the host's declared caches (C.2 sandbox row). The host's `[[caches]]` still turns sharing on and shares caches no check profile resolves, such as a compile server's. A cold run gets none of the host's caches, so its store is its own checkout's, and nothing more is granted, unless the repository's own moon workspace turns sharing on (amended 2026-11-09): the store resolved from the cold run's environment is then granted as any run's is, since the sandbox refuses no read of it and refusing the write would only leave the run's keys unproven. Not taken: options 2 and 3, above.
2. **A pass the run cannot prove has no outcome.** Where a check passed and the run kept nothing that proves it ran, moon's manifest of it, the check has no outcome (114). It is recorded under no key, nobody is blamed, and the check runs again, as for a run refused something (core.35, 115). The record says why, from the fact itself, the manifest missing where the profile reads it, never from moon's words. The submission's status and a wait say it, and the branch's log keeps it once per cause and host, as a standing refusal is kept (core.28). Not taken: blaming the submission for every check whose pass could not be proven, which blames work for what the host did, with the cause said nowhere a reader looks.

## Consequences

- Rules core.35 and core.9 carry both points.
- The units `a-sandboxed-run-writes-its-runners-store` and `a-pass-nothing-proves-blames-no-one` build them.
- A signed branch of any family repository passes its gate under the sandbox once a sluice with point 1 runs its queue. Until then, nothing a repository commits makes its signed runs pass.
