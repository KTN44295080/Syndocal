//! Bounded, redacted, local-window control-plane observations.
//!
//! This adapter is deliberately separate from the legacy snapshot, project
//! authority bundle, remote WebSocket, and every mutation path.  Its cursors
//! are random server-side handles; no authority, path, project content, raw
//! driver error, or caller-provided window identity is serialized into them.

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::Mutex,
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use protocol::{
    control_plane_command::{ProjectMutationFenceV1, MAX_SAFE_JAVASCRIPT_INTEGER},
    control_plane_query::{
        CanonicalObservationEventPayload, CapabilityDescriptor, CapabilityDiscovery,
        ControlPlaneEvent, EventPage, EventPageRequest, GapMarker, GapReason, OpaqueCursorToken,
        OutputOwnershipQueryPayload, Page, PageRequest, ProjectAuthorityQueryPayload, QueryError,
        QueryErrorCode, QueryFence, QueryMachineOutputRole, QueryOutputOwnershipReason,
        QueryOutputOwnershipState, QueryProtocolVersion, QuerySchemaDescriptor, QuerySchemaKind,
        RuntimeDomainGeneration, RuntimeGenerationPayload, SchemaCatalog,
    },
    MachineOutputRole, OutputOwnershipReason, OutputOwnershipState, TimelineFollowRuntimeStatus,
    TimelineLoopRuntimeStatus,
};
use sha2::{Digest, Sha256};
use tauri::{State, WebviewWindow};

use super::{AppState, ProjectCoordinator};

const CURSOR_TTL: Duration = Duration::from_secs(60);
const MAX_CURSORS_PER_WINDOW: usize = 32;
const MAX_CURSORS_PER_PROCESS: usize = 256;
const MAX_HANDOFFS_PER_WINDOW: usize = 32;
const AUTHORED_MUTATION_FENCE_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_AUTHORED_MUTATION_FENCES_PER_WINDOW: usize = 64;
const MAX_AUTHORED_MUTATION_FENCES_PER_PROCESS: usize = 256;
const MAX_EVENT_RING: usize = 2_048;
const MAX_CAPTURE_ATTEMPTS: usize = 8;

const RESOURCE_SCHEMAS: &str = "syndocal.control_plane.schemas";
const RESOURCE_CAPABILITIES: &str = "syndocal.control_plane.capabilities";
const RESOURCE_PROJECT: &str = "syndocal.project.authority";
const RESOURCE_RUNTIME: &str = "syndocal.runtime.generations";
const RESOURCE_OUTPUT: &str = "syndocal.output.ownership";
const RESOURCE_EVENTS: &str = "syndocal.events.observations";

const SCHEMA_PROJECT_PAGE: &str = "syndocal.project.authority.page.v1";
const SCHEMA_RUNTIME_PAGE: &str = "syndocal.runtime.generations.page.v1";
const SCHEMA_OUTPUT_PAGE: &str = "syndocal.output.ownership.page.v1";
const SCHEMA_EVENT_PAGE: &str = "syndocal.events.observations.event_page.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeCandidate {
    domain: String,
    source_generation: u64,
    active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceCapture {
    project: ProjectAuthorityQueryPayload,
    output: OutputOwnershipQueryPayload,
    runtime: Vec<RuntimeCandidate>,
}

#[derive(Debug, Clone)]
struct CanonicalView {
    owner: WindowOwner,
    fence: QueryFence,
    project: ProjectAuthorityQueryPayload,
    output: OutputOwnershipQueryPayload,
    runtime: Vec<RuntimeGenerationPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct WindowOwner {
    label: String,
    incarnation: u64,
}

#[derive(Debug, Clone)]
struct RuntimeObservation {
    source_generation: u64,
    payload: RuntimeGenerationPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CursorKind {
    Snapshot,
    Event,
}

#[derive(Debug, Clone)]
struct CursorEntry {
    owner: WindowOwner,
    session_incarnation: u64,
    resource: String,
    schema: String,
    sort_fingerprint: String,
    filter_fingerprint: String,
    fence: QueryFence,
    next_index: u64,
    kind: CursorKind,
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug, Clone)]
struct IssuedHandoff {
    fence: QueryFence,
    expires_at: Instant,
}

/// Server-only binding for the strict authored mutation fence. The wire DTO
/// deliberately contains no owner/window claim, so the query adapter retains
/// the issuing window incarnation and registered owner incarnation here.
#[derive(Debug, Clone)]
struct IssuedAuthoredMutationFence {
    fence: ProjectMutationFenceV1,
    owner_incarnation: u64,
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug, Clone)]
struct EventRecord {
    event: ControlPlaneEvent<CanonicalObservationEventPayload>,
    /// Exact canonical state after `event` has been applied. The first
    /// bootstrap records may not have a complete project/output/runtime image;
    /// no issued handoff can precede completion of that bootstrap.
    post_fence: Option<QueryFence>,
}

#[derive(Debug)]
struct QueryInner {
    window_incarnations: HashMap<String, u64>,
    handoffs: HashMap<WindowOwner, VecDeque<IssuedHandoff>>,
    authored_mutation_fences: HashMap<WindowOwner, VecDeque<IssuedAuthoredMutationFence>>,
    cursors: HashMap<String, CursorEntry>,
    lru_sequence: u64,
    runtime: BTreeMap<String, RuntimeObservation>,
    last_project: Option<ProjectAuthorityQueryPayload>,
    last_output: Option<OutputOwnershipQueryPayload>,
    event_stream_epoch: u64,
    event_stream_generation: u64,
    events: VecDeque<EventRecord>,
}

/// App-owned state for the local query adapter.  Process/session identities and
/// every window incarnation originate from the OS CSPRNG.
pub(crate) struct ControlPlaneQueryState {
    process_incarnation: u64,
    session_incarnation: u64,
    inner: Mutex<QueryInner>,
}

impl ControlPlaneQueryState {
    pub(crate) fn new() -> Result<Self, String> {
        Ok(Self {
            // These two values cross the strict authored-mutation wire and
            // must survive a JavaScript JSON round-trip exactly. Cursor/event
            // internals retain the unconstrained CSPRNG u64 helper below.
            process_incarnation: random_nonzero_javascript_safe_u64()?,
            session_incarnation: random_nonzero_javascript_safe_u64()?,
            inner: Mutex::new(QueryInner {
                window_incarnations: HashMap::new(),
                handoffs: HashMap::new(),
                authored_mutation_fences: HashMap::new(),
                cursors: HashMap::new(),
                lru_sequence: 0,
                runtime: BTreeMap::new(),
                last_project: None,
                last_output: None,
                event_stream_epoch: random_nonzero_u64()?,
                event_stream_generation: 0,
                events: VecDeque::new(),
            }),
        })
    }

