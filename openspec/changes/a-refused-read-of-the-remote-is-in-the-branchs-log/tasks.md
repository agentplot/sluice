# Tasks

Read first: this change's proposal, design and specs; clause 133 and
the pushed refs row in `agentplot/blueprints:design/sluice/requirements.md`;
records 0017, 0053 and 0061; `Event::CopyBehind` and `note_copy` in
`crates/sluice-core/src/step.rs`, the precedent. Work inside
`devenv shell`. Run suites through `sluice exec main -- <command>`.
Edit nothing outside the paths under Touches.

## 1. The seams and the profiles raise a refused read by its type

- [x] 1.1 Add `ReadRefused { remote, asked, said, note }` and `read_refused(&anyhow::Error) -> Option<&ReadRefused>` to `crates/sluice-seams/src/lib.rs` beside `Unanswered`, its `Display` being `fetching <asked> from <remote> failed: <said>` and the note on the next line (design D1); change `State::stale` to return `Option<ReadRefused>` and add `State::refusal` and `State::keep_refusal` with default methods that keep nothing (D2, D4). Verify with a unit test that `read_refused` finds the error under added context and that its text is today's, and `moon run seams:test` passes.
- [ ] 1.2 In `crates/sluice/src/profiles/pushed_refs.rs`, keep `remote_git`'s note on refused credentials apart from git's stderr for a fetch, a push keeping today's appended text, and raise `ReadRefused` from `fetch` and `fetch_ref`, a missing ref still `None` (D1). Verify in `crates/sluice/tests/pushed_refs.rs` that a fetch from a remote whose path is not there fails with a `ReadRefused` naming the remote, `refs/heads/main` and git's words unchanged, and that a refused credential's second try keeps the note out of `said` and in the error's text; `moon run cli:tools` passes.
- [ ] 1.3 In `crates/sluice/src/profiles/remote_refs.rs`, keep the `ReadRefused` a stale read met and return it from `stale`, raise it from `fetch_heads` and `fetch_candidates`, and keep the host's refusal document at `<git-dir>/sluice-remote/<remote>/refused/<branch>.json`, written atomically; in `local_files.rs` keep it at `<common>/sluice/.refused/<branch>.json`; route both through `profiles/records.rs` (D2, D4). Verify in `crates/sluice/tests/remote_refs.rs` that a refused state fetch leaves `stale` naming the ref and words, that each profile keeps, replaces and removes the document, and that `branches()` never lists `.refused`; `moon run cli:tools` passes.
- [ ] 1.4 Let the fakes in `crates/sluice-fakes/src/lib.rs` refuse the next reads of a branch, or of its state, with given words, and keep the host's refusal document in memory. Verify with a fakes unit test, and `moon run fakes:test` passes.

## 2. The round logs a refusal once per cause and its end

- [ ] 2.1 Add `Event::ReadRefused` and `Event::ReadAgain` to `crates/sluice-core/src/records.rs` as design D6 names them, and the host's document and its updates in `crates/sluice-core/src/refused.rs` (D4). Verify with unit tests that both events round-trip as the spec's lines, that a refused round adds a cause only for a new `(asked, said)` and counts every round, and that `stood` is seconds from `since` to `ended`; `moon run core:test` passes.
- [ ] 2.2 Classify each locked round in `Sluice::step_locked` as design D3 says: a stale first read of the records refuses the round before anything is decided, an error holding `ReadRefused` refuses it, `Ok` ends a standing refusal, and a busy round or any other error counts as nothing; a refused round returns its error unchanged. Verify in a new `crates/sluice-conformance/tests/refused.rs` over the fakes that a refused round forms, starts and lands nothing and its error is the read's, and that a busy round adds no round to the count; `moon run conformance:test` passes.
- [ ] 2.3 Make `Sluice::write` carry the document's unlogged causes and its end, at their own times, and mark them logged or remove the document once the write is written; a refused round and a round that reads again make a write of their own when the document has a line to write and the records can be written (D5). Verify in `refused.rs` the spec's scenarios: forty rounds refused alike log one line; words that change and change back log one line each; two hosts log their own; the end is logged with `since`, `stood` and `rounds`; a refusal after an end opens anew; with the state on the refusing remote, both lines land in the first write after, timed when they happened, and no line is written twice; `moon run conformance:test` passes.

## 3. Status and the pages

- [ ] 3.1 Add `refused` to `Status` in `crates/sluice-core/src/ops.rs` from this host's document while it has not ended, and show it in `crates/sluice/src/lib.rs` above the queue in text and under `refused` in `--json`, taking `stale`'s remote from its `ReadRefused` (D2, D7). Verify in `crates/sluice/tests/cli.rs` that status shows the refusal's ref, remote, since, rounds and git's words during a refusal and nothing once it ended, and that a stale state still says it could not refresh; `moon run cli:test` passes.
- [ ] 3.2 Document `read-refused` and `read-again` in `site/src/content/docs/reference/state.md` beside `copy-behind`, with the spec's two lines as examples and that a line written late keeps the time it happened, and add the host's `.refused/` and `sluice-remote/<remote>/refused/` to its layout; add the refusal's line to `sluice status` in `site/src/content/docs/reference/commands.md` where status's other lines above the queue are. Verify `moon run site:prose` passes with no warning.

## 4. Against a real remote

- [ ] 4.1 In `crates/sluice/tests/remote_branch.rs`, with a bare remote moved away so every fetch fails: a waiting `sluice submit main` prints the fetch's failure as before, `main`'s `log.jsonl` holds one `read-refused` line after several rounds, and once the remote is put back one round logs `read-again` with the rounds counted; with the state on the remote too, both lines land in the first write after the remote is back. Verify `moon run cli:tools` passes.
- [ ] 4.2 Run `sluice exec main` once the work is committed; verify every gate task passes.

## Workflow follow-up

- Archive with `/opsx:archive a-refused-read-of-the-remote-is-in-the-branchs-log` once merged; it syncs `remote-read-log` as a new spec and adds the requirement to `remote-records`.
