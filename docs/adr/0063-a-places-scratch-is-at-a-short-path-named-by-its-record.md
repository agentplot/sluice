---
number: 63
title: A place's scratch is at a short path that does not grow with the place's name, and its record names it
status: proposed
date: 2026-10-27
decision-makers:
- Chuck Swanberg
---

# A place's scratch is at a short path that does not grow with the place's name, and its record names it

Status: proposed to the user on 2026-10-27, as `briefs.15` has it; accepted only on their yes.

Deciders: madswan-design, on slc-2-conductor's question of 2026-10-27 for the chore `the-lay-out-suite-runs-from-any-worktree` (signal 2026-10-27-slc-1-conductor-648c1025/01); the user's yes is asked on the record's card.
Sources: `agentplot/blueprints:design/sluice/requirements.md` clauses 34, 35, 123, C.4 sandbox and container rows; `crates/sluice/src/profiles/places.rs` `tmp`, `root`, `Made`, `crates/sluice/src/runs.rs` `dir_name`, `shortened`, `KEPT`, `plugins/sluice/tests/lay-out.sh` lines 46 to 51, as origin/main held them on 2026-10-27 (2dfb2e4b); rules core.10, core.17.

## Context and problem statement

A place's scratch directory, the one path besides its checkout and the host's declared caches that a confined check may write (35, C.4), is `<git common dir>/sluice/.tmp/<dir name>`, where the dir name is the place's name with its slashes escaped, shortened only past 246 bytes. A session's place is named `session/<the worktree's full path>`, so its scratch path holds the worktree's whole location twice over, escaped, and grows with every directory above the worktree and with the worktree's own name. The plugin's lay-out suite makes a scratch clone under `SLUICE_TMP` and drives Claude Code in it, whose project key is the clone's path with every other character made a dash and is refused past 200 characters; on deneb two of sluice's own worktrees give 201 and the suite refuses to start, so the gate cannot be run from them before submitting. A Unix socket bound under the same path would fail sooner, at macOS's 104 bytes. Trimming the suite's own names wins a few characters and no worktree.

## Considered options

1. The scratch at a path that does not grow with the place's name or its checkout's location: under the repository's own `sluice/.tmp/`, named by a short fixed-length digest of the place's name, as `SLUICE_PLACE` already names the place (`sluice-<10 hex>`); the place's record names the path, so a release, a stored one among them, removes what the record names, and scratch no record names is an abandoned place's (123).
2. Shorten the suite's own names so today's worktrees pass.
3. The scratch under the checkout, in an ignored directory.
4. The scratch under the host's state directory, `$XDG_STATE_HOME/sluice`, apart from the repository.

## Decision outcome

Option 1.

**The limit is a check's, and every check meets it.** The sandbox lets a check write its checkout, its scratch and the declared caches and nothing else (35, C.4). What a check makes under its scratch, a socket, a clone, a tool's state keyed by its path, meets the limits of the tools the repository's checks run: 104 bytes for a socket on macOS, 200 characters for the key the suite met. A scratch whose path carries the worktree's location breaks those limits for a worktree of a long enough name or a deep enough home, and the check's author can do nothing about it from inside the check. A fixed-length name under a fixed root gives every check the same room whatever the place is called or where it lives.

**The record names the path, so nothing is left behind.** Today the scratch path is derived from the place's name each time it is wanted and the record does not carry it. With the name's scheme changed, a release of a place made before the change would derive a path the old scheme never made. So the record names the scratch path, written when the place is made; a release, run at once or stored (123), removes the path the record names, however the name was made; and a directory under `sluice/.tmp/` no record names is an abandoned place's, removed as 123 removes a gone place's. Nothing reads a name back from a path (`shortened`'s own rule), so the digest need not be reversible.

**Where it lives stays.** The scratch stays under the repository's own `sluice/.tmp/`, so the sandbox profile, the container's mount and the release's removal change nothing but the last path segment, and a reader finds a place's scratch where they found it. Option 4 would move it out of the repository and under a path the sandbox and the container do not yet name; option 3 would put a check's scratch inside the tree the check is judged on, where `git status` must stay clean and a submit would carry it. Option 2 is the suite's workaround for one host's worktrees and leaves every other check, and every socket, where it was.

**A host takes it when its sluice is updated.** The scratch is made by the sluice installed on the host, so the suite's guard stands on a host until its sluice carries this record; the guard says why it refuses, in the terms above.

### Consequences

- The place profile names a place's scratch by a short fixed-length digest of the place's name under `sluice/.tmp/`, writes the path on the place's record when the place is made, and removes the path the record names on release; a record with no scratch path is a place made before, whose scratch is at the path the former scheme gave it.
- A directory under `sluice/.tmp/` that no record names is removed with the abandoned places (123).
- The lay-out suite's guard stands until the host's sluice carries this record; the chore `the-lay-out-suite-runs-from-any-worktree` becomes the unit that builds it.
- Rule core.30 is drawn from this record; clause 35 of the statement says the same.