    pub(crate) fn retire_window(&self, label: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(incarnation) = inner.window_incarnations.remove(label) {
                inner.cursors.retain(|_, entry| {
                    entry.owner.label != label || entry.owner.incarnation != incarnation
                });
                inner.handoffs.remove(&WindowOwner {
                    label: label.to_string(),
                    incarnation,
                });
                inner.authored_mutation_fences.remove(&WindowOwner {
                    label: label.to_string(),
                    incarnation,
                });
            }
        }
    }

    /// Issue the exact local query identity plus project E/R/H/publication
    /// fence for an injected WebView. The method deliberately returns no
    /// owner/principal/window claim, no path and no project content. A later
    /// mutation must still validate this fence under external admission and
    /// the project coordinator; this only gives the renderer a canonical
    /// start image instead of letting it guess process/session values.
    pub(crate) fn issue_project_mutation_fence_for_window(
        &self,
        window_label: &str,
        app: &AppState,
    ) -> Result<ProjectMutationFenceV1, QueryError> {
        self.issue_project_mutation_fence_for_window_at_inner(window_label, app, Instant::now())
    }

    /// Test clock seam for the private issue record only. It does not change
    /// any public query cursor, handoff or observation representation.
    #[cfg(test)]
    pub(crate) fn issue_project_mutation_fence_for_window_at(
        &self,
        window_label: &str,
        app: &AppState,
        now: Instant,
    ) -> Result<ProjectMutationFenceV1, QueryError> {
        self.issue_project_mutation_fence_for_window_at_inner(window_label, app, now)
    }

    fn issue_project_mutation_fence_for_window_at_inner(
        &self,
        window_label: &str,
        app: &AppState,
        now: Instant,
    ) -> Result<ProjectMutationFenceV1, QueryError> {
        let view = self.capture_for_window(window_label, app, false)?;
        // Capture the registered owner incarnation only after the query view
        // has finished its coordinator observation. No owner registry lock is
        // ever held with the coordinator, and a rotation racing either side
        // becomes a safe validation failure below rather than an ABA reuse.
        let owner_incarnation = registered_owner_incarnation_for_window(app, window_label)?;
        let fence = project_mutation_fence_from_query_fence(&view.fence);
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| query_error(QueryErrorCode::Internal))?;
        if inner.window_incarnations.get(window_label) != Some(&view.owner.incarnation) {
            return Err(query_error(QueryErrorCode::Forbidden));
        }
        record_authored_mutation_fence(&mut inner, &view.owner, fence, owner_incarnation, now)
    }

    /// Validate only the server-owned query session component of an incoming
    /// mutation fence. The authoritative E/R/H/publication comparison happens
    /// later under the mutation lock order; doing it here would create a
    /// coordinator-before-admission path. Requiring an already-bound window
    /// prevents a raw caller from manufacturing a process/session pair.
    pub(super) fn validate_project_mutation_fence_window(
        &self,
        window_label: &str,
        fence: &ProjectMutationFenceV1,
        owner_incarnation: u64,
    ) -> Result<(), QueryError> {
        self.validate_project_mutation_fence_window_at(
            window_label,
            fence,
            owner_incarnation,
            Instant::now(),
        )
    }

    /// Test clock seam for authored receipt-retention ordering. The caller
    /// still supplies only the injected window label and backend-derived owner
    /// incarnation; no query content or owner state crosses this boundary.
    pub(super) fn validate_project_mutation_fence_window_at(
        &self,
        window_label: &str,
        fence: &ProjectMutationFenceV1,
        owner_incarnation: u64,
        now: Instant,
    ) -> Result<(), QueryError> {
        fence
            .validate()
            .map_err(|_| query_error(QueryErrorCode::InvalidRequest))?;
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| query_error(QueryErrorCode::Internal))?;
        let Some(window_incarnation) = inner.window_incarnations.get(window_label).copied() else {
            return Err(query_error(QueryErrorCode::Forbidden));
        };
        if owner_incarnation == 0 || fence.process_incarnation != self.process_incarnation {
            return Err(query_error(QueryErrorCode::Forbidden));
        }
        let owner = WindowOwner {
            label: window_label.to_string(),
            incarnation: window_incarnation,
        };
        let last_used = inner.lru_sequence.wrapping_add(1);
        inner.lru_sequence = last_used;
        let Some(issued) = inner.authored_mutation_fences.get_mut(&owner) else {
            return Err(query_error(QueryErrorCode::Forbidden));
        };
        issued.retain(|entry| entry.expires_at > now);
        let Some(entry) = issued
            .iter_mut()
            .find(|entry| entry.fence == *fence && entry.owner_incarnation == owner_incarnation)
        else {
            return Err(query_error(QueryErrorCode::Forbidden));
        };
        entry.last_used = last_used;
        Ok(())
    }

    fn capture_for_window(
        &self,
        label: &str,
        app: &AppState,
        issue_handoff: bool,
    ) -> Result<CanonicalView, QueryError> {
        self.capture_serialized(label, issue_handoff, || capture_source(app))
    }

    /// Source capture happens while the query-state mutex is held. This gives
    /// observations one total order: a delayed S1 capture cannot reconcile
    /// after another caller has already published S2.
    fn capture_serialized<F>(
        &self,
        label: &str,
        issue_handoff: bool,
        capture: F,
    ) -> Result<CanonicalView, QueryError>
    where
        F: FnOnce() -> Result<SourceCapture, QueryError>,
    {
        if label.is_empty() {
            return Err(query_error(QueryErrorCode::Forbidden));
        }
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| query_error(QueryErrorCode::Internal))?;
        let owner = bind_window(&mut inner, label)?;
        let source = capture()?;
        reconcile_observations(
            &mut inner,
            &source,
            self.process_incarnation,
            self.session_incarnation,
        )?;
        let runtime = inner
            .runtime
            .values()
            .map(|observation| observation.payload.clone())
            .collect::<Vec<_>>();
        let fence = QueryFence {
            process_incarnation: self.process_incarnation,
            session_incarnation: self.session_incarnation,
            project_epoch: source.project.project_epoch,
            project_revision: source.project.project_revision,
            project_history_generation: source.project.project_history_generation,
            project_checkpoint_hash: source.project.project_checkpoint_hash.clone(),
            project_publication_generation: source.project.project_publication_generation,
            output_epoch: source.output.output_epoch,
            output_generation: source.output.output_generation,
            event_stream_epoch: inner.event_stream_epoch,
            event_stream_generation: inner.event_stream_generation,
            runtime_domains: runtime
                .iter()
                .map(|payload| RuntimeDomainGeneration {
                    domain: payload.domain.clone(),
                    generation: payload.generation,
                })
                .collect(),
        };
        fence
            .validate()
            .map_err(|_| query_error(QueryErrorCode::Internal))?;
        if issue_handoff {
            record_handoff(&mut inner, &owner, &fence, Instant::now());
        }
        Ok(CanonicalView {
            owner,
            fence,
            project: source.project,
            output: source.output,
            runtime,
        })
    }

    fn consume_handoff(
        &self,
        owner: &WindowOwner,
        fence: &QueryFence,
        now: Instant,
    ) -> Result<(), QueryError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| query_error(QueryErrorCode::Internal))?;
        let Some(handoffs) = inner.handoffs.get_mut(owner) else {
            return Err(query_error(QueryErrorCode::SnapshotRequired));
        };
        handoffs.retain(|handoff| handoff.expires_at > now);
        let Some(index) = handoffs.iter().position(|handoff| handoff.fence == *fence) else {
            return Err(query_error(QueryErrorCode::SnapshotRequired));
        };
        handoffs.remove(index);
        Ok(())
    }

    fn issue_cursor(
        &self,
        owner: &WindowOwner,
        resource: &str,
        schema: &str,
        fence: &QueryFence,
        next_index: u64,
        kind: CursorKind,
        now: Instant,
    ) -> Result<OpaqueCursorToken, QueryError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| query_error(QueryErrorCode::Internal))?;
        if inner.window_incarnations.get(&owner.label) != Some(&owner.incarnation) {
            return Err(query_error(QueryErrorCode::Forbidden));
        }
        evict_expired(&mut inner, now);
        evict_for_capacity(&mut inner, owner);
        inner.lru_sequence = inner.lru_sequence.wrapping_add(1);
        let last_used = inner.lru_sequence;
        for _ in 0..8 {
            let token = random_cursor_token()?;
            if inner.cursors.contains_key(token.as_str()) {
                continue;
            }
            inner.cursors.insert(
                token.as_str().to_string(),
                CursorEntry {
                    owner: owner.clone(),
                    session_incarnation: self.session_incarnation,
                    resource: resource.to_string(),
                    schema: schema.to_string(),
                    sort_fingerprint: fingerprint("canonical-domain-order-v1"),
                    filter_fingerprint: fingerprint("all-redacted-observations-v1"),
                    fence: fence.clone(),
                    next_index,
                    kind,
                    expires_at: now + CURSOR_TTL,
                    last_used,
                },
            );
            return Ok(token);
        }
        Err(query_error(QueryErrorCode::Overloaded))
    }

    fn consume_cursor(
        &self,
        owner: &WindowOwner,
        token: &OpaqueCursorToken,
        resource: &str,
        schema: &str,
        kind: CursorKind,
        expected_fence: Option<&QueryFence>,
        now: Instant,
    ) -> Result<CursorEntry, QueryError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| query_error(QueryErrorCode::Internal))?;
        evict_expired(&mut inner, now);
        let entry = inner
            .cursors
            .get(token.as_str())
            .cloned()
            .ok_or_else(|| query_error(QueryErrorCode::CursorInvalid))?;
        if entry.owner != *owner
            || entry.session_incarnation != self.session_incarnation
            || entry.resource != resource
            || entry.schema != schema
            || entry.kind != kind
            || entry.sort_fingerprint != fingerprint("canonical-domain-order-v1")
            || entry.filter_fingerprint != fingerprint("all-redacted-observations-v1")
            || entry.fence.validate().is_err()
        {
            return Err(query_error(QueryErrorCode::CursorInvalid));
        }
        if expected_fence.is_some_and(|expected| entry.fence != *expected) {
            return Err(query_error(QueryErrorCode::CursorStale));
        }
        inner.cursors.remove(token.as_str());
        Ok(entry)
    }

    fn event_slice(&self, start_generation: u64, limit: usize) -> Result<EventSlice, QueryError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| query_error(QueryErrorCode::Internal))?;
        let first_available = inner
            .events
            .front()
            .map(|record| record.event.generation)
            .unwrap_or_else(|| inner.event_stream_generation.saturating_add(1));
        if start_generation.saturating_add(1) < first_available {
            return Ok(EventSlice::Gap {
                stream_epoch: inner.event_stream_epoch,
                next_available_generation: first_available,
            });
        }
        let events = inner
            .events
            .iter()
            .filter(|record| record.event.generation > start_generation)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let last_generation = events
            .last()
            .map(|record| record.event.generation)
            .unwrap_or(start_generation);
        let last_fence = events.last().and_then(|record| record.post_fence.clone());
        Ok(EventSlice::Events {
            stream_epoch: inner.event_stream_epoch,
            events: events.into_iter().map(|record| record.event).collect(),
            last_generation,
            last_fence,
        })
    }

    #[cfg(test)]
    fn cursor_count_for_window(&self, label: &str) -> usize {
        self.inner
            .lock()
            .unwrap()
            .cursors
            .values()
            .filter(|entry| entry.owner.label == label)
            .count()
    }
}

