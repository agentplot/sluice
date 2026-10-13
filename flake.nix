# sluice as a nix package, for `nix profile install github:agentplot/sluice`
# and for the GHA build. The development shell is devenv's (devenv.nix);
# this flake only builds and installs. Rust is the version .prototools
# names, so the release is built by the compiler the checks ran with. The
# version is the release tag's: CI builds with `SLUICE_VERSION=<version>
# nix build --impure`; any other build is 0.0.0, Cargo.toml's.
{
  description = "sluice: a merge queue for agents in worktrees";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-darwin"
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
    in
    {
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
            cargoLock.lockFile = ./Cargo.lock;
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
    };
}
