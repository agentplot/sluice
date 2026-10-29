## Context

See proposal.md for why; record 0072 for the decision and the options
weighed. What is there today (origin/main, f7d7261f):

- `crates/sluice/src/profiles/checks.rs`: `sandbox_profile` gathers the
  place's checkout and scratch, each host cache's directory (given and
  resolved) and socket, and the container engine's sockets, and
  `seatbelt` writes the profile text from them, the runner's stamps, the
  network and the user's temporary directory. The run builds an
  `Enforced` (kind, confined, processors, memory, network, docker) and
  keeps it in the run's `Meta`; `RunReport::executor` carries it, and
  `ResultLine::of` writes it on each ran line.
- After a run in which a check did not pass, `ready_after` runs
  `[tests] ready` once, keeps its answer in the run's directory, and on
  a failure moves the fails and the missing into `not_run` with a
  reason; passes stand. The core reads `not_run` and needs nothing else.
- `crates/sluice-core/src/verdict.rs` `read`: on an unsigned branch a
  line counts when the gate trusts its executor and `same_limits` holds
  against the run's (kind, confined, memory, network, docker); on a
  signed branch only the signer and the system are compared. A replay
  is proven by a counted pass under its task and hash.
- `crates/sluice/src/signing/statement.rs`: the predicate carries the
  executor's kind and the claims.

## Goals / Non-Goals

**Goals:**

- Runs whose sandbox profile differs are never weighed together for a
  flake, and a replay is proven only by a pass under its own grant.
- Places and hosts running one executor with one profile share their
  passes.
- A check that failed because the sandbox refused it something leaves
  no result under its key; a check that failed for its own reasons is
  recorded as before.

**Non-Goals:**

- The Linux sandbox and the container executor.
- Changing moon's hash, or anything moon caches.
- Reading a check's output.

## Decisions

### The grant of each executor

`Enforced` gains `grant: Option<String>`, `#[serde(default,
skip_serializing_if = "Option::is_none")]`, so a line written before it
reads `None`.

- **Sandbox**: `seatbelt` is called a second time with every path
  replaced by its role: the place's checkout `<place>`, its scratch
  `<scratch>`, each cache directory `<cache>`, each cache socket
  `<cache-socket>`, each container engine socket `<engine-socket>`, the
  user's temporary directory `<user-tmp>`. Stamps are written as given.
  Lines are deduplicated in order, so two hosts with two and three
  cache directories render the same text. The grant is `sb1:` and the
  first 16 bytes of the SHA-256 of that text, in hex. The rendering is
  one function beside `seatbelt`, so the profile written and the grant
  recorded cannot drift: a unit test renders both from one input and
  checks that substituting the roles back gives the written profile's
  rules.
- **Open**: `open:1`, the same for every open run.
- The scheme's prefix and number change whenever the rendering does, so
  a new rendering is a new grant and never a collision.

The grant goes on the run's `Meta.executor` when the run starts and on
`RunReport::executor` when it is read, so every ran line carries it. A
session run (`sluice exec`) carries it the same way.

### The gate compares the grant wherever it compares limits

`same_limits` adds `grant` to the tuple it compares: `None == None`, so
two old lines are weighed together, and `None != Some(_)`, so an old
line is never weighed with a new run (138). On a signed branch `counts`
also requires `same_limits` against the run's executor, which brings
clause 19's limits to the signed path, where they were not compared.
Both paths prove a replay only from lines that count, so a replay under
one grant needs a pass under the same grant.

`FlakyKey`'s text is unchanged: the contradicting line is by definition
under the same grant.

### Signed runs

The predicate's executor gains `grant`, signed with the rest; the
records seam's `remote refs` profile reads it from the statement into
the line's `Enforced`, and from nowhere else. The grant needs no
allowance of the signer's: it is what the executor observed.

### moon's cache

Nothing changes in moon's inputs or its hash. A replay moon makes from
outputs another grant produced is unproven when no pass under the run's
grant is recorded, and the round runs it again with no replay, as
`read` already does for an unproven replay.

### The profile marks its denials as the run's

Group 2's first task probes, on macOS, which of these tells one run's
Seatbelt denials apart from another's in the unified log, and the build
uses the first that works:

1. A message the profile attaches to each deny rule, carrying the run's
   name.
2. The sandboxed processes' ids, tied to the run through the
   `sandbox-exec` process the supervisor started.
3. The run's span alone.

If the first two fail, record 0072 is amended to say the span is used,
and what a second run in the same span costs: its fails run again.

### Reading the denials after a run

`crates/sluice/src/profiles/denials.rs` reads them: `log show --style
ndjson --start <run start> --end <run end> --predicate <the kernel's
Sandbox sender and the mark>`, parsed into `Denial { process, operation,
path }`, the operation and the path as the kernel wrote them. It runs
only for a sandbox run in which a check did not pass and that its time
limit did not end, after `ready_after` and only when `ready_after`
moved nothing. Its answer is kept in the run's directory as
`denials.json`, or the error as `denials-error`, so a finished run read
again reads the file and runs nothing.

On one or more denials, every task that failed and every missing task
moves to `not_run` with the reason `the sandbox refused <operation>
<path>`, the first three denials and `and N more`. Passes stand. On an
error, the run is read as with no denial, and the error is kept on the
run and shown by the text views as `sluice could not read the sandbox's
refusals: <the log's words>`.

### What a person is told

`sluice exec` and a submission's failure name a check the sandbox
refused as `could not run: <task>: the sandbox refused …`, followed by
`sluice's sandbox stopped the run, not the test; nothing was recorded
under its key`. The set-aside hint is not shown for it, since there is
nothing to set aside.

## Risks / Trade-offs

- **Every check runs once more under its first grant**: no old pass
  proves a replay. Paid once per grant.
- **Tools that meet a denial in every run** (a tool that tries a path it
  may not write and goes on) make every failing run's fails not run, so
  a real fail is blamed only on the third run in a row (115). The
  denial is named so the profile or the host's `[[caches]]` can grant
  what the tool needs.
- **`log show` time**: seconds to tens of seconds over a long run's
  span, paid only by a run that did not pass, and once.
- **A user who cannot read the kernel's log** gets the error with the
  run; the grant still keeps such a fail off any other grant's key.
