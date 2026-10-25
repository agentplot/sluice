# Runner: moon

**Choose it when** the repository runs its tasks with moon 2.6 or later.
sluice asks moon which required tasks a combination affects, runs only
those, and lets a task whose inputs did not change replay its earlier result
when the branch trusts the cache.

## Keys

```toml
[tests]
runner = "moon"
toolchain = ["devenv", "print-dev-env"]   # optional: the shell the tasks run in
release = "scripts/stack-down"            # optional: removes what tasks hand off
ready = ["docker", "info"]                # optional: the host can run the tasks
not_run = [77]                            # optional: exits meaning "could not run here"
cold = ["#bench"]                         # optional: tests run with none of the host's caches

[branches."bolt/*"]
required = ["#check-fast"]                # moon targets: a tag (#tag) or project:task
trust = ["open", "sandbox", "cache"]   # "cache" lets moon replays count

[branches.main]
required = ["#check-fast", "#check-tools"]
trust = ["confined", "sandbox"]           # no "cache": every required task really runs
```

- `required` names moon targets. Tag the tasks a branch needs (`tags:
  [check-fast]` in `moon.yml`) and require `#check-fast`; a slower tag can be
  required on `main` only.
- sluice runs the affected required tasks with `moon exec`, `--on-failure
  continue` and `--concurrency $SLUICE_JOBS`, and reads moon's run report for
  each task's outcome and hash.
- With a `.prototools`, sluice runs `proto install` in the checkout before
  moon, outside the executor, so the pinned moon is there.
- `cold` names tests, as `required` does, that a cache must not warm (a
  benchmark, a timing). A run holding one gets none of the host's
  `[[caches]]`: no cache directory, socket or variable. The run's other
  tasks lose only speed.

## Make moon's affected query right

- Every task needs accurate `inputs`. A task with none is affected by
  everything; one with too few is skipped when it should run.
- A task that needs a server depends on it with moon 2.6's `wait` and
  `cleanup` dependencies, so the task brings it up and tears it down. sluice
  starts no services.

## Traps

- A branch that trusts `"cache"` accepts results moon replayed from any
  shell's earlier run. Keep `"cache"` off branches whose gate must run for
  real.
- With the cache off a branch's `trust`, every required task runs every time.
  Declare `[[checkout.cache]]` for build outputs (`target`, `node_modules`) so
  checkouts start warm; a cache only spares work and never decides a result.
- moon reads `MOON_*` from the environment; sluice gives moon the run's
  environment only, so set what tasks need through the toolchain.

## Verify

```sh
moon query tasks --affected      # in a worktree with a change: what sluice would consider
sluice exec <branch>             # the affected required tasks, under the branch's limits
```

Reference: <https://agentplot.github.io/sluice/reference/configuration/#tests-how-tests-run>.
