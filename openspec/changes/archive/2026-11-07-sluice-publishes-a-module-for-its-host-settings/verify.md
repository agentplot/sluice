# Verify

The work on `change/sluice-publishes-a-module-for-its-host-settings` at
a320f81a, cut from `bolt/sluices-own-module-writes-a-hosts-settings` at
68d68a9a, checked on 2026-11-07 against this change's proposal, design,
its two delta specs (`host-settings-module` and `repository-gate`) and
its tasks.

**The work holds, and the change is archived.** Every scenario of the
delta specs has a case or a test that passes in the gate. All 9 tasks
are ticked and their conditions are met. The gate passes.

## Checks

| Check | Result |
|---|---|
| `sluice exec bolt/sluices-own-module-writes-a-hosts-settings` on a320f81a, the bolt at 68d68a9a | **exit 0**: `passed: 27 tasks` in 6m 2s, `recorded 27 results under their keys for a320f81` |
| `nix:lint` | passes: statix over `flake.nix` and `nix/` |
| `nix:typecheck` | `nix: typecheck: every case holds` |
| `nix:test` | `nix: test: every case holds`, the evaluated cases and the activation's, under the gate's sandbox with `network = "localhost"` |
| `cli:test` | `instance::tests::the_modules_file_is_read_as_written ... ok`, among 228 unit tests passed |
| `root:format` | 361 files formatted, 0 changed |
| `root:lint`, `root:adrs`, `root:toolchain`, `root:marketplace` | pass |
| `root:changes` | 11 of 11 changes valid, this one among them |
| `root:specs` | 67 of 67 specs pass, before the archive |
| `site:prose`, `site:rules` | pass; `site:prose` 0 errors, 0 warnings |
| `plugin-sluice:validate`, `plugin-sluice:hooks`, `plugin-sluice:lay-out` | pass |
| `openspec validate sluice-publishes-a-module-for-its-host-settings --strict` | valid |
| The bolt's tip, `origin/bolt/sluices-own-module-writes-a-hosts-settings` | 68d68a9a, the branch's merge-base: the work is on the bolt as it stands |

The gate ran on the clean tree, before this report was written, so its
results were recorded under their keys (rule core.18). moon warned that
it could not store the tasks' outputs in its caches. Every task still
passed, and the warning does not come from this change.

## Requirements and scenarios

Every scenario has a case that sets up its WHEN and asserts its THEN.
The cases in `nix/tests/*.nix` ran in `nix:typecheck` or `nix:test`; the
cases in `nix/tests/activate.sh` ran in `nix:test`; the Rust test ran in
`cli:test`.

### host-settings-module

| Requirement | Scenario | Case |
|---|---|---|
| sluice's flake gives a module for nix-darwin and for NixOS | The nix-darwin module | "The nix-darwin module", `nix/tests/typecheck.nix` |
| | The NixOS module | "The NixOS module", `nix/tests/typecheck.nix` |
| The flake gives three systems | Every system's package | "Every system's package", `nix/tests/typecheck.nix` |
| The settings are the host's own file under its own names | Every key the module declares | "Every key", `nix/tests/file.nix`, against `nix/tests/files/every.toml` |
| | A key the module does not declare | "A key the module does not declare", `nix/tests/file.nix` |
| The file's form | No settings | "No settings", `nix/tests/file.nix`, against `nix/tests/files/empty.toml` |
| | A value the file cannot hold | "A value the file cannot hold", `nix/tests/file.nix`, which reads the failure's words through the renderer's `throw` |
| sluice reads the module's file as written | Reading the module's file | `instance::tests::the_modules_file_is_read_as_written`, `crates/sluice/src/instance.rs` |
| A key is named by its path, outside the store | A forge key in the store | "A forge key in the store", `nix/tests/file.nix` |
| | A signing key in the store | "A signing key in the store", `nix/tests/file.nix` |
| | A key's text in place of its path | "A key's text", `nix/tests/file.nix` |
| | A key given as a Nix path | "A key given as a Nix path", `nix/tests/file.nix` |
| The file is linked at sluice's path for the user who runs sluice | On nix-darwin | "On nix-darwin", `nix/tests/darwin.nix` |
| | A primary user with no home | "A primary user with no home", `nix/tests/darwin.nix` |
| | On NixOS | "On NixOS", `nix/tests/nixos.nix` |
| | A user the system does not declare | "A user the system does not declare", `nix/tests/nixos.nix` |
| What is at sluice's path already | Nothing there | "Nothing there", `nix/tests/activate.sh` |
| | A link there | "A link there", `nix/tests/activate.sh` |
| | A file there | "A file there", `nix/tests/activate.sh` |
| | A file there and one aside already | "A file there and one aside already", `nix/tests/activate.sh` |
| The module installs the package and nothing else | The flake's sluice | "The flake's sluice", `nix/tests/darwin.nix` and `nix/tests/nixos.nix` |
| | A host's own sluice | "A host's own sluice", in both |
| | No package | "No package", in both |
| | Nothing else on nix-darwin | "Nothing else on nix-darwin", `nix/tests/darwin.nix` |
| | Nothing else on NixOS | "Nothing else on NixOS", `nix/tests/nixos.nix` |

