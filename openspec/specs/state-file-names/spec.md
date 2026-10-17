# state-file-names Specification

## Purpose

How a checkout's name, a worktree's path above all, becomes the one file
or directory name sluice keeps its state under: short enough for macOS
and Linux, distinct for every name, and unchanged for every name that
already fit.

## Requirements

### Requirement: A worktree at any path can run

`sluice exec` SHALL run, record its run and keep its worktree's state
from a worktree whose path is longer than 255 bytes, on macOS and on
Linux, and every file name it writes for that worktree SHALL be at most
255 bytes.

#### Scenario: A worktree at a long path

- **WHEN** a worktree's path is longer than 255 bytes and `sluice exec`
  runs a command from it
- **THEN** the command runs and passes, the worktree is listed by
  `sluice status --json` under its whole path, and no file name under
  the repository's state is longer than 255 bytes
  (`exec_runs_from_a_worktree_at_a_long_path` in
  `crates/sluice/tests/exec.rs`)

### Requirement: A name that fits keeps its file name

A checkout's name that fits SHALL be written as the same file name an
older sluice wrote for it, so its record, ports and runs are found where
they are, with nothing moved.

#### Scenario: A short name

- **WHEN** the file name of `sluice/try/main/39` or of a worktree at a
  short path is made
- **THEN** it is exactly the name made before this change
  (`a_name_that_fits_is_kept` in `crates/sluice/src/runs.rs`)

### Requirement: A long name is shortened, readable and distinct

A checkout's name too long to keep SHALL be written as the start of its
file name, a mark that no kept name holds, and a hash of the whole name:
the same every time for one name, different for two names, however
little they differ.

#### Scenario: Two long paths that differ at the end

- **WHEN** the file names of two worktrees whose paths are longer than
  255 bytes and differ only in their last character are made
- **THEN** each is at most 246 bytes, begins with the start of the
  name, differs from the other, and is the same when made again
  (`a_long_name_is_shortened_and_distinct` in
  `crates/sluice/src/runs.rs`)
