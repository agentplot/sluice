## Context

See proposal.md for why, and record 0072 for the decision and the
options weighed. What is there today, on
`bolt/a-sandboxs-refusal-blames-no-one` at 47fa1fcc:

- `crates/sluice/src/profiles/checks.rs`: `sandbox_profile` gathers the
  place's checkout and scratch, the run's moon store where it lies
  outside them (record 0105), each host cache's directory and socket,
  given and resolved, and the container engine's sockets and TCP
  addresses. `seatbelt` writes the profile text from them, with the
  runner's stamps, the network and the user's temporary directory.
  `start` writes it to `sandbox.sb` in the run's directory, prefixes
  the command and the release with `sandbox-exec -f`, and keeps an
  `Enforced` (kind, confined, processors, memory, network, docker) in
  the run's `Meta`. `RunReport::executor` carries it, and
  `ResultLine::of` writes it on each ran line.
- `crates/sluice-core/src/step.rs` `after_run`: after a run in which a
  check did not pass and that its time limit did not end, it asks the
  check seam `switched`, then, for a run that asks for the engine,
  `unmet`. Where either answers, every check that failed, was failed by
  a task it depends on (149) or was left without a result moves to
  `not_run` with that reason, `dependencies` is emptied, and passes
  stand. A combination's run, a claimed run and a session's run
  (`Sluice::finish_session`) all pass through it. A command's run never
  does.
- `crates/sluice-core/src/verdict.rs` `read`: on a branch in a host's
  own files a line counts when the gate trusts its executor and
  `same_limits` holds (kind, confined, memory, network, docker). On a
  signed branch only the signer, its claims and the system are compared,
  since a signed line carries no limits. `why_unproven` gives five whys.
- `crates/sluice/src/signing/statement.rs`: the `run/v1` predicate
  carries the results, the claims (executor kind, confined, system),
  the branch, the combination, the claim and the time.
  `crates/sluice/src/profiles/remote_refs.rs` `statements` groups lines
  by branch, combination, kind, confined and system, and `verify` reads
  a statement back into lines.
- `crates/sluice/src/runs.rs` `supervise`: runs the command as the
  leader of a process group, ends the group when it exits or its time
  runs out, and writes `result.json` last. The module names no tool.

### What macOS gives

Probed on deneb, macOS 27.0.1 (26A434), 2026-11-11, as the host's user,
with `sandbox-exec` profiles shaped as `seatbelt` writes them:

1. `(deny file-write* (with message "<mark>"))` is accepted, and the
   kernel's report of each denial that rule makes ends with a line
   holding the mark: `Sandbox: touch(63462) deny(1) file-write-create
   <path>\n<mark>`. `(with report)` on a deny rule is refused at compile
   time; `(with no-report)` on one is accepted and silences it.
2. The reports reach `log stream --style ndjson --predicate 'sender ==
   "Sandbox" AND eventMessage CONTAINS "<mark>"'`, as the host's user
   with no `sudo`. The stream prints `Filtering the log data using …`
   first, then one JSON object a line. `log show` over the same span,
   minutes later, finds none of them, nor the other processes' reports
   the stream saw then. The persisted log does not keep them.
3. Every process the loader starts is refused `file-write-data
   /dev/dtracehelper` and goes on. Under the profile as written, every
   run would report a refusal.
4. Under `network = "localhost"`, a name lookup is refused as
   `network-outbound /private/var/run/mDNSResponder`, and the program
   says the name is not known. This is the refusal behind Flume's
   submission 165.
5. A repeated denial comes as `<n> duplicate report(s) for Sandbox: …`,
   with the mark.
6. A sandboxed process may apply only the profile it already runs
   under; any other gives `sandbox_apply: Operation not permitted`.
   So a test that starts a sandbox runs outside any, as the `cli:sandbox`
   task already has it.

## Goals / Non-Goals

**Goals:**

- Results under two grants are never weighed together for a flake, and
  a replay is proven only by a pass under its run's grant, on every
  branch.
- Places and hosts running one executor with one profile share their
  passes.
- A check that did not pass in a run the sandbox refused something
  leaves no result under its key and blames nobody. A check that failed
  in a run the sandbox refused nothing is recorded as before.
- The person told of it learns what was refused.

**Non-Goals:**

