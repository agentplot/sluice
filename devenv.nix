{
  pkgs,
  lib,
  config,
  ...
}:

let
  # moonrepo's own release builds: nixpkgs trails moon by a minor version or
  # more, and sluice runs its own merge queue on the newest moon. Linux takes
  # the static musl build, so no loader needs patching.
  fromRelease =
    {
      name,
      repo,
      version,
      bins,
      hashes,
    }:
    let
      system = pkgs.stdenv.hostPlatform.system;
      target =
        {
          aarch64-darwin = "aarch64-apple-darwin";
          x86_64-darwin = "x86_64-apple-darwin";
          aarch64-linux = "aarch64-unknown-linux-musl";
          x86_64-linux = "x86_64-unknown-linux-musl";
        }
        .${system} or (throw "${name} ${version}: no release build for ${system}");
    in
    pkgs.stdenvNoCC.mkDerivation {
      pname = name;
      inherit version;
      src = pkgs.fetchurl {
        url = "https://github.com/moonrepo/${repo}/releases/download/v${version}/${name}-${target}.tar.xz";
        sha256 = hashes.${target};
      };
      sourceRoot = "${name}-${target}";
      installPhase = ''
        runHook preInstall
        install -Dm755 -t $out/bin ${pkgs.lib.concatStringsSep " " bins}
        runHook postInstall
      '';
    };

  moon = fromRelease {
    name = "moon_cli";
    repo = "moon";
    version = "2.6.1";
    bins = [
      "moon"
      "moonx"
    ];
    hashes = {
      aarch64-apple-darwin = "2922ce70277cc6f3750106fa9ebd1e25ed890ec8cdba4570148ff8701a5bea53";
      x86_64-apple-darwin = "ef715c5fa43985de841049c319c52e9307a233a8634c08b6729e7b224592dc8e";
      aarch64-unknown-linux-musl = "b1fb67933231c9c565ea4375b54f0f9645112d76ede767a6040c5142bf7c0310";
      x86_64-unknown-linux-musl = "200314608ee89d1deecc7cd0bbd523d4dffa64ffbb365989f003f3a8d91c9080";
    };
  };

  proto = fromRelease {
    name = "proto_cli";
    repo = "proto";
    version = "0.63.1";
    bins = [
      "proto"
      "proto-shim"
    ];
    hashes = {
      aarch64-apple-darwin = "8bfde4c862381eeb6e82d0858bb6215732cbd1e384e4eae45da8fa08095e0ffd";
      x86_64-apple-darwin = "aaaff78c7125de1b765f00177f35da75ee16fb1363e07632ace27213ff5d893b";
      aarch64-unknown-linux-musl = "111f5b6a38d3979ebafbc095c8d91ef69c99d1f018addb3582179d4ab91b3441";
      x86_64-unknown-linux-musl = "8596b98ccd833d1203e71931ba25e866d3239f9d3859c922a035a7b830f29d17";
    };
  };

  # The application toolchain .prototools pins, on PATH: what the shell
  # activates, and what a git hook, which runs outside any shell a session
  # opened, activates for itself. proto's home is this repository's, in the
  # git common dir so every worktree shares it: moon puts $PROTO_HOME/bin on
  # every task's PATH, and the user's own proto installs (an older moon
  # among them) must never shadow the system tools above. Only this
  # repository's pins are read (no global .prototools).
  activate = ''
    PROTO_HOME="$(${lib.getExe pkgs.git} rev-parse --path-format=absolute --git-common-dir)/proto"
    export PROTO_HOME
    eval "$(${proto}/bin/proto activate bash --export --no-bin --config-mode upwards 2>/dev/null)"
  '';

  # The lint gate.
  lint = pkgs.writeShellApplication {
    name = "lint";
    runtimeInputs = [ pkgs.git ];
    text = ''
      cd "$(git rev-parse --show-toplevel)"
      ${activate}
      cargo clippy --workspace --all-targets -- -D warnings
    '';
  };

  # treefmt's rustfmt: the pinned toolchain's.
  rustfmt = pkgs.writeShellApplication {
    name = "rustfmt";
    text = ''
      ${activate}
      exec rustfmt "$@"
    '';
  };

  # target/ grows unseen across worktrees until the disk is full.
  disk = pkgs.writeShellApplication {
    name = "disk";
    runtimeInputs = [
      pkgs.git
      pkgs.coreutils
      pkgs.gawk
    ];
    text = ''
      root="$(git rev-parse --show-toplevel)"
      limit=''${SLUICE_TARGET_LIMIT_GB:-25}
      target=0
      [ -d "$root/target" ] && target=$(($(du -sk "$root/target" | cut -f1) / 1048576))
      used="$(df -P "$root" | awk 'NR == 2 { print $5 }' | tr -d '%')"
      echo "target/ ''${target} GB, the disk ''${used} per cent full"
      status=0
      if [ "$target" -gt "$limit" ]; then
        echo "target/ is ''${target} GB, over the ''${limit} GB bar: run 'cargo clean' here and in the worktrees beside this one" >&2
        status=1
      fi
      if [ "$used" -ge 90 ]; then
        echo "the disk is ''${used} per cent full: a build on a full disk is slow and its processes are killed" >&2
        status=1
      fi
      exit $status
    '';
  };

