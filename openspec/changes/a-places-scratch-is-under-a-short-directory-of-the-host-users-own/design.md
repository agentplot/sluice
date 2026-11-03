# Design

## Context

See proposal.md for why, and record 0063 as amended for the decision and
the options weighed. What is there today (sluice main, fb620e26):

- `crates/sluice/src/profiles/places.rs`. `Places::new(common, runs)`
  holds the repository's git common directory, canonical as
  `LocalRefs::common_dir` gives it. `root()` is `<common>/sluice`.
  `new_scratch(name)` is `root()/.tmp/<scratch_name(name)>`, 10 hex digits
  of the SHA-1 of the place's name made in process with
  `gix::hash::hasher`. `Made::scratch` names it on the record's first
  write, in `materialize`, `adopt` (for a record it creates) and
  `preview`. `scratch(name, made)` reads the record's path, else the
  former `root()/.tmp/<dir_name(name)>`. The directory is made with
  `create_dir_all` in `materialize` and `preview` (after removing one
  left there), in `adopt` at every run, and in `release_gone` before the
  stored release runs in it. `let_go` and `release_preview` remove the
  path the record names.
- The sweep, `Checkout::remove_unnamed_scratch`: `list_scratch` lists
  `root()/.tmp/`, returning early with nothing read when it is empty
  (129); `named_scratch` reads every record under `.places/` and
  `.previews/` into `Named`, the file names of the scratch they name,
  with `former` and `short` set where a record names none or cannot be
  read and its place's name cannot be read back; `remove_unnamed`
  removes each listed directory whose file name `Named::keeps` does not
  keep. Names alone are compared, "however a process spells its path".
  The core calls it at the end of `release_abandoned`
  (`crates/sluice-core/src/session.rs`).
- `crates/sluice/src/profiles/checks.rs`. `environment` sets
  `SLUICE_TMP` to `Place::tmp`; a sandbox run sets `TMPDIR` to it too.
  `sandbox_profile` makes `canon(&place.tmp)` writable, and `seatbelt`
  allows binding, accepting and connecting a Unix socket under each
  writable path unless the network is denied. `choose` refuses
  `kind = "container"`: the container executor is not built.
- Tests that drive the binary leave a place's scratch inside the test
  repository's own git directory, removed with the test's scratch
  directory.

## Goals / Non-Goals

**Goals:**

- One scratch path length on a host for every checkout of every
  repository: 45 bytes on macOS, 38 on Linux for a uid of four digits.
- No directory of another user's, or one others may enter, ever holds a
  scratch.
- A sweep that never weighs another repository's scratch.
- Every place made before keeps working and is released where its
  record says.

**Non-Goals:**

- Changing the sandbox profile, `SLUICE_TMP`'s name or what a check may
  write.
- The container executor and the Linux sandbox.
- Moving a held worktree's scratch.

## Decisions

### The user's directory is `/tmp/sluice-<uid>`, resolved

`user_scratch()` is `canonicalize("/tmp")` joined with
`sluice-<uid>`, the uid the effective user id
(`nix::unistd::geteuid`), so `/private/tmp/sluice-501` on macOS and
`/tmp/sluice-1000` on Linux. It is resolved because Seatbelt matches
resolved paths: the record then names the path the sandbox's rule
names, and `SLUICE_TMP`, `TMPDIR` and a socket a check binds under
them all match it. It is made from nothing a process's environment
says, so every process of the user on the host, a session's, a
service's, a cron job's, finds the same directory; only a test names
another (below).

Alternatives (record 0063 as amended):

- `$XDG_RUNTIME_DIR/sluice` where it is set. The XDG Base Directory
  specification keeps that directory for small files, lets it live in
  memory with a size limit and be cleaned, and leaves it unset outside a
  login session, so a session's `sluice exec` and a service's
  `sluice run` would disagree on the repository's scratch, and builds
  and clones written to a check's `TMPDIR` would fill it.
- `$TMPDIR`: 49 bytes on macOS before anything of sluice's.
- The host's state directory: grows with the home's path.

### The directory is made and checked as the user's own

One function, `make_scratch(&self, path)`, makes every scratch, replacing
the four `create_dir_all` calls. For a path under the user's directory
it:

1. makes the user's directory with mode `0o700`
   (`std::os::unix::fs::DirBuilderExt::mode`); `AlreadyExists` is no
   error, since another process of the user may have made it first;
