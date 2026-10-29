// The parts of `sluice overview --json` (schema sluice/overview@1) the board
// reads, and the values it keeps for the session.

/** What a recent line of the branch's log carries, whatever its event. */
export type RecentLine = {
  schema?: string;
  at: string;
  id?: number;
  ids?: number[];
  to?: string;
  combination?: string;
  /** A conflict's: the submissions in the way. */
  with?: number[];
};

/** A landing or a blame; or a conflict, whose files the overview always writes. */
export type Recent = RecentLine &
  (
    | { event: "landed"; paths?: undefined }
    | { event: "blamed"; paths?: undefined }
    | { event: "conflict"; paths: string[] }
  );

export type OverviewSubmission = {
  id: number;
  state: string;
  from: { agent: string; worktree: string; path?: string };
  failed?: OverviewFailed[];
};

/** A failed test: the system it failed on, where the policy names one. */
export type OverviewFailed = { task: string; log?: string; system?: string; flaky?: Flaky };

/**
 * A check read as failed for being flaky (19): this run's outcome, when and
 * where it ran, and the run it contradicts. A time or host is absent from a
 * record made before sluice kept them.
 */
export type Flaky = {
  here: "pass" | "fail";
  at?: string;
  host?: string;
  other: {
    outcome: "pass" | "fail";
    at: string;
    branch: string;
    combination: string;
    worktree?: string;
    log?: string;
    host?: string;
  };
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
  /** The git remote the branch's policy names, absent for a branch kept in this host's files. */
  remote?: string;
  /** The worktree each submission the recent lines name came from, by id. */
  recent_from?: Record<string, string>;
  /** Its checkouts whose release failed; absent from a sluice older than them. */
  held_checkouts?: HeldCheckout[];
};

/** A place's release that failed (123): what it printed and how often; `stopped` once tried no more. */
export type ReleaseFailure = { failed: string; tries: number; at: string; stopped: boolean };

/** A checkout of a branch whose release failed, as its state names it; `host` on a branch on its remote. */
export type HeldCheckout = ReleaseFailure & { place: string; host?: string | null };

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
  release_failed?: ReleaseFailure;
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
  previews: OverviewPreview[];
  /** Absent from a sluice older than agents' checks in the overview. */
  room?: Room;
  /** The hosts serving a remote, by their last sign of life. */
  hosts?: OverviewHost[];
  /** `false` on a watch's last line where the repository has no sluice (129); absent otherwise. */
  set_up?: false;
};

/** A preview of a landed revision, and the addresses its stack answers on. */
export type OverviewPreview = {
  name: string;
  branch: string;
  revision: string;
  state: string;
  /** Label to address, in the policy's order. */
  addresses?: Record<string, string>;
  release_failed?: ReleaseFailure;
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
  branch: string;
  id: number;
  word: "landed" | "failed" | "conflict";
  to?: string;
  combination?: string;
  paths?: string[];
};

/** A forecast, as `sluice submit <branch> --check --json` prints it (64, 65). */
export type Forecast = {
  conflict: {
    paths: string[];
    with: { id: number; from: { agent: string; worktree: string } }[];
  } | null;
  head?: string;
  onto?: string;
  ahead?: number[];
};

/** What the board last forecast on, and the conflict it told of. */
export type Forecasting = {
  /** The head, the branch's `at` and the others waiting, as last forecast. */
  key: string | null;
  head: string | null;
  /** The conflict told, by its signature, or null. */
  told: string | null;
  /** The branch the last forecast found a conflict on, or null. */
  branch: string | null;
  /** That conflict, or null. */
  conflict: Forecast["conflict"];
};

/** The session's latest submission, as the document shows it. */
export type Latest = { id: number; branch: string; conflict: boolean; active: boolean };

/** A log as an opened failure shows it: its path, its last lines, or why not. */
export type OpenedLog = { log?: string; tail?: string[]; error?: string };

/**
 * A failed test as an opened failure shows it: its log's last lines, or why
 * not; and, for a flaky one, the run it contradicts with its log.
 */
export type OpenedTest = OpenedLog & { task: string; flaky?: Flaky; other?: OpenedLog };

declare module "claude-code" {
  interface PluginState {
    "sluice-board": {
      doc: Overview | null;
      mine: Mine;
      isHidden: boolean;
      now: number;
      /** Each remote's `https://github.com/<owner>/<repo>`, or null when it is not on GitHub. */
      remotes: Record<string, string | null>;
      /** The keys of the failures opened in place on the pane. */
      opened: string[];
      /** What each opened failure showed when it was opened. */
      details: Record<string, OpenedTest[]>;
      /** The branch `SLUICE_BRANCH` named for the session, or null. */
      named: string | null;
      /** The session's worktree's last forecast. */
      forecasting: Forecasting;
      /** The band's line for the overview's last error, until a document comes. */
      error: string | null;
      /** Why the board's watch gave no overview, while it has none; null once one comes. */
      unread: string | null;
      /**
       * The host's sluice has no watch: the first line its `--version` printed,
       * or null when it printed none; the whole null once a watch prints an overview.
       */
      cannot: { version: string | null } | null;
      /**
       * The last watch said the repository has no sluice (129): nothing of
       * sluice's is shown, and only `/sluice` starts the next watch.
       */
      absent: boolean;
      /**
       * What the board said where the host's sluice is outside the releases
       * `.claude-plugin/sluice.json` names, kept so it is said once a session.
       */
      refused: string | null;
    };
  }
}
