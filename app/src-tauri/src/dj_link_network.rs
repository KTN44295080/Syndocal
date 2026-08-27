//! Windows NLM live-network trust primitive for DJ-Link auto-start.
//!
//! Security contract (authoritative review decision):
//!
//! - Trust identity is EXACTLY the persisted `(network_guid, adapter_guid)`
//!   pair compared against the live NLM observation. Network class, name,
//!   category, domain type, connectivity state, registry profiles, IP/subnet/
//!   gateway data, and [`crate::dj_link_machine::DjLinkMachineSettingsV2::
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
use std::net::Ipv4Addr;
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
// Wired IPv4 candidate bridge: canonical binding keys (pure)
// ---------------------------------------------------------------------------

/// The COMPLETE persisted trust tuple for a DJ-Link IPv4 binding: the NLM
/// network GUID, the adapter GUID joined from the live NLM observation, and
/// one currently bindable wired IPv4 address. Every field participates in
/// equality; a partial match is never a match. Alias text and interface
/// indexes are deliberately absent because they are diagnostics only.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DjLinkIpv4BindingKey {
    network_guid: String,
    adapter_guid: String,
    bind_ipv4: Ipv4Addr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkIpv4BindingKeyError {
    NetworkGuidInvalid,
    AdapterGuidInvalid,
}

impl DjLinkIpv4BindingKey {
    /// Validates both GUIDs as canonical and non-NIL before a key may exist;
    /// malformed identity fails closed instead of becoming comparable data.
    pub fn new(
        network_guid: String,
        adapter_guid: String,
        bind_ipv4: Ipv4Addr,
    ) -> Result<Self, DjLinkIpv4BindingKeyError> {
        if !is_canonical_nonnil_uuid(&network_guid) {
            return Err(DjLinkIpv4BindingKeyError::NetworkGuidInvalid);
        }
        if !is_canonical_nonnil_uuid(&adapter_guid) {
            return Err(DjLinkIpv4BindingKeyError::AdapterGuidInvalid);
        }
        Ok(Self {
            network_guid,
            adapter_guid,
            bind_ipv4,
        })
    }

    pub fn network_guid(&self) -> &str {
        &self.network_guid
    }

    pub fn adapter_guid(&self) -> &str {
        &self.adapter_guid
    }

    pub fn bind_ipv4(&self) -> Ipv4Addr {
        self.bind_ipv4
    }
}

// ---------------------------------------------------------------------------
// Header-mirroring constants for the wired gate (kept numeric so the pure
// policy below stays platform independent; the Windows side pins them
// against the generated `windows` bindings in a cfg(windows) drift test).
// ---------------------------------------------------------------------------

/// `IF_TYPE_ETHERNET_CSMACD` (ipifcons.h): wired Ethernet CSMA/CD.
pub const DJ_LINK_IF_TYPE_ETHERNET_CSMACD: u32 = 6;
/// `IF_TYPE_SOFTWARE_LOOPBACK` (ipifcons.h): loopback is never wired.
pub const DJ_LINK_IF_TYPE_SOFTWARE_LOOPBACK: u32 = 24;
/// `TUNNEL_TYPE_NONE` (ifdef.h): no tunnel encapsulation.
pub const DJ_LINK_TUNNEL_TYPE_NONE: i32 = 0;
/// `NdisMedium802_3` (netiodef.h): 802.3 media type in MIB_IF_ROW2.
pub const DJ_LINK_NDIS_MEDIUM_802_3: i32 = 0;
/// `NET_IF_CONNECTION_DEDICATED` (ifdef.h): dedicated (non-demand) link.
pub const DJ_LINK_NET_IF_CONNECTION_DEDICATED: i32 = 1;
/// `NET_IF_ADMIN_STATUS_UP` (ifdef.h): administratively enabled.
pub const DJ_LINK_NET_IF_ADMIN_STATUS_UP: i32 = 1;
/// `IfOperStatusUp` (ifdef.h): operationally up.
pub const DJ_LINK_IF_OPER_STATUS_UP: i32 = 1;
/// `NET_IF_MEDIA_CONNECT_STATE_CONNECTED` (ifdef.h): cable/media connected.
pub const DJ_LINK_MEDIA_CONNECT_STATE_CONNECTED: i32 = 1;
/// `IpDadStatePreferred` (nldef.h): duplicate-address detection passed.
pub const DJ_LINK_NL_DAD_STATE_PREFERRED: i32 = 4;
/// Bit 0 of `MIB_IF_ROW2.InterfaceAndOperStatusFlags` (`BOOLEAN
/// HardwareInterface : 1`; shared/netioapi.h bit order: HardwareInterface,
/// FilterInterface, ConnectorPresent, NotAuthenticated, NotMediaConnected,
/// Paused, LowPower, EndPointInterface).
pub const DJ_LINK_MIB_IF_ROW2_HARDWARE_INTERFACE_MASK: u8 = 0x01;

/// True when an IPv4 address may host a DJ-Link binding. Preferred-only
/// filtering happens separately; this rejects structurally unusable ranges:
/// unspecified (this-host), 127/8 loopback, 169.254/16 link-local, multicast,
/// and broadcast.
fn is_usable_bind_ipv4(address: Ipv4Addr) -> bool {
    !(address.is_unspecified()
        || address.is_loopback()
        || address.is_broadcast()
        || address.is_multicast()
        || address.is_link_local())
}

// ---------------------------------------------------------------------------
// Raw OS adapter row snapshots (pure seam input)
// ---------------------------------------------------------------------------

/// One raw unicast-address row as observed by GetAdaptersAddresses, before
/// any preference or range-quality decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DjLinkRawIpv4Address {
    address: Ipv4Addr,
    dad_preferred: bool,
}

impl DjLinkRawIpv4Address {
    pub fn new(address: Ipv4Addr, dad_preferred: bool) -> Self {
        Self {
            address,
            dad_preferred,
        }
    }
}

/// Cross-source identity observations for one GAA adapter row, before any
/// agreement decision. A snapshot exists only when both GUIDs are canonical,
/// non-NIL, and equal, and all three independent interface-index
/// observations agree exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DjLinkRowIdentityObservations {
    /// Canonical GUID produced by ConvertInterfaceLuidToGuid from the GAA
    /// row's LUID (never parsed from any string form).
    adapter_guid: String,
    /// Canonical GUID from GetIfEntry2.InterfaceGuid.
    ifentry_guid: String,
    /// GAA IfIndex observation (header union arm).
    gaa_ifindex: u32,
    /// ConvertInterfaceLuidToIndex observation.
    luid_ifindex: u32,
    /// GetIfEntry2.InterfaceIndex observation.
    ifentry_ifindex: u32,
}

/// Raw numeric link/media/status inputs for one adapter row, kept
/// unclassified so the wired-gate policy stays in the testable pure layer.
/// Values mirror ipifcons.h / ifdef.h / netiodef.h constants pinned by the
/// cfg(windows) drift tests; classification never consults name text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DjLinkRawGateInputs {
    if_type: u32,
    tunnel_type_raw: i32,
    media_type_raw: i32,
    connection_type_raw: i32,
    admin_status_raw: i32,
    oper_status_raw: i32,
    media_connect_raw: i32,
    hardware_interface: bool,
}

/// Non-trust text carried for diagnostics/display only. Neither field ever
/// joins a binding key or any decision; tests prove reconciliation output
/// is invariant to both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DjLinkRowDiagnostics {
    /// The GAA NetworkGuid carried ONLY as an ignored diagnostic. It is
    /// NEVER the NLM network identity and never joins a binding.
    gaa_network_guid: Option<String>,
    /// Display-only alias text; never consulted by any decision.
    adapter_alias: Option<String>,
}

/// One raw OS adapter row snapshot with ALL cross-source identity fields
/// preserved so the pure layer enforces their agreement itself. The sixteen
/// flat observations are grouped into typed contexts instead of loose
/// constructor scalars:
///
/// - [`DjLinkRowIdentityObservations`]: the LUID-derived adapter GUID and
///   GetIfEntry2 interface GUID plus the three index observations (GAA
///   IfIndex, ConvertInterfaceLuidToIndex, GetIfEntry2.InterfaceIndex),
///   which must all agree,
/// - [`DjLinkRawGateInputs`]: enum-shaped raw/numeric gate inputs so
///   classification happens in the pure policy, not in the Windows adapter,
/// - [`DjLinkRowDiagnostics`]: the ignored GAA NetworkGuid diagnostic and
///   display-only alias text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DjLinkRawAdapterSnapshot {
    identity: DjLinkRowIdentityObservations,
    gate_inputs: DjLinkRawGateInputs,
    diagnostics: DjLinkRowDiagnostics,
    ipv4_addresses: Vec<DjLinkRawIpv4Address>,
}

impl DjLinkRawAdapterSnapshot {
    /// Fails closed unless the adapter GUID pair agrees exactly and all three
    /// interface-index observations agree. Any disagreement is corrupted or
    /// hostile enumeration and aborts the whole discovery pass.
    pub fn try_from_parts(
        identity: DjLinkRowIdentityObservations,
        gate_inputs: DjLinkRawGateInputs,
        diagnostics: DjLinkRowDiagnostics,
        ipv4_addresses: Vec<DjLinkRawIpv4Address>,
    ) -> Result<Self, DjLinkIpv4StructuralAnomaly> {
        if !is_canonical_nonnil_uuid(&identity.adapter_guid)
            || !is_canonical_nonnil_uuid(&identity.ifentry_guid)
            || identity.adapter_guid != identity.ifentry_guid
            || identity.gaa_ifindex != identity.luid_ifindex
            || identity.gaa_ifindex != identity.ifentry_ifindex
        {
            return Err(DjLinkIpv4StructuralAnomaly::RowIdentityMismatch);
        }
        Ok(Self {
            identity,
            gate_inputs,
            diagnostics,
            ipv4_addresses,
        })
    }

    /// The agreed cross-source interface index; current diagnostic only,
    /// never persisted trust authority.
    pub fn interface_index(&self) -> u32 {
        self.identity.gaa_ifindex
    }

    pub fn adapter_guid(&self) -> &str {
        &self.identity.adapter_guid
    }
}

// ---------------------------------------------------------------------------
// Blocked reasons, structural anomalies, discovery failures
// ---------------------------------------------------------------------------

/// Why one adapter row yielded NO eligible candidate. Row-level reasons are
/// reported per blocked row; structural anomalies abort the entire pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkIpv4BlockedReason {
    /// The adapter belongs to zero connected NLM networks.
    ZeroNlmMembership,
    /// The adapter appears under more than one connected NLM network:
    /// ambiguous topology fails closed instead of picking one.
    MultipleNlmMembership,
    /// InterfaceAndOperStatusFlags.HardwareInterface is false.
    NonHardwareInterface,
    /// IF_TYPE_SOFTWARE_LOOPBACK.
    LoopbackInterface,
    /// TUNNEL_TYPE other than NONE (classified by structure, never by name).
    TunnelInterface,
    /// Not wired 802.3 Ethernet: IF_TYPE other than ETHERNET_CSMACD or
    /// MIB_IF_ROW2.MediaType other than NdisMedium802_3.
    UnsupportedPhysicalMedium,
    /// NET_IF_CONNECTION_TYPE other than DEDICATED.
    NonDedicatedConnection,
    /// Administratively down.
    AdminDown,
    /// Operationally down.
    OperDown,
    /// Media disconnected (cable out).
    MediaDisconnected,
    /// No IPv4 unicast rows at all on this adapter.
    Ipv6Only,
    /// IPv4 rows exist but none reached IpDadStatePreferred.
    OnlyNonPreferredIpv4,
    /// Preferred IPv4 rows existed but every address was in a rejected range
    /// (loopback/link-local/multicast/broadcast/unspecified).
    NoUsableIpv4,
}

