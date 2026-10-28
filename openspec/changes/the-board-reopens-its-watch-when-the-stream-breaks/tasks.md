# Tasks

## 1. The wait before the next watch

- [ ] 1.1 In `plugins/sluice-board/hooks/board.ts`, add a pure function giving the wait before the board starts its watch again, from the last wait (none before the first) and how long the watch that ended was open: 1000 ms with no last wait or after a watch open at least 60 000 ms, else twice the last wait, at most 60 000 ms. Its doc comment cites clause 129 and ADR 0056 and says why the count starts over only after a minute's life (design, "The wait doubles"). Verify with tests in `plugins/sluice-board/tests/board.test.ts`: the waits from none run 1000, 2000, 4000, 8000, 16000, 32000, 60000, 60000 for watches that end at once; 60000 followed by a watch open 600 000 ms gives 1000; a watch open 59 999 ms after a 4000 wait gives 8000. Run `moon run plugin-board:test plugin-board:typecheck`.

## 2. The board starts its watch again and never polls

- [ ] 2.1 Rewrite `feed` in `plugins/sluice-board/hooks/register.tsx` as a loop for the session's life: note the time, start `sluice overview --json --watch`, read its lines into `take` through `parsed` (a line that isn't JSON passed over, the watch kept), and when the stream ends or its first pull rejects, wait the function from 1.1 gives on `$.clock.after` and start it again. Remove the 3-second `$.clock.every` poll and its comment; rewrite `feed`'s doc comment to say it watches for the session's life and starts the watch again when it ends (129; ADR 0056). Verify with `moon run plugin-board:typecheck plugin-board:validate`.
- [ ] 2.2 In `plugins/sluice-board/tests/register.test.tsx`, move every test that fed the board documents through the poll onto the watch: `forecastHost` and the tests built on it, and "an overview's error is one line in the band", which becomes "…from --watch, until a document comes" with its later documents printed by watches started again. Give the mock a watch the test hands documents to as it goes (each printed as one line on the open watch), in place of `clock.advance(3000)` driving a poll; each test keeps what it asserted. Verify with `moon run plugin-board:test`, every test passing, and no test left calling `sluice overview --json` after the session's first read.
- [ ] 2.3 Add to `plugins/sluice-board/tests/register.test.tsx` one test per scenario of "The board reads the queue only through its watch" and "The wait before the next watch grows": the watch ends and the next prints #13 landed, the second watch started 1 s after the first ended, the landing told, and `ran` holding one `sluice overview --json`; every watch fails to start, the starts 1, 2, 4, 8, 16, 32, 60 and 60 s apart, no `sluice overview --json` after the first read, the band still showing `main` with #3 queued; a watch open 10 minutes ending, the next started 1 s later; a watch open 10 minutes after 60-second waits, the next started 1 s after it ended; two `/sluice` runs with the watch open leaving one watch running. Verify with `moon run plugin-board:test`.

## 3. Say it to the operator

- [ ] 3.1 In `plugins/sluice-board/README.md`, after "It reads everything from `sluice overview --json --watch`", say that when the watch ends the board starts it again, waiting a second and then longer each time up to a minute, and never polls; the band and the pane keep the last queue read meanwhile. Verify by reading it against the requirements: no gate task reads `*.md` outside the site (core.21), so this one is checked by hand.
- [ ] 3.2 In `site/src/content/docs/guides/claude-code.md`, in the paragraph that begins "It reads everything from `sluice overview --json --watch`", say the same in one or two sentences, with no name of ours (rule core.26). Verify with `moon run site:prose`, every warning cleared.

## 4. The gate

- [ ] 4.1 Run the gate's checks on the committed work, `sluice exec main`, and verify every task passes, `plugin-board:test` and `site:prose` among them.
