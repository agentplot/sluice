# nix:test: the package's version is the tree's, Cargo.toml's workspace
# version, read from the tree alone (record 0111; core.55). nix/check.sh
# evaluates with SLUICE_VERSION=98.76.54 exported, so a flake that read the
# builder's environment for its version would give that one; and no
# system's package hands its build a SLUICE_VERSION.
t:
let
  inherit (t) lib outputs systems;
  workspace = (builtins.fromTOML (builtins.readFile ../../Cargo.toml)).workspace.package.version;

  # What fails of a system's package.
  fails =
    system:
    let
      package = outputs.packages.${system}.default;
      given = package.version;
      attrs = package.drvAttrs;
    in
    t.expect (
      given == workspace
    ) "packages.${system}.default's version is ${given}, and Cargo.toml's ${workspace}"
    ++ t.expect (
      !(attrs ? SLUICE_VERSION || (attrs.env or { }) ? SLUICE_VERSION)
    ) "packages.${system}.default hands its build SLUICE_VERSION";
in
{
  "The package's version is the tree's" = lib.concatMap fails systems;
}
