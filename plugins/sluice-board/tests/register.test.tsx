import type { On } from "claude-code";
import type { Engine } from "claude-code/testing";
import { expect, mock, test } from "claude-code/testing";

import type { Overview } from "../types";
import { NOW, overview, withChecks, withRelease, withTasks } from "./fixture";

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

/**
 * A host where sluice answers from `docs`, streamed by --watch in turn, and
 * the session was started with the variables `env`.
 */
function host(
  on: On,
  docs: Overview[],
  seen: { prompts: string[]; toasts: string[]; status: (string | undefined)[] },
  env: Record<string, string> = {},
) {
  mock.env(on, env);
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
const INLINE = { title: "sluice", isFocused: true, bodyColumns: 100, placement: "inline" } as never;

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
  // Plain, never the yellow of a combination under test.
  expect(running?.props.color).toBeUndefined();
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

test("a branch named by SLUICE_BRANCH on the status line, and first in the band and the pane", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const seen = {
    prompts: [] as string[],
    toasts: [] as string[],
    status: [] as (string | undefined)[],
  };
  const d = withRelease();
  d.branches[0]!.depth = 1;
  d.branches[0]!.submissions = [
    { id: 20, state: "queued", from: { agent: "claude", worktree: "w", path: "/work/sluice/w" } },
  ];
  host(on, [d], seen, { SLUICE_BRANCH: "release" });
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  expect(seen.status).toContain("sluice: release · 3 queued · #12 testing 3/8 · 2 checking");
  const band = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "AbovePrompt",
    props: BAND,
  });
  const lines = await band.findAll({ type: "Text", text: /^sluice / });
  expect(lines.map((l) => l.text.split(" · ")[0])).toEqual(["sluice release", "sluice main"]);
  await band.unmount();
  const pane = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props: PANE,
    requestId: "sluice",
  });
  const heads = await pane.findAll({ type: "Text", text: /^\S+ at / });
  expect(heads.map((h) => h.text.split(" ")[0])).toEqual(["release", "main"]);
  await pane.unmount();
});

test("the pane shows a named branch first even when idle", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const d = overview();
  const main = d.branches[0]!;
  const idle = { ...main, branch: "bolt/a", depth: 0, submissions: [], candidates: [], recent: [] };
  d.branches = [{ ...main, branch: "release" }, idle, main];
  host(on, [d], { prompts: [], toasts: [], status: [] }, { SLUICE_BRANCH: "bolt/a" });
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
  expect(heads.map((h) => h.text.split(" ")[0])).toEqual(["bolt/a", "release", "main"]);
  expect(await pane.find({ type: "Text", text: /with nothing queued/ })).toBeUndefined();
  await pane.unmount();
});

test("the pane names a branch it has no records of", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  host(on, [overview()], { prompts: [], toasts: [], status: [] }, { SLUICE_BRANCH: "hotfix" });
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  const pane = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props: PANE,
    requestId: "sluice",
  });
  const heads = await pane.findAll({ type: "Text", text: /^\S+ (at|·) / });
  expect(heads.map((h) => h.text)).toEqual([
    "hotfix · no records",
    expect.stringMatching(/^main at /),
  ]);
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

const GONE = {
  error: { kind: "internal", message: "reading the records of main: it is gone", next: [] },
};

test("an overview's error is one line in the band, from --watch or a poll, until a document comes", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  // The watch prints a document, then fails; the poll answers `once`.
  let once = { exitCode: 0, stdout: JSON.stringify(overview()) };
  on("session.start", ($, e) => ({ cwd: e.cwd }));
  on("command.register", ($, e) => ({ value: { command: e.name } }));
  on("process.run", ($, e) => {
    const argv = e.argv.join(" ");
    if (argv === "sluice overview --json") return { value: { ...once, stderr: "", ...WHOLE } };
    if (argv === "git rev-parse --show-toplevel")
      return { value: { exitCode: 0, stdout: `${ROOT}\n`, stderr: "", ...WHOLE } };
    return { value: { exitCode: 1, stdout: "", stderr: `no ${argv}`, ...WHOLE } };
  });
  on("process.spawn", async function* () {
    yield { stream: "stdout" as const, text: `${JSON.stringify(overview())}\n` };
    yield { stream: "stdout" as const, text: `${JSON.stringify(GONE)}\n` };
    return { value: { code: 3, signal: null } };
  });
  on("ui.status", () => ({ value: undefined }));
  const band = async () => {
    const b = await $.ui.mount({
      plugin: "sluice-board",
      surface: "terminal",
      component: "AbovePrompt",
      props: BAND,
    });
    const texts = (await b.findAll({ type: "Text" })).map((t) => t.text);
    await b.unmount();
    return texts;
  };
  const QUEUE = /^sluice main · 3 submissions/;

  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  expect(await band()).toEqual(["sluice: reading the records of main: it is gone"]);

  await clock.advance(3000);
  expect((await band()).some((t) => QUEUE.test(t))).toBe(true);

  once = { exitCode: 3, stdout: JSON.stringify(GONE) };
  await clock.advance(3000);
  expect(await band()).toEqual(["sluice: reading the records of main: it is gone"]);

  // A poll that prints nothing it can read leaves the band as it was.
  once = { exitCode: 3, stdout: "" };
  await clock.advance(3000);
  expect(await band()).toEqual(["sluice: reading the records of main: it is gone"]);
});

