// The developer documentation, built by Astro with Starlight and published
// to GitHub Pages at https://agentplot.github.io/sluice by
// .github/workflows/pages.yml. The decisions section is generated from
// docs/adr by scripts/adrs.mjs before every build.
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightLinksValidator from 'starlight-links-validator';
import mermaid from 'astro-mermaid';

export default defineConfig({
  site: 'https://agentplot.github.io',
  base: '/sluice',
  integrations: [
    // Before Starlight, so ```mermaid blocks become diagrams in the browser.
    mermaid({ theme: 'default', autoTheme: true }),
    starlight({
      title: 'sluice',
      description: 'A merge queue for agents working in parallel worktrees.',
      logo: {
        light: './src/assets/mark-light.svg',
        dark: './src/assets/mark-dark.svg',
        alt: 'sluice',
      },
      favicon: '/favicon.svg',
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/agentplot/sluice' },
      ],
      editLink: {
        baseUrl: 'https://github.com/agentplot/sluice/edit/main/site/',
      },
      customCss: ['./src/styles/mermaid.css'],
      plugins: [starlightLinksValidator()],
      sidebar: [
        { label: 'Start', link: '/' },
        { label: 'Guides', items: [{ autogenerate: { directory: 'guides' } }] },
        { label: 'Reference', items: [{ autogenerate: { directory: 'reference' } }] },
        { label: 'Architecture', items: [{ autogenerate: { directory: 'architecture' } }] },
        { label: 'Decisions', collapsed: true, items: [{ autogenerate: { directory: 'decisions' } }] },
        { label: 'Many hosts (designed, not built)', items: [{ autogenerate: { directory: 'many-hosts' } }] },
        { label: 'Roadmap', link: '/roadmap/' },
      ],
    }),
  ],
});
