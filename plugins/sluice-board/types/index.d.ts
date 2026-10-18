// The parts of `sluice overview --json` (schema sluice/overview@1) the board
// reads, and the values it keeps for the session.

export type Recent = {
  schema?: string;
  at: string;
  event: string;
  id?: number;
  ids?: number[];
  to?: string;
  combination?: string;
  paths?: string[];
};

export type OverviewSubmission = {
  id: number;
  state: string;
  from: { agent: string; worktree: string; path?: string };
  failed?: { task: string; log?: string }[];
};

/** One task of a run under test, as its runner says while it goes. */
export type OverviewTask = {
  target: string;
  state: "waiting" | "running" | "passed" | "failed" | "replayed";
  started?: string;
  ended?: string;
  exit?: number;
};

export type OverviewCandidate = {
  id: string;
  submissions: number[];
  revision: string;
  state: string;
  started?: string;
  limit?: number;
  targets: string[];
  executor: string | null;
  ran: number;
  replayed: number;
  failed: { task: string; log?: string }[];
  /** Present while the runner says how far its tasks have got. */
  tasks?: OverviewTask[];
};

export type OverviewBranch = {
  branch: string;
  at: string | null;
  version: number;
  window: number;
  parallel: number | null;
  depth: number;
  candidates: OverviewCandidate[];
  submissions: OverviewSubmission[];
  recent: Recent[];
};

export type Overview = {
  schema: string;
  host: string;
  repository: string;
  at: string;
  branches: OverviewBranch[];
  sessions: { worktree: string; branch: string; removed: boolean }[];
  previews: { name: string; state: string }[];
};

/** How the board knows this session's submissions. */
export type Mine = {
  /** The session's worktree root, matched against `from.path`. */
  root: string | null;
  /** Submission ids seen from this session's submits, or matched before. */
  ids: number[];
  /** `pull/<n>` of the pull requests this session's submits opened. */
  pulls: string[];
};

export type Outcome = {
  key: string;
  branch: string;
  id: number;
  word: "landed" | "failed" | "conflict";
  to?: string;
  combination?: string;
  paths?: string[];
};

/** A failed test as an opened failure shows it: its log's last lines, or why not. */
export type OpenedTest = { task: string; log?: string; tail?: string[]; error?: string };

declare module "claude-code" {
  interface PluginState {
    "sluice-board": {
      doc: Overview | null;
      mine: Mine;
      told: string[];
      isHidden: boolean;
      now: number;
      /** `https://github.com/<owner>/<repo>` when origin is on GitHub. */
      repo: string | null;
      /** The keys of the failures opened in place on the pane. */
      opened: string[];
      /** What each opened failure showed when it was opened. */
      details: Record<string, OpenedTest[]>;
    };
  }
}
