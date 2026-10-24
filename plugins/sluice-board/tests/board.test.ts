import { describe, expect, test } from "claude-code/testing";

import {
  bandLines,
  bar,
  checkLines,
  checkRows,
  clearText,
  conflictLines,
  conflictText,
  duration,
  emptyLines,
  errorLine,
  finishedWords,
  flipped,
  forecastBranch,
  forecastKey,
  forecasts,
  forecastToast,
  forecastWords,
  githubBase,
  heldBack,
  hostLine,
  hostsLine,
  isMine,
  isOverview,
  latest,
  learn,
  moving,
  namedFirst,
  namedLine,
  openedText,
  outcomes,
  progress,
  pullOf,
  recentInWay,
  recentKeys,
  shown,
  signature,
  statusLine,
  submitted,
  taskLines,
  unreadReason,
  wakeText,
} from "../hooks/board";
import type { Mine, Overview } from "../types";
import { NOW, overview, withChecks, withRelease, withTasks } from "./fixture";

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

  test("an overview's error document: its message on one line, and no branch", () => {
    const failed = {
      error: {
        kind: "internal",
        message: "opening the repository at /work/x:\n  it is gone",
        next: [],
      },
    };
    expect(errorLine(failed)).toBe("sluice: opening the repository at /work/x: it is gone");
    expect(isOverview(failed)).toBe(false);
    expect(bandLines(failed as never, NOW)).toEqual([]);
    expect(isOverview(overview())).toBe(true);
    expect(errorLine(overview())).toBeNull();
    for (const v of [null, "error", 3, {}, { error: "gone" }, { error: { kind: "internal" } }]) {
      expect(isOverview(v)).toBe(false);
      expect(errorLine(v)).toBeNull();
    }
  });

  test("a branch shown in the pane: busy, or an event under a quarter hour old", () => {
    const b = overview().branches[0]!;
    expect(shown(b, NOW)).toBe(true);
    const idle = { ...b, depth: 0, candidates: [], submissions: [], recent: [] };
    expect(shown(idle, NOW)).toBe(false);
    const at = (s: string) => [{ schema: "sluice/log@1", at: s, event: "landed", ids: [1] }];
    expect(shown({ ...idle, recent: at("2026-10-16T19:40:00Z") }, NOW)).toBe(true);
    expect(shown({ ...idle, recent: at("2026-10-16T19:20:00Z") }, NOW)).toBe(false);
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
    expect(taskLines(withTasks().branches[0]!.candidates[0]!, NOW).map((t) => t.text)).toEqual([
      "✓ sluice:test           1m48s",
      "✓ site:prose         replayed",
      "✓ sluice:tools          2m30s",
      "… sluice:e2e    running 1m12s",
      "· core:test           waiting",
    ]);
  });

  test("a running task's time since it started, or running alone with none", () => {
    const one = (started?: string) =>
      taskLines(
        { tasks: [{ target: "sluice:e2e", state: "running", ...(started ? { started } : {}) }] },
        NOW,
      ).map((t) => t.text);
    expect(one("2026-10-16T19:43:58Z")).toEqual(["… sluice:e2e  running 1m12s"]);
    expect(one()).toEqual(["… sluice:e2e  running"]);
  });

  test("durations", () => {
    expect(duration(45)).toBe("45s");
    expect(duration(130)).toBe("2m10s");
    expect(duration(3780)).toBe("1h03m");
  });
});

describe("rows opened in place", () => {
  test("a press opens a row and a second folds it, the newest 200 kept", () => {
    let list: string[] = [];
    for (let i = 0; i < 201; i++) list = flipped(list, `k${i}`);
    expect(list).toHaveLength(200);
    expect(list[0]).toBe("k1");
    expect(list[199]).toBe("k200");
    expect(flipped(list, "k7")).not.toContain("k7");
    expect(flipped(flipped(list, "k7"), "k7").at(-1)).toBe("k7");
  });
});

