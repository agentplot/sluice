## Context

See proposal.md for why. The statement's clause 135 (as amended at
agentplot/blueprints c619438), record 0107 and rule core.54 say what
holds. This design says how `sluice run` does it on this change's base,
which holds no host process and no daemon.

**The family.** No other tool's statement has a process of its own take
up a changed settings file: at c619438, `design/` names it for sluice
alone, in 135. Boomdrive's runner takes its file on its command line, so
a changed file is a changed job that the host's activation restarts
(agentplot/boomdrive record 0034, point 5). Nothing needs reconciling.
Should another tool keep a process that reads its own settings, it cites
this design's look (D1) instead of making one of its own.

What the code has today:

- `Instance::read` (`crates/sluice/src/instance.rs`) reads
  `$XDG_CONFIG_HOME/sluice/config.toml` (`config_path`) through
  `Instance::from_host(path, given, all)`. It reads a missing file as
  empty, so every default holds, and refuses a field it does not know,
  a value out of bounds and a key file others may read.
  `Instance` derives `PartialEq`.
- `Bound::open` (`crates/sluice/src/profiles/mod.rs`) reads it once and
  hands its parts to every profile: the forge's app
  (`github::App::open`, which reads and parses the key) to the branch,
  the pull request driver and the pool; the discovery interval and the
  peer (name, lease, alive, signer, systems, deliveries' span) to the
  pull request driver; the signing to the records; the bound, cores and
  caches to the checks and the places' release; the host's name to
  worktrunk's driver.
- `main` (`crates/sluice/src/main.rs`) opens one `Bound` and calls
  `sluice::run` with its seams. `run`'s `Command::Run` arm
  (`crates/sluice/src/lib.rs`) checks the host's systems
  (`Sluice::check_host_systems`), then loops: it takes or keeps the
  repository's `Driver` lock and `Reader`, runs every branch's round,
  writes the reader's document, counts the pass toward `--rounds`, and
  pauses. Between passes it carries the driver, the reader, `asked`,
  `said_beside`, the pass count `n`, the holds it said (`said`) and the
  branches it passed by (`passed`).
- What a pass reads by revision, the policies, the signers and the pull
  requests' answers, is kept in the profiles (`Branches::next_pass`,
  `Records::next_pass`, `Pulls::next_pass`). When the forge was last
  asked, its answers' ETags and a spent allowance are kept in the host's
  state (`github::kept::Kept`), so a new binding finds them.
- A run is a process of its own, started with its request. The request
  holds its share of the cores (`Instance::jobs`), the caches' variables
  and the sandbox's paths, and the run's record keeps what its executor
  held it to (`Enforced`). A later round finds the run finished and
  records its result, signed under the signer that round's records
  hold. The host's count of running checks is in the host's state and
  is read under the host's lock at each start (`host_count`).

## Goals / Non-Goals

**Goals:**

- A kept `sluice run` notices a changed settings file between passes,
  at the cost of a few file-system calls, and takes up what it holds
  before its next pass.
- A file that does not read, or settings that do not bind, leave the run
  on what it holds, said once for each change.
- A run going keeps what it started with. The driver stays the driver.

**Non-Goals:**

- The host process, `sluice daemon`, the hand-over to a process started
  afresh, a wait, a watch, a key changed in place, and a local layer:
  see the proposal's Out of scope.
- A watch of the file by the operating system's notifications.

## Decisions

### D1. The look: the file's sign, never its contents

`HostSettings` (`crates/sluice/src/host_settings.rs`) holds the
`Instance` a process works under and the `Sign` of the file it read it
from. `HostSettings::look` takes the sign again and compares. While the
sign stands it answers `Look::Same` and reads nothing.

The sign is three parts, each `None` where there is nothing to read:

- the file's own entry, by `symlink_metadata`: its modification time
  and size;
- the path the file resolves to through every link, by
  `std::fs::canonicalize`, which takes in a link to its directory;
- the file found there, by `metadata`: its modification time and size.

It is taken with no process started and no git read, so a look costs
about as much as the pause's own clock read (rule core.31, which rules
out a check that starts a process or reads git; core.54 names the
modification time and size as the sign).

