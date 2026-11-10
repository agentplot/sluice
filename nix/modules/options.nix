# services.sluice, the options both modules share (record 0098, core.51),
# given the flake's own `packages`: sluice's package for the host's
# system, which a host may replace or set to null; and `settings`, the
# host's own settings, ~/.config/sluice/config.toml, under the file's own
# names, free-form TOML with every key sluice reads declared for its type
# (RFC 42). A key left unset is left out of the file, so sluice's own
# default holds and the two never drift; a key the module does not
# declare passes to the file, and sluice, which refuses a field it does
# not know, judges it. A bound, a duration's grammar or a path's form is
# sluice's to refuse at its read, naming the setting. A key is named by
# its path outside the store, since the file is in the store, which every
# user reads: a key in the store, or one that is no path, fails
# evaluation (78, 136). `forge.credential`, a command that gives the
# app's token in place of its key, is a list of its program and then each
# argument, and an argument that holds a GitHub token's text fails
# evaluation for the same reason; a program in the store is no secret
# (record 0108, core.51).
{ packages }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (lib) mkOption types;
  cfg = config.services.sluice;
  toml = (pkgs.formats.toml { }).type;
  render = import ./toml.nix { inherit lib; };

  inherit (pkgs.stdenv.hostPlatform) system;
  flakes =
    packages.${system}.default
      or (throw "sluice's flake gives no package for ${system}: it gives ${lib.concatStringsSep ", " (builtins.attrNames packages)}");

  # A key of the file: unset, and so left out, unless the host sets it.
  unset =
    type: description:
    mkOption {
      type = types.nullOr type;
      default = null;
      inherit description;
    };

  # A table of the file, free-form beside the keys it declares.
  table =
    options:
    types.submodule {
      freeformType = toml;
      inherit options;
    };

  # What is unset taken out before rendering: every null at any depth,
  # then every table left with no key. A list stays, an empty one too,
  # since a host that writes `systems = [ ]` says something.
  prune =
    value:
    if builtins.isAttrs value then
      lib.filterAttrs (_: v: v != null && v != { }) (builtins.mapAttrs (_: prune) value)
    else if builtins.isList value then
      map prune value
    else
      value;

  # A key's failures: none where it is unset, or a path outside the store.
  keyOf = name: if cfg.settings.${name} == null then null else cfg.settings.${name}.key;
  outside =
    name:
    let
      key = keyOf name;
      option = "services.sluice.settings.${name}.key";
      put = "put the key on the host, mode 0600, by hand or by the host's handling of secrets, and name its path";
    in
    lib.optionals (key != null) [
      {
        assertion = lib.hasPrefix "/" key || lib.hasPrefix "~/" key;
        message = "${option} is no path, /… or ~/…, and the file that names it is in the Nix store, which every user reads: ${put}";
      }
      {
        assertion = !lib.hasPrefix (builtins.storeDir + "/") key;
        message = "${option} is in the Nix store, which every user reads: ${put}";
      }
    ];

  # A token's text in the forge's credential command: GitHub's tokens
  # begin with one of these.
  prefixes = [
    "ghs_"
    "ghp_"
    "gho_"
    "ghu_"
    "ghr_"
    "github_pat_"
  ];
  credential = if cfg.settings.forge == null then null else cfg.settings.forge.credential;
  noToken = lib.optionals (credential != null) [
    {
      assertion = !lib.any (word: lib.any (p: lib.hasPrefix p word) prefixes) credential;
      message = "services.sluice.settings.forge.credential holds a GitHub token's text, and the file that names it is in the Nix store, which every user reads: name a command that prints the token";
    }
  ];
