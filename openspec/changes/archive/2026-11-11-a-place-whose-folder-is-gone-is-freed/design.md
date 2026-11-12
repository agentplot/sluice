# Design

## Context

**The brief.** A place whose working copy git no longer lists is freed.
Its release runs once from what sluice holds for it, what sluice holds
is let go whatever the outcome, and the place is recorded gone. An
operator's accepting a failed release frees it the same way.

**Sources.**
- Signal 2026-11-10-agentplot-ops-8ac92f49/01 (afterthought/crew-state,
  `agentplot/main`), agentplot-ops on deneb, 2026-11-10 14:56:10Z:
  "sluice holds its gate's try checkouts (sluice/try/main/63, 71 and 76
  on deneb's agentplot/sluice) after their release fails 4 times,
  because git worktree remove refuses a .places directory that git no
  longer lists as a worktree; release should free a checkout whose
  worktree is already gone".
- agentplot/blueprints `design/sluice/requirements.md` (origin/main
  d4db801): clause 123, "A place whose working copy is already gone is
  no one's, found so by the next round or run on its host before either
  does its own work … sluice runs its release once, from what it itself
  holds for the place and with the place's ports, records how that went,
  frees what it holds at once whatever the outcome, records the place
  gone, and never retries it", and its operator's acceptance, "accepting
  its failure and freeing what sluice itself holds for the place (25,
  34)". Clause 25, and clause 34, "release: given a place sluice made,
  its removal". C.1's git worktree row: release by `git worktree
  remove`.

**What deneb holds, read on 2026-11-11.** In
`~/.local/share/flywheel/repositories/agentplot/sluice/sluice.git`,
`sluice/main/log.jsonl` has the full story for `sluice/try/main/63`:
- `testing` at 14:22:12Z;
- `release_failed` at 14:22:47Z, try 1, `git worktree remove …: error:
  failed to delete '…/sluice+2Ftry+2Fmain+2F63': Directory not empty`;
- `release_failed` at 14:23:08Z, 14:23:37Z and 14:24:07Z, tries 2 to 4,
  each `fatal: '…' is not a working tree`, the last `held`.

`71` and `76` read the same. Each folder is still there, with about
2000 entries and its `.git` file naming
`sluice.git/worktrees/sluice+2Ftry+2Fmain+2F63`, which no longer
exists. `git worktree list` names none of them. `git status` in the
folder answers `fatal: not a git repository`. Each place's record names
its path and scratch. `branch.json` lists all three in
`failed_releases`, `stopped`.

git's `worktree remove` deletes the folder first and stops at the first
entry it cannot delete. It then deletes its own entry for the worktree
whatever that deletion did. So a removal that fails part way always
leaves a folder git no longer lists.

**sluice today, on this branch at 5c4e580e.**
- `crates/sluice/src/profiles/places.rs` `gone(name)`: true where the
  record names a path that is no directory. `preview_place` reads a
  preview's `gone` the same way, while its command has not failed.
  `session_places` reads a worktree's place gone where its worktree is
  no directory.
- `remove_worktree(path)`: returns where the path does not exist and
  `registered` says git does not list it. Otherwise it runs `git
  worktree remove --force <path>`. `registered` starts `git worktree
  list --porcelain -z` and compares each entry's path as a string.
  `release` (a candidate's), `free` (any place, nothing run),
  `release_gone` (through `free`) and `release_preview` all call it.
- `crates/sluice-core/src/release.rs` `release_made`: asks
  `checkout.gone`. Where it is gone it calls `release_gone` and logs
  `gone`; otherwise it calls `release`, and a failure counts a try.
  `release_held`, with `accept`, calls `free`, and a failure of it
  counts a try too.
- `crates/sluice-core/src/step.rs` `perform_releases`: each round
  releases a held place of this host's again where `held_gone` says its
  working copy has gone since.
- `crates/sluice-core/src/preview.rs` `release_unusable_previews`: a
  preview the profile lists `gone` is released by `release_gone`.
- `crates/sluice/src/runs.rs` `remove`: ends the run, runs its stored
  release and removes the run's directory before the profile removes
  the folder. So on deneb the release command had already run when git
  failed. `release_gone` finds no stored release left and runs nothing.
- `Places::checked_out` reads git's worktrees through gix
  (`repo.worktrees()`, `base()`), starting no process.

## Goals / Non-Goals

**Goals:**
- A checkout or a preview whose folder git no longer lists is read as
  gone, and freed as 123 frees a gone place: its stored release run
  once, what sluice holds let go, and `gone` logged.
- What is left of its folder is removed, by sluice, since git will not.
- `--accept-failure` frees such a place too.
- The three places deneb holds are freed by the next round on the
  sluice that carries this change, with nothing done by hand.

**Non-Goals:**
- A worktree's place whose folder git no longer lists (proposal, out of
  scope).
- A change to the core, the fakes or the conformance suite. The core
  already frees whatever the seam reads as gone.
- Removing a leftover in the same call as git's failed removal.
- A sweep of folders under `sluice/.places/` that no record names.

## Decisions

### D1. Gone is read as git lists worktrees

The profile gains one reading, `listed(path) -> Result<bool>`: whether
git lists the folder `path` as one of the repository's worktrees. That
is the repository's main worktree, where it has one, or the folder an
entry under `<common>/worktrees/` names. gix reads these from the
repository's files (`repo.workdir()`, `repo.worktrees()` and each
entry's `base()`), as `checked_out` already does. A folder is listed
where one of those names the same folder on disk: the same device and
inode by `symlink_metadata` of each. So `/var/…` and `/private/var/…`,
or a path spelled through a link, read as one folder. An entry whose
folder does not exist, or whose `gitdir` file cannot be read, names
nothing. A directory under `<common>/worktrees/` that cannot be listed
is an error, never read as unlisted.

`gone(name)` for a place sluice made, a candidate's or a preview's,
reads its record's path. It is gone where that path is no directory, as
today, or where `listed` says git lists no worktree there. A place with
no record, or one whose record names no path yet, is not gone, as
today. A reading that fails is an error. The core counts that as a
failed try (`release_made`), or as not gone (`held_gone`). It is never
read as gone. `preview_place` reads a preview's `gone` the same way,
while its command has not failed.

The reading starts no process and reads no object. It reads the
entries' `gitdir` files and stats their folders, and only where the
place's folder still exists (rule core.31).

Not taken:
- **`git worktree list` per reading.** That is a process per held
  place per round, which an idle `sluice run` must not start (rule
  core.31). It is also what `registered` does today, comparing spellings
  as strings.
- **Reading the folder's own `.git` file and the entry it names.**
  That answers only for a linked worktree, and it is a second reading
  beside the one `checked_out` already makes of git's list.
- **Asking git in the folder (`git -C <folder> rev-parse`).** The
  folders sit under the bare repository's own directory. Once their
  `.git` names nothing, git walks up and finds the repository itself.

### D2. One reading, one owner

`remove_worktree` asks `listed`, not `registered`, so the profile has
one answer to whether git lists a folder. `registered` is removed.

- Where git lists the folder, `git worktree remove --force <path>`, as
  today. That holds whether or not the folder is still there, as for a
  folder deleted by hand.
- Where git does not list it and the folder exists, sluice removes the
  folder itself (D3).
- Where git does not list it and no folder exists, nothing.

`remove_worktree` is only ever given `worktree_path(name)`, a place
sluice made under `<common>/sluice/.places/` or `.previews/`. Rule
core.20 holds: sluice removes no working copy it neither made nor
adopted, and it never removes a session's worktree.

### D3. What is left of the folder

git no longer lists the folder, so sluice removes it. It walks the
folder without following links. Each directory its owner cannot write
is made owner-writable, and then the whole folder is removed. A check
can leave a read-only tree, such as a copy out of the Nix store or a
module cache. git's own removal fails on one, and a plain
`remove_dir_all` would fail on the same entry.

A folder that still cannot be removed is a failure to free, as any
failure to free is today (`free`'s contract). Its words name the folder
and what refused it. In a round it counts a try, and the place is held
once its tries are spent, as any failed release is. The next rounds read
it as gone again. Each tries the free with nothing run, since its gone
release is noted as started. The operator's `--accept-failure` tries the
same free.

Not taken:
- **Freeing the place and leaving the folder.** The record, scratch and
  ports would go, and the folder would stay where no record names it
  and nothing removes it. A checkout of this repository holds thousands
  of files and its build's output. That needs a sweep (proposal, out of
  scope), and a word for the leftover in the `gone` line, which the log
  does not carry today.
- **Removing the leftover in the same call, right after `git worktree
  remove` fails.** A writer that made git fail is likely still writing.
  The next round reads the place as gone and takes 123's path, which
  logs `gone`, says how its release went, and runs the release once.

### D4. The release once, and the operator's acceptance

Neither needs code beyond D1 to D3. Once `gone` reads true:
- A round's `release_made` calls `release_gone`. That runs the release
  command the place's runs stored, once, from its scratch directory and
  with its ports, or nothing where `runs.remove` ran it already, as on
  deneb. It then calls `free`, which removes the leftover by D2 and D3,
  and the core logs `gone` with `ran` and `failed`. A place held before
  this change is gone since, for `perform_releases`, and is freed the
  same way by the next round.
- `sluice release <name>` runs `release_made` and takes the same path.
- `sluice release <name> --accept-failure` calls `free`, which runs
  nothing and removes the leftover by D2 and D3. The core logs
  `released` with `accepted`, as for any accepted place.
- A preview whose folder git no longer lists is released by
  `release_unusable_previews` through `release_gone`.

The words stand: "freed <name>: it no longer exists; …". A folder git
no longer lists is no checkout, and sluice has removed what was left of
it by the time it says so.

### D5. A worktree's place keeps its reading

`session_places` reads a worktree's place as gone only once its folder
is no directory, as today. sluice neither made the worktree nor removes
it (rules core.16, core.20), and its driver's `wt remove` or tidy
removes it. Reading a live agent's worktree as gone because git's list
and the record disagree would run its release and give its ports to the
next worktree while the agent still works. sluice's own `git worktree
remove` never runs on a worktree, so the incident cannot leave one
behind.

Not taken:
- **One reading for every kind of place.** It would take core.16's "a
  round or a run releasing a session's place whose worktree exists"
  further than the incident asks. It is a change of its own if a report
  shows a worktree git no longer lists left held.

### D6. The seam

`Checkout::gone`'s doc says: "its record names one, and that path is no
directory, or, for a place sluice made, a combination's checkout or a
preview, a folder git no longer lists as a worktree of the repository".
`Checkout::free`'s doc says the checkout sluice made is removed "by git
where git lists it, and otherwise what is left of its folder by sluice".
The trait's signatures are unchanged. The fake place seam already has a
deleted checkout read as gone (`delete_checkout`), which stands for both
cases. Rules core.1 and core.2 hold: git is named only in the profile.

### D7. The family

No other tool reads or releases sluice's places. Flume's teardown of its
own places calls `sluice release` and then `git worktree remove`, so it
could meet the same leftover. That removal is Flume's, of a place Flume
made. This change leaves it alone, and the stage reports it to the
orchestrator. The statement holds as written: clause 123's "working
copy is already gone" covers a folder git no longer lists, which is no
working copy. Reading it so is this change's, recorded in record 0115.

### D8. The record

Record 0115, or the next free number past those taken on main, this
bolt and the in-flight branches (0113 and 0114 are), is titled "A
checkout git no longer lists is gone, and sluice removes what is left
of it". It is accepted under clause 123 and the planner's brief. D1, D3
and D5 are the construct's own choices, each cheap to change. Rule
core.16's details cite the incident and the record.

### Rules relied on

- core.16: the core releases only places it made, and a session's only
  once its working copy is gone (D5).
- core.20: sluice removes no working copy it neither made nor adopted
  (D2).
- core.31: no process started and no object read on a reader's tick
  (D1).
- core.1, core.2: git only in the profile, the seam unchanged (D6).
- core.21: each task's check is one test or one project's task, which
  the gate runs.

## Risks / Trade-offs

- [A writer that keeps writing into the leftover] → its place stays
  held, and each round tries the free again with nothing run, logging
  `release_failed`. That is what a held gone place whose free fails
  does today. The overview names it with the folder and what refused
  it, and the operator stops the writer.
- [A reading that matches no entry for a folder git does list] → the
  place would be freed while git lists it. D1 compares folders by
  identity, not spelling, and reads only where the folder exists. A
  read that fails is never read as gone.
- [Making a check's read-only directories writable] → only inside a
  place sluice made and is removing, never through a link.

## Migration Plan

Nothing to migrate: no record or state changes form. On a host that
holds a place like deneb's three, the next round on the sluice that
carries this change frees it and logs `gone`.

## Task notes

**1.1 Record 0115.** Front matter as record 0112's: number 115, or the
next free one, title as D8, status accepted, dated the day it is
written. Decision-makers: the planner's brief of 2026-11-11 and the
construct of `a-place-whose-folder-is-gone-is-freed`. The status line
says clause 123 and the brief decide it, and that D1, D3 and D5 are the
construct's own, each cheap to change.
- Context: the signal's words, `main`'s log on deneb for
  `sluice/try/main/63` as Context gives it, and git's removal, which
  stops at its first failure and then drops its entry.
- Considered options: gone read from git's list by identity (D1);
  `git worktree list` per reading; the folder's own `.git` file; the
  place freed with its folder left (D3); every kind of place read the
  same way (D5).
- Decision outcome: D1 to D5 in brief, with the paths not taken.
- Consequences: deneb's three are freed by the next round; a leftover
  sluice cannot remove keeps its place held with what refused it; a
  worktree's place is read as before.

**1.2 Rule.** In core.16's details, add to "Rules out": "a checkout or
a preview sluice made held for good because git refuses to remove a
folder it no longer lists (deneb, 2026-11-10: `sluice/try/main/63`, 71
and 76 on agentplot/sluice, git's removal stopping at `Directory not
empty` and every try after it refused `is not a working tree`, signal
2026-11-10-agentplot-ops-8ac92f49/01; record 0115)". Add to its "Not
ruled out": "a checkout or a preview sluice made read as gone where git
no longer lists its folder, matched by the folder and not its spelling,
and what is left of the folder removed by sluice; a worktree's place
read as gone only once its folder is (record 0115)". Add 0115 to its
ADRs. Then `node site/scripts/rules.mjs`.

**2.1 The reading's test.** `a_checkout_git_no_longer_lists_is_gone` in
`crates/sluice/tests/profiles/places.rs`, under `ports_lock()`, with the
scratch helpers its neighbours use:
1. Make a symlink beside the repository to its folder, and open a
   second `Places` on `<link>/.git` with `Places::new`, as `places()`
   does. A place materialized as `NAME` through it has its record's
   path spelled through the link, and `gone(NAME)` is false. A string
   comparison with git's entry would not match.
2. Drop git's entry by hand. Read the folder's `.git` file, take the
   `gitdir:` it names, and remove that directory, as git's partial
   removal leaves it. `git worktree list` no longer names the folder,
   and the folder still exists. `gone(NAME)` is true.
3. A preview made with `preview(…)`, as
   `a_failed_preview_release_is_held_once_its_retries_are_spent` makes
   one, with its entry dropped the same way: `previews()` lists it
   `gone`.
4. A worktree's place, adopted with `adopt` for the worktree `scratch`
   adds: with its entry dropped the same way, `session_places()` reads
   it `gone == false` while its folder exists.
5. Free each place before the test ends, so it leaves no ports held.

**2.2 The free's test.** `a_checkout_git_no_longer_lists_is_freed` in
the same file:
1. Materialize `NAME` with one port, and `ended_run` a run whose stored
   release appends a line to a mark file. Make `<place>/locked/file`,
   then `chmod 0555 <place>/locked`. Drop git's entry as in 2.1.
   `release_gone(NAME)` returns `ran == true`, `failed == None`, and the
   mark holds one line. `left_behind(&repo, &place)` holds: no folder,
   no scratch, and git lists nothing. `made()` is empty, `gone(NAME)` is
   false, and the next place materialized takes the same port.
2. The incident's shape. A second place, its run's release already run
   by `Runs::remove` before its entry is dropped: `release_gone` returns
   `ran == false`, the mark is unchanged, and the place is freed as
   above.
3. The operator's acceptance. A third place with a read-only directory
   and its entry dropped: `free` returns `Ok`, nothing runs, and the
   place is freed as above.
4. Tidy: make `locked` writable again where an assertion fails first,
   so a failed run leaves no folder the scratch cannot remove.

**3.1 The site.** Each page says it in the reader's words (rule
core.26):
- `reference/commands.md`, "Checkouts held because their release
  failed". The last paragraph says that a checkout whose directory
  someone deleted, or whose directory git no longer lists as a worktree,
  as git leaves one whose removal it stopped part way, held or not, has
  the release its run stored run once from its scratch directory by the
  next round. sluice removes what is left of its directory, it is freed
  whatever the release did, and it is logged `gone`. The
  `--accept-failure` paragraph says it removes the checkout sluice made
  whether or not git still lists it.
- `reference/state.md`, the `gone` paragraph: "a checkout whose
  directory no longer exists" becomes "a checkout whose directory no
  longer exists, or that git no longer lists as a worktree".
