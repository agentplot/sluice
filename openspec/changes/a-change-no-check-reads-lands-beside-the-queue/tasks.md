# Tasks

## 1. The reading

- [x] 1.1 Add `Tests::reads`, `Reading` and `Reads` to `sluice-seams`, defaulting to `Every`; verify `cargo check --workspace` passes
- [x] 1.2 Answer `reads` in the fakes from each task's prefix, with switches for a reading of every check and a failing reading; verify by the conformance scenarios of group 2
- [x] 1.3 Answer `reads` in the check profile: the command runner `Every`, the moon runner in a scratch worktree with the toolchain set up, its own settings read as `Every` (design D1, D2); verify by a test in `tests/profiles/moon.rs` (tools tier) that a docs file reads none, an input reads its task and `.moon/` reads every check, and that the scratch tree is gone after

## 2. The round lands beside

- [x] 2.1 Records: `Submission.beside`, `Submission.act`, `Candidate.carries`, `LandingIntent.beside`, `Event::Landed.beside`, `DecidedRecord::Landed.beside`, `Decided::Landed.beside`; verify round-trip unit tests in `records.rs`
- [x] 2.2 Decide the landing beside before arrange (design D3) and perform it, moving candidates on and carrying the heads (D4); the chain's landing commits carry them; verify `sluice-conformance/tests/beside.rs`: docs land at once with nothing started, a combination under test keeps its runs and lands carrying the docs, two landings beside in a row, a combination's checks that read it and a conflict with an entry ahead each wait behind it, read at base, policy, unverified base, every-check runner, failing reading, mixed work, a group's leading heads, a refused move read again
- [x] 2.3 The landing commit's message for a landing beside; verify a unit test in `text.rs`

## 3. What is said

- [x] 3.1 Submit, wait, status and the notice say the landing beside and the waiting entry's words, and JSON carries `beside`; verify unit tests in `words.rs` and the record's JSON test in `records.rs`
- [x] 3.2 The GitHub check, comment and report say it landed with no check run; verify unit tests in `pulls.rs` and `render.rs`
- [x] 3.3 The site's queue and outcome pages, and the using-sluice skill's submit reference, say what lands beside the queue and what a waiting entry is told; verify `site:prose` passes

## 4. The gate

- [ ] 4.1 Commit, run `sluice exec main`, and see every task pass

## Workflow follow-up

- Archive the change with its spec's Purpose, validate `--strict`, and land through `sluice submit main`.
