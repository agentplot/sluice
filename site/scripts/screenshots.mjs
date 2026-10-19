// Writes WebP copies of the plugins' screenshots, plugins/*/screenshots/*.png,
// into public/screenshots/, where the guides and the start page show them. Run
// before every build and dev (package.json); the copies are not committed.
import { mkdirSync, readdirSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const here = dirname(fileURLToPath(import.meta.url));
const plugins = resolve(here, "../../plugins");
const target = resolve(here, "../public/screenshots");
mkdirSync(target, { recursive: true });
for (const plugin of readdirSync(plugins, { withFileTypes: true })) {
  if (!plugin.isDirectory()) continue;
  const dir = resolve(plugins, plugin.name, "screenshots");
  let files;
  try {
    files = readdirSync(dir).filter((f) => f.endsWith(".png"));
  } catch {
    continue;
  }
  for (const f of files) {
    // Lossless: the frames are text, which lossy WebP smears.
    await sharp(resolve(dir, f))
      .webp({ lossless: true, effort: 6 })
      .toFile(resolve(target, `${basename(f, ".png")}.webp`));
  }
}
