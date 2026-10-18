# Proposal

## Why

`plugins/sluice-board` landed (#79) with nothing in the gate formatting,
linting, type-checking or testing it. Its checks were commands run once by
hand (tasks 3.1 and 3.2 of the archived `claude-code-plugins`), and the
`tsconfig.json` the engine writes beside it is left untracked, which stops
`sluice submit`. The cause is wider than the board: nothing in this
repository says every file the tree tracks is checked by the gate, and the
task rule in `openspec/config.yaml`, "every task names its verification",
is met as well by a command run once as by a task the gate runs. So the
JSON files, the site's code, the diagram scripts and the workflows are
unchecked too, and `treefmt` lets a file no formatter claims pass in
silence.

## What Changes

- A rule, core.21, with its ADR: every file the tree tracks is formatted
  by the gate or excluded by name with its reason, every language with a
  linter is linted, and every directory under `plugins/` and `site/` has a
  check-fast moon task. A task in a change verifies through the gate, or
  says why it cannot.
- `openspec/config.yaml`: a `rules.tasks` entry holding every future
  change to core.21, and a context line that no longer calls the tree
  Rust alone.
- `treefmt` refuses a tracked file that no formatter claims and no exclude
  names (`on-unmatched = "fatal"`), and each exclude says why.
- Biome, through treefmt, formats and lints the TypeScript, JavaScript,
  CSS and JSON. ruff formats and lints the diagram scripts. actionlint
  lints the workflows. taplo and yamlfmt take `.prototools` and
  `.versionrc`, which their extensions hid.
- `plugins/sluice-board` gets a committed `tsconfig.json`, the engine's
  types for the pinned Claude Code release kept beside it, and a
  check-fast moon task, `plugin-board`, running `claude plugin validate`,
  `tsc -p` and `claude plugin test` inside the sandbox. A moon task,
  `plugin-manifests`, validates the marketplace and `plugins/sluice`.
- Claude Code and TypeScript are pinned in `devenv.nix`, so the gate runs
  the same `claude` and `tsc` on every host.
- Once `herdr-plugin` is on main: its extensionless stand-ins go to
  shellcheck and shfmt, and its awk, jq and fixture documents are excluded
  by name with their reasons.

Out of scope: the site's type check (`astro check`) and its `.astro` and
`.mdx` files. The type check needs the site's dependencies, which only
`bun install` with the network fetches, and no place has them; it waits on
a change that gives a place the site's dependencies from a checkout cache
keyed on `site/bun.lock`. The `.astro` files wait on Biome's support for
them leaving its experimental stage. `*.md` stays excluded, as before. The
site's prose is Vale's, and no gate reads the rest.

## Capabilities

### New Capabilities
- `repository-gate`: what the repository's own gate checks of the tree it
  is handed: every tracked file formatted or excluded with a reason, every
  plugin validated, typed and tested.

### Modified Capabilities
<!-- none -->

## Impact

- New: `docs/adr/` record for core.21; `plugins/sluice-board/tsconfig.json`,
  `plugins/sluice-board/engine-types/`, `plugins/sluice-board/tests/check.sh`;
  `plugins/check-manifests.sh`.
- Changed: `docs/architecture/core.md`, `openspec/config.yaml`,
  `treefmt.nix`, `devenv.nix`, `devenv.lock` if an input moves, `moon.yml`,
  `plugins/sluice-board/README.md`, `site/src/content/docs/guides/claude-code.md`
  where it says how the board is tested; and every file Biome, ruff and
  actionlint reformat or flag on their first run.
- After `herdr-plugin` lands: `treefmt.nix`, and `moon.yml`'s
  `plugin-herdr` inputs if the stand-ins move.
- No crate changes; no behaviour of sluice changes.
