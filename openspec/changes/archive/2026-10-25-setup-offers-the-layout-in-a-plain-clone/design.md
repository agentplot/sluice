# Design

## Context

The setup skill (`plugins/sluice/skills/setting-up-sluice/SKILL.md`)
installs the binary, surveys the repository, asks which branches to
gate, writes and reads back `.config/sluice.toml`, and commits it. Its
survey sends a bare repository with worktrees to the `worktrees` skill,
but a plain clone falls through to the survey and the setup goes on
there. The `worktrees` skill already has a procedure for converting a
plain clone (`references/convert-from-plain-clone.md`): commands to run
by hand, with a check for processes in the folder and a rule never to
delete. Nothing in it is run by the gate, its process check covers
working directories only, and it says nothing of Claude Code's
history. The spec's existing scenario for it
(`plugin-worktrees`, "A normal clone with changes and a worktree
elsewhere") has no test behind it.

What Claude Code keeps per path, from its documentation
(code.claude.com: *Manage sessions*, *Explore the .claude directory*,
*How Claude remembers your project*, *Commands*, *Skills*):

- Transcripts are `projects/<key>/<session-id>.jsonl`, with a
  `projects/<key>/<session-id>/` folder beside each for subagents and
  spilled tool results, under `CLAUDE_CONFIG_DIR` or `~/.claude`. The
  key is the working directory's path with every character other than
  a letter or a digit replaced by `-`; a key over 200 characters is cut
  to 200 and a hash of the full path appended.
- Auto memory is `projects/<key>/memory/`, its key "derived from the
  git repository", shared by every worktree. On this host every
  repository in the bare layout keeps its memory under the key of its
  git directory (`…-sluice-sluice-git` for `sluice/sluice.git`); a
  plain clone's is under the key of its folder.
- `/cd <path>` moves the running session to a new directory, keeping
  the conversation, and relocates it to that directory's storage.
- `${CLAUDE_SESSION_ID}` in a skill's text is replaced by the running
  session's id; `${CLAUDE_PLUGIN_ROOT}` by the plugin's directory.
- Prompt recall (`history.jsonl`) and the folder's entry in
  `~/.claude.json` (trust, allowed tools, MCP servers) are keyed by the
  path too.

## Goals / Non-Goals

**Goals:**

- The setup cannot go on in a repository's only working copy without
  the operator having chosen it, and the choice is on record.
- The move is safe to run from the very session that offers it.
- The conversion's safety is a test the gate runs, not prose.

**Non-Goals:**

- Detecting an editor that watches the folder without a working
  directory or an open file in it. The offer asks the operator to close
  the folder in every editor; the script catches what `lsof` shows.
- Carrying prompt recall or the trust entry: both are Claude Code's
  own files of other projects' data, and editing them is not the
  plugin's to do.

## Decisions

### When the setup offers the layout

The setup offers it when, from the folder it was asked in,
`git rev-parse --is-bare-repository` is `false`, the absolute
`--git-common-dir` is `<--show-toplevel>/.git`, and
`git worktree list --porcelain` names one worktree. That is the
clause's "one and only working copy" (126) and S61's "git directory
inside its one working copy". A clone with worktrees of its own does
not match, and the setup goes on as it does today (see proposal, Out
of scope). The check is the first thing after the binary, before the
survey, so nothing is surveyed or written in a folder about to move.

*Alternative:* offering it whenever the git directory sits inside a
working copy, worktrees or not. Broader than the clause; left to the
design agent.

### One script does the plan, the move and the carry

`plugins/sluice/scripts/lay-out.sh`, bash like the plugin's other
scripts, with two forms:

- `lay-out.sh plan <folder> [--session-pid <pid>]` prints the plan and
  changes nothing: the
  git directory's new name (`<name>.git`, where `<name>` is the
  folder's), `main/` on the branch checked out, each process holding
  the folder (id, command, and whether by working directory or an open
  file), each other worktree and whether it will be repaired or stops
  the move, and the history to carry with its old and new keys.
- `lay-out.sh go <folder> [--session <id>] [--session-pid <pid>]`
  checks again, refuses on
  anything the plan would have stopped on, lays the folder out, then
  carries the history, and prints what it did. Exit 0 done, 1 refused
  with nothing moved, 2 a step failed after the move began (every file
  still on disk, as the procedure's "If anything looks wrong" says).

The skill runs `${CLAUDE_PLUGIN_ROOT}/scripts/lay-out.sh`; the
`worktrees` skill's plain-clone procedure runs the same script in place
of its hand commands, so there is one conversion. Its steps are those
the procedure already gives (git directory renamed and made bare, the
`.git` file, the files moved into `main/`, `main/` registered by
`worktree add --no-checkout` and `repair`, index reset, the remote's
refspec and `worktree.useRelativePaths`), with its preconditions as
refusals: `<name>.git` or `main` already there, a registered worktree
inside the folder.

*Alternative:* keep the conversion as prose for Claude to type out.
That is what failed: nothing tests it, and every run is a new
transcription. A script is the only form the gate can hold to the
spec (core.21).

### What "holds the folder" means

A holder is a process outside the session that runs the move, that
session's ancestors and its descendants (126 as amended; record 0057,
proposed). What the refusal protects is a process the move would
surprise: an editor, another session, a build in flight. The session
that runs the move is told the new path and its history is carried
there; what it started (a tool server, a watcher, a sleep inhibitor,
the shell the command runs in) is its own, started from it and ending
with it.

The session is the process the caller names, and, named none, the
conversion's parent:

- Both forms take `--session-pid <pid>`. The script resolves nothing
  about any harness from it (core.1): it is a process id, nothing more.
- Without it, the session is the process that started the script. A
  person who types the command in a terminal starts it from their
  interactive shell, which is then the session.
- Claude Code runs a command line in a shell of its own, whose parent
  is the session's process, and may wrap the command inside that shell,
  so the default would stop one level short. The plugin's skills
  therefore name the session in the command line itself:
  `--session-pid "$PPID"`, which that shell expands to its own parent.

A process is in the session when walking its parent ids reaches the
session; the session's ancestors are found by walking up from it. The
script's own process is left out too. A process whose working
directory is at or under the folder, or that has a file under it open,
holds the folder when it is none of those. The script lists every
process's working directory, open files and parent id once (`lsof`,
whose `R` field gives the parent: `ps` is setuid on macOS and a sandbox
may refuse it), and keeps the paths that are the folder or start with
`<folder>/`, both as given and with symbolic links resolved. If `lsof`
is missing or fails, the script refuses: it cannot tell the folder is
free.

