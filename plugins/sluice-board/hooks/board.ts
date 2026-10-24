// What the board shows, from `sluice overview --json` (sluice/overview@1):
// pure functions over the document, so the tests need no sluice running.

import type {
  Flaky,
  Forecast,
  LastResult,
  Latest,
  Mine,
  Outcome,
  Overview,
  OverviewBranch,
  OverviewCandidate,
  OverviewFailed,
  OverviewHost,
  OverviewSession,
  OverviewTask,
  Recent,
  Room,
} from "../types";

/** Whether `v` is an overview document: one with branches. */
export function isOverview(v: unknown): v is Overview {
  return typeof v === "object" && v !== null && Array.isArray((v as Overview).branches);
}

/**
 * `sluice: <message>`, one line, for the error document a failing overview
 * prints (`{"error":{"kind","message","next"}}`); null for anything else.
 */
export function errorLine(v: unknown): string | null {
  const e = typeof v === "object" && v !== null ? (v as { error?: unknown }).error : undefined;
  const message =
    typeof e === "object" && e !== null ? (e as { message?: unknown }).message : undefined;
  if (typeof message !== "string") return null;
  return `sluice: ${message.trim().replace(/\s*\n\s*/g, " ")}`;
}

/** The hosts serving a remote: how many, their room, and any lapsed. */
export function hostsLine(hosts: readonly OverviewHost[]): string {
  const room = hosts.reduce((n, h) => n + h.room, 0);
  const lapsed = hosts.filter((h) => h.lapsed).length;
  return `${plural(hosts.length, "host")}, room for ${room}${lapsed > 0 ? `, ${lapsed} lapsed` : ""}`;
}

/** A failed test by name, and the system it failed on where there is one. */
export function failedName(f: OverviewFailed): string {
  return f.system ? `${f.task} (${f.system})` : f.task;
}

/** ` 19:42 on devbox`: when and where a run ran, each left out when the record lacks it. */
function ranAt(at: string | undefined, host: string | undefined): string {
  const time = at ? at.slice(11, 16) : "";
  return `${time ? ` ${time}` : ""}${host ? ` on ${host}` : ""}`;
}

/**
 * The headings of a flaky failure's two runs: this run's outcome, time and
 * host, then the run it contradicts, with its branch and combination too.
 */
export function flakyLines(f: Flaky): [string, string] {
  const word = (o: "pass" | "fail") => (o === "pass" ? "passed" : "failed");
  const o = f.other;
  return [
    `this run: ${word(f.here)}${ranAt(f.at, f.host)}`,
    `the run it contradicts: ${word(o.outcome)}${ranAt(o.at, o.host)} · ${o.branch} ${o.combination.slice(0, 7)}`,
  ];
}

/** `2m10s`, `45s`, `1h03m`. */
export function duration(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m${String(s % 60).padStart(2, "0")}s`;
  return `${Math.floor(s / 3600)}h${String(Math.floor((s % 3600) / 60)).padStart(2, "0")}m`;
}

/** `15m` for a limit: whole minutes where it divides. */
export function limit(seconds: number): string {
  return seconds % 60 === 0 && seconds < 3600 ? `${seconds / 60}m` : duration(seconds);
}

export function ids(list: readonly number[]): string {
  return list.map((i) => `#${i}`).join("+");
}

/** The combinations under test on a branch. */
export function testing(b: OverviewBranch) {
  return b.candidates.filter((c) => c.state === "testing" || c.state === "starting");
}

/** Seconds since an RFC 3339 time, at `now` (ms since the epoch). */
export function since(at: string | undefined, now: number): number | undefined {
  if (!at) return undefined;
  const t = Date.parse(at);
  return Number.isNaN(t) ? undefined : (now - t) / 1000;
}

/** What a recent log line says, in a few words. */
export function recentWords(r: Recent): string {
  const short = (v: unknown) => (typeof v === "string" ? v.slice(0, 7) : "");
  switch (r.event) {
    case "landed":
      return `${(r.ids ?? []).map((i) => `#${i}`).join(" ")} landed as ${short(r.to)}`;
    case "blamed":
      return `#${r.id} blamed on ${short(r.combination)}`;
    case "conflict":
      return `#${r.id} conflict · ${(r.paths ?? []).join(", ")}`;
    default:
      return String(r.event);
  }
}

