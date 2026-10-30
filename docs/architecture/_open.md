# Open items

What the rules say ahead of the code. A working file: an item leaves
when its code lands, and the file is deleted when it is empty.

## Accepted, not built

- **core.39 and record 0075** (clauses 8, 10, 23, 24 and 140, added or
  amended 2026-10-30): today a submission whose work no required check
  reads lands with nothing run but joins the chain, waiting for the
  candidates under test ahead of it. Proposed 2026-10-30, on the
  user's card; a unit lands such work beside the chain at
  the first round that reads it, and lands a formed candidate from the
  revision it made, carrying its work.

- **core.38 and record 0074** (clause 84 as amended 2026-10-30): today
  `docker = true` only opens the engine's sockets in the profile and a
  host with no engine runs the checks and fails them. Accepted
  2026-10-30 on the user's card; a unit has the executor
  refuse such a run before claiming it, leave it open, and look at the
  engine again after a run in which a check failed, a gone engine
  making the run one with no outcome (0074 as amended 2026-10-30).

- **core.34, core.35 and record 0072** (clauses 138 and 139, added
  2026-10-29): today a check's key is the task runner's hash alone, so
  a fail under one sandbox profile is weighed against a pass under
  another, and a check that failed because the sandbox refused it
  something is recorded as a fail. Built by the change
  `a-checks-key-holds-its-executor`.