describe("agents' checks", () => {
  const BAND =
    "sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting · last #9 blamed on 4f1c2aa";

  test("the band says when they hold the queue back, at the end of the branch's line", () => {
    expect(bandLines(withChecks(), NOW)).toEqual([`${BAND} · 1 slot to an agent's checks`]);
  });

  test("two agents' checks are two slots", () => {
    const d = withChecks();
    d.room = { bound: 3, queue: 1, sessions: 2, reserve: 0 };
    expect(bandLines(d, NOW)).toEqual([`${BAND} · 2 slots to agents' checks`]);
  });

  test("nothing of checks while the host has a run free", () => {
    const d = withChecks();
    d.room = { bound: 3, queue: 1, sessions: 1, reserve: 0 };
    expect(bandLines(d, NOW)).toEqual([BAND]);
    expect(statusLine(d, ME)).toBe("sluice: 3 submissions · yours #13 testing");
  });

  test("nothing of checks while the branch tests all its window does", () => {
    const d = withChecks();
    d.branches[0]!.window = 1;
    expect(heldBack(d.branches[0]!, d.room)).toBe(false);
    expect(bandLines(d, NOW)).toEqual([BAND]);
  });

  test("nothing of checks with nothing waiting, nor from a sluice with no room", () => {
    const d = withChecks();
    d.branches[0]!.submissions = d.branches[0]!.submissions.filter((s) => s.id !== 14);
    expect(heldBack(d.branches[0]!, d.room)).toBe(false);
    const old = withChecks();
    delete old.room;
    expect(bandLines(old, NOW)).toEqual([BAND]);
    expect(hostLine(old.room)).toBeNull();
  });

  test("a submission whose combination passed waits on no run", () => {
    const d = withChecks();
    d.branches[0]!.submissions.find((s) => s.id === 14)!.state = "passed";
    expect(heldBack(d.branches[0]!, d.room)).toBe(false);
  });

  test("the status line says it once", () => {
    expect(statusLine(withChecks(), learn(withChecks(), ME))).toBe(
      "sluice: 3 submissions · yours #13 testing · 1 slot to an agent's checks",
    );
  });

  test("CHECKING: each check going or ended this quarter hour, never as testing", () => {
    const rows = checkRows(withChecks(), NOW);
    expect(rows.map((r) => r.text)).toEqual([
      "fix-cards → main 3/8 ▓▓▓░░░░░░░ 1m10s",
      "e2e-stack → main make e2e 2m00s",
      "done-ok → main 14 passed · reused when submitted",
      "dirty → main 14 passed · not kept: uncommitted changes",
      "broke → main 2 of 14 failed",
    ]);
    expect(rows.map((r) => r.state)).toEqual(["running", "running", "passed", "passed", "failed"]);
    expect(rows.some((r) => /testing/.test(r.text))).toBe(false);
  });

  test("what a CHECKING row opens to: its run's tasks, or why it has none", () => {
    const d = withChecks();
    const of = (name: string) =>
      checkLines(d.sessions.find((s) => s.worktree.endsWith(`/${name}`))!, NOW).map((l) => l.text);
    expect(of("fix-cards")).toHaveLength(8);
    expect(of("fix-cards")[3]).toMatch(/^… t:3\s+running$/);
    expect(of("e2e-stack")).toEqual(["runs make e2e · no tasks"]);
    expect(of("done-ok")).toHaveLength(14);
    const ended = {
      at: "2026-10-16T19:44:00Z",
      head: "e20a6f9b48",
      outcome: "pass" as const,
      recorded: false,
    };
    const w = { worktree: "/w/x", branch: "main" };
    expect(checkLines({ ...w, last_result: { ...ended, kind: "command" } }, NOW)).toEqual([
      { text: "ran a command · no tasks" },
    ]);
    expect(checkLines({ ...w, last_result: { ...ended, kind: "checks" } }, NOW)).toEqual([
      { text: "no tasks reported" },
    ]);
    expect(checkLines({ ...w, run: { started: "2026-10-16T19:45:00Z" } }, NOW)).toEqual([
      { text: "no tasks reported" },
    ]);
  });

  test("a check whose runner gives no tasks shows its time alone", () => {
    const d = withChecks();
    d.sessions = [
      { worktree: "/w/plain", branch: "main", run: { started: "2026-10-16T19:45:00Z" } },
    ];
    expect(checkRows(d, NOW).map((r) => r.text)).toEqual(["plain → main 10s"]);
  });

  test("what a finished check is worth", () => {
    const r = {
      at: "2026-10-16T19:44:00Z",
      head: "e20a6f9b48",
      kind: "checks" as const,
      outcome: "pass" as const,
      recorded: true,
    };
    const passed = (n: number) =>
      Array.from({ length: n }, (_, i) => ({ target: `t:${i}`, state: "passed" as const }));
    // Recorded, but a gate that does not trust its executor.
    expect(finishedWords({ ...r, tasks: passed(14), reused: false })).toBe("14 passed");
    expect(finishedWords({ ...r, tasks: [{ target: "a", state: "replayed" }], reused: true })).toBe(
      "1 passed · reused when submitted",
    );
    // A command's run, and one with no report.
    expect(finishedWords({ ...r, kind: "command", recorded: false })).toBe("passed");
    expect(finishedWords({ ...r, outcome: "timed_out", recorded: false })).toBe("timed out");
    expect(finishedWords({ ...r, outcome: "lost", recorded: false })).toBe("lost");
  });

  test("the host line", () => {
    expect(hostLine({ bound: 6, queue: 3, sessions: 1, reserve: 0 })).toBe(
      "host: 4 of 6 runs · 3 queue · 1 agent",
    );
    expect(hostLine({ bound: 6, queue: 0, sessions: 2, reserve: 1 })).toBe(
      "host: 2 of 6 runs · 2 agents",
    );
    expect(hostLine({ bound: 6, queue: 0, sessions: 0, reserve: 0 })).toBeNull();
  });

  test("times move while a check goes, and stop once none is left to show", () => {
    const d = overview();
    d.branches[0]!.candidates = [];
    expect(moving(d, NOW)).toBe(false);
    expect(moving(withChecks(), NOW)).toBe(true);
    const later = Date.parse("2026-10-16T21:00:00Z");
    const ended = withChecks();
    ended.branches[0]!.candidates = [];
    ended.sessions = ended.sessions.filter((s) => !s.run);
    expect(moving(ended, later)).toBe(false);
  });
});

