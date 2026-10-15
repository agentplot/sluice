# Tasks

Read first: this change's proposal, specs and design;
`agentplot/blueprints:design/sluice/proposals/previews-1.md`;
`docs/architecture/core.md`; ADRs 0010, 0016, 0018, 0020. Work inside
`devenv shell`. Fast tier: `moon run ':#check-fast'`; real tools:
`moon run ':#check-tools'`; e2e: `moon run ':#check-e2e'`. Commit by
pathspec after each task group, Conventional Commits, scope the crate or
`site`/`adr`/`docs`. Site pages are checked with `moon run 'site:prose'`
(Vale, every warning cleared) and built with `bun run build` in `site/`.

## 1. Policy and the preview record

- [ ] 1.1 Parse `[preview]` (`command` argv, `addresses` map, optional `timeout`) in the core's policy, refusing an address that names a port `[checkout] ports` does not list or a placeholder other than `{name.<port>}`/`{port.<port>}`, with the address and port named; verify with policy tests for a full table, no table, and each refusal's text.
- [ ] 1.2 Add `Kind::Preview` records under `<common>/sluice/.previews/<rev12>.json` in `profiles/places.rs` (branch, revision, path, block, ports, names, addresses, made, run dir, saved release request, failed output); verify in `crates/sluice/tests/profiles/places.rs` that a record round-trips and that `session_places` and every existing reader of `.places/` never see it.
- [ ] 1.3 Build names (D3): `<port>-<rev7>--<branch slug>--<repo slug>`, cut to 63 with `-<hash6>`, the repository slug from the common directory, a clash with any live record in `.places/` or `.previews/` lengthening the revision to 12; verify with unit tests: a short name, a 45-character bolt name, a slug with `/` and capitals, a forced clash.
- [ ] 1.4 Add `Checkout::preview`, `preview_of`, `previews` and `release_preview` to `sluice-seams`, implement them in `Places` (record first, then `git worktree add --detach`, caches restored, block held, `SLUICE_NAME_<NAME>`, `SLUICE_BRANCH`, `SLUICE_REVISION` in the place's environment) and in `sluice-fakes` with helpers to script a preview's state; verify with places.rs over real git: ports and names never shared with a candidate's place or an adopted worktree, release removes the worktree and frees the block, releasing twice is a no-op.

## 2. Asking for a preview

- [ ] 2.1 Add `Sluice::preview(branch, rev)` in the core: read the branch (fetching a remote one first), refuse unless `Branches::contains(tip, rev)`, read `[preview]`, ports, caches and release from `policy_at(rev)` and the executor, limits and time limit from the branch's current revision (D4), return an existing started preview unchanged and take up one with no run (100); verify with core tests over fakes for S35's refusal, a missing `[preview]` table, the revision's command chosen over the branch's, and asking twice.
- [ ] 2.2 Start the command (D6): branch lock, the host's sessions lock, room in the bound less the reserve, `Tests::start` with `record: false`, `Sessions::begin` on the preview's checkout path, the waiting loop and Ctrl-C leaving the record; verify over fakes: a full bound waits and lists `waiting`, a freed run starts it, the reserve is never taken, and the host's count drops once the command ends (102).
- [ ] 2.3 Read state from the run directory (no run: `waiting`; supervised: `starting`; exit 0: `up`; else, time limit included: `failed`) and write no result line for any of it (103); verify over fakes that a failed command writes nothing to `results.jsonl` and an entry queued on the branch lands as it would without the preview.
- [ ] 2.4 Add `Command::Preview{branch, revision, --wait, --json}` in `crates/sluice/src/lib.rs` with a `preview.rs` beside `exec.rs`: print name, state, ports and filled addresses, or one JSON document on stdout; `--wait` exits 0 on `up` and 1 on `failed`; verify in `crates/sluice/tests/cli.rs` over fakes, including addresses printed before the command ends (S30).
- [ ] 2.5 Add a real-tools test in `crates/sluice/tests/stack.rs`: a preview command that starts a container and a server on `SLUICE_PORT_WEB`, writes `SLUICE_NAME_WEB` to a file and exits; the stack is still up afterwards and the name and port reached the command; verify under `moon run ':#check-tools'`.
- [ ] 2.6 Document `sluice preview` in `site/src/content/docs/reference/commands.md` (table row, console example with `--json`, exit codes) and `[preview]` with `SLUICE_NAME_*`, `SLUICE_BRANCH`, `SLUICE_REVISION` in `reference/configuration.md`; verify `site:prose` clean.

## 3. Listing and releasing

- [ ] 3.1 Add `Command::Previews{--json}` listing every preview with name, branch, revision, addresses, state and when it was made; verify in cli.rs over fakes, text and JSON.
- [ ] 3.2 Let `Command::Release` take an optional preview name: release it (saved release request under the executor, process group ended, worktree removed, block freed, record removed), "nothing to release" with exit 0 when no record exists, `--worktree` unchanged without a name; verify in cli.rs over fakes and in places.rs with real git that the saved release command runs even after the branch is deleted.
- [ ] 3.3 Add `Sluice::release_unusable_previews` beside `release_abandoned`, run at the start of every round, `exec`, `preview` and `previews`: a failed or timed-out preview, or one whose checkout is gone, is released under the branch's lock taken without waiting, logged `released` in the branch's log, and kept as a `failed` record with the output tail and nothing held; verify over fakes that a round never releases an `up` preview, and with real git that `git worktree remove` of a preview's checkout frees its block at the next `sluice previews`.
- [ ] 3.4 Add conformance scenarios S30 to S35 to `crates/sluice-conformance/src/shared.rs` over fakes, and an e2e step in `e2e/e2e.sh`: land a submission, `sluice preview main <landed rev> --wait`, `sluice previews --json` shows it up, `sluice release <name>` frees its ports; verify `moon run ':#check-e2e'`.
- [ ] 3.5 Document `sluice previews` and `sluice release <preview>` in `reference/commands.md`, and the `.previews/` records and their run directories in `reference/state.md`; verify `site:prose` clean.

## 4. The previews guide, home page and README

- [ ] 4.1 Write `site/src/content/docs/guides/previews.md` (sidebar order 3, after worktrunk; renumber `github.md`, `github-app.md` and `pull-requests.md` to 4, 5, 6): what a preview is for, a running copy of landed work on the repository's own stack and fakes; what it is not, a deployment with production parity, with the table for choosing; the `[preview]` table and a command that routes `SLUICE_NAME_*` through a `.localhost` proxy with `portless alias` and removes it on release; the lifecycle a caller such as crew or flywheel runs against landings (one preview per landed revision, release the previous, release on the branch's landing, reconcile with `sluice previews` after a crash); states and failure; verify `site:prose` clean and `bun run build` renders it under Guides.
- [ ] 4.2 Add a "Look at what landed" card to `site/src/content/docs/index.mdx` and reword the "No daemon" card so it still holds with a preview's handed-off stack (sluice starts nothing that outlives its command; the repository's tools keep the stack until release); link the guide; verify `bun run build` and the home page in the built site.
- [ ] 4.3 Update `README.md`: a sentence in Why on previews, `sluice preview`, `sluice previews` and `sluice release <preview>` in Commands, and a link to the guide; verify every command named matches `sluice --help`.
- [ ] 4.4 Add a paragraph to the walkthrough's services section in `guides/walkthrough.md` pointing at the previews guide, and mark previews built in `site/src/content/docs/roadmap.md`; verify `site:prose` clean.

