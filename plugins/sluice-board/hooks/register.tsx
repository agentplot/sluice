// sluice's queue inside a Claude Code session: a band above the prompt, a
// pane on /sluice, a status line, and a turn that tells an idle session its
// submission's outcome, or that its work has come to conflict. Everything is
// read from `sluice overview --json` (sluice/overview@1), streamed by
// `--watch` (ADR 0023), and from `sluice submit --check` for its own
// worktree: the board decides nothing and writes nothing of sluice's.

import type { EngineInterface, Register } from "claude-code";
import { atom, read, update } from "claude-code";

import type {
  Forecast,
  Forecasting,
  Mine,
  OpenedTest,
  Outcome,
  Overview,
  OverviewTask,
} from "../types";
import type { CheckRow } from "./board";
import {
  bandLines,
  checkRows,
  clearText,
  combination,
  conflictText,
  duration,
  emptyLines,
  errorLine,
  failedName,
  forecastBranch,
  forecastKey,
  forecasts,
  forecastToast,
  githubBase,
  hostLine,
  hostsLine,
  isMine,
  isOverview,
  latest,
  learn,
  limit,
  moving,
  namedFirst,
  openedText,
  outcomes,
  plural,
  progress,
  pullOf,
  recentKeys,
  recentWords,
  shown,
  signature,
  since,
  statusLine,
  submitted,
  taskLines,
  testing,
  unreadReason,
  wakeText,
} from "./board";

const PANE = "sluice";
const TASK_COLOR: Record<OverviewTask["state"], string | undefined> = {
  passed: "green",
  replayed: "green",
  failed: "red",
  running: "yellow",
  waiting: undefined,
};
// A check under way is plain, never the yellow of a combination under test.
const CHECK_COLOR: Record<CheckRow["state"], string | undefined> = {
  running: undefined,
  passed: "green",
  failed: "red",
  ended: undefined,
};
const NO_ONE: Mine = { root: null, ids: [], pulls: [] };
const NOT_YET: Forecasting = { key: null, head: null, told: null };

const doc = atom({ plugin: "sluice-board", key: "doc" } as const, null);
const mine = atom({ plugin: "sluice-board", key: "mine" } as const, NO_ONE);
const told = atom({ plugin: "sluice-board", key: "told" } as const, []);
const isHidden = atom({ plugin: "sluice-board", key: "isHidden" } as const, false);
const now = atom({ plugin: "sluice-board", key: "now" } as const, 0);
const repo = atom({ plugin: "sluice-board", key: "repo" } as const, null);
// Failures opened in place: their keys, and what each showed when opened.
const opened = atom({ plugin: "sluice-board", key: "opened" } as const, []);
const details = atom({ plugin: "sluice-board", key: "details" } as const, {});
// The branch the session works for, when what started it named one.
const named = atom({ plugin: "sluice-board", key: "named" } as const, null);
const forecasting = atom({ plugin: "sluice-board", key: "forecasting" } as const, NOT_YET);
const error = atom({ plugin: "sluice-board", key: "error" } as const, null);
// Why sluice could not be read, while the board has no overview.
const unread = atom({ plugin: "sluice-board", key: "unread" } as const, null);

type Detail = OpenedTest;

/** What a failure shows opened: each failed test's log tail, read with `tail`. */
async function open($: $, failed: { task: string; log?: string }[]): Promise<Detail[]> {
  const out: Detail[] = [];
  for (const f of failed) {
    if (!f.log) {
      out.push({ task: f.task, error: "no log was kept" });
      continue;
    }
    const got = await $.process.run(["tail", "-n", "20", "--", f.log]).catch(() => null);
    if (got && got.exitCode === 0)
      out.push({ ...f, tail: got.stdout.split("\n").filter((l) => l !== "") });
    else out.push({ ...f, error: (got?.stderr ?? "").trim() || "it could not be read" });
  }
  return out;
}

/** Open a failure in place, or fold it: it never leaves this pane. */
async function toggle($: $, key: string, failed: () => Promise<{ task: string; log?: string }[]>) {
  if ((await read($, opened)).includes(key)) {
    await update($, opened, (list) => list.filter((k) => k !== key));
    return;
  }
  const shown = await open($, await failed());
  await update($, details, (all) => ({ ...all, [key]: shown }));
  await update($, opened, (list) => [...list, key]);
}

