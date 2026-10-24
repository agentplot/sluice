# Tasks

## 1. The segment reads its repository's documents

- [ ] 1.1 In `plugins/herdr/bin/segment.sh`, replace `all_docs | jqw …` with a glob of `$state/docs/*--$(name_of "$repo")-$(sum "$repo").json`: print nothing and exit 0 when none match, else one `jqw -s -r` over the matching files that keeps `.repository == $r` and prints `segment($now)` when not empty. Keep the header comment true to what it does. Verify: the existing `status: segment`, `status: the band says when checks hold the queue back`, `status: branches keep the overview's order` and `status: the focused pane's named branch comes first` cases in `plugins/herdr/tests/run.sh` pass unchanged under `sluice exec main -- moon run plugin-herdr:test-sh plugin-herdr:test-dash`.
- [ ] 1.2 Add a case to `plugins/herdr/tests/run.sh`, `status: another repository's document leaves the segment alone`: feed the board for the focused repository, write a file that is not JSON as `$state/docs/<host>--other-<n>.json` of another repository, run the segment's copy in `$work`, and assert it prints the same band as `status: segment`. Verify: the case fails against the old `all_docs` segment and passes after 1.1, in `moon run plugin-herdr:test-sh` and `plugin-herdr:test-dash`.

## 2. The segment needs only the server's environment

- [ ] 2.1 Add a case to `plugins/herdr/tests/run.sh`, `status: the segment is the server's`: with a document fed and the watcher run one tick under a home whose state is at `$HOME/.local/state/herdr/plugins/agentplot.sluice`, run setup's command as herdr does, `cd "$work" && env -i HOME="$home" HERDR_SOCKET_PATH="$HERDR_SOCKET_PATH" PATH="$PATH" /bin/sh -lc "$cmd"`, and assert it prints the board's line. Change `segment.sh` or `lib/common.sh` only if the case shows it needs more of the environment. Verify: `moon run plugin-herdr:test-sh plugin-herdr:test-dash` through `sluice exec main --`.

## 3. Setup and the README give the line 30 seconds

- [ ] 3.1 In `plugins/herdr/bin/setup.sh`, set the segment's entry to `interval_seconds = 5, timeout_seconds = 30`, with a comment saying why: herdr keeps the last line until a run ends and clears it on the limit, so the limit is what a loaded host must meet. Extend `status: setup shows, never edits` in `plugins/herdr/tests/run.sh` to assert the entry carries `interval_seconds = 5, timeout_seconds = 30`, and keep `status: one tab-bar line for every host` parsing the command out of the new line. Verify: both cases pass in `moon run plugin-herdr:test-sh plugin-herdr:test-dash`.
- [ ] 3.2 In `plugins/herdr/README.md`, change the `tab_bar_right` snippet to `timeout_seconds = 30`, and add to the paragraph after the snippets: herdr runs the command on the server the panes run on, so a client on another host (`herdr --remote`, a saved machine, a session bridging to the server) shows the server's queue; the client's `config.toml` holds the entry, which is why it names `$HOME`; and a limit of a few seconds blanks the line whenever the server is loaded. Verify: `sluice exec main -- moon run root:format` passes and the snippet matches what `setup` prints (the same line, compared by eye against `sh plugins/herdr/bin/setup.sh </dev/null`, since no task reads the README).

## 4. The gate

- [ ] 4.1 Commit, then run `sluice exec main` and get a pass from every check-fast task, `plugin-herdr:manifest`, `plugin-herdr:test-sh`, `plugin-herdr:test-dash` and `root:format` among them.

## Workflow follow-up

- Once landed: on deneb and mac-studio, `~/.config/herdr/config.toml` line 130's entry to `timeout_seconds = 30`, then `herdr server reload-config` per running session (the operator's; setup never edits it).
- On a live server, check that an entry whose run outlasts its interval keeps its last line until the run ends (design.md, Risks).
