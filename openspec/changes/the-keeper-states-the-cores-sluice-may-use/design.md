# Design

## Context

`crates/sluice/src/instance.rs` reads the host's settings,
`$XDG_CONFIG_HOME/sluice/config.toml`, into `Instance`. Its
`processors` field is documented as "the cores sluice may use: the
process's affinity and its cgroup's quota where the host has them", and
is `std::thread::available_parallelism()`. `Instance::jobs()` is
`processors / bound`, at least one, and is the only place a run's share
is computed: the check profile hands it to every run as `SLUICE_JOBS`
and to moon as `--concurrency`, records it as the cores a run was given
where the runner holds the run to them, and gives the release command
the same (`profiles/mod.rs`, `profiles/checks.rs`).
With no `[runs] max`, the bound is `processors / 4`, at least one.

Rust's `available_parallelism` on Linux counts the process's
`sched_getaffinity` mask and its cgroup v1 or v2 CPU quota; on macOS it
is the count of online logical CPUs, since macOS has neither an affinity
mask a process is held to nor a quota. Its documentation says it may
overcount on Linux when the cgroup filesystem or `sched_getaffinity`
cannot be read, as under some sandboxes.

So today what sluice may use is already what the host gives it (71),
and on a Mac that is every core. This change adds the keeper's
statement for a host that gives nothing, and leaves every reader of
`jobs()` as it is.

## Goals / Non-Goals

**Goals:**

- One setting, `[runs] cores`, beside `max` and `reserved`, that is the
  cores sluice may use where the host gives sluice no limit (71).
- The share and the default bound both follow from what sluice may use,
  whichever way it was found.
- A setting sluice cannot honour stops sluice with a message, as a bad
  `max` or `reserved` does.

**Non-Goals:**

- Holding a run's processes to its share by any means the executors do
  not already have (C.2): on macOS the open and sandbox executors still
  hold cores only through the runner's own threads.
- Reporting what sluice may use in `sluice overview` or `sluice status`.

## Decisions

### D1. Whether the host limits sluice is read by comparing two counts

The host gives sluice a limit when `available_parallelism()` is less
than the host's online logical CPUs, read with
`sysconf(_SC_NPROCESSORS_ONLN)` through `nix::libc`, which the crate
already reaches (`protect.rs`). On macOS the two are always equal, so a
Mac never limits sluice and `cores` always applies. On Linux an
affinity mask over every CPU and no quota, or a quota at or above the
CPU count, leave the two equal and are no limit; anything narrower is.

Alternatives: reading `/proc/self/status`'s `Cpus_allowed_list` and
`cpu.max` directly, which repeats what the standard library already
does and must follow cgroup v1 and v2 both; or reading only the
affinity mask, which misses a quota, the common limit under systemd and
containers. Where `sysconf` fails or reports fewer than
`available_parallelism()`, sluice takes the host's cores to be
`available_parallelism()`, which reads as no limit.

### D2. `Instance` is built from both counts

`Instance::from_file(path, processors)` keeps its signature and means a
host that gives sluice all of its `processors` cores, so the many tests
that build an instance from a core count keep working. A new
`Instance::from_host(path, given, all)` does the reading; `from_file`
calls it with `given == all`, and `Instance::read()` calls it with
`available_parallelism()` and the online count. `processors` keeps its
name and becomes, as documented, the cores sluice may use: the host's
limit where it gives one, `cores` where stated, and `all` otherwise.
`jobs()` is unchanged.

### D3. The default bound is a quarter of what sluice may use

With no `max`, the bound is `processors / 4`, at least one, computed
after `processors` is decided, so `cores = 8` with no `max` gives a
bound of two. The statement sets no default for the bound; the quarter
is the instance's and the site's (ADR 0047 names it), and it has always
been a quarter of `processors`, which this change makes the cores
sluice may use. A
quarter of all the host's cores while each run gets a share of fewer
would set the default bound from cores the keeper has said sluice may
not use.

### D4. A statement sluice cannot honour is refused

`RunsForm` gains `cores: Option<i64>`. With it set, `from_host` fails,
before anything else is decided, with:

- below one: `[runs] cores must be at least 1, not {n}`;
- above `all`: `[runs] cores = {n} is more than this host's {all} cores`;
- `given < all`: `[runs] cores = {n}, but this host already gives sluice
  {given} of its {all} cores by its affinity or quota, and that is what
  sluice may use: leave cores out`.

The last follows 71's own split: where the host gives a limit, what
sluice may use is that limit, and the keeper's statement is for a host
that gives none. Refusing names the conflict where the keeper looks
for it; taking the smaller of the two, or ignoring the statement,
would each run sluice on a number one of them did not choose, with
nothing said. Because `Instance::read()` is read by every command that
binds the repository, the refusal stops each of them, as a bad `max`
does now.

### D5. The docs say when to set `cores`

The site's configuration reference gains `cores` in the host's example
and in `[runs]`: what sluice may use, how each run's share and the
default bound follow from it, that a Mac never limits sluice so its
keeper sets `cores` to leave the rest of the host's cores for the
sessions beside sluice, and the refusals. *Parallel and batch*'s *The
host's bound* says the same in a sentence beside "yours to leave room
for". The README's host example and the setup skill's step 5 and its
`[runs] max` bullet gain the key. They use the user words: runs,
cores, the host's settings.

## Risks / Trade-offs

- [`available_parallelism` overcounts on a Linux host whose cgroup
  filesystem sluice cannot read, so a real quota reads as no limit and
  a stated `cores` is taken.] → The keeper's number is then what sluice
  uses, which is what the keeper asked for; nothing is refused that
  should have run.
- [A host config shared between a Mac and a Linux box under a quota
  works on the Mac and is refused on the Linux box.] → The host's
  settings are each host's own (C.7), and the refusal says which line
  to take out.
- [Lowering `cores` with no `max` lowers the default bound as well,
  which a keeper may not expect.] → The configuration reference says
  so beside the default, and setting `max` keeps the bound where it is.

## Migration Plan

None. A host with no `cores` behaves exactly as before: the same bound,
the same share. A sluice built before this change refuses a config that
sets `cores`, as it refuses any unknown key, so a keeper sets it only
once every sluice on the host is this one or later.
