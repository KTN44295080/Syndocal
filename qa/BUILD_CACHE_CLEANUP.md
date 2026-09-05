# Build/cache cleanup safety boundary

Use only when cleanup is part of the task; normal edits and checkpoints do not
require inventory, deletion, or dependency restoration.

- Inventory size and identify verified-regenerable obsolete trees. Resolve each
  absolute deletion target inside its explicitly intended workspace/cache root.
  Preserve current release/QA/evidence artifacts and user-authored files.
- Delete an obsolete checkpoint's build/cache trees only after its owning
  checkpoint is committed and pushed.
- A cleanup harness may delete only when tracked at that checkpoint, its focused
  safety tests pass, and independent adversarial review has accepted its exact
  recurring target set. Added targets need a new exact-path safety review.
  Otherwise do not run harness-driven deletion; record the blocker.
- Record exact paths and reclaimed bytes. After deleting a worktree, package
  store, `node_modules`, or cache that may share links with the active checkout,
  immediately verify its dependency graph and restore/rebuild the affected
  surface from the frozen lockfile before claiming cleanup safe. Record
  restoration separately from reclaimed bytes.
