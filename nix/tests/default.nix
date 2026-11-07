# The nix project's cases, which nix/check.sh evaluates against no store
# (record 0098): `typecheck` and `test`, one for each task, read the
# flake's inputs from SLUICE_FLAKE_INPUTS, the devenv shell's JSON of
# their store paths. Each case is a list of what fails, empty where it
# holds, so one failing case never stops the rest. A check's result is one
# line for each failure, `nix: <case>: <what>`.
let
  t = import ./lib.nix { paths = builtins.fromJSON (builtins.getEnv "SLUICE_FLAKE_INPUTS"); };
  inherit (t.lib) concatLists concatStringsSep mapAttrsToList;

  lines =
    cases:
    concatStringsSep "\n" (
      concatLists (mapAttrsToList (case: map (what: "nix: ${case}: ${what}")) cases)
    );
in
{
  typecheck = lines (import ./typecheck.nix t);
  test = lines { };
}
