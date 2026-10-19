# Design

## Context

See proposal.md, Why. The checks the root ran for the plugins, as of #110:

| Root task | Script | Checks bundled |
| --- | --- | --- |
| `plugin-hooks` | `plugins/sluice/tests/hooks.sh` | the hooks |
| `plugin-board` | `plugins/sluice-board/tests/check.sh` | engine-types release, validate, tsc, tests |
| `plugin-manifests` | `plugins/check-manifests.sh` | the marketplace, and validate of each plugin it lists |
| `plugin-herdr` | `plugins/herdr/tests/shells.sh` | the tests under sh, then under dash |

Every task runs under the sandbox executor with the network denied
beyond localhost, and the toolchain from `devenv print-dev-env`.

## Goals / Non-Goals

**Goals:** moon knows each plugin as a project, its language and its
files; a failing check is named by its own target; core.21 rules out the
shape #110 had.

**Non-Goals:** changing what any check checks; the site's type check.

## Decisions

### D1. One project per plugin, named `plugin-<name>`

`plugin-sluice`, `plugin-board` and `plugin-herdr`, so the board keeps the
id its spec and its tests' fixtures already use, and every plugin's id
says it is one. Each declares `language` (`typescript` for the board,
`bash`, moon's value for shell, for the other two) and `layer:
'application'`, as the site does. Tasks run from the project's root with
project-relative inputs, as the site's do, and name `/devenv.nix` and
`/devenv.lock`, whose pins they run.

### D2. A task per check

| Project | Task | Command |
| --- | --- | --- |
| `plugin-sluice` | `hooks` | `bash tests/hooks.sh` |
| | `validate` | `../claude.sh plugin validate --strict .` |
| `plugin-board` | `engine-types` | `bash tests/engine-types.sh` |
| | `validate` | `../claude.sh plugin validate --strict .` |
| | `typecheck` | `tsc -p .` |
| | `test` | `../claude.sh plugin test .` |
| `plugin-herdr` | `manifest` | `sh tests/manifest.sh` |
| | `test-sh` | `sh tests/run.sh` |
| | `test-dash` | `sh tests/dash.sh` |

`tests/check.sh`'s release check becomes `tests/engine-types.sh`;
`shells.sh` becomes `dash.sh`, which prints a skip where dash is not
installed, as `shells.sh` did. `test-sh` and `test-dash` each make their
own scratch directory and may run at once.

The scratch home every `claude` call needs (no user configuration or
credentials read, nothing of theirs written, no traffic beyond the
command's) was written twice, in `check.sh` and `check-manifests.sh`; it
is now `plugins/claude.sh`, one wrapper the tasks run `claude` through. It
is a tool, as `devenv.nix` is, not a check of any plugin's.

### D3. What stays the root's

- `root:marketplace` (`plugins/check-marketplace.sh`): the marketplace is
  `.claude-plugin/marketplace.json` at the root and names every Claude
  Code plugin, so it is no plugin's. It validates the marketplace and
  finds a manifest for each plugin it lists; validating each plugin's own
  manifest moves to that plugin's `validate`.
- `root:format`: treefmt runs Biome and shellcheck over the whole tree, as
  ADR 0029 decided, so one check finds a file no formatter claims. A
  per-project lint task would run the same linter over the same files a
  second time.

### D4. No moon toolchain for the board

The site is `language: 'typescript'` with no toolchain in
`.moon/toolchains.yml`, and its tasks run devenv's `bun`. The board does
the same with devenv's `tsc` and `claude`: a TypeScript or bun toolchain
of moon's would install through proto, which the sandbox's network denial
refuses. `moon query tasks` shows the plugin tasks on the `system`
toolchain.

### D5. The rule

core.21 says each directory under `plugins/` and `site/` is a moon project
of its own, listed in `.moon/workspace.yml` with its language declared,
with a check-fast task per check; its Details rule out a part's check run
as a task of another project and one task bundling several checks, and
leave a check spanning the parts, the marketplace's, to the root. ADR 0030
records the decision, as ADR 0029 did core.21's first form.
`openspec/config.yaml`'s task rule names the project.

## Risks / Trade-offs

- [Nine tasks where four were] → each is seconds but for the herdr tests,
  which already took minutes; `test-sh` and `test-dash` now run in
  parallel, so the tier is no slower.
- [A plugin added later without a project] → core.21's Details and the
  task rule in `openspec/config.yaml` name it; no check fails on it, as no
  check fails on an undeclared crate either.