/// A deliberately lossy projection: project mutation sees the query-process
/// identity plus project E/R/H/publication fields. The issuing method replaces
/// its session member with a fresh server-only window/owner session before it
/// crosses IPC. The six established read-query representations retain their
/// complete fences and payloads.
fn project_mutation_fence_from_query_fence(fence: &QueryFence) -> ProjectMutationFenceV1 {
    ProjectMutationFenceV1 {
        process_incarnation: fence.process_incarnation,
        session_incarnation: fence.session_incarnation,
        project_epoch: fence.project_epoch,
        project_revision: fence.project_revision,
        project_checkpoint_hash: fence.project_checkpoint_hash.clone(),
        project_publication_generation: fence.project_publication_generation,
    }
}

fn registered_owner_incarnation_for_window(
    app: &AppState,
    window_label: &str,
) -> Result<u64, QueryError> {
    let owners = app
        .project_transaction_owners
        .lock()
        .map_err(|_| query_error(QueryErrorCode::Internal))?;
    if !owners.contains_key(window_label) {
        return Err(query_error(QueryErrorCode::Forbidden));
    }
    let incarnations = app
        .project_transaction_owner_incarnations
        .lock()
        .map_err(|_| query_error(QueryErrorCode::Internal))?;
    incarnations
        .get(window_label)
        .copied()
        .filter(|incarnation| *incarnation != 0)
        .ok_or_else(|| query_error(QueryErrorCode::Forbidden))
}

fn record_authored_mutation_fence(
    inner: &mut QueryInner,
    owner: &WindowOwner,
    mut fence: ProjectMutationFenceV1,
    owner_incarnation: u64,
    now: Instant,
) -> Result<ProjectMutationFenceV1, QueryError> {
    fence
        .validate()
        .map_err(|_| query_error(QueryErrorCode::Internal))?;
    let last_used = inner.lru_sequence.wrapping_add(1);
    inner.lru_sequence = last_used;
    {
        let entries = inner
            .authored_mutation_fences
            .entry(owner.clone())
            .or_default();
        // Owner rotation invalidates all prior mutation sessions for this
        // window. Their opaque session incarnations are not reused, so a
        // delayed old renderer envelope remains distinguishable even when it
        // has the same E/R/H/publication values as a new renderer request.
        entries
            .retain(|entry| entry.expires_at > now && entry.owner_incarnation == owner_incarnation);
        if let Some(existing) = entries
            .iter_mut()
            .find(|entry| same_project_mutation_fence_ignoring_session(&entry.fence, &fence))
        {
            existing.expires_at = now + AUTHORED_MUTATION_FENCE_TTL;
            existing.last_used = last_used;
            return Ok(existing.fence.clone());
        }
        if entries.len() >= MAX_AUTHORED_MUTATION_FENCES_PER_WINDOW {
            return Err(query_error(QueryErrorCode::Overloaded));
        }
    }
    let total = inner
        .authored_mutation_fences
        .values()
        .map(VecDeque::len)
        .sum::<usize>();
    if total >= MAX_AUTHORED_MUTATION_FENCES_PER_PROCESS {
        return Err(query_error(QueryErrorCode::Overloaded));
    }
    fence.session_incarnation =
        random_nonzero_javascript_safe_u64().map_err(|_| query_error(QueryErrorCode::Internal))?;
    fence
        .validate()
        .map_err(|_| query_error(QueryErrorCode::Internal))?;
    let entries = inner
        .authored_mutation_fences
        .entry(owner.clone())
        .or_default();
    entries.push_back(IssuedAuthoredMutationFence {
        fence: fence.clone(),
        owner_incarnation,
        expires_at: now + AUTHORED_MUTATION_FENCE_TTL,
        last_used,
    });
    Ok(fence)
}

fn same_project_mutation_fence_ignoring_session(
    left: &ProjectMutationFenceV1,
    right: &ProjectMutationFenceV1,
) -> bool {
    left.process_incarnation == right.process_incarnation
        && left.project_epoch == right.project_epoch
        && left.project_revision == right.project_revision
        && left.project_checkpoint_hash == right.project_checkpoint_hash
        && left.project_publication_generation == right.project_publication_generation
}

enum EventSlice {
    Gap {
        stream_epoch: u64,
        next_available_generation: u64,
    },
    Events {
        stream_epoch: u64,
        events: Vec<ControlPlaneEvent<CanonicalObservationEventPayload>>,
        last_generation: u64,
        last_fence: Option<QueryFence>,
    },
}

fn bind_window(inner: &mut QueryInner, label: &str) -> Result<WindowOwner, QueryError> {
    let incarnation = match inner.window_incarnations.get(label).copied() {
        Some(incarnation) => incarnation,
        None => {
            let incarnation =
                random_nonzero_u64().map_err(|_| query_error(QueryErrorCode::Internal))?;
            inner
                .window_incarnations
                .insert(label.to_string(), incarnation);
            incarnation
        }
    };
    Ok(WindowOwner {
        label: label.to_string(),
        incarnation,
    })
}

fn record_handoff(inner: &mut QueryInner, owner: &WindowOwner, fence: &QueryFence, now: Instant) {
    let handoffs = inner.handoffs.entry(owner.clone()).or_default();
    handoffs.retain(|handoff| handoff.expires_at > now);
    while handoffs.len() >= MAX_HANDOFFS_PER_WINDOW {
        handoffs.pop_front();
    }
    handoffs.push_back(IssuedHandoff {
        fence: fence.clone(),
        expires_at: now + CURSOR_TTL,
    });
}

fn capture_source(app: &AppState) -> Result<SourceCapture, QueryError> {
    for _ in 0..MAX_CAPTURE_ATTEMPTS {
        let first = capture_source_once(app)?;
        let second = capture_source_once(app)?;
        if first == second {
            return Ok(second);
        }
        std::hint::spin_loop();
    }
    Err(query_error(QueryErrorCode::Unavailable))
}

fn capture_source_once(app: &AppState) -> Result<SourceCapture, QueryError> {
    let project = {
        let coordinator = app
            .project_coordinator
            .lock()
            .map_err(|_| query_error(QueryErrorCode::Internal))?;
        project_projection(&coordinator)
    };
    let snapshot = app.engine.snapshot();
    let output = output_projection(&app.engine.output_ownership_status())?;
    let runtime = vec![
        RuntimeCandidate {
            domain: "timeline.follow".to_string(),
            source_generation: snapshot.timeline.follow_runtime.generation,
            active: snapshot.timeline.follow_runtime.status != TimelineFollowRuntimeStatus::Idle,
        },
        RuntimeCandidate {
            domain: "timeline.loop".to_string(),
            source_generation: snapshot.timeline.loop_runtime.generation,
            active: snapshot.timeline.loop_runtime.status != TimelineLoopRuntimeStatus::Disabled,
        },
        RuntimeCandidate {
            domain: "video.clip_slots".to_string(),
            source_generation: app
                .video_clip_slot_runtime_generation
                .load(std::sync::atomic::Ordering::Acquire),
            active: snapshot.video_clip_runtime.layers.iter().any(|layer| {
                layer.playing
                    || layer.active_slot_id.is_some()
                    || layer.pending_launch.is_some()
                    || layer.transition.is_some()
            }),
        },
        RuntimeCandidate {
            domain: "video.transitions".to_string(),
            source_generation: app
                .video_transition_runtime_generation
                .load(std::sync::atomic::Ordering::Acquire),
            active: !snapshot.video_transition_runtime.buses.is_empty(),
        },
    ];
    Ok(SourceCapture {
        project,
        output,
        runtime,
    })
}

fn project_projection(coordinator: &ProjectCoordinator) -> ProjectAuthorityQueryPayload {
    ProjectAuthorityQueryPayload {
        project_epoch: coordinator.epoch,
        project_revision: coordinator.revision,
        project_history_generation: coordinator.history_generation,
        project_checkpoint_hash: coordinator.checkpoint_hash.clone(),
        project_publication_generation: coordinator.publication_generation,
    }
}

fn output_projection(
    status: &protocol::OutputOwnershipStatus,
) -> Result<OutputOwnershipQueryPayload, QueryError> {
    // The engine's internal counters are zero-based; the query protocol uses
    // zero as an invalid/uninitialized sentinel.  The stable +1 projection is
    // therefore the canonical external generation without weakening either
    // contract.
    let output_epoch = status
        .epoch
        .checked_add(1)
        .ok_or_else(|| query_error(QueryErrorCode::Unavailable))?;
    let output_generation = status
        .generation
        .checked_add(1)
        .ok_or_else(|| query_error(QueryErrorCode::Unavailable))?;
    let payload = OutputOwnershipQueryPayload {
        output_epoch,
        output_generation,
        state: match status.state {
            OutputOwnershipState::Ready => QueryOutputOwnershipState::Ready,
            OutputOwnershipState::Transitioning => QueryOutputOwnershipState::Transitioning,
            OutputOwnershipState::Activating => QueryOutputOwnershipState::Activating,
            OutputOwnershipState::Failed => QueryOutputOwnershipState::Failed,
        },
        effective_role: query_role(status.effective_role),
        desired_role: query_role(status.desired_role),
        lighting_allowed: status.lighting_allowed,
        video_allowed: status.video_allowed,
        lighting_reason: query_reason(status.lighting_reason),
        video_reason: query_reason(status.video_reason),
    };
    payload
        .validate()
        .map_err(|_| query_error(QueryErrorCode::Unavailable))?;
    Ok(payload)
}

