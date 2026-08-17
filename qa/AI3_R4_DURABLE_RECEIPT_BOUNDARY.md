# AI3 R4 Blackout Release durable receipt boundary

Status: implementation contract for the current `codex/ai3-r4-blackout-release` tranche. This document does **not** mark R4 accepted by itself.

Authority: `qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md` section 5 requires commit and terminal-receipt persistence to be one correctness boundary. An exact retry after reply loss must not republish an output action, including after a process crash.

## 1. Problem statement

The current output-control receipt store is process memory. `OutputControlTerminalRecord` contains an `Instant` expiry and is inserted into the in-memory `RuntimeControlPlaneState` only after the Engine operation returns. A process crash after a Blackout Release has changed live state but before that terminal record is retained loses the request identity and terminal result.

A post-ACK JSON write is therefore insufficient: it preserves the same crash gap between Engine commit and durable receipt creation.

The Safety Blackout authority is also currently process-local. A fresh Engine starts with a new in-memory authority image, so restart cannot infer whether the previous process reached `Applied`, `NoOp`, or failed before the terminal receipt was retained.

## 2. Required invariant

For one stable principal, operation ID, request ID, and exact request shape, one of the following durable facts must exist before a Blackout Release can become externally committed:

1. `Prepared`: the request identity is reserved durably and may not be admitted as a new request after restart.
2. `Terminal`: the exact terminal response is durable and an exact retry returns it without republishing.
3. `Tombstone`: the request identity is intentionally retained after full receipt expiry and may not be executed implicitly.

There is no state in which a previously admitted request becomes indistinguishable from a never-seen request.

## 3. Stable durable identity

The current in-memory key contains renderer `window_label` and `owner_incarnation`. Those values are deliberately ephemeral and cannot identify a retry across a process restart.

The durable lookup key must therefore be separate from the renderer ownership binding:

```text
DurableRequestKeyV1
  principal_id       stable authenticated principal identity
  operation_id       syndocal.output.blackout.release.v1
  request_id         client-generated JavaScript-safe non-zero integer
```

For the current local-desktop-only AI3 slice, `principal_id` may be one versioned stable local-desktop principal owned by the backend. AI4 must replace/extend that projection with its authenticated per-principal credential identity; it must not persist a renderer HWND, owner incarnation, process incarnation, session secret, or bearer token as the durable principal key.

The original renderer principal, window label, owner incarnation, process/session incarnation, consent binding, and authority fence remain authorization/audit facts. They do not become the cross-process retry key.

A same durable key with a different `shape_sha256` is rejected and never republished.

## 4. Machine-local durable state

Use a dedicated machine-local control-plane state file under the Tauri app-local-data directory. Do not put this state in the project file, Undo/history, recovery project snapshot, or portable show data.

The file is versioned and bounded. Its logical contents are:

```text
R4DurabilityStateV1
  version
  state_generation
  open_session_id
  last_safety_authority
    engaged
    epoch
    generation
  records[]
    durable_request_key
    shape_sha256
    argument_fingerprint
    state = Prepared | Terminal | Tombstone
    fence_before
    predicted_release_fence
    terminal_response?       only for Terminal
```

`records` has a hard capacity. Capacity exhaustion rejects a new R4 command before physical consent is consumed or Engine publication is attempted. Existing identities are never evicted merely to admit a newer request.

Do not use wall-clock time to decide whether a durable identity may be forgotten. Same-boot receipt presentation may keep the existing monotonic `Instant` TTL, but cross-process durable identities remain until an explicitly reviewed compaction/idempotency-epoch mechanism exists. A full durable store is fail-closed (`overloaded`), not an invitation to forget old requests.

## 5. Atomic file replacement

Reuse the existing machine-state persistence discipline:

1. encode the complete next state;
2. create a unique sibling temporary file;
3. write all bytes;
4. `sync_all()` the temporary file;
5. atomically replace the target;
6. on Windows use `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)` as the existing ownership state does;
7. remove a failed temporary file best-effort.

No R4 command may actuate after a failed `Prepared` persistence.

This contract targets process-crash correctness. Power-loss durability requires the same file and directory flush guarantees on each supported filesystem and is a separate native acceptance item rather than an assumption.

## 6. Release transaction

The canonical `execute_blackout_release_v1` path uses this order.

### A. Existing terminal lookup

Before consent or Engine publication:

- exact durable key + exact shape + `Terminal` -> return the durable terminal response;
- exact durable key + different shape -> reject `invalid_request`;
- `Tombstone` -> reject `receipt_expired`;
- unresolved prior-session `Prepared` -> return its recovery terminal result after startup recovery; never republish it;
- no record -> continue.

The existing in-memory lane remains a same-process serialization/cache layer. Durable state is authoritative across process restart.

### B. Ordinary R4 admission

Perform current owner/fence/operator/rate checks and consume the exact single-use physical consent. No durable secret or raw confirmation code is stored.

### C. Durable prepare before actuation

Before sending `SafetyBlackoutReleasePublished`, atomically persist `Prepared` with:

- stable durable key;
- exact shape hash;
- argument fingerprint;
- admitted start fence;
- the deterministic successor fence which would describe `Applied`.

If this write fails, do not publish Release. Return a terminal/local failure only if that failure itself can be represented without forgetting the request; otherwise latch the R4 lane unavailable and require recovery.

### D. Engine publication

