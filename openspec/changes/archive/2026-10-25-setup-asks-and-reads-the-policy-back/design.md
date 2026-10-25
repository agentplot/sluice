# Design

## Context

The setup skill (`plugins/sluice/skills/setting-up-sluice/SKILL.md`)
surveys the repository, then says in one sentence "ask the user which
branches sluice should gate and what must pass on each", and step 4
writes and commits the policy. Nothing reads the result back.

What sluice does with a policy, read from main as it stands:

- `[tests] runner` picks how checks run, and nothing else. With `"moon"`,
  a branch's `required` lists moon targets, `#tag` or `project:task`;
  sluice runs those the combination affects. A branch with no
  `required`, or an empty one, or whose tags no task carries, plans no
  targets, and the run is a pass with nothing run
  (`crates/sluice/src/profiles/moon.rs`, `Plan { argv: None }` in
  `checks.rs`). The combination lands.
- With `"command"`, the one command runs in full on every combination of
  every gated branch, and `required` is never read.
- With no `runner`, every run fails naming the missing key.
- moon must be 2.6 or later (`FLOOR`); with a `.prototools`, sluice runs
  `proto install` in the checkout first, so the pinned moon is there.

A gate with no checks is a gate sluice honours as written: sluice gives
no gate a meaning of its own (27), and the gate is the branch's policy
(56). The warning therefore belongs to the person writing the policy, at
setup, which is this change. The rules touched are core.21 (no new file
type or project: Markdown and Rust only) and core.12 (no clause is
reinterpreted: the skill describes 27 as it is).

## Goals / Non-Goals

**Goals:**

- One plain, ordered read-back the agent gives the same way every time,
  from rules written down in the skill's references, not improvised.
- The trap the newcomer hit, a moon branch with no check, is named before
  the policy is committed, in words a person who has never read the
  configuration reference understands.

**Non-Goals:**

- A sluice command that prints a policy, or a warning from `sluice
  submit` or `sluice status`; see the proposal's out of scope.
- Reading back host settings (`~/.config/sluice/config.toml`); the skill
  already explains `[runs]` where it sets them.

## Decisions

### The read-back's rules live in one reference page

`references/read-back.md` holds what to say for each kind of entry and
the exact shape of the warning; `SKILL.md` points at it from step 4 and
from a new short section for "what does my policy do". Alternatives: put
it all in `SKILL.md`, which is already long and loaded whole on every
trigger; or a script that parses the TOML, which the plugin's shell has
no TOML reader for, and which would duplicate sluice's own policy reading
and drift from it. A reference page loads only when the skill reaches
that step.

### The shape of the read-back

One block per `[branches."<pattern>"]`, in the file's order (ADR 0036's
order, the order `sluice overview` lists them):

```text
dev/*  every branch whose name starts dev/
  must pass: the moon tasks tagged check-fast (web:lint, web:test, api:test)

main
  must pass: the moon tasks tagged check-fast and e2e (… , web:e2e)

These checks are moon tasks. Every host that runs them needs moon 2.6 or
later: this host has moon 2.7.1, and the repository's workspace is
.moon/workspace.yml.
```

and, for a branch that runs nothing, a line that leads with the
consequence:

```text
⚠ No tests will run on dev/*. Its policy names no check, so every
  submission to it lands untested. Name a check for it, or say that is
  what you want.
```

- A pattern is said in words: `*` as "every branch whose name starts
  `dev/`", a plain name as itself. A table with no pattern character
  matches only that branch.
- `#tag` is "the moon tasks tagged `tag`", followed by the tasks
  `moon query tasks` lists with that tag when moon is on the host;
  `project:task` is that task. A tag no task carries gets the warning.
- Under the command runner, one sentence covers every branch: "`<command>`
  runs in full on every submission to each of these branches". A
  `required` there is named as a moon target the command runner never
  reads.
- `[tests]` gets one sentence, always: it says how checks run, and the
  branches below say which.
- Only the checks are read back, not `parallel`, `batch` or `trust`: the
  skill already tells the user why it chose those when it chose them,
  and the read-back stays short enough to read whole.

Alternative: a table. Rejected because a warning must lead with its
consequence, and a row in a table reads as one fact among others.

### How the moon notice finds moon

The skill runs `moon --version` the way sluice will: inside the
toolchain's shell when `[tests] toolchain` is set
(`devenv shell -- moon --version`, `nix develop -c moon --version`),
otherwise on `PATH`; a `.prototools` naming moon counts as present,
since sluice installs it. It reports the version found or "not
installed on this host", and whether `.moon/workspace.yml` exists. It
never installs moon unasked, as step 1 already rules for sluice.

### When the warning asks

The no-check warning is the one point where the skill stops and waits
before committing, as it already does before installing or committing.
A branch the user confirms is meant to land untested gets a comment on
its table in the written file, `# no checks: lands untested, by choice`,
so the next read-back, and the next reader, sees it was meant.

### The gate checks the examples, not the agent's words

The skill's behaviour is an agent's conversation, which no moon task can
run without a model and credentials; the repository's tests reach no
credentials. What the gate can hold is the material the agent works
from: the example policies. `the_plugins_example_policies_read` gains a
loop over each example's `branches` asserting that, under
`runner = "moon"`, every one has a non-empty `required`, with the
example and branch in the message. The conversation itself is checked
by hand, once, on a scratch repository, and the task says so.

## Risks / Trade-offs

- [The agent paraphrases the read-back and drops the warning] → the
  reference gives the warning's words verbatim and says to show it
  before anything else about that branch.
- [A tag's task list is long] → list up to eight tasks, then "and N
  more"; the count is what tells the user something runs.
- [`moon query tasks` needs moon set up, which a new repository may not
  have yet] → without moon the read-back names the tags only and says
  it could not list their tasks, alongside the moon notice.
- [The skill's reading of sluice drifts from sluice] → the reference
  cites the configuration reference's sections rather than restating
  sluice's rules, and the configuration reference gains the same two
  sentences, so the two are read together.
