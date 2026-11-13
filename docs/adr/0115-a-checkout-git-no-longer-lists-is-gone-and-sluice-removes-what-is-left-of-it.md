---
number: 115
title: A checkout git no longer lists is gone, and sluice removes what is left of it
status: accepted
date: 2026-11-11
decision-makers:
- the planner's brief of 2026-11-11
- the construct of a-place-whose-folder-is-gone-is-freed
---

# A checkout git no longer lists is gone, and sluice removes what is left of it

Status: accepted. Clause 123 and the planner's brief of 2026-11-11 decide it: a place whose working copy is already gone has its release run once from what sluice holds for it, is freed whatever that did and is recorded gone, and the operator's acceptance of a failed release frees it the same way. How gone is read (D1), how what is left of the folder goes (D3) and which places are read so (D5) are the construct's own, each cheap to change.

Deciders: the planner's brief of 2026-11-11, "A place whose working copy git no longer lists is freed"; and the construct of the change `a-place-whose-folder-is-gone-is-freed`, which chose the form below.
Sources: signal 2026-11-10-agentplot-ops-8ac92f49/01 (afterthought/crew-state, `agentplot/main`); agentplot/blueprints `design/sluice/requirements.md` (origin/main d4db801) clauses 25, 34 and 123 and C.1's git worktree row; deneb's `sluice/main/log.jsonl` in agentplot/sluice's `sluice.git`, read 2026-11-11; sluice at 5c4e580e: `crates/sluice/src/profiles/places.rs` `gone`, `preview_place`, `remove_worktree`, `registered`, `is_worktree` and `checked_out`, `crates/sluice-core/src/release.rs` `release_made` and `release_held`, `crates/sluice-core/src/step.rs` `perform_releases`, `crates/sluice-core/src/preview.rs` `release_unusable_previews`, `crates/sluice/src/runs.rs` `remove`; records 0016, 0055, 0063, 0120.

## Context and problem statement

