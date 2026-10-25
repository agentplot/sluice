# Design

## Context

See proposal.md for why. What exists, on main:

- The way-on lines are plain strings in the CLI crate:
  - `lib.rs` `report()`: a blamed submission ends `fix it in your
    worktree and submit again{same_commits}`; a conflict ends `merge or
    rebase onto {branch} in your worktree and submit again{same_commits}`;
    a check the host could not run, `the host could not run {tasks}: once
    it can, submit again{same_commits}`. `same_commits` is empty, or on a
    pull request sent back on its head `, or mark #{n} a draft and then
    ready for review to test the same commits again`.
  - `words.rs`: `timed_out_way_on` (`if the host was slow, submit the
    same head again…; if {which} never ends, fix it in your worktree and
    submit again`) and `kept_out_refusal` (`let it run in CI in this
    worktree, or drop it from {branch}'s policy … and submit again`).
    `set_aside_hint` and `flaky_way_on` name the operator already.
  - `lib.rs` `refusal()`: uncommitted changes end `commit or remove them
    and submit again`; a detached head is `refused: submit from a
    branch: {worktree} is detached`. The group refusal reuses `refusal()`.
  - `home.rs` `write_home()`: outside a worktree, `not in a worktree
    sluice can submit from`.
- `report()` has the submission, whose `from.worktree` is the branch of
  the worktree it came from and `from.agent` the agent recorded at
  submit. `refusal()` has the worktree as given (`.` when no
  `--worktree`); the driver's `session(worktree).from.worktree` gives its
  branch.
- JSON callers read `told::document`'s `{"error":{"kind","message","next"}}`;
  `message` is the text less its `refused: ` prefix. `next` is built
  separately (`refusal_next`, `home::next`).
- The plugin: `using-sluice/SKILL.md` is written to the agent in its
  worktree ("Submit from your own worktree"); `session-context.sh` emits
  one note at SessionStart; `tests/hooks.sh` checks the note by `grep`.
- Claude Code starts a background agent with the Agent tool; with
  `isolation: "worktree"` it gets a git worktree of its own, which is
  kept when the agent changed something and removed when it did not.
  The agent knows only the prompt it is given, so what it should do at
  the end has to be in that prompt.

## Goals / Non-Goals

**Goals:**

- One sentence form for every way-on line: the agent working in
  `<branch>` does the thing there. A reader who is not that agent sees
  it is not theirs; that agent sees it is its own.
- Scripts that read `next` or JSON fields see no change.

**Non-Goals:**

- Which commands `next` lists, or when; exit codes.
- Pull-request comments and checks on the forge, the git hook's
  refusal, the board's wake turns (proposal, out of scope).
- Any new command or flag.

## Decisions

**Name the agent by the worktree's branch, in the third person.** "the
agent working in unit/a fixes it there and submits again". The branch is
what sluice already shows for a worktree (`sluice` with no command prints
`worktree unit/a (agent) at …`; a conflict prints `conflicts with #12
(agent, unit/b)`), and it is the same on every host. Alternatives: the
recorded agent name (`from.agent`), which is the branch itself unless
`SLUICE_AGENT` was set, so it often repeats the branch and sometimes
names something a reader cannot find; the worktree's path, which differs
per host and is long; second person kept and a prefix added (`for the
agent in unit/a:`), which reads as a label and still leaves "you" to
whoever reads. Third person rests on the statement's actors: the session
that built the work submits it (preamble; §2).

**The lines, as they will read.** `<b>` is the worktree's branch:

| where | becomes |
|---|---|
| blamed | `the agent working in <b> fixes it there and submits again` |
| conflict | `the agent working in <b> merges or rebases onto <branch> there and submits again` |
| could not run | `the host could not run <tasks>: once it can, the agent working in <b> submits again` |
| timed out | `…if the host was slow, the agent working in <b> submits the same head again; if <which> never ends, it fixes it there and submits again` |
| uncommitted | `the agent working in <b> commits or removes them and submits again` |
| kept out of CI | `the agent working in <b> lets it run in CI there, or drops it from <branch>'s policy in the same submission that lets it run, and submits again` |
| detached | `refused: <worktree> is detached: the agent working in it submits from a branch` |
| pull request, same commits | `…, or marks #<n> a draft and then ready for review to test the same commits again` |
| outside a worktree | `not in a worktree sluice can submit from: the agent working in a worktree submits its work from there` |

`words.rs` gains one helper, `agent_in(branch) -> String` ("the agent
working in {branch}"), so the phrase is written once; the functions that
build a way-on line take the branch. In a refusal, the branch is the
driver's `session(worktree).from.worktree`; when the driver cannot read
one (a detached head, a path that is no worktree), the worktree as given.

**The plugin says it where a coordinating Claude will read it.** A
section of the using-sluice skill, "Agents you start in worktrees",
says: the prompt for a background agent working in a worktree tells it
to commit and submit its work from there with `sluice submit <branch>`
(waiting as the skill says, with `--no-wait` and `sluice wait` when its
calls are capped), and to report the outcome; work left committed in a
worktree whose agent has ended is submitted by an agent Claude starts in
that worktree with the same instruction; Claude never asks the user to
run `sluice submit`, and a way-on line sluice prints is for the agent it
names. The session note gains one sentence to the same effect, since a
coordinating session may never load the skill before it starts agents.
The skill names what to tell the agent, not a prompt to paste: the
prompt is Claude's own, written for the work.

**Starting an agent rather than submitting with `--worktree`.** Claude
in the main checkout could submit another worktree's head itself with
`sluice submit <branch> --worktree <path>`. The source (the call of
2026-10-20) has Claude start an agent in each worktree to submit, and
the agent in the worktree is the one that can fix a failure or a
conflict there; so the skill says that, and leaves `--worktree` to the
group submit it already describes.

## Risks / Trade-offs

- [Every quoted copy of the old lines goes stale] → The tasks update each
  page, skill reference and test that quotes them (found by `grep` for
  `in your worktree and submit again`, `commit or remove them`, `not in a
  worktree sluice can submit from`); `site:prose` runs Vale over the
  site pages.
- [A person working in a worktree by hand reads "the agent working in
  unit/a"] → sluice records an agent for every submission (`--as`), so
  the word is already the one it uses for whoever submitted.
- [A longer session note] → One sentence; the note stays one paragraph.
