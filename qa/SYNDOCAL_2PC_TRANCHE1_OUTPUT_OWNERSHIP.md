# Syndocal 2-PC Show Sync — Tranche 1 QA

Date: 2026-08-12

## Scope

This tranche implements machine-local output ownership only. The runtime role is exactly `Lighting`, `Video`, `Both`, or `Standby`; existing behavior defaults to `Both`.

The role is runtime state, not project data. It is not added to `ProjectFile` or `EngineSnapshot`, and role changes do not rewrite authored `.sdc` configuration. Output enforcement is an effective gate at the production sender and transport boundaries.

### Current implementation checkpoint

This document records the completed NDI/Spout ownership-fence and creation-lease tranche plus the completed SDK-free Display creation, live-worker failure-fence, and window-retirement integration currently in the tree. It is **not** a completion claim for native GPU/driver behavior or for project replacement. In particular:

- The NDI and Spout startup paths use bounded acknowledgement waits and preserve pending cleanup state on timeout. The static/source-level and injected tests prove the retention protocol, but do not prove a real NDI or Spout SDK constructor hang, GPU driver hang, or operating-system scheduling behavior.
- Display creation/show is covered by the creation-lease helper and native-window registry/retirement path. The live worker acquires the output permit before resize/render/present, arms the failure fence while that permit is still held, and keeps a teardown lease through stop, window close, worker join, and window-retirement acknowledgement. Cleanup-aware creation/sync paths retain the creation lease fail-closed if partial setup cannot be acknowledged. These guarantees are covered by 14 SDK-free native-video-output tests; real GPU, driver, HDMI, and operating-system teardown behavior remains outside this proof.
- Project/new/recovery/backup snapshot replacement is not yet serialized through a full external-output retirement-and-rearm fence. It must remain an explicit future safety boundary: loading a replacement project is not yet proof that a removed or changed output endpoint has been retired before the new snapshot becomes active.

## Effective ownership contract

| Role | Lighting / DMX | External video |
| --- | --- | --- |
| `Lighting` | allowed | blocked |
| `Video` | blocked | allowed |
| `Both` | allowed | allowed |
| `Standby` | blocked | blocked |

Blocked output is disarmed before packet/frame emission or output-window activation. Authored output configuration remains available for a later role change.

Lighting coverage includes the engine's Art-Net, sACN, serial/USB DMX, reconnect paths, additional configured lighting sender paths, direct DMX test, and RDM/TOD commands. Video coverage includes the NDI/Spout transport workers, native test-pattern output, live Display output, and video output windows. Local file recording is deliberately not owned by this gate: it writes local media and remains available while an external output role is blocked.

The backend status command is authoritative and reports desired, persisted, and effective role; `Ready` / `Transitioning` / `Activating` / `Failed` state; generation and epoch; independent lighting/video allow flags; and a truthful reason/error. Allowed means the serialized transition has completed, not merely that a desired enum was selected. The operator control is in the existing Active / Standby surface. A running Standby session keeps the role selector disabled until it is stopped.

The preferred role is persisted atomically in app-local backend data. Engine startup begins all-deny before setup, so the enabled Art-Net default cannot create a sender before the startup safety decision. A genuinely missing state file selects and persists preferred `Both` only after safe initialization; every launch remains `StartupDenied`/all-deny until the operator explicitly presses `Arm selected role` (or changes the role selection). Corrupt, unsupported-version, unreadable, and persistence failures leave `Standby` all-deny with an authoritative startup/transition failure. Thus an old preferred `Both` file cannot auto-arm after a failed Standby-marker write or crash.

## Active / Standby behavior

Starting Standby sets the effective role to `Standby` before the standby runtime loads its checkpoint, and keeps authored outputs disarmed. Standby loads authored project state without using a second output-mutated project representation.

Stopping Standby does not silently arm outputs. Takeover requires a running Standby session and performs the existing stale-heartbeat / split-brain checks. The 2026-08-19 repair binds authenticated Take Over to the exact consent-validated session/generation and rejects valid, corrupt, or noncanonical newer manifest candidates before worker stop; only the explicit local command retains latest-valid fallback behavior. Its checkpoint replacement is **not yet** held behind one output-ownership fence for the whole load interval, so this tranche does not claim that a takeover checkpoint is loaded while outputs remain continuously fenced in `Standby`. The intended follow-up contract is to retain the Standby reservation through the replacement and require a later explicit arm; until that project-swap fence lands, a Take Over operation must be treated as an incomplete safety boundary.

