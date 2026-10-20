import type { On } from "claude-code";
import { expect, mock, test } from "claude-code/testing";

import type { Overview } from "../types";
import { NOW, overview, withChecks, withTasks } from "./fixture";

const ROOT = "/work/sluice/plugin-board";
const WHOLE = { isStdoutTruncated: false, isStderrTruncated: false };
const LOG = "/work/sluice/sluice.git/sluice/.logs/main/c41/cli:test.log";

/** The fixture, then #13 blamed. */
function blamed(): Overview {
  const d = overview();
  d.branches[0]!.candidates = [];
  d.branches[0]!.submissions = d.branches[0]!.submissions.filter((s) => s.id !== 13);
  d.branches[0]!.depth = 2;
  d.branches[0]!.recent.push({
    schema: "sluice/log@1",
    at: "2026-10-16T19:50:00Z",
    event: "blamed",
    id: 13,
    combination: "e20a6f9b48",
  });
  return d;
}

/** Every command the host ran, in order. */
const ran: string[] = [];

/** The fixture, with #9's blame and its failed test kept on a combination. */
function blamedWithLog(log: string): Overview {
  const d = overview();
  d.branches[0]!.candidates.push({
    id: "c40",
    submissions: [9],
    revision: "4f1c2aa0e1",
    state: "failed",
    targets: ["#check-fast"],
    executor: "sandbox",
    ran: 1,
    replayed: 0,
    failed: [{ task: "cli:test", log }],
  });
  return d;
}

/** A host where sluice answers from `docs`, streamed by --watch in turn. */
function host(
  on: On,
  docs: Overview[],
  seen: { prompts: string[]; toasts: string[]; status: (string | undefined)[] },
) {
  on("session.start", ($, e) => ({ cwd: e.cwd }));
  on("command.register", ($, e) => ({ value: { command: e.name } }));
  on("process.run", ($, e) => {
    const argv = e.argv.join(" ");
    ran.push(argv);
    if (argv === `tail -n 20 -- ${LOG}`)
      return { value: { exitCode: 0, stdout: "first\nlast line\n", stderr: "", ...WHOLE } };
    if (argv === "sluice status 9 --json") {
      return {
        value: {
          exitCode: 0,
          stdout: JSON.stringify({ id: 9, failed: [{ task: "cli:test", log: LOG }] }),
          stderr: "",
          ...WHOLE,
        },
      };
    }
    if (argv.startsWith("tail -n 20 -- "))
      return {
        value: { exitCode: 1, stdout: "", stderr: "tail: No such file or directory\n", ...WHOLE },
      };
    if (argv === "git remote get-url origin") {
      return {
        value: {
          exitCode: 0,
          stdout: "git@github.com:agentplot/sluice.git\n",
          stderr: "",
          ...WHOLE,
        },
      };
    }
    if (argv === "git rev-parse --show-toplevel")
      return { value: { exitCode: 0, stdout: `${ROOT}\n`, stderr: "", ...WHOLE } };
    if (argv === "sluice overview --json")
      return { value: { exitCode: 0, stdout: JSON.stringify(docs[0]), stderr: "", ...WHOLE } };
    if (argv === "sluice status 13 --json") {
      return {
        value: {
          exitCode: 0,
          stdout: JSON.stringify({ id: 13, failed: [{ task: "cli:test", log: LOG }] }),
          stderr: "",
          ...WHOLE,
        },
      };
    }
    return { value: { exitCode: 1, stdout: "", stderr: `no ${argv}`, ...WHOLE } };
  });
  on("process.spawn", async function* () {
    for (const d of docs) yield { stream: "stdout" as const, text: `${JSON.stringify(d)}\n` };
    return { value: { code: 0, signal: null } };
  });
  on("ui.status", ($, e) => {
    seen.status.push(e.text);
    return { value: undefined };
  });
  on("ui.toast", ($, e) => {
    seen.toasts.push(e.text);
    return { value: undefined };
  });
  on("ui.notify", () => ({ value: { isSent: true as const, channel: "terminal_bell" } }));
  on("prompt.submit", ($, e) => {
    seen.prompts.push(e.text);
    return { text: e.text };
  });
}

const BAND = { hasSurvey: false, isWorking: false, maxRows: 4, bodyColumns: 120 } as never;
const PANE = { title: "sluice", isFocused: false, bodyColumns: 80, placement: "dock" } as never;