/** The failed tests of a blamed submission, from sluice. */
async function blamedTests($: $, id: number): Promise<{ task: string; log?: string }[]> {
  const cwd = (await read($, mine)).root ?? undefined;
  const got = await $.process
    .run(["sluice", "status", String(id), "--json"], cwd ? { cwd } : undefined)
    .catch(() => null);
  try {
    return got && got.exitCode === 0 ? (JSON.parse(got.stdout).failed ?? []) : [];
  } catch {
    return [];
  }
}

type $ = EngineInterface;

/** Whether the board starts turns, and forecasts the session's worktree. */
type How = { wake: boolean; forecast: boolean };

/**
 * A new document: learn this session's submissions, tell new outcomes. An
 * error document is the band's one line, and the last document stays.
 */
async function take($: $, next: unknown, cwd: string, how: How) {
  if (!isOverview(next)) {
    const said = errorLine(next);
    if (said !== null) await update($, error, () => said);
    return;
  }
  await update($, error, () => null);
  const first = (await read($, doc)) === null;
  await update($, doc, () => next);
  const known = learn(next, await read($, mine));
  await update($, mine, () => known);
  $.ui.status(statusLine(next, known, await read($, named), await $.clock.now()));
  if (how.forecast) void forecast($, how.wake, first);
  if (first) {
    // What happened before the session started is not news.
    await update($, told, (list) => [...new Set([...list, ...recentKeys(next)])]);
    return;
  }
  for (const o of outcomes(next, known, await read($, told))) {
    await update($, told, (list) => [...list, o.key].slice(-500));
    await tell($, o, cwd, how.wake);
  }
}

/** What a command printed as JSON, or null when it is none. */
function parsed(stdout: string): unknown {
  try {
    return JSON.parse(stdout);
  } catch {
    return null;
  }
}

/** A command's output, trimmed, or null when it did not run or failed. */
async function output($: $, argv: string[], cwd: string): Promise<string | null> {
  const got = await $.process.run(argv, { cwd }).catch(() => null);
  return got && got.exitCode === 0 ? got.stdout.trim() : null;
}

// One forecast at a time: a document that comes while one runs is forecast
// once it has ended.
let running = false;
let again = false;

/** 64: forecast the session's worktree when what it rests on has moved. */
async function forecast($: $, wake: boolean, quiet: boolean) {
  if (running) {
    again = true;
    return;
  }
  running = true;
  try {
    let q = quiet;
    do {
      again = false;
      // It decides nothing: one that fails is passed over.
      await forecastOnce($, wake, q).catch(() => undefined);
      q = false;
    } while (again);
  } finally {
    running = false;
  }
}

/**
 * Tell a conflict the board has not told of, and a told one that cleared
 * with the head unchanged; once each, kept across a reload. A conflict
 * there when the session started is not news: a toast, not a turn.
 */
async function forecastOnce($: $, wake: boolean, quiet: boolean) {
  const d = await read($, doc);
  const m = await read($, mine);
  if (!d || m.root === null) return;
  const b = forecastBranch(d, m);
  if (!b || !forecasts(b, m)) return;
  const head = await output($, ["git", "rev-parse", "HEAD"], m.root);
  if (!head) return;
  const key = forecastKey(head, b, m);
  const last = await read($, forecasting);
  if (last.key === key) return;
  const sameHead = last.head === head;
  const ahead = Number(await output($, ["git", "rev-list", "--count", `${b.at}..HEAD`], m.root));
  let f: Forecast | null = null;
  if (ahead > 0) {
    const got = await $.process
      .run(["sluice", "submit", b.branch, "--check", "--json"], { cwd: m.root, timeoutMs: 120_000 })
      .catch(() => null);
    try {
      const v = got ? (JSON.parse(got.stdout) as Partial<Forecast>) : null;
      f = v && "conflict" in v ? (v as Forecast) : null;
    } catch {
      f = null;
    }
    if (!f) {
      // It could not be formed: wait for the next move, and tell nothing.
      await update($, forecasting, () => ({ key, head, told: sameHead ? last.told : null }));
      return;
    }
  }
  const sig = f ? signature(b.branch, f) : null;
  await update($, forecasting, () => ({ key, head, told: sig }));
  if (f?.conflict && sig !== last.told) {
    $.ui.toast(forecastToast(b.branch, f.conflict));
    if (wake && !quiet) await $.prompt.submit({ text: conflictText(b.branch, f.conflict) });
  } else if (f && !f.conflict && last.told && sameHead) {
    if (wake && !quiet) await $.prompt.submit({ text: clearText(b.branch, latest(d, m)) });
  }
}