/// Whole-pass abort reasons. When one fires, discovery returns Err and NO
/// partial eligible list exists at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkIpv4StructuralAnomaly {
    /// Cross-source identity disagreement inside one row (GUID pair or the
    /// three-way index agreement).
    RowIdentityMismatch,
    /// Two GAA rows claimed the same adapter GUID.
    DuplicateAdapterRow,
    /// Two rows claimed the same agreed interface index.
    DuplicateInterfaceIndex,
    /// The same IPv4 address appeared more than once across all rows.
    DuplicateIpv4Address,
}

/// Bounded GetAdaptersAddresses allocation/walk failure modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkGaaWalkError {
    /// The buffer could not be grown to fit within the bounded resize budget.
    ResizeAttemptsExhausted { attempts: usize },
    /// The required size exceeded the hard byte cap.
    BufferBoundExceeded { needed: usize },
    /// A Next offset pointed outside the validated buffer.
    PointerOutOfRange { offset: u64, buffer_len: u64 },
    /// A Next offset violated the row alignment contract.
    MisalignedPointer { offset: u64, alignment: u64 },
    /// Following Next revisited an already-visited row.
    NextCycleDetected,
    /// More adapter rows than the bounded cap allows.
    AdapterLimitExceeded,
    /// More unicast addresses on one adapter than the bounded cap allows.
    AddressLimitExceeded,
    /// An iphlpapi call returned a Win32 error code.
    HostFailed { code: u32 },
    /// The host violated sizing/stream discipline beyond what bounds explain
    /// (e.g. overflow claimed without a larger required size).
    ProtocolViolated,
}

/// Failure of one full discovery pass on the observer worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkIpv4DiscoveryFailure {
    Structural(DjLinkIpv4StructuralAnomaly),
    Walk(DjLinkGaaWalkError),
    Observer(DjLinkObserverFailure),
}

// ---------------------------------------------------------------------------
// Candidates, reports, selection and revalidation (pure)
// ---------------------------------------------------------------------------

/// One eligible wired IPv4 binding candidate. `interface_index` and
/// `adapter_alias` are current diagnostics / display text respectively;
/// neither ever becomes trust authority and neither affects the identity of
/// the underlying binding key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DjLinkIpv4Candidate {
    binding: DjLinkIpv4BindingKey,
    interface_index: u32,
    adapter_alias: Option<String>,
}

impl DjLinkIpv4Candidate {
    fn new(
        binding: DjLinkIpv4BindingKey,
        interface_index: u32,
        adapter_alias: Option<String>,
    ) -> Self {
        Self {
            binding,
            interface_index,
            adapter_alias,
        }
    }

    pub fn binding(&self) -> &DjLinkIpv4BindingKey {
        &self.binding
    }

    /// Current diagnostic only; never persisted trust authority.
    #[cfg(test)]
    pub fn interface_index(&self) -> u32 {
        self.interface_index
    }

    /// Display only; never persisted trust authority.
    pub fn adapter_alias(&self) -> Option<&str> {
        self.adapter_alias.as_deref()
    }
}

/// One blocked adapter row with degraded diagnostics. Attribution to a
/// blocked adapter is by adapter GUID alone: a blocked row never produced a
/// complete trusted tuple, so its reason is the strongest claim available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DjLinkIpv4BlockedRow {
    adapter_guid: String,
    interface_index: u32,
    adapter_alias: Option<String>,
    reason: DjLinkIpv4BlockedReason,
}

impl DjLinkIpv4BlockedRow {
    #[cfg(test)]
    pub fn adapter_guid(&self) -> &str {
        &self.adapter_guid
    }

    #[cfg(test)]
    pub fn reason(&self) -> DjLinkIpv4BlockedReason {
        self.reason
    }
}

/// Complete outcome of one discovery pass: eligible candidates plus typed
/// per-row blocks. Sorted deterministically so two passes over unchanged
/// topology compare equal regardless of OS enumeration order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DjLinkIpv4CandidateReport {
    eligible: Vec<DjLinkIpv4Candidate>,
    blocked: Vec<DjLinkIpv4BlockedRow>,
}

impl DjLinkIpv4CandidateReport {
    pub fn eligible(&self) -> &[DjLinkIpv4Candidate] {
        &self.eligible
    }