Read beside the cases:

- **A key given as a Nix path.** Evaluation cannot read a failure's
  words, so the case asserts that evaluation fails and not what the
  failure names. The words were read once by hand, through
  `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- nix eval …`
  against the same dummy store: `A definition for option
  'services.sluice.settings.forge.key' is not of type 'null or
  string'`. The option is named, and the type check fails before the
  path is copied anywhere.
- **The case names of "Nothing else".** The spec names the case of
  each "Nothing else" scenario "Nothing else on the host". The cases
  are named as the scenarios and tasks 3.1 and 3.2 name them, "Nothing
  else on nix-darwin" and "Nothing else on NixOS". The cases assert
  every THEN of their scenarios. Only the spec's citation differs from
  the cases' names, and the archive carries that citation into
  `openspec/specs/host-settings-module/spec.md`.
- **The activation in "Nothing else on nix-darwin".** The names under
  `environment.etc`, `launchd.*` and `users.users` are compared with
  the flake's package installed. The whole activation, with the link's
  line taken out, is compared on the system that installs no package,
  since installing a package changes nix-darwin's own `applications`
  step. That difference is the package the spec allows, not something
  the module adds.
- **The link's failure stops nothing else.** Each platform's link line
  ends in `|| true`, which "On nix-darwin" and "On NixOS" assert as
  part of the line.

### repository-gate

| Requirement | Scenario | Shown by |
|---|---|---|
| The flake is a project of its own | Listing the flake's project | `.moon/workspace.yml` lists `nix: 'nix'`; `nix/moon.yml` declares `language: 'nix'` and the tasks `lint`, `typecheck` and `test`, each tagged `check-fast`. The bolt's gate, `#check-fast`, ran all three |
| The flake's checks need no store and no network | A failing case | `nix/tests/default.nix` turns each case into a list of failures, each printed `nix: <case>: <what>`, so no failing case stops the rest. `nix/tests/activate.sh` prints `nix: activate: <case>: <what>`. `nix/check.sh` prints them and exits 1. Read from the code: no case fails today, and the spec names no case of its own for this scenario |
| | No store and no network | `nix:test` passed under the gate's sandbox, which allows localhost only. `nix/check.sh` evaluates with `--offline --read-only` against `dummy://?store=/sluice-check/store`, reading the inputs from `SLUICE_FLAKE_INPUTS`, which `devenv.nix` sets |
| The module's rendered files are kept byte for byte | Formatting the tree | `treefmt.nix` excludes `nix/tests/files/**`, with a comment giving the reason. `root:format` passes with 0 changed, and the byte-for-byte cases pass |

## Tasks

All 9 tasks are ticked, and each task's condition holds:

- **1.1, 1.2, 2.1, 2.3, 3.1, 3.2.** The conditions are `nix:lint`,
  `nix:typecheck`, `nix:test` and `root:format`, which passed in the
  gate. `flake.nix` gives the three systems, `darwinModules` and
  `nixosModules`, each with `default`. It takes nix-darwin as the
  tests' input, and `flake.lock` locks it. `devenv.nix` asserts nix
  2.34.8 and statix 0.5.8-unstable-2026-07-17 and sets
  `SLUICE_FLAKE_INPUTS`.
- **2.2.** `from_host` calls `from_text`, and the public readers are
  unchanged. `the_modules_file_is_read_as_written` passed in
  `cli:test`, which `root:lint`'s clippy reads with the rest.
  `crates/sluice/moon.yml` lists `/nix/tests/files/**/*` among
  `cli:test`'s inputs.
- **4.1.** `site/src/content/docs/reference/configuration.md` says what
  the module writes and refuses, with an example.
  `README.md`'s "As a flake input" names both modules. Neither names a
  host, fleet or tool of ours (core.26). `site:prose` passed with 0
  warnings.
- **4.2.** `plugins/sluice/skills/setting-up-sluice/SKILL.md`, setup 2,
  puts the tables in `services.sluice.settings` and says a key file is
  named by its path as a string. `plugin-sluice:validate` passed.

The records and rules the tasks put in the change's own commit are
there: record 0098 (`docs/adr/0098-…`, accepted), core.51, core.21
naming the flake's `nix/`, and both on the site's rules page, which
`site:rules` checks.
