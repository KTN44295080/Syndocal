import type { ProjectAuthorityToken } from './projectAuthority';

// Retry a revision conflict only when the server still has the exact mapping
// baseline. Foreign mapping edits and project/history replacements cannot rebase.
export const projectControlMappingsCasRecoveryIsRetryable = (state: {
  error: string;
  expected: ProjectAuthorityToken;
  recovered: ProjectAuthorityToken;
  originalIdentity: number;
  currentIdentity: number;
  responseIsCurrent: boolean;
  baselineSignature: string | null;
  recoveredSignature: string;
  mappingReplacementGeneration?: number;
  observedMappingReplacementGeneration: number;
}): boolean => /^(?:Error:\s*)?Project mappings are stale \(/.test(state.error)
  && state.responseIsCurrent
  && state.originalIdentity === state.currentIdentity
  && state.expected.project_epoch === state.recovered.project_epoch
  && state.recovered.project_revision > state.expected.project_revision
  && state.baselineSignature !== null
  && state.baselineSignature === state.recoveredSignature
  && state.mappingReplacementGeneration === state.observedMappingReplacementGeneration;