- The Linux sandbox and the container executor.
- Changing moon's hash, or anything moon caches.
- Reading a check's output, exit or framework's words.
- Choosing which refusals matter. Every refusal the sandbox reports
  counts, but for the one device D4 silences.

## Decisions

### D1. The grant of each executor

`Enforced` gains `grant: Option<String>`, `#[serde(default,
skip_serializing_if = "Option::is_none")]`, so a line written before it
reads `None` and is written again without it.

- **Sandbox.** `seatbelt` takes what the run may reach with each path's
  role: the place's checkout `<place>`, its scratch `<scratch>`, each
  cache directory and the runner's store `<cache>`, each cache socket
  `<cache-socket>`, each engine socket `<engine-socket>`, each engine
  TCP address `<engine-tcp>`, the user's temporary directory
  `<user-tmp>`, and the run's mark `<mark>` (D4). One function renders
  the profile either with the paths or with the roles, so the profile
  written and the grant cannot drift. The rendering with roles keeps
  each line once, in order, so two hosts with two and three cache
  directories, or a path given and resolved, render the same. Stamps,
  literal device paths and the resolver's socket are written as they
  are. The grant is `sb1:` and the first 16 bytes of the SHA-256 of
  that text, in hex.
- **The runner's store is a `<cache>`.** What it grants is what a
  declared cache grants: a directory every place's runs share. A role
  of its own would split one host's keys from another's by where moon
  keeps its store alone.
- **Open.** `open:1`, for every open run.
- A new rendering takes a new prefix (`sb2:`), so it is a new grant
  and never a collision.

`start` sets the grant on the `Enforced` it keeps in `Meta`, so every
ran line carries it, a session's run and a preview's alike.

### D2. The gate compares the grant wherever it weighs a key

`same_limits` compares `grant` with the rest: `None == None`, so two
old lines are weighed together, and `None != Some(_)`, so an old line
is never weighed with a new run (138). On a signed branch `counts` also
requires `same_limits` against the run's executor, as record 0072's
consequences have it. That brings clause 19's limits to the signed
path, where they were not compared, since a signed line carried none
(D3). Both paths prove a replay only from lines that count, so a replay
under one grant needs a pass under the same grant.

`why_unproven` reads the limits' why on both paths, after the trust's
on a host's own branch and after the signer's on a signed one, where a
counted result exists and none is held to the run's limits but for the
grant. It gains a sixth why after it, `no result of its key was run
under this run's grant`, where some are and none shares the grant.
`FlakyKey`'s words are unchanged, since the result it names is under
the same grant by construction.

### D3. A signed run carries its limits and grant

The `run/v1` predicate's `Run` gains `held`: what the executor held the
run to beyond its claims, its memory, network, container engine and
grant, each absent where it held none. It is signed with the rest, and
absent on a statement written before it. `statements` adds it to what
splits lines into statements, so one statement holds one executor's
limits and grant. `verify` reads it into the line's `Enforced`, and from
nowhere else. It needs no allowance of the signer's: it is what the
executor observed, as the system is. A statement with no `held` reads
with no limits and no grant, so its results are weighed with no new
run's. Record 0077's change, when it lands, keeps `held` among the Test
Result's annotations (rule core.40).

### D4. The profile marks its refusals as the run's

Every deny rule `seatbelt` writes, `(deny file-write*)` and `(deny
network*)`, carries `(with message "<mark>")`. The mark is
`sluice-run:` and 16 hex characters of the SHA-256 of the run's
directory and its start time, kept in `Meta::mark`. A run an older
sluice started has none, and its refusals are never read.

After the allowed writes, the profile adds `(deny file-write-data
(literal "/dev/dtracehelper") (with no-report))`. Every program's
loader is refused it and goes on, and no check needs it (probe 3).
Granting it was not taken: C.2's sandbox row names the devices a run
may write, and this is none of them.

### D5. The supervisor keeps the reports as the run goes

The persisted log keeps none of the reports (probe 2), so they are kept
while the run goes. `runs::Request` gains `watch: Option<Vec<String>>`,
a command the supervisor runs beside the run's command. The module
still names no tool.

- The supervisor starts the watch in a process group of its own,
  outside the run's, its standard output to `watch.out` and its errors
  to `watch.err`, and writes `watch.pgid`.
- It starts the run's command once `watch.out` holds a line. Where the
  watch exits first or ten seconds pass, it ends the watch, writes
  `watch-error` with what the watch said, and starts the command all
  the same. A watch never holds a run back.