/** `main+#80+#81`: a combination as the band names it. */
export function combination(branch: string, list: readonly number[]): string {
  return [branch, ...list.map((i) => `#${i}`)].join("+");
}

/** The submissions in no combination under test. */
export function waiting(b: OverviewBranch): number[] {
  const under = new Set(testing(b).flatMap((c) => c.submissions));
  return b.submissions.map((s) => s.id).filter((i) => !under.has(i));
}

export function plural(n: number, word: string): string {
  return `${n} ${word}${n === 1 ? "" : "s"}`;
}

/** Whether a task has ended, however. */
export function ended(t: OverviewTask): boolean {
  return t.state === "passed" || t.state === "failed" || t.state === "replayed";
}

/** Ten cells, one filled for each tenth of the tasks ended, rounded down. */
export function bar(done: number, total: number): string {
  const full = total > 0 ? Math.floor((10 * Math.min(done, total)) / total) : 0;
  return "▓".repeat(full) + "░".repeat(10 - full);
}

/** `3/5 ▓▓▓▓▓▓░░░░`: how many of a run's tasks have ended, or null when its runner says nothing. */
export function progress(c: { tasks?: OverviewTask[] }): string | null {
  if (!c.tasks || c.tasks.length === 0) return null;
  const done = c.tasks.filter(ended).length;
  return `${done}/${c.tasks.length} ${bar(done, c.tasks.length)}`;
}

const MARK = { passed: "✓", replayed: "✓", failed: "✗", running: "…", waiting: "·" } as const;

/** What follows a task's name: how long it took once ended, how long it has run, else its state. */
function taskDetail(t: OverviewTask, now: number): string {
  if (t.state === "replayed") return "replayed";
  if (t.state === "running") {
    const took = since(t.started, now);
    return took === undefined ? "running" : `running ${duration(took)}`;
  }
  if (t.state === "waiting") return t.state;
  const took = t.started && t.ended ? (Date.parse(t.ended) - Date.parse(t.started)) / 1000 : NaN;
  return Number.isNaN(took) ? t.state : duration(took);
}

/**
 * The pane's task list: ended tasks in the order they ended, then running
 * ones, then waiting ones, names padded and times right-aligned, at `now`.
 */
export function taskLines(
  c: { tasks?: OverviewTask[] },
  now: number,
): { state: OverviewTask["state"]; text: string }[] {
  const tasks = c.tasks ?? [];
  const rank = (t: OverviewTask) => (ended(t) ? 0 : t.state === "running" ? 1 : 2);
  const order = tasks
    .map((t, i) => ({ t, i }))
    .sort(
      (a, b) =>
        rank(a.t) - rank(b.t) ||
        (ended(a.t) ? Date.parse(a.t.ended ?? "") - Date.parse(b.t.ended ?? "") || 0 : 0) ||
        a.i - b.i,
    )
    .map((x) => x.t);
  const name = Math.max(0, ...order.map((t) => t.target.length));
  const detail = Math.max(0, ...order.map((t) => taskDetail(t, now).length));
  return order.map((t) => ({
    state: t.state,
    text: `${MARK[t.state]} ${t.target.padEnd(name)}  ${taskDetail(t, now).padStart(detail)}`,
  }));
}

/** How many opened rows the pane keeps, the oldest let go first. */
const OPENED = 200;

/** The rows opened in place, with `key` opened, or folded when it was open. */
export function flipped(list: readonly string[], key: string): string[] {
  return list.includes(key) ? list.filter((k) => k !== key) : [...list, key].slice(-OPENED);
}

/** Whether a branch has anything queued or under test; the rest are left out of the band. */
export function busy(b: OverviewBranch): boolean {
  return b.depth > 0 || testing(b).length > 0;
}

/**
 * Whether the pane shows a branch: one busy, or whose last event is under a
 * quarter of an hour old, so a host with many idle branches shows its queues
 * and a landing is still seen once its queue is empty.
 */