Role transitions are serialized at the local capability boundary: the shared permit gate denies new sends/presents and waits for in-flight operations. A durable Standby marker is atomically written before runtime transition publication. The transition then fences and drops DMX senders, stops external workers, waits for the acknowledged NDI sender teardown when NDI is enabled, closes every native output window with the video-output- prefix (including stale project IDs), prepares allowed senders/transports under an epoch-bound activation admission, persists the requested role only after preparation succeeds, and publishes Ready last. A timeout or failure leaves the gate Failed/all-deny; a held physical permit must drain before any recovery transition can complete. This is a local capability fence; it is not a distributed owner lease.

External NDI/Spout resource creation and native Display window creation/show are admitted only through the serialized transition mutex plus a matching ownership epoch. Transition-owned video preparation enters an explicit Activating state with all physical permits fenced; uncoordinated callers cannot create or advertise resources during Transitioning, Standby, Activating, or a lighting-only role. A creation lease is admitted after activation validation and remains live across the native constructor and publication, or across resource retirement. `publish` rechecks the epoch and fence. For NDI and Spout, a fence that wins after construction passes the resource into its retirement path before the lease is released; a failed retirement deliberately leaves the lease in flight. Display uses the same create/publish/retire contract: partial setup or sync failure retires every touched label through stop, close, join, and window acknowledgement before releasing the lease, and an unacknowledged cleanup deliberately retains the lease fail-closed. Runtime resize/render/present failure arms the failure fence while its physical-output permit is live and transfers the teardown lease to the registered worker until retirement acknowledgement. Engine race tests and the 14 SDK-free Display tests prove these ordering and acknowledgement contracts; they do not prove native GPU/driver/HDMI behavior or hardware teardown timing.

The outer sync owns the stop-to-start fence and re-admits the current epoch after every stop phase, so a route replacement cannot use a stale activation and starts are withheld after any stop failure. The NDI worker send/render failure fences ownership before sender teardown, keeps a route-scoped failure/pending identity, and leaves the authoritative status Failed until cleanup acknowledgement and an explicit retry. Worker spawn failure creates no sender before the worker exists; sender startup is acknowledged before the route is published. The NDI cleanup acknowledgement is bounded. A timeout retains the route's pending cleanup handle and an output-ownership teardown lease in the SDK-owned cleanup thread; the gate remains Failed/all-deny and later role changes/NDI sender creation are denied until that handle acknowledges completion. The next explicit retry re-polls the same pending cleanup before any new sender is created. The startup-timeout proof is structural/injected rather than a real SDK constructor-hang test. No generic joined-worker claim is made for a timed-out SDK cleanup, and no NDI SDK or hardware result is claimed by the SDK-free tests.

Spout has no asynchronous close acknowledgement in this path. On a worker failure, `begin_output_ownership_failure_fence` runs before the sender is dropped; the worker stores the teardown lease, and `JoinHandle<Result<...>>` completion is awaited before the route releases that lease. Retry therefore remains blocked until sender destruction and worker teardown acknowledgement. The injected sender tests prove this ordering and the held-frame-permit failure boundary without constructing a real Spout2 sender; they do not claim Windows GPU, Spout2 runtime, or hardware validation.

RDM/TOD async commands acquire the lighting permit before dispatch but move it into the spawn_blocking closure, so dropping the async future cannot release ownership while hardware I/O continues. This is a structural lifetime guarantee; cancellation does not interrupt the underlying blocking driver call, whose existing command timeout remains the physical boundary.

This is local ownership only. No peer lease, LAN discovery, authentication, distributed lock, failover guarantee, or claim of safety against an independently running second machine is implemented in this tranche.

## Verification performed