The refusal names every holder it found, by id, command and how it
holds the folder, and never one of the session's own; the plan lists
the same holders, so what the operator is asked to stop is what `go`
would refuse on. Every other session, editor, shell or server must let
the folder go first (126: "moves nothing an agent or an editor has
open").

*Alternative:* the session as the nearest ancestor whose command is
Claude Code's. That names a harness in the conversion and fails a
person in a terminal (record 0057, option 2).

*Alternative:* only the conversion and its ancestors left out. A
session's harness keeps helpers of its own in the folder, siblings of
the command's shell, so the move would refuse on the very session that
offers it (record 0057, option 3).

### Carrying the history

With the folder laid out, `go` carries, under `CLAUDE_CONFIG_DIR` or
`~/.claude`:

- every `<id>.jsonl` and its `<id>/` folder from the folder's key to
  `main/`'s key, except the session `--session` names;
- the files of `memory/` from the folder's key to the git directory's
  key's `memory/`.

Each is a `mv` that never replaces: a name already at the new key stays
where it is, and both are reported. Nothing is deleted. The setup skill
passes `--session ${CLAUDE_SESSION_ID}`, so its own transcript, still
being written, is left for `/cd`, which moves it the way Claude Code
moves a session. Then the skill asks the operator to type
`/cd <folder>/main` and goes on there.

A key over 200 characters ends in a hash of a path the script cannot
reproduce, so the script carries nothing for it and says to resume
those sessions by id (`claude --resume <id>`), which Claude Code finds
from any directory. Sessions started in a subfolder of the clone keep
their own keys and are reported as left; resuming by id finds them.

*Alternative:* copying rather than moving. Claude Code's resume by id
refuses an id that two projects hold, so a copy would make the old
sessions unreachable from anywhere but `main/`. Moving keeps one of
each.

*Alternative:* carrying after `/cd`, once the session no longer writes
under the folder's key. That would have the script trust what the
session's process reports of its working directory after `/cd`, which
the documentation does not say; naming the session by id does not
depend on it.

### The setup's record of a kept plain clone

The setup's one record is the commit that adds `.config/sluice.toml`,
the last change landed by hand (step 4 of the skill). When the
operator keeps the plain clone, its message carries the line:

```
The operator chose to keep this plain clone over a git directory with
one working copy per branch (126).
```

The commit is what the setup leaves in the repository's history; it
outlives every session, and anyone reading the policy's log finds it.
A comment in the policy would say the same, but anyone may edit it
away in a later landing, and the read-back would then have to explain
it. The statement names no form for the setup's record; until it does,
this is the plugin's.

### Where the offer sits in the skill

A new step 2, "A working copy per branch", between the binary and the
survey. It runs the check, and on a plain clone runs `plan`, shows the
plan in plain words (the costs first: the processes by name, the
history's paths, what stays behind: prompt recall, the folder's trust
and permission answers, and any editor or app to reopen at `main/`),
and asks one question: lay it out, or keep the plain clone. On "lay it
out" it runs `go`; on a refusal it names what holds the folder and
waits for the operator's word to try again. Once `go` succeeds it asks
for `/cd`, then carries on with the survey in `main/`. The worktrunk
template check the `worktrees` skill already asks for is part of the
plan when worktrunk is used.

### Documentation

- README Get started: the Claude Code path says that in a plain clone
  Claude first offers the layout and goes on in the clone only if told
  to; the by-hand path starts from the layout (the worktrees guide's
  new section), then installs and follows the walkthrough.
- `site/…/guides/worktrees.md`: a first section, the layout, with the
  fresh-clone commands from `references/layout.md` and how the setup
  offers it for an existing clone, what it costs and what it carries.
- `site/…/guides/claude-code.md`: "Set up a repository" says what the
  setup offers in a plain clone before anything else.

## Risks / Trade-offs

- [An editor that watches the folder through the system's file events
  shows in no `lsof` listing] → the plan asks the operator to close the
  folder in every editor before saying go, and the moved files are all
  on disk if an editor misbehaves.
- [Claude Code changes how it names a project's key or where memory
  lives] → the script reports each old and new path it uses, and
  carries nothing it cannot compute; the history stays where it was and
  resume by id still finds it.
- [`lsof` over every process is slow on a busy Mac] → it runs once per
  form, a few seconds; the setup is done once per repository.
- [The setup's own session, or a helper it started, holds a file in
  the folder open at the moment of the move] → it is the session's, so
  no holder; the file moves with the rest and the session's next read
  finds it under `main/` after `/cd`.
- [Claude Code puts a wrapper between the session's process and the
  shell that runs the command, so `$PPID` names the wrapper] → the
  session's helpers then show as holders and the move refuses, naming
  them; it never moves something held. The operator can name the
  session's process by hand.
- [A person names as the session a process whose descendants include
  another agent, a terminal multiplexer's server] → those agents are
  left out and may be surprised. The default is the narrowest session
  that fits a person in a terminal; the skills say to name only the
  process that runs this session.

## Migration Plan

None: a plugin change. Repositories already set up in a plain clone are
left as they are; the `worktrees` skill converts one on request with
the same script.
