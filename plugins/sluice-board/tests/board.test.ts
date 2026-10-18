import { describe, expect, test } from "claude-code/testing";

import {
  bandLines,
  bar,
  duration,
  githubBase,
  isMine,
  learn,
  outcomes,
  progress,
  pullOf,
  recentKeys,
  statusLine,
  submitted,
  taskLines,
  wakeText,
} from "../hooks/board";
import type { Mine, Overview } from "../types";
import { NOW, overview, withTasks } from "./fixture";

const ME: Mine = { root: "/work/sluice/plugin-board", ids: [], pulls: [] };

describe("the band", () => {
  test("one line per branch with a queue, with the clock against the limit", () => {
    expect(bandLines(overview(), NOW)).toEqual([
      "sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting · last #9 blamed on 4f1c2aa",
    ]);
  });

  test("nothing while no branch has a queue", () => {
    const quiet = overview();
    quiet.branches[0]!.depth = 0;
    quiet.branches[0]!.candidates = [];
    expect(bandLines(quiet, NOW)).toEqual([]);
    expect(bandLines(null, NOW)).toEqual([]);
  });

  test("a combination's tasks: how many have ended and a bar, never the tasks", () => {
    expect(bandLines(withTasks(), NOW)).toEqual([
      "sluice main · 3 submissions · testing main+#12+#13 3/5 ▓▓▓▓▓▓░░░░ 2m10s/15m · #14 waiting · last #9 blamed on 4f1c2aa",
    ]);
  });

  test("the bar fills a cell per tenth ended, rounded down", () => {
    expect(bar(0, 5)).toBe("░░░░░░░░░░");
    expect(bar(1, 3)).toBe("▓▓▓░░░░░░░");
    expect(bar(5, 5)).toBe("▓▓▓▓▓▓▓▓▓▓");
    expect(bar(0, 0)).toBe("░░░░░░░░░░");
    expect(progress(overview().branches[0]!.candidates[0]!)).toBeNull();
  });

  test("the task list: ended in the order they ended, then running, then waiting", () => {
    expect(taskLines(withTasks().branches[0]!.candidates[0]!).map((t) => t.text)).toEqual([
      "✓ sluice:test      1m48s",
      "✓ site:prose    replayed",
      "✓ sluice:tools     2m30s",
      "… sluice:e2e     running",
      "· core:test      waiting",
    ]);
  });

  test("durations", () => {
    expect(duration(45)).toBe("45s");
    expect(duration(130)).toBe("2m10s");
    expect(duration(3780)).toBe("1h03m");
  });
});

describe("links to pull requests", () => {
  test("a GitHub origin in any spelling", () => {
    for (const url of [
      "https://github.com/agentplot/sluice.git",
      "https://github.com/agentplot/sluice",
      "git@github.com:agentplot/sluice.git",
      "ssh://git@github.com/agentplot/sluice.git\n",
    ]) {
      expect(githubBase(url)).toBe("https://github.com/agentplot/sluice");
    }
    expect(githubBase("https://gitlab.com/agentplot/sluice.git")).toBe(null);
  });

  test("a submission from a pull request", () => {
    expect(pullOf("pull/41")).toBe(41);
    expect(pullOf("plugin-board")).toBe(null);
  });
});

describe("this session's submissions", () => {
  test("matched on from.path, an id it submitted, or its pull request", () => {
    const s = (id: number, worktree: string, path?: string) => ({ id, from: { worktree, path } });
    expect(isMine(s(13, "plugin-board", "/work/sluice/plugin-board"), ME)).toBe(true);
    expect(isMine(s(14, "herdr-panel", "/work/sluice/herdr-panel"), ME)).toBe(false);
    expect(isMine(s(14, "herdr-panel"), { ...ME, ids: [14] })).toBe(true);
    expect(isMine(s(15, "pull/8"), { ...ME, pulls: ["pull/8"] })).toBe(true);
  });

  test("what sluice submit printed names the submission or the pull request", () => {
    expect(submitted("submission 12 to main · nothing queued ahead\n")).toEqual({
      ids: [12],
      pulls: [],
    });
    expect(submitted('{"id":7,"branch":"main","ahead":[]}')).toEqual({ ids: [7], pulls: [] });
    expect(submitted("pushed plugin-board · opened #8: feat\nqueued: next on main\n")).toEqual({
      ids: [],
      pulls: ["pull/8"],
    });
  });

  test("the status line", () => {
    expect(statusLine(overview(), learn(overview(), ME))).toBe(
      "sluice: 3 submissions · yours #13 testing",
    );
  });
});

describe("outcomes", () => {
  test("a blame of this session's submission is told once", () => {
    const before = overview();
    const mine = learn(before, ME);
    const told = recentKeys(before);
    const after: Overview = overview();
    after.branches[0]!.recent.push({
      schema: "sluice/log@1",
      at: "2026-10-16T19:50:00Z",
      event: "blamed",
      id: 13,
      combination: "9a03c1e5d2",
    });
    const found = outcomes(after, mine, told);
    expect(found.length).toBe(1);
    expect(found[0]!.word).toBe("failed");
    expect(found[0]!.id).toBe(13);
    expect(outcomes(after, mine, [...told, found[0]!.key])).toEqual([]);
  });

  test("another session's landing is not this one's", () => {
    const after = overview();
    after.branches[0]!.recent.push({
      at: "2026-10-16T19:51:00Z",
      event: "landed",
      ids: [14],
      to: "aaaaaaa",
    });
    expect(outcomes(after, learn(overview(), ME), recentKeys(overview()))).toEqual([]);
  });

  test("the wake names the blamed tests and their logs", () => {
    const text = wakeText(
      { key: "k", branch: "main", id: 13, word: "failed", combination: "9a03c1e5d2" },
      [{ task: "cli:test", log: "/r/sluice.git/sluice/.logs/main/c7/cli:test.log" }],
    );
    expect(text).toContain("#13 to main failed: cli:test on 9a03c1e");
    expect(text).toContain("Log of cli:test: /r/sluice.git/sluice/.logs/main/c7/cli:test.log");
    expect(text).toContain("sluice exec main");
  });
});
