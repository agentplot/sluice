# repository-gate delta

## ADDED Requirements

### Requirement: Every plugin, and the site, is a project of its own

Each directory under `plugins/`, and `site/`, SHALL be a moon project
listed in `.moon/workspace.yml` with its language declared, and each of
its checks SHALL be a check-fast task of that project, not of the root or
another part's. The site's one check SHALL be its prose check,
`site:prose`: it has no tests, and its build is the Pages workflow's.

#### Scenario: Listing the projects

- **WHEN** `moon query projects` runs in the repository
- **THEN** it lists `plugin-sluice`, `plugin-board`, `plugin-herdr` and
  `site` with the languages `bash`, `typescript`, `bash` and `typescript`

#### Scenario: Listing the root's tasks

- **WHEN** `moon query tasks` runs in the repository
- **THEN** no task of `root` runs a script under a plugin's directory

## MODIFIED Requirements

### Requirement: The board is validated, type-checked and tested

The check-fast tasks `plugin-board:validate`, `plugin-board:typecheck` and
`plugin-board:test` SHALL fail when `plugins/sluice-board` fails, in turn,
`claude plugin validate --strict`, `tsc -p` against the engine types kept
beside it, or a test under `claude plugin test`.

#### Scenario: A type error

- **WHEN** a submission passes a number where `board.ts` declares a string
- **THEN** `moon run plugin-board:typecheck` fails naming the file and line

#### Scenario: A failing test

- **WHEN** a submission changes the band's wording and not its test
- **THEN** `moon run plugin-board:test` fails naming the test

### Requirement: The board's checks need no network and no home

The board's tasks SHALL pass inside the sandbox executor with the network
denied, writing nothing outside its place and its scratch directory, and
reading no Claude Code configuration or credentials of the user's.

#### Scenario: Under the gate

- **WHEN** sluice runs `#check-fast` on a submission under the sandbox
  executor
- **THEN** every `plugin-board` task passes, and the user's `~/.claude`
  and `~/.claude.json` are unchanged

### Requirement: The board's engine types match the pinned Claude Code

`plugin-board:engine-types` SHALL fail when the engine types kept beside
the board were written by a Claude Code release other than the one the
repository pins, naming both releases.

#### Scenario: The pin moves alone

- **WHEN** a submission moves the Claude Code pin and leaves the engine
  types as they were
- **THEN** `moon run plugin-board:engine-types` fails naming the types'
  release and the pinned release

### Requirement: The marketplace and every Claude Code plugin are validated

The check-fast task `root:marketplace` SHALL fail when the marketplace
fails `claude plugin validate --strict` or lists a plugin whose directory
has no manifest, and each Claude Code plugin's `validate` task SHALL fail
when its directory fails `claude plugin validate --strict`.

#### Scenario: A plugin missing from its directory

- **WHEN** a submission lists a plugin in `.claude-plugin/marketplace.json`
  whose directory has no manifest
- **THEN** `moon run root:marketplace` fails naming the plugin

#### Scenario: A broken plugin manifest

- **WHEN** a submission removes the `name` from
  `plugins/sluice/.claude-plugin/plugin.json`
- **THEN** `moon run plugin-sluice:validate` fails
