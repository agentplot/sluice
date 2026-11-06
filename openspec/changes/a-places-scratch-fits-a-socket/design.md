## Context

A checkout's record names its scratch (record 0063). `Places::scratch`
reads it, and gives the former name under `<common>/sluice/.tmp/` for a
record that names none. `adopt` writes a worktree's record at every run
and sets `scratch` only on a record it creates, so a worktree's checkout
an older sluice adopted is given its former scratch for as long as the
worktree lives, unless `sluice release` removes the record. Clause 35 as
the statement words it allows a place's scratch under no repository, and
the user asked on 2026-11-05 for the short scratch "so the tests run
again".

Each run of a worktree's checkout has a directory of its own,
`.runs/<checkout, flat>/<n>/`, whose `lock` its supervisor holds for the
run's life. `Runs::start` returns only once the supervisor holds it, and
`sluice exec` starts a run under the branch's lock, so a run another
`sluice exec` of the branch started before reads as going.

The sandbox profile names the scratch's resolved path as writable. A
run's request, kept in its directory, names its `SLUICE_TMP` and its
release command. The release a worktree's checkout runs with no policy,
and the one run for a checkout whose worktree is gone, is the last run's.

## Goals / Non-Goals

**Goals:**

- A worktree's checkout made before takes the short scratch at its next
  run, with no `sluice release`.
- What its checks keep in its scratch between runs goes with it where the
  system can rename the directory.
- No run going loses the scratch its sandbox writes.
- A test of sluice's own shows a landing's check binding a socket in a
  folder under its scratch within 104 bytes on macOS.

**Non-Goals:**

- Moving a candidate's or a preview's scratch made before.
- Copying a scratch the system cannot rename.

## Decisions

### The move is made when the worktree's checkout is adopted for a run

`adopt` reads the worktree's record at each run. For a record it read,
when `scratch` names no path under the user's directory and no run of
the checkout is going, it sets `scratch` to `new_scratch(name)`, writes
the record with the run's `last_run` as now, and moves the former
scratch there in place of making it. The record names the new path
before the directory is moved, as it names a new scratch before it is
made. The sweep's reading of the records after its listing then keeps
the new path, and leaves the former to the move or to itself (123).

- Rejected: moving at `sluice release`, `sluice status` or a round. Only
  a run needs the scratch, and only a run's adoption holds the branch's
  lock that orders it with another run's start.
- Rejected: a fresh scratch on every host. It would discard what a
  rename keeps where it can.

### The former scratch is renamed, or removed where it cannot be

`move_scratch(from, to)` makes the user's directory and the repository's
under it the user's own, as `make_scratch` does, then renames `from` to
`to`. Where `from` is gone, as the system may empty `/tmp` or a release
may remove it, `to` is made. Where the rename fails otherwise, as from
one file system to another (`EXDEV`) or onto a directory that is not
empty, `to` is made and `from` removed. A removal that fails is warned of
and left to the sweep, since no record names `from` now. A check keeps
nothing in its scratch it cannot make again (record 0063).

`make_scratch` is split. `make_parents(path)` makes and checks the
directories above a scratch under the user's directory. `make_scratch`
calls it and makes the scratch. `move_scratch` calls it and renames.

### A checkout with a run going keeps its former scratch for that run

`Runs::any_going(place)` lists the checkout's numbered run directories
and answers whether a supervisor holds any of them. While one does, the
adoption leaves `scratch` as it is, and the run beside it is given the
former scratch. The going run's sandbox writes only the path it was
given, and a rename would take its files from under it. The first run
with none going moves it.

The list is read only for a record naming a scratch outside the user's
directory, once in the checkout's life, so a run of a checkout made
since reads none.

### A candidate's and a preview's checkout made before keep their scratch

`materialize` releases a candidate's checkout of the same name before it
makes one, so a candidate made before is made again with the short
scratch. A preview's record is given back as it is (100), its command
having run, and its stack may keep in its scratch what its release
reads. Both keep their scratch where their record names until released.

### The landing's socket test

`a_landing_under_the_sandbox_binds_a_socket_in_a_folder_under_its_scratch`
in `crates/sluice/tests/exec.rs` sets the branch's policy to the sandbox
executor and a check that fails unless `$SLUICE_TMP` is under
`user_scratch()` and a socket at
`$SLUICE_TMP/x/sessions/work/x-client.sock` is shorter than 104 bytes
and binds. It submits from a worktree at a path longer than 255 bytes,
with no session run first, so no pass of a session run stands for the
candidate's, and waits for the landing. Its binary is given no
`SLUICE_USER_SCRATCH`, so the candidate's scratch is under the real
`/tmp/sluice-<uid>`. Its name holds `sandbox`, so it runs in
`cli:sandbox` alone, outside any sandbox, as the exec test beside it
does.

## Risks / Trade-offs

- **Two branches' runs of one worktree started at once.** The branch's
  lock orders two `sluice exec` of one branch. One of `main` and one of
  another branch in the same worktree are ordered by nothing. Where the
  second adopts between the first's run directory being made and its
  supervisor taking the lock, which lasts while the first's supervisor starts,
  the second moves the scratch the first is about to be given. The first
  run's checks then write a path that is gone and fail, recorded as a
  fail. This happens only once per checkout, on its first run after the
  update, and only with two branches started in one worktree in the same
  instant.
- **A worktree's last stored release names the former scratch.** Where
  the run after the move fails to start, the last run's stored release
  still names the former `SLUICE_TMP`. A release with no policy, or one
  for a worktree since removed, runs in the new scratch with the former
  `SLUICE_TMP` named. A release that fails is kept and reported (123).
  The next run that starts stores its own.
- **A host where `/tmp` is another file system.** The former scratch's
  contents are removed, not moved. A stack a check handed off and whose
  release reads files in the scratch may fail to release. The release
  fails and is kept (123).

## Migration Plan

None beyond the update. A host whose sluice carries this change moves
each worktree's scratch at its next run. A host with an older sluice
reads the record's new path as any record's.
