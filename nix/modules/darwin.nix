# The nix-darwin module, darwinModules.default: services.sluice's file
# linked at ~/.config/sluice/config.toml for nix-darwin's primary user,
# the user who runs sluice, and sluice in the system profile (record 0098,
# core.51). sluice reads the file at its next command, so the module adds
# no agent, no daemon and nothing else for sluice to run (135, core.31).
{ packages }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.sluice;
  file = pkgs.writeText "sluice-config.toml" cfg.text;

  user = config.system.primaryUser;
  home = if user == null then null else config.users.users.${user}.home or null;
in
{
  imports = [ (import ./options.nix { inherit packages; }) ];

  config = lib.mkIf cfg.enable {
    system.requiresPrimaryUser = [ "services.sluice.enable" ];

    assertions = [
      {
        assertion = user == null || home != null;
        message = "services.sluice links ~/.config/sluice/config.toml for nix-darwin's primary user ${toString user}, whose home users.users.${toString user}.home is not set: set it, as the home the file is linked in.";
      }
    ];

    environment.systemPackages = lib.optional (cfg.package != null) cfg.package;

    # Activation runs as root, with root's HOME, so the link is made as the
    # primary user under their own home. A link that fails says why and
    # stops nothing else of the system's activation.
    system.activationScripts.postActivation.text = ''
      sudo --user=${lib.escapeShellArg user} -- /usr/bin/env HOME=${lib.escapeShellArg home} /bin/sh ${./activate.sh} link ${file} || true
    '';
  };
}
