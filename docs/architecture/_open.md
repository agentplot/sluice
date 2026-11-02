# Open items

What the rules say ahead of the code. A working file: an item leaves
when its code lands, and the file is deleted when it is empty.

## Accepted, not built

- **core.41 and record 0078** (clause 131 as amended 2026-10-28, and
  clause 132): today a finished submission's index line holds its id,
  state, worktree and subject, and the log is one file. Part 2 of the
  change `finished-submissions` keeps the log in parts; the index's
  times, and the overview's rows carrying them, wait on a change of
  their own.

- **core.40 and record 0077** (clause 141, added 2026-11-01; C.6 as
  amended 2026-11-02): today a run's statement carries a predicate of
  sluice's own, `RUN_PREDICATE` in
  `crates/sluice/src/signing/statement.rs`. A change writes in-toto's
  Test Result in its place, and reads both.

- **core.39 and record 0075** (clauses 8, 10, 23, 24 and 140, added or
  amended 2026-10-30): today a submission whose work no required check
  reads lands with nothing run but joins the chain, waiting for the
  candidates under test ahead of it. Proposed 2026-10-30, on the
  user's card; a unit lands such work beside the chain at
  the first round that reads it, and lands a formed candidate from the
  revision it made, carrying its work.

- **core.34, core.35 and record 0072** (clauses 138 and 139, added
  2026-10-29): today a check's key is the task runner's hash alone, so
  a fail under one sandbox profile is weighed against a pass under
  another, and a check that failed because the sandbox refused it
  something is recorded as a fail. Built by the change
  `a-checks-key-holds-its-executor`.
