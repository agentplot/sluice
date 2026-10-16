// What the board shows, from `sluice overview --json` (sluice/overview@1):
// pure functions over the document, so the tests need no sluice running.

import type { Mine, Outcome, Overview, OverviewBranch, Recent } from '../types'

/** `2m10s`, `45s`, `1h03m`. */
export function duration(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds))
  if (s < 60) return `${s}s`
  if (s < 3600) return `${Math.floor(s / 60)}m${String(s % 60).padStart(2, '0')}s`
  return `${Math.floor(s / 3600)}h${String(Math.floor((s % 3600) / 60)).padStart(2, '0')}m`
}

/** `15m` for a limit: whole minutes where it divides. */
export function limit(seconds: number): string {
  return seconds % 60 === 0 && seconds < 3600 ? `${seconds / 60}m` : duration(seconds)
}

export function ids(list: readonly number[]): string {
  return list.map(i => `#${i}`).join('+')
}

/** The combinations under test on a branch. */
export function testing(b: OverviewBranch) {
  return b.candidates.filter(c => c.state === 'testing' || c.state === 'starting')
}

/** Seconds since an RFC 3339 time, at `now` (ms since the epoch). */
export function since(at: string | undefined, now: number): number | undefined {
  if (!at) return undefined
  const t = Date.parse(at)
  return Number.isNaN(t) ? undefined : (now - t) / 1000
}

/** What a recent log line says, in a few words. */
export function recentWords(r: Recent): string {
  const short = (v: unknown) => (typeof v === 'string' ? v.slice(0, 7) : '')
  switch (r.event) {
    case 'landed':
      return `${(r.ids ?? []).map(i => `#${i}`).join(' ')} landed as ${short(r.to)}`
    case 'blamed':
      return `#${r.id} blamed on ${short(r.combination)}`
    case 'conflict':
      return `#${r.id} conflict · ${(r.paths ?? []).join(', ')}`
    default:
      return String(r.event)
  }
}

/** The band's line for one branch, or null when it has nothing to show. */
export function branchLine(b: OverviewBranch, now: number): string | null {
  const under = testing(b)
  if (b.depth === 0 && under.length === 0) return null
  const parts = [`sluice ${b.branch}`, `${b.depth} queued`]
  for (const c of under) {
    const took = since(c.started, now)
    const clock =
      took === undefined ? '' : ` ${duration(took)}${c.limit ? `/${limit(c.limit)}` : ''}`
    parts.push(`testing ${ids(c.submissions)}${clock}`)
  }
  const last = b.recent[b.recent.length - 1]
  if (last) parts.push(`last ${recentWords(last)}`)
  return parts.join(' · ')
}

/** The band: one line per branch with a queue; empty when none has. */
export function bandLines(doc: Overview | null, now: number): string[] {
  if (!doc) return []
  return doc.branches.map(b => branchLine(b, now)).filter((l): l is string => l !== null)
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
  )
}

/** `mine` with every active submission of the document that is this session's. */
export function learn(doc: Overview, mine: Mine): Mine {
  const found = doc.branches
    .flatMap(b => b.submissions)
    .filter(s => isMine(s, mine))
    .map(s => s.id)
  const more = found.filter(i => !mine.ids.includes(i))
  return more.length === 0 ? mine : { ...mine, ids: [...mine.ids, ...more] }
}

/** A recent line's identity, so an outcome is told once. */
export function key(branch: string, r: Recent): string {
  return `${branch}:${r.event}:${r.id ?? (r.ids ?? []).join(',')}:${r.at}`
}

/** Every recent line of the document, keyed. */
export function recentKeys(doc: Overview): string[] {
  return doc.branches.flatMap(b => b.recent.map(r => key(b.branch, r)))
}

/**
 * The outcomes of this session's submissions the document shows that have
 * not been told: a landing, a blame or a conflict naming one of `mine.ids`.
 */
export function outcomes(doc: Overview, mine: Mine, told: readonly string[]): Outcome[] {
  const out: Outcome[] = []
  for (const b of doc.branches) {
    for (const r of b.recent) {
      const k = key(b.branch, r)
      if (told.includes(k)) continue
      if (r.event === 'landed') {
        for (const id of r.ids ?? []) {
          if (mine.ids.includes(id)) {
            out.push({ key: k, branch: b.branch, id, word: 'landed', to: String(r.to ?? '') })
          }
        }
      } else if ((r.event === 'blamed' || r.event === 'conflict') && r.id !== undefined) {
        if (mine.ids.includes(r.id)) {
          out.push({
            key: k,
            branch: b.branch,
            id: r.id,
            word: r.event === 'blamed' ? 'failed' : 'conflict',
            combination: typeof r.combination === 'string' ? r.combination : undefined,
            paths: r.paths,
          })
        }
      }
    }
  }
  return out
}

/** What `sluice submit` printed that names this session's work. */
export function submitted(stdout: string): { ids: number[]; pulls: string[] } {
  const ids = [...stdout.matchAll(/submission (\d+) to /g), ...stdout.matchAll(/"id":\s*(\d+)/g)].map(m =>
    Number(m[1]),
  )
  const pulls = [...stdout.matchAll(/(?:opened|updated) #(\d+)/g), ...stdout.matchAll(/"pull_request":\s*(\d+)/g)].map(
    m => `pull/${m[1]}`,
  )
  return { ids, pulls }
}

/** The status line: what is queued, and where this session's work stands. */
export function statusLine(doc: Overview | null, mine: Mine): string | undefined {
  if (!doc) return undefined
  const queued = doc.branches.reduce((n, b) => n + b.depth, 0)
  const own = doc.branches.flatMap(b => b.submissions).filter(s => isMine(s, mine))
  if (queued === 0 && own.length === 0) return undefined
  const yours = own.map(s => ` · yours #${s.id} ${s.state}`).join('')
  return `sluice: ${queued} queued${yours}`
}

/** The prompt that tells an idle session its submission's outcome. */
export function wakeText(o: Outcome, failed: { task: string; log?: string }[]): string {
  if (o.word === 'landed') {
    return `[sluice] Your submission #${o.id} landed on ${o.branch} as ${(o.to ?? '').slice(0, 7)}.`
  }
  if (o.word === 'conflict') {
    return [
      `[sluice] Your submission #${o.id} to ${o.branch} conflicts: ${(o.paths ?? []).join(', ')}.`,
      `Merge ${o.branch} into your worktree, resolve, commit, and submit again (check first with \`sluice submit --check ${o.branch}\`).`,
    ].join('\n')
  }
  const tasks = failed.map(f => f.task).join(', ') || 'its tests'
  const logs = failed.filter(f => f.log).map(f => `Log of ${f.task}: ${f.log}`)
  return [
    `[sluice] Your submission #${o.id} to ${o.branch} failed: ${tasks}${o.combination ? ` on ${o.combination.slice(0, 7)}` : ''}.`,
    ...logs,
    `Read the log, reproduce with \`sluice exec ${o.branch}\`, fix, commit, and submit again.`,
  ].join('\n')
}