*Why three parts.* sluice's own module links the file into Nix's store
(core.51), and every file there has a modification time of one second
past the epoch. A host's next configuration moves the link to another
store file, which may well have the same size. Its target's time and
size alone would miss that. The link's own entry and the resolved path
catch it. A plain file edited in place, or replaced by a rename, moves
its own time.

*Not taken.* The target's time and size alone, as record 0107 words the sign: it
misses a Nix-managed file, the case that led to this record. The
operating system's notifications, kqueue or inotify: a thread or
descriptor per process and code per platform, for a change that need
only count from the next pass. A digest of the contents at each look: a
read of the file every pass.

### D2. A read is taken only where the sign stood through it

A look whose sign moved reads the file: it takes the sign, reads and
parses the text through `Instance::from_host`, and takes the sign again.
Where the two differ, a writer was still at work, so the read counts
for nothing: the look answers `Same`, keeps the sign it held, and reads
again at the next look. Where they agree, it keeps the new sign and
answers:

- `Look::Refused(words)` where the file did not read (D4), keeping the
  `Instance` it holds;
- `Look::Same` where the settings read equal those held, which an apply
  that rewrote or relinked the file with the same settings gives;
- `Look::Changed(instance)` otherwise. `HostSettings::accept(instance)`
  makes it the one held once the caller has bound it (D5). A caller
  that could not bind it accepts nothing. The sign was kept, so the file
  is read again only once it changes again.

At a process's start, `HostSettings::read` keeps whatever it read, as a
command does, and the sign taken before the read, so a file moving as
the process starts is read again at the first look.

*Not taken.* Reading a file and taking what it gives whatever moved
during the read. A writer that rewrites in place, rather than by a
rename or a link, could leave a pass under half its settings.

### D3. `sluice run` drives one pass at a time

`run`'s `Command::Run` loop becomes `drive(&mut Driving, ...)`, one pass
and its pause. `Driving` holds what the loop carries between passes:
the pass count and its `--rounds` bound, the `Driver` lock, the
`Reader`, `asked`, `said_beside`, `said` and `passed`, and whether the
host's systems were checked. `drive` builds the pass's `Sluice` from the
seams it is given, checks the host's systems on its first pass as `run`
does today, and answers `Driven::Next` after the pause, or
`Driven::Ended(code)` where the bound is reached or the pause was
stopped, with today's codes. `run`'s arm loops `drive` until it ends, so
every test that drives `sluice run` through `run` drives it as before.

`main` takes `sluice run` past `run`: it reads `HostSettings`, binds
the repository with `Bound::open_with`, and loops `drive`. After each
`Driven::Next` it looks (D5) before the next pass. A pass therefore runs
wholly under one binding. Settings are taken up only between two calls
of `drive`, never within a pass (core.54: never within a round; core.14:
a pass's decisions are the records', so a new binding finds them).

*Not taken.* Swapping each profile's parts inside one `Bound`: every
profile holds a part of the settings, the app among them, and making
each swappable spreads the take-up across the profiles. Making `run`
return to `main` whenever the settings changed, letting go of the
driver lock and the reader between: another `sluice run` could take the
repository while this one still lives, which host-driver's "A second
sluice run drives nothing while the first lives" rules out.

### D4. What does not read

A changed file does not read where:

- it is gone. sluice keeps what it holds and says `it is gone; an empty
  file gives sluice's defaults`;
- it cannot be read, or sluice's reading refuses it (an unknown field,
  a value out of bounds, a key file others may read). sluice keeps what
  it holds and says the reader's words.

