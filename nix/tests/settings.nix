# The host-settings-module spec's settings, each rendered to its file
# under nix/tests/files, by names no reader is told to adopt.
{
  # "No settings": nothing set.
  empty = { };

  # "Every key": every key the module declares set.
  every = {
    runs = {
      max = 3;
      cores = 12;
      reserved = 1;
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
      ];
    };
    forge = {
      app_id = 123456;
      key = "~/.config/sluice/app.pem";
      discovery = "60s";
    };
    signing = {
      signer = "host-a";
      key = "~/.config/sluice/signing";
    };
    peers = {
      lease = "5m";
      alive = "2m";
      name = "host-a";
    };
    caches = [
      {
        dir = "~/.cache/sccache";
        socket = "~/.cache/sccache/server.sock";
        env = {
          RUSTC_WRAPPER = "sccache";
          SCCACHE_DIR = "~/.cache/sccache";
        };
      }
      {
        dir = "~/.moon/cache/shared";
        env.MOON_CACHE_SHARED_WORKTREE_CACHE = "true";
      }
    ];
  };
}
