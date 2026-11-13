# nix:typecheck: everything the flake gives evaluates, each module in a
# system of its platform (record 0098).
t:
let
  inherit (t) lib outputs systems;

  # A system's package `name`: a derivation whose path evaluates.
  evaluates =
    name: system:
    let
      package = outputs.packages.${system}.${name} or null;
      drvPath = builtins.tryEval (builtins.seq package.drvPath true);
    in
    if !lib.isDerivation package then
      [ "packages.${system}.${name} is not a derivation" ]
    else
      t.expect drvPath.success "packages.${system}.${name} does not evaluate to a derivation";

  # The plugins the flake gives beside each system's sluice (record 0112 as
  # amended 2026-11-12; core.56).
  plugins = [ "herdr-plugin" ];
in
{
  "Every system's package" =
    t.expect (
      builtins.attrNames outputs.packages == lib.sort lib.lessThan systems
    ) "packages gives ${toString (builtins.attrNames outputs.packages)}, not ${toString systems}"
    ++ lib.concatMap (
      system:
      if outputs.packages ? ${system} then
        evaluates "default" system
      else
        [ "packages gives no ${system}" ]
    ) systems;

  # Each plugin evaluates, its source the tree by its path, so nothing is
  # copied.
  "Every system's plugins" = lib.concatMap (
    system:
    if outputs.packages ? ${system} then lib.concatMap (name: evaluates name system) plugins else [ ]
  ) systems;

  # The nix-darwin module in a minimal system of its platform, whose
  # derivation throws on a failed assertion.
  "The nix-darwin module" =
    t.expect (builtins.tryEval (t.darwinSystem { }).system.drvPath).success
      "a nix-darwin system with services.sluice enabled does not evaluate";

  # The NixOS module in a minimal system of its platform, whose toplevel
  # throws on a failed assertion.
  "The NixOS module" =
    t.expect (builtins.tryEval (t.nixosSystem { }).config.system.build.toplevel.drvPath).success
      "a NixOS system with services.sluice enabled does not evaluate";
}
