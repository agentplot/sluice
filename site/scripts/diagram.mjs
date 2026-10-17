// Writes the start page's copy of the animated diagram, docs/diagrams/sluice.py
// --scoped, into src/generated/. Run before every build and dev (package.json);
// the copy is not committed.
import { execFileSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const target = resolve(here, '../src/generated');
mkdirSync(target, { recursive: true });
execFileSync('python3', [resolve(here, '../../docs/diagrams/sluice.py'), '--scoped', resolve(target, 'sluice.svg')], {
  stdio: 'inherit',
});
