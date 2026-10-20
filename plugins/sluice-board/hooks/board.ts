// What the board shows, from `sluice overview --json` (sluice/overview@1):
// pure functions over the document, so the tests need no sluice running.

import type {
  Mine,
  Outcome,
  Overview,
  OverviewBranch,
  OverviewCandidate,
  OverviewFailed,
  OverviewHost,
  OverviewTask,
  Recent,
} from "../types";

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

/** `3/5 ▓▓▓▓▓▓░░░░`: how many of a combination's tasks have ended, or null when its runner says nothing. */
export function progress(c: OverviewCandidate): string | null {
  if (!c.tasks || c.tasks.length === 0) return null;
  const done = c.tasks.filter(ended).length;
  return `${done}/${c.tasks.length} ${bar(done, c.tasks.length)}`;
}

const MARK = { passed: "✓", replayed: "✓", failed: "✗", running: "…", waiting: "·" } as const;

/** What follows a task's name: how long it took once ended, else its state. */
function taskDetail(t: OverviewTask): string {
  if (t.state === "replayed") return "replayed";
  if (t.state === "running" || t.state === "waiting") return t.state;
  const took = t.started && t.ended ? (Date.parse(t.ended) - Date.parse(t.started)) / 1000 : NaN;
  return Number.isNaN(took) ? t.state : duration(took);
}

/**
 * The pane's task list: ended tasks in the order they ended, then running
 * ones, then waiting ones, names padded and times right-aligned.
 */
export function taskLines(c: OverviewCandidate): { state: OverviewTask["state"]; text: string }[] {
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
  const detail = Math.max(0, ...order.map((t) => taskDetail(t).length));
  return order.map((t) => ({
    state: t.state,
    text: `${MARK[t.state]} ${t.target.padEnd(name)}  ${taskDetail(t).padStart(detail)}`,
  }));
}

/** The band's line for one branch, or null when it has nothing to show. */
export function branchLine(b: OverviewBranch, now: number): string | null {
  const under = testing(b);
  if (b.depth === 0 && under.length === 0) return null;
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
  return parts.join(" · ");
}

/** The band: one line per branch with a queue; empty when none has. */
export function bandLines(doc: Overview | null, now: number): string[] {
  if (!doc) return [];
  return doc.branches.map((b) => branchLine(b, now)).filter((l): l is string => l !== null);
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

/** The status line: what is queued, and where this session's work stands. */
export function statusLine(doc: Overview | null, mine: Mine): string | undefined {
  if (!doc) return undefined;
  const queued = doc.branches.reduce((n, b) => n + b.depth, 0);
  const own = doc.branches.flatMap((b) => b.submissions).filter((s) => isMine(s, mine));
  if (queued === 0 && own.length === 0) return undefined;
  const yours = own.map((s) => ` · yours #${s.id} ${s.state}`).join("");
  return `sluice: ${plural(queued, "submission")}${yours}`;
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
