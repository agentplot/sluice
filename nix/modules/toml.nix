# sluice's host settings, rendered from services.sluice.settings at
# evaluation (record 0098): so the gate compares the text the module writes
# without building it, and one `settings` gives one text on either
# platform. Top-level keys come before tables, each sorted by name. A key
# whose value is null is unset, and left out, so sluice's own default
# holds. Any value but a string, a boolean, an integer, a list of strings,
# a table or a list of tables fails, naming its path. boomdrivedev/boomdrive's
# nix/modules/toml.nix at 3f428cf, its header and its failure naming
# sluice's module.
{ lib }:
let
  inherit (builtins)
    attrNames
    filter
    isAttrs
    isBool
    isInt
    isList
    isString
    typeOf
    ;
  inherit (lib)
    concatLists
    concatMap
    concatMapStringsSep
    concatStringsSep
    filterAttrs
    imap0
    optionals
    ;

  header = "# Written by sluice's flake module from services.sluice.settings; a change goes there.";

  # TOML's basic string: `"`, `\` and every control character escaped.
  hex = n: lib.fixedWidthString 4 "0" (lib.toHexString n);
  controls = lib.range 1 31 ++ [ 127 ];
  short = {
    "8" = "\\b";
    "9" = "\\t";
    "10" = "\\n";
    "12" = "\\f";
    "13" = "\\r";
  };
  string =
    text:
    ''"${
      builtins.replaceStrings
        (
          [
            "\\"
            ''"''
          ]
          ++ map (n: builtins.fromJSON ''"\u${hex n}"'') controls
        )
        (
          [
            "\\\\"
            ''\"''
          ]
          ++ map (n: short.${toString n} or "\\u${hex n}") controls
        )
        text
    }"'';

  # A key as TOML writes it: bare where it can be, else quoted.
  key = name: if builtins.match "[A-Za-z0-9_-]+" name != null then name else string name;
  headerOf = concatMapStringsSep "." key;

  refuse =
    named: value:
    throw "services.sluice.settings.${named} is ${
      if isAttrs value then
        "a table"
      else if value == null then
        "null"
      else
        "a ${typeOf value}"
    }: sluice's file takes strings, booleans, integers, lists of strings, tables and lists of tables";

  # A value written on its key's line.
  inline =
    named: value:
    if isString value then
      string value
    else if isBool value then
      lib.boolToString value
    else if isInt value then
      toString value
    else if isList value then
      "[${
        concatStringsSep ", " (
          imap0 (i: v: if isString v then string v else refuse "${named}[${toString i}]" v) value
        )
      }]"
    else
      refuse named value;

  isTables = value: isList value && value != [ ] && builtins.all isAttrs value;

  # A table's lines: its keys', then its tables', each sorted by name.
  # `path` is its keys from the top, `named` the same as a failure names
  # it, an element of a list of tables by its index.
  table =
    path: named: attrs:
    let
      set = filterAttrs (_: value: value != null) attrs;
      at = name: if named == "" then name else "${named}.${name}";
      nested = name: isAttrs set.${name} || isTables set.${name};
      sectionOf =
        name:
        let
          value = set.${name};
        in
        if isAttrs value then
          section (path ++ [ name ]) (at name) value
        else
          concatLists (imap0 (i: element (path ++ [ name ]) "${at name}[${toString i}]") value);
    in
    {
      keys = map (name: "${key name} = ${inline (at name) set.${name}}") (
        filter (name: !nested name) (attrNames set)
      );
      sections = concatMap sectionOf (filter nested (attrNames set));
    };

  # `[a.b]`, written where it holds a key or nothing else, so a table
  # holding only tables is named by theirs.
  section =
    path: named: attrs:
    let
      inner = table path named attrs;
    in
    optionals (inner.keys != [ ] || inner.sections == [ ]) (
      [
        ""
        "[${headerOf path}]"
      ]
      ++ inner.keys
    )
    ++ inner.sections;

  # `[[a.b]]`, one for each table of the list.
  element =
    path: named: attrs:
    let
      inner = table path named attrs;
    in
    [
      ""
      "[[${headerOf path}]]"
    ]
    ++ inner.keys
    ++ inner.sections;
in
settings:
let
  top = table [ ] "" settings;
in
concatStringsSep "\n" ([ header ] ++ top.keys ++ top.sections) + "\n"
