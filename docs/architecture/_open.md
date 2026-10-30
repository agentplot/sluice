# Open items

What the rules say ahead of the code. A working file: an item leaves
when its code lands, and the file is deleted when it is empty.

## Accepted, not built

- **core.37 and record 0066 as amended** (clauses 84 and 114 as amended
  2026-10-30): today `[tests] ready` is read from the policy and
  `ready_after` runs it after a run that did not pass, making the run
  one with no outcome when it fails. Accepted 2026-10-30 on the user's
  words; the unit sluice-runs-no-readiness-probe takes the key, `READY_LIMIT` and the
  after-run ask out, so a run has no outcome only by 114's three
  causes.

- **core.34, core.35 and record 0072** (clauses 138 and 139, added
  2026-10-29): today a check's key is the task runner's hash alone, so
  a fail under one sandbox profile is weighed against a pass under
  another, and a check that failed because the sandbox refused it
  something is recorded as a fail. Built by the change
  `a-checks-key-holds-its-executor`.

## Proposed

- **core.38 and record 0074** (clause 84 as amended 2026-10-30): today
  `docker = true` only opens the engine's sockets in the profile and a
  host with no engine runs the checks and fails them. Proposed
  2026-10-30, on the user's card; a unit on their yes has the executor
  refuse such a run before claiming it and leave it open.
