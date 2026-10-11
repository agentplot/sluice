_: {
  settings.excludes = [
    "*.md"
    "*.lock"
    "*.json"
    "*.jsonl"
    "docs/examples/**"
    "target/**"
    ".devenv/**"
    ".moon/cache/**"
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
}
