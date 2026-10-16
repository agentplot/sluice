import type { Overview } from '../types'

/** 2026-10-16T19:45:10Z: 130 s after the combination under test started. */
export const NOW = Date.parse('2026-10-16T19:45:10Z')

/** A repository with main testing #12 + #13, #14 queued, and a recent blame. */
export function overview(): Overview {
  return {
    schema: 'sluice/overview@1',
    host: 'mac-studio',
    repository: '/work/sluice/sluice.git',
    at: '2026-10-16T19:45:10Z',
    branches: [
      {
        branch: 'main',
        at: '9ecc34a1b2',
        version: 57,
        window: 1,
        parallel: 4,
        depth: 3,
        candidates: [
          {
            id: 'c41',
            submissions: [12, 13],
            revision: 'e20a6f9b48',
            state: 'testing',
            started: '2026-10-16T19:43:00Z',
            limit: 900,
            targets: ['#check-fast', '#check-tools'],
            executor: 'sandbox',
            ran: 0,
            replayed: 0,
            failed: [],
          },
        ],
        submissions: [
          { id: 12, state: 'testing', from: { agent: 'claude', worktree: 'overview-docs', path: '/work/sluice/overview-docs' } },
          { id: 13, state: 'testing', from: { agent: 'claude', worktree: 'plugin-board', path: '/work/sluice/plugin-board' } },
          { id: 14, state: 'queued', from: { agent: 'herdr', worktree: 'herdr-panel', path: '/work/sluice/herdr-panel' } },
        ],
        recent: [
          { schema: 'sluice/log@1', at: '2026-10-16T19:41:00Z', event: 'landed', ids: [11], to: '9ecc34a1b2' },
          { schema: 'sluice/log@1', at: '2026-10-16T19:42:00Z', event: 'blamed', id: 9, combination: '4f1c2aa0e1' },
        ],
      },
    ],
    sessions: [],
    previews: [],
  }
}
