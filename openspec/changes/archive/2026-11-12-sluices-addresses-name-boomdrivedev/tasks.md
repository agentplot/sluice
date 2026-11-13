# Tasks

Every command runs from the place's root, inside `devenv shell`.

## 1. The addresses of the moved tools name boomdrivedev

- [x] 1.1 The README, CLAUDE.md, `openspec/config.yaml`, the rules in `docs/architecture/core.md` and the site's rules page, the crates' comments, `cliff.toml`, the flake's header, homepage and the modules' keys, the module tests, the pages workflow, the site's configuration and pages, the plugins' manifests, READMEs and skills, the vale styles' links and the open OpenSpec changes name boomdrivedev/sluice, https://boomdrivedev.github.io/sluice and the other moved tools at boomdrivedev (record 0120). Verify that `git grep -n -P 'agentplot(/|\.github\.io/)(sluice|blueprints|boomdrive|boom|crew|flume|site-theme|strategy|timber|weir)(?![\w-])' -- ':!openspec/changes/archive' ':!openspec/specs' ':!plugins/sluice-board/tests' ':!docs/adr/0080-*' ':!docs/adr/0120-*' ':!openspec/changes/sluices-addresses-name-boomdrivedev'` prints only the run and set-aside kinds, a command's printed words, the paths of a host's folders and `@agentplot/site-theme`.
- [x] 1.2 The weir crates' git source in `Cargo.toml` and `Cargo.lock` names boomdrivedev/weir at the same revision. Verify with `cargo metadata --locked --format-version 1`.
- [x] 1.3 The site theme's source in `site/package.json` and `site/bun.lock` names boomdrivedev/site-theme at v0.1.3. Verify with `bun install --frozen-lockfile` in `site/`.
- [x] 1.4 The social preview and the site's card show github.com/boomdrivedev/sluice, rendered from `docs/brand/social-preview.html` as its header says.
- [x] 1.5 The site's rules page is what core.md makes. Verify with `moon run site:rules`.
- [x] 1.6 Record 0120 records which addresses move and which names stay. Verify with `moon run root:adrs`.

## 2. The specs

- [x] 2.1 The delta specs move each requirement's citations, the modules' key and the herdr plugin's install address. Verify with `openspec validate sluices-addresses-name-boomdrivedev --strict`, then archive with `openspec archive sluices-addresses-name-boomdrivedev --yes`.
- [x] 2.2 The gate holds. Verify with `sluice exec main`.