in
{
  # The system tools are this file's, pinned by devenv.lock; the application
  # toolchain (Rust) is .prototools', installed by `moon setup` or `proto
  # install` and activated here. Every cargo and moon command in this
  # repository runs inside `devenv shell`.
  packages = [
    pkgs.git
    pkgs.jq
    moon
    proto
  ];

  # moon's task archives (.moon/cache/outputs) are a link into the git common
  # dir, so every worktree of sluice shares them: a crate's tests that passed
  # in one worktree replay in another. The rest of .moon/cache records the
  # checkout's absolute paths and stays its own.
  enterShell = ''
    if _moon_common="$(git -C "${config.devenv.root}" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)"; then
      _moon_cache="${config.devenv.root}/.moon/cache"
      [ -L "$_moon_cache" ] && rm -f "$_moon_cache"
      mkdir -p "$_moon_common/moon-outputs" "$_moon_cache"
      if [ ! -L "$_moon_cache/outputs" ]; then
        rm -rf "$_moon_cache/outputs"
        ln -s "$_moon_common/moon-outputs" "$_moon_cache/outputs"
      fi
    fi
    unset _moon_common _moon_cache
    # moon puts $MOON_HOME/bin on every task's PATH too: this repository's,
    # beside proto's, so no moon of the user's runs in a task.
    MOON_HOME="$(git -C "${config.devenv.root}" rev-parse --path-format=absolute --git-common-dir)/moon-home"
    export MOON_HOME
    ${activate}
  '';

  treefmt.enable = true;
  treefmt.config = {
    imports = [ ./treefmt.nix ];
    programs.rustfmt.package = rustfmt;
  };

  # `devenv shell -- cargo-tests`: every crate's cargo tests and the
  # conformance citations, through moon, replaying a crate whose inputs
  # already passed.
  scripts.cargo-tests = {
    description = "Every crate's cargo tests: moon run of the tasks tagged cargo-test";
    exec = ''
      set -euo pipefail
      cd ${config.devenv.root}
      moon run ':#cargo-test' "$@" </dev/null
    '';
  };

  # `devenv shell -- cargo-tests-affected`: only the crates a change
  # touches, and those depending on them.
  scripts.cargo-tests-affected = {
    description = "The cargo tests of what changed since main: moon run --affected";
    exec = ''
      set -euo pipefail
      cd ${config.devenv.root}
      moon run ':#cargo-test' --affected --downstream deep "$@" </dev/null
    '';
  };

  scripts.lint = {
    description = "The lint gate: cargo clippy -D warnings";
    exec = ''exec ${lint}/bin/lint "$@"'';
  };
  scripts.disk = {
    description = "What target/ and the disk have grown to";
    exec = ''exec ${disk}/bin/disk "$@"'';
  };

  # The floor under every commit, installed into the common hooks directory
  # on `devenv shell`.
  git-hooks.hooks = {
    treefmt.enable = true;
    clippy-gate = {
      enable = true;
      name = "clippy (-D warnings)";
      entry = "${lint}/bin/lint";
      files = "\\.rs$|Cargo\\.(toml|lock)$";
      pass_filenames = false;
    };
  };

  # devenv 2.4 installs the hook with `prek install -c
  # "$DEVENV_ROOT/.pre-commit-config.yaml"`, and prek writes that absolute
  # path into the hook, which lives in the hooks directory every worktree
  # shares: each worktree's shell would point every commit at its own
  # config. A plain `prek install` writes no path, and prek reads the
  # config at the top of the worktree the commit is in, which devenv links
  # there. Remove this once devenv installs without the absolute path
  # (cachix/devenv#3270).
  tasks."devenv:git-hooks:install".exec = lib.mkForce ''
    set -euo pipefail
    hook="$(${lib.getExe pkgs.git} rev-parse --path-format=absolute --git-path hooks)/pre-commit"
    if ! grep -q 'File generated by prek' "$hook" 2>/dev/null || grep -q -- '--config=' "$hook"; then
      ${lib.getExe config.git-hooks.package} install --force -t pre-commit
    fi
  '';
}
