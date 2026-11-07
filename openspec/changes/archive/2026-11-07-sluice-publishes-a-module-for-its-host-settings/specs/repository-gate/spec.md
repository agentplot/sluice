## ADDED Requirements

### Requirement: The flake is a project of its own

The flake's `nix/` SHALL be a moon project, `nix`, listed in
`.moon/workspace.yml` with the language `nix`. Its check-fast tasks SHALL
be `nix:lint`, statix over `flake.nix` and `nix/`; `nix:typecheck`,
every system's package and each module in a system of its platform; and
`nix:test`, the cases under `nix/tests` and the activation's. Its format
SHALL be `root:format`'s (rule core.21; record 0098).

#### Scenario: Listing the flake's project

- **WHEN** `moon query projects` runs in the repository
- **THEN** it lists `nix` with the language `nix`, and
  `moon query tasks` lists `nix:lint`, `nix:typecheck` and `nix:test`,
  each tagged `check-fast`

### Requirement: The flake's checks need no store and no network

`nix:typecheck` and `nix:test` SHALL evaluate offline and read-only
against a store that is none of the host's, reading the flake's inputs
where the devenv shell fetched them, by the nix `devenv.nix` pins, as
`nix:lint` runs the statix it pins. Each failure SHALL be one line
naming its case, and every case SHALL run (rules core.21, core.24).

#### Scenario: A failing case

- **WHEN** a case under `nix/tests` fails and `moon run nix:test` runs
- **THEN** the task fails with one line, `nix: <case>: <what>`, for each
  failure, and every other case still runs

#### Scenario: No store and no network

- **WHEN** `sluice exec <base> -- moon run nix:test` runs under the
  gate's sandbox, which allows only localhost
- **THEN** `nix eval` reads the flake's inputs from the paths the devenv
  shell names, writes nothing to any store, and the task passes

### Requirement: The module's rendered files are kept byte for byte

The files under `nix/tests/files/`, which the flake's cases compare with
the module's text byte for byte, SHALL be excluded from the formatter by
name, with a comment saying why.

#### Scenario: Formatting the tree

- **WHEN** `moon run root:format` runs on a tree holding
  `nix/tests/files/every.toml`
- **THEN** it does not read that file, and the exclude in `treefmt.nix`
  has a comment above it giving its reason