/** The engine's record of the open panes, and of each open the board asked for. */
function panes(on: On) {
  const open = new Set<string>();
  const opens: { closeOnEscape?: true; focus?: true }[] = [];
  const closes: string[] = [];
  on("ui.open", ($, e) => {
    opens.push(e);
    open.add(e.id);
    return { value: { isPlaced: true as const } };
  });
  on("ui.close", ($, e) => {
    closes.push(e.id);
    open.delete(e.id);
    return { value: undefined };
  });
  on("ui.panes", () => ({
    value: [...open].map((id) => ({
      id,
      title: id,
      isShown: true,
      isFocused: false,
      isPlaced: true,
    })),
  }));
  return { open, opens, closes };
}

/** `/sluice` typed in the composer, full-screen or not. */
function slash(isFullscreen: boolean) {
  return {
    command: "sluice",
    args: "",
    origin: { kind: "composer" as const },
    presentation: { isFullscreen, columns: isFullscreen ? 200 : 100 },
  };
}

test("/sluice opens the pane to close on Escape, and closes it when open", async ($, on) => {
  host(on, [overview()], nothingSeen());
  const { open, opens } = panes(on);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });

  expect((await $.command.run(slash(true))).text).toBe(
    "sluice's queue is open in a pane. Its close mark or /sluice again closes it.",
  );
  expect([...open]).toEqual(["sluice"]);
  expect(opens.map((o) => o.closeOnEscape)).toEqual([true]);
  // Docked, the prompt keeps the keyboard.
  expect(opens.map((o) => o.focus)).toEqual([undefined]);
  expect((await $.command.run(slash(true))).text).toBe("sluice's pane is closed.");
  expect([...open]).toEqual([]);
  await $.command.run(slash(true));
  expect([...open]).toEqual(["sluice"]);
  expect(opens.length).toBe(2);
});

test("/sluice above the prompt asks for the keyboard and says how to close the pane", async ($, on) => {
  host(on, [overview()], nothingSeen());
  const { open, opens } = panes(on);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });

  expect((await $.command.run(slash(false))).text).toBe(
    "sluice's queue is open above the prompt. Esc, its close button or /sluice again closes it.",
  );
  expect([...open]).toEqual(["sluice"]);
  expect(opens.map((o) => [o.focus, o.closeOnEscape])).toEqual([[true, true]]);
  expect((await $.command.run(slash(false))).text).toBe("sluice's pane is closed.");
  expect([...open]).toEqual([]);
});

test("a pane above the prompt has a close button, a docked one none of the board's", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  host(on, [overview()], nothingSeen());
  const { closes } = panes(on);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();

  const inline = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props: INLINE,
    requestId: "sluice",
  });
  const button = await inline.find({ type: "Button", key: "close" });
  expect(button?.props).toMatchObject({ role: "dismiss", hotkey: "x", label: "close" });
  await inline.press({ key: "close" });
  expect(closes).toEqual(["sluice"]);
  await inline.unmount();

  const docked = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props: PANE,
    requestId: "sluice",
  });
  expect(await docked.find({ key: "close" })).toBeUndefined();
  await docked.unmount();
});

/** main with #3 queued and nothing else: a repository just set up, with one submission. */
function justSetUp(): Overview {
  const d = overview();
  d.branches = [
    {
      ...d.branches[0]!,
      depth: 1,
      candidates: [],
      recent: [],
      submissions: [{ id: 3, state: "queued", from: { agent: "claude", worktree: "w3" } }],
    },
  ];
  return d;
}

/** What one `sluice overview --json` answers: an overview, a failed run, or no sluice to start. */
type Read = Overview | { exitCode: number; stderr: string } | "missing";