    #[cfg(test)]
    pub fn blocked(&self) -> &[DjLinkIpv4BlockedRow] {
        &self.blocked
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkIpv4SelectionError {
    /// The report contains no eligible candidates at all.
    EligibleSetEmpty,
    /// Eligible candidates exist but none equals the FULL requested tuple.
    ExactTupleAbsent,
    /// The full requested tuple exists more than once: ambiguity blocks.
    ExactTupleAmbiguous { matches: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DjLinkIpv4BindingError {
    Structural(DjLinkIpv4StructuralAnomaly),
    Walk(DjLinkGaaWalkError),
    Observer(DjLinkObserverFailure),
    /// The exact tuple is not present among eligible candidates and no
    /// blocked row carries its adapter either.
    TupleAbsent,
    /// The tuple's adapter is currently blocked for the carried reason (e.g.
    /// the cable was pulled after trust was established).
    TupleBlocked(DjLinkIpv4BlockedReason),
    /// Multiple eligible copies of the exact tuple exist.
    ExactTupleAmbiguous {
        matches: usize,
    },
}

/// Selects EXACTLY ONE eligible candidate equal to the requested complete
/// tuple `(network_guid, adapter_guid, bind_ipv4)`. Never selects "the first
/// plausible" row: zero matches and multiple matches are distinct typed
/// errors, and matching compares every field of the binding key.
pub fn require_unique_binding<'a>(
    report: &'a DjLinkIpv4CandidateReport,
    requested: &DjLinkIpv4BindingKey,
) -> Result<&'a DjLinkIpv4Candidate, DjLinkIpv4SelectionError> {
    let matches = report
        .eligible
        .iter()
        .filter(|candidate| candidate.binding == *requested)
        .count();
    match matches {
        0 => {
            if report.eligible.is_empty() {
                Err(DjLinkIpv4SelectionError::EligibleSetEmpty)
            } else {
                Err(DjLinkIpv4SelectionError::ExactTupleAbsent)
            }
        }
        1 => Ok(report
            .eligible
            .iter()
            .find(|candidate| candidate.binding == *requested)
            .expect("exactly one full-tuple match established above")),
        n => Err(DjLinkIpv4SelectionError::ExactTupleAmbiguous { matches: n }),
    }
}

/// Binding revalidation against one discovery report. Succeeds only for the
/// unique eligible full tuple. A tuple whose adapter appears in the blocked
/// list fails closed with that row's typed reason instead of degrading to a
/// generic miss.
pub fn validate_binding_against_report(
    report: &DjLinkIpv4CandidateReport,
    requested: &DjLinkIpv4BindingKey,
) -> Result<DjLinkIpv4Candidate, DjLinkIpv4BindingError> {
    match require_unique_binding(report, requested) {
        Ok(candidate) => Ok(candidate.clone()),
        Err(DjLinkIpv4SelectionError::ExactTupleAmbiguous { matches }) => {
            Err(DjLinkIpv4BindingError::ExactTupleAmbiguous { matches })
        }
        Err(DjLinkIpv4SelectionError::EligibleSetEmpty)
        | Err(DjLinkIpv4SelectionError::ExactTupleAbsent) => {
            match report
                .blocked
                .iter()
                .find(|row| row.adapter_guid == requested.adapter_guid())
            {
                Some(row) => Err(DjLinkIpv4BindingError::TupleBlocked(row.reason)),
                None => Err(DjLinkIpv4BindingError::TupleAbsent),
            }
        }
    }
}

/// Joins connected NLM observations with raw OS adapter rows into the typed
/// candidate report.
///
/// Per-row gate order (first failing reason wins): NLM membership count
/// (exactly one CONNECTED network carrying the adapter GUID), hardware flag,
/// loopback type, tunnel type, wired 802.3 medium (IF_TYPE + MediaType),
/// dedicated connection, admin up, operational up, media connected, then the
/// IPv4 address set (any rows at all → preferred-only → usable ranges).
///
/// Structural anomalies (identity mismatch inside any snapshot, duplicate
/// adapter GUIDs / interface indexes / IPv4 addresses anywhere) abort the
/// whole pass with `Err` — no partial eligible list is ever produced. WSL or
/// tunnel adapters are classified ONLY by these structured fields, never by
/// name text.
pub fn reconcile_ipv4_candidates(
    observations: &DjLinkNetworkObservations,
    rows: &[DjLinkRawAdapterSnapshot],
) -> Result<DjLinkIpv4CandidateReport, DjLinkIpv4StructuralAnomaly> {
    // Structural duplicate sweep across ALL rows before any eligibility work:
    // duplicated identity poisons enumeration trust, so it must not leave
    // even a partial report behind.
    let mut seen_adapters = BTreeSet::<&str>::new();
    let mut seen_indexes = BTreeSet::<u32>::new();
    let mut seen_addresses = BTreeSet::<Ipv4Addr>::new();
    for row in rows {
        if !seen_adapters.insert(row.adapter_guid()) {
            return Err(DjLinkIpv4StructuralAnomaly::DuplicateAdapterRow);
        }
        if !seen_indexes.insert(row.interface_index()) {
            return Err(DjLinkIpv4StructuralAnomaly::DuplicateInterfaceIndex);
        }
        for raw in &row.ipv4_addresses {
            if !seen_addresses.insert(raw.address) {
                return Err(DjLinkIpv4StructuralAnomaly::DuplicateIpv4Address);
            }
        }
    }
    let mut report = DjLinkIpv4CandidateReport::default();
    for row in rows {
        // Each adapter must belong to EXACTLY ONE connected NLM network.
        // Membership counts connected entries only, and duplicate entries of
        // one network id count twice, mirroring the trust layer's explicit
        // ambiguity handling.
        let memberships: Vec<&str> = observations
            .entries()
            .iter()
            .filter(|entry| {
                entry.connected() && entry.adapter_guids().contains(&row.identity.adapter_guid)
            })
            .map(|entry| entry.network_guid())
            .collect();
        let reason = match memberships.len() {
            0 => Some(DjLinkIpv4BlockedReason::ZeroNlmMembership),
            2.. => Some(DjLinkIpv4BlockedReason::MultipleNlmMembership),
            1 => classify_wired_row_block(row),
        };
        match reason {
            Some(reason) => report.blocked.push(DjLinkIpv4BlockedRow {
                adapter_guid: row.identity.adapter_guid.clone(),
                interface_index: row.interface_index(),
                adapter_alias: row.diagnostics.adapter_alias.clone(),
                reason,
            }),
            None => {
                let network_guid = memberships[0].to_string();
                for raw in row
                    .ipv4_addresses
                    .iter()
                    .filter(|raw| raw.dad_preferred && is_usable_bind_ipv4(raw.address))
                {
                    let binding = DjLinkIpv4BindingKey::new(
                        network_guid.clone(),
                        row.identity.adapter_guid.clone(),
                        raw.address,
                    )
                    .expect("canonical inputs validated above");
                    report.eligible.push(DjLinkIpv4Candidate::new(
                        binding,
                        row.interface_index(),
                        row.diagnostics.adapter_alias.clone(),
                    ));
                }
            }
        }
    }
    // Deterministic canonical order that deliberately ignores alias text and
    // index values (display fields must not influence ordering).
    report.eligible.sort_by(|left, right| {
        (
            left.binding.network_guid.as_str(),
            left.binding.adapter_guid.as_str(),
            left.binding.bind_ipv4.octets(),
        )
            .cmp(&(
                right.binding.network_guid.as_str(),
                right.binding.adapter_guid.as_str(),
                right.binding.bind_ipv4.octets(),
            ))
    });
    report.blocked.sort_by(|left, right| {
        (left.interface_index, left.adapter_guid.as_str())
            .cmp(&(right.interface_index, right.adapter_guid.as_str()))
    });
    Ok(report)
}

/// Fixed-order wired gate for one membership-validated row. Returns the
/// first failing block reason, or None when the row may contribute usable
/// preferred IPv4 candidates.
fn classify_wired_row_block(row: &DjLinkRawAdapterSnapshot) -> Option<DjLinkIpv4BlockedReason> {
    let gate = &row.gate_inputs;
    if !gate.hardware_interface {
        return Some(DjLinkIpv4BlockedReason::NonHardwareInterface);
    }
    if gate.if_type == DJ_LINK_IF_TYPE_SOFTWARE_LOOPBACK {
        return Some(DjLinkIpv4BlockedReason::LoopbackInterface);
    }
    if gate.tunnel_type_raw != DJ_LINK_TUNNEL_TYPE_NONE {
        return Some(DjLinkIpv4BlockedReason::TunnelInterface);
    }
    if gate.if_type != DJ_LINK_IF_TYPE_ETHERNET_CSMACD
        || gate.media_type_raw != DJ_LINK_NDIS_MEDIUM_802_3
    {
        return Some(DjLinkIpv4BlockedReason::UnsupportedPhysicalMedium);
    }
    if gate.connection_type_raw != DJ_LINK_NET_IF_CONNECTION_DEDICATED {
        return Some(DjLinkIpv4BlockedReason::NonDedicatedConnection);
    }
    if gate.admin_status_raw != DJ_LINK_NET_IF_ADMIN_STATUS_UP {
        return Some(DjLinkIpv4BlockedReason::AdminDown);
    }
    if gate.oper_status_raw != DJ_LINK_IF_OPER_STATUS_UP {
        return Some(DjLinkIpv4BlockedReason::OperDown);
    }
    if gate.media_connect_raw != DJ_LINK_MEDIA_CONNECT_STATE_CONNECTED {
        return Some(DjLinkIpv4BlockedReason::MediaDisconnected);
    }
    if row.ipv4_addresses.is_empty() {
        return Some(DjLinkIpv4BlockedReason::Ipv6Only);
    }
    let preferred = row
        .ipv4_addresses
        .iter()
        .filter(|raw| raw.dad_preferred)
        .count();
    if preferred == 0 {
        return Some(DjLinkIpv4BlockedReason::OnlyNonPreferredIpv4);
    }
    let usable = row
        .ipv4_addresses
        .iter()
        .filter(|raw| raw.dad_preferred && is_usable_bind_ipv4(raw.address))
        .count();
    if usable == 0 {
        return Some(DjLinkIpv4BlockedReason::NoUsableIpv4);
    }
    None
}

// ---------------------------------------------------------------------------
// Bounded GetAdaptersAddresses allocation/walk seams (pure)
// ---------------------------------------------------------------------------

/// Initial GetAdaptersAddresses buffer size (the MSDS-recommended 15 KiB
/// working set).
pub const INITIAL_GAA_BUFFER_BYTES: usize = 15 * 1024;
/// Hard byte ceiling for the GAA buffer; a host demanding more is treated as
/// hostile and fails closed.
pub const MAX_GAA_BUFFER_BYTES: usize = 1 << 20;
/// Maximum number of buffer RESIZES after the initial attempt: at most four
/// total sizing calls may happen per discovery pass.
pub const MAX_GAA_RESIZE_ATTEMPTS: usize = 3;
/// Hard cap on adapter rows per pass (DoS guard, far above real NIC counts).
pub const MAX_GAA_ADAPTERS: usize = 256;
/// Hard cap on unicast address rows per adapter.
pub const MAX_GAA_ADDRESSES_PER_ADAPTER: usize = 64;

/// Outcome of one simulated/real GetAdaptersAddresses sizing attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaaSizeAttempt {
    /// ERROR_SUCCESS: the caller's buffer held the complete table.
    Fit,
    /// ERROR_BUFFER_OVERFLOW: retry with at least `needed` bytes.
    Overflow { needed: usize },
}

/// Drives the bounded resize loop for GetAdaptersAddresses. `attempt`
/// performs EXACTLY one raw call with the given capacity in bytes. Policy:
/// one initial attempt plus at most [`MAX_GAA_RESIZE_ATTEMPTS`] resizes; each
/// overflow must demand strictly MORE space than the current capacity (an
/// overflow that does not grow is a protocol violation); any needed size over
/// [`MAX_GAA_BUFFER_BYTES`] fails immediately without further calls. On
/// success returns the capacity of the attempt that fit.
pub fn drive_gaa_allocation(
    mut attempt: impl FnMut(usize) -> Result<GaaSizeAttempt, DjLinkGaaWalkError>,
) -> Result<usize, DjLinkGaaWalkError> {
    if INITIAL_GAA_BUFFER_BYTES == 0 || MAX_GAA_BUFFER_BYTES < INITIAL_GAA_BUFFER_BYTES {
        return Err(DjLinkGaaWalkError::ProtocolViolated);
    }
    let mut capacity = INITIAL_GAA_BUFFER_BYTES;
    for resizes_used in 0..=MAX_GAA_RESIZE_ATTEMPTS {
        match attempt(capacity)? {
            GaaSizeAttempt::Fit => return Ok(capacity),
            GaaSizeAttempt::Overflow { needed } => {
                if needed <= capacity {
                    return Err(DjLinkGaaWalkError::ProtocolViolated);
                }
                if needed > MAX_GAA_BUFFER_BYTES {
                    return Err(DjLinkGaaWalkError::BufferBoundExceeded { needed });
                }
                if resizes_used == MAX_GAA_RESIZE_ATTEMPTS {
                    return Err(DjLinkGaaWalkError::ResizeAttemptsExhausted {
                        attempts: MAX_GAA_RESIZE_ATTEMPTS + 1,
                    });
                }
                // Grow by doubling, but always satisfy the demanded minimum.
                let doubled = capacity.saturating_mul(2);
                capacity = doubled.max(needed).min(MAX_GAA_BUFFER_BYTES);
            }
        }
    }
    unreachable!("resize budget loop returns inside the match arms above")
}

/// Alignment and length contract for one linked OS row list. `min_row_len`
/// must be the FULL row size so every validated offset guarantees an entire
/// typed row readable inside the buffer.
#[derive(Debug, Clone, Copy)]
pub struct GaaWalkLimits {
    pub row_alignment: u64,
    pub min_row_len: u64,
    pub max_rows: usize,
}

/// Validates one Next offset against range, alignment, and full-row length.
fn validate_gaa_step(
    offset: u64,
    buffer_len: u64,
    limits: &GaaWalkLimits,
) -> Result<(), DjLinkGaaWalkError> {
    if limits.row_alignment == 0
        || !limits.row_alignment.is_power_of_two()
        || limits.min_row_len == 0
    {
        return Err(DjLinkGaaWalkError::ProtocolViolated);
    }
    if offset % limits.row_alignment != 0 {
        return Err(DjLinkGaaWalkError::MisalignedPointer {
            offset,
            alignment: limits.row_alignment,
        });
    }
    let end = offset
        .checked_add(limits.min_row_len)
        .ok_or(DjLinkGaaWalkError::PointerOutOfRange { offset, buffer_len })?;
    if offset >= buffer_len || end > buffer_len {
        return Err(DjLinkGaaWalkError::PointerOutOfRange { offset, buffer_len });
    }
    Ok(())
}

/// Walks one OS `Next`-linked row list as validated byte offsets relative to
/// the allocation base. `read_next_offset(offset)` performs exactly one raw
/// field read of the node's Next pointer (None encodes the terminal null).
/// Every yielded offset is checked for range, alignment, and full-row length;
/// revisiting any offset aborts with [`DjLinkGaaWalkError::NextCycleDetected`];
/// exceeding `limits.max_rows` fails with `limit_exceeded`. Any anomaly means
/// the returned list does not exist at all.
pub fn walk_gaa_linked_list(
    mut read_next_offset: impl FnMut(u64) -> Option<u64>,
    first_offset: u64,
    buffer_len: u64,
    limits: &GaaWalkLimits,
    limit_exceeded: fn() -> DjLinkGaaWalkError,
) -> Result<Vec<u64>, DjLinkGaaWalkError> {
    if limits.max_rows == 0 {
        return Err(limit_exceeded());
    }
    let mut offsets = Vec::new();
    let mut visited = BTreeSet::<u64>::new();
    let mut cursor = Some(first_offset);
    while let Some(offset) = cursor {
        validate_gaa_step(offset, buffer_len, limits)?;
        if !visited.insert(offset) {
            return Err(DjLinkGaaWalkError::NextCycleDetected);
        }
        offsets.push(offset);
        if offsets.len() > limits.max_rows {
            return Err(limit_exceeded());
        }
        cursor = read_next_offset(offset);
    }
    Ok(offsets)
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
        drain_com_pages, drive_gaa_allocation, evaluate_with_timed_settle_dwell, format_uuid_parts,
        is_canonical_nonnil_uuid, real_thread_sleep, reconcile_ipv4_candidates,
        validate_binding_against_report, walk_gaa_linked_list, ComEnumerationLimits,
        DjLinkGaaWalkError, DjLinkIpv4BindingError, DjLinkIpv4BindingKey, DjLinkIpv4Candidate,
        DjLinkIpv4CandidateReport, DjLinkIpv4DiscoveryFailure, DjLinkNetworkObservations,
        DjLinkObservedNetwork, DjLinkObserverFailure, DjLinkPageError, DjLinkRawAdapterSnapshot,
        DjLinkRawGateInputs, DjLinkRawIpv4Address, DjLinkRowDiagnostics,
        DjLinkRowIdentityObservations, DjLinkSettleDwellError, DjLinkTrustDecision, GaaSizeAttempt,
        GaaWalkLimits, DJ_LINK_MIB_IF_ROW2_HARDWARE_INTERFACE_MASK, MAX_GAA_ADAPTERS,
        MAX_GAA_ADDRESSES_PER_ADAPTER,
    };
    use std::collections::BTreeSet;
    use std::net::Ipv4Addr;
    use std::sync::mpsc::{self, RecvTimeoutError, Sender};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;
    use windows::core::GUID;
    use windows::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
    use windows::Win32::NetworkManagement::IpHelper::{
        ConvertInterfaceLuidToGuid, ConvertInterfaceLuidToIndex, GetAdaptersAddresses, GetIfEntry2,
        GAA_FLAG_INCLUDE_ALL_INTERFACES, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
        GAA_FLAG_SKIP_FRIENDLY_NAME, GAA_FLAG_SKIP_MULTICAST, GET_ADAPTERS_ADDRESSES_FLAGS,
        IP_ADAPTER_ADDRESSES_LH, IP_ADAPTER_UNICAST_ADDRESS_LH, MIB_IF_ROW2,
    };
    use windows::Win32::NetworkManagement::Ndis::NET_LUID_LH;
    use windows::Win32::Networking::NetworkListManager::{
        IEnumNetworkConnections, IEnumNetworks, INetwork, INetworkConnection, INetworkListManager,
        NetworkListManager, NLM_ENUM_NETWORK_CONNECTED,
    };
    use windows::Win32::Networking::WinSock::{AF_INET, AF_UNSPEC, SOCKADDR, SOCKADDR_IN};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    const INIT_TIMEOUT: Duration = Duration::from_secs(5);
    const OBSERVE_TIMEOUT: Duration = Duration::from_secs(3);
    /// Discovery runs NLM enumeration plus GetAdaptersAddresses and one
    /// GetIfEntry2 per adapter; it needs a wider budget than a bare observe.
    const DISCOVER_TIMEOUT: Duration = Duration::from_secs(10);
    const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

    enum Request {
        Observe(Sender<Result<DjLinkNetworkObservations, DjLinkObserverFailure>>),
        DiscoverIpv4(Sender<Result<DjLinkIpv4CandidateReport, DjLinkIpv4DiscoveryFailure>>),
        ValidateIpv4Binding {
            binding: DjLinkIpv4BindingKey,
            reply: Sender<Result<DjLinkIpv4Candidate, DjLinkIpv4BindingError>>,
        },
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

