# Phase 1, pass 2: the real profiles

How each real integration works, written so pass 2 can be split among
agents, one section each. Every command, API and output below was run
on this host on 2026-10-11, against the tools as installed; captured
output is pasted as it came, trimmed where marked `…`. Read the brief
(`docs/plans/phase-1-core.md`), the walkthrough (`docs/examples/README.md`),
the statement (agentplot/blueprints `design/sluice/requirements.md`) and
`docs/architecture/core.md` first; this document cites their clauses and
rules and does not restate them.

## What is installed

| tool | version | where | note |
|---|---|---|---|
| git | 2.54.0 | `/etc/profiles/per-user/chuck/bin/git` | also `/usr/bin/git` 2.50.1 (Apple Git-155), which some tools use: see *Branch protection* |
| gix | 0.89.0 on crates.io; the APIs below were read in 0.87.1 (gix-ref 0.67.1, gix-actor 0.42.0) | `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` | depend on 0.89 and re-check the calls section 1 names |
| moon in sluice's shell | 2.6.1 | `<common>/proto/tools/moon/2.6.1/moon` | its version written in `.prototools`, installed by the devenv task `sluice:proto:install` on entering the shell (never by `enterShell`); `devenv shell -- moon --version` |
| proto in sluice's devenv | 0.63.1 | `devenv.nix`, moonrepo's release build | moonrepo's toolchain manager, the one application tool devenv holds: installs and runs the application toolchain `.prototools` pins (Rust 1.98.1, through rustup), which `.moon/toolchains.yml` reads; `PROTO_HOME` and `MOON_HOME` are the repository's (`<common>/proto`, `<common>/moon-home`) |
| moon on `PATH` | 1.41.7 | `~/.bun/bin/moon` | **cannot run crew**: no task tags, no `moon exec` |
| moon in crew's devenv | 2.5.3 | `/nix/store/xn5qbl6xwpb2d9hlchsjzacx6qkgcrml-moon-2.5.3/bin/moon` | what crew's `tests/suite` runs; below the profile's floor until crew moves to 2.6 |
| worktrunk | v0.80.0 | `/etc/profiles/per-user/chuck/bin/wt` | `wt` in zsh is a function wrapping the binary; call `command wt` or the path |
| rustc | 1.98.1 | | `std::fs::File::lock`/`try_lock` are stable (1.89), so no lock crate is needed |

The moon profile's floor is **moon 2.6**. 2.6.0 added a task
dependency's `type` (2.6.1's `.moon/cache/schemas/project.json`,
`TaskDependencyConfig.type`): `required` (the default) runs before the
task and must pass; `wait` runs before it, but the task waits only for
the dependency to start, not to finish; `cleanup` runs after the task's
command, even when it failed, but not when it was skipped or replayed.
Verified in a scratch project whose `test` has a `wait` on a `serve`
that writes a file and sleeps 2 s, and a `cleanup` on a `stop`: `test`
read the file 0.5 s in, failed on purpose, and `stop` still ran. These
are the walkthrough's "wait and cleanup dependencies": how a
repository's checks bring up and take down what they need in the
place (31), never sluice's. A dependency of either kind must be able to
run where its task does: with `cache: false` and no `runInCI`, moon
refused the graph (`task_builder::dependency::run_in_ci_mismatch`).

## Shared conventions

The seam traits are those of `crates/sluice-seams/src/lib.rs` as of pass
1's commit `1fcbe46`: `Checkout` (`materialize`, `release`), `Tests`
(`start`, `progress`, `bound`), `Branch` (`read`, `read_file`,
`form`, `commit`, `contains`, `move_to`), `Driver` (`session`,
`notify`) and `State` (`lock`, `read`, `write`, `branches`,
`allocate_id`, `results`, `record_results`). A profile implements one
trait and nothing of the core (core.2). Each section names the methods it
implements.

Everything sluice keeps lives under `<common>/sluice/`, where `<common>`
is the repository's git common directory (*State on disk* says how to
find it). A branch's records are `<common>/sluice/<branch>/`. What
belongs to the repository rather than a branch is named with a leading
dot, because no component of a git ref name may start with `.` (`git
check-ref-format`), so no branch's directory can ever collide with it:

```
<common>/sluice/
  results.jsonl              69: every check's outcome under its key (pass 1's name)
  .next-id  .lock            the repository's submission counter and the lock that guards it
  .runs/<place>/             one check run: supervisor lock, pgid, result.json, runReport.json, moon.log
  .places/<place>/           git-worktree checkouts
  .tmp/<place>/              SLUICE_TMP of each checkout
  bolt/concurrent-units/     a branch's records (State on disk)
```

`<place>` is the name the core chooses for a place (`Checkout::materialize`
takes it), so a place, its run and its scratch directory are found again
by name after a crash (25, 34, core.14).

Rust crates the profiles add to the workspace: `gix = { version = "0.87",
default-features = false, features = ["max-performance-safe", "sha1",
"basic", "revision"] }` (flywheel-store-git's set without
`blocking-network-client`), `nix = { version = "0.31", default-features
= false, features = ["signal", "process", "hostname"] }`. Nothing else
outside what pass 1 uses.

Every external command sluice runs gets `stdin` from `/dev/null`: `moon
query tasks --affected` reads changed files from stdin and hangs forever
on a pipe that never closes.

---

## 1. Branch, through gix and git

**Implements** `Branch::read`, `read_file`, `form`, `commit`, `move_to`
in a `local-refs` profile (C.4). **Bears on** 10, 12, 23, 24, 42 to 46,
56, 64, 65; core.4, core.5, core.6, core.15; ADR 0006, ADR 0009.

**Inputs and outputs.** `read(branch) -> Option<Rev>`; `read_file(rev,
path) -> Option<bytes>` (the policy, 56); `form(onto, head) ->
Formed::Revision(candidate commit) | Formed::Conflict { paths }`;
`commit(LandingCommit { tree_of, parent, head, message }) -> Rev`;
`move_to(branch, expected, to) -> Moved::Moved | Moved::Elsewhere(found)`.

### Reading, writing commits, moving the ref: gix

Patterns to copy from flywheel-next, `crates/flywheel-store-git/src/`:

| what | where |
|---|---|
| open | `objects.rs:15-16`, `gix::open(dir)` |
| read a ref | `objects.rs:66-85`: a full name by `repo.find_reference(name)?.peel_to_id()`; a short one by `repo.rev_parse_single(..)` (`:75`); absent is `Err(gix::reference::find::existing::Error::NotFound{..})` (`:81`) |
| write a commit | `objects.rs:411-442` `write_commit`: a `gix::objs::Commit { tree, parents, author, committer, encoding: None, message, extra_headers: vec![] }` written by `repo.write_object(&commit)?.detach()`, the signature fixed by hand, never read from git config |
| move from an expected value | `objects.rs:446-475` `set_ref`: `RefEdit { change: Change::Update { log: LogChange { mode: RefLog::AndReference, force_create_reflog: false, message }, expected: PreviousValue::MustExistAndMatch(Target::Object(old)), new: Target::Object(new) }, name, deref: false }` then `repo.edit_reference(..)` |
| common dir | `git.rs:140,192`, `repo.common_dir()` |
| the git command beside gix | `git.rs:72-108` `Repo::run`: `Command::new("git")` with `GIT_AUTHOR_*`/`GIT_COMMITTER_*` pinned and `GIT_CONFIG_NOSYSTEM=1`; used only where gix has no equivalent (`git.rs:1-9`) |

The gix 0.87.1 API, checked in the registry source. The profile is
built on gix 0.89.0, where these names hold except the errors (below):

| item | file:line |
|---|---|
| `PreviousValue::{Any, MustExist, MustNotExist, MustExistAndMatch(Target), ExistingMustMatch(Target)}` | `gix-ref-0.67.1/src/transaction/mod.rs:44-54` |
| `Change::Update { log, expected, new }`, `Change::Delete` | `gix-ref-0.67.1/src/transaction/mod.rs:59-63` |
| `RefEdit { change, name, deref }`, `RefLog::{AndReference, Only}`, `LogChange` | `…/transaction/mod.rs:114, 126, 21` |
| refusal `ReferenceOutOfDate { full_name, expected, actual }`; `MustExist`; `LockAcquire` | `gix-ref-0.67.1/src/store/file/transaction/prepare.rs:507, 505, 488` |
| `Repository::edit_reference`, `edit_references`, `edit_references_as(edits, Option<SignatureRef>)` | `gix-0.87.1/src/repository/reference.rs:136, 147, 157` |
| `find_reference`, `try_find_reference` (`Option`) | `gix-0.87.1/src/repository/reference.rs:323, 370` |
| `write_object`, `new_commit_as` (writes only), `commit_as` | `gix-0.87.1/src/repository/object.rs:250, 499, 361` |
| `rev_parse_single` (feature `revision`) | `gix-0.87.1/src/repository/revision.rs:42` |
| `common_dir()` | `gix-0.87.1/src/repository/location.rs:35` |
| `gix::reference::edit::Error::FileTransactionPrepare(prepare::Error)` | `gix-0.87.1/src/reference/errors.rs:9-22` |
| `gix::actor::Signature { name, email, time }`; `.to_ref(&mut gix::date::parse::TimeBuf::default())` | `gix-actor-0.42.0/src/lib.rs:85, 106` |

Use `MustExistAndMatch` for a move. `ExistingMustMatch` also passes when
the ref is absent, so a deleted branch would be recreated. Do not use
`commit_as`/`commit`: they also update a reference with
`ExistingMustMatch(parent0)` (`object.rs:419-427`). Write the object, then
move the ref as its own step: the landing intent is written between the
two (24).

This program, built against gix 0.87.1, wrote a landing and moved a
branch twice from the same expected value:

```rust
// usage: land <repo> <line-ref> <head> <tree> <expected-old>
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use gix::refs::Target;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let repo = gix::open(&a[1]).expect("open");
    let line = repo.find_reference(a[2].as_str()).expect("find").peel_to_id().expect("peel").detach();
    let head = repo.rev_parse_single(a[3].as_str()).expect("head").detach();
    let tree = gix::ObjectId::from_hex(a[4].as_bytes()).expect("tree");
    let expected = gix::ObjectId::from_hex(a[5].as_bytes()).expect("expected");
    let who = gix::actor::Signature { name: "sluice".into(), email: "sluice@localhost".into(),
        time: gix::date::Time { seconds: 1_791_710_521, offset: 0 } };
    let message = format!("Land #12: unit/touches\n\nSluice-Submission: 12\nSluice-From: crw-1-unit-1 (unit/touches)\nSluice-Tested: {tree}\nSluice-Result: passed (2 run, 31 replayed)\n");
    let commit = gix::objs::Commit { tree, parents: [line, head].into_iter().collect(),
        author: who.clone(), committer: who.clone(), encoding: None, message: message.into(), extra_headers: vec![] };
    let landed = repo.write_object(&commit).expect("write").detach();
    let edit = |from: gix::ObjectId| RefEdit {
        change: Change::Update {
            log: LogChange { mode: RefLog::AndReference, force_create_reflog: false, message: "sluice: land #12".into() },
            expected: PreviousValue::MustExistAndMatch(Target::Object(from)),
            new: Target::Object(landed) },
        name: a[2].as_str().try_into().expect("name"), deref: false };
    match repo.edit_references_as([edit(expected)], Some(who.to_ref(&mut gix::date::parse::TimeBuf::default()))) {
        Ok(done) => println!("moved: {:?}", done[0].change),
        Err(e) => { println!("refused (Display): {e}"); println!("refused (Debug): {e:?}"); std::process::exit(3) }
    }
}
```

```
moved: Update { log: LogChange { mode: AndReference, force_create_reflog: false, message: "sluice: land #12" }, expected: MustExistAndMatch(Object(Sha1(d86635e0...))), new: Object(Sha1(cecaa674...)) }

refused (Display): The reference "refs/heads/bolt/x" should have content d86635e0fcf688502279dea71d149010a9607a17, actual content was cecaa6748c58910c52b047e9ccd82c83586966ed
refused (Debug): FileTransactionPrepare(ReferenceOutOfDate { full_name: "refs/heads/bolt/x", expected: Object(Sha1(d86635e0…)), actual: Object(Sha1(cecaa674…)) })
```

