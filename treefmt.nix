_: {
  settings.excludes = [
    "*.md"
    "*.lock"
    "*.jsonl"
    "docs/examples/**"
    "target/**"
    ".devenv/**"
    ".moon/cache/**"
    # Claude Code rewrites it in its own layout when a setting changes.
    ".claude/settings.json"
    # The engine's own declarations, copied whole from a session of the
    # pinned Claude Code (plugins/sluice-board/README.md).
    "plugins/sluice-board/engine-types/**"
    # The herdr plugin's fixture documents and frames, compared byte for
    # byte.
    "plugins/herdr/tests/docs/**"
  ];

  programs.rustfmt.enable = true;
  programs.rustfmt.edition = "2021";

  programs.deadnix.enable = true;
  programs.nixfmt.enable = true;
  settings.formatter.deadnix.pipeline = "nix";
  settings.formatter.deadnix.priority = 1;
  settings.formatter.nixfmt.pipeline = "nix";
  settings.formatter.nixfmt.priority = 2;

  programs.shellcheck.enable = true;
  programs.shfmt.enable = true;
  programs.shfmt.indent_size = 2;
  settings.formatter.shellcheck.pipeline = "shell";
  settings.formatter.shellcheck.priority = 1;
  settings.formatter.shfmt.pipeline = "shell";
  settings.formatter.shfmt.priority = 2;

  programs.taplo.enable = true;
  programs.yamlfmt.enable = true;
  programs.yamlfmt.settings.formatter = {
    type = "basic";
    retain_line_breaks_single = true;
  };

  # Biome formats and lints the TypeScript, JavaScript, CSS and JSON in
  # one pass (`biome check`), its recommended rules and nothing more; a
  # lint error fails `treefmt --ci`. Its settings are here, not in a
  # biome.json, since treefmt-nix hands Biome a generated one.
  programs.biome.enable = true;
  programs.biome.formatCommand = "check";
  programs.biome.settings = {
    formatter = {
      indentStyle = "space";
      indentWidth = 2;
      lineWidth = 100;
    };
    javascript.formatter = {
      quoteStyle = "double";
      trailingCommas = "all";
      semicolons = "always";
    };
    json.formatter.trailingCommas = "none";
    linter.rules.recommended = true;
  };
  # Biome formats only an .astro file's frontmatter, and only
  # experimentally.
  settings.formatter.biome.excludes = [ "site/**/*.astro" ];
}
