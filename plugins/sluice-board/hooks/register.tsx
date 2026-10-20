// sluice's queue inside a Claude Code session: a band above the prompt, a
// pane on /sluice, a status line, and a turn that tells an idle session its
// submission's outcome. Everything is read from `sluice overview --json`
// (sluice/overview@1), streamed by `--watch` (ADR 0023): the board decides
// nothing and writes nothing of sluice's.

import type { EngineInterface, Register } from "claude-code";
import { atom, read, update } from "claude-code";

import type { Mine, OpenedTest, Outcome, Overview, OverviewTask } from "../types";
import type { CheckRow } from "./board";
import {
  bandLines,
  checkRows,
  combination,
  duration,
  githubBase,
  hostLine,
  isMine,
  learn,
  limit,
  moving,
  outcomes,
  plural,
  progress,
  pullOf,
  recentKeys,
  recentWords,
  shown,
  since,
  statusLine,
  submitted,
  taskLines,
  testing,
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
const CHECK_COLOR: Record<CheckRow["state"], string | undefined> = {
  running: "yellow",
  passed: "green",
  failed: "red",
  ended: undefined,
};
const NO_ONE: Mine = { root: null, ids: [], pulls: [] };

const doc = atom({ plugin: "sluice-board", key: "doc" } as const, null);
const mine = atom({ plugin: "sluice-board", key: "mine" } as const, NO_ONE);
const told = atom({ plugin: "sluice-board", key: "told" } as const, []);
const isHidden = atom({ plugin: "sluice-board", key: "isHidden" } as const, false);
const now = atom({ plugin: "sluice-board", key: "now" } as const, 0);
const repo = atom({ plugin: "sluice-board", key: "repo" } as const, null);
// Failures opened in place: their keys, and what each showed when opened.
const opened = atom({ plugin: "sluice-board", key: "opened" } as const, []);
const details = atom({ plugin: "sluice-board", key: "details" } as const, {});

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

/** A new document: learn this session's submissions, tell new outcomes. */
async function take($: $, next: Overview, cwd: string, wake: boolean) {
  const first = (await read($, doc)) === null;
  await update($, doc, () => next);
  const known = learn(next, await read($, mine));
  await update($, mine, () => known);
  $.ui.status(statusLine(next, known));
  if (first) {
    // What happened before the session started is not news.
    await update($, told, (list) => [...new Set([...list, ...recentKeys(next)])]);
    return;
  }
  for (const o of outcomes(next, known, await read($, told))) {
    await update($, told, (list) => [...list, o.key].slice(-500));
    await tell($, o, cwd, wake);
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
async function feed($: $, cwd: string, wake: boolean) {
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
        if (line) await take($, JSON.parse(line) as Overview, cwd, wake);
        nl = rest.indexOf("\n");
      }
    }
  } catch {
    // Spawning failed or the stream broke: poll instead.
  }
  $.clock.every(3000, async () => {
    const got = await $.process.run(["sluice", "overview", "--json"], { cwd }).catch(() => null);
    if (got && got.exitCode === 0) await take($, JSON.parse(got.stdout) as Overview, cwd, wake);
  });
}

export const register: Register = (on, options) => {
  const wake = options.wake !== false;

  on("session.start", async ($, e, next) => {
    const started = await next(e);
    await $.command.register({ name: "sluice", description: "Show sluice's queue in a pane" });
    const cwd = e.cwd;
    // Only in a repository with sluice's records.
    const probe = await $.process.run(["sluice", "overview", "--json"], { cwd }).catch(() => null);
    if (!probe || probe.exitCode !== 0) return started;
    const top = await $.process
      .run(["git", "rev-parse", "--show-toplevel"], { cwd })
      .catch(() => null);
    const root = top && top.exitCode === 0 ? top.stdout.trim() : null;
    await update($, mine, (m) => ({ ...m, root }));
    const origin = await $.process
      .run(["git", "remote", "get-url", "origin"], { cwd })
      .catch(() => null);
    const base = origin && origin.exitCode === 0 ? githubBase(origin.stdout) : null;
    await update($, repo, () => base);
    await take($, JSON.parse(probe.stdout) as Overview, cwd, wake);
    void feed($, cwd, wake);
    // Elapsed times move while something is under test or an agent's check
    // goes, and a finished check leaves the pane a quarter hour on.
    $.clock.every(5000, async () => {
      const t = await $.clock.now();
      if (moving(await read($, doc), t)) await update($, now, () => t);
    });
    return started;
  });

  on("command.run", { command: "sluice" }, async ($) => {
    const opened = await $.ui.open({ id: PANE, title: "sluice" });
    return {
      text: opened.isPlaced
        ? "sluice's queue is open in a pane."
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
    const lines = bandLines(d, await $.clock.now());
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
    const d = await read($, doc);
    const m = await read($, mine);
    const base = await read($, repo);
    await read($, now);
    if (!d) return <Text dimColor>No sluice records in this repository.</Text>;
    const at = await $.clock.now();
    const visible = d.branches.filter((b) => shown(b, at));
    const quiet = d.branches.length - visible.length;
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
        <Text dimColor>
          {d.repository} on {d.host}
        </Text>
        {visible.length === 0 && <Text dimColor>No branch has a queue.</Text>}
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
                const k = `log-${b.branch}-${f.task}`;
                return (
                  <Box key={`${k}-row`} flexDirection="column">
                    <Button key={k} plain onPress={() => toggle($, k, async () => [f])}>
                      {"    "}log of {f.task}: <Text dimColor>{f.log}</Text>
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
            {held} worktrees held · {d.previews.length} previews
          </Text>
        </Box>
      </Box>
    );
  });
};
