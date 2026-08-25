//! Windows NLM live-network trust primitive for DJ-Link auto-start.
//!
//! Security contract (authoritative review decision):
//!
//! - Trust identity is EXACTLY the persisted `(network_guid, adapter_guid)`
//!   pair compared against the live NLM observation. Network class, name,
//!   category, domain type, connectivity state, registry profiles, IP/subnet/
//!   gateway data, and [`crate::dj_link_machine::DjLinkMachineSettingsV1::
//!   bind_ip`] are NEVER consulted by the decision.
//! - Auto-start Allow requires TWO stable observation passes, separated by a
//!   validated nonzero settle dwell of real elapsed sleep, that are identical
//!   canonical observations, plus
//!   exactly one CONNECTED live `INetwork` whose id equals the stored network
//!   GUID and whose connection adapter-id set contains the stored adapter
//!   GUID.
//! - Every anomaly blocks: COM/HRESULT/thread/join/channel errors, missing,
//!   NIL, or non-canonical GUIDs, zero or multiple network matches, adapter
//!   absence, unstable passes, duplicated network-id entries (explicit
//!   ambiguity instead of silent collapse), and fetched-count inconsistencies.
//!   Stored trust identity is never silently refreshed here.
//! - All COM work runs on ONE dedicated MTA thread (`CoInitializeEx` with
//!   `COINIT_MULTITHREADED`, balanced `CoUninitialize` on that same thread).
//!   COM interface pointers never cross a thread boundary; only plain
//!   observation data crosses channels.
//! - The deterministic policy layer below is pure and platform independent;
//!   the Windows observer is a thin adapter feeding it canonical data.

use std::collections::BTreeSet;
use std::thread;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Canonical locale-free UUID representation
// ---------------------------------------------------------------------------

/// Canonical lowercase hyphenated UUID text length.
pub const CANONICAL_UUID_LEN: usize = 36;

/// Decodes a strictly canonical lowercase hyphenated UUID into its 16 bytes
/// (big-endian field order, matching the RFC 4122 text form).
pub fn decode_canonical_uuid(value: &str) -> Option<[u8; 16]> {
    let bytes = value.as_bytes();
    if bytes.len() != CANONICAL_UUID_LEN {
        return None;
    }
    let mut decoded = [0_u8; 16];
    let mut read = 0_usize;
    let mut written = 0_usize;
    while read < CANONICAL_UUID_LEN {
        if matches!(read, 8 | 13 | 18 | 23) {
            if bytes[read] != b'-' {
                return None;
            }
            read += 1;
            continue;
        }
        let high = hex_nibble(bytes[read])?;
        let low = hex_nibble(bytes[read + 1])?;
        decoded[written] = (high << 4) | low;
        written += 1;
        read += 2;
    }
    debug_assert_eq!(written, 16);
    Some(decoded)
}

