# Runner: command

**Choose it when** the repository has no moon: one command runs the tests,
and exit 0 passes. Every test runs every time; nothing is replayed.

## Keys

```toml
[tests]
runner = "command"
command = "make test"                     # a string run by sh -c, or ["argv", "list"]
toolchain = ["devenv", "print-dev-env"]   # optional
release = "docker compose down -v"        # optional

[branches.main]
parallel = 2
timeout = "20m"
```

- Leave `required` out: the command is the only test, recorded under the
  name of the command.
- The command runs in a clean checkout of the combination, in its own
  process group, under the executor and the branch's `timeout`. sluice kills
  the group when the command exits, times out, or the checkout goes.
- It gets `SLUICE_CHECKOUT`, `SLUICE_TMP`, `SLUICE_JOBS` and a
  `SLUICE_PORT_<NAME>` for each `[checkout] ports` entry. Use `SLUICE_TMP` for
  sockets, pid files and data directories so parallel checkouts never
  collide.

## Traps

- Slow suites test every combination in full. Turn on `batch = true` for the
  branch to test only the longest combination and bisect on failure, and
  keep `parallel` low.
- Anything the command starts outside its process group (a container, a
  detached daemon) must be removed by `[tests] release`.

## Verify

```sh
sluice exec <branch>       # runs the command in the worktree under the branch's limits
```

Reference: <https://agentplot.github.io/sluice/reference/configuration/#tests-how-tests-run>.