- Once the run's group has ended, it gives the watch one more second
  for reports still on their way, then ends its group.
- `Runs::end` ends a watch whose group is alive, whether or not the run
  has ended, so nothing of a lost supervisor outlives its place (rule
  core.10). `Runs::remove` and `Runs::discard` go through it.

For a sandboxed run the check profile asks `["/usr/bin/log", "stream",
"--style", "ndjson", "--predicate", "sender == \"Sandbox\" AND
eventMessage CONTAINS \"<mark>\""]`. Every sandboxed run has a watch,
its own command's and a preview's too, so every path that runs the
sandbox runs it alike. `Checks::with_log` names another program for
tests, as `with_system_link` does for the system's link.

### D6. Reading what the sandbox refused a run

`crates/sluice/src/profiles/refusals.rs` reads `watch.out`. A line that
is no JSON object, the stream's first among them, is passed over, and
so is one whose message does not end with the run's mark. From each
other it takes `Sandbox: <process>(<pid>) deny(<n>) <operation>
<target>`, after any `<n> duplicate report(s) for `. It keeps each
`(process, operation, target)` once, in order.

Its words name the first three and how many more: `sluice's sandbox
refused the run network-outbound /private/var/run/mDNSResponder (a name
lookup, by bun), file-write-create /Users/me/.cache/x (by python3) and
2 more`. A refusal of the resolver's socket is called a name lookup,
since that is what it refused.

The seam gains:

```rust
/// 139: what the executor reports it refused a finished run.
pub enum Refusals {
    /// Nothing, or no report is kept: the open executor, a run with no mark.
    None,
    /// What it refused, in the profile's words.
    Refused(String),
    /// It could not read its own report, in the words of what failed.
    Unread(String),
}
fn refused(&self, run: &RunHandle) -> Result<Refusals>;
```

`Checks::refused` answers `None` for an open run and for a run with no
mark. It answers `Unread` where `watch-error` exists or `watch.out`
cannot be read, with those words. Otherwise it answers `Refused` where
the stream kept a report of the mark, and `None` where it kept none. It
runs nothing, so reading a run again reads the same answer.

### D7. The core reads it after a run in which a check did not pass

`after_run` asks, in order: `switched`; then `refused`; then, for a run
that asks for the engine, `unmet`. The first that answers gives the
reason, and those after it are not asked. A switch can make anything
fail, so it is named first. A refusal is the executor's own report of
the run, so it comes before the engine, which is read from the host
after the run. On `Refused(why)`, every check that failed, was failed
by a task it depends on or was left without a result moves to
`not_run` with `why`, and `dependencies` is emptied, as for a switch
(149; record 0084). Passes stand, and a required check the runner keeps
out of its CI mode stays missing. On `Unread(why)`, the report stands
as it is, `RunReport::refusals_unread` holds `why`, and the engine is
asked as before.

Since `finish_session` reads a session's run through `after_run`, a
session's run reads a refusal as a combination's does. A command's run
does not (record 0074, design D6). The rest is what a run with no
outcome already does: `Event::Lost` names each check with its reason,
the run's `lost` keeps it, and the combination is tested again. After
as many runs in a row as the deployment states, it is read as not
passed, and its session is told each check and what was refused (115).

### D8. A report that could not be read is said with the run

`RunReport::refusals_unread: Option<String>` carries it, `None`
elsewhere. `finish_run` logs it on the run's `Event::Detail` as
`refusals_unread`, absent where `None`. The detail line is written for
a run that has one, whatever else it holds. `words::refusals_unread`
gives `sluice could not read what its sandbox refused the run: <why>`.
`sluice exec` prints it after the run's lines. A submission's failure
prints it under what failed, from its combination's detail lines, as
it does the checks kept out of CI.

### D9. Fakes and conformance

`sluice-fakes`' world gives each host a grant, `None` until a test sets
one, on every report it makes. It also answers `refused` from what a
test sets for the host, `Refused` or `Unread`, and counts each time it
was asked, as it counts system looks. `sluice-core`'s `ops.rs` test
host and `RecordedChecks` in `crates/sluice/tests/profiles/conformance.rs`
answer and forward it as they do `switched`.

## Risks / Trade-offs

- **Every check runs once more under its first grant.** No old pass
  proves a replay. It is paid once per grant.