/// True only for a canonical lowercase hyphenated UUID that is not the all
/// zero NIL UUID. Uppercase, braced, compact, and partially formatted values
/// are all rejected; stored identity must round-trip byte-exact.
pub fn is_canonical_nonnil_uuid(value: &str) -> bool {
    match decode_canonical_uuid(value) {
        Some(decoded) => decoded != [0_u8; 16],
        None => false,
    }
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

/// Formats GUID fields as the canonical lowercase hyphenated text form. This
/// is locale independent and allocation-only; no OS text APIs are involved.
pub fn format_uuid_parts(data1: u32, data2: u16, data3: u16, data4: &[u8; 8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(CANONICAL_UUID_LEN);
    let _ = write!(
        out,
        "{data1:08x}-{data2:04x}-{data3:04x}-{0:02x}{1:02x}-",
        data4[0], data4[1]
    );
    for byte in &data4[2..] {
        let _ = write!(out, "{byte:02x}");
    }
    debug_assert_eq!(out.len(), CANONICAL_UUID_LEN);
    out
}

// ---------------------------------------------------------------------------
// Canonical observations
// ---------------------------------------------------------------------------

/// One observed NLM network: its exact live id, whether the network itself
/// reports connected, and the set of adapter ids attached through its live
/// connections. Adapter sets are order-insensitive; duplicate connection
/// entries collapse within one network because the security question is set
/// membership of the stored adapter id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DjLinkObservedNetwork {
    network_guid: String,
    connected: bool,
    adapter_guids: BTreeSet<String>,
}

impl DjLinkObservedNetwork {
    /// Validates every identifier as canonical and non-NIL before it may take
    /// part in a trust decision. Malformed live data fails closed.
    pub fn new(
        network_guid: String,
        connected: bool,
        adapter_guids: BTreeSet<String>,
    ) -> Result<Self, DjLinkObservationError> {
        if !is_canonical_nonnil_uuid(&network_guid) {
            return Err(DjLinkObservationError::MalformedNetworkGuid);
        }
        for adapter in &adapter_guids {
            if !is_canonical_nonnil_uuid(adapter) {
                return Err(DjLinkObservationError::MalformedAdapterGuid);
            }
        }
        Ok(Self {
            network_guid,
            connected,
            adapter_guids,
        })
    }

    pub fn network_guid(&self) -> &str {
        &self.network_guid
    }

    pub fn connected(&self) -> bool {
        self.connected
    }

    pub fn adapter_guids(&self) -> &BTreeSet<String> {
        &self.adapter_guids
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkObservationError {
    MalformedNetworkGuid,
    MalformedAdapterGuid,
}

/// One full enumeration pass over the live networks reported by NLM.
///
/// Canonical form: entries are kept sorted by their natural order, so two
/// passes compare equal regardless of enumeration order. Entries are NOT
/// deduplicated: two entries sharing one network id stay distinguishable and
/// force an explicit ambiguity verdict downstream instead of collapsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DjLinkNetworkObservations {
    entries: Vec<DjLinkObservedNetwork>,
}

impl DjLinkNetworkObservations {
    pub fn from_entries(
        mut entries: Vec<DjLinkObservedNetwork>,
    ) -> Result<Self, DjLinkObservationError> {
        for entry in &entries {
            if !is_canonical_nonnil_uuid(entry.network_guid()) {
                return Err(DjLinkObservationError::MalformedNetworkGuid);
            }
        }
        entries.sort();
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[DjLinkObservedNetwork] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// ---------------------------------------------------------------------------
// COM Next/fetched/S_FALSE semantics (pure seam)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkPageError {
    /// The underlying COM call returned a failure HRESULT.
    HostFailed { code: i32 },
    /// Fetched counts, buffer sizes, or stream discipline were inconsistent.
    ProtocolViolated,
    /// More items arrived than the bounded cap allows (DoS guard).
    LimitExceeded,
}

/// Outcome of one well-formed COM `Next` invocation.
///
/// COM contract modeled here: on success (`S_OK` or `S_FALSE`) the callee
/// reports how many elements were written; fewer than requested means the
/// stream ended (`S_FALSE`) and the enumeration MUST NOT be advanced again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawPageStatus {
    /// `fetched == requested`: the stream may hold more items.
    Continue,
    /// `fetched < requested`: terminal short fetch (`S_FALSE` semantics).
    Terminal,
}

/// Pure interpreter for one COM `Next` result. Any inconsistency between the
/// reported fetched count and actual buffer contents fails closed.
pub fn interpret_com_next(
    requested: u32,
    returned_items: u32,
    fetched_reported: u32,
) -> Result<RawPageStatus, DjLinkPageError> {
    if requested == 0 {
        return Err(DjLinkPageError::ProtocolViolated);
    }
    if returned_items > requested || fetched_reported != returned_items {
        return Err(DjLinkPageError::ProtocolViolated);
    }
    if fetched_reported < requested {
        Ok(RawPageStatus::Terminal)
    } else {
        Ok(RawPageStatus::Continue)
    }
}

/// Bounded enumeration configuration. Caps sit far above realistic network
/// counts; reaching one is treated as hostile input and fails closed. When a
/// FULL page lands exactly on the item cap the callee still reports that the
/// stream may continue, so the driver stops with
/// [`DjLinkPageError::LimitExceeded`] rather than issuing another page. A
/// TERMINAL short fetch that lands exactly on the cap succeeds, because the
/// callee itself declared the end of the stream.
#[derive(Debug, Clone, Copy)]
pub struct ComEnumerationLimits {
    pub page_size: u32,
    pub max_items: usize,
}

impl Default for ComEnumerationLimits {
    fn default() -> Self {
        Self {
            page_size: 8,
            max_items: 4096,
        }
    }
}

/// Drives repeated COM `Next` calls to exhaustion.
///
/// `step(requested)` performs EXACTLY one raw `Next`: it fills a buffer of
/// `requested` slots and returns the owned items actually written together
/// with the callee-reported fetched count. An `Err` is any failure HRESULT or
/// slot inconsistency already mapped by the adapter. The driver enforces:
/// consistent fetched counts, terminal short-fetch discipline (never
/// advancing past `S_FALSE`), per-page bounds, and the global item cap.
pub fn drain_com_pages<T>(
    mut step: impl FnMut(u32) -> Result<(Vec<T>, u32), DjLinkPageError>,
    limits: ComEnumerationLimits,
) -> Result<Vec<T>, DjLinkPageError> {
    if limits.page_size == 0 || limits.max_items == 0 {
        return Err(DjLinkPageError::LimitExceeded);
    }
    let mut collected: Vec<T> = Vec::new();
    loop {
        if collected.len() >= limits.max_items {
            return Err(DjLinkPageError::LimitExceeded);
        }
        let (items, fetched) = step(limits.page_size)?;
        let status = interpret_com_next(limits.page_size, items.len() as u32, fetched)?;
        collected.extend(items);
        if collected.len() > limits.max_items {
            return Err(DjLinkPageError::LimitExceeded);
        }
        if status == RawPageStatus::Terminal {
            return Ok(collected);
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic trust policy (pure)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkTrustDecision {
    Allow,
    Block(DjLinkTrustBlockReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkTrustBlockReason {
    /// Stored identity strings are missing, NIL, or non-canonical.
    StoredIdentityInvalid,
    /// The two settle-separated passes did not observe identical state.
    PassesDisagree,
    /// A network id appeared more than once in a pass: ambiguous topology.
    DuplicateNetworkAmbiguity,
    /// No connected live network carries the stored network id.
    StoredNetworkAbsent,
    /// More than one connected live network carries the stored id.
    MultipleStoredNetworkMatches,
    /// The matched network's connection set lacks the stored adapter id.
    StoredAdapterAbsent,
    /// The observer itself failed (COM/thread/join/channel/timeout).
    ObserverFailed(DjLinkObserverFailure),
}

/// Pure two-pass trust verdict over canonical observations. Deliberately has
/// NO parameter for IPs, names, classes, or any other classification input.
///
/// Verdict order (each earlier reason wins):
/// 1. [`DjLinkTrustBlockReason::StoredIdentityInvalid`] for NIL/non-canonical
///    stored strings,
/// 2. [`DjLinkTrustBlockReason::PassesDisagree`] for unstable passes,
/// 3. [`DjLinkTrustBlockReason::StoredNetworkAbsent`] /
///    [`DjLinkTrustBlockReason::MultipleStoredNetworkMatches`] counted over
///    CONNECTED entries carrying the stored network id,
/// 4. [`DjLinkTrustBlockReason::DuplicateNetworkAmbiguity`] when ANY network
///    id appears more than once in a pass (including a disconnected twin of
///    the stored id) — enumeration trust is poisoned by topology ambiguity,
/// 5. finally the stored adapter membership check.
pub fn decide_dj_link_network_trust(
    stored_network_guid: &str,
    stored_adapter_guid: &str,
    first_pass: &DjLinkNetworkObservations,
    second_pass: &DjLinkNetworkObservations,
) -> DjLinkTrustDecision {
    if !is_canonical_nonnil_uuid(stored_network_guid)
        || !is_canonical_nonnil_uuid(stored_adapter_guid)
    {
        return DjLinkTrustDecision::Block(DjLinkTrustBlockReason::StoredIdentityInvalid);
    }
    if first_pass != second_pass {
        return DjLinkTrustDecision::Block(DjLinkTrustBlockReason::PassesDisagree);
    }
    let entries = first_pass.entries();
    let connected_matches = entries
        .iter()
        .filter(|entry| entry.connected() && entry.network_guid() == stored_network_guid)
        .count();
    match connected_matches {
        0 => return DjLinkTrustDecision::Block(DjLinkTrustBlockReason::StoredNetworkAbsent),
        2.. => {
            return DjLinkTrustDecision::Block(DjLinkTrustBlockReason::MultipleStoredNetworkMatches)
        }
        1 => {}
    }
    let mut seen = BTreeSet::new();
    for entry in entries {
        if !seen.insert(entry.network_guid()) {
            return DjLinkTrustDecision::Block(DjLinkTrustBlockReason::DuplicateNetworkAmbiguity);
        }
    }
    let matched = entries
        .iter()
        .find(|entry| entry.connected() && entry.network_guid() == stored_network_guid)
        .expect("exactly one connected match established above");
    if matched.adapter_guids().contains(stored_adapter_guid) {
        DjLinkTrustDecision::Allow
    } else {
        DjLinkTrustDecision::Block(DjLinkTrustBlockReason::StoredAdapterAbsent)
    }
}

// ---------------------------------------------------------------------------
// Settle-dwell orchestration seam
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkObserverFailure {
    ComInit { code: i32 },
    ComObjectCreation { code: i32 },
    EnumerationHost { code: i32 },
    EnumerationProtocol,
    EnumerationLimitExceeded,
    ObservationMalformed,
    ObserverChannelClosed,
    ObserverTimeout,
    ObserverThreadPanic,
}

// ---------------------------------------------------------------------------
// Settle-dwell duration contract and timed two-pass orchestration
// ---------------------------------------------------------------------------

/// Inclusive ceiling for one settle dwell request. The dwell exists to let
/// live network topology settle between the two trust passes; requests above
/// this bound are treated as caller defects and fail closed instead of
/// freezing the decision thread for an unbounded time.
pub const MAX_DJ_LINK_SETTLE_DWELL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkSettleDwellError {
    /// A zero dwell cannot separate two observations; refusing it keeps the
    /// two-pass settle protocol meaningful. Never silently allowed.
    ZeroDuration,
    /// The requested dwell exceeds [`MAX_DJ_LINK_SETTLE_DWELL`].
    ExcessiveDuration { requested: Duration },
}

/// Fail-closed validation for caller-supplied dwell durations. Production
/// callers cannot reach the timed orchestration without passing through this
/// check: zero and unbounded durations are rejected, never silently allowed.
fn validate_settle_dwell(dwell: Duration) -> Result<(), DjLinkSettleDwellError> {
    if dwell.is_zero() {
        return Err(DjLinkSettleDwellError::ZeroDuration);
    }
    if dwell > MAX_DJ_LINK_SETTLE_DWELL {
        return Err(DjLinkSettleDwellError::ExcessiveDuration { requested: dwell });
    }
    Ok(())
}

/// The single production sleep binding used between observation passes. Kept
/// as a named function so tests exercise the exact binding production runs.
fn real_thread_sleep(duration: Duration) {
    thread::sleep(duration);
}

/// Runs the full two-pass protocol with a REAL bounded sleep between the
/// passes: validate the dwell fail-closed, observe, sleep exactly once via
/// the supplied binding, observe again, and decide. Any observer failure
/// folds into a blocking verdict; it never panics and never allows.
///
/// Private by contract: the only production entry point is the Windows MTA
/// observer's `evaluate_with_dwell`, which fixes both bindings to this
/// function (its own live `observe_once` and [`real_thread_sleep`]), so
/// callers can never supply a no-op dwell or a substitute observer. The
/// injected parameters exist solely for deterministic in-crate tests.
fn evaluate_with_timed_settle_dwell(
    stored_network_guid: &str,
    stored_adapter_guid: &str,
    settle_dwell: Duration,
    observe: &mut dyn FnMut() -> Result<DjLinkNetworkObservations, DjLinkObserverFailure>,
    sleep: &mut dyn FnMut(Duration),
) -> Result<DjLinkTrustDecision, DjLinkSettleDwellError> {
    validate_settle_dwell(settle_dwell)?;
    let first_pass = match observe() {
        Ok(pass) => pass,
        Err(failure) => {
            return Ok(DjLinkTrustDecision::Block(
                DjLinkTrustBlockReason::ObserverFailed(failure),
            ));
        }
    };
    sleep(settle_dwell);
    let second_pass = match observe() {
        Ok(pass) => pass,
        Err(failure) => {
            return Ok(DjLinkTrustDecision::Block(
                DjLinkTrustBlockReason::ObserverFailed(failure),
            ));
        }
    };
    Ok(decide_dj_link_network_trust(
        stored_network_guid,
        stored_adapter_guid,
        &first_pass,
        &second_pass,
    ))
}

// ---------------------------------------------------------------------------
// Dedicated MTA-thread Windows observer
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
pub use nlm_observer::NlmTrustObserver;

#[cfg(target_os = "windows")]
mod nlm_observer {
    //! Thin Windows adapter. The NLM COM object lives ONLY on this module's
    //! dedicated MTA thread; requests and replies carry plain data only.

    use super::{
        drain_com_pages, evaluate_with_timed_settle_dwell, format_uuid_parts, real_thread_sleep,
        ComEnumerationLimits, DjLinkNetworkObservations, DjLinkObservedNetwork,
        DjLinkObserverFailure, DjLinkPageError, DjLinkSettleDwellError, DjLinkTrustDecision,
    };
    use std::collections::BTreeSet;
    use std::sync::mpsc::{self, RecvTimeoutError, Sender};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;
    use windows::core::GUID;
    use windows::Win32::Networking::NetworkListManager::{
        IEnumNetworkConnections, IEnumNetworks, INetwork, INetworkConnection, INetworkListManager,
        NetworkListManager, NLM_ENUM_NETWORK_CONNECTED,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    const INIT_TIMEOUT: Duration = Duration::from_secs(5);
    const OBSERVE_TIMEOUT: Duration = Duration::from_secs(3);
    const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

    enum Request {
        Observe(Sender<Result<DjLinkNetworkObservations, DjLinkObserverFailure>>),
        Shutdown(Sender<Result<(), DjLinkObserverFailure>>),
    }

    /// Handle to the dedicated MTA observer thread. `Send` so it can live in
    /// application managed state; the COM pointers stay on the worker side.
    pub struct NlmTrustObserver {
        request_tx: Sender<Request>,
        worker: Option<JoinHandle<()>>,
    }

    const _: () = {
        const fn assert_send<T: Send>() {}
        assert_send::<NlmTrustObserver>();
    };

    impl NlmTrustObserver {
        /// Spawns the MTA thread and waits (bounded) for its COM
        /// initialization handshake. Fails closed on any initialization
        /// error; the worker is joined back onto the caller on those paths.
        pub fn spawn() -> Result<Self, DjLinkObserverFailure> {
            let (request_tx, request_rx) = mpsc::channel::<Request>();
            let (ready_tx, ready_rx) = mpsc::channel::<Result<(), DjLinkObserverFailure>>();
            let worker = thread::Builder::new()
                .name("syndocal-nlm-trust".to_string())
                .spawn(move || worker_body(request_rx, ready_tx))
                .map_err(|_| DjLinkObserverFailure::ObserverThreadPanic)?;
            match ready_rx.recv_timeout(INIT_TIMEOUT) {
                Ok(Ok(())) => Ok(Self {
                    request_tx,
                    worker: Some(worker),
                }),
                Ok(Err(failure)) => {
                    let joined = worker.join();
                    debug_assert!(joined.is_ok(), "worker panicked during init");
                    Err(failure)
                }
                Err(_) => {
                    // Handshake timed out: a thread blocked inside COM cannot
                    // be killed safely, so it is left to finish on its own
                    // while this handle fails closed and drops its channel.
                    Err(DjLinkObserverFailure::ObserverTimeout)
                }
            }
        }

        /// Runs exactly one live enumeration pass on the MTA thread.
        pub fn observe_once(&self) -> Result<DjLinkNetworkObservations, DjLinkObserverFailure> {
            let (reply_tx, reply_rx) = mpsc::channel();
            self.request_tx
                .send(Request::Observe(reply_tx))
                .map_err(|_| DjLinkObserverFailure::ObserverChannelClosed)?;
            match reply_rx.recv_timeout(OBSERVE_TIMEOUT) {
                Ok(result) => result,
                Err(RecvTimeoutError::Timeout) => Err(DjLinkObserverFailure::ObserverTimeout),
                Err(RecvTimeoutError::Disconnected) => {
                    Err(DjLinkObserverFailure::ObserverChannelClosed)
                }
            }
        }

        /// Two-pass protocol over this live observer. `settle_dwell` must be
        /// a nonzero duration within [`MAX_DJ_LINK_SETTLE_DWELL`]; invalid
        /// values fail closed with a typed error before any observation runs.
        /// THIS method performs the real OS sleep exactly once between the
        /// two passes via the fixed production binding, so callers can never
        /// inject a no-op dwell.
        pub fn evaluate_with_dwell(
            &self,
            stored_network_guid: &str,
            stored_adapter_guid: &str,
            settle_dwell: Duration,
        ) -> Result<DjLinkTrustDecision, DjLinkSettleDwellError> {
            let mut observe = || self.observe_once();
            evaluate_with_timed_settle_dwell(
                stored_network_guid,
                stored_adapter_guid,
                settle_dwell,
                &mut observe,
                &mut real_thread_sleep,
            )
        }

        /// Explicit cooperative shutdown: joins the worker and surfaces join
        /// failures so callers can block on thread errors as required.
        pub fn shutdown(mut self) -> Result<(), DjLinkObserverFailure> {
            self.request_shutdown();
            match self.worker.take() {
                Some(worker) => worker
                    .join()
                    .map_err(|_| DjLinkObserverFailure::ObserverThreadPanic),
                None => Ok(()),
            }
        }

        fn request_shutdown(&self) {
            let (ack_tx, ack_rx) = mpsc::channel();
            if self.request_tx.send(Request::Shutdown(ack_tx)).is_ok() {
                let _ = ack_rx.recv_timeout(SHUTDOWN_TIMEOUT);
            }
        }
    }

    impl Drop for NlmTrustObserver {
        fn drop(&mut self) {
            self.request_shutdown();
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }

    fn worker_body(
        request_rx: mpsc::Receiver<Request>,
        ready_tx: Sender<Result<(), DjLinkObserverFailure>>,
    ) {
        // SAFETY: dedicated thread created immediately above; this is its
        // first COM call and the balancing CoUninitialize below runs on the
        // same thread before it exits. S_OK and S_FALSE both leave the
        // apartment initialized exactly once from this thread's perspective.
        let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if initialized.is_err() {
            // Includes RPC_E_CHANGED_MODE: an STA thread can never satisfy
            // the MTA-only contract, so initialization fails closed.
            let _ = ready_tx.send(Err(DjLinkObserverFailure::ComInit {
                code: initialized.0,
            }));
            return;
        }
        // SAFETY: in-proc activation of the fixed NLM CLSID; the returned
        // interface pointer stays on this thread for the worker's lifetime.
        let manager: Result<INetworkListManager, _> =
            unsafe { CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL) };
        let manager = match manager {
            Ok(manager) => manager,
            Err(error) => {
                let _ = ready_tx.send(Err(DjLinkObserverFailure::ComObjectCreation {
                    code: error.code().0,
                }));
                // SAFETY: balances the successful CoInitializeEx above.
                unsafe { CoUninitialize() };
                return;
            }
        };
        if ready_tx.send(Ok(())).is_err() {
            // SAFETY: same thread as above; no receiver remains.
            unsafe { CoUninitialize() };
            return;
        }
        while let Ok(request) = request_rx.recv() {
            match request {
                Request::Observe(reply) => {
                    let _ = reply.send(observe_connected_networks(&manager));
                }
                Request::Shutdown(ack) => {
                    let _ = ack.send(Ok(()));
                    break;
                }
            }
        }
        // SAFETY: balances CoInitializeEx on this same thread.
        unsafe { CoUninitialize() };
    }

    fn observe_connected_networks(
        manager: &INetworkListManager,
    ) -> Result<DjLinkNetworkObservations, DjLinkObserverFailure> {
        // SAFETY: manager is a valid interface living on this thread; the
        // returned enumerator is consumed before this call returns.
        let networks =
            unsafe { manager.GetNetworks(NLM_ENUM_NETWORK_CONNECTED) }.map_err(|error| {
                DjLinkObserverFailure::EnumerationHost {
                    code: error.code().0,
                }
            })?;
        let limits = ComEnumerationLimits::default();
        let network_items = drain_com_pages(
            |requested| next_interface_page::<INetwork>(&networks, requested),
            limits,
        )
        .map_err(map_page_error)?;
        let mut entries = Vec::with_capacity(network_items.len());
        for network in network_items {
            // SAFETY: network came straight from the enumerator on this
            // thread; property reads are apartment-local and synchronous.
            let id = unsafe { network.GetNetworkId() }.map_err(hresult_failure)?;
            let connected = unsafe { network.IsConnected() }.map_err(hresult_failure)?;
            let connections =
                unsafe { network.GetNetworkConnections() }.map_err(hresult_failure)?;
            let connection_items = drain_com_pages(
                |requested| next_interface_page::<INetworkConnection>(&connections, requested),
                limits,
            )
            .map_err(map_page_error)?;
            let mut adapter_guids = BTreeSet::new();
            for connection in connection_items {
                // SAFETY: apartment-local property read on a live connection.
                let adapter = unsafe { connection.GetAdapterId() }.map_err(hresult_failure)?;
                adapter_guids.insert(format_windows_guid(&adapter));
            }
            let entry = DjLinkObservedNetwork::new(
                format_windows_guid(&id),
                connected.as_bool(),
                adapter_guids,
            )
            .map_err(|_| DjLinkObserverFailure::ObservationMalformed)?;
            entries.push(entry);
        }
        DjLinkNetworkObservations::from_entries(entries)
            .map_err(|_| DjLinkObserverFailure::ObservationMalformed)
    }

    fn hresult_failure(error: windows::core::Error) -> DjLinkObserverFailure {
        DjLinkObserverFailure::EnumerationHost {
            code: error.code().0,
        }
    }

    fn map_page_error(error: DjLinkPageError) -> DjLinkObserverFailure {
        match error {
            DjLinkPageError::HostFailed { code } => DjLinkObserverFailure::EnumerationHost { code },
            DjLinkPageError::ProtocolViolated => DjLinkObserverFailure::EnumerationProtocol,
            DjLinkPageError::LimitExceeded => DjLinkObserverFailure::EnumerationLimitExceeded,
        }
    }

    /// One raw COM `Next` call over either pinned enumerator, shaped for
    /// [`drain_com_pages`]. Enforces that every slot the callee claims was
    /// actually populated before handing ownership over.
    fn next_interface_page<T>(
        enumerator: &impl EnumeratorOf<T>,
        requested: u32,
    ) -> Result<(Vec<T>, u32), DjLinkPageError> {
        let requested = requested as usize;
        let mut buffer: Vec<Option<T>> = (0..requested).map(|_| None).collect();
        let mut fetched: u32 = 0;
        // SAFETY: apartment-live enumerator; buffer/fetched are the generated
        // out-parameters and stay alive for the duration of the call.
        let result = unsafe { enumerator.next_raw(&mut buffer, Some(&mut fetched as *mut _)) };
        if let Err(error) = result {
            return Err(DjLinkPageError::HostFailed {
                code: error.code().0,
            });
        }
        if fetched as usize > requested {
            return Err(DjLinkPageError::ProtocolViolated);
        }
        let mut items = Vec::with_capacity(fetched as usize);
        for slot in buffer.into_iter().take(fetched as usize) {
            match slot {
                Some(item) => items.push(item),
                None => return Err(DjLinkPageError::ProtocolViolated),
            }
        }
        Ok((items, fetched))
    }

    /// Minimal uniform surface over `IEnumNetworks`/`IEnumNetworkConnections`
    /// so one page helper serves both pinned enumerators.
    trait EnumeratorOf<T> {
        /// # Safety
        /// Caller guarantees the enumerator is apartment-live.
        unsafe fn next_raw(
            &self,
            buffer: &mut [Option<T>],
            fetched: Option<*mut u32>,
        ) -> windows::core::Result<()>;
    }

    impl EnumeratorOf<INetwork> for IEnumNetworks {
        unsafe fn next_raw(
            &self,
            buffer: &mut [Option<INetwork>],
            fetched: Option<*mut u32>,
        ) -> windows::core::Result<()> {
            // SAFETY: apartment-live enumerator; out-params forwarded verbatim.
            unsafe { Self::Next(self, buffer, fetched) }
        }
    }

    impl EnumeratorOf<INetworkConnection> for IEnumNetworkConnections {
        unsafe fn next_raw(
            &self,
            buffer: &mut [Option<INetworkConnection>],
            fetched: Option<*mut u32>,
        ) -> windows::core::Result<()> {
            // SAFETY: apartment-live enumerator; out-params forwarded verbatim.
            unsafe { Self::Next(self, buffer, fetched) }
        }
    }

    fn format_windows_guid(guid: &GUID) -> String {
        format_uuid_parts(guid.data1, guid.data2, guid.data3, &guid.data4)
    }

    #[cfg(test)]
    mod windows_tests {
        use super::{
            DjLinkNetworkObservations, DjLinkObserverFailure, DjLinkSettleDwellError,
            DjLinkTrustDecision, NlmTrustObserver,
        };
        use std::time::Duration;

        #[test]
        fn observer_handle_is_send_for_managed_state() {
            fn assert_send<T: Send>() {}
            assert_send::<NlmTrustObserver>();
        }

        /// The Windows observer surface is wired into the application by a
        /// later integration lane; until then this test declares its intended
        /// liveness WITHOUT touching COM (pure fn-pointer coercions, no call).
        /// It also pins the exact exported signatures future wiring compiles
        /// against.
        #[test]
        fn observer_surface_signatures_stay_pinned_without_com_side_effects() {
            let spawn: fn() -> Result<NlmTrustObserver, DjLinkObserverFailure> =
                NlmTrustObserver::spawn;
            let observe_once: fn(
                &NlmTrustObserver,
            )
                -> Result<DjLinkNetworkObservations, DjLinkObserverFailure> =
                NlmTrustObserver::observe_once;
            let evaluate_with_dwell: fn(
                &NlmTrustObserver,
                &str,
                &str,
                Duration,
            )
                -> Result<DjLinkTrustDecision, DjLinkSettleDwellError> =
                NlmTrustObserver::evaluate_with_dwell;
            let shutdown: fn(NlmTrustObserver) -> Result<(), DjLinkObserverFailure> =
                NlmTrustObserver::shutdown;
            // Binding (not calling) the pointers is the whole point: zero COM
            // side effects while keeping the exported surface lint-live.
            let _pinned_surface = (spawn, observe_once, evaluate_with_dwell, shutdown);
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const NET_A: &str = "a1000000-0000-4000-8000-000000000001";
    pub(crate) const NET_B: &str = "b2000000-0000-4000-8000-000000000002";
    pub(crate) const ADA_1: &str = "c3000000-0000-4000-8000-000000000003";
    pub(crate) const ADA_2: &str = "d4000000-0000-4000-8000-000000000004";

    fn entry(network: &str, connected: bool, adapters: &[&str]) -> DjLinkObservedNetwork {
        DjLinkObservedNetwork::new(
            network.to_string(),
            connected,
            adapters.iter().map(|a| (*a).to_string()).collect(),
        )
        .expect("valid entry")
    }

    fn observations(entries: Vec<DjLinkObservedNetwork>) -> DjLinkNetworkObservations {
        DjLinkNetworkObservations::from_entries(entries).expect("valid observations")
    }

    fn single_pass_net_a() -> DjLinkNetworkObservations {
        observations(vec![entry(NET_A, true, &[ADA_1])])
    }

    // -- canonical UUID helpers ----------------------------------------------

    #[test]
    fn canonical_uuid_accepts_only_lowercase_hyphenated_nonnil() {
        assert!(is_canonical_nonnil_uuid(NET_A));
        assert!(decode_canonical_uuid(NET_A).is_some());
        for rejected in [
            "",
            NET_A.to_uppercase().as_str(),
            "a1000000000040008000000000000001",
            "{a1000000-0000-4000-8000-000000000001}",
            "a1000000-0000-4000-8000-00000000000",
            "a1000000-0000-4000-8000-0000000000011",
            "g1000000-0000-4000-8000-000000000001",
            "a1000000-0000-4000-8000-00000000000g",
            "a1000000/0000/4000/8000/000000000001",
            "00000000-0000-0000-0000-000000000000",
        ] {
            assert!(!is_canonical_nonnil_uuid(rejected), "accepted {rejected:?}");
        }
        assert_eq!(
            decode_canonical_uuid("00000000-0000-0000-0000-000000000000"),
            Some([0_u8; 16]),
            "NIL decodes but must fail the non-nil predicate"
        );
    }

    #[test]
    fn uuid_formatting_roundtrips_field_layout() {
        let text = format_uuid_parts(
            0xdcb00c01,
            0x570f,
            0x4a9b,
            &[0x8d, 0x69, 0x19, 0x9f, 0xdb, 0xa5, 0x72, 0x3b],
        );
        assert_eq!(text, "dcb00c01-570f-4a9b-8d69-199fdba5723b");
        assert_eq!(text.len(), CANONICAL_UUID_LEN);
        assert!(is_canonical_nonnil_uuid(&text));
    }

    // -- observation canonicalization ----------------------------------------

    #[test]
    fn observations_are_order_insensitive_and_duplicates_preserved() {
        let shuffled = observations(vec![
            entry(NET_B, false, &[ADA_2]),
            entry(NET_A, true, &[ADA_1, ADA_2]),
            entry(NET_B, true, &[ADA_1]),
        ]);
        let ordered = observations(vec![
            entry(NET_A, true, &[ADA_1, ADA_2]),
            entry(NET_B, false, &[ADA_2]),
            entry(NET_B, true, &[ADA_1]),
        ]);
        assert_eq!(
            shuffled, ordered,
            "canonical ordering must ignore input order"
        );
        assert_eq!(ordered.entries().len(), 3, "duplicate ids stay distinct");
        let duplicated = observations(vec![
            entry(NET_A, true, &[ADA_1]),
            entry(NET_A, true, &[ADA_1]),
        ]);
        assert_eq!(
            duplicated.entries().len(),
            2,
            "identical duplicate entries must not collapse"
        );
    }

    #[test]
    fn observation_construction_rejects_malformed_and_nil_ids() {
        assert_eq!(
            DjLinkObservedNetwork::new(
                "00000000-0000-0000-0000-000000000000".to_string(),
                true,
                BTreeSet::new()
            ),
            Err(DjLinkObservationError::MalformedNetworkGuid)
        );
        assert_eq!(
            DjLinkObservedNetwork::new("not-a-uuid".to_string(), true, BTreeSet::new()),
            Err(DjLinkObservationError::MalformedNetworkGuid)
        );
        assert_eq!(
            DjLinkObservedNetwork::new(NET_A.to_string(), true, ["nope".to_string()].into()),
            Err(DjLinkObservationError::MalformedAdapterGuid)
        );
        assert!(
            DjLinkObservedNetwork::new(NET_A.to_string(), false, BTreeSet::new()).is_ok(),
            "disconnected-but-valid observations are representable"
        );
    }

    // -- COM Next/fetched/S_FALSE interpreter --------------------------------

    #[test]
    fn com_next_interpreter_models_full_short_and_inconsistent_fetches() {
        assert_eq!(
            interpret_com_next(8, 8, 8),
            Ok(RawPageStatus::Continue),
            "S_OK full page continues"
        );
        assert_eq!(
            interpret_com_next(8, 3, 3),
            Ok(RawPageStatus::Terminal),
            "S_FALSE partial page terminates"
        );
        assert_eq!(
            interpret_com_next(8, 0, 0),
            Ok(RawPageStatus::Terminal),
            "S_FALSE empty page terminates"
        );
        for (requested, returned, fetched) in [
            (0_u32, 0_u32, 0_u32),
            (8, 9, 9),
            (8, 8, 7),
            (8, 3, 4),
            (8, 0, 1),
        ] {
            assert_eq!(
                interpret_com_next(requested, returned, fetched),
                Err(DjLinkPageError::ProtocolViolated),
                "(requested={requested}, returned={returned}, fetched={fetched}) must violate"
            );
        }
    }

    #[test]
    fn drain_collects_all_pages_and_stops_at_terminal_short_fetch() {
        let script = [
            (vec![1_u32, 2, 3, 4, 5, 6, 7, 8], 8_u32),
            (vec![9, 10, 11, 12, 13, 14, 15, 16], 8),
            (vec![17, 18], 2),
        ];
        let mut cursor = 0_usize;
        let mut calls = 0_usize;
        let drained = drain_com_pages(
            |requested| {
                assert_eq!(requested, 8, "driver must request bounded pages");
                calls += 1;
                let (items, fetched) = script[cursor].clone();
                cursor += 1;
                Ok((items, fetched))
            },
            ComEnumerationLimits::default(),
        )
        .expect("drain succeeds");
        assert_eq!(
            drained,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18]
        );
        assert_eq!(calls, 3, "driver must not advance past the terminal page");
    }

    #[test]
    fn drain_fail_closed_on_host_errors_terminal_discipline_and_caps() {
        let host_failure: Result<Vec<u32>, DjLinkPageError> = drain_com_pages(
            |_requested| Err(DjLinkPageError::HostFailed { code: -2147024891 }),
            ComEnumerationLimits::default(),
        );
        assert_eq!(
            host_failure,
            Err(DjLinkPageError::HostFailed { code: -2147024891 }),
            "HRESULT failures propagate verbatim"
        );

        let mut advanced_past_terminal = false;
        let terminal_empty: Result<Vec<u32>, DjLinkPageError> = drain_com_pages(
            |_requested| {
                if advanced_past_terminal {
                    panic!("driver advanced past a terminal short fetch");
                }
                advanced_past_terminal = true;
                // Consistent S_FALSE terminal page: nothing written, zero
                // fetched, stream ended.
                Ok((Vec::new(), 0))
            },
            ComEnumerationLimits::default(),
        );
        assert_eq!(terminal_empty, Ok(Vec::<u32>::new()));

        // A callee claiming a nonzero fetched count without producing the
        // matching items is an inconsistent fetch and must fail closed.
        let inconsistent: Result<Vec<u32>, DjLinkPageError> = drain_com_pages(
            |_requested| Ok((vec![1_u32], 2_u32)),
            ComEnumerationLimits::default(),
        );
        assert_eq!(
            inconsistent,
            Err(DjLinkPageError::ProtocolViolated),
            "fetched count must match produced items"
        );

        let mut counter = 0_u64;
        assert_eq!(
            drain_com_pages(
                move |requested| {
                    counter += 1;
                    Ok((vec![counter as u32; requested as usize], requested))
                },
                ComEnumerationLimits::default()
            ),
            Err(DjLinkPageError::LimitExceeded),
            "endless streams hit the hard cap"
        );

        assert_eq!(
            drain_com_pages(
                |_requested| Ok((Vec::<u32>::new(), 0)),
                ComEnumerationLimits {
                    page_size: 0,
                    max_items: 16
                }
            ),
            Err(DjLinkPageError::LimitExceeded),
            "degenerate configuration fails closed"
        );
    }

    #[test]
    fn drain_terminal_short_fetch_landing_exactly_on_cap_succeeds() {
        let script = [
            (vec![1_u32, 2, 3, 4, 5, 6, 7, 8], 8_u32),
            (vec![9, 10, 11, 12], 4),
        ];
        let mut cursor = 0_usize;
        let mut calls = 0_usize;
        let drained = drain_com_pages(
            |requested| {
                assert_eq!(requested, 8, "driver must request bounded pages");
                calls += 1;
                let (items, fetched) = script[cursor].clone();
                cursor += 1;
                Ok((items, fetched))
            },
            ComEnumerationLimits {
                page_size: 8,
                max_items: 12,
            },
        )
        .expect("a terminal short fetch declaring the end at the cap is bounded and safe");
        assert_eq!(drained, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        assert_eq!(calls, 2, "driver must not advance past the terminal page");
    }

    #[test]
    fn drain_full_page_landing_exactly_on_cap_fails_closed() {
        let mut calls = 0_usize;
        let outcome = drain_com_pages(
            |requested| {
                calls += 1;
                Ok((vec![7_u32; requested as usize], requested))
            },
            ComEnumerationLimits {
                page_size: 8,
                max_items: 16,
            },
        );
        assert_eq!(
            outcome,
            Err(DjLinkPageError::LimitExceeded),
            "a full page at the cap still reports possible continuation"
        );
        assert_eq!(calls, 2, "no further page is issued once the cap is full");
    }

    // -- policy --------------------------------------------------------------

    #[test]
    fn exact_stable_match_allows() {
        let pass = single_pass_net_a();
        assert_eq!(
            decide_dj_link_network_trust(NET_A, ADA_1, &pass, &pass),
            DjLinkTrustDecision::Allow
        );
    }

    #[test]
    fn unstable_passes_block_before_any_matching() {
        let first = single_pass_net_a();
        let second = observations(vec![entry(NET_A, true, &[])]);
        assert_eq!(
            decide_dj_link_network_trust(NET_A, ADA_1, &first, &second),
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::PassesDisagree)
        );
        let flipped = observations(vec![entry(NET_B, true, &[ADA_1])]);
        assert_eq!(
            decide_dj_link_network_trust(NET_A, ADA_1, &first, &flipped),
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::PassesDisagree)
        );
    }

    #[test]
    fn same_network_different_adapter_blocks() {
        let pass = observations(vec![entry(NET_A, true, &[ADA_2])]);
        assert_eq!(
            decide_dj_link_network_trust(NET_A, ADA_1, &pass, &pass),
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::StoredAdapterAbsent)
        );
        let multi_adapter = observations(vec![entry(NET_A, true, &[ADA_1, ADA_2])]);
        assert_eq!(
            decide_dj_link_network_trust(NET_A, ADA_1, &multi_adapter, &multi_adapter),
            DjLinkTrustDecision::Allow,
            "stored adapter may be one of several live connections"
        );
    }

    #[test]
    fn missing_or_empty_network_blocks() {
        let pass = observations(vec![entry(NET_B, true, &[ADA_1])]);
        assert_eq!(
            decide_dj_link_network_trust(NET_A, ADA_1, &pass, &pass),
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::StoredNetworkAbsent)
        );
        assert_eq!(
            decide_dj_link_network_trust(
                NET_A,
                ADA_1,
                &observations(vec![]),
                &observations(vec![])
            ),
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::StoredNetworkAbsent)
        );
    }

    #[test]
    fn disconnected_network_is_excluded_not_matched() {
        let pass = observations(vec![entry(NET_A, false, &[ADA_1])]);
        assert_eq!(
            decide_dj_link_network_trust(NET_A, ADA_1, &pass, &pass),
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::StoredNetworkAbsent)
        );
        let mixed = observations(vec![
            entry(NET_A, false, &[ADA_1]),
            entry(NET_B, true, &[ADA_2]),
        ]);
        assert_eq!(
            decide_dj_link_network_trust(NET_A, ADA_1, &mixed, &mixed),
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::StoredNetworkAbsent)
        );
    }

    #[test]
    fn multiple_matching_networks_block_as_ambiguous() {
        let pass = observations(vec![
            entry(NET_A, true, &[ADA_1]),
            entry(NET_A, true, &[ADA_1]),
        ]);
        assert_eq!(
            decide_dj_link_network_trust(NET_A, ADA_1, &pass, &pass),
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::MultipleStoredNetworkMatches)
        );
    }

    #[test]
    fn duplicate_network_id_anywhere_forces_explicit_ambiguity() {
        let pass = observations(vec![
            entry(NET_A, true, &[ADA_1]),
            entry(NET_B, false, &[ADA_2]),
            entry(NET_B, true, &[ADA_2]),
        ]);
        assert_eq!(
            decide_dj_link_network_trust(NET_A, ADA_1, &pass, &pass),
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::DuplicateNetworkAmbiguity)
        );
    }

    #[test]
    fn connected_plus_disconnected_stored_id_twin_blocks_as_ambiguous() {
        let pass = observations(vec![
            entry(NET_A, true, &[ADA_1]),
            entry(NET_A, false, &[ADA_1]),
        ]);
        assert_eq!(
            decide_dj_link_network_trust(NET_A, ADA_1, &pass, &pass),
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::DuplicateNetworkAmbiguity),
            "a disconnected twin of the stored id poisons enumeration trust \
             even though exactly one connected copy exists"
        );
    }

    #[test]
    fn invalid_stored_identity_blocks_regardless_of_live_state() {
        let pass = single_pass_net_a();
        for (network, adapter) in [
            ("00000000-0000-0000-0000-000000000000", ADA_1),
            (NET_A, "00000000-0000-0000-0000-000000000000"),
            (NET_A.to_uppercase().as_str(), ADA_1),
            (NET_A, ""),
            ("", ""),
        ] {
            assert_eq!(
                decide_dj_link_network_trust(network, adapter, &pass, &pass),
                DjLinkTrustDecision::Block(DjLinkTrustBlockReason::StoredIdentityInvalid),
                "({network:?}, {adapter:?})"
            );
        }
    }

    #[test]
    fn decision_never_consults_bind_ip() {
        use crate::dj_link_machine::{
            DjLinkMachineSettingsV1, DjLinkMachineTransactionState,
            DJ_LINK_MACHINE_SETTINGS_VERSION,
        };
        let make_settings = |bind_ip: &str| DjLinkMachineSettingsV1 {
            version: DJ_LINK_MACHINE_SETTINGS_VERSION,
            revision: 3,
            transaction_state: DjLinkMachineTransactionState::Idle,
            credential_generation: Some(1),
            adapter_guid: Some(ADA_1.to_string()),
            network_guid: Some(NET_A.to_string()),
            bind_ip: Some(bind_ip.to_string()),
            bind_port: Some(49152),
            auto_start_armed: true,
        };
        let left = make_settings("192.168.7.9");
        let right = make_settings("10.255.255.1");
        assert_ne!(left.bind_ip, right.bind_ip);
        let pass = single_pass_net_a();
        let left_decision = decide_dj_link_network_trust(
            left.network_guid.as_deref().expect("pair present"),
            left.adapter_guid.as_deref().expect("pair present"),
            &pass,
            &pass,
        );
        let right_decision = decide_dj_link_network_trust(
            right.network_guid.as_deref().expect("pair present"),
            right.adapter_guid.as_deref().expect("pair present"),
            &pass,
            &pass,
        );
        assert_eq!(left_decision, DjLinkTrustDecision::Allow);
        assert_eq!(left_decision, right_decision);

        let flipped_adapter = observations(vec![entry(NET_A, true, &[ADA_2])]);
        assert_ne!(
            decide_dj_link_network_trust(NET_A, ADA_1, &flipped_adapter, &flipped_adapter),
            left_decision,
            "control: the decision DOES react to identity changes"
        );
    }

    // -- timed dwell orchestration -------------------------------------------

    /// Conservative test dwell: far above scheduling jitter for the
    /// lower-bound assertion, small enough to keep the suite fast.
    const TEST_DWELL: Duration = Duration::from_millis(150);

    #[derive(Default)]
    struct SleepRecorder {
        calls: Vec<Duration>,
    }

    impl SleepRecorder {
        fn record(&mut self, duration: Duration) {
            self.calls.push(duration);
        }
    }

    struct ScriptedObserver {
        calls: Vec<&'static str>,
        fail_on: Option<usize>,
        passes: Vec<DjLinkNetworkObservations>,
    }

    impl ScriptedObserver {
        fn failing_first(pass: DjLinkNetworkObservations) -> Self {
            Self {
                calls: Vec::new(),
                fail_on: Some(0),
                passes: vec![pass],
            }
        }

        fn stable_pair(pass: DjLinkNetworkObservations) -> Self {
            Self {
                calls: Vec::new(),
                fail_on: None,
                passes: vec![pass.clone(), pass],
            }
        }

        fn observe(&mut self) -> Result<DjLinkNetworkObservations, DjLinkObserverFailure> {
            let index = self.calls.len();
            self.calls.push("observe");
            if self.fail_on == Some(index) {
                return Err(DjLinkObserverFailure::EnumerationHost { code: -1 });
            }
            Ok(self.passes.remove(0))
        }
    }

    #[test]
    fn evaluation_runs_two_passes_around_exactly_one_timed_sleep() {
        let mut observer = ScriptedObserver::stable_pair(single_pass_net_a());
        let mut recorder = SleepRecorder::default();
        let decision = evaluate_with_timed_settle_dwell(
            NET_A,
            ADA_1,
            TEST_DWELL,
            &mut || observer.observe(),
            &mut |duration| recorder.record(duration),
        )
        .expect("validated dwell");
        assert_eq!(decision, DjLinkTrustDecision::Allow);
        assert_eq!(observer.calls, vec!["observe", "observe"]);
        assert_eq!(
            recorder.calls,
            vec![TEST_DWELL],
            "exactly one dwell request between the passes"
        );
    }

    #[test]
    fn observer_failure_folds_into_block_and_first_pass_never_sleeps() {
        let mut first_fails = ScriptedObserver::failing_first(single_pass_net_a());
        let mut recorder = SleepRecorder::default();
        let decision = evaluate_with_timed_settle_dwell(
            NET_A,
            ADA_1,
            TEST_DWELL,
            &mut || first_fails.observe(),
            &mut |duration| recorder.record(duration),
        )
        .expect("validated dwell");
        assert_eq!(
            decision,
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::ObserverFailed(
                DjLinkObserverFailure::EnumerationHost { code: -1 }
            ))
        );
        assert_eq!(
            first_fails.calls.len(),
            1,
            "no second pass after first failure"
        );
        assert!(
            recorder.calls.is_empty(),
            "first-pass block performs NO sleep"
        );

        let mut second_fails = ScriptedObserver {
            fail_on: Some(1),
            ..ScriptedObserver::stable_pair(single_pass_net_a())
        };
        let mut recorder = SleepRecorder::default();
        let decision = evaluate_with_timed_settle_dwell(
            NET_A,
            ADA_1,
            TEST_DWELL,
            &mut || second_fails.observe(),
            &mut |duration| recorder.record(duration),
        )
        .expect("validated dwell");
        assert!(matches!(
            decision,
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::ObserverFailed(_))
        ));
        assert_eq!(second_fails.calls.len(), 2);
        assert_eq!(
            recorder.calls,
            vec![TEST_DWELL],
            "second-pass block still slept exactly once"
        );
    }

    #[test]
    fn unstable_live_pair_blocks_through_evaluation_after_one_sleep() {
        let first = single_pass_net_a();
        let second = observations(vec![entry(NET_A, true, &[ADA_1, ADA_2])]);
        let mut observer = ScriptedObserver {
            fail_on: None,
            passes: vec![first, second],
            calls: Vec::new(),
        };
        let mut recorder = SleepRecorder::default();
        let decision = evaluate_with_timed_settle_dwell(
            NET_A,
            ADA_1,
            TEST_DWELL,
            &mut || observer.observe(),
            &mut |duration| recorder.record(duration),
        )
        .expect("validated dwell");
        assert_eq!(
            decision,
            DjLinkTrustDecision::Block(DjLinkTrustBlockReason::PassesDisagree)
        );
        assert_eq!(recorder.calls, vec![TEST_DWELL]);
    }

    #[test]
    fn settle_dwell_rejects_zero_and_excessive_before_touching_the_observer() {
        for (bad, expected) in [
            (Duration::ZERO, DjLinkSettleDwellError::ZeroDuration),
            (
                MAX_DJ_LINK_SETTLE_DWELL + Duration::from_nanos(1),
                DjLinkSettleDwellError::ExcessiveDuration {
                    requested: MAX_DJ_LINK_SETTLE_DWELL + Duration::from_nanos(1),
                },
            ),
            (
                Duration::from_secs(u64::MAX),
                DjLinkSettleDwellError::ExcessiveDuration {
                    requested: Duration::from_secs(u64::MAX),
                },
            ),
        ] {
            let mut observer = ScriptedObserver::stable_pair(single_pass_net_a());
            let mut recorder = SleepRecorder::default();
            let outcome = evaluate_with_timed_settle_dwell(
                NET_A,
                ADA_1,
                bad,
                &mut || observer.observe(),
                &mut |duration| recorder.record(duration),
            );
            assert_eq!(outcome, Err(expected), "dwell {bad:?}");
            assert!(
                observer.calls.is_empty(),
                "invalid dwell must not trigger any observation"
            );
            assert!(recorder.calls.is_empty(), "invalid dwell must never sleep");
        }
    }

    #[test]
    fn settle_dwell_accepts_the_inclusive_ceiling_without_real_sleep_cost_here() {
        let mut observer = ScriptedObserver::stable_pair(single_pass_net_a());
        let mut recorder = SleepRecorder::default();
        let decision = evaluate_with_timed_settle_dwell(
            NET_A,
            ADA_1,
            MAX_DJ_LINK_SETTLE_DWELL,
            &mut || observer.observe(),
            &mut |duration| recorder.record(duration),
        )
        .expect("the documented ceiling itself must be accepted");
        assert_eq!(decision, DjLinkTrustDecision::Allow);
        assert_eq!(recorder.calls, vec![MAX_DJ_LINK_SETTLE_DWELL]);
    }

    #[test]
    fn allow_path_performs_one_real_wall_clock_sleep_of_at_least_the_requested_duration() {
        let mut observer = ScriptedObserver::stable_pair(single_pass_net_a());
        // Lower bound only: a minimum never flakes on slow machines, and no
        // tight upper bound is asserted.
        let started = std::time::Instant::now();
        let decision = evaluate_with_timed_settle_dwell(
            NET_A,
            ADA_1,
            TEST_DWELL,
            &mut || observer.observe(),
            &mut real_thread_sleep,
        )
        .expect("validated dwell");
        let elapsed = started.elapsed();
        assert_eq!(decision, DjLinkTrustDecision::Allow);
        assert_eq!(observer.calls.len(), 2);
        assert!(
            elapsed >= TEST_DWELL,
            "production sleep binding must take at least the requested \
             conservative dwell, got {elapsed:?}"
        );
    }
}
