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

```toml
[[checkout.cache]]
key = ["Cargo.lock", ".prototools"]   # files whose content keys the cache
paths = ["target"]                    # ignored paths it holds

[[checkout.cache]]
key = ["bun.lock"]
paths = ["node_modules"]
```

A checkout starts with the cache of its key, or the one saved last; a
checkout whose combination passed saves its paths under its key. sluice's
own repository: a two-and-a-half-minute cold test build becomes under a
second.

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
