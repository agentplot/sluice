# What every case reads: flake.nix imported as an attribute set, and its
# outputs called with the lock's inputs and `self` as this tree, as
# flake-compat calls them (record 0098). agentplot/boomdrive's
# nix/tests/lib.nix at 3f428cf, for sluice's flake.
{ paths }:
let
  inherit ((import ../inputs.nix { inherit paths; })) inputs;
  flake = import ../../flake.nix;
  outputs = flake.outputs (inputs // { inherit self; });
  # The tree by its path, never copied: the checks write nothing.
  self = outputs // {
    outPath = toString ../..;
    inherit inputs outputs;
    _type = "flake";
  };
in
{
  inherit
    flake
    inputs
    outputs
    self
    ;
  inherit (inputs.nixpkgs) lib;

  # The systems the flake gives: nixpkgs refuses x86_64-darwin since
  # 26.11.
  systems = [
    "aarch64-darwin"
    "aarch64-linux"
    "x86_64-linux"
  ];

  # A failure where `holds` is false, as a list of what fails.
  expect = holds: what: if holds then [ ] else [ what ];
}
