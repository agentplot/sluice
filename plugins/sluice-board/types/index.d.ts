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
  failed?: OverviewFailed[];
};

/** A failed test: the system it failed on, where the policy names one. */
export type OverviewFailed = { task: string; log?: string; system?: string };

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
  failed: OverviewFailed[];
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

/** How a worktree's last `sluice exec` ended (ADR 0027). */
export type LastResult = {
  at: string;
  head: string;
  kind: "checks" | "command";
  outcome: "pass" | "fail" | "timed_out" | "lost" | "stopped" | "not_run";
  /** Its results are under their keys. */
  recorded: boolean;
  /** The finished run's tasks, from its report. */
  tasks?: OverviewTask[];
  /** The gate takes its passes when its head is submitted. */
  reused?: boolean;
};

/** An agent's `sluice exec` going in its worktree (ADR 0035). */
export type SessionRun = {
  started: string;
  /** The command, for `sluice exec <branch> -- <command>`. */
  command?: string;
  tasks?: OverviewTask[];
};

/** A worktree sluice holds, or one that has a branch sluice lands on checked out. */
export type OverviewSession = {
  worktree: string;
  branch: string;
  /** Present for a worktree whose runs hold ports, absent for one only warned of. */
  ports?: Record<string, string>;
  last_run?: string | null;
  removed?: boolean;
  last_result?: LastResult;
  holds_queued_branch?: string;
  run?: SessionRun;
};

/** The host's bound and what counts against it (session-runs-1 clauses 89, 91). */
export type Room = {
  bound: number;
  /** Combinations under test on the repository's branches. */
  queue: number;
  /** Agents' checks on the host, every repository's. */
  sessions: number;
  /** The part of the bound agents' checks never take. */
  reserve: number;
};

export type Overview = {
  schema: string;
  host: string;
  repository: string;
  at: string;
  branches: OverviewBranch[];
  sessions: OverviewSession[];
  previews: { name: string; state: string }[];
  /** Absent from a sluice older than agents' checks in the overview. */
  room?: Room;
  /** The hosts serving a remote, by their last sign of life. */
  hosts?: OverviewHost[];
};

export type OverviewHost = {
  host: string;
  signer?: string;
  system?: string;
  held: number;
  room: number;
  lapsed: boolean;
  at?: string;
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