export function shown(b: OverviewBranch, now: number): boolean {
  const last = b.recent[b.recent.length - 1];
  const age = last ? since(last.at, now) : undefined;
  return busy(b) || (age !== undefined && age < 900);
}

/**
 * Whether agents' checks hold a branch's queue back: the host has no run
 * free, checks hold at least one, and the branch has a submission queued
 * while it tests fewer combinations than its window. False with no room,
 * from a sluice older than it.
 */
export function heldBack(b: OverviewBranch, room: Room | undefined): boolean {
  if (!room || room.sessions === 0) return false;
  if (room.queue + room.sessions < room.bound) return false;
  const queued = b.submissions.some((s) => s.state === "queued");
  return queued && testing(b).length < b.window;
}

/** `1 slot to an agent's checks`: what holds the queue back, when it is checks. */
export function checksWords(room: Room): string {
  return room.sessions === 1
    ? "1 slot to an agent's checks"
    : `${room.sessions} slots to agents' checks`;
}

/** The band's line for one branch, or null when it has nothing to show. */
export function branchLine(b: OverviewBranch, now: number, room?: Room): string | null {
  if (!busy(b)) return null;
  const under = testing(b);
  const parts = [`sluice ${b.branch}`, plural(b.depth, "submission")];
  for (const c of under) {
    const took = since(c.started, now);
    const clock =
      took === undefined ? "" : ` ${duration(took)}${c.limit ? `/${limit(c.limit)}` : ""}`;
    const done = progress(c);
    parts.push(`testing ${combination(b.branch, c.submissions)}${done ? ` ${done}` : ""}${clock}`);
  }
  const idle = waiting(b);
  if (idle.length > 0) parts.push(`${idle.map((i) => `#${i}`).join(" ")} waiting`);
  const last = b.recent[b.recent.length - 1];
  if (last) parts.push(`last ${recentWords(last)}`);
  if (room && heldBack(b, room)) parts.push(checksWords(room));
  return parts.join(" · ");
}

/** The branch named for this session first, then the rest in the overview's order. */
export function namedFirst<B extends { branch: string }>(
  branches: readonly B[],
  named: string | null,
): B[] {
  if (!named) return [...branches];
  return [
    ...branches.filter((b) => b.branch === named),
    ...branches.filter((b) => b.branch !== named),
  ];
}

/**
 * The band: one line per branch with a queue, in the overview's order but
 * for the branch named for this session, first; empty when none has.
 */
export function bandLines(
  doc: Overview | null,
  now: number,
  named: string | null = null,
): string[] {
  // A document an earlier load took without branches shows nothing.
  if (!isOverview(doc)) return [];
  return namedFirst(doc.branches, named)
    .map((b) => branchLine(b, now, doc.room))
    .filter((l): l is string => l !== null);
}

/**
 * The state of a branch named for a session whose worktree can't say it, in
 * the herdr plugin's words for a pane's: `release · 3 queued · #12 testing
 * 3/8 · 2 checking`, its oldest combination under test and the agents'
 * checks held to it; `release · nothing queued`, or `release · no records`
 * when the overview does not list it.
 */
export function namedLine(doc: Overview, branch: string, now: number): string {
  const b = doc.branches.find((x) => x.branch === branch);
  if (!b) return `${branch} · no records`;
  const parts = [branch];
  const queued = waiting(b).length;
  if (queued > 0) parts.push(`${queued} queued`);
  const under = testing(b);
  const key = (c: OverviewCandidate) => c.started ?? "~";
  const oldest = under.reduce<OverviewCandidate | undefined>(
    (min, c) => (min === undefined || key(c) < key(min) ? c : min),
    undefined,
  );
  if (oldest) {
    const tasks = oldest.tasks ?? [];
    const took = since(oldest.started, now);
    const count =
      tasks.length > 0
        ? ` ${tasks.filter(ended).length}/${tasks.length}`
        : took === undefined
          ? ""
          : ` ${Math.floor(took / 60)}m`;
    const more = under.length > 1 ? ` +${under.length - 1} more` : "";
    parts.push(`${ids(oldest.submissions)} testing${count}${more}`);
  }
  const checking = doc.sessions.filter((s) => s.branch === branch && s.run).length;
  if (checking > 0) parts.push(`${checking} checking`);
  if (parts.length === 1) parts.push("nothing queued");
  return parts.join(" · ");
}

/** How long a finished check stays in the pane: as long as a recent event does. */
const CHECK_SHOWN = 900;

const OUTCOME_WORD: Record<LastResult["outcome"], string> = {
  pass: "passed",
  fail: "failed",
  timed_out: "timed out",
  lost: "lost",
  stopped: "stopped",
  not_run: "not run",
};

/**
 * What a finished check is worth: `14 passed · reused when submitted` when
 * the gate takes its passes, `· not kept: uncommitted changes` when the
 * checkout held more than its head, `2 of 14 failed` when any failed.
 */
export function finishedWords(r: LastResult): string {
  const tasks = r.tasks ?? [];
  const failed = tasks.filter((t) => t.state === "failed").length;
  if (failed > 0) return `${failed} of ${tasks.length} failed`;
  const passed = tasks.filter((t) => t.state === "passed" || t.state === "replayed").length;
  const words = tasks.length > 0 ? `${passed} passed` : OUTCOME_WORD[r.outcome];
  if (r.outcome !== "pass") return words;
  if (r.reused) return `${words} · reused when submitted`;
  if (r.kind === "checks" && !r.recorded) return `${words} · not kept: uncommitted changes`;
  return words;
}

/** A worktree's last path segment: the name an agent's checkout goes by. */
function basename(path: string): string {
  return (
    path
      .split("/")
      .filter((s) => s !== "")
      .pop() ?? path
  );
}

/** One row of CHECKING: an agent's check going, or one that ended this quarter hour. */
export type CheckRow = {
  key: string;
  state: "running" | "passed" | "failed" | "ended";
  text: string;
  /** The checkout it came from, whose run the row opens to. */
  session: OverviewSession;
};

function checkRow(s: OverviewSession, now: number): CheckRow | null {
  const who = `${basename(s.worktree)} → ${s.branch}`;
  if (s.run) {
    const took = since(s.run.started, now);
    const clock = took === undefined ? [] : [duration(took)];
    const what = s.run.command ?? progress(s.run);
    return {
      key: s.worktree,
      state: "running",
      text: [who, ...(what ? [what] : []), ...clock].join(" "),
      session: s,
    };
  }
  const r = s.last_result;
  const age = r ? since(r.at, now) : undefined;
  if (!r || age === undefined || age >= CHECK_SHOWN) return null;
  const words = finishedWords(r);
  return {
    key: s.worktree,
    state: r.outcome === "pass" ? "passed" : r.outcome === "fail" ? "failed" : "ended",
    text: `${who} ${words}`,
    session: s,
  };
}

/**
 * What a CHECKING row opens to: the tasks of the run going, else of the run
 * that ended; a command's run has none, and says what it runs while it goes,
 * since a finished run keeps no command.
 */
export function checkLines(
  s: OverviewSession,
  now: number,
): { state?: OverviewTask["state"]; text: string }[] {
  const run = s.run ?? s.last_result;
  if (s.run?.command) return [{ text: `runs ${s.run.command} · no tasks` }];
  if (!s.run && s.last_result?.kind === "command") return [{ text: "ran a command · no tasks" }];
  if (!run?.tasks || run.tasks.length === 0) return [{ text: "no tasks reported" }];
  return taskLines(run, now);
}

/** CHECKING: agents' checks, as load on the host and never as submissions. */
export function checkRows(doc: Overview, now: number): CheckRow[] {
  return doc.sessions.map((s) => checkRow(s, now)).filter((r): r is CheckRow => r !== null);
}

/** `host: 4 of 6 runs · 3 queue · 1 agent`, or null while nothing counts against the bound. */
export function hostLine(room: Room | undefined): string | null {
  if (!room || room.queue + room.sessions === 0) return null;
  const parts = [`host: ${room.queue + room.sessions} of ${room.bound} runs`];
  if (room.queue > 0) parts.push(`${room.queue} queue`);
  if (room.sessions > 0) parts.push(plural(room.sessions, "agent"));
  return parts.join(" · ");
}

/** Whether times on the board move: a combination under test, or a check going or just ended. */
export function moving(doc: Overview | null, now: number): boolean {
  if (!doc) return false;
  return doc.branches.some((b) => testing(b).length > 0) || checkRows(doc, now).length > 0;
}

/** Whether a submission is this session's. */
export function isMine(
  s: { id: number; from: { worktree: string; path?: string } },
  mine: Mine,
): boolean {
  return (
    (mine.root !== null && s.from.path === mine.root) ||
    mine.ids.includes(s.id) ||
    mine.pulls.includes(s.from.worktree)
  );
}

/** `mine` with every active submission of the document that is this session's. */
export function learn(doc: Overview, mine: Mine): Mine {
  const found = doc.branches
    .flatMap((b) => b.submissions)
    .filter((s) => isMine(s, mine))
    .map((s) => s.id);
  const more = found.filter((i) => !mine.ids.includes(i));
  return more.length === 0 ? mine : { ...mine, ids: [...mine.ids, ...more] };
}

/** A recent line's identity, so an outcome is told once. */
export function key(branch: string, r: Recent): string {
  return `${branch}:${r.event}:${r.id ?? (r.ids ?? []).join(",")}:${r.at}`;
}

/** Every recent line of the document, keyed. */
export function recentKeys(doc: Overview): string[] {
  return doc.branches.flatMap((b) => b.recent.map((r) => key(b.branch, r)));
}

/**
 * The outcomes of this session's submissions the document shows that have
 * not been told: a landing, a blame or a conflict naming one of `mine.ids`.
 */
export function outcomes(doc: Overview, mine: Mine, told: readonly string[]): Outcome[] {
  const out: Outcome[] = [];
  for (const b of doc.branches) {
    for (const r of b.recent) {
      const k = key(b.branch, r);
      if (told.includes(k)) continue;
      if (r.event === "landed") {
        for (const id of r.ids ?? []) {
          if (mine.ids.includes(id)) {
            out.push({ key: k, branch: b.branch, id, word: "landed", to: String(r.to ?? "") });
          }
        }
      } else if ((r.event === "blamed" || r.event === "conflict") && r.id !== undefined) {
        if (mine.ids.includes(r.id)) {
          out.push({
            key: k,
            branch: b.branch,
            id: r.id,
            word: r.event === "blamed" ? "failed" : "conflict",
            combination: typeof r.combination === "string" ? r.combination : undefined,
            paths: r.paths,
          });
        }
      }
    }
  }
  return out;
}

/** What `sluice submit` printed that names this session's work. */
export function submitted(stdout: string): { ids: number[]; pulls: string[] } {
  const ids = [
    ...stdout.matchAll(/submission (\d+) to /g),
    ...stdout.matchAll(/"id":\s*(\d+)/g),
  ].map((m) => Number(m[1]));
  const pulls = [
    ...stdout.matchAll(/(?:opened|updated) #(\d+)/g),
    ...stdout.matchAll(/"pull_request":\s*(\d+)/g),
  ].map((m) => `pull/${m[1]}`);
  return { ids, pulls };
}

/**
 * The status line: what is queued, and where this session's work stands;
 * with no submission of its own, the state of the branch named for it.
 */
export function statusLine(
  doc: Overview | null,
  mine: Mine,
  named: string | null = null,
  now = 0,
): string | undefined {
  if (!doc) return undefined;
  const queued = doc.branches.reduce((n, b) => n + b.depth, 0);
  const own = doc.branches.flatMap((b) => b.submissions).filter((s) => isMine(s, mine));
  const held =
    doc.room && doc.branches.some((b) => heldBack(b, doc.room))
      ? ` · ${checksWords(doc.room)}`
      : "";
  if (named && own.length === 0) return `sluice: ${namedLine(doc, named, now)}${held}`;
  if (queued === 0 && own.length === 0) return undefined;
  const yours = own.map((s) => ` · yours #${s.id} ${s.state}`).join("");
  return `sluice: ${plural(queued, "submission")}${yours}${held}`;
}

/** The prompt that tells an idle session its submission's outcome. */
export function wakeText(o: Outcome, failed: OverviewFailed[]): string {
  if (o.word === "landed") {
    return `[sluice] Your submission #${o.id} landed on ${o.branch} as ${(o.to ?? "").slice(0, 7)}.`;
  }
  if (o.word === "conflict") {
    return [
      `[sluice] Your submission #${o.id} to ${o.branch} conflicts: ${(o.paths ?? []).join(", ")}.`,
      `Merge ${o.branch} into your worktree, resolve, commit, and submit again (check first with \`sluice submit --check ${o.branch}\`).`,
    ].join("\n");
  }
  const tasks = failed.map(failedName).join(", ") || "its tests";
  const logs = failed.filter((f) => f.log).map((f) => `Log of ${failedName(f)}: ${f.log}`);
  return [
    `[sluice] Your submission #${o.id} to ${o.branch} failed: ${tasks}${o.combination ? ` on ${o.combination.slice(0, 7)}` : ""}.`,
    ...logs,
    `Read the log, reproduce with \`sluice exec ${o.branch}\`, fix, commit, and submit again.`,
  ].join("\n");
}

/**
 * The session's latest submission the document shows: still in a queue, or
 * named by a recent landing, blame or conflict. Null when it shows none.
 */
export function latest(doc: Overview, mine: Mine): Latest | null {
  let found: Latest | null = null;
  const see = (l: Latest) => {
    if (!found || l.id > found.id) found = l;
  };
  for (const b of doc.branches) {
    for (const s of b.submissions)
      if (isMine(s, mine)) see({ id: s.id, branch: b.branch, conflict: false, active: true });
    for (const r of b.recent) {
      const named = r.id !== undefined ? [r.id] : (r.ids ?? []);
      for (const id of named)
        if (mine.ids.includes(id) && !b.submissions.some((s) => s.id === id))
          see({ id, branch: b.branch, conflict: r.event === "conflict", active: false });
    }
  }
  return found;
}

/**
 * The branch the session's work would land on: its latest submission's,
 * else the one its checkout's runs are held to, else the only branch.
 */
export function forecastBranch(doc: Overview, mine: Mine): OverviewBranch | null {
  const name =
    latest(doc, mine)?.branch ??
    doc.sessions.find((s) => mine.root !== null && s.worktree === mine.root)?.branch ??
    (doc.branches.length === 1 ? doc.branches[0]!.branch : undefined);
  return doc.branches.find((b) => b.branch === name) ?? null;
}

/**
 * Whether to forecast the session's work on `b`: not while a submission of
 * its own waits there or is under test, which the round judges itself.
 */
export function forecasts(b: OverviewBranch, mine: Mine): boolean {
  return b.at !== null && !b.submissions.some((s) => isMine(s, mine));
}

/** What a forecast rests on: the head, the branch's revision, the others waiting. */
export function forecastKey(head: string, b: OverviewBranch, mine: Mine): string {
  const others = b.submissions
    .filter((s) => !isMine(s, mine))
    .map((s) => s.id)
    .sort((x, y) => x - y);
  return `${head} ${b.at ?? ""} ${others.join(",")}`;
}

/** A conflict by its files and what they are contested with, or null for none. */
export function signature(branch: string, f: Forecast): string | null {
  if (!f.conflict) return null;
  const paths = [...f.conflict.paths].sort().join(",");
  const with_ = f.conflict.with.map((c) => c.id).sort((x, y) => x - y);
  return `${branch}|${paths}|${with_.join(",")}`;
}

/** A submission in a conflict's way, and the worktree it came from where it is known. */
export type InWay = { id: number; worktree?: string };

/** What a conflict is contested with: the submissions, or what the branch landed. */
function inTheWay(branch: string, inWay: readonly InWay[]): string {
  return inWay.length === 0
    ? `what ${branch} has landed`
    : inWay.map((c) => (c.worktree ? `#${c.id} (${c.worktree})` : `#${c.id}`)).join(", ");
}

function contested(branch: string, f: NonNullable<Forecast["conflict"]>): string {
  return inTheWay(branch, forecastInWay(f));
}

/** What an opened conflict shows: its files, what is in the way, and the next step. */
export function conflictLines(
  branch: string,
  paths: readonly string[],
  inWay: readonly InWay[],
): string[] {
  return [
    `files: ${paths.join(", ")}`,
    `in the way: ${inTheWay(branch, inWay)}`,
    `next: take ${branch} into the worktree, resolve and commit, then sluice submit ${branch}`,
  ];
}

/** What is in a RECENT conflict's way, each named with the worktree it came from. */
export function recentInWay(b: OverviewBranch, r: Recent): InWay[] {
  const from = b.recent_from ?? {};
  return (r.with ?? []).map((id) => {
    const worktree = from[String(id)];
    return worktree ? { id, worktree } : { id };
  });
}

/** The row a conflict a forecast found draws under its branch. */
export function forecastWords(f: NonNullable<Forecast["conflict"]>): string {
  const more = f.paths.length > 1 ? ` +${f.paths.length - 1}` : "";
  return `your work would conflict · ${f.paths[0] ?? "a file"}${more}`;
}

/** What is in a forecast conflict's way, as the forecast names it. */
export function forecastInWay(f: NonNullable<Forecast["conflict"]>): InWay[] {
  return f.with.map((c) => ({ id: c.id, worktree: c.from.worktree }));
}

/** The toast for a conflict a forecast found. */
export function forecastToast(branch: string, f: NonNullable<Forecast["conflict"]>): string {
  const more = f.paths.length > 1 ? ` +${f.paths.length - 1}` : "";
  return `sluice: your work would conflict with ${branch} in ${f.paths[0] ?? "a file"}${more}`;
}

/** The prompt that tells a session its work has come to conflict. */
export function conflictText(branch: string, f: NonNullable<Forecast["conflict"]>): string {
  return [
    `[sluice] Your work would conflict with ${contested(branch, f)}, in ${f.paths.join(", ")}.`,
    `Merge ${branch} into your worktree now and resolve it while it is small (check with \`sluice submit --check ${branch}\`).`,
  ].join(" ");
}

/** The prompt that tells a session the conflict it was told of has cleared. */
export function clearText(branch: string, last: Latest | null): string {
  const again =
    last?.conflict && !last.active && last.branch === branch
      ? ` #${last.id} can be submitted again: \`sluice submit ${branch}\`.`
      : "";
  return `[sluice] Your work no longer conflicts with ${branch} or anything queued.${again}`;
}

/**
 * The reply to `/sluice` that opened the pane: how to close it where it
 * opened, docked beside a full-screen transcript or above the prompt.
 */
export function openedText(isFullscreen: boolean): string {
  return isFullscreen
    ? "sluice's queue is open in a pane. Its close mark or /sluice again closes it."
    : "sluice's queue is open above the prompt. Esc, its close button or /sluice again closes it.";
}

/**
 * Why `sluice overview --json` gave no overview: the command could not be
 * started (`got` null), else sluice's first line on stderr, else its exit.
 */
export function unreadReason(got: { exitCode: number; stderr: string } | null): string {
  if (!got) return "the sluice command was not found";
  const line = got.stderr
    .split("\n")
    .map((l) => l.trim())
    .find((l) => l !== "");
  return line ?? `sluice overview exited ${got.exitCode}`;
}

/**
 * What the pane says under its first line when sluice has records of no
 * branch and the session names none: how a submission gets there.
 */
export function emptyLines(doc: Overview, named: string | null): string[] {
  if (doc.branches.length > 0 || named !== null) return [];
  return [
    "Nothing submitted yet.",
    "When an agent in a worktree runs sluice submit <branch>, its submission shows here until it lands.",
  ];
}

/** `https://github.com/<owner>/<repo>` from a remote's URL, or null off GitHub. */
export function githubBase(url: string): string | null {
  const m = url
    .trim()
    .match(/^(?:https:\/\/|ssh:\/\/git@|git@)github\.com[:/]([^/\s]+)\/([^/\s]+?)(?:\.git)?\/?$/);
  return m ? `https://github.com/${m[1]}/${m[2]}` : null;
}

/** The pull request number of a submission from `pull/<n>`, or null. */
export function pullOf(worktree: string): number | null {
  const m = worktree.match(/^pull\/(\d+)$/);
  return m ? Number(m[1]) : null;
}