On deneb, sluice holds three of agentplot/sluice's checkouts on `main` for good: `sluice/try/main/63`, `71` and `76`. agentplot-ops on 2026-11-10 at 14:56:10Z: "sluice holds its gate's try checkouts (sluice/try/main/63, 71 and 76 on deneb's agentplot/sluice) after their release fails 4 times, because git worktree remove refuses a .places directory that git no longer lists as a worktree; release should free a checkout whose worktree is already gone". The held place says:

> sluice/try/main/63 is held: its release failed 4 times: git worktree remove …/sluice.git/sluice/.places/sluice+2Ftry+2Fmain+2F63: fatal: '…/sluice+2Ftry+2Fmain+2F63' is not a working tree

`main`'s log on deneb has the whole of it for `sluice/try/main/63`: `testing` at 14:22:12Z; `release_failed` at 14:22:47Z, try 1, `git worktree remove …: error: failed to delete '…/sluice+2Ftry+2Fmain+2F63': Directory not empty`; then `release_failed` at 14:23:08Z, 14:23:37Z and 14:24:07Z, tries 2 to 4, each `fatal: '…' is not a working tree`, the last held. `71` and `76` read the same. Each folder is still there, with about 2000 entries and a `.git` file naming an entry under `sluice.git/worktrees/` that no longer exists; `git worktree list` names none of them.

git's `worktree remove` deletes the folder first and stops at the first entry it cannot delete, then deletes its own entry for the worktree whatever that deletion did. So a removal that fails part way always leaves a folder git no longer lists. The run's release command had already run, since `Runs::remove` runs it before the profile removes the folder.

Clause 123 says what sluice does with a place whose working copy is already gone: it runs its release once from what it holds for it, frees what it holds whatever that did, records the place gone and never retries it. The profile never read such a place as gone: `gone` asked only whether the recorded path is a directory, and the folder is. So every round called `release`, which asked git again, and `free`, behind `--accept-failure`, asked git too.

## Considered options

1. A combination's checkout or a preview is gone where git no longer lists its folder, read from git's own list of worktrees with no process and matched by the folder on disk; what is left of the folder is removed by sluice.
2. `git worktree list` run for each reading, its entries compared with the path as strings, as `registered` did.
3. The folder's own `.git` file read, and the entry it names looked for.
4. `git -C <folder> rev-parse` asked whether the folder is a worktree.
5. The place freed and its folder left where it is.
6. Every place read the same way, a worktree's among them.

## Decision outcome

Option 1.

- **D1. Gone is read as git lists worktrees.** The profile has one reading, `listed(path)`: whether git lists the folder as one of the repository's worktrees, its main worktree where it has one or the folder an entry under `<common>/worktrees/` names. gix reads them from the repository's files, as `checked_out` does, and nothing starts a process or reads an object (rule core.31). A folder is matched by what it is on disk, its device and inode, never by how its path is spelled, so `/var/…` and `/private/var/…`, or a path spelled through a link, read as one folder. A folder that no longer exists is matched by its nearest ancestor that does and the rest of its path below it, so an entry and a place whose folders are both deleted still match. An entry whose `gitdir` file cannot be read names nothing; a list of worktrees that cannot be read is an error, never read as unlisted. `gone` reads a candidate's or a preview's place as gone where its recorded path is no directory, as before, or where `listed` is false. `preview_place` reads a preview's `gone` so while its command has not failed. A place with no record, or whose record names no path yet, is not gone. A failed read is an error: the core counts it as a failed try, or as not gone, never as gone.
- **D2. One reading, one owner.** `remove_worktree` asks `listed`. A folder git lists goes by `git worktree remove --force`, whether or not it is still there; a folder git does not list goes by sluice's own removal (D3); nothing where neither is. `registered`, which started `git worktree list` and compared spellings, is removed, and so is `is_worktree` (record 0120), whose canonical-path comparison `listed` answers. `remove_worktree` is only given a place sluice made under `<common>/sluice/.places/` or `.previews/`, so rule core.20 holds.
- **D3. What is left of the folder.** sluice walks it without following links, makes every directory its owner cannot write, read or search its owner's to do so, and removes each entry, then the folder. A check can leave a read-only tree, a copy out of the Nix store or a module cache, which git's removal and a plain `remove_dir_all` fail on alike. A folder that still cannot be removed is a failure to free, its words naming the folder, the entry that could not go and what refused it; the place stays held with it, as any failed free is.
- **D4. The release once, and the operator's acceptance.** Neither needs more. Once `gone` reads true, a round's `release_made` and `sluice release <name>` call `release_gone`, which runs the stored release once, or nothing where `Runs::remove` already ran it as on deneb, then frees the place by D2 and D3, and the core logs `gone`. A place held before this change reads as gone since for `perform_releases` and is freed the same way. `sluice release <name> --accept-failure` calls `free`, which runs nothing and removes the leftover by D2 and D3. A preview git no longer lists is released by `release_unusable_previews`.
- **D5. A worktree's place keeps its reading.** `session_places` reads a worktree's place as gone only once its folder is no directory. sluice neither made nor removes a session's worktree (rules core.16, core.20), and reading a live agent's worktree as gone would run its release and give its ports away while the agent works. sluice's own `git worktree remove` never runs on a worktree, so the incident cannot leave one behind.

Not taken:

- **Option 2.** A process per held place per round, which an idle `sluice run` must not start (rule core.31), and spellings compared as strings, which `/var` and `/private/var` defeat.
- **Option 3.** It answers only for a linked worktree, and it is a second reading beside the one `checked_out` makes of git's list.
- **Option 4.** The folders sit under the repository's own directory; once their `.git` names nothing, git walks up and finds the repository itself.
- **Option 5.** The record, scratch and ports would go and the folder would stay where no record names it and nothing removes it, thousands of files and a build's output each; that needs a sweep and a word for the leftover in the `gone` line, which the log does not carry.
- **Removing the leftover in the same call, right after `git worktree remove` fails.** A writer that made git fail is likely still writing; the next round reads the place as gone and takes 123's path, which logs `gone` and says how its release went.
- **Option 6.** It would take core.16's rule against releasing a session's place whose worktree exists further than the incident asks; it is a change of its own if a report shows a worktree git no longer lists left held.

### Consequences

- Good: deneb's three, and any place like them, are freed by the next round on the sluice that carries this, with nothing done by hand, and `gone` is logged for each.
- Good: the profile reads git's list of worktrees one way, with no process, wherever it asks.
- Neutral: no record or state changes form, and no command's words change.
- Bad: a leftover sluice cannot remove keeps its place held, each round trying the free again with nothing run and logging `release_failed`, until the operator stops whatever writes into it.
- Bad: a worktree's place git no longer lists is still read as held while its folder exists.
