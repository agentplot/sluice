// Copies docs/adr/*.md into src/content/docs/decisions/, turning each
// record's frontmatter (number, title, status, date) into Starlight's and
// dropping the H1 the record repeats. Run before every build and dev
// (package.json); the generated pages are not committed.
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const source = resolve(here, '../../docs/adr');
const target = resolve(here, '../src/content/docs/decisions');

rmSync(target, { recursive: true, force: true });
mkdirSync(target, { recursive: true });

const files = readdirSync(source).filter((f) => /^\d{4}-.*\.md$/.test(f)).sort();
for (const file of files) {
  const text = readFileSync(join(source, file), 'utf8');
  const match = text.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
  if (!match) throw new Error(`${file}: no frontmatter`);
  const [, head, rest] = match;
  const field = (name) => (head.match(new RegExp(`^${name}:\\s*(.+)$`, 'm')) ?? [])[1]?.trim();
  const number = String(field('number')).padStart(4, '0');
  const title = field('title');
  const status = field('status');
  const date = field('date');
  const body = rest.replace(/^\s*# .*\n/, '');
  const links = body.replace(/\]\((\d{4})-[^)]*\.md\)/g, (_, n) => `](/sluice/decisions/${n}/)`);
  const page = [
    '---',
    `title: ${JSON.stringify(`ADR ${number}: ${title}`)}`,
    `description: ${JSON.stringify(`${status}, ${date}`)}`,
    `sidebar:`,
    `  label: ${JSON.stringify(`${number} ${title}`)}`,
    `  order: ${Number(number)}`,
    '---',
    '',
    `**Status:** ${status} · **Date:** ${date}`,
    '',
    links.trim(),
    '',
  ].join('\n');
  writeFileSync(join(target, `${number}.md`), page);
}
console.log(`decisions: ${files.length} records from docs/adr`);