async function tell($: $, o: Outcome, cwd: string, wake: boolean) {
  const word =
    o.word === "landed"
      ? `landed on ${o.branch}`
      : o.word === "failed"
        ? "was blamed"
        : "conflicts";
  $.ui.toast(`sluice: #${o.id} ${word}`);
  let failed: { task: string; log?: string }[] = [];
  if (o.word === "failed") {
    const got = await $.process
      .run(["sluice", "status", String(o.id), "--json"], { cwd })
      .catch(() => null);
    try {
      failed = got && got.exitCode === 0 ? (JSON.parse(got.stdout).failed ?? []) : [];
    } catch {
      failed = [];
    }
    void $.ui.notify(`#${o.id} failed on ${o.branch}`, { title: "sluice" }).catch(() => undefined);
  }
  if (wake) await $.prompt.submit({ text: wakeText(o, failed) });
}

/** `sluice overview --json --watch` for the session's life, else a poll. */
async function feed($: $, cwd: string, how: How) {
  try {
    let rest = "";
    for await (const chunk of $.process.spawn({
      argv: ["sluice", "overview", "--json", "--watch"],
      cwd,
    })) {
      if (chunk.stream !== "stdout") continue;
      rest += chunk.text;
      let nl = rest.indexOf("\n");
      while (nl >= 0) {
        const line = rest.slice(0, nl).trim();
        rest = rest.slice(nl + 1);
        if (line) await take($, JSON.parse(line), cwd, how);
        nl = rest.indexOf("\n");
      }
    }
  } catch {
    // Spawning failed or the stream broke: poll instead.
  }
  $.clock.every(3000, async () => {
    const got = await $.process.run(["sluice", "overview", "--json"], { cwd }).catch(() => null);
    // A failing overview prints its error document on stdout too.
    if (got) await take($, parsed(got.stdout), cwd, how);
  });
}

/** `sluice overview --json` once: the overview, or null with why kept in `unread`. */
async function probe($: $, cwd: string): Promise<Overview | null> {
  const got = await $.process.run(["sluice", "overview", "--json"], { cwd }).catch(() => null);
  // Only a document with branches is an overview.
  const opening = got && got.exitCode === 0 ? parsed(got.stdout) : null;
  const ok = isOverview(opening);
  await update($, unread, () => (ok ? null : unreadReason(got)));
  return ok ? opening : null;
}

// The session's directory, kept for /sluice, which carries none; and whether
// the board follows sluice yet: once a load, however many reads succeed.
let dir: string | null = null;
let following = false;

/**
 * Follow sluice from its first overview: the band, the pane, the wake and
 * the forecast, as the options turn them on.
 */
async function follow($: $, cwd: string, first: Overview, chosen: How) {
  if (following) return;
  following = true;
  const top = await $.process
    .run(["git", "rev-parse", "--show-toplevel"], { cwd })
    .catch(() => null);
  const root = top && top.exitCode === 0 ? top.stdout.trim() : null;
  await update($, mine, (m) => ({ ...m, root }));
  // An orchestrator that starts several agents in one checkout names each
  // one's branch, which the checkout can't say.
  const hint = await $.env.get("SLUICE_BRANCH").catch(() => undefined);
  await update($, named, () => (hint ? hint : null));
  const origin = await $.process
    .run(["git", "remote", "get-url", "origin"], { cwd })
    .catch(() => null);
  const base = origin && origin.exitCode === 0 ? githubBase(origin.stdout) : null;
  await update($, repo, () => base);
  // A session whose orchestrator forecasts for it turns this off.
  const env = await $.env.get("SLUICE_FORECAST").catch(() => undefined);
  const how = { wake: chosen.wake, forecast: chosen.forecast && env !== "0" };
  await take($, first, cwd, how);
  void feed($, cwd, how);
  // Elapsed times move while something is under test or an agent's check
  // goes, and a finished check leaves the pane a quarter hour on.
  $.clock.every(5000, async () => {
    const t = await $.clock.now();
    if (moving(await read($, doc), t)) await update($, now, () => t);
  });
}

