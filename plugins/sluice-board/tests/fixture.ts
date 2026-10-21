import type { Overview, OverviewTask } from "../types";

/** 2026-10-16T19:45:10Z: 130 s after the combination under test started. */
export const NOW = Date.parse("2026-10-16T19:45:10Z");

/** A repository with main testing #12 + #13, #14 queued, and a recent blame. */
export function overview(): Overview {
  return {
    schema: "sluice/overview@1",
    host: "mac-studio",
    repository: "/work/sluice/sluice.git",
    at: "2026-10-16T19:45:10Z",
    branches: [
      {
        branch: "main",
        at: "9ecc34a1b2",
        version: 57,
        window: 1,
        parallel: 4,
        depth: 3,
        candidates: [
          {
            id: "c41",
            submissions: [12, 13],
            revision: "e20a6f9b48",
            state: "testing",
            started: "2026-10-16T19:43:00Z",
            limit: 900,
            targets: ["#check-fast", "#check-tools"],
            executor: "sandbox",
            ran: 0,
            replayed: 0,
            failed: [],
          },
        ],
        submissions: [
          {
            id: 12,
            state: "testing",
            from: {
              agent: "claude",
              worktree: "overview-docs",
              path: "/work/sluice/overview-docs",
            },
          },
          {
            id: 13,
            state: "testing",
            from: { agent: "claude", worktree: "plugin-board", path: "/work/sluice/plugin-board" },
          },
          { id: 14, state: "queued", from: { agent: "herdr", worktree: "pull/41" } },
        ],
        recent: [
          {
            schema: "sluice/log@1",
            at: "2026-10-16T19:41:00Z",
            event: "landed",
            ids: [11],
            to: "9ecc34a1b2",
          },
          {
            schema: "sluice/log@1",
            at: "2026-10-16T19:42:00Z",
            event: "blamed",
            id: 9,
            combination: "4f1c2aa0e1",
          },
        ],
      },
    ],
    sessions: [],
    previews: [],
  };
}

/** The same, with main's combination 3 of its 5 tasks through, as moon says. */
export function withTasks(): Overview {
  const doc = overview();
  doc.branches[0]!.candidates[0]!.tasks = [
    {
      target: "sluice:tools",
      state: "passed",
      started: "2026-10-16T19:42:30Z",
      ended: "2026-10-16T19:45:00Z",
      exit: 0,
    },
    {
      target: "sluice:test",
      state: "passed",
      started: "2026-10-16T19:43:00Z",
      ended: "2026-10-16T19:44:48Z",
      exit: 0,
    },
    { target: "core:test", state: "waiting" },
    { target: "sluice:e2e", state: "running" },
    { target: "site:prose", state: "replayed", ended: "2026-10-16T19:44:54Z" },
  ];
  return doc;
}

/** `n` finished tasks of a check, the first `failed` of them failed. */
function finished(n: number, failed = 0): OverviewTask[] {
  return Array.from({ length: n }, (_, i) => ({
    target: `t:${i}`,
    state: i < failed ? ("failed" as const) : ("passed" as const),
  }));
}

/**
 * The fixture with agents' checks: main tests one combination of a window
 * of 3 with #14 waiting, and the host's last run of 2 goes to an agent's
 * checks, so the queue is held back. Two checks go, three ended this
 * quarter hour, one ended long ago, and one worktree is only warned of.
 */
export function withChecks(): Overview {
  const doc = overview();
  doc.branches[0]!.window = 3;
  doc.room = { bound: 2, queue: 1, sessions: 1, reserve: 0 };
  const ports = { web: "4001" };
  const tasks: OverviewTask[] = [
    ...finished(3),
    { target: "t:3", state: "running" },
    ...Array.from({ length: 4 }, (_, i) => ({ target: `t:${4 + i}`, state: "waiting" as const })),
  ];
  const result = (at: string, outcome: "pass" | "fail", more: object) => ({
    at,
    head: "e20a6f9b48",
    kind: "checks" as const,
    outcome,
    recorded: true,
    ...more,
  });
  doc.sessions = [
    {
      worktree: "/work/sluice/fix-cards",
      branch: "main",
      ports,
      run: { started: "2026-10-16T19:44:00Z", tasks },
    },
    {
      worktree: "/work/sluice/e2e-stack",
      branch: "main",
      ports,
      run: { started: "2026-10-16T19:43:10Z", command: "make e2e" },
    },
    {
      worktree: "/work/sluice/done-ok",
      branch: "main",
      ports,
      last_result: result("2026-10-16T19:43:10Z", "pass", {
        tasks: finished(14),
        reused: true,
      }),
    },
    {
      worktree: "/work/sluice/dirty",
      branch: "main",
      ports,
      last_result: result("2026-10-16T19:44:10Z", "pass", {
        recorded: false,
        tasks: finished(14),
        reused: false,
      }),
    },
    {
      worktree: "/work/sluice/broke",
      branch: "main",
      ports,
      last_result: result("2026-10-16T19:44:30Z", "fail", {
        tasks: finished(14, 2),
        reused: false,
      }),
    },
    {
      worktree: "/work/sluice/long-ago",
      branch: "main",
      ports,
      last_result: result("2026-10-16T19:20:00Z", "pass", { tasks: finished(5), reused: true }),
    },
    { worktree: "/work/sluice", branch: "main", holds_queued_branch: "main" },
  ];
  return doc;
}

/**
 * main idle, and release, whose submissions come from other worktrees, as
 * an orchestrator's agents make them: #12 under test with 3 of its 8 tasks
 * ended, #13 to #15 queued, and two agents' checks held to it. The herdr
 * plugin's tests name the same branch and expect the same words.
 */
export function withRelease(): Overview {
  const doc = overview();
  const main = doc.branches[0]!;
  const task = (i: number, state: OverviewTask["state"]): OverviewTask => ({
    target: `t${i}:test`,
    state,
  });
  doc.branches = [
    { ...main, depth: 0, candidates: [], submissions: [], recent: [] },
    {
      branch: "release",
      at: "9ecc34a1b2",
      version: 3,
      window: 1,
      parallel: 1,
      depth: 4,
      candidates: [
        {
          id: "c9",
          submissions: [12],
          revision: "x",
          state: "testing",
          started: "2026-10-16T19:42:00Z",
          limit: 900,
          targets: ["#check-fast"],
          executor: "sandbox",
          ran: 0,
          replayed: 0,
          failed: [],
          tasks: [0, 1, 2, 3, 4, 5, 6, 7].map((i) => task(i, i < 3 ? "passed" : "waiting")),
        },
      ],
      submissions: [12, 13, 14, 15].map((id) => ({
        id,
        state: id === 12 ? "testing" : "queued",
        from: { agent: "unit", worktree: `u${id}`, path: `/elsewhere/u${id}` },
      })),
      recent: [],
    },
  ];
  doc.sessions = [1, 2].map((i) => ({
    worktree: `/elsewhere/u${i}`,
    branch: "release",
    run: { started: "2026-10-16T19:44:00Z" },
  }));
  return doc;
}