Settings that read can still fail to bind (D5), and those are refused
the same way. `main` says each refusal on stderr through
`telemetry::warn`:
`sluice: warning: <path> is refused, so this sluice run keeps the
settings it holds until the file changes: <why>`. The file is read only
when its sign moves, and the sign is kept even for a file that did not
read, so each change is said once and never again while the file
stands (record 0107: "said once, naming the file and why, until the file
reads again").

*Why a gone file keeps what is held.* A command that starts with no file
takes the defaults, and still does. A process that has been running
loses its file mostly in an apply's moment between removing a link and
making the next one. Read as the defaults, it would drop the forge's app
and the signer for a pass, pushing with the host's own credentials and
signing nothing. Rule core.54 rules out dropping the settings held for a
file that cannot be read. An operator who wants the defaults writes an
empty file, as the words say.

*Not taken.* A gone file read as the defaults, as a starting command
reads it; see above. Saying a refusal at every look while the file
stands: core.47's and core.28's way is once per cause, and a look reads
nothing new to say.

### D5. Taking up: bind afresh, then accept

On `Look::Changed(instance)`, `main` binds the repository afresh with
`Bound::open_with(dir, program, instance.clone(), retry)` and checks
the new binding's host systems (`Sluice::check_host_systems`) before
anything is dropped. Where both succeed, the new binding replaces the
old one, `HostSettings::accept(instance)` is called, and `main` says on
stderr `sluice: took up the host's settings in <path>: <tables>
changed`, naming `Instance::changed_tables(held, new)`. Where either
fails, the old binding stays, nothing is accepted, and the failure is
said as D4 says a refusal.

`Instance::changed_tables(&self, other)` names, in the configuration
reference's order, each table whose settings differ: `[runs]` for the
bound, the reserve, the cores sluice may use and the systems; `[forge]`;
`[signing]`; `[peers]`; `[deliveries]` for the span; `[[caches]]`.

Every table the file holds today binds afresh in place, so every change
on this base is taken up this way. This step is where a later setting
that no process can take up in place is handed over instead (the
proposal's Out of scope).

*Not taken.* Saying nothing on a take-up: an operator who changed the
file could not tell whether it counted without a status that says so.
One line for each change is enough.

### D6. What a run keeps, and what the next round does

A run's share of the cores, its caches and its executor's grant are in
its request and its record from its start. A new binding changes
neither, and nothing stops a run or starts it again. The host's count is
read at each start under the bound the round holds, so a smaller bound
starts nothing until enough runs end, and a larger one starts more at
the next pass.

A run is recorded when a round finds it finished. That round signs its
result with the signer its own binding holds, as a round of a restarted
`sluice run` would today. Signing is the round's act as the host, not
the run's (core.22 reads the result through the signer the branch's
policy names).

*Not taken.* Keeping each run's signer and key in its record and signing
its result with them. The old key would have to stay readable after the
operator replaced it, and a result signed with a key the branch no
longer lists reads as missing either way (core.22).

A `[peers] name` changed while runs are going is read as a rename of the
host. A run claimed under the old name is another host's to this one,
and its claim lapses as a quiet host's does (77), as after a restart.

### D7. The first pass under a new binding reads each policy once

A new binding starts with none of the profiles' per-revision reads, so
its first pass reads each branch's policy once more, as any process's
first pass does. Rule core.31 rules out a policy read again while its
revision stood. That rule is about a quiet host's passes, each starting
git with nothing changed. A change of the host's settings is a move on
the host, and core.54 allows taking it up in a process started afresh,
which reads every policy. No read repeats while the file and the
revisions stand.

*Not taken.* Carrying the profiles' per-revision reads into the new
binding: it saves one read per branch for each change of the settings
and couples every profile to the take-up.

## Risks / Trade-offs

- **A writer that rewrites in place, within one tick of the clock's
  resolution, to the same size** is missed until the file next changes.
  APFS and ext4 keep nanoseconds, so the risk is a file system with
  coarse times. The reference's ways of writing the file, an editor's
  rename and Nix's link, are seen.
- **A read caught mid-write that parses** is ruled out only where the
  sign moved during the read (D2). A writer that pauses between two
  writes with the read between them is not caught. Nix's links and an
  editor's rename write nothing in place.
- **A refusal said in an apply's moment between two links.** If a look
  falls between a link's removal and the next, that pass says the file
  is gone, and the next takes up what the new link gives. The window is
  milliseconds against a pass of seconds.
- **A changed key file at the same path is not seen** (proposal, Out of
  scope). The reference tells the operator.

## Migration Plan

Nothing to migrate: the file, the state and the records keep their
form. A host that restarts sluice when the file changes, as
afterthought/swancloud's driver module does, keeps working and may drop
its restart once its sluice has this change.

## Open Questions

None.