describe("the branches' order", () => {
  test("the band keeps the overview's order, whatever the names", () => {
    const d = overview();
    const main = d.branches[0]!;
    d.branches = [{ ...main, branch: "release" }, { ...main, branch: "bolt/a" }, main];
    expect(bandLines(d, NOW).map((l) => l.split(" · ")[0])).toEqual([
      "sluice release",
      "sluice bolt/a",
      "sluice main",
    ]);
  });

  test("a branch named for the session comes first, the rest as the overview has them", () => {
    const d = overview();
    const main = d.branches[0]!;
    d.branches = [{ ...main, branch: "release" }, { ...main, branch: "bolt/a" }, main];
    expect(bandLines(d, NOW, "bolt/a").map((l) => l.split(" · ")[0])).toEqual([
      "sluice bolt/a",
      "sluice release",
      "sluice main",
    ]);
    expect(namedFirst(d.branches, "main").map((b) => b.branch)).toEqual([
      "main",
      "release",
      "bolt/a",
    ]);
    expect(namedFirst(d.branches, "hotfix").map((b) => b.branch)).toEqual([
      "release",
      "bolt/a",
      "main",
    ]);
    expect(namedFirst(d.branches, null).map((b) => b.branch)).toEqual([
      "release",
      "bolt/a",
      "main",
    ]);
  });
});

describe("a branch named for the session", () => {
  test("its queue, its oldest combination under test, and the checks held to it", () => {
    expect(namedLine(withRelease(), "release", NOW)).toBe(
      "release · 3 queued · #12 testing 3/8 · 2 checking",
    );
  });

  test("its oldest combination's minutes when it lists no tasks, and how many more", () => {
    const d = withRelease();
    const b = d.branches[1]!;
    const c = { ...b.candidates[0]!, started: "2026-10-16T19:41:10Z" };
    delete c.tasks;
    b.candidates = [{ ...c, id: "c10", submissions: [14], started: "2026-10-16T19:43:00Z" }, c];
    b.submissions = b.submissions
      .filter((s) => s.id === 12 || s.id === 14)
      .map((s) => ({ ...s, state: "testing" }));
    d.sessions = [];
    expect(namedLine(d, "release", NOW)).toBe("release · #12 testing 4m +1 more");
  });

  test("nothing queued, or no records", () => {
    expect(namedLine(withRelease(), "main", NOW)).toBe("main · nothing queued");
    expect(namedLine(withRelease(), "hotfix", NOW)).toBe("hotfix · no records");
  });

  test("on the status line, after the session's own submissions", () => {
    expect(statusLine(withRelease(), ME, "release", NOW)).toBe(
      "sluice: release · 3 queued · #12 testing 3/8 · 2 checking",
    );
    expect(statusLine(withRelease(), ME, "hotfix", NOW)).toBe("sluice: hotfix · no records");
    expect(statusLine(overview(), learn(overview(), ME), "release", NOW)).toBe(
      "sluice: 3 submissions · yours #13 testing",
    );
    expect(statusLine(withRelease(), ME, null, NOW)).toBe("sluice: 4 submissions");
  });
});

