# Checkout: ports, caches and the release command

Every combination is tested in a clean checkout of its own: `git worktree
add --detach` at the combination, removed when its result is read. Nothing
runs in it but the tests: no git or worktrunk hooks. There is one way to make
a checkout, so no key names it (an older `method = "git-worktree"` is still
read).

## Ports

```toml
[checkout]
ports = ["web", "db"]   # SLUICE_PORT_WEB, SLUICE_PORT_DB, free and held for the checkout
ports_from = 20000      # where blocks of ports begin
```

Use them when tests listen on fixed ports. An agent's worktree gets its ports
on its first `sluice exec` and keeps them until `sluice release`.

## Caches

A checkout starts from its revision alone and saves nothing when it is
released; the policy declares no cache. A cache every build on the host
shares, a compile server's or moon's shared worktree cache, is the host's:
declare it in the host's config, which whoever keeps the host writes.

```toml
# ~/.config/sluice/config.toml
[[caches]]
dir = "~/.cache/sccache"                  # outside every checkout
socket = "~/.cache/sccache/server.sock"   # the cache's server, if it has one
env = { RUSTC_WRAPPER = "sccache", SCCACHE_DIR = "/Users/me/.cache/sccache" }

[[caches]]
dir = "~/.moon/cache/shared"
env = { MOON_CACHE_SHARED_WORKTREE_CACHE = "true", MOON_EXPERIMENT_CAS_OUTPUTS_CACHE = "true" }
```

Every run on the host gets each `env`, written as the agents' shells have
it, and the sandbox opens each `dir` and `socket`. sluice starts no cache's
server. A cache table an older policy's `[checkout]` still holds is read as
nothing.

## Services and the release command

sluice starts no services. The tests bring up what they need (moon `wait` and
`cleanup` dependencies, testcontainers, Playwright's `webServer`, a pytest
fixture) and tear it down. What escapes the process group, a container above
all, is removed by the release command:

```toml
[tests]
release = "docker compose -p \"$SLUICE_PLACE\" down -v"

[executor]
docker = true
```

The release command runs once when a checkout is released, under the same
executor, toolchain and environment as the tests, ports included, and in an
agent's worktree on `sluice release`. Label what tests start with
`SLUICE_PLACE` so the release command finds exactly it.

Reference: <https://agentplot.github.io/sluice/reference/configuration/#checkout-where-a-combination-is-tested>.