/**
 * A host whose `sluice overview --json` answers each read with the next of
 * `reads`, the last again once they run out, and whose watch streams the
 * last overview read; it counts the reads and the watches.
 */
function readsHost(on: On, reads: Read[]) {
  const n = { reads: 0, watches: 0 };
  let last: Overview | null = null;
  on("session.start", ($, e) => ({ cwd: e.cwd }));
  on("command.register", ($, e) => ({ value: { command: e.name } }));
  on("process.run", ($, e, next) => {
    const argv = e.argv.join(" ");
    if (argv !== "sluice overview --json")
      return { value: { exitCode: 1, stdout: "", stderr: `no ${argv}`, ...WHOLE } };
    const r = reads[Math.min(n.reads, reads.length - 1)]!;
    n.reads += 1;
    // Nothing beneath answers: the run rejects, as for a command not found.
    if (r === "missing") return next(e);
    if ("schema" in r) {
      last = r;
      return { value: { exitCode: 0, stdout: JSON.stringify(r), stderr: "", ...WHOLE } };
    }
    return { value: { exitCode: r.exitCode, stdout: "", stderr: r.stderr, ...WHOLE } };
  });
  on("process.spawn", async function* () {
    n.watches += 1;
    if (last) yield { stream: "stdout" as const, text: `${JSON.stringify(last)}\n` };
    return { value: { code: 0, signal: null } };
  });
  on("ui.status", () => ({ value: undefined }));
  on("ui.toast", () => ({ value: undefined }));
  return n;
}

test("/sluice reads sluice again where the session could not, and follows it once it can", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const n = readsHost(on, [{ exitCode: 3, stderr: "sluice: no policy here\n" }, justSetUp()]);
  panes(on);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  expect(n.reads).toBe(1);
  expect(n.watches).toBe(0);

  await $.command.run(slash(true));
  await clock.settle();
  expect(n.reads).toBe(2);
  const pane = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props: PANE,
    requestId: "sluice",
  });
  const heads = await pane.findAll({ type: "Text", text: /^(TESTING|QUEUED|RECENT)$/ });
  expect(heads.map((h) => h.text)).toEqual(["QUEUED"]);
  expect(await pane.find({ type: "Text", text: /#3 queued\s+w3 claude/ })).toBeDefined();
  await pane.unmount();
  const band = await $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "AbovePrompt",
    props: BAND,
  });
  expect(
    await band.find({ type: "Text", text: "sluice main · 1 submission · #3 waiting" }),
  ).toBeDefined();
  await band.unmount();
});

/** The pane as `/sluice` drew it, docked unless `props` says otherwise. */
function mountPane($: Engine, props = PANE) {
  return $.ui.mount({
    plugin: "sluice-board",
    surface: "terminal",
    component: "Pane",
    props,
    requestId: "sluice",
  });
}

const ASK = "Ask Claude to set sluice up in this repository.";

test("/sluice reads sluice again where it still cannot be started, and says so", async ($, on) => {
  const n = readsHost(on, ["missing"]);
  panes(on);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await $.command.run(slash(false));
  expect(n.reads).toBe(2);
  expect(n.watches).toBe(0);
  const pane = await mountPane($, INLINE);
  expect(
    await pane.find({
      type: "Text",
      text: "sluice could not be read here: the sluice command was not found",
    }),
  ).toBeDefined();
  expect(await pane.find({ type: "Text", text: ASK })).toBeDefined();
  expect(await pane.find({ type: "Button", key: "close" })).toBeDefined();
  await pane.unmount();
});

test("a pane whose sluice overview failed gives sluice's words and what to do", async ($, on) => {
  readsHost(on, [{ exitCode: 3, stderr: "sluice: not a git repository\n" }]);
  panes(on);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await $.command.run(slash(true));
  const pane = await mountPane($);
  const lines = await pane.findAll({ type: "Text" });
  expect(lines.map((l) => l.text)).toEqual([
    "sluice could not be read here: sluice: not a git repository",
    ASK,
  ]);
  await pane.unmount();
});

test("a pane with no branch says how a submission gets there", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const none = overview();
  none.branches = [];
  readsHost(on, [none]);
  panes(on);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  await $.command.run(slash(true));
  const pane = await mountPane($);
  expect(await pane.find({ type: "Text", text: "Nothing submitted yet." })).toBeDefined();
  expect(
    await pane.find({
      type: "Text",
      text: "When an agent in a worktree runs sluice submit <branch>, its submission shows here until it lands.",
    }),
  ).toBeDefined();
  expect(await pane.find({ type: "Text", text: "No branch has a queue." })).toBeUndefined();
  await pane.unmount();
});

