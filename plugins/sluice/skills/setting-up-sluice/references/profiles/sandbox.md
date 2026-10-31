# Executor: open or sandbox

**Choose `sandbox`** on macOS when the gate should hold tests to their
checkout: they may write only the checkout, their scratch directory and the
host's declared caches, and reach only the network the policy allows
(Seatbelt). **Choose `open`** on
Linux today, or when tests need what a sandbox refuses. `container` is
designed and not built.

## Keys

```toml
[executor]
kind = "sandbox"
network = "localhost"   # false: none; "localhost": 127.0.0.1 and ::1; true: anywhere
memory = "8G"           # refused where the executor cannot hold it
docker = true           # tests may reach the host's container engine

[branches.main]
trust = ["confined", "sandbox"]   # which results satisfy the branch
```

- Every result records which executor ran it and what it held. A branch's
  `trust` says which it accepts: `"confined"` (no agent could reach the
  run), an executor kind, and `"cache"` for moon replays.
- A branch with no `trust` accepts every run sluice starts and no replay.
- `docker = true` means a container may mount any path, so such a run never
  counts as confined. A branch that trusts only `"confined"` then starts
  nothing and says why: trust `"sandbox"` too.
- `docker = true` also asks the host for a container engine that answers, as
  the user sluice runs as, where the tests would reach it: at `DOCKER_HOST`,
  a unix socket (`unix://…`) or a TCP address (`tcp://host:port`), and at
  the usual sockets. A TCP address is asked plainly, or over TLS when
  `DOCKER_TLS_VERIFY` or `DOCKER_TLS` is set, with the certificates in
  `DOCKER_CERT_PATH`, as the engine's own client does; the sandbox lets the
  tests reach that address whatever `network` says. An `ssh://` address and
  Docker contexts are not read. A host where no engine answers runs none of
  the branch's tests and leaves them for a host that can, and
  `sluice exec <branch>` there refuses the tests, saying where it looked. A
  test that did not pass, after which the engine no longer answers, has no
  result: nothing is recorded and nobody is blamed.
- A host whose executor cannot hold a stated limit runs none of the
  branch's tests and leaves them for a host that can; it never runs them
  without it.

## Traps

- Tests that write outside the checkout (`~/.cache`, `/tmp`) fail in the
  sandbox. Point their scratch files at `SLUICE_TMP`. A cache every build on
  the host shares (a compile server's, moon's shared worktree cache) is the
  host's: declare it in the host's file, `[[caches]]` in
  `~/.config/sluice/config.toml` or where the host's configuration is
  managed (the skill's step 6, *The host's file*), with its `dir`, its server's `socket` and
  the `env` its clients read. The sandbox opens each `dir` and `socket`, and
  every run gets the `env`. A repository's policy declares no cache.
- Tests that call out to the internet need `network = true`; prefer fakes.

## Verify

```sh
sluice exec <branch>      # same executor and limits as the gate
```

Reference: <https://agentplot.github.io/sluice/reference/configuration/#executor-what-the-tests-may-touch>.