test("the band and the pane show the queue on every surface", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const seen = {
    prompts: [] as string[],
    toasts: [] as string[],
    status: [] as (string | undefined)[],
  };
  host(on, [overview()], seen);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  expect(seen.status).toContain("sluice: 3 submissions · yours #13 testing");

  for (const surface of ["terminal", "desktop"] as const) {
    const band = await $.ui.mount({
      plugin: "sluice-board",
      surface,
      component: "AbovePrompt",
      props: BAND,
    });
    expect(
      await band.find({
        type: "Text",
        text: "sluice main · 3 submissions · testing main+#12+#13 2m10s/15m · #14 waiting · last #9 blamed on 4f1c2aa",
      }),
    ).toBeDefined();
    await band.unmount();

    const pane = await $.ui.mount({
      plugin: "sluice-board",
      surface,
      component: "Pane",
      props: PANE,
      requestId: "sluice",
    });
    expect(
      await pane.find({ type: "Text", text: /#13 testing\s+plugin-board claude\s+← you/ }),
    ).toBeDefined();
    expect(
      await pane.find({
        type: "Text",
        text: /3 submissions · 1 combination under test \(up to 4 at once\)/,
      }),
    ).toBeDefined();
    expect(
      await pane.find({ type: "Text", text: /▶ testing main\+#12\+#13\s+2m10s \/ 15m/ }),
    ).toBeDefined();
    // Headed as the herdr plugin's popup is.
    const heads = await pane.findAll({ type: "Text", text: /^(TESTING|QUEUED|RECENT)$/ });
    expect(heads.map((h) => h.text)).toEqual(["TESTING", "QUEUED", "RECENT"]);
    const link = await pane.find({ type: "Link" });
    expect(link?.props.href).toBe("https://github.com/agentplot/sluice/pull/41");
    await pane.unmount();
  }
});

test("the pane leaves out branches with nothing queued and no event this quarter hour", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const d = overview();
  const idle = (branch: string, recent: Overview["branches"][number]["recent"]) => ({
    ...d.branches[0]!,
    branch,
    depth: 0,
    submissions: [],
    candidates: [],
    recent,
  });
  for (let i = 0; i < 30; i++) d.branches.push(idle(`bolt/b${i}`, []));
  const landed = (at: string) => [
    { schema: "sluice/log@1", at, event: "landed", ids: [77], to: "abcdef0" },
  ];
  d.branches.push(idle("bolt/done", landed("2026-10-16T19:40:00Z")));
  d.branches.push(idle("bolt/old", landed("2026-10-16T19:20:00Z")));
  host(on, [d], { prompts: [], toasts: [], status: [] });
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  const pane = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props: PANE,
    requestId: "sluice",
  });
  expect(await pane.find({ type: "Text", text: /^main at / })).toBeDefined();
  expect(await pane.find({ type: "Text", text: /^bolt\/done at / })).toBeDefined();
  expect(await pane.findAll({ type: "Text", text: /bolt\/(b\d|old)/ })).toEqual([]);
  expect(
    await pane.find({ type: "Text", text: "31 other branches with nothing queued" }),
  ).toBeDefined();
  await pane.unmount();
});

test("a combination's tasks: the count and bar in the band, the list in the pane alone", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const seen = {
    prompts: [] as string[],
    toasts: [] as string[],
    status: [] as (string | undefined)[],
  };
  host(on, [withTasks()], seen);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();

  const band = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "AbovePrompt",
    props: BAND,
  });
  expect(
    await band.find({ type: "Text", text: /testing main\+#12\+#13 3\/5 ▓▓▓▓▓▓░░░░ 2m10s\/15m/ }),
  ).toBeDefined();
  expect(await band.find({ type: "Text", text: /sluice:e2e/ })).toBeUndefined();
  await band.unmount();

  const pane = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props: PANE,
    requestId: "sluice",
  });
  expect(
    await pane.find({
      type: "Text",
      text: /▶ testing main\+#12\+#13\s+3\/5 ▓▓▓▓▓▓░░░░\s+2m10s \/ 15m/,
    }),
  ).toBeDefined();
  expect(await pane.find({ type: "Text", text: /✓ sluice:test\s+1m48s/ })).toBeDefined();
  expect(await pane.find({ type: "Text", text: /… sluice:e2e\s+running/ })).toBeDefined();
  expect(await pane.find({ type: "Text", text: /· core:test\s+waiting/ })).toBeDefined();
  await pane.unmount();
});

