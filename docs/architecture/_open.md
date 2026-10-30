# Open items

What the rules say ahead of the code. A working file: an item leaves
when its code lands, and the file is deleted when it is empty.

## Accepted, not built

- **core.34, core.35 and record 0072** (clauses 138 and 139, added
  2026-10-29): today a check's key is the task runner's hash alone, so
  a fail under one sandbox profile is weighed against a pass under
  another, and a check that failed because the sandbox refused it
  something is recorded as a fail. Built by the change
  `a-checks-key-holds-its-executor`.

## Proposed

- **core.36 and record 0073** (clause 84 as amended 2026-10-30): today
  `ready_after` runs the probe in the run's place at the candidate's
  tree, so a candidate that removes what the probe runs is read as the
  host not ready. Proposed 2026-10-30, on the user's card; a unit on
  their yes brings the place to the branch's revision before the
  probe.

- **core.38 and record 0074** (clause 84 as amended 2026-10-30): today
  `docker = true` only opens the engine's sockets in the profile and a
  host with no engine runs the checks and fails them. Proposed
  2026-10-30, on the user's card; a unit on their yes has the executor
  refuse such a run before claiming it and leave it open.
