# Reading a policy back

Say what `.config/sluice.toml` does in plain words, the same way every time:
after writing or changing it, before it is committed, and whenever the user
asks what their policy does. Read back only the checks: what each branch
pattern matches and what must pass there. `parallel`, `batch`, `trust` and
the rest were explained when they were chosen.

What sluice does with each key is the configuration reference's:
[`[tests]`](https://agentplot.github.io/sluice/reference/configuration/#tests-how-tests-run)
and [`required`](https://agentplot.github.io/sluice/reference/configuration/#required).
This page says only how to put it.

## The order

1. One sentence on `[tests]`.
2. One block per `[branches."<pattern>"]` table, in the order the file lists
   them. A block whose branch runs no check leads with the warning (below).
3. The moon notice, when any branch's checks are moon tasks.

```text
[tests] says how the checks run: as moon tasks, each run only when a
submission affects it. It chooses none of them; each branch below does.

dev/*  every branch whose name starts dev/, with no further / (dev/login)
  must pass: the moon tasks tagged check-fast (web:lint, web:test, api:test)

main
  must pass: the moon tasks tagged check-fast and e2e (web:lint, web:test,
  api:test, web:e2e)

These checks are moon tasks. Every host that runs them needs moon 2.6 or
later: this host has moon 2.7.1, and the repository's workspace is
.moon/workspace.yml.
```

## `[tests]`, in one sentence

Always say it, since nothing in `[tests]` chooses what runs on a branch:

- `runner = "moon"`: "`[tests]` says how the checks run: as moon tasks,
  each run only when a submission affects it. It chooses none of them; each
  branch below does."
- `runner = "command"`: "`[tests]` says how the checks run: `<command>`, the
  same on every branch below. No branch can choose different checks; that
  needs moon."
- No `runner`: "`[tests]` names no runner, so every run fails until it
  does."

## A branch pattern, in words

- A plain name is itself: `main` matches the branch `main` only.
- `*` stands for any run of characters without a `/`: `dev/*` is "every
  branch whose name starts `dev/`, with no further `/`", giving one
  example (`dev/login`, not `dev/a/b`).
- When a branch matches a plain name and a pattern, the plain name gives
  its gate; between two patterns, the one with more characters besides
  `*`.

## The checks

Under `runner = "moon"`, the line `must pass:` lists the branch's
`required`:

- `#tag` is "the moon tasks tagged `tag`", then in parentheses the tasks
  moon lists with that tag: up to eight, then "and N more". Several tags
  share one phrase: "the moon tasks tagged `check-fast` and `e2e`".
- `project:task` is itself: `cli:build`.

List a tag's tasks in the same shell sluice runs moon in (below):

```sh
moon query tasks --tags check-fast | jq -r '.tasks[][].target'   # each target; without jq, read "target" in the JSON
```

When moon can't run on this host, name the tags alone and say their tasks
could not be listed, beside the moon notice.

Under `runner = "command"`, one sentence covers every branch, in place of
each block's `must pass:` line: "`<command>` runs in full on every
submission to each of these branches." A `required` there gets its own
line on its branch: "`#test` is a moon target, which the command runner
never reads: only `<command>` runs."

## When a branch runs no check

Under `runner = "moon"`, no test runs on a branch when:

- it has no `required`, or `required = []`;
- every tag it requires is one no task carries (`moon query tasks --tags
  <tag>` lists none), and it requires no `project:task`.

Lead that branch's block with these words, the branch's pattern in place of
`dev/*`, before anything else about it:

```text
⚠ No tests will run on dev/*. Its policy names no check, so every
  submission to it lands untested. Name a check for it, or say that is
  what you want.
```

When it is the tags that match nothing, say why in the second sentence:
"No task carries the tag `suite` it requires, so every submission to it
lands untested."

A tag no task carries on a branch that still runs other checks gets a line
of its own in that block: "`#suite` matches no task, so nothing runs for it
on main."

Then stop, and commit nothing until the user answers:

- They name a check: add it to the branch's `required`, and read the
  policy back again.
- They say it is meant: write this comment on the branch's table header,
  and commit.

  ```toml
  [branches."dev/*"] # no checks: lands untested, by choice
  ```

On a later read-back, a table with that comment leads its block with "⚠ No
tests will run on dev/*: every submission to it lands untested, by choice
(the comment on its table)." and asks nothing.

## The moon notice

When any branch's checks are moon tasks, say that every host that runs them
needs moon 2.6 or later, then what this host and repository have:

- this host's moon: the version, or "moon is not installed on this host";
- the workspace: ".moon/workspace.yml", or "the repository has no moon
  workspace, so these checks can't run".

Find moon the way sluice will:

- With `[tests] toolchain` set, inside that shell:
  `devenv shell -- moon --version` for `["devenv", "print-dev-env"]`,
  `nix develop -c moon --version` for `["nix", "print-dev-env"]`.
- Otherwise `moon --version` on `PATH`.
- A `.prototools` that names moon counts as present, since sluice installs
  what it pins before each run: say "the repository's `.prototools` pins
  moon `<version>`, which sluice installs before each run".

Below 2.6, say so: "this host has moon 2.5.1, older than sluice reads, so
sluice refuses to run these checks here until it is upgraded". Never
install or upgrade moon unasked.
