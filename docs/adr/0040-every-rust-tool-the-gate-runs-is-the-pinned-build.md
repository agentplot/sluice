---
number: 40
title: Every Rust tool the gate runs is the pinned build
status: accepted
date: 2026-10-21
decision-makers:
- Chuck Swanberg
---

# Every Rust tool the gate runs is the pinned build

Deciders: the operator, after CI and the gate disagreed on main, 2026-10-21.
Sources: rules core.21, core.24; ADR 0029.

## Context and problem statement

`.prototools` pins Rust, and the devenv shell puts that toolchain's directory first on `PATH`. rustup installs a toolchain with the components its profile names. On a Mac with rustup's default profile the pinned toolchain holds clippy and rustfmt. On a GitHub runner, whose profile is minimal, it holds rustc, cargo and the standard library only. There `cargo clippy` and `rustfmt` were found further along `PATH`, as rustup's proxies, which ran the runner's default toolchain, 1.99.0, against a tree pinned to 1.98.1. A lint new in 1.99 failed every CI run on main while `sluice exec main`, on 1.98.1, passed the same tree. Nothing checked which build ran. How does every host run the build the tree pins?

## Considered options

1. A `rust-toolchain.toml` naming the pinned channel and the components the gate runs, and a root check-fast task, `root:toolchain`, that fails unless its channel is `.prototools`' and rustc, cargo, clippy and rustfmt each report that build.
2. moon's Rust toolchain settings: `components`, with `syncToolchainConfig` writing the channel.
3. Installing the components in the shell's `sluice:proto:install` task.

## Decision outcome

Option 1. With the toolchain file, every rustup proxy in the tree resolves to the pinned toolchain and installs a missing component of it, so no tool falls through to a host's default. The check makes the guarantee visible in the gate: a pin the two files disagree on, or a tool from another build, fails `root:toolchain` on the host where it happens.

Option 2 runs `rustup component add` with no toolchain named, which adds to the host's default, and its sync writes a tracked file during a run, which leaves a session's worktree beyond its head (core.18). Option 3 covers a devenv shell but not a rustup proxy reached any other way, and checks nothing.

### Consequences

- `.prototools` stays the pin moon hashes Rust tasks by; `rust-toolchain.toml` repeats its channel and `root:toolchain` holds them equal.
- A host whose pinned toolchain lacks a component downloads it the first time a tool needs it.
- A test still meets the host's platform: the gate runs on macOS, CI's deeper tiers on Linux. A test written for one platform's behaviour, such as the ports a host hands out, fails on the other, and pins can't make them one.
