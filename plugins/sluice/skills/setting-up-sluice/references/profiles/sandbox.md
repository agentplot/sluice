# Executor: open or sandbox

**Choose `sandbox`** on macOS when the gate should hold tests to their
checkout: they may write only the checkout and their scratch directory, and
reach only the network the policy allows (Seatbelt). **Choose `open`** on
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
- An executor that cannot enforce a stated limit refuses the run; it never
  runs without it.

## Traps

- Tests that write outside the checkout (`~/.cache`, `/tmp`) fail in the
  sandbox. Point caches at `SLUICE_TMP` or the checkout.
- Tests that call out to the internet need `network = true`; prefer fakes.

## Verify

```sh
sluice exec <branch>      # same executor and limits as the gate
```

Reference: <https://agentplot.github.io/sluice/reference/configuration/#executor-what-the-tests-may-touch>.
