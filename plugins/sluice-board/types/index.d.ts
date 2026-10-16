// The parts of `sluice overview --json` (schema sluice/overview@1) the board
// reads, and the values it keeps for the session.

export type Recent = {
  schema?: string
  at: string
  event: string
  id?: number
  ids?: number[]
  to?: string
  combination?: string
  paths?: string[]
}

export type OverviewSubmission = {
  id: number
  state: string
  from: { agent: string; worktree: string; path?: string }
  failed?: { task: string; log?: string }[]
}

export type OverviewCandidate = {
  id: string
  submissions: number[]
  revision: string
  state: string
  started?: string
  limit?: number
  targets: string[]
  executor: string | null
  ran: number
  replayed: number
  failed: { task: string; log?: string }[]
}

export type OverviewBranch = {
  branch: string
  at: string | null
  version: number
  window: number
  parallel: number | null
  depth: number
  candidates: OverviewCandidate[]
  submissions: OverviewSubmission[]
  recent: Recent[]
}

export type Overview = {
  schema: string
  host: string
  repository: string
  at: string
  branches: OverviewBranch[]
  sessions: { worktree: string; branch: string; removed: boolean }[]
  previews: { name: string; state: string }[]
}

/** How the board knows this session's submissions. */
export type Mine = {
  /** The session's worktree root, matched against `from.path`. */
  root: string | null
  /** Submission ids seen from this session's submits, or matched before. */
  ids: number[]
  /** `pull/<n>` of the pull requests this session's submits opened. */
  pulls: string[]
}

export type Outcome = {
  key: string
  branch: string
  id: number
  word: 'landed' | 'failed' | 'conflict'
  to?: string
  combination?: string
  paths?: string[]
}

declare module 'claude-code' {
  interface PluginState {
    'sluice-board': {
      doc: Overview | null
      mine: Mine
      told: string[]
      isHidden: boolean
      now: number
      /** `https://github.com/<owner>/<repo>` when origin is on GitHub. */
      repo: string | null
    }
  }
}