```
$ git cat-file -p bolt/x
tree 67ca456f3041b4cea7ea0b93d2d1b1c8bee03f42
parent d86635e0fcf688502279dea71d149010a9607a17
parent 317b4376c789dcdfc393131585987e9343de1cdd
author sluice <sluice@localhost> 1791710521 +0000
committer sluice <sluice@localhost> 1791710521 +0000

Land #12: unit/touches

Sluice-Submission: 12
Sluice-From: crw-1-unit-1 (unit/touches)
Sluice-Tested: 67ca456f3041b4cea7ea0b93d2d1b1c8bee03f42
Sluice-Result: passed (2 run, 31 replayed)
$ git reflog bolt/x
cecaa67 bolt/x@{0}: sluice: land #12
d86635e bolt/x@{1}: commit: other
```

`git log -1` shows `Merge: d86635e 317b437` and `Author: sluice
<sluice@localhost>`; `git interpret-trailers --parse` returns the four
`Sluice-*` lines exactly. The walkthrough's `Sluice-Tested` names the
candidate commit, not its tree: pass the candidate's id there (the
experiment used the tree only because it had no candidate commit).

`move_to` maps the errors to the seam's values. From gix 0.89 (gix-ref
0.69) a refusal is a `gix::Error` (`gix_error`) carrying the cause; the
profile (`crates/sluice/src/profiles/local_refs.rs`) recovers it as
below, and the acceptance tests hold it to these values:

| refusal | meaning | `move_to` returns |
|---|---|---|
| `e.downcast_any_ref::<gix::refs::file::transaction::prepare::ReferenceOutOfDate>()` with `actual` | the branch is elsewhere | `Ok(Moved::Elsewhere(Some(actual)))` (24, core.5: reported, never retried blind) |
| `e.classify().is_not_found()` | the branch was deleted | `Ok(Moved::Elsewhere(None))` |
| anything else, as `<ref>.lock` held by git or another gix | transient | `Err`: the step fails and the next one tries again |

Lock interplay, measured: with `refs/heads/bolt/x.lock` present gix
retried for about 0.73 s and failed with `LockAcquire { source:
PermanentlyLocked { mode: AfterDurationWithBackoff(100ms), attempts: 8 } }`;
gix reads git's own `core.filesRefLockTimeout` (100 ms default;
`gix-0.87.1/src/config/cache/access.rs:236-240`). git itself fails at once
(`cannot lock ref … File exists`, exit 128). Both use the `<ref>.lock`
protocol, so they serialize against each other (46).

A refused move leaves the landing commit written and unreferenced; `git
gc` prunes it after `gc.pruneExpire` (two weeks). Nothing needs it.

**gix runs no hooks.** The gix 0.87.1 and gix-ref 0.67.1 sources contain no
`reference-transaction` or `hooksPath` code. With a logging
`reference-transaction` hook installed, `git branch -f` logged
`preparing`, `prepared` and `committed`; the gix move logged nothing, with
or without `core.hooksPath`; and with a hook that refuses everything, `git
update-ref` failed (`fatal: in 'prepared' phase, update aborted by the
reference-transaction hook`, exit 128) while the gix move of the same ref
succeeded. *Branch protection* builds on this.

### Common dir and opening

From a linked worktree, `repo.common_dir()` is
`/…/repo/.git/worktrees/linked/../..`, not normalized; canonicalize it
(`std::fs::canonicalize`) before using it as a key or printing it. From
the shell, `git rev-parse --git-common-dir` is relative (`.git`,
`../.git`) in the main checkout and absolute in a linked one; always pass
`--path-format=absolute`.

### Forming: `git merge-tree --write-tree`

ADR 0009: a candidate conflicts exactly when `git merge` would say so, so
forming runs the git command. Run it with full object ids (never branch
names; see the error case), from the common dir or any worktree:

```
git merge-tree --write-tree -z --name-only --no-messages <onto> <head>
```

Exit 0 is a clean merge and stdout is the tree id. Exit 1 is a conflict:
stdout is the tree id with conflict markers written into it, then the
conflicted paths. Captured, in a scratch repository on git 2.54.0:

```
== clean
$ git merge-tree --write-tree bolt/x unit/touches
d3f7d6356d7e3f4b2e228cfa89b5dff601fb771f
exit=0

== content conflict (default output)
$ git merge-tree --write-tree unit/touches unit/batch
ac8a378447024536213b6fe7b755de3d3842d607
100644 de980441c3ab03a8c07dda1ad27b8a11f39deb1e 1	plan.py
100644 778a77e347932f464071cffa042308d5afd20fdc 2	plan.py
100644 73380522dc6127cfdf6cb5bb97627ea37502d147 3	plan.py

Auto-merging plan.py
CONFLICT (content): Merge conflict in plan.py
exit=1

== --name-only
ac8a378447024536213b6fe7b755de3d3842d607
plan.py

Auto-merging plan.py
CONFLICT (content): Merge conflict in plan.py
exit=1

== -z --name-only --no-messages  (od -c)
a c 8 a 3 7 8 … d 6 0 7 \0 p l a n . p y \0

== modify/delete
e15d1b8d7f6150c7f6c6b68ecb825a6bd6484fef
100644 2fa992c0b8b5c6acd2bdd4fa31de29d29799bdd5 1	del.txt
100644 5ea2ed416fbd4a4cbe227b75fe255dd7fa6bd4d6 2	del.txt

CONFLICT (modify/delete): del.txt deleted in unit/del and modified in unit/moddel.  Version unit/moddel of del.txt left in tree.
exit=1

== rename/rename
b5a1828c04c3e70bb7755bfb636436c856478960
100644 0ec1772728d194cd451203c28b1d4184aa1575cc 2	new.txt
100644 0ec1772728d194cd451203c28b1d4184aa1575cc 1	old.txt
100644 0ec1772728d194cd451203c28b1d4184aa1575cc 3	other-name.txt

CONFLICT (rename/rename): old.txt renamed to new.txt in unit/ren and to other-name.txt in unit/ren2.
exit=1

== -z with messages (NUL shown as |)
ac8a…d607|100644 de98… 1	plan.py|100644 778a… 2	plan.py|100644 7338… 3	plan.py||1|plan.py|Auto-merging|Auto-merging plan.py
|1|plan.py|CONFLICT (contents)|CONFLICT (content): Merge conflict in plan.py
|
== rename -z --name-only
b5a1…8960|new.txt|old.txt|other-name.txt||3|old.txt|new.txt|other-name.txt|CONFLICT (rename/rename)|CONFLICT (rename/rename): old.txt renamed …
```

Parsing, for `-z --name-only --no-messages`: split stdout on NUL; field 0
is the tree; fields 1.. up to the first empty field are the conflicted
paths, each once. Those paths are `Formed::Conflict { paths }` (12, 43,
65). With `-z` and messages, the section after the empty field is
records of `<n>\0<path>…\0<type>\0<message>\0`, where `<type>` is a
stable string (`Auto-merging`, `CONFLICT (contents)`, `CONFLICT
(rename/rename)`, `CONFLICT (modify/delete)`); keep `--no-messages` unless
a CLI line needs the reason. A modify/delete or rename conflict names
every path involved (`old.txt`, `new.txt`, `other-name.txt`), and those
are the "content contested" of 65.

Exit codes are not enough on their own:

```
$ git merge-tree --write-tree bolt/x nosuch
merge-tree: nosuch - not something we can merge
exit=1                                    # an error, with the conflict's exit code and empty stdout
$ git merge-tree --write-tree bolt/x orphan
fatal: refusing to merge unrelated histories
exit=128
```

So: resolve both sides to ids through gix first; read exit 1 as a
conflict only when stdout's first field is a 40-hex id; anything else,
and any exit above 1, is an `Err`. Unrelated histories are an error, not
a conflict (`--allow-unrelated-histories` is not for sluice).

`--quiet` exits 0/1 and writes no tree; it is no use, since a clean
candidate needs its tree. `--stdin` batches merges but reports a clean
merge as `1` and a conflict as `0` in its status field and always exits
0; avoid it.

**Forming a chain.** A candidate must be a commit: moon's `MOON_HEAD`,
`git worktree add` and `wt switch --base` all need one, and the next
candidate's merge base comes from its history. `form(onto, head)` is:

```
T=$(git merge-tree --write-tree -z --name-only --no-messages <onto> <head>)   # exit 0
C=$(git commit-tree <T> -p <onto> -p <head> -m "sluice candidate")            # env: GIT_{AUTHOR,COMMITTER}_{NAME,EMAIL}=sluice / sluice@localhost
```

(or the tree from merge-tree and the commit through gix's `write_object`,
as in the program above, with message `sluice candidate`). The next
candidate is `form(C, next_head)`. Captured:

```
C1=4ff5ce7…   # bolt/x + unit/clean
T2=7b796b6…   # merge-tree C1 unit/touches, exit 0
C2=0b8dc20…   # commit-tree T2 -p C1 -p unit/touches
$ git merge-tree --write-tree --name-only --no-messages $C2 unit/batch
946bd9c9e72faea04427672153aefe3fe9af6798
plan.py
exit=1
$ git switch --detach $C2 && git merge --no-edit unit/batch    # does git merge agree?
git merge exit=1; conflicted: plan.py
```

`--merge-base=<base> <tree1> <tree2>` gave the same tree as the commit
chain here, but it forms from trees with a base sluice picks, so it is not
"what `git merge` would report" when history has several merge bases.
Commits are the reference; use them.

A candidate commit is not the landing commit. The landing (`commit`)
writes, per entry, a commit whose tree is the candidate's tree
(`tree_of`), whose parents are the branch's revision before it and the
entry's head, and whose message is the walkthrough's section 8 (23, ADR
0006). Its first parent is the previous landing, not the candidate.
Candidate commits stay unreferenced (the worktrunk checkout's
`sluice/try/<branch>/<n>` branch holds one while it exists); nothing reads them
after the landing but `Sluice-Tested`.

**Forecast (64, 65).** The same `form` over the branch and the entries
ahead, with no records written; it is the core's, through this seam.

**`read_file` (56, core.15).** `repo.find_object(rev)?.peel_to_tree()?`
then `tree.lookup_entry_by_path(path)` and the blob's data, from the
branch's revision only. Never read a file from a worktree.

### Acceptance tests

Against a scratch repository made in a temp dir by the test:

1. `read` of an existing, a missing and a packed ref (`git pack-refs --all`).
2. `form` clean: the candidate's tree equals `git merge-tree`'s; its parents are `[onto, head]`.
3. `form` conflicts: content, modify/delete, rename/rename, add/add; `paths` as captured above.
4. `form` on a chain of three, and the third conflicting only with the first (65's "every entry ahead up to the one at which the conflict appeared" is the core's; the seam returns the paths).
5. `form` with a missing object and with unrelated histories returns `Err`, not a conflict.
6. `commit`: `git cat-file -p` shows both parents, author and committer `sluice <sluice@localhost>`; `git interpret-trailers --parse` gives the `Sluice-*` lines; the tree is `tree_of`'s.
7. `move_to` from the right revision moves the branch and writes a reflog entry; from a stale one returns `Elsewhere(Some(actual))` and leaves the branch; on a deleted branch returns `Elsewhere(None)`; with `<ref>.lock` present returns `Err` (S8; 24, 45, 46).
8. Two threads `move_to` from one expected revision: exactly one moves (46).
9. A `reference-transaction` hook that refuses everything does not stop `move_to` (gix runs no hooks).
10. `read_file` reads the policy from the branch's revision while a worktree holds a different version of it (56, S12).

---

## 2. Branch protection

