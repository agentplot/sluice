// Writes the site's rules page, src/content/docs/architecture/rules.md,
// from the rule list of docs/architecture/core.md, each rule's id linked to
// core.md's details. The page is committed; `--check` writes nothing and
// fails when the committed page is not what core.md makes (site:rules).
// Change a rule in core.md, then run `node site/scripts/rules.mjs`.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const source = resolve(here, "../../docs/architecture/core.md");
const target = resolve(here, "../src/content/docs/architecture/rules.md");
const blob = "https://github.com/agentplot/sluice/blob/main/docs/architecture/core.md";

const text = readFileSync(source, "utf8");
const start = text.indexOf("\n## Rules\n");
const end = text.indexOf("\n## Details\n");
if (start < 0 || end < start) throw new Error("core.md: no `## Rules` section before `## Details`");
const rules = text
  .slice(start + "\n## Rules\n".length, end)
  .trim()
  // Each group of rules is a section of the page.
  .replace(/^### /gm, "## ")
  .replace(/\*\*\[(core\.\d+)\]\*\*/g, (_, id) => `**[${id}](${blob}#details)**`);
if (!/\*\*\[core\.\d+\]\(/.test(rules)) throw new Error("core.md: no rule in `## Rules`");

const page = [
  "---",
  "# Written by site/scripts/rules.mjs from docs/architecture/core.md; site:rules",
  "# fails when the two differ. Change core.md, then run the script.",
  "title: Rules",
  "description: The rules that bind every change to the repository, cited by clause and decision.",
  `editUrl: https://github.com/agentplot/sluice/edit/main/docs/architecture/core.md`,
  "sidebar:",
  "  order: 2",
  "---",
  "",
  `The rules of [docs/architecture/core.md](${blob}), the binding copy,`,
  "as it lists them. A reviewer can point at a diff and say which rule it",
  "breaks. Each rule's id links to its details in core.md: what it rules",
  "out, what it allows and the clauses and decisions it rests on.",
  "",
  rules,
  "",
].join("\n");

if (process.argv.includes("--check")) {
  let committed = "";
  try {
    committed = readFileSync(target, "utf8");
  } catch {}
  if (committed !== page) {
    console.error(
      "site/src/content/docs/architecture/rules.md differs from what docs/architecture/core.md makes; run `node site/scripts/rules.mjs` and commit it.",
    );
    process.exit(1);
  }
  console.log("rules: the page matches docs/architecture/core.md");
} else {
  writeFileSync(target, page);
  console.log("rules: written from docs/architecture/core.md");
}
