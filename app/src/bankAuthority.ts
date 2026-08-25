import type { CueListSummary, CueSummary, PlaybackExecutorSummary } from "./types";

export interface BankAuthorityCueIdentity {
  id: number;
  cue_list_id: number;
}

export interface BankAuthorityExecutorIdentity {
  id: number;
  cue_list_id: number;
}

export type BankAuthorityIssue =
  | { kind: "invalid_bank_id"; id: number; index: number }
  | { kind: "duplicate_bank_id"; id: number; firstIndex: number; index: number }
  | { kind: "invalid_cue_id"; id: number; index: number }
  | { kind: "duplicate_cue_id"; id: number; firstIndex: number; index: number }
  | { kind: "invalid_cue_bank_id"; cueId: number; id: number; index: number }
  | { kind: "missing_cue_bank"; cueId: number; id: number; index: number }
  | { kind: "invalid_bank_active_cue_id"; bankId: number; id: number; index: number }
  | { kind: "missing_bank_active_cue"; bankId: number; cueId: number; index: number }
  | { kind: "cross_bank_active_cue"; bankId: number; cueId: number; cueBankId: number; index: number }
  | { kind: "invalid_executor_id"; id: number; index: number }
  | { kind: "duplicate_executor_id"; id: number; firstIndex: number; index: number }
  | { kind: "invalid_executor_bank_id"; executorId: number; id: number; index: number }
  | { kind: "missing_executor_bank"; executorId: number; id: number; index: number };

export interface BankAuthoritySnapshot<
  CueT extends BankAuthorityCueIdentity,
  ExecutorT extends BankAuthorityExecutorIdentity = BankAuthorityExecutorIdentity,
> {
  /** Exact engine-owned rows. Labels are never rewritten or normalized. */
  cueLists: readonly CueListSummary[];
  /** Exact engine-owned Cue rows used by this surface. */
  cues: readonly CueT[];
  executors: readonly ExecutorT[];
  issue: BankAuthorityIssue | null;
  cueListById: ReadonlyMap<number, CueListSummary>;
  cueById: ReadonlyMap<number, CueT>;
  executorById: ReadonlyMap<number, ExecutorT>;
}

export type FullBankAuthoritySnapshot = BankAuthoritySnapshot<CueSummary, PlaybackExecutorSummary>;

const validIdentity = (id: number) => Number.isSafeInteger(id) && id > 0;

/**
 * Builds the one frontend Bank authority used by editing, placement, and
 * playback-executor surfaces. One invalid/duplicate primary key, or one Cue
 * that cannot resolve its exact Bank foreign key, invalidates the complete
 * snapshot. Callers must not continue with the valid-looking subset.
 */
export const inspectBankAuthority = <
  CueT extends BankAuthorityCueIdentity,
  ExecutorT extends BankAuthorityExecutorIdentity = BankAuthorityExecutorIdentity,