- `cargo fmt --all -- --check`
- `cargo test -p protocol --locked machine_output_role`
- `cargo test -p engine --locked machine_output_role`
- `cargo test -p engine --locked standby_role_loads_authored_dmx_config_without_arming_output`
- `cargo test -p engine --locked output_ownership_gate`
- `cargo test -p engine --locked output_ownership_gate_timeout_keeps_fence_until_permit_drains`
- `cargo test -p engine --locked output_ownership_teardown_lease_blocks_rearm_until_cleanup_ack`
- `cargo test -p engine --locked output_worker_failure_fence_blocks_rearm_until_teardown_ack`
- `cargo test -p engine --locked output_ownership_gate_poison_during_transition`
- `cargo test -p engine --locked output_ownership_activation_requires_fenced_activation_phase`
- `cargo test -p engine --locked output_resource_creation`
- `cargo test -p engine --locked fail_closed_engine_start`
- `cargo test -p engine --locked output_sender_preparation_failure`
- `cargo test -p engine --locked output_ownership_role_matrix`
- `cargo test -p video --locked external_video_transport`
- `cargo test -p video --locked external_video_transport_runtime_reports_driver_failures`
- `cargo test -p syndocal --locked machine_output_ownership_state_is_atomic`
- `cargo test -p syndocal --locked startup_preference_requires_explicit_arm`
- `cargo test -p syndocal --locked output_ownership_target_is_not_durable_until_preparation_succeeds`
- `cargo test -p syndocal --locked standby_output_role_lock`
- `cargo test -p syndocal --locked standby_sync_running_publication_requires_completed_all_deny_status`
- `cargo test -p syndocal --locked native_video_output_window_prefix`
- `cargo test -p syndocal --locked native_video_output_window_close_attempts_all_labels`
- `cargo test -p syndocal --locked standby_sync_role_preserves_authored_project_bytes_for_runtime_gate`
- `cargo test -p syndocal --locked warm_standby_project_disarms_every_output`
- `cargo test -p syndocal --locked fenced_resource_creation_rejects_ndi_spout_and_display_constructor_seams`
- `cargo test -p syndocal --locked failure_fence_before_spout_creation_admission_never_constructs_sender`
- `cargo test -p syndocal --locked injected_spout_`
- `cargo test -p io --features ndi --locked teardown_ack`
- `cargo test -p syndocal --no-default-features --features ndi --locked injected_worker_spawn_failure_constructs_no_sender`
- `cargo test -p syndocal --no-default-features --features ndi --locked capture_decoder`
- `cargo check -p protocol -p engine -p video -p syndocal --locked`
- `cargo check -p syndocal --no-default-features --features ndi --locked`
- `pnpm --dir app exec tsc --noEmit`
- `pnpm --dir app run check:output-ownership`
- `pnpm --dir app run check:localization`
- `git diff --check`

The focused tests cover the exact role matrix, default compatibility, runtime-only persistence, app-local role-state missing/corrupt/version/unreadable handling, explicit-arm restart safety with an old preferred `Both`, durable Standby-first/target-after-preparation persistence ordering, Art-Net and sACN UDP loopback role blocking/re-enable, sender-preparation failure, permit fencing/no-deadlock/held-permit timeout recovery, worker-failure fencing and teardown-lease re-arm denial, activation admission, the activation-token/failure-fence race with zero constructor attempts, the creation-lease/fence/retry race through resource retirement acknowledgement, standby-before-running publication, standby lock/takeover invariants, stale native-window prefix matching, production sender gating, external video route stop/restart with stop-to-start re-admission, and standby authored-project preservation. SDK-free app seams cover the NDI/Spout/Display admission counters, injected Spout sender teardown, Display permit-before-resize/render/present ordering, physical-error fencing before permit release, partial-create/sync cleanup, stale-window reconciliation, and stop/close/join/window acknowledgement. They do not exercise an NDI or Spout SDK constructor or native output hardware. The static frontend/localization harness covers the role options, accessibility labels, explicit-arm startup ordering, authoritative status fields, activation/permit ordering, the NDI/Spout/Display source-level creation-lease markers, pending NDI teardown retention, the actual NDI spawn helper seam, Spout failure-fence/join/teardown-lease order, stop-failure start withholding, and standby control lockout. It does not prove project-swap retirement or native hardware behavior.

Audio and MIDI ownership are outside tranche 1. Project/new/recovery/backup replacement fencing and physical SDK startup-timeout behavior remain incomplete local-runtime boundaries. Cross-PC authenticated peer leases, discovery, authentication, distributed exclusivity, failover, ShowClock, replication, MSC, LTC, and ArtSync remain future boundaries. A local role does not prove that another PC is not transmitting.

The default-feature `syndocal` focused tests and the SDK-free ownership seams passed. The NDI-only io check/test and syndocal feature test were attempted but are blocked in this environment: the NDI SDK header is absent at `C:\TEMP\NDI 6 SDK/include`, and the feature test executable cannot load the NDI runtime DLL (`STATUS_DLL_NOT_FOUND`). No NDI or Spout SDK/hardware behavior is claimed from those seams. The original tranche verification recorded here did not perform a combined all-feature check, package installation, native release build, or native UI launch. A later Take Over repair checkpoint did perform the repository native build/launch gate; see [the 2026-08-19 handoff](CODEX_HANDOFF_2026-08-19.md#take-over-repair-checkpoint).

## Deliberate verification boundaries

Within the original tranche verification recorded in this document, no native release build, native UI launch, process control, hardware output, Daslight interaction, LAN peer testing, or distributed ownership test was performed. The later [2026-08-19 Take Over repair checkpoint](CODEX_HANDOFF_2026-08-19.md#take-over-repair-checkpoint) adds repair-scoped native build/launch/process evidence only; it does not add native hardware/SDK, LAN peer, or distributed ownership proof, and it does not close this tranche's remaining physical boundaries. Final native release build and responsive-window verification remain supervisor-owned under the repository completion gate.
