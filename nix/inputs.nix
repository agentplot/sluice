# The flake's inputs as flake.lock locks them, for the devenv shell and the
# nix project's checks, which evaluate flake.nix's outputs without nix's
# flake machinery (record 0098). Each node of the lock is its locked
# source: fetched by builtins.fetchTree, as the devenv shell does outside
# the gate's sandbox, or, where `paths` names it, read where the shell
# fetched it to, as the checks do inside it against no store. A flake's
# node is its outputs, called with its own inputs and `follows` followed
# from the root, as flake-compat does. agentplot/boomdrive's
# nix/inputs.nix at 3f428cf, its comments naming sluice's shell and checks.
{
  lock ? builtins.fromJSON (builtins.readFile ../flake.lock),
  paths ? { },
}:
let
  inherit (builtins)
    head
    isList
    mapAttrs
    removeAttrs
    substring
    tail
    ;
  inherit (lock) nodes root;

  # A node's input: a node's name, or a `follows` path from the root.
  resolve = spec: if isList spec then follow root spec else spec;
  follow =
    from: path:
    if path == [ ] then from else follow (resolve nodes.${from}.inputs.${head path}) (tail path);

  # A node's source, as fetchTree gives it.
  sourceOf =
    name:
    let
      inherit (nodes.${name}) locked;
    in
    if paths ? ${name} then
      removeAttrs locked [ "dir" ]
      // {
        outPath = paths.${name};
      }
      // (if locked ? rev then { shortRev = substring 0 7 locked.rev; } else { })
    else
      builtins.fetchTree (removeAttrs locked [ "dir" ]);

  nodeOf =
    name:
    let
      node = nodes.${name};
      sourceInfo = sourceOf name;
      dir = if node.locked ? dir then "/${node.locked.dir}" else "";
      inputs = mapAttrs (_: spec: resolved.${resolve spec}) (node.inputs or { });
      outputs = (import (sourceInfo.outPath + dir + "/flake.nix")).outputs (inputs // { self = flake; });
      flake =
        outputs
        // sourceInfo
        // {
          inherit inputs outputs sourceInfo;
          _type = "flake";
        };
    in
    if node.flake or true then flake else sourceInfo;

  resolved = mapAttrs (name: _: nodeOf name) (removeAttrs nodes [ root ]);
in
{
  # The root's inputs, as flake.nix's outputs takes them.
  inputs = mapAttrs (_: spec: resolved.${resolve spec}) nodes.${root}.inputs;

  # Each node's source by its store path, the JSON of
  # SLUICE_FLAKE_INPUTS.
  paths = mapAttrs (name: _: (sourceOf name).outPath) (removeAttrs nodes [ root ]);
}