>(
  cueLists: readonly CueListSummary[],
  cues: readonly CueT[] = [],
  executors: readonly ExecutorT[] = [],
): BankAuthoritySnapshot<CueT, ExecutorT> => {
  const cueListById = new Map<number, CueListSummary>();
  const cueListFirstIndex = new Map<number, number>();
  let issue: BankAuthorityIssue | null = null;

  for (let index = 0; index < cueLists.length; index += 1) {
    const cueList = cueLists[index];
    if (!validIdentity(cueList.id)) {
      issue ??= { kind: "invalid_bank_id", id: cueList.id, index };
      continue;
    }
    const firstIndex = cueListFirstIndex.get(cueList.id);
    if (firstIndex !== undefined) {
      issue ??= { kind: "duplicate_bank_id", id: cueList.id, firstIndex, index };
      continue;
    }
    cueListFirstIndex.set(cueList.id, index);
    cueListById.set(cueList.id, cueList);
  }

  const cueById = new Map<number, CueT>();
  const cueFirstIndex = new Map<number, number>();
  for (let index = 0; index < cues.length; index += 1) {
    const cue = cues[index];
    if (!validIdentity(cue.id)) {
      issue ??= { kind: "invalid_cue_id", id: cue.id, index };
      continue;
    }
    const firstIndex = cueFirstIndex.get(cue.id);
    if (firstIndex !== undefined) {
      issue ??= { kind: "duplicate_cue_id", id: cue.id, firstIndex, index };
      continue;
    }
    cueFirstIndex.set(cue.id, index);
    cueById.set(cue.id, cue);
    if (!validIdentity(cue.cue_list_id)) {
      issue ??= {
        kind: "invalid_cue_bank_id",
        cueId: cue.id,
        id: cue.cue_list_id,
        index,
      };
    } else if (!cueListById.has(cue.cue_list_id)) {
      issue ??= {
        kind: "missing_cue_bank",
        cueId: cue.id,
        id: cue.cue_list_id,
        index,
      };
    }
  }

  for (let index = 0; index < cueLists.length; index += 1) {
    const cueList = cueLists[index];
    const activeCueId = cueList.active_cue_id;
    if (activeCueId === null || activeCueId === undefined) continue;
    if (!validIdentity(activeCueId)) {
      issue ??= {
        kind: "invalid_bank_active_cue_id",
        bankId: cueList.id,
        id: activeCueId,
        index,
      };
      continue;
    }
    const activeCue = cueById.get(activeCueId);
    if (!activeCue) {
      issue ??= {
        kind: "missing_bank_active_cue",
        bankId: cueList.id,
        cueId: activeCueId,
        index,
      };
    } else if (activeCue.cue_list_id !== cueList.id) {
      issue ??= {
        kind: "cross_bank_active_cue",
        bankId: cueList.id,
        cueId: activeCueId,
        cueBankId: activeCue.cue_list_id,
        index,
      };
    }
  }

  const executorById = new Map<number, ExecutorT>();
  const executorFirstIndex = new Map<number, number>();
  for (let index = 0; index < executors.length; index += 1) {
    const executor = executors[index];
    if (!validIdentity(executor.id)) {
      issue ??= { kind: "invalid_executor_id", id: executor.id, index };
      continue;
    }
    const firstIndex = executorFirstIndex.get(executor.id);
    if (firstIndex !== undefined) {
      issue ??= { kind: "duplicate_executor_id", id: executor.id, firstIndex, index };
      continue;
    }
    executorFirstIndex.set(executor.id, index);
    executorById.set(executor.id, executor);
    if (!validIdentity(executor.cue_list_id)) {
      issue ??= {
        kind: "invalid_executor_bank_id",
        executorId: executor.id,
        id: executor.cue_list_id,
        index,
      };
    } else if (!cueListById.has(executor.cue_list_id)) {
      issue ??= {
        kind: "missing_executor_bank",
        executorId: executor.id,
        id: executor.cue_list_id,
        index,
      };
    }
  }

  return Object.freeze({
    cueLists,
    cues,
    executors,
    issue,
    cueListById,
    cueById,
    executorById,
  });
};

export const bankAuthorityIssueMessage = (issue: BankAuthorityIssue): string => {
  switch (issue.kind) {
    case "invalid_bank_id":
      return `Timeline Bank L${issue.id} is unavailable because its identity is invalid`;
    case "duplicate_bank_id":
      return `Timeline Bank L${issue.id} is unavailable because its identity is duplicated`;
    case "invalid_cue_id":
      return `Scene ${issue.id} is unavailable because its Scene identity is invalid`;
    case "duplicate_cue_id":
      return `Scene ${issue.id} is unavailable because its Scene identity is duplicated`;
    case "invalid_cue_bank_id":
      return `Scene ${issue.cueId} is unavailable because its Bank identity is invalid`;
    case "missing_cue_bank":
      return `Scene ${issue.cueId} is unavailable because its Bank is missing`;
    case "invalid_bank_active_cue_id":
      return `Timeline Bank L${issue.bankId} is unavailable because its active Scene identity is invalid`;
    case "missing_bank_active_cue":
      return `Timeline Bank L${issue.bankId} is unavailable because active Scene ${issue.cueId} is missing`;
    case "cross_bank_active_cue":
      return `Timeline Bank L${issue.bankId} is unavailable because active Scene ${issue.cueId} belongs to Bank L${issue.cueBankId}`;
    case "invalid_executor_id":
      return `Playback Executor ${issue.id} is unavailable because its identity is invalid`;
    case "duplicate_executor_id":
      return `Playback Executor ${issue.id} is unavailable because its identity is duplicated`;
    case "invalid_executor_bank_id":
      return `Playback Executor ${issue.executorId} is unavailable because its Bank identity is invalid`;
    case "missing_executor_bank":
      return `Playback Executor ${issue.executorId} is unavailable because its Bank is missing`;
  }
};

