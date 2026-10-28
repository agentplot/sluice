# Tasks

## 1. Telling a sluice with no watch

- [x] 1.1 In `plugins/sluice-board/hooks/board.ts`, add `WATCH_SINCE = "0.12"`, `noWatch(stderr)` (true on `unrecognized subcommand 'overview'` or `unexpected argument '--watch'`) and `cannotFollow(version)` (`can't follow <version>: it has no watch; the board needs sluice 0.12 or later`, `this sluice` for a null or empty version); verify with `board.test.ts` cases for both clap wordings, an unrelated stderr, a version line and none, run by `moon run sluice-board:test` under `sluice exec main --`.

## 2. The watch as the first read

- [ ] 2.1 In `plugins/sluice-board/hooks/register.tsx`, remove `probe`; `session.start` starts `feed` at once, and `follow`'s once-a-load setup runs on the first overview `feed` hands `take`, before `take` reads it as the first queue; verify with `register.test.tsx` tests that the band shows `main` with #3 waiting from the watch's first line and that no `sluice overview --json` (without `--watch`) is ever run, and move every existing test that fed a first read through the one-shot overview onto the watch.
- [ ] 2.2 `feed` keeps what each watch wrote to stderr and its exit, and on a watch that ended with no overview since the board last had none sets `unread` with `unreadReason`; while the board has no overview, an error document the watch prints sets `unread` to its message too; `unread` clears on the first overview; verify with `register.test.tsx` tests for "sluice not installed", "The overview fails" (exit 3, `sluice: not a git repository`) and "The watch prints only its failure".
- [ ] 2.3 `feed`'s wait can be woken: `/sluice`, while the board has no overview or can't follow and no watch is open, wakes it so the next watch starts at once, and runs no one-shot read; verify with `register.test.tsx` tests for "sluice set up during the session", "Still failing" and "A watch already open" (one watch running after `/sluice`), and that "One watch at a time" still holds.

## 3. The can't-follow line

- [ ] 3.1 On a watch that ended with `noWatch` true, `feed` runs `sluice --version` and keeps its first line, or null, in a `cannot` atom; the next overview clears it; verify with a `register.test.tsx` test that `sluice --version` runs after such an end and not after any other.
- [ ] 3.2 While `cannot` is set, the band shows `cannotFollow(...)` as its one line, the pane shows it and `Ask Claude to upgrade sluice.` in place of the queue, and the status line is cleared with `$.ui.status(undefined)`; `doc` and `told` are left as they are, so the next overview redraws the queue and tells what was decided meanwhile; verify with `register.test.tsx` tests for "An older sluice on the host", "No version to name", "The host's sluice replaced by one with no watch" and "The host's sluice upgraded" (including an outcome decided while it couldn't follow, told once the queue is back).
- [ ] 3.3 Run the board project's checks, `sluice exec main -- moon run sluice-board:typecheck sluice-board:validate sluice-board:test`, and verify all pass.

## 4. Documentation

- [ ] 4.1 In `plugins/sluice-board/README.md`, say the board reads only through its watch from the session's start, never a one-shot `sluice overview`; that `/sluice` with no queue starts the watch at once; and that where the host's sluice has no watch the band and the pane say the board can't follow it, naming its version and 0.12, and the status line is empty; verify by reading it against the spec delta.
- [ ] 4.2 Say the same, in the user words, in `site/src/content/docs/guides/claude-code.md`; verify with `sluice exec main -- moon run site:prose` clearing every Vale warning.
