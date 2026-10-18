# Design

## Context

See proposal.md, Why. The gate today is `moon.yml`'s check-fast tasks:
`root:format` (`treefmt --ci` over what `treefmt.nix` configures: rustfmt,
deadnix and nixfmt, shellcheck and shfmt, taplo, yamlfmt), `root:lint`
(clippy), `root:plugin-hooks`, each crate's cargo tests and `site:prose`
(Vale). `treefmt.nix` excludes `*.json`, and treefmt's default
`on-unmatched = "info"` passes a file no formatter claims without a word.
On main, 40 tracked files are claimed by no formatter and named by no
exclude: the board's six TypeScript modules, the site's code, the diagram
scripts, the brand and diagram images, the test keys, `.prototools`,
`.versionrc`, `.vale.ini`, `.worktreeinclude`, two `.gitkeep`s and Vale's
vocabulary.

Every task's command runs under the sandbox executor
(`crates/sluice/src/profiles/checks.rs`): writes only to the place and its
scratch directory, the network as the policy says (`localhost` for
sluice's own gate), the environment cut to `PATH`, `HOME`, `TMPDIR` and a
few more, and the toolchain from `devenv print-dev-env`.

This change's scenarios are checked by the gate's own moon tasks, not by
the conformance suite or an e2e test as `openspec/config.yaml` asks of a
spec: the capability is the repository's gate, not a behaviour of sluice.

## Goals / Non-Goals

**Goals:** a file type the gate does not check can no longer land
unnoticed; the board is checked as the shell hooks are; the rule and the
task rule hold every later change to the same.

**Non-Goals:** the site's type check and its `.astro` and `.mdx` files
(proposal.md, Out of scope); formatting `*.md`; changing any behaviour of
sluice.

## Decisions

### D1. treefmt refuses an unclaimed file

`treefmt.nix` sets `settings.on-unmatched = "fatal"`. `treefmt --ci`, and
so `root:format`, then fails naming any tracked file that no formatter's
includes match and no exclude names. Each exclude gets a comment giving
its reason (core.21). `root:format`'s inputs become every tracked file
(`/**/*`), since a file of any type can now fail it.

The alternative, a script listing extensions against a table, would say
the same thing in a second place; treefmt already knows which files it
formats.

### D2. Every unclaimed file type, decided

| Files | Decision | Reason |
| --- | --- | --- |
| `*.ts`, `*.tsx`, `*.d.ts`, `*.mjs`, `*.css`, `*.json` | Biome (D3) | |
| `*.py` | ruff format and ruff check | the diagram scripts; `sluice.py` is an input to the site's build |
| `.github/workflows/*.yml` | actionlint, beside yamlfmt | yamlfmt formats a workflow but reads none of its keys |
| `.prototools` | taplo, by name | TOML under a name with no extension |
| `.versionrc` | yamlfmt, by name | convco's YAML under a name with no extension |
| `*.md`, `*.mdx` | excluded | prose; the site's pages are Vale's (`site:prose`) and no formatter here rewraps prose |
| `*.lock` | excluded | written by cargo, devenv and bun in their own layouts |
| `*.jsonl`, `docs/examples/**` | excluded | records as sluice writes them, kept byte for byte |
| `*.svg`, `*.png` | excluded | drawn images and rendered diagrams; no formatter keeps them stable |
| `*.pem` | excluded | test keys, kept byte for byte |
| `docs/brand/social-preview.html` | excluded | the page the social image is rendered from, kept as drawn; Biome's HTML support is experimental |
| `.vale.ini`, `styles/config/vocabularies/**` | excluded | Vale's own configuration and word lists, which no formatter reads |
| `.worktreeinclude`, `*.gitkeep` | excluded | a list in gitignore syntax; empty markers |
| `.claude/settings.json` | excluded | Claude Code rewrites it in its own layout when a setting changes |
| `plugins/sluice-board/engine-types/**` | excluded | the engine's own declarations, copied whole (D6) |
| `site/**/*.astro` | excluded, from every formatter | Biome formats only an `.astro` file's frontmatter, and only experimentally |
| `target/**`, `.devenv/**`, `.moon/cache/**` | excluded | build outputs and caches |

The site's `.ts`, `.mjs` and `.css` are formatted and linted now: Biome
reads them with no install. Its type check is out of scope (proposal.md).

Once `herdr-plugin` is on main:

| Files | Decision | Reason |
| --- | --- | --- |
| `plugins/herdr/tests/fake/*`, `plugins/herdr/tests/fake-*/*` | shellcheck and shfmt, by path | `#!/bin/sh` stand-ins with no extension |
| `plugins/herdr/lib/*.awk`, `plugins/herdr/lib/*.jq` | excluded | no formatter or linter for either in the toolchain; `root:plugin-herdr` runs every program under sh and dash, and a parse error fails it |
| `plugins/herdr/tests/docs/**` | excluded | fixture documents and frames, compared byte for byte |

### D3. Biome through treefmt, its settings in treefmt.nix

`programs.biome.enable` in `treefmt.nix`, with `formatCommand = "check"`,
so one pass formats, lints with the recommended rules and sorts imports,
and a lint error fails `treefmt --ci`. Biome comes from the nixpkgs
`devenv.lock` pins. The settings are the operator's usual ones, kept
minimal: spaces, width 2, lines of 100; double quotes, trailing commas,
semicolons; JSON with no trailing commas; the recommended lint rules and
nothing more.

treefmt-nix hands Biome its settings by `--config-path`, so a `biome.json`
at the root would be read by an editor and not by the gate. The settings
are written once, in `treefmt.nix`, beside every other formatter's. An
editor that wants them runs Biome through treefmt. A committed
`biome.json` with treefmt-nix's own formatter entry replaced by a hand-made
one would save editors that step, at the cost of a second place to keep
in step. Not worth it for six modules.

### D4. Claude Code and TypeScript pinned in devenv.nix

`claude plugin validate` and `claude plugin test` run the engine they
check against. The gate must run the same release on every host, or a
host's own update fails, or passes, a key that hashed the same: a
host-caused result kept under the key. So `devenv.nix` fetches Claude
Code's native build by version, for each platform, checksum taken from
that release's `manifest.json`, as it fetches proto and Vale, and puts it
on `packages`. `devenv print-dev-env` puts it ahead of the user's own
`claude` on the run's `PATH`. The pin's version is written once.

That also keeps the gate off a user's `claude` wrapper. On the operator's
hosts `claude` is a script that adds `--plugin-dir` options. `claude plugin
test` refuses to run that way ("not run from this spelling"), seen on
2.1.296.

nixpkgs' `claude-code` was the alternative. It trails releases, and the
mod API is early access and moves between them.

`tsc` is nixpkgs' `typescript` (5.9.3 at this lock). The engine types ask
for 5.4 or newer.

### D5. The board's checks, and where they can run

`plugins/sluice-board/tests/check.sh`, run by the check-fast task
`root:plugin-board` (inputs `/plugins/sluice-board/**/*`, `/devenv.nix`,
`/devenv.lock`), the pattern `root:plugin-hooks` set:

1. the engine types' first line, `// Written by Claude Code <v>.`, names
   the `claude --version` on `PATH`, else it fails naming both;
2. `claude plugin validate --strict plugins/sluice-board`;
3. `tsc -p plugins/sluice-board`;
4. `claude plugin test plugins/sluice-board`.

It runs with `HOME` a directory under `$SLUICE_TMP`, or under a `mktemp
-d` it removes when run by hand, and with
`CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1`. With a scratch `HOME`,
Claude Code writes its `.claude.json` and queues telemetry there. It never
reads the user's configuration or credentials, and the user's `~/.claude`
is never written.

Tried on 2.1.296 under a Seatbelt profile built as `seatbelt()` builds
one: writes to the place and the scratch directory only, `(deny
network*)`, the environment cut to `PATH=/usr/bin:/bin`, `HOME`, `USER`,
`TMPDIR` and `LANG`. All four steps passed, the tests 14 of 14. Nothing
was written outside the scratch directory but Node's compile cache under
`TMPDIR`. None of the four calls a setuid tool. The network is not needed
at all, so the task holds under `network = "denied"` as well as sluice's
`localhost`.

`plugins/check-manifests.sh`, run by `root:plugin-manifests` (inputs
`/.claude-plugin/**/*`, `/plugins/**/*`, `/devenv.nix`, `/devenv.lock`),
validates the marketplace with `claude plugin validate --strict .` and
every plugin the marketplace lists, under the same `HOME` and environment.

### D6. The engine types kept beside the board

The engine writes a mod's types only when a session loads it:
`.claude-plugin/types/`, ignored by its own `.gitignore`, and a
`tsconfig.json` extending it when the mod has none. No command writes
them. A place in the gate has never been loaded, so it has no types, and
`tsc` cannot run there from the engine's output.

So `plugins/sluice-board/engine-types/claude-code/index.d.ts` and
`engine-types/claude-code-tools/index.d.ts` are copied from a load by the
pinned release and committed. The MCP types are left out: the board calls
no MCP tool, and they hold whatever servers the loading session had. The
committed `tsconfig.json` carries the options the engine's own carries,
with `engine-types` its only type root, so an editor and the gate type the
board against the same declarations. The engine leaves a mod's own
`tsconfig.json` alone, so a session's load leaves the worktree clean.

Moving the pin is one change: the version in `devenv.nix`, the types
copied again from a session of that release (`claude --plugin-dir
plugins/sluice-board`, then copy the two folders from
`.claude-plugin/types/`), and any fix the new types ask for. Step 1 of D5
fails a change that moves one without the other. The board's README says
how.

### D7. The rule, the ADR and the task rule

core.md gets a section, "The gate", with:

- **[core.21]** Every file the tree tracks is owned by a formatter the
  gate runs, or is excluded by name with its reason; every language with a
  linter is linted; every directory under `plugins/` and `site/` has a
  check-fast moon task that validates, type-checks and tests it.
- Details: "Rules out: a file type treefmt neither formats nor excludes;
  an exclude with no comment saying why; a plugin directory whose
  manifest, types or tests no moon task runs; a task in a change whose
  only verification is a command run by hand, unless the task says why
  the gate cannot run it. Not ruled out: the site's build, which is the
  Pages workflow's; the site's type check until a place holds the site's
  dependencies; `*.md` outside the site, which no gate reads. Source: ADR
  <n>."

It guards the repository's own checks, not a clause of the statement, so
like core.13 its source is an ADR alone, and core.12 does not reach it.
The ADR records the cause in this proposal's Why, and the options: a rule
alone; a rule with `on-unmatched = "fatal"`; a script comparing extensions
to a table. It takes the next free number when it lands (0028 is held by
the unlanded `run-progress`). The `openspec/config.yaml` context's list
"core.1 to core.17" becomes "core.1 to core.21".

`openspec/config.yaml` `rules.tasks` gains: "A change that adds a
language, a file type or a directory under plugins/ or site/ has a task
that puts it under treefmt, its linter and a check-fast moon task
(core.21); a verification is a moon task the gate runs, or the task says
why it can only be run by hand." The context's "written in Rust" becomes
"written in Rust, with plugins in shell and TypeScript and a site in
Astro".

## Risks / Trade-offs

- [The first Biome, ruff and actionlint runs rewrite or flag files across
  the tree] → one commit per tool, the rewrite alone, before the commit
  that turns `on-unmatched` to fatal. Each is reviewed as a diff with no
  change of behaviour, and `root:plugin-board` and `site`'s build run
  after it.
- [treefmt-nix checks Biome's settings against a schema for an older
  Biome than nixpkgs ships] → the settings use only keys that schema
  has. If validation still refuses, `validate.enable = false`, said in a
  comment.
- [The pinned Claude Code is not the release the operator's sessions run]
  → the gate checks the board against the pin. The operator's newer
  release is ahead of it, and moving the pin is the change that catches up
  (D6).
- [Each host's store holds a second Claude Code build, about 240 MB] → it
  is fetched once per pin and shared by every worktree.
- [`root:format` with every tracked file as input runs on any change] → it
  takes seconds, and a replay covers an unchanged tree.
- [The CI `fast` job on Linux runs `root:plugin-board` with no Seatbelt] →
  the pin covers `linux-x64`, and the task needs nothing the job lacks.
- [`herdr-plugin` lands its own `treefmt.nix` or `moon.yml` edits first]
  → the herdr group is applied on top of main as it then stands, and
  treefmt's fatal unmatched run says what is left.

## Migration Plan

Built only after `herdr-plugin` has landed on main, in the order of
tasks.md, and landed through `sluice submit main`. Rolling back is
reverting the change. Nothing outside the repository reads what it adds.