const fn query_role(role: MachineOutputRole) -> QueryMachineOutputRole {
    match role {
        MachineOutputRole::Lighting => QueryMachineOutputRole::Lighting,
        MachineOutputRole::Video => QueryMachineOutputRole::Video,
        MachineOutputRole::Both => QueryMachineOutputRole::Both,
        MachineOutputRole::Standby => QueryMachineOutputRole::Standby,
    }
}

const fn query_reason(reason: OutputOwnershipReason) -> QueryOutputOwnershipReason {
    match reason {
        OutputOwnershipReason::OwnedByMachineRole => QueryOutputOwnershipReason::OwnedByMachineRole,
        OutputOwnershipReason::BlockedByMachineRole => {
            QueryOutputOwnershipReason::BlockedByMachineRole
        }
        OutputOwnershipReason::Transitioning => QueryOutputOwnershipReason::Transitioning,
        OutputOwnershipReason::TransitionFailed => QueryOutputOwnershipReason::TransitionFailed,
        OutputOwnershipReason::ProjectSwapDisarmed => {
            QueryOutputOwnershipReason::ProjectSwapDisarmed
        }
        OutputOwnershipReason::StartupDenied => QueryOutputOwnershipReason::StartupDenied,
    }
}

fn reconcile_observations(
    inner: &mut QueryInner,
    source: &SourceCapture,
    process_incarnation: u64,
    session_incarnation: u64,
) -> Result<(), QueryError> {
    if inner.last_project.as_ref() != Some(&source.project) {
        inner.last_project = Some(source.project.clone());
        append_event(
            inner,
            CanonicalObservationEventPayload::ProjectAuthorityChanged(source.project.clone()),
            process_incarnation,
            session_incarnation,
        )?;
    }
    for candidate in &source.runtime {
        let changed = inner
            .runtime
            .get(&candidate.domain)
            .map_or(true, |current| {
                current.source_generation != candidate.source_generation
                    || current.payload.active != candidate.active
            });
        if !changed {
            continue;
        }
        let generation = inner
            .runtime
            .get(&candidate.domain)
            .map(|current| current.payload.generation)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| query_error(QueryErrorCode::Unavailable))?;
        let payload = RuntimeGenerationPayload {
            domain: candidate.domain.clone(),
            generation,
            active: candidate.active,
        };
        payload
            .validate()
            .map_err(|_| query_error(QueryErrorCode::Internal))?;
        inner.runtime.insert(
            candidate.domain.clone(),
            RuntimeObservation {
                source_generation: candidate.source_generation,
                payload: payload.clone(),
            },
        );
        append_event(
            inner,
            CanonicalObservationEventPayload::RuntimeGenerationChanged(payload),
            process_incarnation,
            session_incarnation,
        )?;
    }
    if inner.last_output.as_ref() != Some(&source.output) {
        inner.last_output = Some(source.output.clone());
        append_event(
            inner,
            CanonicalObservationEventPayload::OutputOwnershipChanged(source.output.clone()),
            process_incarnation,
            session_incarnation,
        )?;
    }
    Ok(())
}

fn append_event(
    inner: &mut QueryInner,
    payload: CanonicalObservationEventPayload,
    process_incarnation: u64,
    session_incarnation: u64,
) -> Result<(), QueryError> {
    let generation = inner
        .event_stream_generation
        .checked_add(1)
        .ok_or_else(|| query_error(QueryErrorCode::Unavailable))?;
    let event = ControlPlaneEvent {
        protocol_version: QueryProtocolVersion::CURRENT,
        resource: RESOURCE_EVENTS.to_string(),
        schema: format!("{RESOURCE_EVENTS}.event.v1"),
        stream_epoch: inner.event_stream_epoch,
        generation,
        payload,
    };
    event
        .validate()
        .map_err(|_| query_error(QueryErrorCode::Internal))?;
    inner.event_stream_generation = generation;
    let post_fence = canonical_fence_from_inner(inner, process_incarnation, session_incarnation)?;
    inner.events.push_back(EventRecord { event, post_fence });
    while inner.events.len() > MAX_EVENT_RING {
        inner.events.pop_front();
    }
    Ok(())
}

fn canonical_fence_from_inner(
    inner: &QueryInner,
    process_incarnation: u64,
    session_incarnation: u64,
) -> Result<Option<QueryFence>, QueryError> {
    let (Some(project), Some(output)) = (&inner.last_project, &inner.last_output) else {
        return Ok(None);
    };
    let fence = QueryFence {
        process_incarnation,
        session_incarnation,
        project_epoch: project.project_epoch,
        project_revision: project.project_revision,
        project_history_generation: project.project_history_generation,
        project_checkpoint_hash: project.project_checkpoint_hash.clone(),
        project_publication_generation: project.project_publication_generation,
        output_epoch: output.output_epoch,
        output_generation: output.output_generation,
        event_stream_epoch: inner.event_stream_epoch,
        event_stream_generation: inner.event_stream_generation,
        runtime_domains: inner
            .runtime
            .values()
            .map(|observation| RuntimeDomainGeneration {
                domain: observation.payload.domain.clone(),
                generation: observation.payload.generation,
            })
            .collect(),
    };
    fence
        .validate()
        .map_err(|_| query_error(QueryErrorCode::Internal))?;
    Ok(Some(fence))
}

fn schema_catalog() -> SchemaCatalog {
    let mut schemas = vec![
        QuerySchemaDescriptor {
            schema_id: format!("{RESOURCE_CAPABILITIES}.response.v1"),
            version: QueryProtocolVersion::CURRENT,
            kind: QuerySchemaKind::Response,
        },
        QuerySchemaDescriptor {
            schema_id: format!("{RESOURCE_SCHEMAS}.response.v1"),
            version: QueryProtocolVersion::CURRENT,
            kind: QuerySchemaKind::Response,
        },
        QuerySchemaDescriptor {
            schema_id: SCHEMA_EVENT_PAGE.to_string(),
            version: QueryProtocolVersion::CURRENT,
            kind: QuerySchemaKind::Event,
        },
        QuerySchemaDescriptor {
            schema_id: format!("{RESOURCE_EVENTS}.event.v1"),
            version: QueryProtocolVersion::CURRENT,
            kind: QuerySchemaKind::Event,
        },
        QuerySchemaDescriptor {
            schema_id: SCHEMA_OUTPUT_PAGE.to_string(),
            version: QueryProtocolVersion::CURRENT,
            kind: QuerySchemaKind::Response,
        },
        QuerySchemaDescriptor {
            schema_id: SCHEMA_PROJECT_PAGE.to_string(),
            version: QueryProtocolVersion::CURRENT,
            kind: QuerySchemaKind::Response,
        },
        QuerySchemaDescriptor {
            schema_id: SCHEMA_RUNTIME_PAGE.to_string(),
            version: QueryProtocolVersion::CURRENT,
            kind: QuerySchemaKind::Response,
        },
    ];
    schemas.sort_by(|left, right| left.schema_id.cmp(&right.schema_id));
    SchemaCatalog {
        protocol_version: QueryProtocolVersion::CURRENT,
        catalog_generation: 1,
        schemas,
    }
}

fn capability_discovery(fence: QueryFence) -> CapabilityDiscovery {
    let mut capabilities = vec![
        CapabilityDescriptor {
            capability_id: "syndocal.discovery".to_string(),
            operation_ids: vec![
                "syndocal.query.control_plane.capabilities.v1".to_string(),
                "syndocal.query.control_plane.schemas.v1".to_string(),
            ],
            allowed_during_full_lock: true,
        },
        CapabilityDescriptor {
            capability_id: "syndocal.output_ownership".to_string(),
            operation_ids: vec!["syndocal.query.output.ownership.v1".to_string()],
            allowed_during_full_lock: true,
        },
        CapabilityDescriptor {
            capability_id: "syndocal.project_authority".to_string(),
            operation_ids: vec!["syndocal.query.project.authority.v1".to_string()],
            allowed_during_full_lock: true,
        },
        CapabilityDescriptor {
            capability_id: "syndocal.runtime".to_string(),
            operation_ids: vec![
                "syndocal.query.events.observations.v1".to_string(),
                "syndocal.query.runtime.generations.v1".to_string(),
            ],
            allowed_during_full_lock: true,
        },
    ];
    capabilities.sort_by(|left, right| left.capability_id.cmp(&right.capability_id));
    CapabilityDiscovery {
        protocol_version: QueryProtocolVersion::CURRENT,
        discovery_generation: 1,
        snapshot_fence: fence,
        capabilities,
    }
}

