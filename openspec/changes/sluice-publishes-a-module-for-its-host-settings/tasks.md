# Tasks

Work in this change's place on its branch, cut from
`bolt/sluices-own-module-writes-a-hosts-settings`, the base every
`sluice exec` below names. Run suites through sluice
(`sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- …`); a
quick `cargo check`, `cargo clippy` or one unit test may run directly.
The flake's checks evaluate offline and read-only against a dummy store,
with nix's configuration, caches and state in a scratch folder, and touch
nothing of the host's: no test reads or writes the host's own
`~/.config/sluice`, its state or its nix store. The design's Task notes
give each task's files and names. Commit by pathspec, Conventional
Commits, scoped to the part of the tree, citing clauses by number and
rules by id. Record 0098, rule core.51, core.21's amendment and the
site's rules page are in the change's own commit.

## 1. The flake's checks

- [x] 1.1 Give the flake its checks' tools, inputs and project, as the design's Decisions 1 and 8 and Task note 1.1 say. `flake.nix` gives aarch64-darwin, aarch64-linux and x86_64-linux and takes nix-darwin as the tests' input, and `flake.lock` locks it. `nix/inputs.nix` is Boomdrive's at 3f428cf. `devenv.nix` pins nix and statix by version and sets `SLUICE_FLAKE_INPUTS`. `nix/moon.yml` holds the project `nix` with its `lint` task, and `.moon/workspace.yml` lists it. True when `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- moon run nix:lint` passes.
- [x] 1.2 Add the type check and the test's frame, as Decision 8 and Task note 1.2 say: `nix/check.sh`, `nix/tests/default.nix`, `nix/tests/lib.nix`, and `nix/tests/typecheck.nix` with the host-settings-module spec's "Every system's package"; `nix/moon.yml` gains `typecheck` and `test`. True when `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- moon run nix:typecheck` passes.

## 2. The module's settings and file

- [x] 2.1 Write `services.sluice`'s shared options and the file they render, as Decisions 2 to 4 and Task note 2.1 say: `nix/modules/toml.nix`, Boomdrive's renderer at 3f428cf naming sluice's module; `nix/modules/options.nix`, with the key assertions and the dropping of what is unset; `nix/tests/settings.nix`, `nix/tests/files/empty.toml` and `nix/tests/files/every.toml`; and `nix/tests/file.nix` with the spec's "No settings", "Every key", "A key the module does not declare", "A value the file cannot hold", "A forge key in the store", "A signing key in the store", "A key's text" and "A key given as a Nix path". Exclude `nix/tests/files/**` in `treefmt.nix` with its reason. True when `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- moon run nix:test` and `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- moon run root:format` pass.
- [x] 2.2 Show that sluice reads what the module writes, as Decision 9 and Task note 2.2 say. In `crates/sluice/src/instance.rs`, split `from_host` so a test gives its text and its home, the public readers unchanged. Add `the_modules_file_is_read_as_written`, which reads `nix/tests/files/every.toml` as the spec's "Reading the module's file" says. Add `/nix/tests/files/**/*` to `cli:test`'s inputs in `crates/sluice/moon.yml`. True when `cargo test -p sluice --lib instance::tests::the_modules_file_is_read_as_written` passes and `cargo clippy -p sluice --all-targets -- -D warnings` is clean.
- [x] 2.3 Write the activation's link, as Decision 5 and Task note 2.3 say: `nix/modules/activate.sh link <file>`, and `nix/tests/activate.sh`, which `nix/check.sh test` runs after the evaluated cases. It holds the spec's "Nothing there", "A link there", "A file there" and "A file there and one aside already". True when `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- moon run nix:test` passes.

## 3. The modules on each platform

- [x] 3.1 Write the nix-darwin module, as Decisions 5 and 6 and Task note 3.1 say: `nix/modules/darwin.nix`, `darwinModules.sluice` and `default` in `flake.nix`, and `darwinSystem` in `nix/tests/lib.nix`. Add "The nix-darwin module" to `nix/tests/typecheck.nix`. Add `nix/tests/darwin.nix` with the spec's "On nix-darwin", "A primary user with no home", "The flake's sluice", "A host's own sluice", "No package" and "Nothing else on nix-darwin". True when `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- moon run nix:typecheck` and `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- moon run nix:test` pass.
- [ ] 3.2 Write the NixOS module, as Decisions 5 and 6 and Task note 3.2 say: `nix/modules/nixos.nix` with `services.sluice.user`, `nixosModules.sluice` and `default` in `flake.nix`, and `nixosSystem` in `nix/tests/lib.nix`. Add "The NixOS module" to `nix/tests/typecheck.nix`. Add `nix/tests/nixos.nix` with the spec's "On NixOS", "A user the system does not declare", "The flake's sluice", "A host's own sluice", "No package" and "Nothing else on NixOS". True when `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- moon run nix:typecheck` and `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- moon run nix:test` pass.

## 4. Say it to the operator

- [ ] 4.1 In `site/src/content/docs/reference/configuration.md`, "Who writes it", say what sluice's own module for nix-darwin and NixOS writes and refuses, with a short example, as Decision 10 says. In `README.md`, "As a flake input", name the modules beside the package. Name no host, fleet or tool of ours (core.26). True when `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- moon run site:prose` passes with no warning on the page. `README.md` has no check of its own: no gate reads `*.md` outside the site (core.21's details).
- [ ] 4.2 In `plugins/sluice/skills/setting-up-sluice/SKILL.md`, "The host's file", setup 2, say that where the host's configuration imports sluice's module, the tables go in `services.sluice.settings`, each key named as in the file, as Decision 10 says. True when `sluice exec bolt/sluices-own-module-writes-a-hosts-settings -- moon run plugin-sluice:validate` passes.