**Implements** the protection half of C.4's local-refs profile: no seam
method; a `sluice hook reference-transaction <state>` subcommand and an
installer. **Bears on** 1, 2 (actors), I2, S1 ("neither session ran a
merge"); core.4.

### Facts

- git runs `reference-transaction` with one argument, the state:
  `preparing`, `prepared`, `committed` or `aborted`, and one stdin line
  per ref: `<old> SP <new> SP <ref>`. A non-zero exit in `preparing` or
  `prepared` aborts the transaction (`git help githooks`).
- **Decide in `prepared`, never in `preparing`.** In `preparing`,
  symbolic refs are not resolved: a `git merge` or `git commit` on a
  checked-out branch arrives as `HEAD`, and only in `prepared` as
  `refs/heads/bolt/x`. A hook that refused in `preparing` let `git
  merge`, `git commit` and `git reset --hard` move the branch. Captured,
  from `git reset --hard` in a worktree on `bolt/x`:

  ```
  preparing  4fc2eb0… 4fc2eb0… HEAD
  prepared   4fc2eb0… 4fc2eb0… refs/heads/bolt/x
  committed  4fc2eb0… 4fc2eb0… refs/heads/bolt/x
  ```
- `<old>` is all zeros whenever the caller gave no expected value, for a
  forced update of an existing ref as much as for a creation (`git branch
  -f bolt/x …` sent `0000… 08f654f… refs/heads/bolt/x`). Read the ref's
  current value instead (`git rev-parse --verify -q <ref>`, or gix), which
  in `prepared` is still the old one: the lock is held, the ref file is
  not yet replaced.
- No-op updates arrive too: `git worktree add ../bolt bolt/x` runs a
  `reset --hard` that "updates" `refs/heads/bolt/x` from X to X. Allow an
  update whose new value equals the current one.
- Every git command runs it, including `git-receive-pack`, so a push
  into this repository is caught (the parent process was
  `git-receive-pack /…/repo/.`).
- **gix runs no hooks** (section 1), so sluice's own moves never reach
  the hook and need no marker. Anything else that writes refs through gix
  or libgix/libgit2 also bypasses the hook. Protection guards against
  agents running git, which is what they run; it is not a security
  boundary, and an agent can delete the hook. That is acceptable for
  clause 1's "refused and reported" on one host; the forge's rules are the
  boundary on a remote (C.4 pushed refs).
- A refused `git merge`, `git commit` or `git reset --hard` has already
  changed the worktree and index before the ref transaction: the branch
  does not move, but the worktree holding it is left with the merge
  staged (and `MERGE_HEAD`), the commit's changes staged, or the reset's
  files changed. Captured after a refused fast-forward `git merge
  unit/touches` on `bolt/x`: `A  t.txt`, HEAD unchanged. `git merge
  --abort` or `git reset --hard` (a no-op on the ref, so allowed) cleans
  it. The hook's message should say so.

### Config-based hooks are not enough

git 2.54 runs hooks named in config (`hook.<name>.command` plus
`hook.<name>.event`), beside the hookdir's file hook and under any
`core.hooksPath`:

```
$ git config set hook.sluice.command /…/sluice-protect
$ git config set --append hook.sluice.event reference-transaction
$ git hook list --show-scope reference-transaction
local	sluice
hook from hookdir
```

It worked with an existing `.git/hooks/reference-transaction` and with
`core.hooksPath` set: both ran, sluice's refused. **But git 2.53.0 and
Apple's git 2.50.1 ignore config hooks and moved the protected branch
(exit 0)**, and `/usr/bin/git` is 2.50.1 on this host, used by Xcode and
anything with a plain `PATH`. So the file hook is the one to install. The
same `/usr/bin/git update-ref refs/heads/bolt/x main` against the file
hook below was refused (`fatal: ref updates aborted by hook`, exit 128).

### Installing without clobbering

The hookdir is `git rev-parse --path-format=absolute --git-path hooks`
(it follows `core.hooksPath`; for crew's bare layout it is
`crew/crew.git/hooks`; crew sets no `core.hooksPath` and has no
`reference-transaction` hook today). In that directory:

- no `reference-transaction`: write sluice's shim;
- a `reference-transaction` that is sluice's shim (recognized by its
  marker line): rewrite it if the sluice path changed, else nothing;
- any other `reference-transaction`: rename it to
  `reference-transaction.pre-sluice` and write the shim, which runs it
  after sluice's check with the same argument and stdin and fails if it
  fails (verified: the chained hook still logged every state);
- `core.hooksPath` pointing inside a worktree (a tracked hooks directory,
  as husky or lefthook keep): refuse to install and say so, since writing
  there changes a tracked file.

The shim, written with the absolute path of the installing binary
(`std::env::current_exe()`, canonicalized):

```sh
#!/bin/sh
# sluice: refuses updates to branches sluice protects (docs: branch protection). Installed by sluice.
input=$(cat)
d=$(dirname "$0")
if [ -x '/abs/path/to/sluice' ]; then
  printf '%s\n' "$input" | '/abs/path/to/sluice' hook reference-transaction "$1" || exit $?
else
  echo "warning: sluice's branch protection is installed but /abs/path/to/sluice is missing" >&2
fi
[ -x "$d/reference-transaction.pre-sluice" ] || exit 0
printf '%s\n' "$input" | "$d/reference-transaction.pre-sluice" "$@"
```

Missing sluice fails open with a warning: failing closed would refuse
every commit in every worktree of the repository.

`submit`, `step` and `run` ensure it at their start, idempotently, for one
file read; nothing has to be run first. `sluice protect --remove` takes
it out, and `sluice protect` puts it back.

### `sluice hook reference-transaction <state>`

1. Exit 0 at once unless `state == "prepared"`. This hook runs on every
   ref update in every worktree (commits on unit branches, `ORIG_HEAD`,
   `AUTO_MERGE`), so it must be fast: one gix open, no moon, no network.
2. For each stdin line whose ref is `refs/heads/<b>`: let `cur` be the
   ref's current id (gix `try_find_reference`; absent is zero). Skip it
   if `new == cur` (no-op) or `cur` is zero (a creation: a bolt's branch
   is made with `wt switch --create bolt/<x>`).
3. Read `.config/sluice.toml` from `cur`'s tree (56, the same as
   `Branch::read_file`) and match `<b>` against its `[branches]` patterns
   with the core's matcher (pass 1's `policy.rs`). No match: allow.
4. A match: print `error: <b> is protected by sluice; submit the change
   to sluice instead` (naming no driver, core.1) and a second line, `the
   worktree may hold the attempted change: git merge --abort or git reset
   --hard`; append the attempt to the branch's log; exit 1.

Captured with a shell stand-in for steps 1 to 4:

```
$ git merge unit/touches            # in a worktree on bolt/x
Updating 4fc2eb0..08f654f
Fast-forward
error: bolt/x is protected by sluice; use wt submit
fatal: in 'prepared' phase, update aborted by the reference-transaction hook
exit=128
$ git branch -f bolt/x unit/touches
error: bolt/x is protected by sluice; use wt submit
fatal: in 'prepared' phase, update aborted by the reference-transaction hook
```

Caught, each refused with the same two lines and exit 128: `git merge`
(fast-forward and `--no-ff`), `git commit`, `git reset --hard <other>`,
`git rebase`, `git update-ref` (with and without an old value), `git
branch -f` (of a branch no worktree holds; git refuses one that is
checked out itself), `git fetch . a:bolt/x`, and
`wt merge bolt/x` (worktrunk printed `✗ Can't push to local bolt/x
branch` / `Failed to update ref: error: bolt/x is protected by sluice;
use wt submit`, exit 1). A `git push` into a checked-out branch is
refused first by git's own `receive.denyCurrentBranch`; into a branch
not checked out, the hook refuses it in `git-receive-pack`.
`git branch -D` of a checked-out branch is refused by git itself;
deleting one that is not checked out reaches the hook as `new` = zero.
The hook allows that delete only when sluice holds no submission to the
branch that is queued, waiting, testing or landing; otherwise it refuses
and names them, so a bolt is withdrawn or landed before its branch goes.
`git branch -m bolt/x bolt/y` is a delete and a creation, so it follows
the same rule.
The branch's records stay readable after the delete (26).

**Logging the attempt (clause 1: "refused and reported").** The hook
appends `{"at":…,"event":"protected","ref":"refs/heads/bolt/x","from":<cur>,"to":<new>,"by":"<parent command>"}`
(the `protected` event, since `refused` is a landing that found its
branch moved) to the branch's `log.jsonl` through the file store (section 6), taking
the branch's lock with a bounded wait (one second) and bumping the
version, so the event is a record like any other. The parent command is
`ps -o command= -p <ppid>` (captured: `git merge unit/touches`, `git
-c … reset -q --hard …`, `git-receive-pack /…/repo/.`). If the lock is not
had in time, the refusal stands and only the stderr line is lost.

**A branch checked out in a worktree.** crew's bolt branches are checked
out in their conductors' worktrees (`crew/bolts/<name>` holds
`bolt/<name>`). When sluice moves such a branch through gix, that
worktree's HEAD moves with the ref but its index and files do not, so
`git status` there shows the landing reversed, staged. Captured after a
move under a clean worktree on `bolt/x` with one untracked file:
`D  t.txt`, `?? local.txt`. `git read-tree -m -u <old> HEAD` in that
worktree (the two-way merge `git checkout` does) brought it level and
kept the untracked file. sluice does not run it: the worktree is not a
place sluice made (core.16). The conductor runs it when its bolt's branch
moves, which crew learns from the submission's state. crew is to move to
a design where no worktree has a bolt's branch checked out, and the
hazard goes with it.

### Acceptance tests

In a scratch repository with a policy protecting `bolt/*`, a worktree on
`bolt/x`, and the hook installed by the installer:

1. Each command in the "caught" list is refused with exit 128, the branch unmoved, one `protected` event in the log.
2. `git worktree add ../w bolt/x`, `git reset --hard` with no target, `git branch bolt/new` (creation) and commits on `unit/*` are allowed.
3. A pre-existing `reference-transaction` hook still runs, with the same stdin, and its refusal still refuses.
4. `/usr/bin/git` (Apple git 2.50) is refused too.
5. A gix move by `Branch::move_to` is not refused and logs nothing.
6. Installing twice changes nothing; installing with the binary moved rewrites the path.
7. `core.hooksPath` pointing into a worktree: the installer refuses and names it.
8. A branch the policy does not name, and a protected one whose own revision's policy does not name it, are allowed (56).

---

## 3. moon: which checks a change affects, and what a run reports

**Implements** the moon half of `Tests::start` / `Tests::progress` (what
to run, and reading the result); the process handling is section 4's.
**Bears on** 15, 16, 19, 36 to 38, 69, 70;
core.7, core.9, core.10; ADR 0004.

