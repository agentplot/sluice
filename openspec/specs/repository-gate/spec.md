# repository-gate Specification

## Purpose
What this repository's own gate checks of the tree a submission hands it,
so that every file the tree tracks is formatted, linted or excluded with a
reason, and every plugin is validated, type-checked and tested on every
submission.

## Requirements

### Requirement: Every tracked file is claimed or excluded

The format check SHALL fail when the tree tracks a file that no formatter
claims and no exclude names, and SHALL name that file.

#### Scenario: A file type nobody claims

- **WHEN** a submission adds `notes/plan.rst` and nothing claims `*.rst`
- **THEN** `moon run root:format` fails naming `notes/plan.rst`

#### Scenario: A file type an exclude names

- **WHEN** a submission adds a file under `docs/examples/`
- **THEN** `moon run root:format` passes without reading it

### Requirement: Every exclude says why

Every exclude in the formatter's configuration SHALL carry a comment
saying why no formatter owns those files.

#### Scenario: Reading the excludes

- **WHEN** a reviewer reads `treefmt.nix`
- **THEN** each exclude, or each group of excludes, has a comment above it
  giving its reason

### Requirement: TypeScript, JavaScript, CSS and JSON are formatted and linted

The format check SHALL fail on a `.ts`, `.tsx`, `.mjs`, `.css` or `.json`
file that Biome would rewrite or whose lint Biome's recommended rules
fail, outside the files excluded by name.

#### Scenario: An unformatted module

- **WHEN** a submission changes `plugins/sluice-board/hooks/board.ts` to
  use single quotes
- **THEN** `moon run root:format` fails naming the file

#### Scenario: A lint error

- **WHEN** a submission compares with `==` in a board module
- **THEN** `moon run root:format` fails naming the rule and the file

### Requirement: Python is formatted and linted

The format check SHALL fail on a `.py` file that ruff would reformat or
whose default rules ruff fails.

#### Scenario: An unformatted diagram script

- **WHEN** a submission changes `docs/diagrams/sluice.py` so ruff would
  reformat it
- **THEN** `moon run root:format` fails naming the file

### Requirement: Workflows are linted

The format check SHALL fail on a workflow under `.github/workflows/` that
actionlint rejects.

#### Scenario: A mistyped key

- **WHEN** a submission writes `runs-on` as `run-on` in `ci.yml`
- **THEN** `moon run root:format` fails naming the workflow and the line

### Requirement: The board is validated, type-checked and tested

The check-fast task `root:plugin-board` SHALL fail when
`plugins/sluice-board` fails `claude plugin validate --strict`, fails
`tsc -p` against the engine types kept beside it, or fails a test under
`claude plugin test`.

#### Scenario: A type error

- **WHEN** a submission passes a number where `board.ts` declares a string
- **THEN** `moon run root:plugin-board` fails naming the file and line

#### Scenario: A failing test

- **WHEN** a submission changes the band's wording and not its test
- **THEN** `moon run root:plugin-board` fails naming the test

### Requirement: The board's checks need no network and no home

`root:plugin-board` SHALL pass inside the sandbox executor with the
network denied, writing nothing outside its place and its scratch
directory, and reading no Claude Code configuration or credentials of the
user's.

#### Scenario: Under the gate

- **WHEN** sluice runs `#check-fast` on a submission under the sandbox
  executor
- **THEN** `root:plugin-board` passes, and the user's `~/.claude` and
  `~/.claude.json` are unchanged

### Requirement: The board's engine types match the pinned Claude Code

`root:plugin-board` SHALL fail when the engine types kept beside the board
were written by a Claude Code release other than the one the repository
pins, naming both releases.

#### Scenario: The pin moves alone

- **WHEN** a submission moves the Claude Code pin and leaves the engine
  types as they were
- **THEN** `moon run root:plugin-board` fails naming the types' release and
  the pinned release

### Requirement: The marketplace and every Claude Code plugin are validated

The check-fast task `root:plugin-manifests` SHALL fail when the
marketplace or any plugin it lists fails `claude plugin validate
--strict`.

#### Scenario: A plugin missing from its directory

- **WHEN** a submission lists a plugin in `.claude-plugin/marketplace.json`
  whose directory has no manifest
- **THEN** `moon run root:plugin-manifests` fails naming the plugin

### Requirement: The board's type configuration is tracked

`plugins/sluice-board` SHALL hold a tracked `tsconfig.json`, so loading
the board in a session leaves the worktree clean.

#### Scenario: After a session loads the board

- **WHEN** a session started with `--plugin-dir plugins/sluice-board`
  loads the board in a worktree
- **THEN** `git status --porcelain` in that worktree prints nothing
