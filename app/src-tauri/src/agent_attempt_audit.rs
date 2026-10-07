//! Redacted metadata for external authorization observations, not effect receipts.
use protocol::control_plane::OperationRisk;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct BridgeAttempt {
    pub adapter: &'static str,
    pub risk: &'static str,
    pub principal_incarnation: u64,
    pub request_sha256: String,
    pub argument_sha256: String,
    pub observed_unix_ms: Option<u64>,
    pub consent_policy: &'static str,
}
impl BridgeAttempt {
    pub(crate) fn new(risk: OperationRisk, incarnation: u64, request: &str, arguments: &[u8;32]) -> Self {
        Self {
            adapter: "external_mcp",
            risk: match risk { OperationRisk::R0=>"R0",OperationRisk::R1=>"R1",OperationRisk::R2=>"R2",
                OperationRisk::R3=>"R3",OperationRisk::R4=>"R4",OperationRisk::R5=>"R5",OperationRisk::S0=>"S0" },
            principal_incarnation: incarnation,
            request_sha256: super::diagnostic_audit::identity_hash(request),
            argument_sha256: super::diagnostic_audit::hex(arguments),
            // Presentation only. All authority/expiry/rate decisions stay monotonic.
            observed_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH).ok()
                .and_then(|duration|u64::try_from(duration.as_millis()).ok())
                .filter(|ms|*ms<=protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER),
            consent_policy: "exact_grant_no_individual_approval",
        }
    }
}