test("a pane whose branches have nothing to show says no branch has a queue", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const quiet = overview();
  quiet.branches[0] = {
    ...quiet.branches[0]!,
    depth: 0,
    candidates: [],
    submissions: [],
    recent: [{ schema: "sluice/log@1", at: "2026-10-16T19:20:00Z", event: "landed", ids: [11] }],
  };
  readsHost(on, [quiet]);
  panes(on);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  const pane = await mountPane($);
  expect(await pane.find({ type: "Text", text: "No branch has a queue." })).toBeDefined();
  expect(await pane.find({ type: "Text", text: "Nothing submitted yet." })).toBeUndefined();
  await pane.unmount();
});

test("a named branch with no records keeps its line, and no 'Nothing submitted yet'", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  mock.env(on, { SLUICE_BRANCH: "hotfix" });
  const none = overview();
  none.branches = [];
  readsHost(on, [none]);
  panes(on);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  const pane = await mountPane($);
  expect(await pane.find({ type: "Text", text: "hotfix · no records" })).toBeDefined();
  expect(await pane.find({ type: "Text", text: "No branch has a queue." })).toBeDefined();
  expect(await pane.find({ type: "Text", text: "Nothing submitted yet." })).toBeUndefined();
  await pane.unmount();
});

test("two good reads of sluice start one watch between them", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const n = readsHost(on, ["missing", justSetUp()]);
  panes(on);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  // Two /sluice at once, each finding no overview yet, each reading it.
  await Promise.all([$.command.run(slash(true)), $.command.run(slash(true))]);
  await clock.settle();
  expect(n.reads).toBe(3);
  expect(n.watches).toBe(1);
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

/** What a forecast host answers, changed as a test goes. */
type World = { doc: Overview; head: string; answer: object; forecasts: number };

/** The fixture with #13, this session's, gone from main's queue. */
function free(at = "9ecc34a1b2"): Overview {
  const d = overview();
  d.branches[0]!.at = at;
  d.branches[0]!.submissions = d.branches[0]!.submissions.filter((s) => s.id !== 13);
  return d;
}

const CLEAR = { conflict: null, head: "h1", onto: "9ecc34a1b2", ahead: [12, 14] };
const LANDED = { conflict: { paths: ["docs/builds.md"], with: [] } };
const WITH_217 = {
  conflict: {
    paths: ["docs/builds.md"],
    with: [{ id: 217, from: { agent: "runner", worktree: "bolt/runner" } }],
  },
};

/**
 * A host whose overview is `w.doc`, read by a poll every 3 s (the watch
 * ends at once), whose worktree is 4 commits ahead at `w.head`, and whose
 * forecast says `w.answer`.
 */
function forecastHost(
  on: On,
  w: World,
  seen: { prompts: string[]; toasts: string[]; status: (string | undefined)[] },
) {
  on("session.start", ($, e) => ({ cwd: e.cwd }));
  on("command.register", ($, e) => ({ value: { command: e.name } }));
  on("process.run", ($, e) => {
    const argv = e.argv.join(" ");
    const out = (stdout: string) => ({ value: { exitCode: 0, stdout, stderr: "", ...WHOLE } });
    if (argv === "git rev-parse --show-toplevel") return out(`${ROOT}\n`);
    if (argv === "sluice overview --json") return out(JSON.stringify(w.doc));
    if (argv === "git rev-parse HEAD") return out(`${w.head}\n`);
    if (argv.startsWith("git rev-list --count ")) return out("4\n");
    if (argv === "sluice submit main --check --json") {
      w.forecasts += 1;
      return out(JSON.stringify(w.answer));
    }
    return { value: { exitCode: 1, stdout: "", stderr: `no ${argv}`, ...WHOLE } };
  });
  on("process.spawn", async function* () {
    // The watch ends at once, so the board polls.
    yield* [];
    return { value: { code: 1, signal: null } };
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

function nothingSeen() {
  return { prompts: [] as string[], toasts: [] as string[], status: [] as (string | undefined)[] };
}

test("main landing a change to the same lines starts one turn, once, a reload included", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const w: World = { doc: free(), head: "h1", answer: CLEAR, forecasts: 0 };
  const seen = nothingSeen();
  forecastHost(on, w, seen);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  expect(w.forecasts).toBe(1);
  expect(seen.prompts).toEqual([]);

  w.doc = free("b2c3d4e5f6");
  w.answer = LANDED;
  await clock.advance(3000);
  expect(w.forecasts).toBe(2);
  expect(seen.prompts).toEqual([
    "[sluice] Your work would conflict with what main has landed, in docs/builds.md. " +
      "Merge main into your worktree now and resolve it while it is small (check with `sluice submit --check main`).",
  ]);
  expect(seen.toasts).toContain("sluice: your work would conflict with main in docs/builds.md");

  // Nothing it rests on moved: no forecast.
  await clock.advance(3000);
  expect(w.forecasts).toBe(2);

  // A reload tells nothing again.
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.advance(3000);
  expect(seen.prompts).toHaveLength(1);
});

test("a conflict with a queued submission names it, and its failing clears it once", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  // #220, this session's, waits on main: no forecast while it does.
  const d0 = free();
  d0.branches[0]!.submissions.push({
    id: 220,
    state: "queued",
    from: { agent: "claude", worktree: "design-runner", path: ROOT },
  });
  const w: World = { doc: d0, head: "h1", answer: CLEAR, forecasts: 0 };
  const seen = nothingSeen();
  forecastHost(on, w, seen);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  expect(w.forecasts).toBe(0);

  // #220 is removed for a conflict with #217, queued ahead of it.
  const d1 = free();
  d1.branches[0]!.submissions.push({
    id: 217,
    state: "testing",
    from: { agent: "runner", worktree: "bolt/runner" },
  });
  d1.branches[0]!.recent.push({
    schema: "sluice/log@1",
    at: "2026-10-16T19:45:00Z",
    event: "conflict",
    id: 220,
    paths: ["docs/builds.md"],
  });
  w.doc = d1;
  w.answer = WITH_217;
  await clock.advance(3000);
  expect(seen.prompts.some((p) => p.includes("with #217 (bolt/runner), in docs/builds.md"))).toBe(
    true,
  );
  const told = seen.prompts.length;

  // #217 fails; the head has not moved.
  const d2 = free();
  d2.branches[0]!.recent = d1.branches[0]!.recent;
  w.doc = d2;
  w.answer = CLEAR;
  await clock.advance(3000);
  expect(seen.prompts.slice(told)).toEqual([
    "[sluice] Your work no longer conflicts with main or anything queued. #220 can be submitted again: `sluice submit main`.",
  ]);
  await clock.advance(3000);
  expect(seen.prompts).toHaveLength(told + 1);
});

test("a conflict cleared by the agent's own merge starts no turn", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const w: World = { doc: free(), head: "h1", answer: CLEAR, forecasts: 0 };
  const seen = nothingSeen();
  forecastHost(on, w, seen);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  w.doc = free("b2c3d4e5f6");
  w.answer = LANDED;
  await clock.advance(3000);
  expect(seen.prompts).toHaveLength(1);

  w.head = "h2";
  w.answer = CLEAR;
  await clock.advance(3000);
  expect(w.forecasts).toBe(3);
  expect(seen.prompts).toHaveLength(1);
});

test("a conflict there when the session starts is a toast, not a turn", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  const w: World = { doc: free(), head: "h1", answer: LANDED, forecasts: 0 };
  const seen = nothingSeen();
  forecastHost(on, w, seen);
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  expect(w.forecasts).toBe(1);
  expect(seen.prompts).toEqual([]);
  expect(seen.toasts).toContain("sluice: your work would conflict with main in docs/builds.md");
});

test("SLUICE_FORECAST=0 turns forecasting off", async ($, on) => {
  const clock = mock.clock(on);
  await clock.set(NOW);
  mock.env(on, { SLUICE_FORECAST: "0" });
  const w: World = { doc: free(), head: "h1", answer: LANDED, forecasts: 0 };
  forecastHost(on, w, nothingSeen());
  await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
  await clock.settle();
  w.doc = free("b2c3d4e5f6");
  await clock.advance(3000);
  expect(w.forecasts).toBe(0);
});

test(
  "the forecast option turns forecasting off",
  { options: { forecast: false } },
  async ($, on) => {
    const clock = mock.clock(on);
    await clock.set(NOW);
    const w: World = { doc: free(), head: "h1", answer: LANDED, forecasts: 0 };
    forecastHost(on, w, nothingSeen());
    await $.session.start({ cwd: ROOT, surface: "terminal", isInteractive: true });
    await clock.settle();
    w.doc = free("b2c3d4e5f6");
    await clock.advance(3000);
    expect(w.forecasts).toBe(0);
  },
);
