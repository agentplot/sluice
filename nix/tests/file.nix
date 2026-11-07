# The host-settings-module spec's file (record 0098), over options.nix
# evaluated alone: each of the spec's settings renders the text of its
# file under nix/tests/files, byte for byte; a key the module does not
# declare is written as given; a value the file cannot hold fails,
# naming its option; and a key in the store, a key's text or a key given
# as a Nix path fails evaluation.
t:
let
  inherit (t) lib;
  settings = import ./settings.nix;
  pkgs = t.inputs.nixpkgs.legacyPackages.x86_64-linux;

  # options.nix enabled with `given`, beside a system's `assertions`.
  evaluated =
    given:
    (lib.evalModules {
      modules = [
        (import ../modules/options.nix { inherit (t.outputs) packages; })
        {
          options.assertions = lib.mkOption {
            type = lib.types.listOf lib.types.unspecified;
            default = [ ];
          };
        }
        {
          _module.args = { inherit pkgs; };
          services.sluice = {
            enable = true;
            settings = given;
          };
        }
      ];
    }).config;

  # The messages of the assertions `given` fails.
  failures = given: map (a: a.message) (lib.filter (a: !a.assertion) (evaluated given).assertions);
  says = given: words: lib.hasInfix words (toString (failures given));

  # The text the module writes for `given`.
  text = given: (evaluated given).services.sluice.text;

  # The text of `name`'s settings, held to its file.
  rendersAs =
    name:
    t.expect (text settings.${name} == builtins.readFile ./files/${name}.toml)
      "${name}'s settings render ${
        builtins.toJSON (text settings.${name})
      }, not nix/tests/files/${name}.toml"
    ++
      t.expect (failures settings.${name} == [ ])
        "${name}'s settings fail: ${toString (failures settings.${name})}";

  # A float, which no key of the file takes. Evaluation can catch a
  # failure but not read its words, so the words are read from the
  # renderer imported with its `throw` writing them into the text.
  float = {
    runs.share = 1.5;
  };
  cannot = builtins.tryEval (builtins.seq (text float) true);
  shown = builtins.scopedImport { throw = words: "<failure: ${words}>"; } ../modules/toml.nix {
    inherit lib;
  } (evaluated float).services.sluice.settings;

  # A key under the store directory, a key's text, and a key given as a
  # Nix path value: a file that is there, so that the option's type alone
  # can refuse it.
  inStore = table: {
    ${table}.key = "${builtins.storeDir}/0000000000000000000000000000000-app.pem";
  };
  keyText = {
    signing.key = "-----BEGIN OPENSSH PRIVATE KEY-----";
  };
  nixPath = builtins.tryEval (
    builtins.seq (evaluated { forge.key = ./settings.nix; }).services.sluice.settings.forge.key true
  );

  refused =
    given: option: words:
    t.expect (failures given != [ ]) "${option} evaluates"
    ++ lib.concatMap (
      w: t.expect (says given w) "the failure names no ${w}: ${toString (failures given)}"
    ) ([ option ] ++ words);
in
{
  "No settings" =
    rendersAs "empty"
    ++ t.expect (
      lib.length (lib.splitString "\n" (text settings.empty)) == 2
    ) "the empty settings render more than the header: ${builtins.toJSON (text settings.empty)}";

  "Every key" = rendersAs "every";

  "A key the module does not declare" =
    let
      given = {
        runs = {
          later = "soon";
          max = 1;
        };
      };
    in
    t.expect (lib.hasInfix ''
      [runs]
      later = "soon"
      max = 1
    '' (text given)) "the file's [runs] is not later, then max: ${builtins.toJSON (text given)}";

  "A value the file cannot hold" =
    t.expect (!cannot.success) "a file with settings.runs.share = 1.5 evaluates"
    ++ t.expect (lib.hasInfix "<failure: services.sluice.settings.runs.share is a float" shown) "the failure names no services.sluice.settings.runs.share: ${builtins.toJSON shown}";

  "A forge key in the store" = refused (inStore "forge") "services.sluice.settings.forge.key" [
    "Nix store, which every user reads"
  ];

  "A signing key in the store" = refused (inStore "signing") "services.sluice.settings.signing.key" [
    "Nix store, which every user reads"
  ];

  "A key's text" = refused keyText "services.sluice.settings.signing.key" [
    "no path"
    "Nix store, which every user reads"
  ];

  "A key given as a Nix path" = t.expect (
    !nixPath.success
  ) "services.sluice.settings.forge.key evaluates as the Nix path ./settings.nix";
}