fn page_from_items<T: protocol::control_plane_query::ControlPlaneQueryPayload + Clone>(
    state: &ControlPlaneQueryState,
    view: &CanonicalView,
    request: PageRequest,
    resource: &str,
    schema: &str,
    items: &[T],
) -> Result<Page<T>, QueryError> {
    request
        .validate()
        .map_err(|_| query_error(QueryErrorCode::InvalidRequest))?;
    let start = match request.cursor {
        Some(cursor) => {
            let entry = state.consume_cursor(
                &view.owner,
                &cursor,
                resource,
                schema,
                CursorKind::Snapshot,
                Some(&view.fence),
                Instant::now(),
            )?;
            usize::try_from(entry.next_index)
                .map_err(|_| query_error(QueryErrorCode::CursorInvalid))?
        }
        None => 0,
    };
    if start > items.len() {
        return Err(query_error(QueryErrorCode::CursorInvalid));
    }
    let end = start
        .saturating_add(request.limit as usize)
        .min(items.len());
    let next_cursor = if end < items.len() {
        Some(state.issue_cursor(
            &view.owner,
            resource,
            schema,
            &view.fence,
            end as u64,
            CursorKind::Snapshot,
            Instant::now(),
        )?)
    } else {
        None
    };
    Ok(Page {
        protocol_version: QueryProtocolVersion::CURRENT,
        resource: resource.to_string(),
        snapshot_fence: view.fence.clone(),
        items: items[start..end].to_vec(),
        next_cursor,
    })
}

fn observation_page(
    state: &ControlPlaneQueryState,
    view: &CanonicalView,
    request: EventPageRequest,
) -> Result<EventPage<CanonicalObservationEventPayload>, QueryError> {
    request
        .validate()
        .map_err(|_| query_error(QueryErrorCode::InvalidRequest))?;
    let (start_generation, mut page_fence) = match (request.expected_fence, request.cursor) {
        (Some(expected), None) => {
            state.consume_handoff(&view.owner, &expected, Instant::now())?;
            (expected.event_stream_generation, expected)
        }
        (None, Some(cursor)) => {
            let entry = state.consume_cursor(
                &view.owner,
                &cursor,
                RESOURCE_EVENTS,
                SCHEMA_EVENT_PAGE,
                CursorKind::Event,
                None,
                Instant::now(),
            )?;
            (entry.next_index, entry.fence)
        }
        _ => return Err(query_error(QueryErrorCode::InvalidRequest)),
    };
    let slice = state.event_slice(start_generation, request.limit as usize)?;
    match slice {
        EventSlice::Gap {
            stream_epoch,
            next_available_generation,
        } => {
            let reason = if page_fence.event_stream_epoch == stream_epoch {
                GapReason::RetentionExpired
            } else {
                GapReason::EpochChanged
            };
            let gap = GapMarker {
                previous_stream_epoch: page_fence.event_stream_epoch,
                previous_generation: page_fence.event_stream_generation,
                next_stream_epoch: stream_epoch,
                next_available_generation,
                reason,
                resnapshot_required: true,
            };
            Ok(EventPage {
                protocol_version: QueryProtocolVersion::CURRENT,
                resource: RESOURCE_EVENTS.to_string(),
                snapshot_fence: page_fence,
                events: Vec::new(),
                gap: Some(gap),
                next_cursor: None,
            })
        }
        EventSlice::Events {
            stream_epoch,
            events,
            last_generation,
            last_fence,
        } => {
            if page_fence.event_stream_epoch != stream_epoch {
                let gap = GapMarker {
                    previous_stream_epoch: page_fence.event_stream_epoch,
                    previous_generation: page_fence.event_stream_generation,
                    next_stream_epoch: stream_epoch,
                    next_available_generation: 1,
                    reason: GapReason::EpochChanged,
                    resnapshot_required: true,
                };
                return Ok(EventPage {
                    protocol_version: QueryProtocolVersion::CURRENT,
                    resource: RESOURCE_EVENTS.to_string(),
                    snapshot_fence: page_fence,
                    events: Vec::new(),
                    gap: Some(gap),
                    next_cursor: None,
                });
            }
            page_fence.event_stream_generation = start_generation;
            let continuation_fence = if events.is_empty() {
                page_fence.clone()
            } else {
                last_fence.ok_or_else(|| query_error(QueryErrorCode::Internal))?
            };
            if continuation_fence.event_stream_epoch != stream_epoch
                || continuation_fence.event_stream_generation != last_generation
            {
                return Err(query_error(QueryErrorCode::Internal));
            }
            let next_cursor = Some(state.issue_cursor(
                &view.owner,
                RESOURCE_EVENTS,
                SCHEMA_EVENT_PAGE,
                &continuation_fence,
                last_generation,
                CursorKind::Event,
                Instant::now(),
            )?);
            Ok(EventPage {
                protocol_version: QueryProtocolVersion::CURRENT,
                resource: RESOURCE_EVENTS.to_string(),
                snapshot_fence: page_fence,
                events,
                gap: None,
                next_cursor,
            })
        }
    }
}

fn evict_expired(inner: &mut QueryInner, now: Instant) {
    inner.cursors.retain(|_, entry| entry.expires_at > now);
}

fn evict_for_capacity(inner: &mut QueryInner, owner: &WindowOwner) {
    while inner
        .cursors
        .values()
        .filter(|entry| entry.owner == *owner)
        .count()
        >= MAX_CURSORS_PER_WINDOW
    {
        let candidate = inner
            .cursors
            .iter()
            .filter(|(_, entry)| entry.owner == *owner)
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(token, _)| token.clone());
        if let Some(candidate) = candidate {
            inner.cursors.remove(&candidate);
        } else {
            break;
        }
    }
    while inner.cursors.len() >= MAX_CURSORS_PER_PROCESS {
        let candidate = inner
            .cursors
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(token, _)| token.clone());
        if let Some(candidate) = candidate {
            inner.cursors.remove(&candidate);
        } else {
            break;
        }
    }
}

fn random_nonzero_u64() -> Result<u64, String> {
    for _ in 0..8 {
        let mut bytes = [0_u8; 8];
        getrandom::getrandom(&mut bytes).map_err(|error| error.to_string())?;
        let value = u64::from_le_bytes(bytes);
        if value != 0 {
            return Ok(value);
        }
    }
    Err("OS random source repeatedly returned zero".to_string())
}

fn random_nonzero_javascript_safe_u64() -> Result<u64, String> {
    for _ in 0..8 {
        let mut bytes = [0_u8; 8];
        getrandom::getrandom(&mut bytes).map_err(|error| error.to_string())?;
        let value = u64::from_le_bytes(bytes) & MAX_SAFE_JAVASCRIPT_INTEGER;
        if value != 0 {
            return Ok(value);
        }
    }
    Err("OS random source repeatedly returned zero JavaScript-safe incarnation".to_string())
}

fn random_cursor_token() -> Result<OpaqueCursorToken, QueryError> {
    let mut bytes = [0_u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|_| query_error(QueryErrorCode::Internal))?;
    OpaqueCursorToken::try_new(URL_SAFE_NO_PAD.encode(bytes))
        .map_err(|_| query_error(QueryErrorCode::Internal))
}

