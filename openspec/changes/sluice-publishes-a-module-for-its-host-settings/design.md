## Context

sluice reads the host's own settings from
`$XDG_CONFIG_HOME/sluice/config.toml`, `~/.config/sluice/config.toml`
where that is unset (`crates/sluice/src/instance.rs`, `config_path`). It
reads the file at every command, never writes it, and refuses a field it
does not know (`ConfigForm` and its tables, `deny_unknown_fields`). The
tables are `[runs]` (`max`, `cores`, `reserved`, `systems`), `[forge]`
(`app_id`, `key`, `discovery`), `[signing]` (`signer`, `key`), `[peers]`
(`lease`, `alive`, `name`) and `[[caches]]` (`dir`, `socket`, `env`)
(35, 71, 74, 78, 87, 136). `[forge] key` and `[signing] key` are paths to
private keys, `/…` or `~/…`, and sluice refuses a key file anyone but
its owner may read (`private_key`). Branch protection's hook reads
`[forge]` alone and passes over the rest (`Instance::guarding`).

sluice's `flake.nix` builds the package for four systems and gives
nothing else. Its nixpkgs has refused x86_64-darwin since 26.11, so
`packages.x86_64-darwin.default` no longer evaluates (read 2026-11-07:
the evaluation fails, pointing at nixpkgs' 26.11 release notes).

agentplot/boomdrive record 0023 (accepted 2026-11-06) has each tool's
own flake publish a module that writes the tool's own file, each key
named as the file names it, and never another tool's. The Boomdrive
flake re-exports each module and sets its options. The record's sketch
of a host sets `services.sluice.settings` with `runs`, `caches`,
`peers.name`, `forge` and `signing`, and says "A key's file is named by
its path; no key is in the store". Principle 10 of agentplot/blueprints
`design/principles.md` calls that file the managed layer, `config.toml`.
The local layer, `config.local.toml`, is read over it. Boomdrive's
design says sluice takes the layers "when it gains layers".

The family's modules have one shape, Boomdrive's at 3f428cf (its record
0020 and design Decisions 8 to 14). `nix/modules/options.nix` holds the
options, given the flake's packages. `toml.nix` renders the file at
evaluation. `darwin.nix` and `nixos.nix` link it by `activate.sh link`.
The checks are a moon project `nix` that evaluates offline and read-only
against a dummy store (`nix/check.sh`, `nix/inputs.nix`, `nix/tests/`).
afterthought/crew's change `crew-publishes-a-flake-whose-module-writes-its-host-file`
copies that shape for crew's file, citing Boomdrive's files by revision.
agentplot/flume's own module, record 0055, is planned beside this. This
design takes the same shape for sluice's file and cites Boomdrive's
files at 3f428cf.

Rules this design keeps: core.21 (the flake's part is a moon project of
its own, one task per check, its rendered files excluded by name with
their reason), core.24 (nix and statix are the versions `devenv.nix`
pins), core.26 (the pages name no host or tool of ours), core.31 and
clause 135 (nothing installed for sluice to run), and core.51 (added
with this change). Record 0098 holds the decisions below.

## Goals / Non-Goals

**Goals:**

- A nix-darwin or NixOS host writes sluice's host settings as
  `services.sluice.settings`, each key named as the file names it, from
  sluice's own flake, with no Boomdrive.
- No key reaches the store by the module.
- The module writes sluice's file and nothing else, and installs nothing
  for sluice to run.
- The gate shows the module's text, its links and that sluice reads what
  it writes, offline and with no store.

**Non-Goals:**

- sluice reading a local layer, `config.local.toml`.
- A home-manager module.
- The Boomdrive flake re-exporting this module or setting its options.
- Any change to what sluice reads or how.

## Decisions

### Decision 1: the modules, the systems and the layout

`flake.nix` gives `darwinModules = rec { sluice = import ./nix/modules/darwin.nix { inherit packages; }; default = sluice; }`,
and `nixosModules` likewise from `nixos.nix`. `packages` is the flake's
own `packages` output. Both modules import `nix/modules/options.nix`,
given `{ packages }`. Everything of the flake but `flake.nix` and
`flake.lock` is under `nix/`, as Boomdrive's is. `flake.nix`'s opening
comment says it gives the modules too.

The flake's `systems` become aarch64-darwin, aarch64-linux and
x86_64-linux. With x86_64-darwin listed, "Every system's package" could
not evaluate. No release builds that system: the README offers macOS on
Apple silicon and Linux on x86-64 and ARM.

The tests take nix-darwin as an input of the flake, following its
nixpkgs, as Boomdrive's does. A host's own configuration brings its own
nix-darwin, and the modules import nothing of it.

Not taken: a home-manager module. A nix-darwin or NixOS host need not
run home-manager, and the family's modules are system modules. Also not
taken: Boomdrive carrying sluice's module, which record 0023 rejects as
its option 3.

### Decision 2: `services.sluice`'s options, named as the file names them

- `enable`: `lib.mkEnableOption "sluice's host settings, linked for the user who runs sluice, and its package"`.
- `package`: `types.nullOr types.package`, default the flake's
  `packages.${pkgs.stdenv.hostPlatform.system}.default`. Its
  `defaultText` names that path. A system the flake gives no package for
  fails evaluation naming the systems it gives, as Boomdrive's `set`
  does. `null` installs none.
- `settings`: `types.submodule { freeformType = (pkgs.formats.toml { }).type; options = …; }`,
  default `{ }`. It declares each key sluice reads, each
  `types.nullOr <type>` with default `null`:
  - `runs`: a submodule, default `{ }`, freeform likewise. `max`,
    `cores` and `reserved` are `types.int`; `systems` is
    `types.listOf types.str`.
  - `forge`: `nullOr` a submodule, freeform. `app_id` is `types.int`;
    `key` and `discovery` are `types.str`.
  - `signing`: `nullOr` a submodule, freeform. `signer` and `key` are
    `types.str`.
  - `peers`: a submodule, default `{ }`, freeform. `lease`, `alive` and
    `name` are `types.str`.
  - `caches`: `nullOr (types.listOf` a submodule, freeform`)`, default
    `null`. `dir` is `types.str`; `socket` is `nullOr types.str`; `env`
    is `nullOr (types.attrsOf types.str)`, default `null`.

  Each description says what the key sets, sluice's default where it is
  unset, and the clause it rests on. The descriptions are in the
  configuration page's words.
- `user`, NixOS only: `types.str`, no default. The user whose
  `~/.config/sluice/config.toml` is linked (Decision 5).

Types and nothing finer. A bound, a duration's grammar, a path's form
and a default are sluice's. sluice refuses a value it cannot take at its
read, naming the setting (`[runs] cores must be at least 1`), so the
module is no second home of a rule or a default that could drift.
`settings` is free-form, as RFC 42 has it and Boomdrive's is: a key the
module does not declare passes to the file, and sluice, which refuses
any field it does not know, judges it. A `package` set to another sluice
then gets the keys it reads.

A table left empty, `peers = { }` or `runs = { }`, renders nothing
(Decision 4). So the defaults of `{ }` give no `[runs]` or `[peers]`
header.

Not taken: a closed set of options with bounds and grammars. Every rule
of sluice's would live twice, and a key a newer sluice reads would wait
on the module. Also not taken: `caches` as an attribute set by name.
The file's `[[caches]]` is a list, and record 0023 has each key named as
the file names it.

### Decision 3: a key is named by its path, outside the store

The store is read by every user. `options.nix` asserts that
`settings.forge.key` and `settings.signing.key`, where set, each begin
with `/` or `~/` and do not begin with `builtins.storeDir + "/"`. Each
failure is its own message, naming the option and saying why:
`services.sluice.settings.forge.key is in the Nix store, which every
user reads: put the key on the host, mode 0600, by hand or by the host's
handling of secrets, and name its path`. A key that is no path gets the
same message with "is no path, /… or ~/…". A Nix path value, `./app.pem`,
fails `types.str` before anything copies it, so the module never puts a
key in the store.

sluice already refuses a key file others may read, and a store file is
mode 0444. But that refusal comes at sluice's read, after the key is in
the store. The module must refuse at evaluation.

Not taken: Boomdrive's refusal of every key named `key`, `token`,
`secret` or `password`. Here it would refuse sluice's own `key` paths,
which name a file and hold no secret. Also not taken: a secrets
integration of the module's own. The host's way of placing secrets is
the host's, and the module names the path.

### Decision 4: the file is rendered at evaluation

`nix/modules/toml.nix` is Boomdrive's renderer at 3f428cf, with two
changes. Its header is
`# Written by sluice's flake module from services.sluice.settings; a change goes there.`
Its failure names `services.sluice.settings.<path>`. Its rules are
unchanged:

- top-level keys first, then tables, each sorted by name;
- a null value is unset and left out;
- a table that holds only tables is named by theirs, and a table that
  holds nothing at all renders nothing;
- strings, booleans, integers, lists of strings, tables and lists of
  tables, anything else failing with its path.

The file is `pkgs.writeText "sluice-config.toml" (render cfg.settings)`.
The header line is a TOML comment, which sluice's reader passes over.

That renderer writes `[runs]` for an empty `runs` table, since it holds
neither keys nor tables, and writes an empty list inline, `caches = []`.
Settings with nothing set must render the header alone ("No settings").
So `caches` and each cache's `env` default to null, and the module drops
what is unset before rendering. It drops every null at any depth, then
every table left with no key, at any depth. It never drops a list, an
empty one included, since `systems = [ ]` written by a host says
something. This is done in `options.nix` before `render`, not in the
renderer, so the renderer stays Boomdrive's.

Why at evaluation: the gate's check evaluates against a dummy store and
builds nothing (Decision 8), so the text has to exist at evaluation to be
compared. One `settings` then gives one text on either platform. nixpkgs
has no TOML renderer at evaluation; `pkgs.formats.toml`'s `generate`
renders in a build. Not taken: importing Boomdrive's renderer from its
flake. Boomdrive takes sluice's flake as an input, so sluice's flake
would take its own taker as an input. Boomdrive owns the family's module
shape, and sluice cites its files by revision, as crew's change does.

### Decision 5: the activation links the file for the user who runs sluice

sluice reads its settings from the user's own home, so the module writes
into one user's home. sluice's host process and its state are per user
(135). `nix/modules/activate.sh link <file>` is Boomdrive's `link`, for
`$HOME/.config/sluice/config.toml`:

- a link to `<file>` there already: nothing done, nothing said;
- any other link there: replaced;
- a file or folder there that is no link: moved once to
  `config.toml.before-nix`, both paths named;
- where `config.toml.before-nix` is there already: neither changed, both
  named, exit 1.

It makes `~/.config/sluice` where it is missing and links the file
alone, never the folder. The folder holds the host's keys,
`~/.config/sluice/app.pem` as the configuration page writes it, and the
module never writes them. It says a line only where it moved or linked
something, and prints usage and exits 2 on any other arguments. It
carries no `keep`: sluice has no unit to restart.

- **nix-darwin**: in `system.activationScripts.postActivation`, as the
  primary user (`system.requiresPrimaryUser = [ "services.sluice.enable" ]`):
  `sudo --user=<user> -- /usr/bin/env HOME=<home> /bin/sh <activate.sh> link <file> || true`.
  A primary user with no `users.users.<user>.home` fails evaluation,
  naming that option.
- **NixOS**: `system.userActivationScripts.sluice`,
  `if [ "$(id -un)" = <user> ]; then <runtimeShell> <activate.sh> link <file> || true; fi`.
  Every user's activation runs at their manager's start and at a switch.
  A `services.sluice.user` that `users.users` declares as neither a
  normal nor a system user fails evaluation, naming
  `services.sluice.user` and `users.users.<user>`.

`|| true` on both: a link that fails says why and stops nothing else of
the system's activation.

sluice reads the file at its next command. A host process already
running keeps what it read until it ends (135), as after a hand edit, and
the module restarts nothing.

Not taken:

- A file written in place by `install`. A link names the generation
  that wrote it, and the configuration page and the setting-up skill
  already read a link as "managed".
- A sluice flag or variable naming the file. Every caller of every
  command would have to pass it.
- Linking at a non-default `XDG_CONFIG_HOME`. The activation runs
  outside the user's session and cannot read their environment.

### Decision 6: the package, and nothing else on the host

`environment.systemPackages = lib.optional (cfg.package != null) cfg.package`.
Beside the activation's line, the module adds nothing:

- no launchd agent or daemon, and no systemd unit (135, core.31);
- no file under `/etc`, no user and no lingering;
- no list of repositories;
- no file of the herdr plugin's, worktrunk's, Claude Code's, Flume's or
  the runner's (principle 10).

A case compares each platform's system with the module enabled against
the same system with it not enabled.

`package = null` is for a host that installs sluice its own way, by a
release binary or cargo, and writes its settings alone. Not taken:
crew's package with no null. crew's has to be there because Boomdrive
reads crew's plugin path from it. Nothing reads sluice's package from
this module; Boomdrive packages the herdr plugin itself.

### Decision 7: the managed layer alone

The module writes `config.toml`, the managed layer by principle 10's
words. sluice reads no local layer today. When it gains one, the local
file is the operator's or a control plane's and never the module's, and
the module's options still name the managed file alone. Which keys a
local layer may set, and the managed layer's pins, wait on that change.

### Decision 8: the checks, a moon project `nix`

`nix/moon.yml`, `language: 'nix'`, holds three tasks. Each is tagged
`check-fast` and runs from the workspace root.

- `lint`: `statix check flake.nix && statix check nix`.
- `typecheck`: `sh nix/check.sh typecheck`.
- `test`: `sh nix/check.sh test`.

The flake's format is `root:format`'s: treefmt's nixfmt and deadnix over
the `.nix` files, and shellcheck and shfmt over the `.sh` files, which
core.21 allows. A `nix:format` would run the same nixfmt again.

`nix/check.sh` is Boomdrive's at 3f428cf, with two changes:
`SLUICE_FLAKE_INPUTS` in place of `BOOMDRIVE_FLAKE_INPUTS`, and the store
`dummy://?store=/sluice-check/store`. It runs
`nix eval --raw --offline --read-only --impure --store <dummy> --file nix/tests <check>`,
with nix's configuration, caches and state in a scratch folder. So it
computes every path, writes none, and needs neither the network nor the
daemon, which the gate's sandbox denies. Its `test` also runs
`sh nix/tests/activate.sh`. Each failure is one line,
`nix: <case>: <what>`, and every case runs.

`nix/inputs.nix` is Boomdrive's at 3f428cf, its comment naming sluice's
shell and checks. It turns `flake.lock` into the inputs `flake.nix`'s
outputs take: by `builtins.fetchTree` in the shell, and by the store
paths of `SLUICE_FLAKE_INPUTS` in the check.

`devenv.nix` gains:

- `pkgs.nix` and `pkgs.statix`, each asserted to its version as
  Boomdrive's `devenv.nix` asserts them, with a comment citing core.24;
- `env.SLUICE_FLAKE_INPUTS = builtins.toJSON (import ./nix/inputs.nix { }).paths;`.

treefmt's nixfmt and deadnix are already the tree's, from the same
nixpkgs.

`nix/tests/` follows Boomdrive's:

- `default.nix`: `typecheck` and `test`, each the lines of its cases;
- `lib.nix`: the flake, the lock's inputs, `outputs`, `self` by its
  path, `lib`, `systems`, `expect`, and `darwinSystem` and `nixosSystem`
  once the module cases need them;
- `typecheck.nix`, `settings.nix`, `file.nix`, `darwin.nix`,
  `nixos.nix`, `activate.sh`, and `files/empty.toml` and
  `files/every.toml`.

Each task's inputs are what its outcome depends on: `/flake.nix`,
`/flake.lock`, `/nix/**/*`, `/devenv.nix`, `/devenv.lock`, and, for
`typecheck`, `/Cargo.toml`, `/Cargo.lock`, `/.prototools` and
`/crates/**/*`, which the package's derivation reads.

`treefmt.nix` excludes `nix/tests/files/**` with the comment "The
module's rendered files, compared byte for byte by the flake's cases;
taplo would rewrite them".

`.moon/workspace.yml` lists `nix: 'nix'`. The projects comment above it
says the flake's checks are its own project.

Not taken: a build of the file or of a system in the gate. The sandbox
gives no daemon, and a darwin system cannot be built on CI's Linux.

### Decision 9: sluice reads what the module writes

`crates/sluice/src/instance.rs` gains the unit test
`the_modules_file_is_read_as_written`. It reads
`include_str!("../../../nix/tests/files/every.toml")` through
`from_host`'s reading, on a host of sixteen cores. A home of the test's
own holds `.config/sluice/app.pem` and `.config/sluice/signing` at mode
`0600`. The test asserts each field of the `Instance` that
`settings.nix`'s `every` names.

The reader expands `~/` from `HOME`, and unit tests run on threads of
one process. So the test gives the home to the reader rather than
setting `HOME`. The build stage splits `from_host` so that its text and
its home can be given, the public readers unchanged.

The reader refuses a field it does not know. So a key the module
declares that sluice does not read, or a form of the renderer that
sluice reads otherwise, fails sluice's own tests.

`crates/sluice/moon.yml`'s `test` task gains `/nix/tests/files/**/*` among
its inputs, so a change to the fixture runs it again.

### Decision 10: what the operator reads

- `site/src/content/docs/reference/configuration.md`, "Who writes it",
  "A host under configuration management": a paragraph on sluice's own
  module for nix-darwin and NixOS. It covers:
  - `services.sluice.settings`, each key named as in the file;
  - a key file named by its path and kept out of the store, evaluation
    failing otherwise;
  - the file linked at `~/.config/sluice/config.toml`, a file there
    moved aside once;
  - the package in the system profile, `package = null` installing none;
  - nothing else written or installed.

  With a short example.
- `README.md`, "As a flake input": the modules beside the package, in
  two lines and a short example.
- `plugins/sluice/skills/setting-up-sluice/SKILL.md`, "The host's
  file", setup 2: where the link points into the store and the host's
  configuration imports sluice's module, the tables go in
  `services.sluice.settings`, each key named as in the file.

None names a host, a fleet or a tool of ours (core.26). nix-darwin and
NixOS are named as products.

## Risks / Trade-offs

- **Two writers of one path while a host moves.** A host that enables
  the module while another writer, a home-manager file among them, still
  links `~/.config/sluice/config.toml` has two writers. Whichever
  activation runs last wins. The host drops the other writer in the
  deploy that enables the module.
- **The first entry into the shell after `flake.lock` moves fetches the
  inputs.** devenv's evaluation fetches nixpkgs, rust-overlay and
  nix-darwin once for each lock, as Boomdrive's shell does. An entry
  with no network then fails the flake's checks for want of them.
- **The renderer is a copy of Boomdrive's.** A fix to one is not a fix
  to the other. Each file's opening comment names the revision it was
  taken from, so a reader can compare.
- **A key path sluice cannot read still evaluates.** The module refuses
  a store path and a non-path. A path to a missing file, or to one of
  the wrong mode, is refused by sluice at its read, naming the file and
  the mode it needs, as for a hand-written file.

## Task notes

**Base.** The place's base is
`bolt/sluices-own-module-writes-a-hosts-settings`. Every
`sluice exec <base>` below names it.

**1.1** `flake.nix`: the opening comment says the flake also gives the
modules (Decision 1); `systems` loses `x86_64-darwin`; the input
`nix-darwin = { url = "github:nix-darwin/nix-darwin"; inputs.nixpkgs.follows = "nixpkgs"; }`
with the comment "The tests' nix-darwin; a host's configuration brings
its own". `flake.lock` from `nix flake lock`, run in the shell.
`nix/inputs.nix`: Boomdrive's at 3f428cf, its comment naming
`SLUICE_FLAKE_INPUTS`. `devenv.nix`: `pkgs.nix` and `pkgs.statix` with
version asserts, and `env.SLUICE_FLAKE_INPUTS`, each with a comment
citing core.24 and record 0098. `nix/moon.yml` with the opening comment
and `lint`. `.moon/workspace.yml`: `nix: 'nix'`, and the comment.

**1.2** `nix/check.sh` (Decision 8). `nix/tests/default.nix`;
`nix/tests/lib.nix`, Boomdrive's, with `systems` the three of Decision
1; `nix/tests/typecheck.nix` with "Every system's package". It asserts
`builtins.attrNames outputs.packages` is the three, and each one's
`default.drvPath` evaluates. `nix/moon.yml`: `typecheck` and `test`,
`test` with no case yet.

**2.1** `nix/modules/toml.nix` (Decision 4). `nix/modules/options.nix`
(Decisions 2 and 3, and Decision 4's dropping of empty tables, exposed
as the rendered text the platform modules write).
`nix/tests/settings.nix`: `empty = { }` and `every` as "Every key"
names it, using names no reader is told to adopt (`host-a`,
`aarch64-darwin` and `x86_64-linux` as the systems). The caches:

- `{ dir = "~/.cache/sccache"; socket = "~/.cache/sccache/server.sock"; env = { RUSTC_WRAPPER = "sccache"; SCCACHE_DIR = "~/.cache/sccache"; }; }`;
- `{ dir = "~/.moon/cache/shared"; env.MOON_CACHE_SHARED_WORKTREE_CACHE = "true"; }`.

`nix/tests/files/empty.toml` and `every.toml`: the text the renderer
gives, written once, read and checked by hand against the configuration
page's example, then committed. `nix/tests/file.nix`, over `options.nix`
evaluated alone with `lib.evalModules` beside an `assertions` option, as
Boomdrive's is. Its cases are the spec's:

- "No settings", "Every key", "A key the module does not declare";
- "A value the file cannot hold", by `builtins.tryEval` of the rendered
  text and the failure's words;
- "A forge key in the store", "A signing key in the store", "A key's
  text", by the assertions' messages;
- "A key given as a Nix path", by `tryEval` of the option's value.

`treefmt.nix`: the exclude of Decision 8.

**2.2** `crates/sluice/src/instance.rs` (Decision 9): split `from_host`
so the test can give its text and home; add
`the_modules_file_is_read_as_written`, citing 35, 71, 74, 78, 87, 136
and record 0098. `crates/sluice/moon.yml`: the input.

**2.3** `nix/modules/activate.sh` (Decision 5). `nix/tests/activate.sh`,
which `nix/check.sh test` runs after the evaluated cases. Each case runs
in a scratch `HOME` and prints `nix: activate: <case>: <what>` for each
failure. The cases are the spec's "Nothing there", "A link there", "A
file there" and "A file there and one aside already".

**3.1** `nix/modules/darwin.nix` (Decisions 5 and 6), given
`{ packages }`, importing `options.nix`. `flake.nix`: `darwinModules`.
`nix/tests/lib.nix`: `darwinSystem`, as Boomdrive's: aarch64-darwin,
primary user alice at `/Users/alice`, `system.stateVersion = 6`,
`services.sluice.enable = true`, and the module `given`.
`nix/tests/typecheck.nix`: "The nix-darwin module". `nix/tests/darwin.nix`:
"On nix-darwin", "A primary user with no home", "The flake's sluice", "A
host's own sluice" (`pkgs.writeTextDir "bin/sluice" ""`), "No package",
and "Nothing else on the host".

**3.2** `nix/modules/nixos.nix`, as 3.1 for NixOS, with the `user`
option and its assertion. `flake.nix`: `nixosModules`.
`nix/tests/lib.nix`: `nixosSystem`, as Boomdrive's: x86_64-linux, normal
users alice and bob, the module enabled for alice, Boomdrive's minimal
boot and file system. `nix/tests/typecheck.nix`: "The NixOS module".
`nix/tests/nixos.nix`: "On NixOS", "A user the system does not declare"
(`carol`), "The flake's sluice", "A host's own sluice", "No package", and
"Nothing else on the host". `nix/tests/default.nix` takes `file.nix`,
`darwin.nix` and `nixos.nix` into `test` as each lands.