Verified on moon 2.6.1 (sluice's devenv) in sluice's own workspace and
in a two-project scratch workspace (`alpha:test`, and `beta:test`
depending on it, both tagged `check-fast`). The crew captures below
(the run report's shapes, the replays) were made on 2.5.3 in a
throwaway clone of crew; the scratch workspace reproduced each shape on
2.6.1. Differences on 1.41.7 are noted.

### Targets: a branch requires checks by tier

A repository tags each check with its tier, and a branch's policy names
the tiers it requires. sluice's own workspace is the example. Its
checks fall into three tiers, by what they need and how long they take:

| tag | the checks | needs | takes |
|---|---|---|---|
| `check-fast` | every crate's cargo tests over the fakes; the citations check | cargo | seconds |
| `check-tools` | each profile against real git, moon and worktrunk on scratch repositories; the conformance suite over the real profiles | the devenv toolchain | tens of seconds |
| `check-e2e` | `e2e.sh` (section 8): a whole repository driven through `wt submit` | the toolchain and worktrunk | minutes |

Each task carries its tier's tag in its `moon.yml`:

```yaml
tasks:
  test:
    command: 'cargo test -p sluice-core'
    tags: [cargo-test, check-fast]
```

and the policy gates each branch on the tiers its work must pass:

```toml
[branches."bolt/*"]          # a bolt's units: the fast tier, landing in seconds
required = ["#check-fast"]

[branches.main]              # the shared branch: every tier
required = ["#check-fast", "#check-tools", "#check-e2e"]
batch = true
```

A required `#<tag>` is resolved to targets with a query, in the
candidate's place (captured in sluice's workspace; only `check-fast`
has tasks so far):

```
$ moon query tasks --tags check-fast </dev/null | jq -c '[.tasks[][] | .target]'
["cli:test","conformance:citations","conformance:test","core:test","fakes:test","seams:test"]
$ moon query tasks --tags check-tools </dev/null | jq -c '[.tasks[][] | .target]'
[]
```

The union of every required tag's targets, plus each required
`<project>:<task>` (itself a target), is what the run passes to `moon
exec` (*The run*, below). A required tag with no tasks contributes
nothing; the gate is read over the checks it requires (15), so a tier
with no tasks yet cannot hold a branch back, and a tier that gains tasks
gates from the revision that adds them (56).

A bare tag is not a target on the command line:

```
$ moon exec '#check-fast'
error: invalid value '#check-fast' for '[TARGETS]...': Invalid target #check-fast, tags are not supported for default project targets.
```

| form | selects (2.6.1) |
|---|---|
| `:#check-fast` | every project's tasks tagged `check-fast` |
| `alpha:#check-fast` | one project's |
| `#check-fast:test` | projects tagged `check-fast`; finds nothing (`No tasks found`, exit 1): moon 1.x's form, for project tags |
| `moon query tasks --tags check-fast` | the tagged tasks, as JSON |

crew today gates on one tag, `#suite`, over its 59 test files; it reads
as a single tier in this scheme.

### Which checks a range affects (36)

```
MOON_BASE=<base> MOON_HEAD=<head> moon query affected --downstream deep </dev/null
```

`query tasks` and `query affected` still have no `--base`/`--head` flags
in 2.6.1; they take them from `MOON_BASE`/`MOON_HEAD`. Captured in the
scratch workspace after a commit changing `alpha/value.txt`:

```json
{"tasks":{"alpha:test":{"files":["alpha/value.txt"],"other":false},"beta:test":{"upstream":["alpha:test"],"other":false}},"projects":{"alpha":{"files":["alpha/value.txt"],"tasks":["alpha:test"],"other":false},"beta":{"upstream":["alpha"],"other":false}}}
```

Without `--downstream deep` only `alpha:test` (and `alpha`) is listed.

Changed files alone:

```
$ moon query changed-files --base 21f97fc7bf4d --head 9955da1b1b2a
{"files":["alpha/value.txt"],"options":{"base":"21f9…","defaultBranch":false,"head":"9955…","local":true,"status":[],"stdin":false},"shallow":false}
```

Range semantics, `moon query tasks --tags check-fast --affected
--downstream deep` in the scratch workspace (2.6.1), and in the crew
clone (2.5.3) for `--affected remote` and a docs-only range:

| query | tasks |
|---|---|
| `MOON_BASE`/`MOON_HEAD` set to a range changing `alpha/value.txt` | `["alpha:test","beta:test"]` |
| set to a range changing `beta/notes.md` | `["beta:test"]` |
| set to a range changing `.moon/workspace.yml` | `["alpha:test","beta:test"]`: every task, since `.moon/*.yml` is an input of every task |
| bare `--affected`, clean tree | `[]` (compares uncommitted changes) |
| `--affected remote` | crew: `["t-docs"]` (merge-base with `origin/main`) |
| base = head | `[]` |
| a docs-only range | crew: `[]` |

Base and head may be any commits in the object store; the working tree
need not be at head. But **the project and task graph is read from the
working tree the query runs in**, not from head, so the query runs in
the candidate's place, which holds the candidate.

Task dependents come only through task `deps`. With project
`dependsOn: ['alpha']` and no task dep, `beta` appears under `projects`
but `beta:test` is not under `tasks`.

### The run: which targets (37, 70)

In the candidate's place:

```
MOON_BASE=<since> MOON_HEAD=<candidate> moon query tasks --tags <tag> --affected --downstream deep </dev/null | jq -r '.tasks[][] | .target'
moon query tasks --tags <tag> </dev/null                       # when since is None, or moon's configuration changed: every required check (70)
moon exec <target>… --on-failure continue </dev/null
```

`RunRequest.since` is the branch's revision when it is covered by a
verdict and `None` otherwise (70; pass 1's Q12). When the changed files
between `since` and the candidate include moon's own configuration
(`.moon/**`, any `moon.yml`), the project graph itself changed: run
every required check. Pass the explicit
targets to `moon exec`:

- `moon exec ':#check-fast' --affected --base B --head H --downstream
  deep` ran only `RunTask(alpha:test)` although the report's
  `context.affected` listed `beta:test` (2.6.1, as on 2.5.3); with
  `-g`/`--include-relations` it ran both. The explicit list avoids the
  trap, and the set run is then exactly the set sluice recorded as
  affected.
- `--on-failure` defaults to `bail`; `continue` runs every selected task.
- `moon ci` writes `.moon/cache/ciReport.json`, not `runReport.json`, and
  its downstream defaults to none. Use `exec`.
- A task's own `timeout` option (seconds) cancels that task; the
  policy's `timeout` (66) is the whole run's and is section 4's to
  enforce, whatever the tasks declare.
- `-c/--concurrency` (`MOON_CONCURRENCY`) caps moon's threads: the
  instance's bound (71, `Tests::bound`) counts runs, not moon
  threads, so set `-c` from the CPUs sluice may use divided by the
  bound (section 4).

An empty target list is a pass with nothing run; do not call moon.

Exit codes (2.6.1): 0 when every task passed or replayed; 1 when any
failed (`Tasks: 1 failed, 1 skipped`). With nothing affected, `moon exec
--affected` exits 0, prints `No tasks affected by changed files. Unable
to execute action pipeline.` and **writes no `runReport.json`**. Never
read the exit code as the verdict; read the report.

### The run report: `<place>/.moon/cache/runReport.json`

Top-level keys `actions, comparisonEstimate, context, duration, status`
(`status: "completed"`), the same on 2.6.1. `actions[]` holds non-task
actions too (`SyncWorkspace`, `SyncProject(alpha)`); a task's has
`label: "RunTask(<target>)"` and `node.params.target`. A `wait`
dependency runs as `RunPersistentTask(<target>)`; it is the check's
helper, not a check, so the profile reads `RunTask(` actions only, and a
required target that appears only as `RunPersistentTask` is missing.
2.6.1 adds `allowFailure` and `expectFailure` to each action. From
crew's real runs (2.5.3; the scratch workspace gave the same operations
and statuses on 2.6.1):

A task that ran and passed (first run; 181 s):

```json
{"label":"RunTask(tests:t-after)","status":"passed","flaky":false,"duration":{"secs":181,"nanos":401781333},
 "operations":[
  {"status":"passed","meta":{"type":"hash-generation","hash":"687fab2c773ca4fd2dc2bc2c01eb33fe7346536f02f1edab260f4493ae734ad9"}},
  {"status":"skipped","meta":{"type":"output-hydration"}},
  {"status":"passed","meta":{"type":"task-execution","command":"tests/run t-after","exitCode":0}},
  {"status":"passed","meta":{"type":"archive-creation"}}]}
```

The same task replayed (second run of the suite: `59 passed (59
replayed)`, 1.0 s against 8 m 48 s):

```json
{"label":"RunTask(tests:t-after)","status":"cached","duration":{"secs":0,"nanos":75216500},
 "operations":[
  {"status":"passed","meta":{"type":"hash-generation","hash":"687fab2c…4ad9"}},
  {"status":"cached","meta":{"type":"output-hydration","command":"tests/run t-after","exitCode":0}}]}
```

A failing task (an `exit 1` added to `tests/t-docs.sh`):

```json
{"label":"RunTask(tests:t-docs)","status":"failed","error":"Task tests:t-docs failed to run.",
 "operations":[
  {"status":"passed","meta":{"type":"hash-generation","hash":"81a9a4363e4fe7da26e67a8c71902a7e2e2761bcc7f228fb4aaefbecf7d3487e"}},
  {"status":"skipped","meta":{"type":"output-hydration"}},
  {"status":"failed","meta":{"type":"task-execution","command":"tests/run t-docs","exitCode":1}}]}
```

No archive is made for a failure, and running it again fails again rather
than replaying (ADR 0004 holds on 2.5.3 and 2.6.1).

A task's own failure options do not change what sluice reads: `status`
is moon's verdict on the task, and the gate is the policy's (27). On
2.6.1, a task with `allowFailure: true` that exits 1 reads `"status":
"failed"`, `"allowFailure": true`: a Fail, though moon's pipeline goes
on. A task with `expectFailure: true` (new in 2.6.0: "fail the entire
action pipeline if it passes … never cached") that exits 1 reads
`"status": "passed"` with `task-execution` `exitCode` 1: a Pass, never
replayed.

A dependent of a failed task, in the scratch workspace's S4 candidate:
`[{"t":"alpha:test","s":"failed"},{"t":"beta:test","s":"skipped"}]`. In
another run the dependent was absent from the report altogether. Either
way it was not run.

| `TaskResult` field | from |
|---|---|
| `task` | `node.params.target` |
| `hash` (the key) | the `hash-generation` operation's `meta.hash`: present on a pass, a replay and a fail alike |
| `outcome` | `status`: `passed` → Pass; `failed` → Fail; `cached` and `cached-from-remote` → Pass, replayed |
| `source` | `cached`/`cached-from-remote`, or an `output-hydration` operation with status `cached` → `Source::Replayed`; else `Source::Ran { host }`, the host from `nix::unistd::gethostname` |
| exit code | `task-execution` `meta.exitCode` on a run, `output-hydration` `meta.exitCode` on a replay |

A required affected task that is absent from the report, or `skipped`,
or any other status, has no `TaskResult`: the core reads it as missing
(15, 19), never as passed. A replay's `from_run_at` (walkthrough log) is
not in the report: take it from the newest `ran` with the same task and
hash in `results.jsonl` (69), or leave it out.

`context.targetStates` (`{"state":"passed","hash":…}`, `{"state":"failed"}`
with no hash) cannot tell a replay from a run and has no hash for a
failure: do not use it. crew's working parser is `tests/suite:82`:

```
jq -r '.actions[] | select(.label | startswith("RunTask(tests:")) | "\(.label | ltrimstr("RunTask(tests:") | rtrimstr(")")) \(.status)"'
```

and its status mapping is `tests/suite:89-94`.

### Per-task run records: `<place>/.moon/cache/states/<project>/<task>/`

`lastRun.json`, `stdout.log`, `stderr.log` (crew prints the logs of a
failed task, `tests/suite:101-105`):

```json
{"exitCode":0,"hash":"687fab2c…4ad9","lastRunTime":1791753037223,"target":"tests:t-after"}
{"exitCode":1,"hash":"81a9a436…487e","lastRunTime":1791753401936,"target":"tests:t-docs"}
```

`lastRun.json` is rewritten on a replay (only `lastRunTime` changes) and
carries no replay flag, so the replay distinction is only in the report
(core.9). Use `lastRun.json` only to cross-check a failure's hash and
exit code. A failed task's `stdout.log`/`stderr.log` stay in the place
until it is released; what the culprit's session is shown of them (20)
is the driver's (section 7).

On 2.6.1 `lastRun.json` also carries `"outputGlobsHash"`, for example
`{"exitCode":1,"hash":"65b28ff5…91f0","lastRunTime":1791811362337,"outputGlobsHash":"","target":"alpha:test"}`.

Elsewhere in the cache: `.moon/cache/outputs/<hash>.tar.gz` holds
archives, passes only. The hash manifest (inputs, command, args, env,
deps) was `.moon/cache/hashes/<hash>.json` on 2.5.3; on 2.6.1 `hashes/`
stays empty and the manifests are content-addressed JSON under
`.moon/cache/blobs/<2 hex>/<hash>`. The profile reads neither.

### Replays across places

Each checkout has its own `.moon/cache`, so a fresh sluice place replays
nothing by itself: in the scratch workspace every candidate ran
`alpha:test` again. crew shares replays by linking
`.moon/cache/outputs` to `<common>/moon-outputs` in devenv's `enterShell`
(`crew/main/devenv.nix:13-24`), so a place replays only when moon runs
inside `devenv shell --`. Verified: in the crew e2e fixture below, run
through `devenv shell --` in fresh detached worktrees, the second
candidate reported `{"t":"tests:t-docs","s":"cached"}`. A worktrunk
checkout of crew gets no help from hooks: crew's `.config/wt.toml` has
only `[pre-merge]` and `[post-merge]`.

So the moon profile must be able to run moon under the repository's
shell: the policy's `[tests] shell` (section 4, *The shell a run
happens in*).

### Environment

moon hands its tasks `MOON_CACHE_DIR, MOON_CACHE, MOON_LOG,
MOON_PROJECT_ID, MOON_PROJECT_ROOT, MOON_PROJECT_SNAPSHOT,
MOON_PROJECT_SOURCE, MOON_TARGET, MOON_TASK_HASH, MOON_TASK_ID,
MOON_TASK_RETRY_ATTEMPT, MOON_TASK_RETRY_TOTAL, MOON_THEME, MOON_VERSION,
MOON_WORKING_DIR, MOON_WORKSPACE_ROOT` (captured on 2.5.3 and again on
2.6.1), and reads `MOON_*` as
options (`MOON_BASE`, `MOON_HEAD`, `MOON_AFFECTED`, `MOON_FORCE`,
`MOON_CACHE`, `MOON_ON_FAILURE`, `MOON_CONCURRENCY`, …). A sluice started
from inside a moon task (a test of sluice, a waiting `wt submit` run by a
task) would hand those on. Clear every inherited `MOON_*` and set only
the ones sluice means, as crew's `tests/suite:62-66` does. crew's devenv
sets `MOON_TOOLCHAIN_FORCE_GLOBALS=true`; that comes with the shell.

moon also puts the tools of its homes ahead of everything on a task's
`PATH`: `$MOON_HOME/bin` (default `~/.moon/bin`), `$PROTO_HOME/shims`
and `$PROTO_HOME/bin` (default `~/.proto`). Captured on 2.6.1: a task's
`PATH` began `~/.cargo/bin`, the pinned rustup toolchain, proto's own
directory, `~/.proto/shims`, `~/.proto/bin`, `~/.moon/bin`, and only
then the shell's. With the user's own installs there (moon 2.4.6 in
`~/.proto/bin`, 1.41.8 in `~/.moon/bin`), every moon a task ran was one
of those, not the shell's 2.6.1. A repository's shell that wants its
tools to win sets both homes to its own; sluice's sets
`PROTO_HOME=<common>/proto` and `MOON_HOME=<common>/moon-home`, shared
by its worktrees.

### Archive places

In a `git archive` export (no `.git`), on 2.5.3 and again on 2.6.1:
`moon exec ':#check-fast'` runs (2 completed); `moon exec --affected
--base HEAD~1 --head HEAD` **silently ran everything**, exit 0; `moon
query tasks --affected` failed (`Process git failed: exit code 128 …
fatal: not a git repository`). With the archive
method, compute the targets in a probe worktree, one long-lived
detached worktree per repository at `<common>/sluice/.places/probe`,
moved with `git checkout --detach -f <candidate>` under the repository
lock (`.lock`), and run the explicit list in the archive.

### moon 1.41.7 (on `PATH`)

No task `tags` (`tasks.test.tags: unknown field 'tags'`; tags are
project-level and targeted `#<tag>:test`), no `moon exec` (use `moon
run`), no `--base/--head` on `run`, `vcs.manager` where 2.x has
`vcs.client`, `query touched-files` where 2.x has `changed-files`, no
`query affected`, and `query tasks` needs `--json`. The report's shape is
the same, with `context.touchedFiles` for `changedFiles`. The profile
refuses moon below 2.6 by name (`moon --version` → `moon 2.6.1`): 2.5
runs the queries and reports above, but a repository whose checks use
`wait` or `cleanup` dependencies fails to load on it.

### Acceptance tests

On a scratch workspace like the end-to-end one (two projects, `beta`'s
`test` depending on `alpha:test`), with moon ≥ 2.6:

1. The targets run for a change to `alpha/value.txt` are `alpha:test` and `beta:test`; to `beta/notes.md`, `beta:test`; to `.moon/workspace.yml`, every tagged task.
2. A run of an unaffected candidate starts nothing and reports a pass with no tasks.
3. First run: both tasks `Ran`, with hashes; second run in the same place: both `Replayed`, same hashes (16, 38, core.9).
4. A failing task: `Fail` with its hash; its dependent has no result (missing, not pass).
5. A `since: None` request runs every tagged task (70).
6. With `MOON_FORCE=1` and `MOON_BASE=nonsense` in sluice's own environment, the run is unaffected (env cleared).
7. moon 1.41.7 and moon 2.5.3 on `PATH` are refused with their versions named.
8. Two required tags, `#check-fast` and `#check-tools`, run the union of their targets; a tag with no tasks adds none.
9. A check with a `wait` dependency on a helper and a `cleanup` dependency: the helper's `RunPersistentTask` is not read as a result, and the check's own outcome is.
10. A task with `allowFailure: true` that fails is a Fail.

---

## 4. Running tests: one process group, a supervisor, a time limit

**Implements** the process half of `Tests::start` and `Tests::progress`
(for the moon profile and the `command` runner alike), and the
run-ending half of `Checkout::release`. **Bears on** 31, 32, 34, 37, 60,
66, 71; core.10, core.14, core.16; ADR 0010.

### Why a supervisor

A step holds nothing between steps (core.14; ADR 0005) and exits, while a
run takes minutes. The process that starts moon cannot be the one that
waits for it, and a process that is not a child's parent cannot learn its
exit status. So `start` spawns a small supervisor, `sluice
supervise <run dir>`, that outlives the step, runs the command, enforces
the time limit, ends the group and writes the result; `progress` reads
what it wrote.

It is built with the moon profile, the first runner, not after it. moon
does none of the four things: it does not outlive the step for it, it
has no time limit of its own over a whole run, it leaves a server a task
started running after it exits, and its exit code is not where its
results are. The supervisor is `sluice` itself under another subcommand,
about two hundred lines over `std` and `nix`, and every later runner
reuses it.

```
<common>/sluice/.runs/<place>/
  request.json    what to run: argv, cwd (the place), env, limit, grace, and the runner's meta; written by start before the spawn
  lock            held (flock) by the supervisor for its whole life; holds its pid
  pgid            the command's process group id, written once it is spawned
  out.log         the command's stdout and stderr
  result.json     {"exit": 0|n|null, "signal": null|n, "timed_out": bool, "error"?: …, "started": …, "ended": …}; written last, by rename
```

`progress` reads moon's report from the place
(`<place>/.moon/cache/runReport.json`, removed by `start` before the
run), which the core releases only after reading the run.

`RunHandle` is the place name. Run directories are removed when the
place is released, after the core has read the result.

### Facts, verified with a Rust probe (nix 0.31, macOS and Linux)

Run on macOS (rustc 1.98.1) and on Linux 7.2 in containers on arm64 and
x86_64 (rustc 1.99.0, nix 0.31.3), with the same results; the captures
are macOS's. Two differences the code must handle:

- **A zombie is still a member.** A leader that exited but was not reaped
  keeps `killpg(pgid, None)` returning `Ok`. The supervisor `wait`s its
  own leader before reading `ESRCH` as "group gone"; orphans reparented
  to PID 1 are reaped by init, except in a container with no init that
  reaps (run checks in containers with `--init`).
- **Listing members.** `/proc/<pid>/stat` is Linux's alone and `ps` is
  not in every image; the code tests liveness with `killpg(pgid, None)`
  and never lists members. Detaching uses `setsid()` in `pre_exec`, not
  the `setsid` command, which macOS lacks and which on Linux forks only
  for a group leader.

- `std::os::unix::process::CommandExt::process_group(0)` (stable since
  1.64) puts the child in a new group whose id is its pid. A grandchild
  backgrounded by a leader that then exits stays in the group, and
  `nix::sys::signal::killpg(pgid, SIGTERM)` reaches it after the leader is
  gone:

  ```
  == orphan
  grandchild=50934
  leader 50933 exited Some(0); group 50933 alive=true
  members:
  50934 50933      0 sleep 300
  ended by TERM
  members after: []
  ```
- `killpg(pgid, None)` (signal 0) is `Ok` while any member lives and
  `ESRCH` once none does: the liveness test for a group.
- A group whose members ignore TERM is ended by KILL after the grace:

  ```
  == timeout          (sh -c "trap '' TERM; sleep 300 & sleep 300; wait", limit 2 s, grace 1 s)
  timed out after 2.092135583s: ended by KILL; leader status None (signal Some(9))
  members after: []
  ```
- An unrelated later process can end a detached group: a group made by
  a process that then exited was ended by `kill -TERM -<pgid>` from a
  separate process, every member gone. So a step can end a run the
  supervisor left behind.
- A process that calls `setsid()` itself leaves the group and escapes
  killpg:

  ```
  == escape
  escaper=51782
  group alive after leader exit: false
  escaped pids: 51782
  ```
  Daemons and docker containers escape this way; the walkthrough leaves
  them to the repository (moon cleanup tasks, devenv, testcontainers' reaper,
  a worktrunk `post-remove` hook).
- moon keeps its tasks in its own group: `moon exec` pid 87695 had pgid
  87695, and so did its task `sleep 37`; `kill -TERM -- -87695` ended both.

### The recipe

`start(request)`:

1. Create the run directory, write `request.json` (argv from the moon
   profile or `[tests] command`; cwd the place; env below; the limit).
2. Spawn `sluice supervise <run dir>` (the current executable) with
   `stdin`, `stdout`, `stderr` to `/dev/null` and, in `pre_exec`,
   `nix::unistd::setsid()`: a new session, so a terminal's Ctrl-C or
   hang-up, delivered to the foreground group of a waiting `wt submit`,
   never reaches a run. Do not wait for it; it is reparented when the step
   exits.
3. Return the handle.

`sluice supervise <run dir>`:

1. Take `lock` with `File::lock()` (held until exit; the descriptor is
   close-on-exec, as Rust opens files, so the command does not inherit it).
2. Spawn the command with `process_group(0)`, `stdin` null, `stdout` and
   `stderr` to `out.log`, `env_clear()` and the env below. Write `pgid`
   (= its pid) by rename.
3. Loop: `child.try_wait()` every 100 ms against the deadline (std has
   no `wait_timeout`; polling keeps it dependency-free).
4. On exit: `killpg(pgid, SIGTERM)`, poll `killpg(pgid, None)` up to a
   grace of 10 s, then `killpg(pgid, SIGKILL)`. Every run ends its group,
   so a server a task left running cannot fail the next candidate (32).
5. On the deadline: the same TERM, grace, KILL, and `timed_out: true`
   (66).
6. Write `result.json` by rename (section 6's atomic write). Exit.

`progress(handle)`:

- `result.json` present → parse it and the report → `Finished(RunReport
  { tasks, executor, missing, timed_out })`.
- absent, and `lock` is held (`File::try_lock()` gives `WouldBlock`) →
  `Running`.
- absent, and `lock` is free → the supervisor died without a result: end
  the group (`killpg` TERM, grace, KILL, if `pgid` exists and the group
  is alive) and return `Lost` (60: no outcome, run again).

`Checkout::release(name)` first ends the run of that place: if
`.runs/<name>/pgid` exists and the group is alive, TERM, grace, KILL;
then it removes the place (34, core.16). This is how the core's
"ended and its place released" (pass 1's Q7) stops a run nobody will
read. The supervisor and the group-ending code are `crates/sluice/src/runs.rs`,
outside every profile and naming no tool, so both profiles use it and
no profile calls another (core.2).

A recorded pgid can be reused once its group is gone. A group is
signalled from `pgid` only while no `result.json` says the supervisor
ended it: when the supervisor died first (`Lost`), or by `release` of a
run still in progress.

**The instance's bound (71).** An instance of sluice starts at most `B`
runs at once, whatever the windows of the branches it serves. `B` is the
instance's, never a policy's: `[runs] max` in
`$XDG_CONFIG_HOME/sluice/config.toml` (default `~/.config`), else the
CPUs sluice may use divided by four, never fewer than one. "The CPUs
sluice may use" is `std::thread::available_parallelism()`, which on
Linux honours the process's CPU affinity and its cgroup quota, so a
container or a CI runner limited to four CPUs reads four, not the
machine's sixty-four.

`Tests::bound()` is `B`. The core starts no more than `B` less the
candidates under test (`starting` or `testing`) in the records of the
branches the instance serves: the stepped branch for `sluice step` and a
waiting `wt submit`, every branch with records for `sluice run`
(`Serves`). Nothing is held in memory and nothing new is stored
(core.14). A machine is bounded as a whole when one instance
starts every run on it: one `sluice run` that every `wt submit
--remote` submits to, on that machine or another. When each session's
waiting `wt submit` runs its own rounds instead, each is an instance
with its own bound, and the windows of the branches are the only limit
between them (71 says no more).

Each run is told its share: the runner is given
`available_parallelism() / B` threads, moon's `--concurrency` and
`SLUICE_JOBS` in a `command` runner's environment for the command to
pass on, so `B` runs at once use the CPUs sluice may use and no more.

**Environment for the command.** `env_clear()`, then `PATH`, `HOME`,
`USER`, `LOGNAME`, `SHELL`, `TMPDIR`, `LANG`/`LC_*`, `TERM`, and the tool
homes `PATH` points into, `PROTO_HOME` and `MOON_HOME` (a proto shim on
`PATH` resolves its tool under its home: without it, a shim in a moon
task fell back to `~/.proto` and the user's global pin, moon 1.41.7),
from sluice's own environment; `SLUICE_CHECKOUT=<place path>`, `SLUICE_TMP=<common>/sluice/.tmp/<place>`
(made empty by `materialize`); nothing of `MOON_*`, `GIT_*`
(`GIT_DIR`/`GIT_WORK_TREE` from a hook would point git at the wrong
repository), `DEVENV_*` or `DIRENV_*` (a waiting `wt submit` runs in a
session's shell, and that session's place must not leak into the
candidate's, 35).

**The toolchain a run uses.** A repository's tools often exist only
inside an environment it declares: crew's moon and replay cache only
inside its devenv shell. The policy names the command that prints that
environment as shell code:

```toml
[tests]
toolchain = ["devenv", "print-dev-env"]
```

`start` runs it in the place, outside the executor, with the environment
above, and keeps what it prints in the run directory (`toolchain.sh`).
Every command the runner runs in the place, its queries and the run
itself, is `bash -c 'source <toolchain.sh> >&2; exec "$@"'`, so the
script sets the tools up and anything it prints goes to stderr. The
command is opaque to sluice, which reads no confinement or limit from
it (those are the executor's, below); read from the branch's revision
like the rest of the policy (56). Anything that needs the network or the
user's home to set up (nix evaluating the shell, proto installing a
pinned version) happens here, outside the executor, so a sandboxed run
only uses what is already installed. Other toolchains fit the same key:
`proto activate bash --export`, `mise env -s bash`, `nix
print-dev-env`, `direnv export bash`. When sluice itself already runs
inside the repository's environment, the key is left out: the run keeps
that `PATH` and, as above, nothing that names a session's place (35).

Never install anything in the toolchain's own setup commands
(devenv's `enterShell`): the printed script holds them, so they run
again inside the executor, where installing fails. devenv tasks are not
in the printed script.

Before any moon command, the moon runner runs, in the place and outside
the executor, with the toolchain sourced:

1. `proto install --config-mode upwards`, when the place has a
   `.prototools`: it installs the exact versions that file lists, moon
   among them, so moon exists before anything calls it. Only the
   repository's own `.prototools` is read, not the user's global one.
2. `moon --version`, refused below 2.6.
3. The task queries, then `moon setup`.

The moon runner runs `moon setup` outside the executor, after the
toolchain: it installs the workspace's toolchains (`.moon/toolchains.yml`,
through proto). Inside, moon still runs its setup actions, which find
them installed, put them on the task's `PATH` and make each task's hash
carry the toolchain's version; proto stamps each one it uses with a
`.last-used` file in the tool's directory, so the sandbox lets the run
write files of that name and nothing else outside the place. `moon exec
--no-actions` avoids the stamp but runs whatever is on `PATH` under a
hash that still names the configured version: captured on 2.6.1, nix's
cargo ran and the key read rust 1.98.1. So it is never used. With no
network, moon skips installing dependencies (`No internet connection,
skipping dependency install`).

The `[tests] runner = "command"` profile runs
`sh -c <command>` in the place under the same supervisor, one task named
by the command, never replayed (C.2 script).

### Acceptance tests

1. A command that backgrounds `sleep 300` and exits 0: `Finished`, pass, and no member of the group remains (32).
2. A command over its limit: `Finished` with `timed_out`, read as failed, the group gone, a TERM-ignoring member killed (66).
3. `kill -9` of the supervisor mid-run: `progress` returns `Lost` and the command's group is ended (60).
4. `release` of a place with a run in progress: the group is ended before the place is removed (34).
5. A waiting `wt submit` killed with Ctrl-C (SIGINT to its foreground group): the run continues and a later `step` reads its result.
6. The command sees `SLUICE_CHECKOUT`, `SLUICE_TMP` and none of `MOON_*`, `GIT_DIR`, `DEVENV_ROOT` set in sluice's environment.
7. With `[runs] max = 2` and three branches each with a window of four, one `sluice run` has at most two runs under test at once across all three, and each moon run is given half the CPUs (71).

### The executor (83, 84)

The executor is how a run is held to its limits and whether it is
confined. It is part of the check seam: `Tests::start` takes the limits
and the run's report says what the executor enforced, recorded with each
result (16). The policy states what a branch's runs may use and reach:

```toml
[executor]
kind = "sandbox"          # "open" | "sandbox" | "container"
network = "localhost"     # false: none | "localhost": only 127.0.0.1 and ::1 | true: any
memory = "8G"
# processors come from the instance: available_parallelism() / B
```

| kind | how | enforces |
|---|---|---|
| `open` | the supervisor above: a process group in the place | the processors, through the runner's threads; nothing else; never confined |
| `sandbox` | the command under the host's kernel sandbox, as Anthropic's sandbox-runtime and Codex's sandbox do: Seatbelt (`sandbox-exec` with a generated profile) on macOS, bubblewrap with Landlock on Linux; writes allowed only to the place and `SLUICE_TMP`, the network only as the policy says | paths and network on both; memory and processors only on Linux, by starting the run in a cgroup (`systemd-run --user --scope -p MemoryMax= -p CPUQuota=`) |
| `container` | the place mounted into a new container (`docker run --rm --init --cpus --memory --network none -v <place>:<place> -w <place> <image>`), the image the repository declares | processors, memory, paths and network |

`network = "localhost"` is for checks that start a server and test
against it: on macOS the sandbox lets the run listen and connect on
localhost, by address or by name, and refuses every other connection,
name lookups included (`(deny network*)`, then `network-bind` and
`network-inbound` on local `localhost:*` and `network-outbound` to remote
`localhost:*`; checked: a remote address fails with "Operation not
permitted", IPv4 and IPv6 loopback both work). `false` denies local
sockets too. The `open` executor holds neither, so it refuses both.

A limit the policy states that the executor cannot enforce refuses the
run (84): a `memory` limit under `sandbox` on macOS is refused with the
reason, never run without it. Every run's record carries `executor`
(`kind`, `confined`, and each limit held), so a gate that trusts only
confined runs (29) reads it from the record and never from the policy.
`confined` is true only for `container`, and for `sandbox` when no
session on the host can write the place; `open` never is. Containers
run with `--init` so orphans are reaped and a group's end is seen
(section 4's Linux facts).

**What a branch trusts.** Which results a gate accepts is the policy's
(27, 28), named by what the record says, never by what the policy asked
for:

```toml
[branches."bolt/*"]
trust = ["open", "sandbox", "container", "cache"]

[branches.main]
# On one Mac, until runs are confined: main accepts sandboxed runs too.
trust = ["confined", "sandbox"]
```

`trust` lists `confined`, executor kinds (`open`, `sandbox`,
`container`) and `cache`. A ran result counts when its record's
`confined` is true and `confined` is listed, or its `kind` is listed. A
replay has no executor of its own (16, 38): the task runner's cache
holds whatever any run wrote to it, a person's open shell included, so
a replay counts only when `cache` is listed (ADR 0004: a bolt's gate
takes the local cache, the shared branch's only what a trusted executor
ran). A gate without `cache` asks the runner for no replays
(`RunRequest.replay = false`; moon's `--force`), and a replay a runner
gives anyway satisfies nothing. A result that does not count is left out
of the key's reading entirely (28): a check with no counted result reads
as missing. A table without `trust` trusts only `confined`, the
conservative reading.

A gate the branch's executor can never satisfy is refused before any run
starts: `[executor] kind = "sandbox"` under `trust = ["confined"]`
stops the branch's step with that reason, and nothing is started,
attributed or landed (29). Widening `main` to `sandbox` is how it lands
through sluice on one Mac today: the records still say `confined:
false`, so the day the agents are fenced off from sluice's places (or the
runs move to a container), narrowing `trust` back to `confined` is one
line and every earlier result reads for what it was. Like every key of
the policy, `trust` is read from the branch's own revision, so a
submission cannot widen the gate it is judged by (56).

The supervisor wraps every kind: it spawns the executor's command line
(`sandbox-exec …`, `bwrap …`, `docker run …`) as the group leader and
ends the group at the time limit as above; a container is also removed
by name (`docker rm -f sluice-<place>`), since its processes live in the
engine, not in the group.

#### What sluice's own checks need under Seatbelt

With the toolchain started inside the run (`devenv shell --` as a
prefix), the probe below found what devenv and nix need there. With the
toolchain printed outside (`[tests] toolchain`, above), sluice's fast
tier ran under the bare fence: writes only to the place and
`SLUICE_TMP`, no network at all, all six tasks passed.

Probed on macOS with sluice's fast tier, `devenv shell -- moon exec
':#check-fast' --force`, in a detached worktree, under `sandbox-exec`
with `(allow default)`, `(deny file-write*)` except the place and
`SLUICE_TMP` (and `/dev/null`, `/dev/tty*`, `/dev/fd/*`), and `(deny
network*)`, with `TMPDIR` set to `SLUICE_TMP`. Each denial in turn:

| denied | by | needed |
|---|---|---|
| `/tmp/devenv-<hash>` | devenv's runtime directory, whatever `TMPDIR` says | yes: devenv will not start |
| `~/.cache/nix/fetcher-locks/*.lock` | nix evaluating the shell | yes |
| connecting to `/nix/var/nix/daemon-socket/socket` | nix, through the daemon | yes: a unix socket, denied by `network*` |
| a GC root symlink | devenv | no: a warning |
| `~/Library/Caches/treefmt/…` | the shell's `treefmt` enter task | no: that task fails, the shell still enters |
| `<common>/moon-outputs/*.tar.gz` (the place's `.moon/cache/outputs` links there) | moon archiving a passed task | no: a warning, the task still passes, and nothing a sandboxed run made reaches the shared cache |

With the first three allowed, all six tasks ran and passed (cargo
built in the place's `target/`; nothing under `~/.cargo` was written).
So a repository whose toolchain is a devenv shell needs, beyond the
place and `SLUICE_TMP`, two paths to write and one unix socket to reach;
which ones is the repository's, so the policy states them (83: "the
paths it may write that the branch's policy states").

#### Acceptance tests

1. `sandbox` on macOS and on Linux: a write outside the place and `SLUICE_TMP` fails inside the run; with `network = false`, a connection fails, even to localhost; with `network = "localhost"`, a server on localhost is reached and any other address is refused as not permitted (83).
2. `sandbox` with `memory = "1G"` on macOS: the run is refused, naming the limit; on Linux, a run that allocates past it is killed and read as failed (84).
3. `container`: a run over its time limit is ended and its container removed; nothing of it is left in `docker ps -a`.
4. Every recorded run carries the executor's `kind`, `confined` and limits; an `open` run is never read as confined (29, 84).

---

## 5. Checkouts

**Implements** `Checkout::materialize(name, rev) -> Place { name, path,
tmp }` and `Checkout::release(name, passed)` in one profile, a detached
git worktree (C.1; ADR 0013); `[checkout] method = "git-worktree"` names
it, and any other method is refused with the reason. **Bears on** 25, 32
to 35, S11; core.3, core.10, core.16; ADR 0010, ADR 0013.

`materialize` makes `<common>/sluice/.tmp/<name>/` empty and returns it
as `tmp`; `SLUICE_TMP` lives outside the checkout so the checkout stays
exactly the candidate. `release` ends the place's run (section 4), removes
the checkout and `tmp`, and succeeds when the place is already gone, so a
release repeated after a crash is harmless (the trait's doc requires
that for a name never materialized). Neither ever touches a place sluice
did not make (core.16): every path is derived from `name`, never searched.

### `git-worktree`

```
git -C <common> worktree add --detach <common>/sluice/.places/<name> <candidate>
git -C <common> worktree remove --force <common>/sluice/.places/<name>
```

- add: exit 0, stderr `Preparing worktree (detached HEAD 4d819b0)` /
  `HEAD is now at 4d819b0 cand`. It makes
  `<common>/worktrees/<name>/`, and the checkout's `.git` file reads
  `gitdir: …/.git/worktrees/<name>`.
- It **runs the repository's `post-checkout` hook** (observed:
  `post-checkout 0000…0000 4d819b0… 1`), so "nothing is run in it" holds
  only where none exists. `git -c core.hooksPath=/dev/null worktree add
  …` keeps the method's promise: with a logging `post-checkout` hook, a
  plain add logged `post-checkout 0000…0000 bbe36b7… 1` and the add with
  the flag logged nothing.
- remove without `--force` on a dirty checkout: exit 128, `fatal: '<dir>'
  contains modified or untracked files, use --force to delete it`. Tests
  write into their checkout, so always `--force`.
- Directory deleted out from under it: `git worktree list` shows it
  `prunable`; `git worktree remove --force <dir>` still exits 0 and
  cleans up. `git worktree prune -v` prints `Removing worktrees/<name>:
  gitdir file points to non-existent location`.
- Left behind after remove: nothing (`.git/worktrees` itself goes when
  empty). No branch is made.
- A checkout under the common dir works for git and moon: a worktree at
  `.git/sluice/places/try-1` gave `git rev-parse --show-toplevel` = that
  path, `moon exec ':#suite'` ran 2 tasks, and the affected query worked.

### Caches (35)

A new place builds cold. A branch's policy declares the caches a place
may hold beyond the candidate, the way a CI job declares `actions/cache`:

```toml
[[checkout.cache]]
key = ["Cargo.lock", ".prototools"]   # files whose content keys the cache
paths = ["target"]                    # ignored paths it restores and saves
```

- **Key.** A hash of the declared `paths` and the content of each `key`
  file in the candidate's tree (a missing file hashes as missing). The
  common dir already scopes it to the repository.
- **Where.** `<common>/sluice/caches/<hash of paths>/<key>/`, each saved
  whole or not at all (written beside, then renamed).
- **Restore**, in `materialize`, once the checkout exists: the cache with the candidate's key, or else the one
  of those `paths` saved last (as `actions/cache`'s restore keys), cloned
  into the place: `/bin/cp -c -p -R` on macOS (blocks shared until
  written, times kept, which cargo's fingerprints read; by path, since
  a toolchain's GNU `cp` may come first on `PATH` and has no `-c`), `cp
  -a --reflink=auto` on Linux. None yet: the place starts cold.
- **Save**, in `release` of a place whose candidate passed, before the
  checkout is removed: the declared `paths` move out of the place into a
  new cache of the candidate's key, unless that key is saved already (a
  saved cache never changes). A failed, dropped or recovered place saves
  nothing. The last two keys of each cache are kept; older ones go.
- **Never a verdict.** Restoring runs nothing, and a cache only spares
  work its tool redoes when the cache is stale: cargo rebuilds what its
  fingerprints say changed. A cache of something that would turn a run
  into a replay, such as moon's `.moon/cache`, is never declared.

The seam: `Checkout::release(name, passed)`, the core passing whether
the place's candidate passed (the verdict it read, never a run's own
report).

Measured on sluice's tests (`cargo test --no-run --features tools`):
cold, cargo took 162 s; with `target/` built by the same compiler cloned
in (915 MB in 8 s), 0.6 s and nothing compiled, the workspace's own crates
included. A `target/` built by another compiler saved little (85 s, 112
of 148 crates compiled), so `.prototools`, which names the compiler, is
in the key.

### `archive` (in C.1, not built)

```
mkdir -p $TMPDIR/sluice/<repo id>/<name> && git -C <common> archive <candidate> | tar -x -C $TMPDIR/sluice/<repo id>/<name>
rm -rf $TMPDIR/sluice/<repo id>/<name>        # release, after the run's group is ended
```

- No `.git`, no submodules, and **it honours `export-ignore` and
  `export-subst`**: a file marked `export-ignore` was absent. So the
  archive is not exactly the candidate's tree (33, 35). The exact tree
  with no git metadata:

  ```
  GIT_INDEX_FILE=<tmp>/index git read-tree <candidate> && GIT_INDEX_FILE=<tmp>/index git checkout-index -a -f --prefix=<dir>/
  ```
  (the trailing `/` is required; verified to include the ignored file).
  Use that, and call the method "archive" still.
- Not under the common dir: moon walks up looking for a VCS root, and
  one found above the place would be the repository's. `<repo id>` is a
  hash of the canonical common dir, so two repositories' places never
  collide (32).
- moon's `--affected` does not work without `.git` (section 3): targets
  come from the probe worktree.

### Acceptance tests

In a scratch repository:

1. `materialize` gives a place whose tree equals the candidate's (`git diff --stat <candidate>` empty).
2. `tmp` exists, is empty and is outside the checkout.
3. `release` removes everything listed as left behind, and a second `release` succeeds.
4. `release` of a name that never existed succeeds and touches nothing else.
5. A `post-checkout` hook is not run, and a place left by a crash is cleaned and made again.
6. No session's worktree, and no branch, is changed by either call (core.16, S11).
7. With a `[[checkout.cache]]`: the first place starts with none of its `paths`; released as passed, its `paths` are saved and the next place with the same key holds them, with their times; released as failed, nothing is saved; a place whose key differs takes the cache saved last; a third key removes the first.
8. A method other than `"git-worktree"` is refused, naming the one there is.

---

## 6. State on disk

**Implements** `State::lock`, `read`, `write`, `branches`, `allocate_id`,
`results`, `record_results` in the `local files` profile (C.6). **Bears
on** 26, 51, 53, 58, 61 to 63, 69, S10; core.11, core.14; ADR 0005,
ADR 0007.

### Where

`<common>` is `git rev-parse --path-format=absolute --git-common-dir`, or
gix's `repo.common_dir()` canonicalized (section 1). It is the same
directory from every worktree of the repository, so every place on the
host reads the same records (26, core.11).

A branch's records are `<common>/sluice/<branch>/`, the branch name's
`/` nesting directories, as
`docs/examples/state/bolt/concurrent-units/`. Files, by pass 1's
`records.rs`:

```
<common>/sluice/bolt/concurrent-units/
  version              "57\n": bumped on every write
  branch.json          the queue's own state (pass 1's Q18)
  submissions/<id>.json
  log.jsonl
  lock                 the branch's lock (never deleted)
  .pending.json        a write in progress (below); absent between writes
<common>/sluice/results.jsonl   the repository's results log (69)
<common>/sluice/.next-id        the next submission id, repository-wide (pass 1's Q5)
<common>/sluice/.lock           guards .next-id and the probe worktree
```

Two branches cannot have colliding directories while both exist (git
forbids `bolt` and `bolt/x` as branches at once), but records outlive
their branch. `write` refuses a branch whose directory path passes
through another branch's (an ancestor directory holding a `version`
file) and one whose name is `results.jsonl`; `branches()` lists the
directories holding a `version` file.

### The lock (ADR 0005, core.14)

`std::fs::File::{lock, try_lock, unlock}` are stable since Rust 1.89;
on macOS and Linux alike they are `flock(2)` (the same std code for
both; Linux's `/proc/locks` shows `FLOCK`), released by the kernel when
the holder dies. No crate
(`fs4`, `fs2`, `rustix`) is needed. `try_lock` returns `Result<(), std::fs::TryLockError>` with
`WouldBlock` and `Error(io::Error)`. Measured with a std-only probe:

```
try (free)                    -> acquired
hold 3s & ; try               -> WouldBlock: another step holds the branch
after holder exits; try       -> acquired
hold 30s; kill -9 holder; try -> acquired   (the kernel releases flock on death: no stale lock)
```

Open with `OpenOptions::new().create(true).truncate(false).write(true)`;
never delete the lock file (a delete races a concurrent open, and two
steps could each lock a different inode). `State::lock` maps
`WouldBlock` to `Ok(None)`: a step that finds the lock held leaves.
`State::wait_lock` blocks in `File::lock()` instead: a write outside a
step (a submission, an amendment, a withdrawal) waits out the round in
progress and then writes.
flock is per host; it is not for several hosts (57 to 63 are not phase
1).

### Atomic writes and the version check (61 to 63)

One file, atomically:

```rust
fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = dir.join(format!(".{name}.tmp.{}", std::process::id()));
    let mut f = File::create(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    std::fs::rename(&tmp, dir.join(name))?;
    File::open(dir)?.sync_all() // the rename itself, durable
}
```

Verified on macOS APFS and on Linux overlayfs, tmpfs and a FUSE host
share: `File::open(dir)?.sync_all()` succeeds on each. `sync_all` is
`fcntl(F_FULLFSYNC)` on macOS, slow but durable, and `fsync` on Linux.
Some Linux network and FUSE filesystems answer a directory fsync with
`EINVAL`; treat that as "not supported", not as a failed write. The temp file
is in the same directory so the rename is atomic. (`name` with a `/`, as
`submissions/12.json`, uses that subdirectory as `dir`.)

A `write(branch, read_at, changes)` touches several files (`put`:
`branch.json`, `submissions/<id>.json`; `append`: `log.jsonl`) and must be
all or nothing (63). Under the branch's lock:

1. Read `version`; if it is not `read_at`, return `Written::Refused`
   (62).
2. Write `.pending.json` atomically: `{"version": read_at + 1, "put":
   {path: base64}, "append": {path: base64}, "lengths": {path: <length
   before append>}}`.
3. Apply it: each `put` by `write_atomic`; each `append` by truncating
   the log to its recorded length, then `OpenOptions::append`, `write_all`
   of whole lines, `sync_all`.
4. `write_atomic` of `version`.
5. Delete `.pending.json`, sync the directory. Return
   `Written::Written(read_at + 1)`.

`read` and `lock` first finish any `.pending.json` left by a crash
(steps 3 to 5; the truncate makes the append idempotent). A reader with
no lock (`sluice status` while nothing runs, 26, 51) reads the files and,
if `.pending.json` exists, overlays it: what it reads is the state
sluice last wrote. `write` holds the lock already (`State::lock`); a
caller that did not take it gets an error, not a write.

`allocate_id`: under `.lock`, read `.next-id` (absent is 1), write it
plus one atomically, return the read value.

`results` reads `results.jsonl`; `record_results` appends whole lines
under `.lock` (pass 1's Q14: written before the branch's write that
carries the verdict, never appended twice: skip a line already present
for the same run).

### Acceptance tests

1. Two processes: the second's `lock` returns `None` while the first holds it; `kill -9` of the first frees it.
2. Two `write`s from one version: one `Written`, one `Refused` (62, 58).
3. A crash injected after each of steps 2, 3 and 4 (a test hook that aborts the process): the next `read` sees the whole write or none of it, and `log.jsonl` has no duplicated or partial line (63).
4. `read` with no lock and a `.pending.json` present gives the pending state (51).
5. The bytes written equal pass 1's fake store's bytes for the same `Changes` (53).
6. Branch `bolt/x` and `main` and `a/b/c` round-trip; `results.jsonl` as a branch name is refused.
7. Records are readable from a linked worktree with no sluice process (26).

---

## 7. The CLI and `wt-submit`

**Implements** the `worktrunk` driver profile: `Driver::session`,
`Driver::notify`, and the `wt-submit` binary (C.5). The CLI's commands
and exits are pass 1's (`crates/sluice/src/lib.rs`: `LANDED = 0`, `FAILED
= 1`, `CONFLICT = 2`). **Bears on** 1, 2, 3, 20, 47, 47a, 48 to 51, 54,
64, 67; core.1, core.4.

### How `wt submit` reaches `wt-submit`

Verified with a dummy `wt-submit` first on `PATH`:

- `wt submit bolt/x --no-wait --check` runs `wt-submit` with argv `bolt/x
  --no-wait --check`: everything after `submit`, verbatim; `--help` is
  forwarded too.
- wt consumes its own global flags: `wt -C /private/tmp -v --yes submit
  bolt/x` gave argv `bolt/x` and cwd `/private/tmp`. `-C` becomes the
  child's cwd; `-v` and `--yes` are not forwarded.
- cwd is the caller's, or `-C`'s. stdin, stdout and the tty pass
  through. The environment is inherited unchanged; through the zsh
  wrapper `WORKTRUNK_DIRECTIVE_CD_FILE` is set (unused by sluice).
- The child's exit code is wt's (2 gave 2, 7 gave 7).
- A built-in name shadows `PATH`: a `wt-list` was ignored.
- With no `wt-submit` on `PATH`: `error: unrecognized subcommand
  'submit'`, **exit 2, the same as a conflict**. `wt-submit`'s output
  always ends with a line sluice wrote (the walkthrough's), so a reader can
  tell; a driver in code checks `command -v wt-submit` first.
- `wt --help` does not document custom subcommands.

So `wt-submit` is `sluice submit` with the same arguments and
`--worktree .` (pass 1 builds it as a second binary forwarding to
`submit`). C.5's `wt submit --withdraw` and `--status` map to `sluice
withdraw` and `sluice status`; pass 1's CLI does not have them as
`submit` flags yet.

### `Driver::session(worktree)`

| field | from | captured |
|---|---|---|
| head | `git -C <worktree> rev-parse HEAD` (or gix) | |
| branch (`from.worktree`, as the walkthrough's `"worktree": "unit/batch-review"`) | `git symbolic-ref -q --short HEAD` | detached: exit 1, no output (`--abbrev-ref HEAD` prints `HEAD`): refuse, exit 2, `submit from a branch: <worktree> is detached` |
| agent (`from.agent`) | `SLUICE_AGENT` from the environment; else the branch name | crew sets `CREW_AGENT` (e.g. `crw-1-unit-1`) and `CREW_LABEL` in every role's environment (`crew/plugin/bin/crew-role:18`, restored by `plugin/hooks/session-start:40-55`); core.1 forbids naming crew here, so crew's role environment exports `SLUICE_AGENT=$CREW_AGENT` |
| dirty (2) | `git -C <worktree> status --porcelain=v1 -z --untracked-files=all` | below |

The dirty check, raw bytes (`od -c`, trimmed):

```
D  .config/wt.toml\0MM .gitattributes\0 M a.txt\0R  b-renamed.txt\0b.txt\0A  staged.txt\0?? .config/wt.toml\0?? sp ace.txt\0?? sub/new.txt\0
```

Records are `XY<space><path>\0`. When X or Y is `R` or `C`, a second
NUL-terminated path follows, and in `-z` it is **new then old**
(`R  b-renamed.txt\0b.txt\0`; without `-z` it prints `R  b.txt ->
b-renamed.txt`, old first). `-z` does no quoting (`sp ace.txt` raw;
without `-z`, `"sp ace.txt"`). Ignored files are excluded. Without
`--untracked-files=all` an untracked directory collapses to `?? sub/`.
Any record refuses the submission with exit 2, naming each path (2; pass
1's Q2 puts the refusal in the core, on `session`'s report):

```
refused: unit/touches holds changes that are not committed
  M a.txt
  ?? sub/new.txt
commit or remove them and submit again
```

### Waiting, `--no-wait`, `--check`

As pass 1 built it: a waiting submit loops `step(branch)` then reads its
submission's state, pausing between rounds (`Sleep`, a few seconds, walkthrough
section 6), until `landed` (exit 0), `failed` (1), `conflict` (2) or
`withdrawn` (1). `--no-wait` prints the submission line (or `{"id",
"branch", "ahead"}` with `--json`) and exits 0. `--check` forms with no
entry written (64, 47a) and exits 2 on `would conflict: …`, 0 otherwise.
A refused submission (dirty, detached, no `[branches]` table) exits 2.

What pass 2 adds:

- Ctrl-C while waiting ends only the wait: the submission stays queued
  (5), its run continues (section 4's `setsid`), and `sluice status <id>`
  reads it later. sluice prints `still queued: sluice status <id>` and
  exits 130. `wt` runs `wt-submit` as its child, so the terminal's SIGINT
  reaches both, as it does every process of the foreground group.
- A waiting submit's step may start runs and land other sessions' work;
  that is pass 1's Q1.

### `Driver::notify` (20, 50, 51)

For this profile the outcome of a waiting submit is its exit and its
last lines; the notification's `id` is printed in `--json` so a repeat
after a crash is known as one. For a `--no-wait` submission, `notify`
records nothing beyond the core's record (`told`); the session reads it
with `sluice status`. Prompting a session's pane through the multiplexer
is a later driver profile's (core.1: no multiplexer named here).

### Acceptance tests

1. `wt submit <branch>` from a clean worktree reaches `sluice submit` with that worktree, and exits with the outcome (S1, S3, S2: 0, 1, 2).
2. Dirty worktrees (modified, staged, untracked in a new directory, renamed, a path with a space): exit 2 and every path named, nothing submitted (2).
3. Detached HEAD: exit 2, nothing submitted.
4. `--no-wait` exits 0 at once; `sluice status <id>` later shows the outcome.
5. `--check` writes no submission and moves nothing (64).
6. `SLUICE_AGENT=x` sets `from.agent`; unset, the branch is used.
7. A second submit from the same worktree withdraws the first, which reads `withdrawn` (67).
8. SIGINT during a wait leaves the submission queued and its run running.

---

## 8. End to end

One script, `e2e.sh`, builds a repository with a moon workspace, a
`bolt/*` policy, a branch per scenario and three sessions' worktrees,
and then either proves the fixture with git and moon alone (`fixture`)
or drives S1 to S4 through `wt submit` and `sluice` and checks the
outcome (`sluice`). `E2E_REPO=crew` runs the same against a throwaway
clone of crew, with every moon and sluice call inside crew's devenv
shell.

Verified: `fixture` passes on the scratch repository (moon 2.5.3 and
moon 2.6.1, bash 3.2) and on the crew clone (through `devenv shell --`,
crew's moon 2.5.3). The fixture itself needs only moon 2.5, so it keeps
that floor while crew runs 2.5.3; the `sluice` half needs the profile's
2.6.

```
$ MOON=/nix/store/1vc28y8m…-moon_cli-2.6.1/bin/moon /bin/bash e2e.sh /tmp/…/run4 fixture
ok    S1 a: pass [{"t":"alpha:test","s":"passed"},{"t":"beta:test","s":"passed"}]
ok    S1 a+b: pass [{"t":"alpha:test","s":"passed"},{"t":"beta:test","s":"passed"}]
ok    S2 a+c forms: conflict alpha/notes.md
ok    S2 a: pass [{"t":"alpha:test","s":"passed"},{"t":"beta:test","s":"passed"}]
ok    S3 a: pass [{"t":"alpha:test","s":"passed"},{"t":"beta:test","s":"passed"}]
ok    S3 a+b: fail [{"t":"alpha:test","s":"passed"},{"t":"beta:test","s":"failed"}]
ok    S4 a: fail [{"t":"alpha:test","s":"failed"},{"t":"beta:test","s":"skipped"}]
ok    S4 b formed again without a: pass [{"t":"alpha:test","s":"passed"},{"t":"beta:test","s":"passed"}]

$ E2E_REPO=crew /bin/bash e2e.sh /tmp/…/crew1 fixture
ok    S1 a: pass [{"t":"tests:t-docs","s":"passed"}]
ok    S1 a+b: pass [{"t":"tests:t-docs","s":"cached"},{"t":"tests:t-after","s":"passed"}]
ok    S2 a+c forms: conflict tests/t-docs.sh
ok    S2 a: pass [{"t":"tests:t-docs","s":"passed"}]
ok    S3 a: pass [{"t":"tests:t-docs","s":"cached"}]
ok    S3 a+b: fail [{"t":"tests:t-docs","s":"cached"},{"t":"tests:t-after","s":"failed"}]
ok    S4 a: fail [{"t":"tests:t-docs","s":"failed"}]
ok    S4 b formed again without a: pass [{"t":"tests:t-after","s":"cached"}]
```

The script is `e2e/e2e.sh`. Its `sluice` half passes on the scratch
repository with git-worktree places and with worktrunk places (moon
2.6.1, every check `ok`, exit
0): S1 lands both in order, each landing's second parent the session's
head; S2 tells the conflict before any check and lands the first; S3
blames the second alone; S4 fails the first, re-forms the second without
it and lands it.

Notes for whoever runs it:

- Every branch is created before sluice protects `bolt/*`.
- On a branch with no verdict yet, the first candidate runs every
  required check (70). For crew that is the whole suite, 59 tasks,
  8 m 48 s on this host from cold; with crew's shared outputs it replays
  whatever any worktree of the clone already passed.
- `E2E_REPO=crew` needs crew on moon 2.6, the moon profile's floor.
- crew's tests need `openspec` on `PATH` (`tests/run`), as every host
  has it.

---

## 9. sluice's own checks, by tier

sluice gates its own branches with the tiers below, as moon tags a
branch's policy requires, once pass 2's profiles can land them. Each
tier is slower than the one before and needs more of the host.

| tier | tag | what | needs | time |
|---|---|---|---|---|
| fast | `check-fast` | the cargo tests: units, the conformance suite over the fakes, the CLI over the fakes; the citations check; `cargo fmt --check`; clippy `-D warnings` | cargo | seconds |
| tools | `check-tools` | each section's acceptance tests on scratch repositories; the conformance suite over the real profiles (below) | git, moon, worktrunk, a writable temp dir | tens of seconds |
| end to end | `check-e2e` | `e2e.sh sluice` on the scratch workspace; `E2E_REPO=crew` against a clone of crew | the above, and crew's devenv for the crew variant | minutes; crew cold is 8 m 48 s, replayed about a second |

sluice's own `.config/sluice.toml`:

```toml
[tests]
runner = "moon"
toolchain = ["devenv", "print-dev-env"]

[checkout]
method = "git-worktree"

[[checkout.cache]]
key = ["Cargo.lock", ".prototools"]
paths = ["target"]

[executor]
kind = "sandbox"
network = "localhost"

[branches."bolt/*"]
required = ["#check-fast"]
trust = ["open", "sandbox", "container", "cache"]

[branches.main]
required = ["#check-fast", "#check-tools", "#check-e2e"]
batch = true
trust = ["confined", "sandbox"]
```

The tasks:

- each crate's cargo tests, and the citations check, tagged
  `check-fast` (and `cargo-test`);
- the workspace's own project (`root`, source `.`, `moon.yml` at the
  top): `format` (`treefmt --ci`: every formatter `treefmt.nix`
  configures, failing on any change and keeping no cache outside the
  tree) and `lint` (`cargo clippy --workspace --all-targets -- -D
  warnings`), tagged `check-fast`, so the gate holds a candidate to what
  the commit hooks hold a commit to;
- `cli:tools`, tagged `check-tools`: the acceptance tests that need a
  real tool (`cargo test -p sluice --features tools --test …`), the
  conformance scenarios on the real profiles among them;
- `cli:e2e`, tagged `check-e2e`, never replayed: `e2e/check.sh` builds
  `sluice` and `wt-submit`, puts them first on `PATH`, and runs `e2e.sh
  <scratch> sluice`. The scratch repository is removed when it passes
  and kept, its path printed, when it fails.

`cargo-tests` stays the fast tier and runs before every `wt merge`;
`moon run ':#check-fast' ':#check-tools' ':#check-e2e'` is what a person
runs before landing a profile change, until sluice gates `main`.

### The conformance suite over the real profiles

The statement asks for "the profiles of Part C, each passing the suite"
(section 12). Scenarios run on a **`Rig`**, a trait in
`sluice-conformance`: what every binding can do. It makes a session's
place with work on the branch's tip, or on another head, or with files
beyond its commits; makes a named check fail when a path holds given
content; steps and settles the branch; and reads the branch, its commits
and trees, the records, the results log, and what the core did through
the seams (runs requested and started, places made and released, moves,
what each session was told).

- **The fakes' rig** is `Harness`, over `sluice_fakes::World`. What only
  the fakes can stage stays on `World` (a crash between two writes, a
  held or lost run, an interfering write, a host limit), and the tests
  that need one run on the fakes alone.
- **The real rig** (`crates/sluice/tests/profiles/conformance.rs`, in
  `cli:tools`) binds the real profiles to a scratch repository holding a
  moon workspace of three projects, each with a `test` task tagged
  `gate`: local refs, local files, git-worktree places and the moon
  runner under the supervisor. Each task runs a script that fails when a
  rule the test wrote names it and a path holds the rule's content, so
  both rigs give the same verdicts. It wraps the place, check, branch
  and driver profiles to record what the core asked of them, and names
  each session's agent as `SLUICE_AGENT` would (the tests share one
  process's environment).
- **`sluice_conformance::shared`** holds the twenty scenarios that need
  no fault, each written once over any rig. The fake tests in
  `sluice-conformance/tests/` call them and carry the `satisfies:`
  citations; the real rig runs each one too. Where timing differs, the
  scenario allows both: on real tools a run whose candidate ahead has
  already failed may be ended before it reports, and a round may find
  the branch's lock busy and be taken again, as `sluice run` does.

### Acceptance tests

1. `moon run ':#check-fast'` runs in under a minute on a warm cache and needs no tool beyond cargo.
2. `moon run ':#check-tools'` with git, moon or worktrunk missing fails naming the tool, never passes by skipping.
3. Every conformance test that does not need a fake-only fault passes over the real rig.