## 5. Decisions and rules

- [ ] 5.1 Write ADR 0021 "A preview is a place sluice makes at a landed revision and releases when asked" in `docs/adr/` (MADR front matter as 0020; sources: blueprint proposal previews-1 clauses 98 to 103, clauses 25, 31, 34, 56, 66, 71; ADRs 0010, 0016, 0018, 0020; options: a bolt worktree kept by hand, a separate tool, a preview as a third kind of place; consequences: the directory of its own, executor from the branch and command from the revision, no lapse); verify `site/scripts/adrs.mjs` renders it at `/sluice/decisions/0021/`.
- [ ] 5.2 Update `docs/architecture/core.md` and its mirror `site/src/content/docs/architecture/rules.md`: core.16's rule and details for a preview's release (design D9); core.10's "Not ruled out" gains a preview's handed-off stack; add core.19 with its details (rules out: a preview of a revision the branch has not landed, a preview's run in `results.jsonl`, a gate or round reading a preview, a round releasing a preview that is up); core.1 is unchanged, since the proxy is named only in site pages and the repository's own policy; verify every code comment citing core.16 still reads true.

## 6. The design book and the proposal

- [ ] 6.1 In agentplot/blueprints, add `books/sluice/src/previews.md`, a chapter on the proposal under review as `pull-requests.md` is (the third kind of place, names and addresses, the life of a preview, the caller's side and the line between a preview and a deployment), list it in `src/SUMMARY.md` after "Places and runs", and add "preview" to `src/glossary.md`; cite clauses by number and never restate one; verify `mdbook build` in `books/sluice` with no unresolved include.
- [ ] 6.2 Amend `design/sluice/proposals/previews-1.md` with what the build settled: one preview per revision whichever branch asked, the `waiting` state and an asker that blocks for room, the executor and limits from the branch's current revision with the command from the previewed one, a failed preview kept listed until released; verify the README row still describes it.

## 7. Integration

- [ ] 7.1 Run `:#check-fast`, `:#check-tools` and `:#check-e2e`, and `openspec validate previews --strict`; verify all pass and every task above is checked.

## Workflow follow-up

- Land with `devenv shell -- wt submit main` from the `previews` worktree.
- Archive with `/opsx:archive previews` after landing.
- Ask the operator to rule on `previews-1.md` so clauses 98 to 103 move into `requirements.md`; the book's chapter then includes them by anchor.
