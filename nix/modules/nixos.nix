# The NixOS module, nixosModules.default: services.sluice's file linked at
# ~/.config/sluice/config.toml for services.sluice.user, the user who runs
# sluice, in that user's own activation, which runs at their manager's
# start and at a switch, and sluice in the system profile (record 0098,
# core.51). sluice reads the file at its next command, so the module adds
# no unit, no lingering and nothing else for sluice to run (135,
# core.31).
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

  inherit (cfg) user;
  account =
    config.users.users.${user} or {
      isNormalUser = false;
      isSystemUser = false;
    };
in
{
  imports = [ (import ./options.nix { inherit packages; }) ];

  options.services.sluice.user = lib.mkOption {
    type = lib.types.str;
    example = "alice";
    description = "The user who runs sluice, whose `~/.config/sluice/config.toml` is linked to the module's file in their own activation.";
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = account.isNormalUser || account.isSystemUser;
        message = "services.sluice.user is ${user}, whom users.users.${user} does not declare: declare the user who runs sluice, in whose home the file is linked.";
      }
    ];

    environment.systemPackages = lib.optional (cfg.package != null) cfg.package;

    # Run by every user's activation; the user who runs sluice alone links.
    # A link that fails says why and stops nothing else of the activation.
    system.userActivationScripts.sluice.text = ''
      if [ "$(id -un)" = ${lib.escapeShellArg user} ]; then ${pkgs.runtimeShell} ${./activate.sh} link ${file} || true; fi
    '';
  };
}
