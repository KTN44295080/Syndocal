import type { ProjectAuthorityToken } from "./projectAuthority";

const same = (a: ProjectAuthorityToken, b: ProjectAuthorityToken) =>
  a.project_epoch === b.project_epoch
  && a.project_revision === b.project_revision
  && a.checkpoint_hash === b.checkpoint_hash;

/** Local metadata only. Each member retains its own authoritative publication. */
export const createProjectUndoGroup = (
  initial: ProjectAuthorityToken,
  id = globalThis.crypto.randomUUID(),
) => {
  let expected = { ...initial };
  let open = true;
  const isCurrent = (current: ProjectAuthorityToken): boolean => {
    if (!open || !same(expected, current)) {
      open = false;
      return false;
    }
    return true;
  };
  return {
    id,
    isCurrent,
    assertCurrent(current: ProjectAuthorityToken) {
      if (!isCurrent(current)) throw new Error("Project changed during folder relink. Review the remaining files and try again.");
    },
    acceptPublication(terminal: ProjectAuthorityToken, current: ProjectAuthorityToken): boolean {
      if (!open || terminal.project_epoch !== expected.project_epoch
        || terminal.project_revision < expected.project_revision || !same(terminal, current)
        || (terminal.project_revision === expected.project_revision && terminal.checkpoint_hash !== expected.checkpoint_hash)) {
        open = false;
        return false;
      }
      expected = { ...terminal };
      return true;
    },
    close() { open = false; },
  };
};
export type ProjectUndoGroup = ReturnType<typeof createProjectUndoGroup>;
