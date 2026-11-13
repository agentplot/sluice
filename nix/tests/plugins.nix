# nix:test: each plugin the flake gives is the version of the `sluice` it
# is built with, and reads it from that package alone (record 0112 as
# amended 2026-11-12; core.56). With version.nix's case, which holds the
# package to Cargo.toml's version, the plugins are the tree's version. It
# compares versions the evaluation knows and reads no file (core.21).
t:
let
  inherit (t) lib outputs systems;

  # A version no release has, which a plugin given a `sluice` of it says
  # only where it reads its version from that `sluice`.
  other = "98.76.54";

  plugins = [ "herdr-plugin" ];

  # What fails of a system's plugin `name`.
  fails =
    system: name:
    let
      inherit (outputs.packages.${system}) default;
      plugin = outputs.packages.${system}.${name};
      given =
        (plugin.override {
          sluice = default // {
            version = other;
          };
        }).version;
    in
    t.expect (
      plugin.version == default.version
    ) "packages.${system}.${name}'s version is ${plugin.version}, and its sluice's ${default.version}"
    ++ t.expect (given == other) "packages.${system}.${name} given a sluice of ${other} says ${given}";
in
{
  "Each plugin is its sluice's version" = lib.concatMap (
    system: lib.concatMap (fails system) plugins
  ) systems;
}
