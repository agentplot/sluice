# Open items

What the rules say ahead of the code. A working file: an item leaves
when its code lands, and the file is deleted when it is empty.

## Accepted, not built

- **core.33, the part of core.32 that reads a branch's open requests in
  one query, and record 0071's "One host takes a branch's requests in"
  and "One query per branch"** (clauses 136 and 137, added 2026-10-28):
  today every host that reaches a branch takes its pull requests in,
  each into its own queue where the records are each host's own files,
  a wait on any host runs intake for the branch, and a request out of
  draft that waits on its conditions is read by REST per request at each
  discovery. Built by the change
  `one-host-takes-requests-in-and-reads-them-in-one-query`; clause 137
  and record 0071 as amended 2026-10-29: a driver pushes the mark when
  it starts, and a branch one host reaches by waits and steps alone
  keeps none.

- **core.34, core.35 and record 0072** (clauses 138 and 139, added
  2026-10-29): today a check's key is the task runner's hash alone, so
  a fail under one sandbox profile is weighed against a pass under
  another, and a check that failed because the sandbox refused it
  something is recorded as a fail. Built by the change
  `a-checks-key-holds-its-executor`.
