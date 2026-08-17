//! Machine-local crash boundary for output-disruptive R4 request identity.
//!
//! This module owns only the strict durable state machine. Filesystem path
//! resolution and integration with the live Release transaction remain in the
//! app adapter so the state model can be tested without Tauri or output I/O.

use std::collections::HashSet;

use protocol::control_plane_command::{
    next_timeline_transport_authority, OutputControlErrorCodeV1, OutputControlFenceV1,
    OutputControlRejectionV1, OutputControlResponseV1, MAX_SAFE_JAVASCRIPT_INTEGER,
    OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
};
use serde::{Deserialize, Serialize};

pub(crate) const R4_DURABILITY_STATE_VERSION: u32 = 1;
pub(crate) const MAX_DURABLE_R4_RECORDS: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DurableR4RequestKeyV1 {
    pub principal_id: String,
    pub operation_id: String,
    pub request_id: u64,
}

impl DurableR4RequestKeyV1 {
    fn validate(&self) -> Result<(), DurableR4StateError> {
        if self.principal_id.is_empty()
            || self.principal_id.len() > 128
            || !self.principal_id.is_ascii()
            || self
                .principal_id
                .bytes()
                .any(|byte| byte.is_ascii_control())
        {
            return Err(DurableR4StateError::InvalidPrincipal);
        }
        if self.operation_id != OUTPUT_BLACKOUT_RELEASE_OPERATION_ID {
            return Err(DurableR4StateError::InvalidOperation);
        }
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(DurableR4StateError::InvalidRequestId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DurableR4RecordStateV1 {
    Prepared,
    Terminal { response: OutputControlResponseV1 },
    Tombstone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DurableR4RecordV1 {
    pub key: DurableR4RequestKeyV1,
    pub shape_sha256: String,
    pub argument_fingerprint: String,
    pub fence_before: OutputControlFenceV1,
    /// Exact successor which would describe an Applied Release. It is retained
    /// even when the live result may later be NoOp so crash recovery can advance
    /// beyond every authority pair the interrupted transaction could have used.
    pub predicted_applied_fence: OutputControlFenceV1,
    pub record: DurableR4RecordStateV1,
}

impl DurableR4RecordV1 {
    fn validate(&self) -> Result<(), DurableR4StateError> {
        self.key.validate()?;
        validate_hash(&self.shape_sha256)?;
        validate_hash(&self.argument_fingerprint)?;
        self.fence_before
            .validate()
            .map_err(|_| DurableR4StateError::InvalidFence)?;
        self.predicted_applied_fence
            .validate()
            .map_err(|_| DurableR4StateError::InvalidFence)?;
        if !is_exact_release_successor(&self.fence_before, &self.predicted_applied_fence) {
            return Err(DurableR4StateError::InvalidFence);
        }
        if let DurableR4RecordStateV1::Terminal { response } = &self.record {
            response
                .validate()
                .map_err(|_| DurableR4StateError::InvalidTerminal)?;
            match response {
                OutputControlResponseV1::Receipt(receipt) => {
                    if receipt.operation_id != self.key.operation_id
                        || receipt.request_id != self.key.request_id
                        || receipt.shape_sha256 != self.shape_sha256
                        || receipt.argument_fingerprint != self.argument_fingerprint
                        || receipt.fence_before != self.fence_before
                    {
                        return Err(DurableR4StateError::InvalidTerminal);
                    }
                }
                OutputControlResponseV1::Rejected(rejection) => {
                    if rejection.operation_id != self.key.operation_id
                        || rejection.request_id != self.key.request_id
                    {
                        return Err(DurableR4StateError::InvalidTerminal);
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DurableR4StateV1 {
    pub version: u32,
    pub state_generation: u64,
    /// `false` is written before an app session may leave its startup-safe
    /// output gate. A clean shutdown flips it to `true` durably.
    pub clean_shutdown: bool,
    pub safety_blackout_engaged: bool,
    pub safety_blackout_epoch: u64,
    pub safety_blackout_generation: u64,
    pub records: Vec<DurableR4RecordV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DurableR4Reservation {
    Reserved,
    ExistingPrepared,
    ExistingTerminal(OutputControlResponseV1),
    ReceiptExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DurableR4StateError {
    InvalidVersion,
    InvalidGeneration,
    InvalidPrincipal,
    InvalidOperation,
    InvalidRequestId,
    InvalidHash,
    InvalidFence,
    InvalidTerminal,
    DuplicateKey,
    ShapeConflict,
    Capacity,
    MissingPrepared,
    AuthorityExhausted,
}

impl DurableR4StateV1 {
    pub(crate) fn new(
        safety_blackout_engaged: bool,
        safety_blackout_epoch: u64,
        safety_blackout_generation: u64,
    ) -> Result<Self, DurableR4StateError> {
        let state = Self {
            version: R4_DURABILITY_STATE_VERSION,
            state_generation: 1,
            clean_shutdown: true,
            safety_blackout_engaged,
            safety_blackout_epoch,
            safety_blackout_generation,
            records: Vec::new(),
        };
        state.validate()?;
        Ok(state)
    }

    pub(crate) fn validate(&self) -> Result<(), DurableR4StateError> {
        if self.version != R4_DURABILITY_STATE_VERSION {
            return Err(DurableR4StateError::InvalidVersion);
        }
        validate_nonzero_safe(self.state_generation)?;
        validate_nonzero_safe(self.safety_blackout_epoch)?;
        validate_nonzero_safe(self.safety_blackout_generation)?;
        if self.records.len() > MAX_DURABLE_R4_RECORDS {
            return Err(DurableR4StateError::Capacity);
        }
        let mut keys = HashSet::with_capacity(self.records.len());
        for record in &self.records {
            record.validate()?;
            if !keys.insert(record.key.clone()) {
                return Err(DurableR4StateError::DuplicateKey);
            }
        }
        Ok(())
    }

    pub(crate) fn reserve_prepared(
        &mut self,
        key: DurableR4RequestKeyV1,
        shape_sha256: String,
        argument_fingerprint: String,
        fence_before: OutputControlFenceV1,
    ) -> Result<DurableR4Reservation, DurableR4StateError> {
        key.validate()?;
        validate_hash(&shape_sha256)?;
        validate_hash(&argument_fingerprint)?;
        fence_before
            .validate()
            .map_err(|_| DurableR4StateError::InvalidFence)?;

        if let Some(existing) = self.records.iter().find(|record| record.key == key) {
            if existing.shape_sha256 != shape_sha256 {
                return Err(DurableR4StateError::ShapeConflict);
            }
            return Ok(match &existing.record {
                DurableR4RecordStateV1::Prepared => DurableR4Reservation::ExistingPrepared,
                DurableR4RecordStateV1::Terminal { response } => {
                    DurableR4Reservation::ExistingTerminal(response.clone())
                }
                DurableR4RecordStateV1::Tombstone => DurableR4Reservation::ReceiptExpired,
            });
        }
        if self.records.len() >= MAX_DURABLE_R4_RECORDS {
            return Err(DurableR4StateError::Capacity);
        }
        let predicted_applied_fence = release_successor(&fence_before)?;
        self.records.push(DurableR4RecordV1 {
            key,
            shape_sha256,
            argument_fingerprint,
            fence_before,
            predicted_applied_fence,
            record: DurableR4RecordStateV1::Prepared,
        });
        self.bump_state_generation()?;
        Ok(DurableR4Reservation::Reserved)
    }

    pub(crate) fn store_terminal(
        &mut self,
        key: &DurableR4RequestKeyV1,
        shape_sha256: &str,
        response: OutputControlResponseV1,
    ) -> Result<(), DurableR4StateError> {
        response
            .validate()
            .map_err(|_| DurableR4StateError::InvalidTerminal)?;
        let record = self
            .records
            .iter_mut()
            .find(|record| &record.key == key)
            .ok_or(DurableR4StateError::MissingPrepared)?;
        if record.shape_sha256 != shape_sha256 {
            return Err(DurableR4StateError::ShapeConflict);
        }
        if !matches!(&record.record, DurableR4RecordStateV1::Prepared) {
            return Err(DurableR4StateError::MissingPrepared);
        }
        let candidate = DurableR4RecordV1 {
            record: DurableR4RecordStateV1::Terminal {
                response: response.clone(),
            },
            ..record.clone()
        };
        candidate.validate()?;
        record.record = DurableR4RecordStateV1::Terminal { response };
        if let DurableR4RecordStateV1::Terminal {
            response: OutputControlResponseV1::Receipt(receipt),
        } = &record.record
        {
            if matches!(
                receipt.outcome,
                protocol::control_plane_command::OutputControlReceiptOutcomeV1::Applied
            ) {
                self.safety_blackout_engaged = false;
            }
            self.safety_blackout_epoch = receipt.fence_after.safety_blackout_epoch;
            self.safety_blackout_generation = receipt.fence_after.safety_blackout_generation;
        }
        self.bump_state_generation()
    }

    /// Enter a new app session. An unclean previous session or any unresolved
    /// Prepared record is recovered toward Blackout-on before output arm.
    /// Interrupted requests become durable terminals and are never replayed.
    pub(crate) fn begin_session(&mut self) -> Result<usize, DurableR4StateError> {
        self.validate()?;
        let interrupted = self
            .records
            .iter()
            .filter(|record| matches!(&record.record, DurableR4RecordStateV1::Prepared))
            .count();
        if !self.clean_shutdown || interrupted > 0 {
            let mut maximum = (self.safety_blackout_epoch, self.safety_blackout_generation);
            for record in self
                .records
                .iter()
                .filter(|record| matches!(&record.record, DurableR4RecordStateV1::Prepared))
            {
                maximum = maximum.max((
                    record.predicted_applied_fence.safety_blackout_epoch,
                    record.predicted_applied_fence.safety_blackout_generation,
                ));
            }
            let (epoch, generation) = next_timeline_transport_authority(maximum.0, maximum.1)
                .ok_or(DurableR4StateError::AuthorityExhausted)?;
            self.safety_blackout_engaged = true;
            self.safety_blackout_epoch = epoch;
            self.safety_blackout_generation = generation;
            for record in self
                .records
                .iter_mut()
                .filter(|record| matches!(&record.record, DurableR4RecordStateV1::Prepared))
            {
                record.record = DurableR4RecordStateV1::Terminal {
                    response: OutputControlResponseV1::Rejected(OutputControlRejectionV1 {
                        operation_id: record.key.operation_id.clone(),
                        request_id: record.key.request_id,
                        error: OutputControlErrorCodeV1::InterruptedBeforeCommit,
                    }),
                };
            }
        }
        self.clean_shutdown = false;
        self.bump_state_generation()?;
        self.validate()?;
        Ok(interrupted)
    }

    pub(crate) fn finish_session(&mut self) -> Result<(), DurableR4StateError> {
        self.clean_shutdown = true;
        self.bump_state_generation()?;
        self.validate()
    }

    fn bump_state_generation(&mut self) -> Result<(), DurableR4StateError> {
        self.state_generation = self
            .state_generation
            .checked_add(1)
            .filter(|value| *value <= MAX_SAFE_JAVASCRIPT_INTEGER)
            .ok_or(DurableR4StateError::InvalidGeneration)?;
        Ok(())
    }
}

fn validate_nonzero_safe(value: u64) -> Result<(), DurableR4StateError> {
    if value == 0 || value > MAX_SAFE_JAVASCRIPT_INTEGER {
        Err(DurableR4StateError::InvalidGeneration)
    } else {
        Ok(())
    }
}

fn validate_hash(value: &str) -> Result<(), DurableR4StateError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(DurableR4StateError::InvalidHash)
    }
}

fn release_successor(
    before: &OutputControlFenceV1,
) -> Result<OutputControlFenceV1, DurableR4StateError> {
    let (epoch, generation) = next_timeline_transport_authority(
        before.safety_blackout_epoch,
        before.safety_blackout_generation,
    )
    .ok_or(DurableR4StateError::AuthorityExhausted)?;
    let mut after = before.clone();
    after.safety_blackout_epoch = epoch;
    after.safety_blackout_generation = generation;
    Ok(after)
}

fn is_exact_release_successor(before: &OutputControlFenceV1, after: &OutputControlFenceV1) -> bool {
    release_successor(before).is_ok_and(|expected| expected == *after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::control_plane_command::{
        OutputControlReceiptOutcomeV1, OutputControlReceiptV1,
    };

    fn hash(seed: char) -> String {
        std::iter::repeat_n(seed, 64).collect()
    }

    fn fence() -> OutputControlFenceV1 {
        OutputControlFenceV1 {
            process_incarnation: 1,
            session_incarnation: 2,
            project_epoch: 3,
            project_revision: 4,
            project_checkpoint_hash: hash('a'),
            project_publication_generation: 5,
            output_epoch: 6,
            output_generation: 7,
            safety_blackout_epoch: 8,
            safety_blackout_generation: 9,
        }
    }

    fn key(request_id: u64) -> DurableR4RequestKeyV1 {
        DurableR4RequestKeyV1 {
            principal_id: "local-desktop-v1".to_string(),
            operation_id: OUTPUT_BLACKOUT_RELEASE_OPERATION_ID.to_string(),
            request_id,
        }
    }

    #[test]
    fn exact_terminal_survives_serialization_and_shape_conflict_never_reopens() {
        let mut state = DurableR4StateV1::new(true, 8, 9).unwrap();
        let start = fence();
        let shape = hash('b');
        let argument = hash('c');
        assert_eq!(
            state
                .reserve_prepared(key(10), shape.clone(), argument.clone(), start.clone())
                .unwrap(),
            DurableR4Reservation::Reserved
        );
        let after = release_successor(&start).unwrap();
        let response = OutputControlResponseV1::Receipt(OutputControlReceiptV1 {
            operation_id: OUTPUT_BLACKOUT_RELEASE_OPERATION_ID.to_string(),
            request_id: 10,
            shape_sha256: shape.clone(),
            argument_fingerprint: argument.clone(),
            audit_sequence: 1,
            fence_before: start,
            fence_after: after,
            outcome: OutputControlReceiptOutcomeV1::Applied,
        });
        state
            .store_terminal(&key(10), &shape, response.clone())
            .unwrap();
        state.finish_session().unwrap();
        let encoded = serde_json::to_vec(&state).unwrap();
        let mut decoded: DurableR4StateV1 = serde_json::from_slice(&encoded).unwrap();
        decoded.validate().unwrap();
        assert_eq!(
            decoded
                .reserve_prepared(key(10), shape.clone(), argument, fence())
                .unwrap(),
            DurableR4Reservation::ExistingTerminal(response)
        );
        assert_eq!(
            decoded.reserve_prepared(key(10), hash('d'), hash('c'), fence()),
            Err(DurableR4StateError::ShapeConflict)
        );
    }

    #[test]
    fn unclean_prepared_release_recovers_to_strictly_newer_blackout_and_terminalizes() {
        let mut state = DurableR4StateV1::new(true, 8, 9).unwrap();
        let start = fence();
        let predicted = release_successor(&start).unwrap();
        state
            .reserve_prepared(key(11), hash('b'), hash('c'), start)
            .unwrap();
        state.clean_shutdown = false;
        assert_eq!(state.begin_session().unwrap(), 1);
        assert!(state.safety_blackout_engaged);
        assert!(
            (state.safety_blackout_epoch, state.safety_blackout_generation)
                > (
                    predicted.safety_blackout_epoch,
                    predicted.safety_blackout_generation
                )
        );
        let record = state.records.iter().find(|record| record.key == key(11)).unwrap();
        assert!(matches!(
            &record.record,
            DurableR4RecordStateV1::Terminal {
                response: OutputControlResponseV1::Rejected(OutputControlRejectionV1 {
                    error: OutputControlErrorCodeV1::InterruptedBeforeCommit,
                    ..
                })
            }
        ));
    }

    #[test]
    fn durable_store_never_evicts_identity_to_admit_a_new_release() {
        let mut state = DurableR4StateV1::new(true, 1, 1).unwrap();
        for request_id in 1..=MAX_DURABLE_R4_RECORDS as u64 {
            assert_eq!(
                state
                    .reserve_prepared(key(request_id), hash('b'), hash('c'), fence())
                    .unwrap(),
                DurableR4Reservation::Reserved
            );
        }
        assert_eq!(
            state.reserve_prepared(
                key(MAX_DURABLE_R4_RECORDS as u64 + 1),
                hash('b'),
                hash('c'),
                fence(),
            ),
            Err(DurableR4StateError::Capacity)
        );
        assert_eq!(state.records.len(), MAX_DURABLE_R4_RECORDS);
        assert!(state.records.iter().any(|record| record.key == key(1)));
    }

    #[test]
    fn wire_is_strict_and_unknown_fields_fail_closed() {
        let state = DurableR4StateV1::new(true, 1, 1).unwrap();
        let mut json = serde_json::to_value(state).unwrap();
        json.as_object_mut()
            .unwrap()
            .insert("future_field".to_string(), serde_json::json!(true));
        assert!(serde_json::from_value::<DurableR4StateV1>(json).is_err());
    }
}
