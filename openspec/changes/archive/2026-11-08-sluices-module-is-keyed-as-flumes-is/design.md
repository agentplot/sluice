# Design

## Context

`flake.nix` gives `darwinModules.sluice` as `import
./nix/modules/darwin.nix { inherit packages; }`, and `nixosModules.sluice`
the same of `nixos.nix`: each a function's value, a module with no file
and no key. nix's module system collects the modules a configuration
imports and takes each once by its key, which it gives a module
imported by path from the path, and an anonymous module from its
importer's key and its place among the importer's imports. Imported
from two places, sluice's module has two such keys, so it is taken
twice, and every option of `services.sluice` is declared twice, which
nix refuses.

### Looking across

- **Flume** gives its modules by `moduleOf platform file`: `_file` the
  module's file, `key` `github:agentplot/flume#${platform}Modules.flume`
  and the module given the flake's packages as its one import. Its
  `nix/tests/darwin.nix` and `nix/tests/nixos.nix` each hold "The module
  imported twice", which imports the module again under another module
  and counts the declarations of `services.flume.enable`.
- **Boomdrive**'s flake keys its own modules the same way,
  `github:agentplot/boomdrive#${platform}Modules.boomdrive`, and
  re-exports Flume's and sluice's (its record 0023).

One design, Flume's, serves all three; sluice takes it as it stands.

## Decisions

### D1. Each module is keyed as Flume's are

`flake.nix` gives each module by a `moduleOf platform file` of its own:
`_file` the module's file, so nix names it in a failure; `key`
`github:agentplot/sluice#${platform}Modules.sluice`; and the module
given the flake's packages as its one import. `default` is the same
value, so it carries the same key, and a configuration that imports
`default` here and `sluice` there takes one module.

The key names the module's output, not the flake's revision, so two
revisions of sluice's flake in one configuration are taken as one, the
first nix collects. That is Flume's choice, and a configuration that
pins two sluices is one this design need not serve.

Not taken: giving the module as a path, which nix keys by itself. The
module needs the flake's packages, so it would read them from a module
argument the host must set, which no configuration of the family sets.

### D2. A nix test imports each module twice

`nix/tests/darwin.nix` and `nix/tests/nixos.nix` each gain "The module
imported twice on nix-darwin" and "on NixOS", as Flume's tests have it:
a system from `t.darwinSystem` or `t.nixosSystem`, which import
`default`, given a module that imports `sluice` again. The case holds
where `sluice` and `default` carry the key, the system evaluates, its
`services.sluice.enable` has one declaration, and its activation holds
the link once enabled.

## Risks

- A host that had worked around the double import, by importing
  sluice's module in one place only, is unchanged.
