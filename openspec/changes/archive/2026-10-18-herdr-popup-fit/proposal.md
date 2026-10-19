# Proposal

## Why

Since `run-progress` a combination under test lists each of its tasks in
the popup, and a gate of thirty tasks makes the frame taller than the
popup. The popup redraws from the top of the terminal, so a frame that
runs past the bottom scrolls it, and every redraw scrolls it again: the
head of the frame, the repository and the branch, is printed over and
over and the rest pushed out of sight. The dashboard draws the same way.

Opened from a pane in no git repository, the popup says `No sluice
records in this repository.`, which reads as a fault of the repository
the user thinks they are in, and shows nothing though the watcher may
hold several.

## What Changes

- The popup and the dashboard read the terminal's height at each draw and
  never print more lines than it has, nor end the last with a newline.
  Where the frame does not fit, each combination's task list is cut to its
  running and failed tasks with a line counting the rest
  (`… 18 passed, 10 waiting`), then RECENT's oldest entries go, and only
  then the bottom is cut with a line saying how many lines went. A line
  wider than the pane is cut at its edge rather than wrapped.
- From a pane in no git repository the popup names the folder and shows
  every repository the watcher holds; it says `No sluice records in this
  repository.` only when it holds none.

Out of scope: scrolling the popup, which waits on a change that gives it
keys to move through a frame.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `herdr-status`: the popup fits its pane, and outside git shows every
  repository held.

## Impact

- Changed: `plugins/herdr/bin/status.sh`, `plugins/herdr/lib/words.jq`,
  `plugins/herdr/tests/run.sh`, and the herdr guide on the site.
- No crate changes; no clause of the statement is touched.
