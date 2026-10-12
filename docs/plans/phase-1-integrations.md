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
| moon in sluice's devenv | 2.6.1 | `/nix/store/1vc28y8m628iwc0z2qz4yll877i45r0m-moon_cli-2.6.1/bin/moon` | moonrepo's release build, pinned in `devenv.nix`; `devenv shell -- moon --version` |
| proto in sluice's devenv | 0.63.1 | `devenv.nix`, as moon | moonrepo's toolchain manager; moon runs the shell's tools, never proto's (`MOON_TOOLCHAIN_FORCE_GLOBALS`) |
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
4. A match: print `error: <b> is protected by sluice; use wt submit` and
   a second line, `the worktree may hold the attempted change: git merge
   --abort or git reset --hard`; append the attempt to the branch's log;
   exit 1.

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
branch -f`, `git branch -m bolt/x bolt/y`, `git fetch . a:bolt/x`, and
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
The branch's records stay readable after the delete (26).

**Logging the attempt (clause 1: "refused and reported").** The hook
appends `{"at":…,"event":"refused","ref":"refs/heads/bolt/x","from":<cur>,"to":<new>,"by":"<parent command>"}`
to the branch's `log.jsonl` through the file store (section 6), taking
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

1. Each command in the "caught" list is refused with exit 128 (wt: 1), the branch unmoved, one `refused` event in the log.
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
2.6.1, where `MOON_TOOLCHAIN_FORCE_GLOBALS` from the shell also passes
through), and reads `MOON_*` as
options (`MOON_BASE`, `MOON_HEAD`, `MOON_AFFECTED`, `MOON_FORCE`,
`MOON_CACHE`, `MOON_ON_FAILURE`, `MOON_CONCURRENCY`, …). A sluice started
from inside a moon task (a test of sluice, a waiting `wt submit` run by a
task) would hand those on. Clear every inherited `MOON_*` and set only
the ones sluice means, as crew's `tests/suite:62-66` does. crew's devenv
sets `MOON_TOOLCHAIN_FORCE_GLOBALS=true`; that comes with the shell.

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
`USER`, `LOGNAME`, `SHELL`, `TMPDIR`, `LANG`/`LC_*`, `TERM` from sluice's
own environment; `SLUICE_CHECKOUT=<place path>`, `SLUICE_TMP=<common>/sluice/.tmp/<place>`
(made empty by `materialize`); nothing of `MOON_*`, `GIT_*`
(`GIT_DIR`/`GIT_WORK_TREE` from a hook would point git at the wrong
repository), `DEVENV_*` or `DIRENV_*` (a waiting `wt submit` runs in a
session's shell, and that session's place must not leak into the
candidate's, 35).

**The shell a run happens in.** A repository's tools often exist only
inside a shell it declares: crew's moon and replay cache only inside
`devenv shell`. The policy names it:

```toml
[tests]
shell = ["devenv", "shell", "--"]
```

The executor runs `shell + argv` in the place, so the shell sets the
tools up and the command runs inside it. The environment above is what
goes into the shell; what the shell adds is the repository's. The shell
only enters the toolchain: sluice treats it as opaque and reads no
confinement or limit from it, which is the executor's (below). Read from
the branch's revision like the rest of the policy (56). When sluice
itself already runs inside the repository's shell, as an app does under
a manifest that declares one, the key is left out: the run keeps that
shell's `PATH` and, as above, nothing that names a session's place (35).

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
network = false
memory = "8G"
# processors come from the instance: available_parallelism() / B
```

| kind | how | enforces |
|---|---|---|
| `open` | the supervisor above: a process group in the place | the processors, through the runner's threads; nothing else; never confined |
| `sandbox` | the command under the host's kernel sandbox, as Anthropic's sandbox-runtime and Codex's sandbox do: Seatbelt (`sandbox-exec` with a generated profile) on macOS, bubblewrap with Landlock on Linux; writes allowed only to the place and `SLUICE_TMP`, the network only as the policy says | paths and network on both; memory and processors only on Linux, by starting the run in a cgroup (`systemd-run --user --scope -p MemoryMax= -p CPUQuota=`) |
| `container` | the place mounted into a new container (`docker run --rm --init --cpus --memory --network none -v <place>:<place> -w <place> <image>`), the image the repository declares | processors, memory, paths and network |