        /// Runs exactly one discovery pass on the SAME dedicated MTA worker:
        /// connected NLM enumeration plus the bounded GetAdaptersAddresses
        /// walk, joined by the pure reconciliation. No COM pointer ever
        /// crosses the reply channel; only canonical plain data does.
        pub fn discover_ipv4_candidates(
            &self,
        ) -> Result<DjLinkIpv4CandidateReport, DjLinkIpv4DiscoveryFailure> {
            let (reply_tx, reply_rx) = mpsc::channel();
            self.request_tx
                .send(Request::DiscoverIpv4(reply_tx))
                .map_err(|_| {
                    DjLinkIpv4DiscoveryFailure::Observer(
                        DjLinkObserverFailure::ObserverChannelClosed,
                    )
                })?;
            match reply_rx.recv_timeout(DISCOVER_TIMEOUT) {
                Ok(result) => result,
                Err(RecvTimeoutError::Timeout) => Err(DjLinkIpv4DiscoveryFailure::Observer(
                    DjLinkObserverFailure::ObserverTimeout,
                )),
                Err(RecvTimeoutError::Disconnected) => Err(DjLinkIpv4DiscoveryFailure::Observer(
                    DjLinkObserverFailure::ObserverChannelClosed,
                )),
            }
        }

        /// Revalidates one exact binding tuple through a FRESH discovery pass
        /// on the same worker: succeeds only if the complete tuple exists
        /// exactly once among eligible candidates right now. Blocked rows on
        /// the same adapter surface their typed reason; nothing is cached.
        pub fn validate_ipv4_binding(
            &self,
            binding: &DjLinkIpv4BindingKey,
        ) -> Result<DjLinkIpv4Candidate, DjLinkIpv4BindingError> {
            let (reply_tx, reply_rx) = mpsc::channel();
            self.request_tx
                .send(Request::ValidateIpv4Binding {
                    binding: binding.clone(),
                    reply: reply_tx,
                })
                .map_err(|_| {
                    DjLinkIpv4BindingError::Observer(DjLinkObserverFailure::ObserverChannelClosed)
                })?;
            match reply_rx.recv_timeout(DISCOVER_TIMEOUT) {
                Ok(result) => result,
                Err(RecvTimeoutError::Timeout) => Err(DjLinkIpv4BindingError::Observer(
                    DjLinkObserverFailure::ObserverTimeout,
                )),
                Err(RecvTimeoutError::Disconnected) => Err(DjLinkIpv4BindingError::Observer(
                    DjLinkObserverFailure::ObserverChannelClosed,
                )),
            }
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
            // Release the apartment-bound interface before balancing COM.
            // Dropping it after CoUninitialize can terminate the process.
            drop(manager);
            // SAFETY: same thread as above; no receiver remains.
            unsafe { CoUninitialize() };
            return;
        }
        while let Ok(request) = request_rx.recv() {
            match request {
                Request::Observe(reply) => {
                    let _ = reply.send(observe_connected_networks(&manager));
                }
                Request::DiscoverIpv4(reply) => {
                    let _ = reply.send(discover_ipv4_candidates_on_worker(&manager));
                }
                Request::ValidateIpv4Binding { binding, reply } => {
                    let _ = reply.send(
                        discover_ipv4_candidates_on_worker(&manager)
                            .map_err(|failure| match failure {
                                DjLinkIpv4DiscoveryFailure::Structural(anomaly) => {
                                    DjLinkIpv4BindingError::Structural(anomaly)
                                }
                                DjLinkIpv4DiscoveryFailure::Walk(error) => {
                                    DjLinkIpv4BindingError::Walk(error)
                                }
                                DjLinkIpv4DiscoveryFailure::Observer(failure) => {
                                    DjLinkIpv4BindingError::Observer(failure)
                                }
                            })
                            .and_then(|report| validate_binding_against_report(&report, &binding)),
                    );
                }
                Request::Shutdown(ack) => {
                    let _ = ack.send(Ok(()));
                    break;
                }
            }
        }
        // COM interfaces must be released while their apartment is still
        // initialized. The reverse order caused an access violation after an
        // otherwise successful live candidate-discovery result.
        drop(manager);
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

    /// Full IPv4 discovery pass on the MTA thread: connected NLM networks
    /// plus the bounded wired-adapter join. Runs entirely inside the worker;
    /// replies carry plain canonical data only.
    fn discover_ipv4_candidates_on_worker(
        manager: &INetworkListManager,
    ) -> Result<DjLinkIpv4CandidateReport, DjLinkIpv4DiscoveryFailure> {
        let observations =
            observe_connected_networks(manager).map_err(DjLinkIpv4DiscoveryFailure::Observer)?;
        let snapshots = collect_adapter_snapshots()?;
        reconcile_ipv4_candidates(&observations, &snapshots)
            .map_err(DjLinkIpv4DiscoveryFailure::Structural)
    }

    /// Exact GAA request flags required by the trust contract: every
    /// interface including down ones, with everything not needed for the
    /// identity/address join skipped so no string forms are fetched.
    fn gaa_request_flags() -> GET_ADAPTERS_ADDRESSES_FLAGS {
        GAA_FLAG_INCLUDE_ALL_INTERFACES
            | GAA_FLAG_SKIP_ANYCAST
            | GAA_FLAG_SKIP_MULTICAST
            | GAA_FLAG_SKIP_DNS_SERVER
            | GAA_FLAG_SKIP_FRIENDLY_NAME
    }

    fn collect_adapter_snapshots(
    ) -> Result<Vec<DjLinkRawAdapterSnapshot>, DjLinkIpv4DiscoveryFailure> {
        let mut buffer: Vec<u64> = Vec::new();
        let mut table_ptr: *mut IP_ADAPTER_ADDRESSES_LH = std::ptr::null_mut();
        let mut used_bytes: usize = 0;
        drive_gaa_allocation(|capacity| {
            buffer.resize(capacity.div_ceil(core::mem::size_of::<u64>()), 0);
            let capacity_bytes = buffer.len() * core::mem::size_of::<u64>();
            // SAFETY: table points into our own live allocation sized
            // capacity_bytes; bytes/fetched are plain out-parameter locals.
            let table = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
            let mut bytes = capacity_bytes as u32;
            let code = unsafe {
                GetAdaptersAddresses(
                    AF_UNSPEC.0 as u32,
                    gaa_request_flags(),
                    None,
                    Some(table),
                    &mut bytes,
                )
            };
            if code == 0 {
                table_ptr = table;
                used_bytes = bytes as usize;
                Ok(GaaSizeAttempt::Fit)
            } else if code == ERROR_BUFFER_OVERFLOW.0 {
                Ok(GaaSizeAttempt::Overflow {
                    needed: bytes as usize,
                })
            } else {
                Err(DjLinkGaaWalkError::HostFailed { code })
            }
        })
        .map_err(DjLinkIpv4DiscoveryFailure::Walk)?;
        if table_ptr.is_null() || used_bytes == 0 {
            return Ok(Vec::new());
        }
        let capacity_bytes = buffer.len() * core::mem::size_of::<u64>();
        if used_bytes > capacity_bytes {
            return Err(DjLinkIpv4DiscoveryFailure::Walk(
                DjLinkGaaWalkError::ProtocolViolated,
            ));
        }
        let base = buffer.as_ptr() as usize;
        let buffer_len = capacity_bytes as u64;
        let limits = GaaWalkLimits {
            row_alignment: core::mem::align_of::<IP_ADAPTER_ADDRESSES_LH>() as u64,
            min_row_len: core::mem::size_of::<IP_ADAPTER_ADDRESSES_LH>() as u64,
            max_rows: MAX_GAA_ADAPTERS,
        };
        let next_field = core::mem::offset_of!(IP_ADAPTER_ADDRESSES_LH, Next);
        // SAFETY: walk_gaa_linked_list validates EVERY offset for full-row
        // range and 8-byte alignment BEFORE handing it here; the read below
        // therefore stays inside [base, base + buffer_len) at all times.
        let read_next = |offset: u64| -> Option<u64> {
            let next = unsafe {
                core::ptr::read_unaligned(
                    (base + offset as usize + next_field) as *const *mut IP_ADAPTER_ADDRESSES_LH,
                )
            };
            if next.is_null() {
                None
            } else {
                Some((next as usize).wrapping_sub(base) as u64)
            }
        };
        let first_offset = (table_ptr as usize).wrapping_sub(base) as u64;
        let offsets = walk_gaa_linked_list(read_next, first_offset, buffer_len, &limits, || {
            DjLinkGaaWalkError::AdapterLimitExceeded
        })
        .map_err(DjLinkIpv4DiscoveryFailure::Walk)?;
        let mut snapshots = Vec::with_capacity(offsets.len());
        for offset in offsets {
            // SAFETY: same validation argument as above; full typed row lies
            // inside the allocation and is naturally aligned.
            let row = unsafe { &*((base + offset as usize) as *const IP_ADAPTER_ADDRESSES_LH) };
            snapshots.push(extract_adapter_snapshot(row, base, buffer_len)?);
        }
        Ok(snapshots)
    }

    fn extract_adapter_snapshot(
        row: &IP_ADAPTER_ADDRESSES_LH,
        base: usize,
        buffer_len: u64,
    ) -> Result<DjLinkRawAdapterSnapshot, DjLinkIpv4DiscoveryFailure> {
        let host =
            |code: u32| DjLinkIpv4DiscoveryFailure::Walk(DjLinkGaaWalkError::HostFailed { code });
        let protocol = || DjLinkIpv4DiscoveryFailure::Walk(DjLinkGaaWalkError::ProtocolViolated);
        let luid: NET_LUID_LH = row.Luid;
        let mut luid_guid = GUID::zeroed();
        // SAFETY: apartment-free iphlpapi calls on caller-owned out-params.
        let code = unsafe { ConvertInterfaceLuidToGuid(&luid, &mut luid_guid) };
        if code.0 != 0 {
            return Err(host(code.0));
        }
        let adapter_guid = format_windows_guid(&luid_guid);
        if !is_canonical_nonnil_uuid(&adapter_guid) {
            return Err(protocol());
        }
        let mut luid_index = 0_u32;
        let code = unsafe { ConvertInterfaceLuidToIndex(&luid, &mut luid_index) };
        if code.0 != 0 {
            return Err(host(code.0));
        }
        let gaa_network_guid = format_windows_guid(&row.NetworkGuid);
        let mut if_entry = MIB_IF_ROW2::default();
        if_entry.InterfaceLuid = luid;
        // SAFETY: GetIfEntry2 fills the caller-owned POD row keyed by the
        // LUID set above.
        let code = unsafe { GetIfEntry2(&mut if_entry) };
        if code.0 != 0 {
            return Err(host(code.0));
        }
        let ifentry_guid = format_windows_guid(&if_entry.InterfaceGuid);
        let hardware_interface = (if_entry.InterfaceAndOperStatusFlags._bitfield
            & DJ_LINK_MIB_IF_ROW2_HARDWARE_INTERFACE_MASK)
            != 0;
        let ipv4_addresses = collect_raw_ipv4_addresses(row.FirstUnicastAddress, base, buffer_len)?;
        // SAFETY: IfIndex lives inside the header union; the GAA host
        // always populates the anonymous struct arm on success.
        let gaa_ifindex = unsafe { row.Anonymous1.Anonymous.IfIndex };
        DjLinkRawAdapterSnapshot::try_from_parts(
            DjLinkRowIdentityObservations {
                adapter_guid,
                ifentry_guid,
                gaa_ifindex,
                luid_ifindex: luid_index,
                ifentry_ifindex: if_entry.InterfaceIndex,
            },
            DjLinkRawGateInputs {
                if_type: row.IfType,
                tunnel_type_raw: row.TunnelType.0,
                media_type_raw: if_entry.MediaType.0,
                connection_type_raw: if_entry.ConnectionType.0,
                admin_status_raw: if_entry.AdminStatus.0,
                oper_status_raw: if_entry.OperStatus.0,
                media_connect_raw: if_entry.MediaConnectState.0,
                hardware_interface,
            },
            DjLinkRowDiagnostics {
                gaa_network_guid: Some(gaa_network_guid),
                adapter_alias: alias_to_display(&if_entry.Alias),
            },
            ipv4_addresses,
        )
        .map_err(DjLinkIpv4DiscoveryFailure::Structural)
    }

