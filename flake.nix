# sluice as a nix package, for `nix profile install github:agentplot/sluice`
# and for the GHA build, and services.sluice, the nix-darwin and NixOS
# modules that write the host's own settings and install the package
# (record 0098, core.51). The development shell is devenv's (devenv.nix);
# everything else of the flake's is under nix/, the moon project `nix`.
# Rust is the version .prototools names, so the release is built by the
# compiler the checks ran with. The version is the release tag's: CI
# builds with `SLUICE_VERSION=<version> nix build --impure`; any other
# build is 0.0.0, Cargo.toml's.
{
  description = "sluice: a merge queue for agents in worktrees";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # The tests' nix-darwin; a host's configuration brings its own.
    nix-darwin = {
      url = "github:nix-darwin/nix-darwin";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      # nixpkgs refuses x86_64-darwin since 26.11, and no release builds
      # it (record 0098).
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      forAll =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );
      rustVersion = (builtins.fromTOML (builtins.readFile ./.prototools)).rust;
      workspace = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package;
      released = builtins.getEnv "SLUICE_VERSION";
      version = if released != "" then released else workspace.version;
      packages = forAll (
        pkgs:
        let
          rust = pkgs.rust-bin.stable.${rustVersion}.minimal;
          platform = pkgs.makeRustPlatform {
            cargo = rust;
            rustc = rust;
          };
        in
        {
          # `sluice` and `wt-submit`, which worktrunk runs as `wt submit`.
          default = platform.buildRustPackage {
            pname = "sluice";
            inherit version;
            env.SLUICE_VERSION = version;
            src = self;
            cargoLock = {
              lockFile = ./Cargo.lock;
              # agentplot/weir at the revision Cargo.toml names, which
              # moves with it (record 0102).
              outputHashes."weir-delivery-0.0.0" = "sha256-29h7paqrZ2AONJ/Xs2KBTF+9qYJWJMoJBZDE+OIxqoc=";
            };
            cargoBuildFlags = [
              "-p"
              "sluice"
            ];
            # The tests need git, moon and worktrunk; CI runs them in the
            # devenv shell, by tier.
            doCheck = false;
            meta = {
              description = "A merge queue for agents in worktrees";
              homepage = "https://github.com/agentplot/sluice";
              license = pkgs.lib.licenses.mit;
              mainProgram = "sluice";
            };
          };
        }
      );

      # A module of a platform, keyed as Flume's flake keys its own, so a
      # configuration that imports it twice, directly and through
      # Boomdrive's flake, declares services.sluice once (record 0098).
      moduleOf = platform: file: {
        _file = file;
        key = "github:agentplot/sluice#${platform}Modules.sluice";
        imports = [ (import file { inherit packages; }) ];
      };
    in
    {
      inherit packages;

      # services.sluice, the host's own settings and the package (record
      # 0098, core.51).
      darwinModules = rec {
        sluice = moduleOf "darwin" ./nix/modules/darwin.nix;
        default = sluice;
      };
      nixosModules = rec {
        sluice = moduleOf "nixos" ./nix/modules/nixos.nix;
        default = sluice;
      };
    };
}
