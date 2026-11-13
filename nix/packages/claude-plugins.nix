# sluice's Claude Code marketplace: .claude-plugin/marketplace.json of the
# source of the `sluice` it is given, and each plugin it names, copied whole
# to the path it names, at that package's version, so the plugins and the
# CLI are one revision (record 0112 as amended 2026-11-12; core.56). It
# reads its source, version, name and homepage from that package alone, so
# a marketplace overridden with another `sluice` follows it. The plugins
# are those the marketplace file lists as `jq` reads it at build time, so a
# plugin added to it is carried with no change here. Nothing is compiled or
# rewritten: Claude Code runs the board's TypeScript as it is. A tree with
# no links copied by `cp -R` is one store folder of regular files, which a
# host adds with `claude plugin marketplace add <path>` as the marketplace
# `sluice`, the one the `release` branch gives.
{
  stdenvNoCC,
  jq,
  sluice,
}:
stdenvNoCC.mkDerivation {
  pname = "${sluice.pname}-claude-plugins";
  inherit (sluice) version src;
  nativeBuildInputs = [ jq ];
  dontConfigure = true;
  dontBuild = true;
  installPhase = ''
    runHook preInstall
    mkdir -p "$out/.claude-plugin"
    cp .claude-plugin/marketplace.json "$out/.claude-plugin/"
    jq -r '.plugins[].source' .claude-plugin/marketplace.json | while IFS= read -r source; do
      mkdir -p "$out/$(dirname "$source")"
      cp -R "$source" "$out/$source"
    done
    runHook postInstall
  '';
  dontFixup = true;
  meta = {
    description = "sluice's Claude Code marketplace, at sluice ${sluice.version}";
    inherit (sluice.meta) homepage;
  };
}
