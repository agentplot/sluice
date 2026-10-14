// The developer documentation, built by Astro with Starlight and published
// to GitHub Pages at https://agentplot.github.io/sluice by
// .github/workflows/pages.yml. The decisions section is generated from
// docs/adr by scripts/adrs.mjs before every build.
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightLinksValidator from 'starlight-links-validator';
import rehypeMermaid from 'rehype-mermaid';

// ```mermaid blocks are drawn as SVG at build time, a light and a dark one
// in a <picture>, so a page needs no script to show them. Drawing needs a
// Chromium: Playwright's (PLAYWRIGHT_BROWSERS_PATH in the devenv shell).
// The dark one's arrows and labels are near white on the dark page.
const dark = {
  theme: 'dark',
  themeVariables: {
    lineColor: '#d8dce4',
    textColor: '#f2f3f5',
    signalColor: '#d8dce4',
    signalTextColor: '#f2f3f5',
    edgeLabelBackground: '#2a2f38',
    titleColor: '#f2f3f5',
  },
};

export default defineConfig({
  site: 'https://agentplot.github.io',
  base: '/sluice',
  markdown: {
    rehypePlugins: [
      [rehypeMermaid, { strategy: 'img-svg', dark, launchOptions: { channel: 'chromium' } }],
    ],
  },
  integrations: [
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