fn fingerprint(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn query_error(code: QueryErrorCode) -> QueryError {
    QueryError::from_code(code)
}

#[tauri::command]
pub(crate) fn get_control_plane_query_schema_catalog(
    window: WebviewWindow,
    state: State<'_, ControlPlaneQueryState>,
) -> Result<SchemaCatalog, QueryError> {
    if window.label().is_empty() {
        return Err(query_error(QueryErrorCode::Forbidden));
    }
    // Binding creates the backend-owned incarnation even though this immutable
    // catalog needs no application snapshot.
    let mut inner = state
        .inner
        .lock()
        .map_err(|_| query_error(QueryErrorCode::Internal))?;
    bind_window(&mut inner, window.label())?;
    drop(inner);
    let catalog = schema_catalog();
    catalog
        .validate()
        .map_err(|_| query_error(QueryErrorCode::Internal))?;
    Ok(catalog)
}

#[tauri::command]
pub(crate) fn get_control_plane_query_capabilities(
    window: WebviewWindow,
    state: State<'_, ControlPlaneQueryState>,
    app: State<'_, AppState>,
) -> Result<CapabilityDiscovery, QueryError> {
    let view = state.capture_for_window(window.label(), &app, true)?;
    let discovery = capability_discovery(view.fence);
    discovery
        .validate()
        .map_err(|_| query_error(QueryErrorCode::Internal))?;
    Ok(discovery)
}

#[tauri::command]
pub(crate) fn query_control_plane_project_authority(
    window: WebviewWindow,
    state: State<'_, ControlPlaneQueryState>,
    app: State<'_, AppState>,
    request: PageRequest,
) -> Result<Page<ProjectAuthorityQueryPayload>, QueryError> {
    query_project_authority_for_window(window.label(), &state, &app, request)
}

fn query_project_authority_for_window(
    window_label: &str,
    state: &ControlPlaneQueryState,
    app: &AppState,
    request: PageRequest,
) -> Result<Page<ProjectAuthorityQueryPayload>, QueryError> {
    let view = state.capture_for_window(window_label, app, true)?;
    page_from_items(
        state,
        &view,
        request,
        RESOURCE_PROJECT,
        SCHEMA_PROJECT_PAGE,
        std::slice::from_ref(&view.project),
    )
}

#[tauri::command]
pub(crate) fn query_control_plane_runtime_generations(
    window: WebviewWindow,
    state: State<'_, ControlPlaneQueryState>,
    app: State<'_, AppState>,
    request: PageRequest,
) -> Result<Page<RuntimeGenerationPayload>, QueryError> {
    let view = state.capture_for_window(window.label(), &app, true)?;
    page_from_items(
        &state,
        &view,
        request,
        RESOURCE_RUNTIME,
        SCHEMA_RUNTIME_PAGE,
        &view.runtime,
    )
}

#[tauri::command]
pub(crate) fn query_control_plane_output_ownership(
    window: WebviewWindow,
    state: State<'_, ControlPlaneQueryState>,
    app: State<'_, AppState>,
    request: PageRequest,
) -> Result<Page<OutputOwnershipQueryPayload>, QueryError> {
    let view = state.capture_for_window(window.label(), &app, true)?;
    page_from_items(
        &state,
        &view,
        request,
        RESOURCE_OUTPUT,
        SCHEMA_OUTPUT_PAGE,
        std::slice::from_ref(&view.output),
    )
}

#[tauri::command]
pub(crate) fn poll_control_plane_observation_events(
    window: WebviewWindow,
    state: State<'_, ControlPlaneQueryState>,
    app: State<'_, AppState>,
    request: EventPageRequest,
) -> Result<EventPage<CanonicalObservationEventPayload>, QueryError> {
    let view = state.capture_for_window(window.label(), &app, false)?;
    observation_page(&state, &view, request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{MediaAssetA6CommandHarness, MEDIA_ASSET_A6_OWNER};

    fn test_project(hash_seed: &str) -> ProjectAuthorityQueryPayload {
        ProjectAuthorityQueryPayload {
            project_epoch: 1,
            project_revision: 2,
            project_history_generation: 3,
            project_checkpoint_hash: fingerprint(hash_seed),
            project_publication_generation: 4,
        }
    }

    fn test_output() -> OutputOwnershipQueryPayload {
        OutputOwnershipQueryPayload {
            output_epoch: 1,
            output_generation: 1,
            state: QueryOutputOwnershipState::Ready,
            effective_role: QueryMachineOutputRole::Both,
            desired_role: QueryMachineOutputRole::Both,
            lighting_allowed: true,
            video_allowed: true,
            lighting_reason: QueryOutputOwnershipReason::OwnedByMachineRole,
            video_reason: QueryOutputOwnershipReason::OwnedByMachineRole,
        }
    }

    fn test_output_lighting() -> OutputOwnershipQueryPayload {
        OutputOwnershipQueryPayload {
            output_epoch: 1,
            output_generation: 2,
            state: QueryOutputOwnershipState::Ready,
            effective_role: QueryMachineOutputRole::Lighting,
            desired_role: QueryMachineOutputRole::Lighting,
            lighting_allowed: true,
            video_allowed: false,
            lighting_reason: QueryOutputOwnershipReason::OwnedByMachineRole,
            video_reason: QueryOutputOwnershipReason::BlockedByMachineRole,
        }
    }

    fn seeded_view(state: &ControlPlaneQueryState, label: &str) -> CanonicalView {
        let source = SourceCapture {
            project: test_project("project-a"),
            output: test_output(),
            runtime: vec![RuntimeCandidate {
                domain: "timeline.follow".to_string(),
                source_generation: 0,
                active: false,
            }],
        };
        let mut inner = state.inner.lock().unwrap();
        let owner = bind_window(&mut inner, label).unwrap();
        reconcile_observations(
            &mut inner,
            &source,
            state.process_incarnation,
            state.session_incarnation,
        )
        .unwrap();
        let runtime = inner
            .runtime
            .values()
            .map(|entry| entry.payload.clone())
            .collect::<Vec<_>>();
        let fence = QueryFence {
            process_incarnation: state.process_incarnation,
            session_incarnation: state.session_incarnation,
            project_epoch: 1,
            project_revision: 2,
            project_history_generation: 3,
            project_checkpoint_hash: fingerprint("project-a"),
            project_publication_generation: 4,
            output_epoch: 1,
            output_generation: 1,
            event_stream_epoch: inner.event_stream_epoch,
            event_stream_generation: inner.event_stream_generation,
            runtime_domains: runtime
                .iter()
                .map(|payload| RuntimeDomainGeneration {
                    domain: payload.domain.clone(),
                    generation: payload.generation,
                })
                .collect(),
        };
        record_handoff(&mut inner, &owner, &fence, Instant::now());
        CanonicalView {
            owner,
            fence,
            project: source.project,
            output: source.output,
            runtime,
        }
    }

    #[test]
    fn project_mutation_fence_projection_preserves_six_query_outputs_counts_and_fences() {
        let state = ControlPlaneQueryState::new().unwrap();
        let view = seeded_view(&state, "main");
        let catalog_before = schema_catalog();
        let capabilities_before = capability_discovery(view.fence.clone());
        let project_before = view.project.clone();
        let runtime_before = view.runtime.clone();
        let output_before = view.output.clone();
        let fence_before = view.fence.clone();
        let counts_before = {
            let inner = state.inner.lock().unwrap();
            (
                inner.window_incarnations.len(),
                inner.handoffs.len(),
                inner.cursors.len(),
                inner.runtime.len(),
                inner.events.len(),
                inner.event_stream_generation,
            )
        };

        let mutation_fence = project_mutation_fence_from_query_fence(&view.fence);
        mutation_fence.validate().unwrap();
        assert!(mutation_fence.process_incarnation <= MAX_SAFE_JAVASCRIPT_INTEGER);
        assert!(mutation_fence.session_incarnation <= MAX_SAFE_JAVASCRIPT_INTEGER);
        assert_eq!(
            mutation_fence.process_incarnation,
            fence_before.process_incarnation
        );
        assert_eq!(
            mutation_fence.session_incarnation,
            fence_before.session_incarnation
        );
        assert_eq!(mutation_fence.project_epoch, fence_before.project_epoch);
        assert_eq!(
            mutation_fence.project_revision,
            fence_before.project_revision
        );
        assert_eq!(
            mutation_fence.project_checkpoint_hash,
            fence_before.project_checkpoint_hash
        );
        assert_eq!(
            mutation_fence.project_publication_generation,
            fence_before.project_publication_generation
        );

        // Schema catalog, capability discovery, project, runtime, output and
        // their complete canonical fence are the six pre-existing outputs.
        assert_eq!(schema_catalog(), catalog_before);
        assert_eq!(
            capability_discovery(view.fence.clone()),
            capabilities_before
        );
        assert_eq!(view.project, project_before);
        assert_eq!(view.runtime, runtime_before);
        assert_eq!(view.output, output_before);
        assert_eq!(view.fence, fence_before);
        let counts_after = {
            let inner = state.inner.lock().unwrap();
            (
                inner.window_incarnations.len(),
                inner.handoffs.len(),
                inner.cursors.len(),
                inner.runtime.len(),
                inner.events.len(),
                inner.event_stream_generation,
            )
        };
        assert_eq!(counts_after, counts_before);
    }

    #[test]
    fn issued_local_mutation_fence_is_owner_bound_without_changing_six_query_outputs() {
        let harness = MediaAssetA6CommandHarness::new();
        let state = ControlPlaneQueryState::new().unwrap();
        let before = state
            .capture_for_window("media-asset-a6", &harness.state, false)
            .unwrap();
        let catalog_before = schema_catalog();
        let capabilities_before = capability_discovery(before.fence.clone());
        let counts_before = {
            let inner = state.inner.lock().unwrap();
            (
                inner.window_incarnations.len(),
                inner.handoffs.len(),
                inner.cursors.len(),
                inner.runtime.len(),
                inner.events.len(),
                inner.event_stream_generation,
            )
        };
        let owner_incarnation = harness
            .state
            .project_transaction_owner_incarnations
            .lock()
            .unwrap()
            .get("media-asset-a6")
            .copied()
            .unwrap();
        assert_eq!(
            harness
                .state
                .project_transaction_owners
                .lock()
                .unwrap()
                .get("media-asset-a6")
                .map(String::as_str),
            Some(MEDIA_ASSET_A6_OWNER)
        );

        let issued = state
            .issue_project_mutation_fence_for_window("media-asset-a6", &harness.state)
            .unwrap();
        issued.validate().unwrap();
        assert!(issued.process_incarnation <= MAX_SAFE_JAVASCRIPT_INTEGER);
        assert!(issued.session_incarnation <= MAX_SAFE_JAVASCRIPT_INTEGER);
        assert_eq!(issued.process_incarnation, before.fence.process_incarnation);
        assert_ne!(
            issued.session_incarnation, before.fence.session_incarnation,
            "the mutation session is server-only and owner-scoped"
        );
        assert_eq!(issued.project_epoch, before.fence.project_epoch);
        assert_eq!(issued.project_revision, before.fence.project_revision);
        assert_eq!(
            issued.project_checkpoint_hash,
            before.fence.project_checkpoint_hash
        );
        assert_eq!(
            issued.project_publication_generation,
            before.fence.project_publication_generation
        );
        state
            .validate_project_mutation_fence_window("media-asset-a6", &issued, owner_incarnation)
            .unwrap();
        let after = state
            .capture_for_window("media-asset-a6", &harness.state, false)
            .unwrap();
        // The existing schema catalog, capability discovery, project,
        // runtime, output and complete query fence are untouched. Only the
        // private mutation-issue table gained a bounded record.
        assert_eq!(schema_catalog(), catalog_before);
        assert_eq!(
            capability_discovery(after.fence.clone()),
            capabilities_before
        );
        assert_eq!(after.project, before.project);
        assert_eq!(after.runtime, before.runtime);
        assert_eq!(after.output, before.output);
        assert_eq!(after.fence, before.fence);
        let counts_after = {
            let inner = state.inner.lock().unwrap();
            (
                inner.window_incarnations.len(),
                inner.handoffs.len(),
                inner.cursors.len(),
                inner.runtime.len(),
                inner.events.len(),
                inner.event_stream_generation,
            )
        };
        assert_eq!(counts_after, counts_before);
    }

    #[test]
    fn cursors_are_owner_bound_single_use_and_tamper_evident() {
        let state = ControlPlaneQueryState::new().unwrap();
        let view_a = seeded_view(&state, "a");
        let view_b = seeded_view(&state, "b");
        let token = state
            .issue_cursor(
                &view_a.owner,
                RESOURCE_RUNTIME,
                SCHEMA_RUNTIME_PAGE,
                &view_a.fence,
                1,
                CursorKind::Snapshot,
                Instant::now(),
            )
            .unwrap();
        assert_eq!(
            state
                .consume_cursor(
                    &view_b.owner,
                    &token,
                    RESOURCE_RUNTIME,
                    SCHEMA_RUNTIME_PAGE,
                    CursorKind::Snapshot,
                    Some(&view_a.fence),
                    Instant::now(),
                )
                .unwrap_err()
                .code(),
            QueryErrorCode::CursorInvalid
        );
        assert_eq!(
            state
                .consume_cursor(
                    &view_a.owner,
                    &token,
                    RESOURCE_PROJECT,
                    SCHEMA_PROJECT_PAGE,
                    CursorKind::Snapshot,
                    Some(&view_a.fence),
                    Instant::now(),
                )
                .unwrap_err()
                .code(),
            QueryErrorCode::CursorInvalid
        );
        let legitimate = state
            .consume_cursor(
                &view_a.owner,
                &token,
                RESOURCE_RUNTIME,
                SCHEMA_RUNTIME_PAGE,
                CursorKind::Snapshot,
                Some(&view_a.fence),
                Instant::now(),
            )
            .unwrap();
        assert_eq!(legitimate.next_index, 1);
        assert_eq!(
            state
                .consume_cursor(
                    &view_a.owner,
                    &token,
                    RESOURCE_RUNTIME,
                    SCHEMA_RUNTIME_PAGE,
                    CursorKind::Snapshot,
                    Some(&view_a.fence),
                    Instant::now(),
                )
                .unwrap_err()
                .code(),
            QueryErrorCode::CursorInvalid
        );
        let tampered = OpaqueCursorToken::try_new(format!("{}A", token.as_str())).unwrap();
        assert_eq!(
            state
                .consume_cursor(
                    &view_a.owner,
                    &tampered,
                    RESOURCE_RUNTIME,
                    SCHEMA_RUNTIME_PAGE,
                    CursorKind::Snapshot,
                    Some(&view_a.fence),
                    Instant::now(),
                )
                .unwrap_err()
                .code(),
            QueryErrorCode::CursorInvalid
        );
    }

    #[test]
    fn cursor_expiry_capacity_retirement_and_replay_are_bounded() {
        let state = ControlPlaneQueryState::new().unwrap();
        let view = seeded_view(&state, "main");
        let now = Instant::now();
        let expired = state
            .issue_cursor(
                &view.owner,
                RESOURCE_RUNTIME,
                SCHEMA_RUNTIME_PAGE,
                &view.fence,
                0,
                CursorKind::Snapshot,
                now,
            )
            .unwrap();
        assert_eq!(
            state
                .consume_cursor(
                    &view.owner,
                    &expired,
                    RESOURCE_RUNTIME,
                    SCHEMA_RUNTIME_PAGE,
                    CursorKind::Snapshot,
                    Some(&view.fence),
                    now + CURSOR_TTL,
                )
                .unwrap_err()
                .code(),
            QueryErrorCode::CursorInvalid
        );
        let mut newest = None;
        for index in 0..(MAX_CURSORS_PER_WINDOW + 4) {
            newest = Some(
                state
                    .issue_cursor(
                        &view.owner,
                        RESOURCE_RUNTIME,
                        SCHEMA_RUNTIME_PAGE,
                        &view.fence,
                        index as u64,
                        CursorKind::Snapshot,
                        now,
                    )
                    .unwrap(),
            );
        }
        assert_eq!(
            state.cursor_count_for_window("main"),
            MAX_CURSORS_PER_WINDOW
        );
        let newest = newest.unwrap();
        state
            .consume_cursor(
                &view.owner,
                &newest,
                RESOURCE_RUNTIME,
                SCHEMA_RUNTIME_PAGE,
                CursorKind::Snapshot,
                Some(&view.fence),
                now,
            )
            .unwrap();
        assert_eq!(
            state
                .consume_cursor(
                    &view.owner,
                    &newest,
                    RESOURCE_RUNTIME,
                    SCHEMA_RUNTIME_PAGE,
                    CursorKind::Snapshot,
                    Some(&view.fence),
                    now,
                )
                .unwrap_err()
                .code(),
            QueryErrorCode::CursorInvalid
        );
        state.retire_window("main");
        assert_eq!(state.cursor_count_for_window("main"), 0);
    }

    #[test]
    fn process_cursor_capacity_is_bounded_across_windows() {
        let state = ControlPlaneQueryState::new().unwrap();
        let now = Instant::now();
        for window in 0..9 {
            let view = seeded_view(&state, &format!("window-{window}"));
            for index in 0..MAX_CURSORS_PER_WINDOW {
                state
                    .issue_cursor(
                        &view.owner,
                        RESOURCE_RUNTIME,
                        SCHEMA_RUNTIME_PAGE,
                        &view.fence,
                        index as u64,
                        CursorKind::Snapshot,
                        now,
                    )
                    .unwrap();
            }
        }
        assert_eq!(
            state.inner.lock().unwrap().cursors.len(),
            MAX_CURSORS_PER_PROCESS
        );
    }

    #[test]
    fn snapshot_cursor_rejects_stale_fence_and_page_bounds_are_strict() {
        let state = ControlPlaneQueryState::new().unwrap();
        let view = seeded_view(&state, "main");
        let first = page_from_items(
            &state,
            &view,
            PageRequest {
                limit: 1,
                cursor: None,
            },
            RESOURCE_RUNTIME,
            SCHEMA_RUNTIME_PAGE,
            &[
                view.runtime[0].clone(),
                RuntimeGenerationPayload {
                    domain: "video.transitions".to_string(),
                    generation: 1,
                    active: false,
                },
            ],
        )
        .unwrap();
        let cursor = first.next_cursor.clone().unwrap();
        let mut stale = view.clone();
        stale.fence.project_revision += 1;
        let error = page_from_items(
            &state,
            &stale,
            PageRequest {
                limit: 1,
                cursor: Some(cursor.clone()),
            },
            RESOURCE_RUNTIME,
            SCHEMA_RUNTIME_PAGE,
            &stale.runtime,
        )
        .unwrap_err();
        assert_eq!(error.code(), QueryErrorCode::CursorStale);
        state
            .consume_cursor(
                &view.owner,
                &cursor,
                RESOURCE_RUNTIME,
                SCHEMA_RUNTIME_PAGE,
                CursorKind::Snapshot,
                Some(&view.fence),
                Instant::now(),
            )
            .unwrap();
        assert_eq!(
            PageRequest {
                limit: 101,
                cursor: None
            }
            .validate()
            .unwrap_err(),
            protocol::control_plane_query::QueryContractValidationError::InvalidPageLimit
        );
    }

    #[test]
    fn project_replacement_is_redacted_and_emits_a_canonical_event() {
        let state = ControlPlaneQueryState::new().unwrap();
        let _ = seeded_view(&state, "main");
        let replacement = SourceCapture {
            project: test_project("project-b"),
            output: test_output(),
            runtime: vec![RuntimeCandidate {
                domain: "timeline.follow".to_string(),
                source_generation: 0,
                active: false,
            }],
        };
        let mut inner = state.inner.lock().unwrap();
        let before = inner.event_stream_generation;
        reconcile_observations(
            &mut inner,
            &replacement,
            state.process_incarnation,
            state.session_incarnation,
        )
        .unwrap();
        assert_eq!(inner.event_stream_generation, before + 1);
        let json = serde_json::to_string(&inner.events.back().unwrap().event).unwrap();
        assert!(json.contains(&fingerprint("project-b")));
        for forbidden in ["path", "snapshot", "mappings", "operator_policy", "error"] {
            assert!(
                !json.contains(forbidden),
                "leaked forbidden field {forbidden}"
            );
        }
    }

    #[test]
    fn retained_event_gap_is_explicit_and_requires_resnapshot() {
        let state = ControlPlaneQueryState::new().unwrap();
        let view = seeded_view(&state, "main");
        {
            let mut inner = state.inner.lock().unwrap();
            for generation in 0..(MAX_EVENT_RING + 8) {
                append_event(
                    &mut inner,
                    CanonicalObservationEventPayload::RuntimeGenerationChanged(
                        RuntimeGenerationPayload {
                            domain: "timeline.follow".to_string(),
                            generation: generation as u64 + 2,
                            active: false,
                        },
                    ),
                    state.process_incarnation,
                    state.session_incarnation,
                )
                .unwrap();
            }
        }
        let page = observation_page(
            &state,
            &view,
            EventPageRequest {
                limit: 10,
                expected_fence: Some(view.fence.clone()),
                cursor: None,
            },
        )
        .unwrap();
        page.validate().unwrap();
        let gap = page.gap.expect("retention overflow must be explicit");
        assert_eq!(gap.reason, GapReason::RetentionExpired);
        assert!(gap.resnapshot_required);
    }

    #[test]
    fn event_backlog_pages_compose_and_converge_to_the_exact_final_fence() {
        let state = ControlPlaneQueryState::new().unwrap();
        let initial = seeded_view(&state, "main");
        let replacement = SourceCapture {
            project: test_project("project-b"),
            output: test_output_lighting(),
            runtime: vec![RuntimeCandidate {
                domain: "timeline.follow".to_string(),
                source_generation: 1,
                active: true,
            }],
        };
        let final_fence = {
            let mut inner = state.inner.lock().unwrap();
            reconcile_observations(
                &mut inner,
                &replacement,
                state.process_incarnation,
                state.session_incarnation,
            )
            .unwrap();
            canonical_fence_from_inner(&inner, state.process_incarnation, state.session_incarnation)
                .unwrap()
                .unwrap()
        };

        let page_one = observation_page(
            &state,
            &initial,
            EventPageRequest {
                limit: 1,
                expected_fence: Some(initial.fence.clone()),
                cursor: None,
            },
        )
        .unwrap();
        page_one.validate().unwrap();
        assert_eq!(page_one.events.len(), 1);
        let page_two = observation_page(
            &state,
            &initial,
            EventPageRequest {
                limit: 1,
                expected_fence: None,
                cursor: page_one.next_cursor.clone(),
            },
        )
        .unwrap();
        page_two.validate().unwrap();
        assert_eq!(page_two.events.len(), 1);
        let page_three = observation_page(
            &state,
            &initial,
            EventPageRequest {
                limit: 1,
                expected_fence: None,
                cursor: page_two.next_cursor.clone(),
            },
        )
        .unwrap();
        page_three.validate().unwrap();
        assert_eq!(page_three.events.len(), 1);
        let converged = observation_page(
            &state,
            &initial,
            EventPageRequest {
                limit: 1,
                expected_fence: None,
                cursor: page_three.next_cursor.clone(),
            },
        )
        .unwrap();
        converged.validate().unwrap();
        assert!(converged.events.is_empty());
        assert_eq!(converged.snapshot_fence, final_fence);

        let mut composed = initial.fence.clone();
        for event in page_one
            .events
            .iter()
            .chain(page_two.events.iter())
            .chain(page_three.events.iter())
        {
            match &event.payload {
                CanonicalObservationEventPayload::ProjectAuthorityChanged(project) => {
                    composed.project_epoch = project.project_epoch;
                    composed.project_revision = project.project_revision;
                    composed.project_history_generation = project.project_history_generation;
                    composed.project_checkpoint_hash = project.project_checkpoint_hash.clone();
                    composed.project_publication_generation =
                        project.project_publication_generation;
                }
                CanonicalObservationEventPayload::RuntimeGenerationChanged(runtime) => {
                    let domain = composed
                        .runtime_domains
                        .iter_mut()
                        .find(|domain| domain.domain == runtime.domain)
                        .unwrap();
                    domain.generation = runtime.generation;
                }
                CanonicalObservationEventPayload::OutputOwnershipChanged(output) => {
                    composed.output_epoch = output.output_epoch;
                    composed.output_generation = output.output_generation;
                }
            }
            composed.event_stream_generation = event.generation;
        }
        assert_eq!(composed, final_fence);
        assert_eq!(
            page_two.snapshot_fence.event_stream_generation,
            page_one.events[0].generation
        );
        assert_eq!(
            page_three.snapshot_fence.event_stream_generation,
            page_two.events[0].generation
        );
    }

    #[test]
    fn initial_event_handoff_rejects_future_and_altered_fences_without_burning_valid_one() {
        let state = ControlPlaneQueryState::new().unwrap();
        let view = seeded_view(&state, "main");
        let mut forged = Vec::new();

        let mut future = view.fence.clone();
        future.event_stream_generation += 1;
        forged.push(future);
        let mut project = view.fence.clone();
        project.project_revision += 1;
        forged.push(project);
        let mut output = view.fence.clone();
        output.output_generation += 1;
        forged.push(output);
        let mut runtime = view.fence.clone();
        runtime.runtime_domains[0].generation += 1;
        forged.push(runtime);

        for fence in forged {
            let error = observation_page(
                &state,
                &view,
                EventPageRequest {
                    limit: 1,
                    expected_fence: Some(fence),
                    cursor: None,
                },
            )
            .unwrap_err();
            assert_eq!(error.code(), QueryErrorCode::SnapshotRequired);
        }
        let valid = observation_page(
            &state,
            &view,
            EventPageRequest {
                limit: 1,
                expected_fence: Some(view.fence.clone()),
                cursor: None,
            },
        )
        .unwrap();
        valid.validate().unwrap();
    }

    #[test]
    fn serialized_capture_prevents_delayed_older_source_regression() {
        use std::sync::{mpsc, Arc};

        let state = Arc::new(ControlPlaneQueryState::new().unwrap());
        let older = SourceCapture {
            project: test_project("project-a"),
            output: test_output(),
            runtime: vec![RuntimeCandidate {
                domain: "timeline.follow".to_string(),
                source_generation: 0,
                active: false,
            }],
        };
        let newer = SourceCapture {
            project: test_project("project-b"),
            output: test_output_lighting(),
            runtime: vec![RuntimeCandidate {
                domain: "timeline.follow".to_string(),
                source_generation: 1,
                active: true,
            }],
        };
        let (older_entered_tx, older_entered_rx) = mpsc::channel();
        let (release_older_tx, release_older_rx) = mpsc::channel();
        let older_state = Arc::clone(&state);
        let older_thread = std::thread::spawn(move || {
            older_state
                .capture_serialized("main", false, || {
                    older_entered_tx.send(()).unwrap();
                    release_older_rx.recv().unwrap();
                    Ok(older)
                })
                .unwrap()
        });
        older_entered_rx.recv().unwrap();

        let (newer_captured_tx, newer_captured_rx) = mpsc::channel();
        let newer_state = Arc::clone(&state);
        let newer_thread = std::thread::spawn(move || {
            newer_state
                .capture_serialized("main", false, || {
                    newer_captured_tx.send(()).unwrap();
                    Ok(newer)
                })
                .unwrap()
        });
        assert!(matches!(
            newer_captured_rx.recv_timeout(Duration::from_millis(50)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        release_older_tx.send(()).unwrap();
        older_thread.join().unwrap();
        newer_captured_rx.recv().unwrap();
        let newest_view = newer_thread.join().unwrap();
        assert_eq!(newest_view.project, test_project("project-b"));
        assert_eq!(
            state.inner.lock().unwrap().last_project,
            Some(test_project("project-b"))
        );
    }

    #[test]
    fn catalogs_are_sorted_bounded_and_full_lock_safe() {
        let catalog = schema_catalog();
        catalog.validate().unwrap();
        let state = ControlPlaneQueryState::new().unwrap();
        let view = seeded_view(&state, "main");
        let capabilities = capability_discovery(view.fence);
        capabilities.validate().unwrap();
        assert!(capabilities
            .capabilities
            .iter()
            .all(|capability| capability.allowed_during_full_lock));
    }

    #[test]
    fn real_engine_handle_capture_is_observational_and_history_free() {
        let harness = crate::tests::MediaAssetA6CommandHarness::new();
        let before_snapshot = harness.state.engine.snapshot();
        let before_status = harness.state.engine.output_ownership_status();
        let before_authority = {
            let coordinator = harness.state.project_coordinator.lock().unwrap();
            (
                coordinator.epoch,
                coordinator.revision,
                coordinator.history_generation,
                coordinator.history.undo.len(),
                coordinator.history.redo.len(),
                coordinator.publication_generation,
            )
        };
        let state = ControlPlaneQueryState::new().unwrap();
        let page = query_project_authority_for_window(
            "real-engine-window",
            &state,
            &harness.state,
            PageRequest::default(),
        )
        .unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].project_epoch, before_authority.0);
        assert_eq!(page.items[0].project_revision, before_authority.1);
        assert_eq!(page.items[0].project_history_generation, before_authority.2);
        assert_eq!(harness.state.engine.snapshot(), before_snapshot);
        assert_eq!(
            harness.state.engine.output_ownership_status(),
            before_status
        );
        let after_authority = {
            let coordinator = harness.state.project_coordinator.lock().unwrap();
            (
                coordinator.epoch,
                coordinator.revision,
                coordinator.history_generation,
                coordinator.history.undo.len(),
                coordinator.history.redo.len(),
                coordinator.publication_generation,
            )
        };
        assert_eq!(after_authority, before_authority);
    }
}