A limit the policy states that the executor cannot enforce refuses the
run (84): a `memory` limit under `sandbox` on macOS is refused with the
reason, never run without it. Every run's record carries `executor`
(`kind`, `confined`, and each limit held), so a gate that trusts only
confined runs (29) reads it from the record and never from the policy.
`confined` is true only for `container`, and for `sandbox` when no
session on the host can write the place; `open` never is. Containers
run with `--init` so orphans are reaped and a group's end is seen
(section 4's Linux facts).

**What a branch trusts.** Which runs a gate accepts is the policy's
(27), named by what the record says, never by what the policy asked
for:

```toml
[branches.main]
required = ["#check-fast", "#check-tools", "#check-e2e"]
trust = ["confined"]            # the default for a shared branch (ADR 0007)

# On one Mac, until runs are confined: main accepts sandboxed runs too.
# trust = ["confined", "sandbox"]
```

`trust` lists `confined` or executor kinds (`open`, `sandbox`,
`container`); a result counts when its record's `confined` is true and
`confined` is listed, or its `kind` is listed. A bolt branch's default
is `["open", "sandbox", "container"]`. Widening `main` to `sandbox` is
how it lands through sluice on one Mac today: the records still say
`confined: false`, so the day the agents are fenced off from sluice's
places (or the runs move to a container), narrowing `trust` back to
`confined` is one line and every earlier result reads for what it was.
Like every key of the policy, `trust` is read from the branch's own
revision, so a submission cannot widen the gate it is judged by (56).

The supervisor wraps every kind: it spawns the executor's command line
(`sandbox-exec …`, `bwrap …`, `docker run …`) as the group leader and
ends the group at the time limit as above; a container is also removed
by name (`docker rm -f sluice-<place>`), since its processes live in the
engine, not in the group.

#### Acceptance tests

1. `sandbox` on macOS and on Linux: a write outside the place and `SLUICE_TMP` fails inside the run; with `network = false`, a connection fails (83).
2. `sandbox` with `memory = "1G"` on macOS: the run is refused, naming the limit; on Linux, a run that allocates past it is killed and read as failed (84).
3. `container`: a run over its time limit is ended and its container removed; nothing of it is left in `docker ps -a`.
4. Every recorded run carries the executor's `kind`, `confined` and limits; an `open` run is never read as confined (29, 84).

---

## 5. Checkouts

**Implements** `Checkout::materialize(name, rev) -> Place { name, path,
tmp }` and `Checkout::release(name)` in three profiles (C.1), selected by
`[checkout] method`. **Bears on** 25, 32 to 35, S11; core.10, core.16;
ADR 0009, ADR 0010.

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

### `worktrunk`

```
command wt -C <repo> switch --create sluice/try/<branch>/<n> --base <candidate> --no-cd --yes --format=json </dev/null
command wt -C <repo> remove -D --force --foreground --yes --format=json sluice/try/<branch>/<n> </dev/null
```

(`<n>` is the place name; `-C` is a global flag; call the binary, not
the zsh function, which only adds `cd` handling.)

switch, exit 0, JSON on stdout, progress on stderr:

```
{"action":"created","branch":"sluice/try/2","path":"/…/wt/proj.sluice-try-2","created_branch":true,"base_branch":"4d819b0cfa180e7b72329bf94033ba70b2ae7f78"}
◎ Running pre-start project hook @ …/proj.sluice-try-2
✓ Created branch sluice/try/2 from 4d819b0… and worktree @ …/proj.sluice-try-2
◎ Running post-start: project @ …/proj.sluice-try-2
```

- `--base` takes a raw commit on no branch; the branch `sluice/try/<branch>/<n>`
  is created. It is not protected (no policy names it), so the hook
  allows it.
- **Read the path from the JSON**; never compute it. The user's
  worktree-path template (`~/.config/worktrunk/config.toml`) puts it at
  `<parent>/<repo>.sluice-try-<n>` for a normal clone and `crew/sluice-try-<n>`
  beside the agents' worktrees for crew's bare layout. `wt list
  --format=json` also has it (`items[].worktree.path`) but is slower and
  writes merge-state caches.
- Hooks in v0.80: `pre-switch post-switch pre-start post-start
  pre-commit post-commit pre-merge post-merge pre-remove post-remove`.
  `pre-*` block and a failure aborts; **`post-*` run detached**
  (`post-start` logged about 2 s after `switch` returned), logged under
  `.git/wt/logs/<branch>-<hash>/project/post-start/`. Only `pre-start`
  is done before tests start, and sluice waits for nothing more: what a
  check needs is in a `pre-start` hook or a dependency of the check (a
  moon task it depends on), as the walkthrough tells repositories. crew
  has neither today.
- **Without `--yes`**, an unapproved project hook makes switch exit 1
  with nothing created:

  ```
  ▲ proj needs approval to execute 2 commands: …
  ✗ Cannot prompt for approval in non-interactive environment
  ↳ To skip prompts in CI/CD, add --yes; to pre-approve commands, run wt config approvals add --yes
  ```

  `--yes` runs them unapproved and writes nothing to
  `~/.config/worktrunk/approvals.toml`. sluice passes `--yes`: its
  checkouts are worktrunk's like any other, so the repository's hooks
  run in them, and nobody can approve hooks in a directory sluice makes
  and removes on its own. The hooks come from the candidate's tree, the
  same trust as the checks sluice already runs there. (crew's rule that
  agents never pass `--yes` is about sessions, not sluice.)
- Existing branch: exit 1, `✗ Branch sluice/try/2 already exists`.
  Existing directory: exit 1, `✗ Directory already exists: …`
  (`--clobber` moves it to `<path>.bak.<timestamp>` and leaves that
  behind). After a crash, `materialize` of a name whose branch exists
  releases it first.

remove:

- `-D` deletes the branch even though unmerged (without it the result was
  `"branch_outcome":"retained_unmerged"` and the branch stayed); `--force`
  removes a dirty worktree (without it: `✗ Cannot remove worktree:
  sluice/try/1 has uncommitted changes`); **`--foreground` is required**:
  by default `wt remove` renames the directory into
  `.git/wt/trash/<dir>-<epoch>` and deletes it in the background, and here
  the trash was never deleted (the deleter died with the calling process's
  group, the likely cause). `--reap` (experimental) kills processes whose
  cwd is in the worktree; it does not replace ending the group.
- JSON: `[{"branch":"sluice/try/1","branch_checked_out_at":null,"branch_outcome":"deleted","kind":"worktree","path":"…/proj.sluice-try-1"}]`.
  Anything but `deleted` is a failed release. A branch with no worktree
  gives `{"kind":"branch_only","pruned":false,"branch_outcome":"deleted"}`
  after `○ No worktree found for branch sluice/try/1`.
- `pre-remove` runs in the worktree and blocks; `post-remove` runs in
  the background from the primary worktree.
- Left behind: no directory, no `.git/worktrees/<name>`, no branch; but
  `.git/wt/logs/commands.jsonl` (a line per hook),
  `.git/wt/logs/sluice-try-<n>-<hash>/` per checkout (never pruned by
  worktrunk, so `release` removes that checkout's log directory after
  `wt remove` succeeds) and
  `.git/wt/cache/{merge-tree-conflicts,merge-add-probe,is-ancestor,diff-stats}/*.json`.
- `sluice/try/*` worktrees appear in every agent's `wt list` while they
  exist.

### `archive`

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

For each method, in a scratch repository:

1. `materialize` gives a place whose tree equals the candidate's (`git diff --stat <candidate>` empty for the worktree methods; a file-by-file compare for archive, including an `export-ignore` file).
2. `tmp` exists, is empty and is outside the checkout.
3. `release` removes everything listed as left behind, and a second `release` succeeds.
4. `release` of a name that never existed succeeds and touches nothing else.
5. worktrunk: a `pre-start` hook has run when `materialize` returns; switch without approval and without `--yes` fails with the approval message; a stale `sluice/try/<branch>/<n>` from a crash is cleaned and recreated.
6. git-worktree: a `post-checkout` hook is not run.
7. No session's worktree, and no branch but `sluice/try/<branch>/<n>`, is changed by either call (core.16, S11).

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
  reads it later. Print `still queued: sluice status <id>` on SIGINT.
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
crew's moon 2.5.3); the `sluice` half is written against the
walkthrough's CLI and waits for pass 2. The fixture itself needs only
moon 2.5, so it keeps that floor while crew runs 2.5.3; the `sluice`
half needs the profile's 2.6.

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

Notes for whoever runs the `sluice` half:

- Every branch is created before sluice protects `bolt/*`.
- On a branch with no verdict yet, the first candidate runs every
  required check (70). For crew that is the whole suite, 59 tasks,
  8 m 48 s on this host from cold; with crew's shared outputs it replays
  whatever any worktree of the clone already passed.
- The `sluice` half assumes `sluice status <id|branch> --json` shapes:
  a submission as walkthrough section 7, a branch as `{"submissions":
  [...]}`. Adjust `status`/`settle` to what pass 1's `status_json`
  prints.
- crew's tests need `openspec` on `PATH` (`tests/run`), as every host
  has it.

```bash
#!/usr/bin/env bash
# sluice end to end: S1 to S4 of the statement, for real, on a scratch repository or a throwaway clone of crew.
#
#   e2e.sh <new dir> fixture   build the repository, then prove each scenario's fixture with git and moon alone:
#                              the candidates sluice must form, which conflict, which pass and which fail
#   e2e.sh <new dir> sluice    build the repository, then drive S1 to S4 through wt submit and sluice, and check
#
#   E2E_REPO=scratch (default)  a new repository: a moon workspace of two projects, beta depending on alpha
#   E2E_REPO=crew               a clone of crew (E2E_CREW=<repo>, default ~/Code/github_afterthought/crew/crew.git),
#                               every moon and sluice call made inside crew's devenv shell
#   E2E_CHECKOUT=git-worktree (default) | worktrunk | archive    the policy's [checkout] method
#   MOON=<path>                 the moon to use for the scratch repository (>= 2.5)
#
# Needs git >= 2.54 and jq; moon >= 2.5 (crew: devenv); in sluice mode, sluice and wt-submit on PATH.
# Written for bash 3.2.
set -euo pipefail

ROOT=${1:?usage: e2e.sh <new dir> fixture|sluice}
MODE=${2:?usage: e2e.sh <new dir> fixture|sluice}
VARIANT=${E2E_REPO:-scratch}
CHECKOUT=${E2E_CHECKOUT:-git-worktree}
MOON=${MOON:-moon}

die() { echo "e2e: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null || die "needs $1 on PATH"; }
need git; need jq
[ "$MODE" = sluice ] && { need sluice; need wt-submit; }
[ -e "$ROOT" ] && die "$ROOT exists; give a new directory"
mkdir -p "$ROOT/places"
ROOT=$(cd "$ROOT" && pwd -P)
REPO=$ROOT/repo
# moon's own variables never leak in from a caller's shell: moon reads MOON_* as options.
for v in $(env | sed -n 's/^\(MOON_[A-Z0-9_]*\)=.*/\1/p'); do unset "$v"; done
export GIT_AUTHOR_NAME=e2e GIT_AUTHOR_EMAIL=e2e@localhost GIT_COMMITTER_NAME=e2e GIT_COMMITTER_EMAIL=e2e@localhost

# inenv <dir> <command...>: run a command in a directory, inside crew's devenv shell for the crew variant.
inenv() {
  local dir=$1; shift
  if [ "$VARIANT" = crew ]; then (cd "$dir" && devenv shell -- "$@"); else (cd "$dir" && "$@"); fi
}

policy() { # the walkthrough's policy for bolt branches, with this run's checkout method
  cat <<EOF
[tests]
runner = "moon"

[checkout]
method = "$CHECKOUT"

[branches."bolt/*"]
required = ["#suite"]
parallel = 4
batch = false
flaky = "fail"
timeout = "20m"
EOF
}

# --- the repository ------------------------------------------------------------------------------------------------
if [ "$VARIANT" = scratch ]; then
  need "$MOON"
  git init -q -b main "$REPO"
  cd "$REPO"
  mkdir -p .moon .config alpha beta
  cat > .moon/workspace.yml <<'EOF'
$schema: 'https://moonrepo.dev/schemas/workspace.json'
projects:
  alpha: 'alpha'
  beta: 'beta'
vcs:
  client: 'git'
  defaultBranch: 'main'
telemetry: false
EOF
  cat > alpha/moon.yml <<'EOF'
$schema: 'https://moonrepo.dev/schemas/project.json'
tasks:
  test:
    command: 'sh test.sh'
    inputs: ['test.sh', 'value.txt', 'notes.md']
    tags: [suite]
EOF
  cat > alpha/test.sh <<'EOF'
#!/bin/sh
grep -qx ok value.txt || { echo "alpha: value.txt is $(cat value.txt)"; exit 1; }
echo "alpha ok"
EOF
  cat > beta/moon.yml <<'EOF'
$schema: 'https://moonrepo.dev/schemas/project.json'
dependsOn: ['alpha']
tasks:
  test:
    command: 'sh test.sh'
    inputs: ['test.sh', 'value.txt', 'notes.md', '/alpha/value.txt']
    deps: ['alpha:test']
    tags: [suite]
EOF
  cat > beta/test.sh <<'EOF'
#!/bin/sh
grep -qx ok value.txt || { echo "beta: value.txt is $(cat value.txt)"; exit 1; }
grep -qx ok ../alpha/value.txt || { echo "beta: alpha's value.txt is $(cat ../alpha/value.txt)"; exit 1; }
echo "beta ok"
EOF
  for p in alpha beta; do echo ok > $p/value.txt; printf '%s notes\n\nnothing yet\n' $p > $p/notes.md; done
  echo '.moon/cache/' > .gitignore
  policy > .config/sluice.toml
  git add -A
  git commit -qm "two moon projects, a test each tagged suite, and sluice's policy"
  # What each scenario edits: a file both a and c write (S2), and a test each of a (S4) and b (S3) breaks.
  A_FILE=alpha/notes.md A_LINE=3 B_FILE=beta/notes.md B_LINE=3
  B_BREAKS=beta/value.txt B_FAILS=beta:test A_BREAKS=alpha/value.txt A_FAILS=alpha:test
  BROKEN=broken
else
  git clone -q --no-local "${E2E_CREW:-$HOME/Code/github_afterthought/crew/crew.git}" "$REPO"
  cd "$REPO"
  git switch -q main 2>/dev/null || git switch -q -c main origin/main
  mkdir -p .config
  policy > .config/sluice.toml
  git add .config/sluice.toml
  git commit -qm "sluice's policy for bolt branches"
  # crew: a comment line of two tests; a test broken by an exit before it sources lib.sh.
  A_FILE=tests/t-docs.sh A_LINE=1 B_FILE=tests/t-after.sh B_LINE=1
  B_BREAKS=tests/t-after.sh B_FAILS=tests:t-after A_BREAKS=tests/t-docs.sh A_FAILS=tests:t-docs
  BROKEN='exit 1 # e2e: fails on purpose'
fi
case $(inenv "$REPO" "$MOON" --version) in
  "moon 2."[5-9]*|"moon 2."[1-9][0-9]*|"moon "[3-9]*) ;;
  *) die "needs moon >= 2.5, found $(inenv "$REPO" "$MOON" --version)" ;;
esac
# Every scenario's branch exists before sluice protects bolt/*: made here, and from now on moved only by sluice.
for s in s1 s2 s3 s4; do git branch "bolt/$s" main; done

# --- three sessions' places, each on a fresh unit branch per scenario ----------------------------------------------
place() { # place <session> <scenario>: the session's worktree on unit/<session>-<scenario>, off bolt/<scenario>
  local dir=$ROOT/places/$1
  [ -d "$dir" ] || git -C "$REPO" worktree add -q --detach "$dir" main
  git -C "$dir" switch -q -c "unit/$1-$2" "bolt/$2"
}
at() { git -C "$ROOT/places/$1" switch -q "unit/$1-$2"; } # at <session> <scenario>: the place back on that branch
edit() { # edit <session> <path> <line number> <text>: that line replaced, as one commit in the session's place
  local f=$ROOT/places/$1/$2
  awk -v n="$3" -v l="$4" 'NR==n{print l; next} {print}' "$f" > "$f.e2e" && cat "$f.e2e" > "$f" && rm "$f.e2e"
  git -C "$ROOT/places/$1" commit -qam "$1: $2"
}
breaks() { # breaks <session> <path>: the test reading that file made to fail, as one commit
  local f=$ROOT/places/$1/$2
  if [ "$VARIANT" = scratch ]; then echo "$BROKEN" > "$f"; else
    awk -v l="$BROKEN" 'NR==2{print l} {print}' "$f" > "$f.e2e" && cat "$f.e2e" > "$f" && rm "$f.e2e"; fi
  git -C "$ROOT/places/$1" commit -qam "$1: breaks $2"
}

# S1: a and b touch different files; both pass and land in order.
place a s1; edit a "$A_FILE" "$A_LINE" "# a was here"
place b s1; edit b "$B_FILE" "$B_LINE" "# b was here"
# S2: a and c write the same line of one file; c conflicts with a.
place a s2; edit a "$A_FILE" "$A_LINE" "# a says one thing"
place c s2; edit c "$A_FILE" "$A_LINE" "# c says another"
# S3: a passes; b breaks a test, which fails in b's candidate.
place a s3; edit a "$A_FILE" "$A_LINE" "# a was here"
place b s3; breaks b "$B_BREAKS"
# S4: a breaks a test; b passes once formed without a.
place a s4; breaks a "$A_BREAKS"
place b s4; edit b "$B_FILE" "$B_LINE" "# b was here"

FAILED=0
expect() { # expect <what> <actual> <glob>
  case $2 in $3) echo "ok    $1: $2" ;; *) echo "FAIL  $1: $2 (expected $3)"; FAILED=1 ;; esac
}

# --- fixture: what sluice must find, shown with git and moon alone -------------------------------------------------
# form <onto> <head>: the candidate commit, or "conflict <paths>" (ADR 0009: git merge-tree decides).
form() {
  local out=$ROOT/merge-tree.out rc=0
  git -C "$REPO" merge-tree --write-tree -z --name-only --no-messages "$1" "$2" >"$out" || rc=$?
  case $rc in
    0) git -C "$REPO" commit-tree "$(tr -d '\0' <"$out")" -p "$1" -p "$2" -m "sluice candidate" ;;
    # a shell variable cannot hold NUL, so the -z output is read from the file: the tree, then one path per
    # field up to the empty field that ends the conflicted-file section
    1) echo "conflict $(tr '\0' '\n' <"$out" | sed -n '2,$p' | sed '/^$/q' | tr '\n' ' ')" ;;
    *) die "merge-tree exited $rc" ;;
  esac
}
# verdict <branch> <candidate>: the candidate's affected #suite tasks run in a detached checkout; pass or fail
verdict() {
  local dir=$ROOT/check-$2 targets rc=0
  git -C "$REPO" worktree add -q --detach "$dir" "$2"
  targets=$(inenv "$dir" env MOON_BASE="$1" MOON_HEAD="$2" "$MOON" query tasks --tags suite --affected --downstream deep \
    </dev/null | jq -r '.tasks[][] | .target')
  if [ -z "$targets" ]; then echo "pass (nothing affected)"; else
    # shellcheck disable=SC2086
    inenv "$dir" "$MOON" exec $targets --on-failure continue </dev/null >"$dir.log" 2>&1 || rc=$?
    report=$(jq -c '[.actions[] | select(.label | startswith("RunTask(")) | {t: .node.params.target, s: .status}]' \
      "$dir/.moon/cache/runReport.json")
    if [ $rc -eq 0 ]; then echo "pass $report"; else echo "fail $report"; fi
  fi
  git -C "$REPO" worktree remove --force "$dir"
}

if [ "$MODE" = fixture ]; then
  r() { git -C "$REPO" rev-parse "$1"; }
  L=$(r bolt/s1); C1=$(form "$L" "$(r unit/a-s1)"); C2=$(form "$C1" "$(r unit/b-s1)")
  expect "S1 a" "$(verdict "$L" "$C1")" "pass*"
  expect "S1 a+b" "$(verdict "$L" "$C2")" "pass*"
  L=$(r bolt/s2); C1=$(form "$L" "$(r unit/a-s2)")
  expect "S2 a+c forms" "$(form "$C1" "$(r unit/c-s2)")" "conflict $A_FILE*"
  expect "S2 a" "$(verdict "$L" "$C1")" "pass*"
  L=$(r bolt/s3); C1=$(form "$L" "$(r unit/a-s3)"); C2=$(form "$C1" "$(r unit/b-s3)")
  expect "S3 a" "$(verdict "$L" "$C1")" "pass*"
  expect "S3 a+b" "$(verdict "$L" "$C2")" "fail*$B_FAILS\",\"s\":\"failed*"
  L=$(r bolt/s4); C1=$(form "$L" "$(r unit/a-s4)")
  expect "S4 a" "$(verdict "$L" "$C1")" "fail*$A_FAILS\",\"s\":\"failed*"
  expect "S4 b formed again without a" "$(verdict "$L" "$(form "$L" "$(r unit/b-s4)")")" "pass*"
  exit $FAILED
fi

# --- sluice: S1 to S4 for real -------------------------------------------------------------------------------------
COMMON=$(git -C "$REPO" rev-parse --path-format=absolute --git-common-dir)
submit() { # submit <session> <scenario> [flags...]: wt submit from the session's place, as its agent would
  local who=$1 s=$2; shift 2
  at "$who" "$s"
  SLUICE_AGENT="e2e-$who" inenv "$ROOT/places/$who" wt-submit "bolt/$s" "$@"
}
id_of() { submit "$1" "$2" --no-wait --json | jq -r .id; }
status() { inenv "$REPO" sluice status "$@" --json; }
settle() { # settle <scenario>: rounds until nothing on the branch is queued or under test, for at most an hour
  local i=0
  while status "bolt/$1" | jq -e '[.submissions[] | select(.state == "queued" or .state == "testing")] | length > 0' >/dev/null; do
    inenv "$REPO" sluice step "bolt/$1" >/dev/null
    i=$((i + 1)); [ $i -lt 720 ] || die "bolt/$1 did not settle"; sleep 5
  done
}
landed() { git -C "$REPO" log --reverse --first-parent --format='%(trailers:key=Sluice-Submission,valueonly)' "main..bolt/$1" | sed '/^$/d' | tr '\n' ' '; }

# S1: two sessions submit to one bolt branch; both candidates pass; both land, in order; neither ran a merge.
A=$(id_of a s1)
out=$(submit b s1) && rc=0 || rc=$?
expect "S1 b's exit" "$rc" 0
expect "S1 b's last line" "$(printf '%s\n' "$out" | tail -1)" "passed · landed as *"
expect "S1 a" "$(status "$A" | jq -r .state)" landed
expect "S1 landing order" "$(landed s1)" "$A $((A + 1)) "
for c in $(git -C "$REPO" rev-list --first-parent main..bolt/s1); do
  expect "S1 $c author" "$(git -C "$REPO" log -1 --format='%an <%ae>' "$c")" "sluice <sluice@localhost>"
  expect "S1 $c second parent" "$(git -C "$REPO" rev-parse "$c^2")" "$(git -C "$REPO" rev-parse unit/a-s1)|$(git -C "$REPO" rev-parse unit/b-s1)"
done
expect "S1 moves refused by the hook" "$(grep -c '"event":"refused"' "$COMMON/sluice/bolt/s1/log.jsonl" || true)" 0

# S2: the second head conflicts with the first; told the paths before any check runs; the first lands.
A=$(id_of a s2)
out=$(submit c s2 --check) && rc=0 || rc=$?
expect "S2 c's forecast exit" "$rc" 2
expect "S2 c's forecast" "$out" "would conflict: $A_FILE with #$A (e2e-a, unit/a-s2)"
out=$(submit c s2) && rc=0 || rc=$?
expect "S2 c's exit" "$rc" 2
expect "S2 c's conflict" "$(printf '%s\n' "$out" | sed -n 2p)" "conflict: $A_FILE"
C=$(status bolt/s2 | jq -r '[.submissions[] | select(.from.agent == "e2e-c")][0].id')
expect "S2 c was never tested" "$(status "$C" | jq -r '.tested // "none"')" none
settle s2
expect "S2 a" "$(status "$A" | jq -r .state)" landed

# S3: the first passes and the second fails; the second's session is told the failing checks; the first landed.
A=$(id_of a s3)
out=$(submit b s3) && rc=0 || rc=$?
expect "S3 b's exit" "$rc" 1
expect "S3 b's failure" "$(printf '%s\n' "$out" | grep '^failed:')" "failed: $B_FAILS"
expect "S3 a" "$(status "$A" | jq -r .state)" landed
expect "S3 a's result" "$(status "$A" | jq -r .result)" passed
expect "S3 one blame" "$(grep -c '"event":"blamed"' "$COMMON/sluice/bolt/s3/log.jsonl")" 1

# S4: the first fails and its session is told; the second is formed again without it, passes, and lands first.
A=$(id_of a s4); B=$(id_of b s4)
settle s4
expect "S4 a" "$(status "$A" | jq -r .state)" failed
expect "S4 a's failure" "$(status "$A" | jq -r '.failed[].task')" "$A_FAILS"
expect "S4 b" "$(status "$B" | jq -r .state)" landed
expect "S4 only b landed" "$(landed s4)" "$B "
expect "S4 b tested without a" "$(status "$B" | jq -c .tested.submissions)" "[$B]"
expect "S4 none of a's work on the branch" "$(git -C "$REPO" grep -c -F "$BROKEN" bolt/s4 -- "$A_BREAKS" || echo 0)" 0

# S11 in passing: sluice left every session's place alone.
for w in a b c; do expect "place $w kept" "$(git -C "$ROOT/places/$w" rev-parse --is-inside-work-tree)" true; done
exit $FAILED
```

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

```toml
# sluice's own .config/sluice.toml, once sluice lands sluice
[branches."bolt/*"]
required = ["#check-fast"]

[branches.main]
required = ["#check-fast", "#check-tools", "#check-e2e"]
batch = true
```

The cargo test tasks carry `check-fast` today. Pass 2 adds:

- a workspace-level moon project (`root`, source `.`) with `format`
  (`cargo fmt --all -- --check`) and `lint` (clippy), tagged
  `check-fast`, so the gate holds what the commit hook holds;
- a `tools` task per profile crate or module, tagged `check-tools`,
  running that section's acceptance tests (`cargo test -p <crate>
  --features tools`, or `#[ignore]`d tests run with `--ignored`, so a
  plain `cargo test` stays fast and needs no tool);