describe("the pane", () => {
  test("the reply to /sluice says how to close it where it opened", () => {
    expect(openedText(false)).toBe(
      "sluice's queue is open above the prompt. Esc, its close button or /sluice again closes it.",
    );
    expect(openedText(true)).toBe(
      "sluice's queue is open in a pane. Its close mark or /sluice again closes it.",
    );
  });

  test("why sluice could not be read: not started, its stderr, else its exit", () => {
    const got = (exitCode: number, stderr: string) => ({ exitCode, stderr });
    expect(unreadReason(null)).toBe("the sluice command was not found");
    expect(unreadReason(got(3, "sluice: not a git repository\n"))).toBe(
      "sluice: not a git repository",
    );
    expect(unreadReason(got(2, "\n  \nsluice: no policy\nmore\n"))).toBe("sluice: no policy");
    expect(unreadReason(got(1, ""))).toBe("sluice overview exited 1");
    expect(unreadReason(got(4, " \n"))).toBe("sluice overview exited 4");
  });

  test("with no branch and none named, how a submission gets there", () => {
    const none = overview();
    none.branches = [];
    expect(emptyLines(none, null)).toEqual([
      "Nothing submitted yet.",
      "When an agent in a worktree runs sluice submit <branch>, its submission shows here until it lands.",
    ]);
    expect(emptyLines(none, "release")).toEqual([]);
    expect(emptyLines(overview(), null)).toEqual([]);
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

  test("a failure names the system it failed on", () => {
    const text = wakeText(
      { key: "k", branch: "main", id: 13, word: "failed", combination: "9a03c1e5d2" },
      [{ task: "cli:build", log: "/l/cli:build.log", system: "linux-arm64" }, { task: "cli:test" }],
    );
    expect(text).toContain("failed: cli:build (linux-arm64), cli:test on 9a03c1e");
    expect(text).toContain("Log of cli:build (linux-arm64): /l/cli:build.log");
  });
});

describe("hosts", () => {
  test("the hosts line counts hosts, their room and the lapsed", () => {
    expect(
      hostsLine([
        { host: "box", signer: "box", held: 1, room: 3, lapsed: false },
        { host: "gone", held: 1, room: 0, lapsed: true },
      ]),
    ).toBe("2 hosts, room for 3, 1 lapsed");
    expect(hostsLine([{ host: "box", held: 0, room: 4, lapsed: false }])).toBe(
      "1 host, room for 4",
    );
  });
});

describe("forecasts", () => {
  /** The fixture with #13, this session's, gone from main's queue. */
  function free(): Overview {
    const d = overview();
    d.branches[0]!.submissions = d.branches[0]!.submissions.filter((s) => s.id !== 13);
    return d;
  }
  const landed = { conflict: { paths: ["docs/builds.md"], with: [] }, ahead: [] };
  const queued = {
    conflict: {
      paths: ["docs/builds.md", "docs/identity.md"],
      with: [{ id: 217, from: { agent: "runner", worktree: "bolt/runner" } }],
    },
  };

  test("not while a submission of the session's waits or is under test", () => {
    const b = overview().branches[0]!;
    expect(forecasts(b, ME)).toBe(false);
    expect(forecasts(free().branches[0]!, ME)).toBe(true);
    expect(forecasts({ ...free().branches[0]!, at: null }, ME)).toBe(false);
  });

  test("the branch: the latest submission's, else the checkout's, else the only one", () => {
    const d = free();
    const other = { ...d.branches[0]!, branch: "release", submissions: [], recent: [] };
    d.branches.push(other);
    expect(forecastBranch(d, ME)).toBeNull();
    d.sessions = [{ worktree: ME.root!, branch: "release" }];
    expect(forecastBranch(d, ME)?.branch).toBe("release");
    d.branches[0]!.recent.push({ at: "2026-10-16T19:44:00Z", event: "conflict", id: 20 });
    expect(forecastBranch(d, { ...ME, ids: [20] })?.branch).toBe("main");
    expect(forecastBranch(free(), ME)?.branch).toBe("main");
  });

  test("the latest submission: in a queue, or named by a recent line", () => {
    const d = free();
    d.branches[0]!.recent.push({ at: "2026-10-16T19:44:00Z", event: "conflict", id: 20 });
    expect(latest(d, { ...ME, ids: [20] })).toEqual({
      id: 20,
      branch: "main",
      conflict: true,
      active: false,
    });
    expect(latest(overview(), ME)).toEqual({
      id: 13,
      branch: "main",
      conflict: false,
      active: true,
    });
    expect(latest(free(), ME)).toBeNull();
  });

  test("the key: the head, the branch's revision, the others waiting in order", () => {
    const b = free().branches[0]!;
    expect(forecastKey("h1", b, ME)).toBe("h1 9ecc34a1b2 12,14");
    expect(forecastKey("h1", overview().branches[0]!, ME)).toBe("h1 9ecc34a1b2 12,14");
  });

  test("a conflict's signature: the files and the submissions, in any order", () => {
    expect(signature("main", { conflict: null })).toBeNull();
    expect(signature("main", queued)).toBe("main|docs/builds.md,docs/identity.md|217");
    const swapped = {
      conflict: { ...queued.conflict, paths: ["docs/identity.md", "docs/builds.md"] },
    };
    expect(signature("main", swapped)).toBe(signature("main", queued));
  });

  test("an opened conflict: its files, what is in the way, and the next step", () => {
    const next = "next: take main into the worktree, resolve and commit, then sluice submit main";
    const b = { ...free().branches[0]!, recent_from: { "217": "bolt/runner" } };
    const line = (w: number[]) => ({
      at: "2026-10-16T19:45:00Z",
      event: "conflict",
      id: 218,
      with: w,
    });
    expect(conflictLines("main", ["docs/builds.md"], recentInWay(b, line([217])))).toEqual([
      "files: docs/builds.md",
      "in the way: #217 (bolt/runner)",
      next,
    ]);
    // An id the sluice gave no worktree for is named alone.
    expect(conflictLines("main", ["a", "b"], recentInWay(b, line([217, 219])))).toEqual([
      "files: a, b",
      "in the way: #217 (bolt/runner), #219",
      next,
    ]);
    expect(conflictLines("main", ["docs/builds.md"], recentInWay(b, line([])))[1]).toBe(
      "in the way: what main has landed",
    );
    expect(conflictLines("release", ["x"], [])[2]).toBe(
      "next: take release into the worktree, resolve and commit, then sluice submit release",
    );
  });

  test("the turns name the files, what they are contested with, and what to do", () => {
    expect(conflictText("main", landed.conflict)).toBe(
      "[sluice] Your work would conflict with what main has landed, in docs/builds.md. " +
        "Merge main into your worktree now and resolve it while it is small (check with `sluice submit --check main`).",
    );
    expect(conflictText("main", queued.conflict)).toContain(
      "with #217 (bolt/runner), in docs/builds.md, docs/identity.md.",
    );
    expect(forecastToast("main", queued.conflict)).toBe(
      "sluice: your work would conflict with main in docs/builds.md +1",
    );
    expect(forecastWords(queued.conflict)).toBe("your work would conflict · docs/builds.md +1");
    expect(forecastWords(landed.conflict)).toBe("your work would conflict · docs/builds.md");
    expect(clearText("main", null)).toBe(
      "[sluice] Your work no longer conflicts with main or anything queued.",
    );
    expect(clearText("main", { id: 220, branch: "main", conflict: true, active: false })).toBe(
      "[sluice] Your work no longer conflicts with main or anything queued. #220 can be submitted again: `sluice submit main`.",
    );
    expect(clearText("main", { id: 220, branch: "main", conflict: false, active: false })).toBe(
      "[sluice] Your work no longer conflicts with main or anything queued.",
    );
  });
});