in
{
  options.services.sluice = {
    enable = lib.mkEnableOption "sluice's host settings, linked for the user who runs sluice, and its package";

    package = mkOption {
      type = types.nullOr types.package;
      default = flakes;
      defaultText = lib.literalExpression "sluice.packages.\${pkgs.stdenv.hostPlatform.system}.default";
      description = "The sluice the system profile holds: sluice's flake's for the host's system. `null` installs none, for a host that installs sluice its own way.";
    };

    settings = mkOption {
      type = table {
        runs = mkOption {
          type = table {
            max = unset types.int "The host's bound: how many runs share the host's cores at once, whatever repository or branch each is for. Where unset, a quarter of the cores sluice may use, never less than one (71).";
            cores = unset types.int "How many of the host's logical CPUs sluice may use. Where unset, what the host gives sluice: on Linux its affinity or its cgroup's quota, and every core where it sets neither (71).";
            reserved = unset types.int "How many runs of the bound are kept for submissions, which an agent's run in its worktree never takes. Where unset, 0 (71).";
            systems = unset (types.listOf types.str) "The systems this host runs, by the names of the policy's `[systems]`. Where unset, the one system the host is (87).";
          };
          default = { };
          description = "`[runs]`: how many runs share the host's cores at once, how many of its cores they use, and how many runs are kept for submissions (71).";
        };
        forge =
          unset
            (table {
              app_id = unset types.int "The GitHub App sluice pushes as.";
              key = unset types.str "The path of the app's private key on this host, absolute or under `~/`, outside the Nix store. sluice refuses a key file anyone but its owner may read.";
              credential = unset (types.listOf types.str) "In place of `key`, a command that gives the app's token: its program, then each argument, which sluice runs with no shell as git runs a credential helper. The program is a path, absolute or under `~/`, or a name the PATH finds. Never a token's text, since the file is in the Nix store, which every user reads (record 0108).";
              discovery = unset types.str "How often sluice asks GitHub for a branch's open pull requests while nothing on the host changes, a duration such as `\"60s\"` or `\"2m\"`. Where unset, `\"60s\"`; the least is `\"10s\"` (136).";
            })
            "`[forge]`: the GitHub App sluice pushes as, for every branch whose `remote` is on GitHub, and its private key or a command that gives its token. Where unset, sluice pushes with the host's own git credentials (136; record 0108).";
        signing =
          unset
            (table {
              signer = unset types.str "The signer this host signs its results as, one of the policy's `[signers]`.";
              key = unset types.str "The path of the signer's SSH private key on this host, absolute or under `~/`, outside the Nix store. sluice refuses a key file anyone but its owner may read.";
            })
            "`[signing]`: the signer this host signs its results as, and its key. Where unset, the host records no result on a branch whose state is on its remote (78).";
        peers = mkOption {
          type = table {
            lease = unset types.str "How long this host waits, having seen a branch's state stand unchanged, before it takes the branch from a host that has gone quiet. Where unset, `\"5m\"` (74).";
            alive = unset types.str "How often this host rewrites its sign of life, shorter than `lease`. Where unset, `\"2m\"` (74).";
            name = unset types.str "What this host is called among the hosts that share a remote. Where unset, the host's own name (74).";
          };
          default = { };
          description = "`[peers]`: how long a quiet host keeps a branch, how often this host shows it is alive, and its name among the hosts (74).";
        };
        deliveries = mkOption {
          type = table {
            expire_after = unset types.str "How long an outcome this host's rounds record is owed to its submitter's work before it shows as expired, still owed, a duration such as `\"168h\"` or `\"90m\"`, at least a second. Where unset, `\"168h\"` (51).";
          };
          default = { };
          description = "`[deliveries]`: how long an outcome is owed before it shows as expired (51).";
        };
        caches =
          unset
            (types.listOf (table {
              dir = unset types.str "The directory the cache keeps, outside every checkout: an absolute path, or one under `~/`.";
              socket = unset types.str "The socket of the cache's server, where it has one, written the same way.";
              env = unset (types.attrsOf types.str) "The variables the cache's clients read, each a string, which every run gets exactly as written.";
            }))
            "`[[caches]]`: the caches every run on the host is given, a compile server's or the task runner's shared cache. Where unset, none (35).";
      };
      default = { };
      description = "The host's own settings, `~/.config/sluice/config.toml`, each key named as the file names it, rendered at evaluation. A key left unset is left out of the file, so sluice's own default holds; a key the module does not declare is written as given, for sluice to judge. The file is in the Nix store, which every user reads: a key is named by its path on the host, never held here.";
    };

    text = mkOption {
      type = types.str;
      internal = true;
      readOnly = true;
      description = "The file's text, rendered from `settings` with what is unset taken out, which each platform's module writes.";
    };
  };

  config = lib.mkMerge [
    { services.sluice.text = render (prune cfg.settings); }
    (lib.mkIf cfg.enable { assertions = outside "forge" ++ outside "signing" ++ noToken; })
  ];
}