2. reads it with `symlink_metadata` and refuses it unless it is a
   directory, not a link, owned by the user, with no bit of `0o077` set;
3. makes the repository's directory under it the same way;
4. makes the scratch itself with `create_dir_all`.

A path under the repository's `sluice/.tmp/`, a place made before, is
made with `create_dir_all` as now.

The check in step 2 is a pure function, `own_dir(kind, owner, mode,
uid) -> Result<(), &'static str>`, so its refusals are unit tested with
no other user and no root: "is a link", "is not a directory", "belongs to
another user", "may be entered by others". The refusal, an error of the
place's making, reads
`<path> is not a directory of this user's own, closed to everyone else: <what>; remove it, and sluice makes it again`,
and nothing of the directory is changed or removed. A mode the user
widened is refused too, never narrowed: sluice changes nothing it finds,
as a terminal multiplexer refuses its sockets' directory.

`materialize` and `preview` still remove a scratch left at the record's
path before making it.

### The repository's key is a digest of where the repository is

`repository_key(common)`: the first 10 hex digits of the SHA-1 of the
canonical common directory's path as bytes (`OsStrExt::as_bytes`),
made in process with `gix::hash::hasher`, as `scratch_name` is: no
process is started for it (129). `Places` computes it once, in a
`OnceCell` beside `repo`. The repository's directory is
`user_scratch()/<key>`, and `new_scratch(name)` is
`<that>/<scratch_name(name)>`.

A key kept in the repository's state would be copied with the
repository, and each copy's sweep would remove the other's live
scratch; a digest of the path gives two copies two keys. A repository
moved gives its next places a new key; its existing records still name
their old paths.

`places.rs` exports `pub fn repository_scratch(common: &Path) ->
Result<PathBuf>`, the repository's directory under the user's, and
`pub fn repository_key`, for the tests' fixture to remove what a test's
repository left there and under the tests' own directory (below).

### Where the user's directory comes from, in tests

`Places::with_scratch_root(dir)`, beside `with_blocks`, names the
directory standing for `/tmp/sluice-<uid>`. The profile tests and the
unit tests use it, so a test that makes it a link or opens it to others
touches no real directory.

sluice's gate runs its tests under the sandbox, which lets a check
write its checkout, its scratch and the host's caches and nothing else,
and a binary a test starts runs under it too: there `/tmp/sluice-<uid>`
cannot be made (`creating /private/tmp/sluice-501: Operation not
permitted`). So the binary reads `SLUICE_USER_SCRATCH`, the tests' way
to name the directory standing for the user's, as `SLUICE_ENGINE_SOCKETS`
is theirs to name a container engine: where it is set, the binding hands
it to `with_scratch_root`. A host sets none, and no page names it.
`support::isolated` sets it on every sluice a test starts, to
`sluice-test-<uid>` in the tests' temporary directory, resolved, which
every test process of the user shares as a host's processes share the
user's directory, each repository under its own key. The tests that
build `Places` beside the binary, the stack's and the conformance
suite's, hand the same directory to `with_scratch_root`.
`support::Repo` removes its repository's directory there, and
`repository_scratch(<its .git>)`, when it is dropped, so a test leaves
nothing in either.

The path a host gets is proven where nothing confines the tests: by the
unit test of `user_scratch`, and end to end by the sandbox scenario of
`sluice exec` in `cli:sandbox`, run outside any sandbox, whose binary is
given no `SLUICE_USER_SCRATCH` and makes its scratch under the real
`/tmp/sluice-<uid>`.

Alternatives:

- The tests that drive the binary at the real `/tmp/sluice-<uid>`:
  refused under the gate's sandbox, as above.
- The sandbox letting a check write the user's directory: every check
  could then write every other checkout's scratch on the host, and the
  sandbox the gate runs under is the installed sluice's, not the one
  under test.

### The sweep lists two roots and compares names

`list_scratch` lists the repository's directory under the user's and
the former `root()/.tmp/`, each as it is now. Before listing the first,
it checks the user's directory with `own_dir` where it exists; where it
is refused, the sweep warns with the same words, lists nothing under it
and goes on to the former root. It never lists the user's directory
itself, so another repository's directory is never read. When both
listings are empty, no record is read (129).