Publish Release using the exact admitted Safety Blackout epoch/generation. The S0 priority Engage path is never blocked by the R4 durable store.

If a newer S0 Engage wins the race, Engine returns stale authority and Release does not clear it.

### E. Durable terminal before reply

After definitive Engine acknowledgement and construction of the exact deterministic terminal response, atomically replace `Prepared` with `Terminal` **before** returning the response to the adapter.

Only after durable terminal persistence succeeds may the command reply be considered committed.

If terminal persistence fails after a live Release acknowledgement:

1. do not return an `Applied` receipt;
2. immediately request safer-direction S0 Blackout Engage;
3. keep the durable `Prepared` record;
4. disable further R4 Release admission for the process until durable recovery succeeds;
5. retain the same-process request identity in memory as well.

This converts loss of the durability medium into a fail-closed output state rather than an unrecorded energizing commit.

## 7. Crash recovery

Recovery runs before output ownership can leave its startup-denied/Standby-safe gate.

### Clean prior session

If the prior session closed cleanly and no `Prepared` record exists, load the durable terminal/tombstone cache and continue. The current process/session incarnation is still new; old terminal receipts are historical facts, not current authority grants.

### Unclean prior session or unresolved `Prepared`

An unresolved `Prepared` record is never replayed automatically.

Before physical output can be armed:

1. force Safety Blackout to `engaged=true`;
2. assign a recovery authority strictly newer than both the prepared start authority and its possible Release successor, so a crash cannot reuse the same authority pair with a different engaged bit;
3. durably persist that recovered safe authority;
4. convert each unresolved `Prepared` to a terminal `interrupted_before_commit` result, or an equivalent dedicated typed recovery code reviewed into the protocol;
5. only then allow normal startup/output-arm workflows.

The recovery terminal result is exact for the recovered transaction: the previous process never reached the durable commit boundary and the backend deliberately rolled the ambiguous live state to the safer Blackout-on state instead of guessing that Release committed.

A process restart itself may therefore create a newer safety authority. An older durable `Applied` receipt remains true about its historical commit even if a later recovery/startup Blackout makes the current snapshot blacked out again.

## 8. Why the recovery authority must advance

Using the old `fence_before` authority with `engaged=true` after a crash is invalid because the previous process may have transiently reached the predicted Release successor with `engaged=false`.

Recovery therefore uses a successor newer than every authority state the unresolved transaction could have reached. This prevents ABA reuse and makes old consent/output fences stale by construction.

## 9. S0 interaction

S0 Safety Blackout Engage remains priority queued and safer-direction-only.

A later S0 Engage may race an R4 Release. The Engine exact epoch/generation check decides the winner. The durable R4 layer never converts a stale failure into `Applied`.

On an unclean process restart, the recovery gate defaults toward Blackout-on before output arm. This also prevents an S0 Engage that occurred shortly before a crash from being lost merely because its in-memory authority was not durably written before process death.

## 10. Required protocol/runtime additions

Before R4 can be accepted, implementation needs:

- a stable durable principal ID distinct from renderer owner incarnation;
- a versioned machine-local R4 durability state;
- atomic load/store with strict unknown-field rejection and corruption fail-closed behavior;
- durable `Prepared`, `Terminal`, and `Tombstone` request states;
- a dedicated typed `interrupted_before_commit`/recovery terminal error rather than overloading `internal`;
- startup recovery before output arm;
- an R4 process-local durability-failed latch;
- exact retry lookup from durable storage before new admission;
- bounded capacity which never evicts an identity to make room;
- clean-session marker semantics sufficient to distinguish orderly shutdown from crash recovery.

Secrets, physical confirmation digits, consent tokens, pairing material, raw project/media content, and filesystem paths are not persisted in the R4 receipt state.

## 11. Required tests

The tranche is not accepted without non-vacuous tests for at least:

1. crash simulation after durable `Prepared` and before Engine publication -> restart Blackout-on, one interrupted terminal, no Release republish;
2. crash simulation after Engine Release acknowledgement but before terminal persistence -> restart Blackout-on, interrupted terminal, no Release republish;
3. terminal persistence before reply loss -> restart exact retry returns byte-equivalent terminal receipt and does not publish Engine state;
4. same durable key with a different shape -> rejected after restart;
5. unresolved `Prepared` plus a racing S0 successor -> recovery authority is strictly newer and engaged;
6. corrupt/truncated/unknown-version durability file -> output arm remains fail-closed and R4 unavailable;
7. durability store at capacity -> new R4 rejected before consent consumption/actuation while old retries still resolve;
8. failed terminal persistence -> S0 re-engage attempted, R4 durability-failed latch set, no success receipt returned;
9. clean shutdown -> next startup does not synthesize an interrupted record;
10. process restart changes renderer owner/process incarnation without losing durable terminal lookup by stable principal ID.

Windows native acceptance must additionally prove the durability file is from the current release executable and that startup recovery completes before any output-arm path becomes available.

## 12. Physical-output caveat

This boundary makes backend command/receipt state crash-consistent. It cannot by itself make an external DMX receiver transactional across host power loss or a dead process: a fixture/node may hold the last transmitted frame while Syndocal is down. Hardware/network fail-safe behavior must be covered by the output hardware acceptance matrix. The backend must nevertheless never use that physical limitation as a reason to replay an ambiguous R4 command automatically.