- **A refusal a passing run meets.** A tool that tries a path it may
  not write and goes on makes every failing run's fails not run, so a
  real fail is read as not passed only on the deployment's count of
  runs in a row (115), its session told each test and what was refused.
  Task 5.7 lists what a passing run of sluice's own `cli:test` is
  refused, under "What a passing run is refused" below.
- **A stream beside every sandboxed run.** One `log stream` process for
  each run's life, a second added to its end, and nothing read for a
  passing run. The stream's predicate holds the run's mark, so it writes
  only that run's reports.
- **A report the log drops.** The kernel folds repeats into duplicate
  reports and may drop some under load. A refusal no report reaches
  reads as none, so the fail is recorded, under the run's grant only.
- **macOS changes.** The message's form and the stream's are read as
  macOS 27.0.1 gives them. A macOS that keeps none reads every run as
  refused nothing, which is today's behaviour.
- **A host whose user cannot stream the log.** Each failing run says
  so, and is read as today, its fails under its own grant.

## What a passing run is refused

Run on deneb, macOS 27.0.1 (26A434), 2026-11-11, as the host's user:
`target/debug/sluice exec bolt/a-sandboxs-refusal-blames-no-one -- moon
run cli:test` in this change's place, at d198045d, task 5.3's code,
under the branch's policy (`kind = "sandbox"`, `network = "localhost"`,
`docker = true`) and the host's two `[[caches]]`, sccache's and moon's.
The run passed, 692 tests in all, the crate's 279 unit tests among
them. Its watch was ready, wrote nothing to `watch.err`
and left no `watch-error`, and `watch.out` kept 35 reports of the run's
mark, `/dev/dtracehelper` reported by none. Each refusal, once, with the
programs that met it:

| Refused | By | What it is |
|---|---|---|
| `file-write-create <git common dir>/moon-home/cache/shared/blobs/temp/<n>` | moon | The runner's store, the one C.2's sandbox row grants (record 0105). `sandbox_profile` already opens it to a run of the moon runner, as a `<cache>`; this run is a command's, which resolves no store, and a command's refusals are never read (record 0074, D6). Nothing to grant. |
| `file-write-create <git common dir>/moon-outputs/<hash>.tar.gz` | moon | moon's archive of `cli:test`'s outputs, written to the place's `.moon/cache/outputs`, which this repository's shell links to a directory every worktree shares. The kernel judges the path the link resolves to, which is no path C.2 grants: neither the place, its scratch, a declared cache nor the runner's store. A failing run will read it. |
| `file-write-data ~/.cargo/.package-cache`, `.package-cache-mutate` and `.global-cache`; `file-write-xattr` of `~/.cargo/registry` and `~/.cargo/git` | cargo | cargo's locks and marks in the user's cargo home, which cargo takes before it builds and goes on without. Not a path C.2 grants; a host that wants it granted declares it among its `[[caches]]`. A failing run will read it. |
| `file-write-create ~/Library/Caches/BytecodeAlliance.wasmtime/modules/…` | proto, moon | wasmtime's cache of the compiled plugins proto and moon load. Not a path C.2 grants; a host may declare it. A failing run will read it. |
| `network-outbound remote:*:53` and `/private/var/run/mDNSResponder` | moon | moon's look for an internet connection before it installs a toolchain's dependencies, which it skips ("No internet connection"). Refused as `network = "localhost"` says. A failing run will read it. |
| `network-outbound /private/var/run/mDNSResponder` | git | Name lookups by five of the tests' git processes, each of which passes without the answer. Refused as `network = "localhost"` says. A failing run will read it. |

What that costs (115). Every run of sluice's own `cli:test` under this
policy meets refusals that do not fail it, so once this change lands
every run of sluice's gate in which a check fails reads those checks as
not run, naming the first three refusals, records nothing under their
keys and blames nobody; the combination is tested again, and only after
as many runs in a row as the deployment states is it not passed, its
session told each check and what the sandbox refused. A fail that would
have been blamed on the first run costs two more runs, and the person
told of it reads a refusal that did not cause it. The same holds for any
repository whose tools write outside the place or look up a name under
`network = "localhost"` and go on.

None of these is granted in `sandbox_profile`: the one path of the run's
own that C.2's sandbox row grants, the runner's store, already is. None
is silenced: D4 silences `/dev/dtracehelper` alone, and silencing any
other, or granting any path C.2 does not name, is the user's to decide.