test("agents' checks: the clause in the band, CHECKING and the host line in the pane", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const seen = {
    prompts: [] as string[],
    toasts: [] as string[],
    status: [] as (string | undefined)[],
  };
  host(on, [withChecks()], seen);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  expect(seen.status).toContain(
    "sluice: 3 submissions · yours #13 testing · 1 slot to an agent's checks",
  );

  const band = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "AbovePrompt",
    props: BAND,
  });
  expect(
    await band.find({ type: "Text", text: /#14 waiting · .* · 1 slot to an agent's checks$/ }),
  ).toBeDefined();
  expect(await band.find({ type: "Text", text: /fix-cards/ })).toBeUndefined();
  await band.unmount();

  const pane = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props: PANE,
    requestId: "sluice",
  });
  const heads = await pane.findAll({ type: "Text", text: /^(TESTING|QUEUED|RECENT|CHECKING)$/ });
  expect(heads.map((h) => h.text)).toEqual(["TESTING", "QUEUED", "RECENT", "CHECKING"]);
  const running = await pane.find({ type: "Text", text: /fix-cards → main 3\/8 ▓▓▓░░░░░░░ 1m10s/ });
  expect(running?.props.color).toBe("yellow");
  expect(await pane.find({ type: "Text", text: /e2e-stack → main make e2e 2m00s/ })).toBeDefined();
  expect(
    await pane.find({ type: "Text", text: /done-ok → main 14 passed · reused when submitted/ }),
  ).toBeDefined();
  expect(
    await pane.find({
      type: "Text",
      text: /dirty → main 14 passed · not kept: uncommitted changes/,
    }),
  ).toBeDefined();
  expect(await pane.find({ type: "Text", text: /broke → main 2 of 14 failed/ })).toBeDefined();
  expect(await pane.find({ type: "Text", text: /long-ago/ })).toBeUndefined();
  expect(
    await pane.find({ type: "Text", text: "host: 2 of 2 runs · 1 queue · 1 agent" }),
  ).toBeDefined();
  // A check is never a submission: QUEUED lists #14 alone.
  expect(await pane.findAll({ type: "Text", text: /^\s+#\d+ queued/ })).toHaveLength(1);
  // The worktree only warned of holds no ports, and is not counted held.
  expect(await pane.find({ type: "Text", text: "6 worktrees held · 0 previews" })).toBeDefined();
  await pane.unmount();
});

test("the pane keeps the overview's order of branches, and counts the idle ones", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const d = overview();
  const main = d.branches[0]!;
  const idle = { ...main, branch: "bolt/a", depth: 0, submissions: [], candidates: [], recent: [] };
  d.branches = [{ ...main, branch: "release" }, idle, main];
  host(on, [d], { prompts: [], toasts: [], status: [] });
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  const pane = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props: PANE,
    requestId: "sluice",
  });
  const heads = await pane.findAll({ type: "Text", text: /^\S+ at / });
  expect(heads.map((h) => h.text.split(" ")[0])).toEqual(["release", "main"]);
  expect(
    await pane.find({ type: "Text", text: "1 other branch with nothing queued" }),
  ).toBeDefined();
  await pane.unmount();
});

test("a blame of this session's submission starts one turn naming the test and its log", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const seen = {
    prompts: [] as string[],
    toasts: [] as string[],
    status: [] as (string | undefined)[],
  };
  host(on, [overview(), blamed()], seen);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  expect(seen.prompts.length).toBe(1);
  expect(seen.prompts[0]).toContain("#13 to main failed: cli:test on e20a6f9");
  expect(seen.prompts[0]).toContain(`Log of cli:test: ${LOG}`);
  expect(seen.toasts).toContain("sluice: #13 was blamed");
});

test("the wake can be turned off", { options: { wake: false } }, async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const seen = {
    prompts: [] as string[],
    toasts: [] as string[],
    status: [] as (string | undefined)[],
  };
  host(on, [overview(), blamed()], seen);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  expect(seen.prompts).toEqual([]);
  expect(seen.toasts).toContain("sluice: #13 was blamed");
});

test("a failure opens in place, folds, and moves no one", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const seen = {
    prompts: [] as string[],
    toasts: [] as string[],
    status: [] as (string | undefined)[],
  };
  host(on, [blamedWithLog(LOG)], seen);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  ran.length = 0;
  const pane = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props: PANE,
    requestId: "sluice",
  });
  // The blamed submission: its test, its log and the log's last lines.
  await pane.press({ key: "blamed-main-9-2026-10-16T19:42:00Z" });
  await pane.redraw();
  expect(await pane.find({ type: "Text", text: "last line" })).toBeDefined();
  expect(await pane.find({ type: "Text", text: `${LOG} [copy]` })).toBeDefined();
  await pane.press({ key: "blamed-main-9-2026-10-16T19:42:00Z" });
  await pane.redraw();
  expect(await pane.find({ type: "Text", text: "last line" })).toBeUndefined();
  // Nothing but sluice's own record and the log was read: no pane moved.
  expect(ran).toEqual(["sluice status 9 --json", `tail -n 20 -- ${LOG}`]);
  await pane.unmount();
});

test("a log it cannot read says why", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const seen = {
    prompts: [] as string[],
    toasts: [] as string[],
    status: [] as (string | undefined)[],
  };
  host(on, [blamedWithLog("/gone/cli:test.log")], seen);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  const pane = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props: PANE,
    requestId: "sluice",
  });
  await pane.press({ key: "log-main-cli:test" });
  await pane.redraw();
  expect(
    await pane.find({ type: "Text", text: /cannot read it: tail: No such file or directory/ }),
  ).toBeDefined();
  await pane.unmount();
});
