# The documentation site

The developer documentation at https://agentplot.github.io/sluice, an
[Astro](https://astro.build) site with [Starlight](https://starlight.astro.build),
published by `.github/workflows/pages.yml` on every push to `main` that
touches `site/`, `docs/` or the README.

```sh
cd site
bun install            # the pinned dependencies, from bun.lock
bun run dev            # http://localhost:4321/sluice/
bun run build          # dist/, with every internal link checked
```

## Where the pages live

`src/content/docs/`, one Markdown file per page, with the sidebar built
from the directories in `astro.config.mjs`:

| directory | pages |
|---|---|
| `index.mdx` | the start page: why, install, quick start |
| `guides/` | the walkthrough |
| `reference/` | configuration, commands, state, telemetry |
| `architecture/` | how sluice is built; the rules, mirrored from `docs/architecture/core.md` |
| `decisions/` | **generated**: one page per record in `docs/adr/`, by `scripts/adrs.mjs` before every build and dev; not committed |
| `many-hosts/` | the multi-host design, marked as under review and not built |
| `roadmap.md` | what is built and what is designed |

The brand marks are copied from `docs/brand/` into `src/assets/` and
`public/favicon.svg`.

## Conventions

The pages use the words a user sees: branch, submission, checkout,
combination, blamed, result, test, parallel, batch, state. The rules
page keeps the statement's design words and says how they map. Nothing
commercial appears here (rule core.13).