- an `e2e` task tagged `check-e2e`, `cache: false` for the crew variant,
  since it reads a repository outside this tree.

`cargo-tests` stays the fast tier and runs before every `wt merge`;
`checks-all` (`moon run ':#check-fast' ':#check-tools'`) is what a person
runs before landing a profile change, and the end-to-end tier runs by
hand or nightly until sluice gates `main`.

### The conformance suite over the real profiles

The statement asks for "the profiles of Part C, each passing the suite"
(section 12). Today the suite's `Harness` builds a `sluice_fakes::World`
and its tests script it directly (`h.w().fails_with(…)`,
`h.w().crash(…)`). Pass 2 splits that in two:

- **`Rig`**, a trait in `sluice-conformance`, holds what every binding
  can do: make a session's place with given files, read the branch's tip
  and tree, move the branch by hand, make a named check fail when a path
  holds given content, read the records' files, read what each session
  was told. The fakes implement it over `World`; the real rig
  implements it over a scratch repository in a temp dir, with the local
  refs branch, local files records, git-worktree places, the `command`
  runner and a driver that appends every notification to a file.
- **What only the fakes can stage** stays on `World`: a crash between
  two writes, a held or lost run, an interfering write. A test that
  needs one says so (`fn crash_after_move(h: &Harness<FakeRig>)`) and
  runs over the fakes alone.

The real rig scripts checks through the tree, as real repositories do:
the `command` runner runs a script that fails a named check when a path
holds the content `fails_with` named, so the same test sees the same
verdicts over both rigs. Hangs and holds become a check that sleeps
past the limit or waits for a file the test writes.

Every test is generic over the rig, and `tests/*.rs` instantiates it
twice: `mod fakes` (tier fast) and `mod real` behind the `tools` feature
(tier tools). The citations check counts a clause as cited once, whichever
rigs run it.

### Acceptance tests

1. `moon run ':#check-fast'` runs in under a minute on a warm cache and needs no tool beyond cargo.
2. `moon run ':#check-tools'` with git, moon or worktrunk missing fails naming the tool, never passes by skipping.
3. Every conformance test that does not need a fake-only fault passes over the real rig.
