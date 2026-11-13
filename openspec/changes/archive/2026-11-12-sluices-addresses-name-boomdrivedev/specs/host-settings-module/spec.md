## MODIFIED Requirements

### Requirement: sluice's flake gives a module for nix-darwin and for NixOS

sluice's flake SHALL give `darwinModules.sluice` and
`nixosModules.sluice`, each also `default`, declaring
`services.sluice.enable`, `services.sluice.package` and
`services.sluice.settings`, and `services.sluice.user` on NixOS. Each
SHALL evaluate in a system of its platform with the module enabled and
no other flake's module (boomdrivedev/boomdrive record 0023; record 0098;
rule core.51).

#### Scenario: The nix-darwin module

- **WHEN** an aarch64-darwin system whose primary user is `alice`, home
  `/Users/alice`, imports `darwinModules.default` and sets
  `services.sluice.enable = true`
- **THEN** the system evaluates to a derivation ("The nix-darwin module"
  in `nix/tests/typecheck.nix`)

#### Scenario: The NixOS module

- **WHEN** an x86_64-linux system with the normal user `alice` imports
  `nixosModules.default` and sets `services.sluice.enable = true` and
  `services.sluice.user = "alice"`
- **THEN** the system's toplevel evaluates to a derivation ("The NixOS
  module" in `nix/tests/typecheck.nix`)

### Requirement: The module installs the package and nothing else

With `services.sluice.enable`, the module SHALL put
`services.sluice.package` in the system profile. Its default SHALL be
sluice's flake's package for the host's system. A host MAY replace the
package, or set it to null to install none. The module SHALL add no
launchd agent or daemon, no systemd unit, no file under `/etc`, no user
and no list of repositories. Beside the link, it SHALL write no file of
another tool's (135; boomdrivedev/blueprints principle 10; rules
core.31, core.51).

#### Scenario: The flake's sluice

- **WHEN** the nix-darwin system of "The nix-darwin module" enables the
  module and sets no package
- **THEN** its system profile holds the flake's sluice for
  aarch64-darwin ("The flake's sluice" in `nix/tests/darwin.nix`)

#### Scenario: A host's own sluice

- **WHEN** it sets `services.sluice.package` to a package of its own
- **THEN** the system profile holds that package and not the flake's,
  and the activation's line is unchanged ("A host's own sluice" in
  `nix/tests/darwin.nix`)

#### Scenario: No package

- **WHEN** it sets `services.sluice.package = null`
- **THEN** the system profile holds no sluice, and the activation still
  holds the link's line ("No package" in `nix/tests/darwin.nix`)

#### Scenario: Nothing else on nix-darwin

- **WHEN** the nix-darwin system with the module enabled is compared with
  the same system with it not enabled
- **THEN** the names under `environment.etc`, `launchd.agents`,
  `launchd.daemons`, `launchd.user.agents` and `users.users` are equal,
  and the activation, with the link's line taken out, is equal ("Nothing
  else on the host" in `nix/tests/darwin.nix`)

#### Scenario: Nothing else on NixOS

- **WHEN** the NixOS system with the module enabled is compared with the
  same system with it not enabled
- **THEN** the names under `environment.etc`, `systemd.services` and
  `systemd.user.services` are equal, each user's `linger` is equal, and
  the user activations' names differ by `sluice` alone ("Nothing else on
  the host" in `nix/tests/nixos.nix`)

### Requirement: A module imported twice is taken once

`darwinModules.sluice` and `nixosModules.sluice`, and each `default`,
SHALL carry the key `github:boomdrivedev/sluice#<platform>Modules.sluice`,
`darwin` or `nixos`, as Flume's flake keys its own, so a system that
imports the module twice, itself and through another flake's module,
declares `services.sluice` once (boomdrivedev/boomdrive record 0023;
record 0098; rule core.51).

#### Scenario: The nix-darwin module imported twice

- **WHEN** an aarch64-darwin system whose primary user is `alice`, home
  `/Users/alice`, imports `darwinModules.default`, sets
  `services.sluice.enable = true`, and imports `darwinModules.sluice`
  again under another module
- **THEN** `darwinModules.sluice` and `darwinModules.default` each carry
  the key `github:boomdrivedev/sluice#darwinModules.sluice`, the system
  evaluates to a derivation, `services.sluice.enable` is declared once,
  and the activation links the file ("The module imported twice on
  nix-darwin" in `nix/tests/darwin.nix`)

#### Scenario: The NixOS module imported twice

- **WHEN** an x86_64-linux system with the normal user `alice` imports
  `nixosModules.default`, sets `services.sluice.enable = true` and
  `services.sluice.user = "alice"`, and imports `nixosModules.sluice`
  again under another module
- **THEN** `nixosModules.sluice` and `nixosModules.default` each carry
  the key `github:boomdrivedev/sluice#nixosModules.sluice`, the system's
  toplevel evaluates to a derivation, `services.sluice.enable` is
  declared once, and alice's user activation links the file ("The
  module imported twice on NixOS" in `nix/tests/nixos.nix`)