    fn collect_raw_ipv4_addresses(
        first_unicast: *mut IP_ADAPTER_UNICAST_ADDRESS_LH,
        base: usize,
        buffer_len: u64,
    ) -> Result<Vec<DjLinkRawIpv4Address>, DjLinkIpv4DiscoveryFailure> {
        let mut raw_addresses = Vec::new();
        if first_unicast.is_null() {
            return Ok(raw_addresses);
        }
        let limits = GaaWalkLimits {
            row_alignment: core::mem::align_of::<IP_ADAPTER_UNICAST_ADDRESS_LH>() as u64,
            min_row_len: core::mem::size_of::<IP_ADAPTER_UNICAST_ADDRESS_LH>() as u64,
            max_rows: MAX_GAA_ADDRESSES_PER_ADAPTER,
        };
        // Layout pin: lpSockaddr must stay the first SOCKET_ADDRESS member
        // for the binary reads below to be correct.
        const _: () = assert!(
            core::mem::offset_of!(
                windows::Win32::Networking::WinSock::SOCKET_ADDRESS,
                lpSockaddr
            ) == 0
        );
        let next_field = core::mem::offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, Next);
        let address_field = core::mem::offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, Address);
        let dad_field = core::mem::offset_of!(IP_ADAPTER_UNICAST_ADDRESS_LH, DadState);
        // SAFETY: identical contract to the adapter-row walk: every offset
        // handed here was validated for full-node range and alignment.
        let read_next = |offset: u64| -> Option<u64> {
            let next = unsafe {
                core::ptr::read_unaligned(
                    (base + offset as usize + next_field)
                        as *const *mut IP_ADAPTER_UNICAST_ADDRESS_LH,
                )
            };
            if next.is_null() {
                None
            } else {
                Some((next as usize).wrapping_sub(base) as u64)
            }
        };
        let first_offset = (first_unicast as usize).wrapping_sub(base) as u64;
        let offsets = walk_gaa_linked_list(read_next, first_offset, buffer_len, &limits, || {
            DjLinkGaaWalkError::AddressLimitExceeded
        })
        .map_err(DjLinkIpv4DiscoveryFailure::Walk)?;
        for offset in offsets {
            let node_off = offset as usize;
            // SAFETY: validated full-node in-allocation access.
            let sock_addr = unsafe {
                core::ptr::read_unaligned((base + node_off + address_field) as *const *mut SOCKADDR)
            };
            let Some(octets) = read_ipv4_sockaddr(sock_addr, base, buffer_len)
                .map_err(DjLinkIpv4DiscoveryFailure::Walk)?
            else {
                continue;
            };
            let dad_state =
                unsafe { core::ptr::read_unaligned((base + node_off + dad_field) as *const i32) };
            raw_addresses.push(DjLinkRawIpv4Address::new(
                Ipv4Addr::from(octets),
                dad_state == super::DJ_LINK_NL_DAD_STATE_PREFERRED,
            ));
        }
        Ok(raw_addresses)
    }

    /// Reads one bounded IPv4 socket address through the generated Windows
    /// structure rather than hard-coding ABI offsets. `sin_addr` is at byte 4
    /// in `SOCKADDR_IN`; byte 8 begins `sin_zero` padding and previously made
    /// every adapter appear to own `0.0.0.0`, poisoning discovery as a false
    /// duplicate IPv4 address.
    fn read_ipv4_sockaddr(
        sock_addr: *const SOCKADDR,
        base: usize,
        buffer_len: u64,
    ) -> Result<Option<[u8; 4]>, DjLinkGaaWalkError> {
        if sock_addr.is_null() {
            return Err(DjLinkGaaWalkError::ProtocolViolated);
        }
        let offset = (sock_addr as usize)
            .checked_sub(base)
            .ok_or(DjLinkGaaWalkError::ProtocolViolated)? as u64;
        let end = offset
            .checked_add(core::mem::size_of::<SOCKADDR_IN>() as u64)
            .ok_or(DjLinkGaaWalkError::ProtocolViolated)?;
        if end > buffer_len {
            return Err(DjLinkGaaWalkError::ProtocolViolated);
        }
        // SAFETY: the full generated SOCKADDR_IN structure was bounded above;
        // unaligned reads are required because the OS owns the packed buffer.
        let ipv4 = unsafe { core::ptr::read_unaligned(sock_addr.cast::<SOCKADDR_IN>()) };
        if ipv4.sin_family != AF_INET {
            return Ok(None);
        }
        // SAFETY: S_un_b is the byte representation of the initialized IN_ADDR
        // union returned by GetAdaptersAddresses, already copied into `ipv4`.
        let bytes = unsafe { ipv4.sin_addr.S_un.S_un_b };
        Ok(Some([bytes.s_b1, bytes.s_b2, bytes.s_b3, bytes.s_b4]))
    }

    /// Decode-only alias extraction for DISPLAY diagnostics: lossless UTF-16
    /// decoding of the nul-terminated MIB_IF_ROW2.Alias text. Never parsed,
    /// never matched, never persisted as identity.
    fn alias_to_display(alias: &[u16]) -> Option<String> {
        let end = alias
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(alias.len());
        let text = String::from_utf16_lossy(&alias[..end]);
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
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
        use super::super::{
            DJ_LINK_IF_TYPE_ETHERNET_CSMACD, DJ_LINK_IF_TYPE_SOFTWARE_LOOPBACK,
            DJ_LINK_MEDIA_CONNECT_STATE_CONNECTED, DJ_LINK_NDIS_MEDIUM_802_3,
            DJ_LINK_NET_IF_ADMIN_STATUS_UP, DJ_LINK_NET_IF_CONNECTION_DEDICATED,
            DJ_LINK_NL_DAD_STATE_PREFERRED, DJ_LINK_TUNNEL_TYPE_NONE,
        };
        use super::{
            read_ipv4_sockaddr, DjLinkNetworkObservations, DjLinkObserverFailure,
            DjLinkSettleDwellError, DjLinkTrustDecision, NlmTrustObserver,
        };
        use std::time::Duration;
        use windows::Win32::Networking::WinSock::{
            ADDRESS_FAMILY, AF_INET, IN_ADDR_0_0, SOCKADDR, SOCKADDR_IN,
        };

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

        /// Same discipline for the T0.5 IPv4 discovery surface: signatures
        /// stay pinned WITHOUT spawning the worker, touching COM, or reading
        /// any real network state.
        #[test]
        fn ipv4_discovery_surface_signatures_stay_pinned_without_com_side_effects() {
            use super::{
                DjLinkIpv4BindingError, DjLinkIpv4BindingKey, DjLinkIpv4Candidate,
                DjLinkIpv4CandidateReport, DjLinkIpv4DiscoveryFailure,
            };
            let discover: fn(
                &NlmTrustObserver,
            )
                -> Result<DjLinkIpv4CandidateReport, DjLinkIpv4DiscoveryFailure> =
                NlmTrustObserver::discover_ipv4_candidates;
            let validate: fn(
                &NlmTrustObserver,
                &DjLinkIpv4BindingKey,
            ) -> Result<DjLinkIpv4Candidate, DjLinkIpv4BindingError> =
                NlmTrustObserver::validate_ipv4_binding;
            // Bound, never called: no COM init, no thread spawn, no I/O.
            let _pinned_ipv4_surface = (discover, validate);
        }

        #[test]
        fn sockaddr_in_reader_uses_sin_addr_instead_of_padding() {
            let mut address = SOCKADDR_IN {
                sin_family: AF_INET,
                sin_port: 0x3412,
                ..SOCKADDR_IN::default()
            };
            address.sin_addr.S_un.S_un_b = IN_ADDR_0_0 {
                s_b1: 192,
                s_b2: 168,
                s_b3: 50,
                s_b4: 1,
            };
            address.sin_zero = [0x7f; 8];
            let base = (&address as *const SOCKADDR_IN) as usize;
            let socket = (&address as *const SOCKADDR_IN).cast::<SOCKADDR>();
            assert_eq!(
                read_ipv4_sockaddr(socket, base, core::mem::size_of::<SOCKADDR_IN>() as u64),
                Ok(Some([192, 168, 50, 1]))
            );

            let non_ipv4 = SOCKADDR_IN {
                sin_family: ADDRESS_FAMILY(23),
                ..SOCKADDR_IN::default()
            };
            let non_ipv4_base = (&non_ipv4 as *const SOCKADDR_IN) as usize;
            let non_ipv4_socket = (&non_ipv4 as *const SOCKADDR_IN).cast::<SOCKADDR>();
            assert_eq!(
                read_ipv4_sockaddr(
                    non_ipv4_socket,
                    non_ipv4_base,
                    core::mem::size_of::<SOCKADDR_IN>() as u64
                ),
                Ok(None)
            );
            assert_eq!(
                read_ipv4_sockaddr(socket, base, 4),
                Err(super::DjLinkGaaWalkError::ProtocolViolated)
            );
        }

        /// Explicit operator-only diagnostic for the real Windows NLM/GAA
        /// boundary. Normal test runs must remain deterministic and therefore
        /// skip this test; run it only when a physical show adapter is present
        /// and candidate discovery has failed in the native UI.
        #[test]
        #[ignore = "reads live Windows network state"]
        fn live_ipv4_discovery_reports_typed_result() {
            let observer = NlmTrustObserver::spawn()
                .expect("live NLM observer must initialize for hardware diagnosis");
            let result = observer.discover_ipv4_candidates();
            match &result {
                Ok(report) => eprintln!(
                    "live DJ Link eligible IPv4 candidates: {:#?}",
                    report.eligible()
                ),
                Err(failure) => {
                    eprintln!("live DJ Link IPv4 discovery failure: {failure:#?}")
                }
            }
            observer
                .shutdown()
                .expect("live NLM observer must shut down cleanly");
            assert!(
                result.is_ok(),
                "live DJ Link IPv4 discovery failed: {result:?}"
            );
        }

        /// Pins every header-mirroring numeric constant used by the pure gate
        /// against the actual generated bindings, so a `windows` crate update
        /// that shifts an ABI value fails here instead of silently
        /// reclassifying real adapters.
        #[test]
        fn header_constant_mirror_matches_windows_bindings() {
            use windows::Win32::NetworkManagement::IpHelper::{
                IF_TYPE_ETHERNET_CSMACD, IF_TYPE_SOFTWARE_LOOPBACK,
            };
            use windows::Win32::NetworkManagement::Ndis::{
                MediaConnectStateConnected, NdisMedium802_3, NET_IF_ADMIN_STATUS_UP,
                NET_IF_CONNECTION_DEDICATED, TUNNEL_TYPE_NONE,
            };
            use windows::Win32::Networking::WinSock::IpDadStatePreferred;
            assert_eq!(
                DJ_LINK_IF_TYPE_ETHERNET_CSMACD, IF_TYPE_ETHERNET_CSMACD,
                "wired Ethernet if_type drifted from ipifcons.h"
            );
            assert_eq!(DJ_LINK_IF_TYPE_SOFTWARE_LOOPBACK, IF_TYPE_SOFTWARE_LOOPBACK);
            assert_eq!(DJ_LINK_TUNNEL_TYPE_NONE, TUNNEL_TYPE_NONE.0);
            assert_eq!(DJ_LINK_NDIS_MEDIUM_802_3, NdisMedium802_3.0);
            assert_eq!(
                DJ_LINK_NET_IF_CONNECTION_DEDICATED,
                NET_IF_CONNECTION_DEDICATED.0
            );
            assert_eq!(DJ_LINK_NET_IF_ADMIN_STATUS_UP, NET_IF_ADMIN_STATUS_UP.0);
            assert_eq!(
                DJ_LINK_MEDIA_CONNECT_STATE_CONNECTED,
                MediaConnectStateConnected.0
            );
            assert_eq!(
                DJ_LINK_NL_DAD_STATE_PREFERRED, IpDadStatePreferred.0,
                "preferred DAD state drifted from nldef.h"
            );
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
            DjLinkMachineSettingsV2, DjLinkMachineTransactionState,
            DJ_LINK_MACHINE_SETTINGS_VERSION,
        };
        let make_settings = |bind_ip: &str| DjLinkMachineSettingsV2 {
            version: DJ_LINK_MACHINE_SETTINGS_VERSION,
            revision: 3,
            transaction_state: DjLinkMachineTransactionState::Idle,
            credential_generation: Some(1),
            credential_generation_high_water: 1,
            adapter_guid: Some(ADA_1.to_string()),
            network_guid: Some(NET_A.to_string()),
            bind_ip: Some(bind_ip.to_string()),
            bind_port: Some(49152),
            auto_start_armed: true,
            disarm_cleanup_pending: false,
            rollback: None,
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

    // -- IPv4 candidate bridge (pure) ----------------------------------------

    use DjLinkGaaWalkError as WalkErr;
    use DjLinkIpv4BindingError as BindErr;
    use DjLinkIpv4BlockedReason as Blocked;
    use DjLinkIpv4SelectionError as SelectErr;
    use DjLinkIpv4StructuralAnomaly as Anomaly;

    /// Fully wired-and-up Ethernet gate inputs; failure-path tests reuse it
    /// so their assertions vary ONLY the identity under test.
    fn wired_gate_inputs() -> DjLinkRawGateInputs {
        DjLinkRawGateInputs {
            if_type: DJ_LINK_IF_TYPE_ETHERNET_CSMACD,
            tunnel_type_raw: DJ_LINK_TUNNEL_TYPE_NONE,
            media_type_raw: DJ_LINK_NDIS_MEDIUM_802_3,
            connection_type_raw: DJ_LINK_NET_IF_CONNECTION_DEDICATED,
            admin_status_raw: DJ_LINK_NET_IF_ADMIN_STATUS_UP,
            oper_status_raw: DJ_LINK_IF_OPER_STATUS_UP,
            media_connect_raw: DJ_LINK_MEDIA_CONNECT_STATE_CONNECTED,
            hardware_interface: true,
        }
    }

    /// Fully wired-and-up Ethernet row factory; individual tests mutate the
    /// grouped raw fields directly (same-module privacy) to exercise each
    /// gate.
    fn gated_row(adapter: &str, index: u32, ips: &[(&str, bool)]) -> DjLinkRawAdapterSnapshot {
        DjLinkRawAdapterSnapshot::try_from_parts(
            DjLinkRowIdentityObservations {
                adapter_guid: adapter.to_string(),
                ifentry_guid: adapter.to_string(),
                gaa_ifindex: index,
                luid_ifindex: index,
                ifentry_ifindex: index,
            },
            wired_gate_inputs(),
            DjLinkRowDiagnostics {
                gaa_network_guid: None,
                adapter_alias: Some("Ethernet 1".to_string()),
            },
            ips.iter()
                .map(|(address, preferred)| {
                    DjLinkRawIpv4Address::new(address.parse().expect("test ipv4"), *preferred)
                })
                .collect(),
        )
        .expect("valid gated row")
    }

    fn bind_key(network: &str, adapter: &str, ip: &str) -> DjLinkIpv4BindingKey {
        DjLinkIpv4BindingKey::new(
            network.to_string(),
            adapter.to_string(),
            ip.parse().expect("test ipv4"),
        )
        .expect("valid binding key")
    }

    #[test]
    fn usable_range_predicate_rejects_special_ranges() {
        for rejected in [
            "0.0.0.0",
            "127.0.0.1",
            "127.254.255.254",
            "169.254.0.0",
            "169.254.9.9",
            "224.0.0.5",
            "239.255.255.250",
            "255.255.255.255",
        ] {
            let address: Ipv4Addr = rejected.parse().unwrap();
            assert!(!is_usable_bind_ipv4(address), "{rejected} must be rejected");
        }
        for accepted in [
            "10.0.0.7",
            "172.16.5.4",
            "192.168.17.20",
            "169.253.255.255",
            "126.255.255.255",
            "128.0.0.1",
        ] {
            let address: Ipv4Addr = accepted.parse().unwrap();
            assert!(is_usable_bind_ipv4(address), "{accepted} must be accepted");
        }
    }

    #[test]
    fn ipv4_candidate_joins_one_nlm_adapter_to_one_hardware_ipv4() {
        let observations = observations(vec![entry(NET_A, true, &[ADA_1])]);
        let rows = vec![gated_row(ADA_1, 7, &[("192.168.17.20", true)])];
        let report = reconcile_ipv4_candidates(&observations, &rows).expect("clean join");
        assert_eq!(report.eligible().len(), 1, "exactly one joined candidate");
        assert!(report.blocked().is_empty());
        let candidate = &report.eligible()[0];
        assert_eq!(
            candidate.binding(),
            &bind_key(NET_A, ADA_1, "192.168.17.20")
        );
        assert_eq!(candidate.interface_index(), 7, "current diagnostic only");
        assert_eq!(
            candidate.adapter_alias(),
            Some("Ethernet 1"),
            "display only"
        );

        // Two usable preferred addresses yield two candidates on ONE row.
        let multi = vec![gated_row(
            ADA_1,
            7,
            &[("192.168.17.20", true), ("192.168.17.21", true)],
        )];
        let report = reconcile_ipv4_candidates(&observations, &multi).expect("clean join");
        assert_eq!(report.eligible().len(), 2);
        assert!(report.eligible().iter().all(|candidate| {
            candidate.binding().network_guid() == NET_A
                && candidate.binding().adapter_guid() == ADA_1
        }));
    }

    #[test]
    fn blocks_zero_and_multiple_nlm_membership() {
        let rows = vec![gated_row(ADA_1, 1, &[("192.168.17.20", true)])];

        let unrelated = observations(vec![entry(NET_B, true, &[ADA_2])]);
        let report = reconcile_ipv4_candidates(&unrelated, &rows).expect("structurally clean pass");
        assert!(report.eligible().is_empty());
        assert_eq!(report.blocked().len(), 1);
        assert_eq!(report.blocked()[0].reason(), Blocked::ZeroNlmMembership);
        assert_eq!(report.blocked()[0].adapter_guid(), ADA_1);

        // Two connected networks carrying the SAME adapter fail closed.
        let twins = observations(vec![
            entry(NET_A, true, &[ADA_1]),
            entry(NET_B, true, &[ADA_1]),
        ]);
        let report = reconcile_ipv4_candidates(&twins, &rows).expect("structurally clean pass");
        assert!(report.eligible().is_empty());
        assert_eq!(report.blocked()[0].reason(), Blocked::MultipleNlmMembership);

        // Membership counts CONNECTED networks only.
        let disconnected_only = observations(vec![entry(NET_A, false, &[ADA_1])]);
        let report =
            reconcile_ipv4_candidates(&disconnected_only, &rows).expect("structurally clean pass");
        assert_eq!(report.blocked()[0].reason(), Blocked::ZeroNlmMembership);

        // Duplicate entries of one network id count twice: ambiguity wins.
        let duplicated_id = observations(vec![
            entry(NET_A, true, &[ADA_1]),
            entry(NET_A, true, &[ADA_1]),
        ]);
        let report =
            reconcile_ipv4_candidates(&duplicated_id, &rows).expect("structurally clean pass");
        assert_eq!(report.blocked()[0].reason(), Blocked::MultipleNlmMembership);
    }

    #[test]
    fn blocks_duplicate_adapter_row_index_and_ipv4_address() {
        let observations = observations(vec![entry(NET_A, true, &[ADA_1, ADA_2])]);

        let twin_adapter_rows = vec![
            gated_row(ADA_1, 1, &[("192.168.17.20", true)]),
            gated_row(ADA_1, 2, &[("192.168.17.21", true)]),
        ];
        assert_eq!(
            reconcile_ipv4_candidates(&observations, &twin_adapter_rows),
            Err(Anomaly::DuplicateAdapterRow),
            "two rows claiming one adapter GUID abort the whole pass"
        );

        let twin_index_rows = vec![
            gated_row(ADA_1, 5, &[("192.168.17.20", true)]),
            gated_row(ADA_2, 5, &[("192.168.17.21", true)]),
        ];
        assert_eq!(
            reconcile_ipv4_candidates(&observations, &twin_index_rows),
            Err(Anomaly::DuplicateInterfaceIndex)
        );

        let twin_address_rows = vec![
            gated_row(ADA_1, 1, &[("192.168.17.20", true)]),
            gated_row(ADA_2, 2, &[("192.168.17.20", true)]),
        ];
        assert_eq!(
            reconcile_ipv4_candidates(&observations, &twin_address_rows),
            Err(Anomaly::DuplicateIpv4Address),
            "one IPv4 on two rows poisons enumeration trust"
        );
    }

    #[test]
    fn blocks_ipv6_only_loopback_tunnel_and_nonhardware() {
        let observations = observations(vec![entry(NET_A, true, &[ADA_1])]);

        // IPv6-only: no IPv4 unicast rows at all.
        let mut row = gated_row(ADA_1, 1, &[]);
        let report = reconcile_ipv4_candidates(&observations, &[row.clone()])
            .expect("structurally clean pass");
        assert_eq!(report.blocked()[0].reason(), Blocked::Ipv6Only);

        // Non-hardware beats everything else in gate order.
        row.gate_inputs.hardware_interface = false;
        row.ipv4_addresses = vec![DjLinkRawIpv4Address::new(
            "192.168.17.20".parse().unwrap(),
            true,
        )];
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(report.blocked()[0].reason(), Blocked::NonHardwareInterface);

        // Loopback type is classified structurally.
        row.gate_inputs.hardware_interface = true;
        row.gate_inputs.if_type = DJ_LINK_IF_TYPE_SOFTWARE_LOOPBACK;
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(report.blocked()[0].reason(), Blocked::LoopbackInterface);

        // Tunnel type other than NONE (structure, never name text).
        row.gate_inputs.if_type = DJ_LINK_IF_TYPE_ETHERNET_CSMACD;
        row.gate_inputs.tunnel_type_raw = 2;
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(report.blocked()[0].reason(), Blocked::TunnelInterface);

        // Wrong IF_TYPE and wrong MediaType each mean "not wired 802.3".
        row.gate_inputs.tunnel_type_raw = DJ_LINK_TUNNEL_TYPE_NONE;
        row.gate_inputs.if_type = 999;
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(
            report.blocked()[0].reason(),
            Blocked::UnsupportedPhysicalMedium
        );
        row.gate_inputs.if_type = DJ_LINK_IF_TYPE_ETHERNET_CSMACD;
        row.gate_inputs.media_type_raw = DJ_LINK_NDIS_MEDIUM_802_3 + 1;
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(
            report.blocked()[0].reason(),
            Blocked::UnsupportedPhysicalMedium
        );

        // Non-dedicated connection types are refused.
        row.gate_inputs.media_type_raw = DJ_LINK_NDIS_MEDIUM_802_3;
        row.gate_inputs.connection_type_raw = DJ_LINK_NET_IF_CONNECTION_DEDICATED + 1;
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(
            report.blocked()[0].reason(),
            Blocked::NonDedicatedConnection
        );

        // Gate-order control: hardware flag dominates loopback classification.
        let mut weird = gated_row(ADA_1, 1, &[("192.168.17.20", true)]);
        weird.gate_inputs.if_type = DJ_LINK_IF_TYPE_SOFTWARE_LOOPBACK;
        weird.gate_inputs.hardware_interface = false;
        let report = reconcile_ipv4_candidates(&observations, &[weird]).unwrap();
        assert_eq!(report.blocked()[0].reason(), Blocked::NonHardwareInterface);
    }

    #[test]
    fn blocks_down_media_disconnected_and_nonpreferred_only() {
        let observations = observations(vec![entry(NET_A, true, &[ADA_1])]);
        let mut row = gated_row(ADA_1, 1, &[("192.168.17.20", true)]);

        row.gate_inputs.admin_status_raw = DJ_LINK_NET_IF_ADMIN_STATUS_UP + 1;
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(report.blocked()[0].reason(), Blocked::AdminDown);
        row.gate_inputs.admin_status_raw = DJ_LINK_NET_IF_ADMIN_STATUS_UP;

        row.gate_inputs.oper_status_raw = DJ_LINK_IF_OPER_STATUS_UP + 1;
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(report.blocked()[0].reason(), Blocked::OperDown);
        row.gate_inputs.oper_status_raw = DJ_LINK_IF_OPER_STATUS_UP;

        row.gate_inputs.media_connect_raw = DJ_LINK_MEDIA_CONNECT_STATE_CONNECTED + 1;
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(report.blocked()[0].reason(), Blocked::MediaDisconnected);
        row.gate_inputs.media_connect_raw = DJ_LINK_MEDIA_CONNECT_STATE_CONNECTED;

        // Preferred flag missing everywhere.
        row.ipv4_addresses = vec![DjLinkRawIpv4Address::new(
            "192.168.17.20".parse().unwrap(),
            false,
        )];
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(report.blocked()[0].reason(), Blocked::OnlyNonPreferredIpv4);

        // Preferred but every address unusable (APIPA range).
        row.ipv4_addresses = vec![DjLinkRawIpv4Address::new(
            "169.254.9.9".parse().unwrap(),
            true,
        )];
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(report.blocked()[0].reason(), Blocked::NoUsableIpv4);

        // Usable address that is NOT preferred cannot rescue the pass.
        row.ipv4_addresses = vec![
            DjLinkRawIpv4Address::new("169.254.9.9".parse().unwrap(), true),
            DjLinkRawIpv4Address::new("192.168.17.20".parse().unwrap(), false),
        ];
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(
            report.blocked()[0].reason(),
            Blocked::NoUsableIpv4,
            "preferred-only filtering rejects the usable-but-not-preferred address"
        );

        // One preferred usable address among junk is enough.
        row.ipv4_addresses = vec![
            DjLinkRawIpv4Address::new("169.254.9.9".parse().unwrap(), true),
            DjLinkRawIpv4Address::new("127.0.0.1".parse().unwrap(), true),
            DjLinkRawIpv4Address::new("192.168.17.20".parse().unwrap(), true),
        ];
        let report = reconcile_ipv4_candidates(&observations, std::slice::from_ref(&row)).unwrap();
        assert_eq!(report.eligible().len(), 1);
        assert_eq!(
            report.eligible()[0].binding(),
            &bind_key(NET_A, ADA_1, "192.168.17.20")
        );
    }

    #[test]
    fn multiple_eligible_requires_explicit_exact_selection() {
        let observations = observations(vec![entry(NET_A, true, &[ADA_1, ADA_2])]);
        let rows = vec![
            gated_row(
                ADA_1,
                1,
                &[("192.168.17.10", true), ("192.168.17.11", true)],
            ),
            gated_row(ADA_2, 2, &[("192.168.17.12", true)]),
        ];
        let report = reconcile_ipv4_candidates(&observations, &rows).expect("clean join");
        assert_eq!(report.eligible().len(), 3);

        // FIRST-CHOICE MUTATION TRAP: the first candidate shares its network
        // and adapter with the requested tuple but carries .10; requesting
        // the full tuple for .11 must resolve .11, never the leading row.
        let selected = require_unique_binding(&report, &bind_key(NET_A, ADA_1, "192.168.17.11"))
            .expect("exact tuple present");
        assert_eq!(
            selected.binding(),
            &bind_key(NET_A, ADA_1, "192.168.17.11"),
            "selection must compare the COMPLETE tuple, not pick the first row"
        );

        // Same-address-different-adapter trap: the first eligible row shares
        // nothing with this request except the address family; a partial-key
        // matcher would wrongly return it.
        let selected = require_unique_binding(&report, &bind_key(NET_A, ADA_2, "192.168.17.12"))
            .expect("exact tuple present");
        assert_eq!(selected.binding().adapter_guid(), ADA_2);

        // Absent tuples stay absent even though the set is non-empty, and
        // the network GUID participates in equality.
        assert_eq!(
            require_unique_binding(&report, &bind_key(NET_A, ADA_1, "192.168.17.99")),
            Err(SelectErr::ExactTupleAbsent)
        );
        assert_eq!(
            require_unique_binding(&report, &bind_key(NET_B, ADA_1, "192.168.17.10")),
            Err(SelectErr::ExactTupleAbsent),
            "swapping the NLM network identity must invalidate the tuple"
        );

        // Empty eligible set is its own typed outcome.
        let empty = reconcile_ipv4_candidates(&observations, &[]).expect("clean pass");
        assert_eq!(
            require_unique_binding(&empty, &bind_key(NET_A, ADA_1, "192.168.17.10")),
            Err(SelectErr::EligibleSetEmpty)
        );

        // Ambiguity: the complete tuple twice must NEVER select the first.
        let duplicated_tuple = DjLinkIpv4CandidateReport {
            eligible: vec![
                DjLinkIpv4Candidate::new(bind_key(NET_A, ADA_1, "192.168.17.10"), 1, None),
                DjLinkIpv4Candidate::new(bind_key(NET_A, ADA_1, "192.168.17.10"), 2, None),
            ],
            blocked: Vec::new(),
        };
        assert_eq!(
            require_unique_binding(&duplicated_tuple, &bind_key(NET_A, ADA_1, "192.168.17.10")),
            Err(SelectErr::ExactTupleAmbiguous { matches: 2 })
        );
    }

    #[test]
    fn binding_revalidation_exact_tuple_blocked_absent_and_ambiguous() {
        let observations = observations(vec![entry(NET_A, true, &[ADA_1])]);

        // Happy revalidation round-trips the exact candidate.
        let rows = vec![gated_row(ADA_1, 1, &[("192.168.17.20", true)])];
        let report = reconcile_ipv4_candidates(&observations, &rows).unwrap();
        assert_eq!(
            validate_binding_against_report(&report, &bind_key(NET_A, ADA_1, "192.168.17.20")),
            Ok(report.eligible()[0].clone())
        );

        // Cable pulled AFTER trust: the adapter is present but blocked, and
        // the failure names the CURRENT reason instead of degrading.
        let mut unplugged = gated_row(ADA_1, 1, &[("192.168.17.20", true)]);
        unplugged.gate_inputs.media_connect_raw = DJ_LINK_MEDIA_CONNECT_STATE_CONNECTED + 1;
        let report = reconcile_ipv4_candidates(&observations, &[unplugged]).unwrap();
        assert_eq!(
            validate_binding_against_report(&report, &bind_key(NET_A, ADA_1, "192.168.17.20")),
            Err(BindErr::TupleBlocked(Blocked::MediaDisconnected))
        );

        // Administratively disabled surfaces its own reason.
        let mut disabled = gated_row(ADA_1, 1, &[("192.168.17.20", true)]);
        disabled.gate_inputs.admin_status_raw = DJ_LINK_NET_IF_ADMIN_STATUS_UP + 1;
        let report = reconcile_ipv4_candidates(&observations, &[disabled]).unwrap();
        assert_eq!(
            validate_binding_against_report(&report, &bind_key(NET_A, ADA_1, "192.168.17.20")),
            Err(BindErr::TupleBlocked(Blocked::AdminDown))
        );

        // Unknown adapter: plain absence.
        let report = reconcile_ipv4_candidates(
            &observations,
            &[gated_row(ADA_1, 1, &[("192.168.17.20", true)])],
        )
        .unwrap();
        assert_eq!(
            validate_binding_against_report(&report, &bind_key(NET_A, ADA_2, "192.168.17.20")),
            Err(BindErr::TupleAbsent)
        );

        // Eligible adapter but different address: absence, NOT a block.
        assert_eq!(
            validate_binding_against_report(&report, &bind_key(NET_A, ADA_1, "192.168.17.99")),
            Err(BindErr::TupleAbsent)
        );

        // Duplicated eligible tuple: ambiguity propagates typed.
        let duplicated_tuple = DjLinkIpv4CandidateReport {
            eligible: vec![
                DjLinkIpv4Candidate::new(bind_key(NET_A, ADA_1, "192.168.17.20"), 1, None),
                DjLinkIpv4Candidate::new(bind_key(NET_A, ADA_1, "192.168.17.20"), 1, None),
            ],
            blocked: Vec::new(),
        };
        assert_eq!(
            validate_binding_against_report(
                &duplicated_tuple,
                &bind_key(NET_A, ADA_1, "192.168.17.20")
            ),
            Err(BindErr::ExactTupleAmbiguous { matches: 2 })
        );
    }

    #[test]
    fn alias_and_index_are_display_only_and_never_change_trust_identity() {
        let make = |alias: Option<&str>, index: u32| {
            let mut row = gated_row(ADA_1, index, &[("192.168.17.20", true)]);
            row.diagnostics.adapter_alias = alias.map(str::to_string);
            row
        };
        let observations_net_a = || observations(vec![entry(NET_A, true, &[ADA_1])]);

        let plain =
            reconcile_ipv4_candidates(&observations_net_a(), &[make(Some("Ethernet 1"), 5)])
                .unwrap();
        let renamed = reconcile_ipv4_candidates(
            &observations_net_a(),
            &[make(Some("vEthernet (WSL (Mirrored))"), 999)],
        )
        .unwrap();

        // Display fields differ wildly; binding identity does not move.
        assert_ne!(
            plain.eligible()[0].adapter_alias(),
            renamed.eligible()[0].adapter_alias()
        );
        assert_ne!(
            plain.eligible()[0].interface_index(),
            renamed.eligible()[0].interface_index()
        );
        assert_eq!(
            plain.eligible()[0].binding(),
            renamed.eligible()[0].binding()
        );

        // Selection succeeds identically under both display variants.
        let selected = require_unique_binding(&plain, &bind_key(NET_A, ADA_1, "192.168.17.20"))
            .expect("exact tuple present");
        assert_eq!(selected.binding(), &bind_key(NET_A, ADA_1, "192.168.17.20"));
        assert!(
            require_unique_binding(&renamed, &bind_key(NET_A, ADA_1, "192.168.17.20")).is_ok(),
            "renamed/reindexed adapter must still resolve the same tuple"
        );

        // Blocked-row attribution ignores display fields too.
        let mut unplugged_plain = make(Some("Ethernet 1"), 5);
        unplugged_plain.gate_inputs.media_connect_raw = DJ_LINK_MEDIA_CONNECT_STATE_CONNECTED + 1;
        let mut unplugged_renamed = make(Some("vEthernet (WSL)"), 999);
        unplugged_renamed.gate_inputs.media_connect_raw = DJ_LINK_MEDIA_CONNECT_STATE_CONNECTED + 1;
        let blocked_plain =
            reconcile_ipv4_candidates(&observations_net_a(), &[unplugged_plain]).unwrap();
        let blocked_renamed =
            reconcile_ipv4_candidates(&observations_net_a(), &[unplugged_renamed]).unwrap();
        assert_eq!(
            validate_binding_against_report(
                &blocked_plain,
                &bind_key(NET_A, ADA_1, "192.168.17.20")
            ),
            validate_binding_against_report(
                &blocked_renamed,
                &bind_key(NET_A, ADA_1, "192.168.17.20")
            ),
            "revalidation failure must not depend on alias text or index"
        );

        // Control (mutation sensitivity): changing the BINDING IPV4 flips
        // the outcome, proving the invariance assertions are not vacuous.
        let moved_address = reconcile_ipv4_candidates(
            &observations_net_a(),
            &[gated_row(ADA_1, 5, &[("192.168.17.21", true)])],
        )
        .unwrap();
        assert_ne!(
            moved_address.eligible()[0].binding(),
            plain.eligible()[0].binding()
        );
        assert_eq!(
            validate_binding_against_report(
                &moved_address,
                &bind_key(NET_A, ADA_1, "192.168.17.20")
            ),
            Err(BindErr::TupleAbsent)
        );
    }

    #[test]
    fn gaa_network_guid_is_never_trusted_as_nlm_identity() {
        let make_with_gaa_guid = |gaa_guid: Option<&str>| {
            let mut row = gated_row(ADA_1, 1, &[("192.168.17.20", true)]);
            row.diagnostics.gaa_network_guid = gaa_guid.map(str::to_string);
            row
        };

        // The GAA NetworkGuid varies across NIL, a foreign network, and
        // garbage text: reconciliation output MUST be invariant.
        let baseline = reconcile_ipv4_candidates(
            &observations(vec![entry(NET_A, true, &[ADA_1])]),
            &[make_with_gaa_guid(None)],
        )
        .unwrap();
        for hostile_gaa_guid in [
            Some(NET_B),
            Some("00000000-0000-0000-0000-000000000000"),
            Some("garbage"),
        ] {
            let report = reconcile_ipv4_candidates(
                &observations(vec![entry(NET_A, true, &[ADA_1])]),
                &[make_with_gaa_guid(hostile_gaa_guid)],
            )
            .unwrap();
            assert_eq!(
                report, baseline,
                "GAA NetworkGuid {hostile_gaa_guid:?} must be ignored entirely"
            );
            // Trust would break: even with the GAA claiming NET_B, a NET_B
            // tuple NEVER validates — the adapter is eligible only through
            // its real NLM membership, so the foreign tuple is absent.
            assert_eq!(
                validate_binding_against_report(&report, &bind_key(NET_B, ADA_1, "192.168.17.20")),
                Err(BindErr::TupleAbsent)
            );
        }

        // Controls: changing the actual NLM membership DOES change the
        // outcome — eligibility MOVES to the new network — proving the
        // invariance assertion above is not vacuous.
        let moved_membership = reconcile_ipv4_candidates(
            &observations(vec![entry(NET_B, true, &[ADA_1])]),
            &[make_with_gaa_guid(None)],
        )
        .unwrap();
        assert_ne!(moved_membership, baseline);
        assert_eq!(moved_membership.eligible().len(), 1);
        assert_eq!(
            moved_membership.eligible()[0].binding(),
            &bind_key(NET_B, ADA_1, "192.168.17.20")
        );
    }

    #[test]
    fn row_identity_mismatch_fails_closed() {
        // GetIfEntry2 GUID disagreeing with ConvertInterfaceLuidToGuid.
        assert_eq!(
            DjLinkRawAdapterSnapshot::try_from_parts(
                DjLinkRowIdentityObservations {
                    adapter_guid: ADA_1.to_string(),
                    ifentry_guid: ADA_2.to_string(),
                    gaa_ifindex: 1,
                    luid_ifindex: 1,
                    ifentry_ifindex: 1,
                },
                wired_gate_inputs(),
                DjLinkRowDiagnostics {
                    gaa_network_guid: None,
                    adapter_alias: None,
                },
                Vec::new(),
            ),
            Err(Anomaly::RowIdentityMismatch)
        );

        // Three-way index disagreement.
        for (luid_index, ifentry_index) in [(2_u32, 1_u32), (1, 3)] {
            assert_eq!(
                DjLinkRawAdapterSnapshot::try_from_parts(
                    DjLinkRowIdentityObservations {
                        adapter_guid: ADA_1.to_string(),
                        ifentry_guid: ADA_1.to_string(),
                        gaa_ifindex: 1,
                        luid_ifindex: luid_index,
                        ifentry_ifindex: ifentry_index,
                    },
                    wired_gate_inputs(),
                    DjLinkRowDiagnostics {
                        gaa_network_guid: None,
                        adapter_alias: None,
                    },
                    Vec::new(),
                ),
                Err(Anomaly::RowIdentityMismatch),
                "(luid={luid_index}, ifentry={ifentry_index})"
            );
        }

        // Non-canonical identity strings never become rows.
        assert_eq!(
            DjLinkRawAdapterSnapshot::try_from_parts(
                DjLinkRowIdentityObservations {
                    adapter_guid: ADA_1.to_uppercase(),
                    ifentry_guid: ADA_1.to_string(),
                    gaa_ifindex: 1,
                    luid_ifindex: 1,
                    ifentry_ifindex: 1,
                },
                wired_gate_inputs(),
                DjLinkRowDiagnostics {
                    gaa_network_guid: None,
                    adapter_alias: None,
                },
                Vec::new(),
            ),
            Err(Anomaly::RowIdentityMismatch)
        );
    }

    #[test]
    fn reconciliation_output_is_order_insensitive() {
        let observations = observations(vec![entry(NET_A, true, &[ADA_1, ADA_2])]);
        let forward = vec![
            gated_row(ADA_1, 1, &[("192.168.17.10", true)]),
            gated_row(ADA_2, 2, &[("192.168.17.12", true)]),
        ];
        let reversed: Vec<_> = forward.iter().rev().cloned().collect();
        assert_eq!(
            reconcile_ipv4_candidates(&observations, &forward),
            reconcile_ipv4_candidates(&observations, &reversed),
            "OS enumeration order must not leak into the report"
        );
    }

    #[test]
    fn gaa_buffer_growth_bounded_to_three_resize_attempts() {
        // Always overflowing: exactly four total sizing calls, then a typed
        // exhaustion failure with the total attempt count.
        let mut calls = 0_usize;
        let outcome = drive_gaa_allocation(|capacity| {
            calls += 1;
            Ok(GaaSizeAttempt::Overflow {
                needed: capacity + 1,
            })
        });
        assert_eq!(
            outcome,
            Err(WalkErr::ResizeAttemptsExhausted {
                attempts: MAX_GAA_RESIZE_ATTEMPTS + 1
            })
        );
        assert_eq!(calls, MAX_GAA_RESIZE_ATTEMPTS + 1);

        // Growing demand that fits on the third call succeeds and reports
        // the capacity of the fitting attempt.
        let mut calls = 0_usize;
        let outcome = drive_gaa_allocation(|capacity| {
            calls += 1;
            match calls {
                1 => Ok(GaaSizeAttempt::Overflow {
                    needed: capacity + 100,
                }),
                2 => Ok(GaaSizeAttempt::Overflow {
                    needed: capacity + 100,
                }),
                _ => Ok(GaaSizeAttempt::Fit),
            }
        });
        assert!(
            matches!(outcome, Ok(_)),
            "third attempt must fit: {outcome:?}"
        );
        assert_eq!(calls, 3);

        // Demand beyond the hard byte cap fails IMMEDIATELY.
        let mut calls = 0_usize;
        let outcome = drive_gaa_allocation(|_capacity| {
            calls += 1;
            Ok(GaaSizeAttempt::Overflow {
                needed: MAX_GAA_BUFFER_BYTES + 1,
            })
        });
        assert_eq!(
            outcome,
            Err(WalkErr::BufferBoundExceeded {
                needed: MAX_GAA_BUFFER_BYTES + 1
            })
        );
        assert_eq!(calls, 1, "no further call after an over-cap demand");

        // Overflow claimed without demanding growth violates protocol.
        let mut calls = 0_usize;
        let outcome = drive_gaa_allocation(|capacity| {
            calls += 1;
            Ok(GaaSizeAttempt::Overflow { needed: capacity })
        });
        assert_eq!(outcome, Err(WalkErr::ProtocolViolated));
        assert_eq!(calls, 1);

        // Immediate fit performs exactly one call at the initial size.
        let mut calls = 0_usize;
        let outcome = drive_gaa_allocation(|_capacity| {
            calls += 1;
            Ok(GaaSizeAttempt::Fit)
        });
        assert_eq!(outcome, Ok(INITIAL_GAA_BUFFER_BYTES));
        assert_eq!(calls, 1);

        // Host errors propagate verbatim out of the driver.
        let outcome = drive_gaa_allocation(|_capacity| Err(WalkErr::HostFailed { code: 232 }));
        assert_eq!(outcome, Err(WalkErr::HostFailed { code: 232 }));
    }

    #[test]
    fn gaa_pointer_walk_rejects_cycle_out_of_range_and_misalignment() {
        let limits = GaaWalkLimits {
            row_alignment: 8,
            min_row_len: 32,
            max_rows: 4,
        };
        let buffer_len = 4096_u64;

        // Well-formed three-node chain ends at the terminal null.
        let walked = walk_gaa_linked_list(
            |offset| match offset {
                0 => Some(32),
                32 => Some(64),
                _ => None,
            },
            0,
            buffer_len,
            &limits,
            || DjLinkGaaWalkError::AdapterLimitExceeded,
        );
        assert_eq!(walked, Ok(vec![0, 32, 64]));

        // Next pointing back to a visited node is a cycle.
        assert_eq!(
            walk_gaa_linked_list(
                |offset| match offset {
                    0 => Some(32),
                    32 => Some(64),
                    _ => Some(32),
                },
                0,
                buffer_len,
                &limits,
                || DjLinkGaaWalkError::AdapterLimitExceeded,
            ),
            Err(WalkErr::NextCycleDetected)
        );

        // Self-looping head.
        assert_eq!(
            walk_gaa_linked_list(
                |offset| Some(offset),
                48,
                buffer_len,
                &limits,
                || DjLinkGaaWalkError::AdapterLimitExceeded,
            ),
            Err(WalkErr::NextCycleDetected)
        );

        // Out-of-range target beyond the allocation.
        assert_eq!(
            walk_gaa_linked_list(
                |offset| if offset == 0 { Some(4096) } else { None },
                0,
                buffer_len,
                &limits,
                || DjLinkGaaWalkError::AdapterLimitExceeded,
            ),
            Err(WalkErr::PointerOutOfRange {
                offset: 4096,
                buffer_len
            })
        );

        // Aligned start whose FULL row would cross the allocation end.
        assert_eq!(
            walk_gaa_linked_list(
                |_offset| None,
                4080,
                buffer_len,
                &limits,
                || DjLinkGaaWalkError::AdapterLimitExceeded,
            ),
            Err(WalkErr::PointerOutOfRange {
                offset: 4080,
                buffer_len
            }),
            "a full row must fit inside the buffer, not just its first byte"
        );

        // Misaligned mid-chain target.
        assert_eq!(
            walk_gaa_linked_list(
                |offset| if offset == 0 { Some(36) } else { None },
                0,
                buffer_len,
                &limits,
                || DjLinkGaaWalkError::AdapterLimitExceeded,
            ),
            Err(WalkErr::MisalignedPointer {
                offset: 36,
                alignment: 8
            })
        );

        // Misaligned FIRST offset fails identically.
        assert_eq!(
            walk_gaa_linked_list(
                |_offset| None,
                4,
                buffer_len,
                &limits,
                || DjLinkGaaWalkError::AdapterLimitExceeded,
            ),
            Err(WalkErr::MisalignedPointer {
                offset: 4,
                alignment: 8
            })
        );

        // Row cap enforced with the CALLER'S limit error (adapters vs
        // addresses share one walker).
        let endless = |offset: u64| Some(offset + 32);
        assert_eq!(
            walk_gaa_linked_list(endless, 0, buffer_len, &limits, || {
                DjLinkGaaWalkError::AdapterLimitExceeded
            },),
            Err(WalkErr::AdapterLimitExceeded)
        );
        assert_eq!(
            walk_gaa_linked_list(endless, 0, buffer_len, &limits, || {
                DjLinkGaaWalkError::AddressLimitExceeded
            },),
            Err(WalkErr::AddressLimitExceeded)
        );

        // Degenerate configuration fails closed before walking.
        let degenerate = GaaWalkLimits {
            row_alignment: 8,
            min_row_len: 32,
            max_rows: 0,
        };
        assert_eq!(
            walk_gaa_linked_list(
                |_offset| None,
                0,
                buffer_len,
                &degenerate,
                || DjLinkGaaWalkError::AdapterLimitExceeded,
            ),
            Err(WalkErr::AdapterLimitExceeded)
        );

        // First offset itself out of range.
        assert_eq!(
            walk_gaa_linked_list(
                |_offset| None,
                5000,
                buffer_len,
                &limits,
                || DjLinkGaaWalkError::AdapterLimitExceeded,
            ),
            Err(WalkErr::PointerOutOfRange {
                offset: 5000,
                buffer_len
            })
        );
    }
}