export const register: Register = (on, options) => {
  const chosen = { wake: options.wake !== false, forecast: options.forecast !== false };

  on("session.start", async ($, e, next) => {
    const started = await next(e);
    await $.command.register({ name: "sluice", description: "Show sluice's queue in a pane" });
    dir = e.cwd;
    // Only where sluice can be read; /sluice tries again where it can't.
    const first = await probe($, e.cwd);
    if (first) await follow($, e.cwd, first, chosen);
    return started;
  });

  // /sluice opens the pane, and closes it when it is open; Escape closes it too.
  // Above the prompt it asks for the keyboard, so Escape is the pane's; docked,
  // the prompt keeps it.
  on("command.run", { command: "sluice" }, async ($, e) => {
    if ((await $.ui.panes()).some((p) => p.id === PANE)) {
      await $.ui.close({ id: PANE });
      return { text: "sluice's pane is closed." };
    }
    // Where sluice could not be read, read it again: it may be set up now.
    if (!following && dir !== null && (await read($, doc)) === null) {
      const first = await probe($, dir);
      if (first) await follow($, dir, first, chosen);
    }
    const inline = !e.presentation.isFullscreen;
    const opened = await $.ui.open({
      id: PANE,
      title: "sluice",
      closeOnEscape: true,
      ...(inline ? { focus: true as const } : {}),
    });
    return {
      text: opened.isPlaced
        ? openedText(e.presentation.isFullscreen)
        : "Widen the terminal to see the sluice pane.",
    };
  });

  // This session's own submits name their submission or pull request.
  on("tool.call", { tool: "Bash" }, async ($, e, next) => {
    const ran = await next(e);
    const command = e.command;
    if (/\b(sluice|wt)[ -]submit\b/.test(command) && "result" in ran) {
      const out = (ran.result as { stdout?: string } | undefined)?.stdout ?? "";
      const seen = submitted(out);
      if (seen.ids.length > 0 || seen.pulls.length > 0) {
        await update($, mine, (m) => ({
          ...m,
          ids: [...new Set([...m.ids, ...seen.ids])],
          pulls: [...new Set([...m.pulls, ...seen.pulls])],
        }));
      }
    }
    return ran;
  }).catch(($, e, next) => next(e)); // never in the way of the command itself

  on("ui.render", { component: "AbovePrompt" }, async ($, e, next) => {
    const d = await read($, doc);
    await read($, now);
    const said = await read($, error);
    const lines = said !== null ? [said] : bandLines(d, await $.clock.now(), await read($, named));
    if (lines.length === 0 || (await read($, isHidden)) || e.props.hasSurvey) return next(e);
    const { Box, Button, Text } = $.ui.resolve(e);
    return (
      <Box flexDirection="column">
        {lines.map((line, i) => (
          <Box key={`line-${i}`}>
            <Text dimColor wrap="truncate-end">
              {line}
            </Text>
            {i === 0 && (
              <Button
                key="hide"
                plain
                label=" [hide]"
                onPress={() => update($, isHidden, () => true)}
              />
            )}
          </Box>
        ))}
      </Box>
    );
  });

  on("ui.render", { component: "Pane", requestId: PANE }, async ($, e) => {
    const { Box, Button, Link, Text } = $.ui.resolve(e);
    // Above the prompt the board draws its own close; docked, the engine's
    // close mark is there already.
    const close =
      e.props.placement === "inline" ? (
        // biome-ignore lint/a11y/useValidAriaRole: the engine's Button role, a drawing hint, not ARIA's
        <Button
          key="close"
          role="dismiss"
          hotkey="x"
          label="close"
          onPress={() => $.ui.close({ id: PANE })}
        />
      ) : null;
    const d = await read($, doc);
    const m = await read($, mine);
    const base = await read($, repo);
    await read($, now);
    if (!d) {
      // No overview: why sluice could not be read, and what to do; or the
      // session's first read is still under way.
      const why = await read($, unread);
      if (why === null)
        return (
          <Box gap={1}>
            <Text dimColor>Reading sluice…</Text>
            {close}
          </Box>
        );
      return (
        <Box flexDirection="column">
          <Box gap={1}>
            <Text>sluice could not be read here: {why}</Text>
            {close}
          </Box>
          <Text>Ask Claude to set sluice up in this repository.</Text>
        </Box>
      );
    }
    const at = await $.clock.now();
    // The session's named branch first, shown when idle too.
    const first = await read($, named);
    const visible = namedFirst(d.branches, first).filter((b) => b.branch === first || shown(b, at));
    const quiet = d.branches.length - visible.length;
    const unknown = first !== null && !d.branches.some((b) => b.branch === first);
    // sluice has records of no branch: how a submission gets here.
    const empty = emptyLines(d, first);
    const checks = checkRows(d, at);
    const room = hostLine(d.room);
    const held = d.sessions.filter((s) => s.ports !== undefined).length;
    const open_ = await read($, opened);
    const shownOf = await read($, details);
    // An opened failure, under its row: each test, its log, the log's tail.
    const view = (k: string, shown: Detail[]) => (
      <Box key={`${k}-opened`} flexDirection="column" marginLeft={6}>
        {shown.length === 0 && <Text dimColor>no failed tests recorded</Text>}
        {shown.map((t) => (
          <Box key={`${k}-${t.task}`} flexDirection="column">
            <Text bold>{t.task}</Text>
            {t.log && (
              <Button
                key={`${k}-${t.task}-copy`}
                plain
                onPress={() =>
                  $.ui.copy({ text: t.log ?? "", surface: e.surface }).then(() => undefined)
                }
              >
                <Text dimColor>{t.log} [copy]</Text>
              </Button>
            )}
            {t.error && <Text color="red">cannot read it: {t.error}</Text>}
            {(t.tail ?? []).map((l, n) => (
              <Text key={`${k}-${t.task}-${n}`} dimColor wrap="truncate-end">
                {l}
              </Text>
            ))}
          </Box>
        ))}
      </Box>
    );
    return (
      <Box flexDirection="column">
        <Box gap={1}>
          <Text dimColor>
            {d.repository} on {d.host}
          </Text>
          {close}
        </Box>
        {unknown && (
          <Box marginTop={1}>
            <Text bold>{first} · no records</Text>
          </Box>
        )}
        {empty.length > 0 && (
          <Box flexDirection="column" marginTop={1}>
            {empty.map((line) => (
              <Text key={line}>{line}</Text>
            ))}
          </Box>
        )}
        {empty.length === 0 && visible.length === 0 && <Text dimColor>No branch has a queue.</Text>}
        {visible.map((b) => (
          <Box key={`branch-${b.branch}`} flexDirection="column" marginTop={1}>
            <Text bold>
              {b.branch} at {(b.at ?? "nothing").slice(0, 7)} · {plural(b.depth, "submission")} ·{" "}
              {plural(testing(b).length, "combination")} under test
              {b.parallel ? ` (up to ${b.parallel} at once)` : ""}
            </Text>
            {/* Headed as the herdr plugin's popup heads them. */}
            {(testing(b).length > 0 || b.candidates.some((c) => c.state === "formed")) && (
              <Text bold>TESTING</Text>
            )}
            {testing(b).map((c) => {
              const took = since(c.started, at);
              return (
                <Box key={`cand-${b.branch}-${c.id}`} flexDirection="column">
                  <Text color="yellow">
                    ▶ testing {combination(b.branch, c.submissions)}
                    {progress(c) ? `  ${progress(c)}` : ""}
                    {took === undefined
                      ? ""
                      : `  ${duration(took)}${c.limit ? ` / ${limit(c.limit)}` : ""}`}
                  </Text>
                  <Text dimColor>
                    {"    "}
                    {c.targets.join(" ")}
                    {c.executor ? ` · ${c.executor}` : ""}
                  </Text>
                  {taskLines(c).map((t) => (
                    <Text
                      key={`task-${b.branch}-${c.id}-${t.text}`}
                      color={TASK_COLOR[t.state]}
                      dimColor={t.state === "waiting"}
                    >
                      {"    "}
                      {t.text}
                    </Text>
                  ))}
                </Box>
              );
            })}
            {b.candidates
              .filter((c) => c.state === "formed")
              .map((c) => (
                <Text key={`formed-${b.branch}-${c.id}`} dimColor>
                  {"    "}formed, not running: {combination(b.branch, c.submissions)}
                </Text>
              ))}
            {b.submissions.length > 0 && <Text bold>QUEUED</Text>}
            {b.submissions.map((s) => {
              const pull = pullOf(s.from.worktree);
              return (
                <Text key={`sub-${b.branch}-${s.id}`}>
                  {"  "}#{s.id} {s.state.padEnd(9)}{" "}
                  {pull !== null && base ? (
                    <Link href={`${base}/pull/${pull}`}>PR {pull}</Link>
                  ) : (
                    s.from.worktree
                  )}{" "}
                  {s.from.agent}
                  {isMine(s, m) ? "  ← you" : ""}
                </Text>
              );
            })}
            {b.recent.length > 0 && <Text bold>RECENT</Text>}
            {[...b.recent]
              .reverse()
              .slice(0, 5)
              .map((r, i) => {
                const mark = r.event === "landed" ? "✓" : r.event === "blamed" ? "✗" : "⚠";
                const color =
                  r.event === "landed" ? "green" : r.event === "blamed" ? "red" : "yellow";
                if (r.event === "blamed" && r.id !== undefined) {
                  const id = r.id;
                  const k = `blamed-${b.branch}-${id}-${r.at}`;
                  return (
                    <Box key={`recent-${b.branch}-${i}`} flexDirection="column">
                      <Button key={k} plain onPress={() => toggle($, k, () => blamedTests($, id))}>
                        <Text color={color}>
                          {"  "}
                          {mark} {r.at.slice(11, 16)} {recentWords(r)}
                        </Text>
                      </Button>
                      {open_.includes(k) && view(k, shownOf[k] ?? [])}
                    </Box>
                  );
                }
                return (
                  <Text key={`recent-${b.branch}-${i}`} color={color}>
                    {"  "}
                    {mark} {r.at.slice(11, 16)} {recentWords(r)}
                  </Text>
                );
              })}
            {b.candidates
              .filter((c) => c.failed.some((f) => f.log))
              .slice(-1)
              .flatMap((c) => c.failed.filter((f) => f.log))
              .map((f) => {
                const k = `log-${b.branch}-${f.task}${f.system ? `-${f.system}` : ""}`;
                return (
                  <Box key={`${k}-row`} flexDirection="column">
                    <Button key={k} plain onPress={() => toggle($, k, async () => [f])}>
                      {"    "}log of {failedName(f)}: <Text dimColor>{f.log}</Text>
                    </Button>
                    {open_.includes(k) && view(k, shownOf[k] ?? [])}
                  </Box>
                );
              })}
          </Box>
        ))}
        {quiet > 0 && visible.length > 0 && (
          <Box marginTop={1}>
            <Text dimColor>
              {quiet === 1 ? "1 other branch" : `${quiet} other branches`} with nothing queued
            </Text>
          </Box>
        )}
        {/* Agents' checks are load on the host, never submissions. */}
        {(checks.length > 0 || room) && (
          <Box flexDirection="column" marginTop={1}>
            {checks.length > 0 && <Text bold>CHECKING</Text>}
            {checks.map((c) => (
              <Text key={`check-${c.key}`} color={CHECK_COLOR[c.state]}>
                {"  "}
                {c.text}
              </Text>
            ))}
            {room && <Text dimColor>{room}</Text>}
          </Box>
        )}
        <Box marginTop={1}>
          <Text dimColor>
            {plural(held, "worktree")} held · {d.previews.length} previews
            {(d.hosts ?? []).length > 0 ? ` · ${hostsLine(d.hosts ?? [])}` : ""}
          </Text>
        </Box>
      </Box>
    );
  });
};
