# Tasks

Work in `fix/sluices-plugins-install-from-its-release--landing-96`, cut
from `bolt/sluices-plugins-install-from-its-release`, the base every
`sluice exec` below names. No task reads a Markdown file to check it
(core.21; principle 16), so a task that changes only a page says what
its writer keeps and is verified by the gate's passing alone.

Commit by pathspec, with Conventional Commits subjects scoped to the
part of the tree touched, citing record 0112 and rule core.56.

## 1. No task reads the install commands

- [x] 1.1 Remove `check-installs.sh`, and the root project's `installs` task with its comment from `moon.yml` (spec "Every install command names a release"). True when `sluice exec bolt/sluices-plugins-install-from-its-release` passes and lists no `root:installs`.
- [x] 1.2 core.56's details in `docs/architecture/core.md` say no task reads the pages for it, its writers keeping it (core.21; principle 16). Its rule line is unchanged, so `site:rules` passes with `rules.md` unchanged. True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run site:rules` passes.

## 2. The herdr plugin is installed from a release in setup

- [x] 2.1 `plugins/sluice/skills/setting-up-sluice/references/herdr.md` offers `herdr plugin install agentplot/sluice/plugins/herdr --ref v<version>`, `<version>` the version `sluice --version` prints (spec "The herdr plugin is installed only on the person's word"). No task reads the skill (principle 16), so the gate cannot check it: its writer keeps core.56, and the gate's passing is its verification.

## 3. The specs reconcile

- [x] 3.1 Archive this change with `openspec archive sluices-install-commands-are-kept-by-their-writers --yes`. True when `sluice exec bolt/sluices-plugins-install-from-its-release -- moon run root:specs root:changes` passes.
