# sluice's herdr plugin: plugins/herdr of the source of the `sluice` it is
# given, copied whole at that package's version, so the plugin and the CLI
# are one revision (record 0112 as amended 2026-11-12; core.56). It reads
# its source, version, name and homepage from that package alone, so a
# plugin overridden with another `sluice` follows it. Nothing in it is
# rewritten, its scripts' interpreters among them: herdr runs them by sh,
# and they find sluice on the user's PATH. It is the folder `herdr plugin
# install agentplot/sluice/plugins/herdr --ref v<version>` gives, and a
# host links it with `herdr plugin link <path>`. agentplot/boomdrive's
# nix/packages/herdr-plugin.nix, with its `tool` named `sluice`.
{
  stdenvNoCC,
  sluice,
}:
stdenvNoCC.mkDerivation {
  pname = "${sluice.pname}-herdr-plugin";
  inherit (sluice) version;
  src = "${sluice.src}/plugins/herdr";
  dontConfigure = true;
  dontBuild = true;
  installPhase = ''
    runHook preInstall
    cp -R . "$out"
    runHook postInstall
  '';
  dontFixup = true;
  meta = {
    description = "sluice's herdr plugin, at sluice ${sluice.version}";
    inherit (sluice.meta) homepage;
  };
}