export const bankAuthoritySelectedCueList = <
  CueT extends BankAuthorityCueIdentity,
  ExecutorT extends BankAuthorityExecutorIdentity,
>(
  authority: BankAuthoritySnapshot<CueT, ExecutorT>,
  selectedCueListId: number | null,
): CueListSummary | null => {
  if (authority.issue) return null;
  if (selectedCueListId === null) return authority.cueLists[0] ?? null;
  return authority.cueListById.get(selectedCueListId) ?? null;
};

/**
 * Semantic identity of the complete frontend Bank authority. This is captured
 * beside delayed commands so an intervening invalid, stale, or replaced
 * authority cannot be hidden by later rows that happen to reuse the same IDs.
 */
export const bankAuthoritySemanticToken = <
  CueT extends BankAuthorityCueIdentity,
  ExecutorT extends BankAuthorityExecutorIdentity,
>(authority: BankAuthoritySnapshot<CueT, ExecutorT>): string => JSON.stringify({
  issue: authority.issue,
  banks: authority.cueLists.map((bank) => [bank.id, bank.label, bank.active_cue_id ?? null]),
  cues: authority.cues.map((cue) => [cue.id, cue.cue_list_id]),
  executors: authority.executors.map((executor) => [executor.id, executor.cue_list_id]),
});

export interface BankAuthorityProjectCheckpoint {
  project_epoch: number;
  project_revision: number;
  checkpoint_hash: string;
}

export interface BankAuthorityDelayCapture extends BankAuthorityProjectCheckpoint {
  semantic_token: string;
  generation: number;
}

const sameProjectCheckpoint = (
  left: BankAuthorityProjectCheckpoint,
  right: BankAuthorityProjectCheckpoint,
) => left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.checkpoint_hash === right.checkpoint_hash;

/**
 * A monotonic fence for delayed Scene commands. Observing fault then repair
 * advances twice, so an old command cannot revive when identical IDs return.
 */
export const createBankAuthorityDelayFence = (initialSemanticToken: string) => {
  let semanticToken = initialSemanticToken;
  let generation = 0;
  let exhausted = false;
  return {
    observe: (nextSemanticToken: string): boolean => {
      if (exhausted || nextSemanticToken === semanticToken) return false;
      semanticToken = nextSemanticToken;
      if (!Number.isSafeInteger(generation) || generation >= Number.MAX_SAFE_INTEGER) {
        exhausted = true;
        return true;
      }
      generation += 1;
      return true;
    },
    capture: (project: BankAuthorityProjectCheckpoint): BankAuthorityDelayCapture => ({
      ...project,
      semantic_token: semanticToken,
      generation,
    }),
    isCurrent: (
      captured: BankAuthorityDelayCapture,
      project: BankAuthorityProjectCheckpoint,
      currentSemanticToken: string,
    ): boolean => !exhausted
      && captured.generation === generation
      && captured.semantic_token === semanticToken
      && currentSemanticToken === semanticToken
      && sameProjectCheckpoint(captured, project),
  };
};

export const nextBankAuthorityId = (
  existingIds: Iterable<number>,
): number | null => {
  let maximum = 0;
  for (const id of existingIds) {
    if (!validIdentity(id)) return null;
    maximum = Math.max(maximum, id);
  }
  const next = maximum + 1;
  return validIdentity(next) ? next : null;
};
