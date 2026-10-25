# Proposal

## Why

A newcomer followed the README, asked Claude to set sluice up in their
existing project, and the setup went on in their plain clone without a
word: it said there was no bare repository and used the clone as it
stood. Agents' worktrees then piled up beside the original folder, and
by the time anyone noticed, 28 processes had the folder open and moving
it would have cut the Claude sessions off from their history. The
person helping said sluice should not have allowed it. (Signal
`2026-10-23-madswan-operator-bea2f287/01`, from the onboarding call of
2026-10-20.)

The design now says what the setup does instead (126, S61): sluice is
set up in a repository whose working copies stand apart from its git
directory, one per branch, so every session works and submits from a
working copy of its own (C.1). Asked to set up in a repository's one
and only working copy, the setup offers that layout first, says what
the move costs, moves nothing an agent or an editor has open, carries
the sessions' history to the new path, and goes on in the plain copy
only on the operator's word, which the setup's record says.

## What Changes

- Asked to set up in a repository's only working copy, the setup skill
  stops before writing anything and offers the layout: the git
  directory kept bare in the folder, the working copy moved into
  `<folder>/main`, and one folder per branch beside it.
- The offer says what the move costs, by name: each process running in
  the folder (a dev server, a shell, an editor, another agent) and that
  it must be stopped and started again from `main/`; the Claude Code
  history kept under the folder's path and where it will be carried;
  what stays behind (prompt recall, the folder's trust and permission
  answers, which Claude asks again once for `main/`).
- The move refuses while anything but the setup's own session, and what
  that session started, holds the folder; names what holds it, never the
  session's own; and waits for the operator to let it go. It moves
  nothing and deletes nothing until then. The session is the process the
  caller names, and, named none, the one that started the move (126 as
  amended, record 0057).
- With the move, the setup carries the Claude Code history across: the
  folder's other sessions and its project memory, moved to the places
  Claude Code reads for the new paths; then this session itself, by
  Claude Code's own `/cd` into `main/`, which the operator types.
- The setup then goes on in `main/` as today: survey, interview, policy,
  read-back.
- The operator may keep the plain clone instead. The setup then goes on
  there, and the commit that adds `.config/sluice.toml`, the setup's
  last landing by hand, says in its message that the operator chose to
  keep the plain clone.
- The move and the history carry are one script the plugin ships, run
  by both the setup skill and the worktrees skill's plain-clone
  procedure, and tested by a moon task the gate runs.
- The README's Get started sends a newcomer through that setup, and its
  by-hand path through the layout first; neither sends anyone into a
  plain clone. The site's worktrees guide gains the layout and how to
  reach it, and the Claude Code guide says what the setup offers.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plugin-setup`: the setup offers the layout before going on in a
  repository's only working copy, says its costs, and records the
  operator's choice to keep the plain clone.
- `plugin-worktrees`: the plain-clone conversion refuses while anything
  outside its session holds the folder, and carries Claude Code's
  history to the new paths.

## Out of scope

- A plain clone that already has worktrees of its own, inside it or
  beside it. Clause 126 binds the one and only working copy; whether
  the setup offers the layout there too is a question for the design
  agent, and no later change is planned for it yet. The worktrees
  skill's conversion keeps handling such a clone, on request.
- sluice itself refusing or warning in a plain clone (`sluice submit`,
  `sluice protect`, `sluice status`). The clause binds the setup; a
  warning from sluice is a design question first.
- Carrying the history of sessions started in a subfolder of the clone,
  and editing Claude Code's prompt history or its per-folder trust
  entries in `~/.claude.json`. The offer names them as left behind.
- Other agents' history (codex, opencode). The plugin is Claude Code's.

## Impact

- `plugins/sluice/scripts/lay-out.sh`: new, the plan, the move and the
  history carry.
- `plugins/sluice/tests/lay-out.sh` and `plugins/sluice/moon.yml`: new
  test and its moon task.
- `plugins/sluice/skills/setting-up-sluice/SKILL.md`: a new step before
  the survey, and the policy commit's message.
- `plugins/sluice/skills/worktrees/SKILL.md` and
  `references/convert-from-plain-clone.md`: the procedure runs the
  script.
- `README.md` (Get started), `site/src/content/docs/guides/worktrees.md`,
  `site/src/content/docs/guides/claude-code.md`.
- No change to sluice's binary, its policy format or its records.

## Touches

```
plugins/sluice/scripts/lay-out.sh
plugins/sluice/tests/lay-out.sh
plugins/sluice/moon.yml
plugins/sluice/skills/setting-up-sluice/SKILL.md
plugins/sluice/skills/worktrees/SKILL.md
plugins/sluice/skills/worktrees/references/convert-from-plain-clone.md
README.md
site/src/content/docs/guides/worktrees.md
site/src/content/docs/guides/claude-code.md
```