`named_scratch` and `Named::keeps` are unchanged: a listed directory is
kept when any record names a scratch of its file name, in either root.
A record naming a scratch under the user's directory and one naming the
same 10 hex digits under `.tmp/` are the same checkout's name made at
two times, so keeping both while either record stands keeps nothing
another checkout needs, and the former one goes once its name is free.
Comparing whole paths would need every process to spell a record's path
alike, which records written by older sluices do not promise.

### The sandbox and the check seam change nothing

`checks.rs` reads the scratch only as `Place::tmp`. With the record
naming the resolved path, `canon(&place.tmp)` in `sandbox_profile` is
that path, the Seatbelt profile's `file-write*` and `unix-socket` rules
name it, and `SLUICE_TMP` and `TMPDIR` are it. A check that makes
`$SLUICE_TMP/x/sessions/work/` and binds `x-client.sock` there binds a
path of 75 bytes on macOS, under the profile as written. The grant of a
sandbox run (138; the change `a-checks-key-holds-its-executor`), where
it is built, writes the scratch by its role, so the move gives every run
the grant it had.

The container executor is not built. Its row in C.4 mounts "a scratch
directory"; when a change builds it, that is the scratch the checkout's
record names. A run with `docker = true` under the sandbox may hand its
scratch's path to a container engine; an engine whose machine shares
only the user's home sees an empty folder there, which the configuration
page says.

### A place made before keeps its scratch

`scratch(name, made)` is unchanged, so a record naming
`<common>/sluice/.tmp/<10 hex>` keeps that path for every run, release
and gone release, and a record naming none keeps the former name. A
worktree's record is rewritten at each run with its `scratch` as it was,
never replaced: `adopt` sets `scratch` only on a record it creates. After
`sluice release` removes the record, the worktree's next run creates
one naming the user's directory.

### The words a reader sees

`site/src/content/docs/reference/state.md` gives the scratch its own
block after the repository's: `/tmp/sluice-<uid>/<repository code>/<code>/`,
`/private/tmp` on macOS, made the user's own, emptied when the host
restarts, and the repository's `.tmp/` kept for checkouts an older
sluice made. `configuration.md`'s `SLUICE_TMP` row says the path is short
so a socket a tool keeps in a folder under it fits, that a worktree keeps
its own across runs until the host restarts, and that an engine must
share `/tmp` for a container to see it. Neither names a host, a tool or a
team of ours (rule core.26).

## Risks / Trade-offs

- **`/tmp` is emptied** when a host restarts, and some systems clean it
  of what nobody touched for days. A worktree's scratch then loses what
  its checks kept between runs, and is made again at its next run; a
  check keeps nothing in its scratch it cannot make again. The
  configuration page says so.
- **Another user takes the name first.** On a host with other users, a
  directory named `/tmp/sluice-<uid>` that is not the user's stops every
  place of that user from being made until it is removed. The refusal
  says which path and why; sluice never works around it, since a
  scratch in another's directory is one they can read and change.
- **`/tmp` mounted with no programs allowed to run.** A check that runs
  a program it built under `SLUICE_TMP` fails there. On Linux, the open
  executor sets no `TMPDIR`, so only a check that uses `SLUICE_TMP`
  itself meets it; the Linux sandbox is not built. Out of scope until a
  host needs a setting.
- **Two users on one repository.** A scratch is now under the user who
  made the place; another user's sluice cannot remove it, as it already
  cannot remove that place's checkout, whose files are its maker's.
  Each user releases the places they made.
- **Two repositories with one key.** 40 bits of a digest of two paths;
  with the few repositories a host holds, a shared key is unlikely past
  any host's life. Were it to happen, each sweep would keep what any of
  its own records names and could remove the other's live scratch; the
  key can be lengthened then, since records name their paths.
- **A host with sluices of two versions.** An older sluice reads a newer
  record's `scratch` as any record's and runs in it; its sweep lists only
  `sluice/.tmp/`. The newer sluice sweeps `.tmp/` by the rules the older
  one keeps. Neither removes the other's.

## Migration Plan

Nothing to migrate: records name their paths. A host takes the new path
for the places it makes once its sluice is updated; a worktree whose
place was made before takes it after `sluice release`. Going back to an
older sluice leaves scratch under `/tmp/sluice-<uid>` that the older
sluice uses through its records and never sweeps; a restart empties it.
