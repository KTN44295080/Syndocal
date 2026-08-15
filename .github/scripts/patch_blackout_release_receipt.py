from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    old_count = text.count(old)
    new_count = text.count(new)
    if old_count != 1 or new_count != 0:
        raise RuntimeError(
            f"{label}: expected old=1/new=0, found old={old_count}/new={new_count}"
        )
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def replace_all(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    old_count = text.count(old)
    new_count = text.count(new)
    if old_count == 0 or new_count != 0:
        raise RuntimeError(
            f"{label}: expected old>0/new=0, found old={old_count}/new={new_count}"
        )
    path.write_text(text.replace(old, new), encoding="utf-8")


protocol = Path("crates/protocol/src/control_plane_command.rs")
controller = Path("app/src/blackoutReleaseRuntimeController.ts")
checker = Path("app/scripts/check-blackout-release-runtime.mjs")
main_rs = Path("app/src-tauri/src/main.rs")
registry = Path("app/src-tauri/src/control_plane.rs")
invoke_manifest = Path("app/src/tauri-invoke-manifest.json")
invoke_types = Path("app/src/tauriInvokeCommands.ts")

replace_exact(
    protocol,
    '''        match self.outcome {
            OutputControlReceiptOutcomeV1::NoOp if self.fence_before != self.fence_after => {
                Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
            }
            OutputControlReceiptOutcomeV1::Applied if self.fence_before == self.fence_after => {
                Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
            }
            _ => Ok(()),
        }
''',
    '''        match self.outcome {
            OutputControlReceiptOutcomeV1::NoOp if self.fence_before != self.fence_after => {
                Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
            }
            OutputControlReceiptOutcomeV1::Applied
                if self.operation_id == OUTPUT_BLACKOUT_RELEASE_OPERATION_ID =>
            {
                validate_blackout_release_applied_fence_transition(
                    &self.fence_before,
                    &self.fence_after,
                )
            }
            OutputControlReceiptOutcomeV1::Applied if self.fence_before == self.fence_after => {
                Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
            }
            _ => Ok(()),
        }
''',
    "protocol receipt outcome validation",
)

replace_exact(
    protocol,
    '''fn validate_output_request_id(request_id: u64) -> Result<(), OutputControlValidationErrorV1> {
''',
    '''fn validate_blackout_release_applied_fence_transition(
    before: &OutputControlFenceV1,
    after: &OutputControlFenceV1,
) -> Result<(), OutputControlValidationErrorV1> {
    let Some((safety_blackout_epoch, safety_blackout_generation)) =
        next_timeline_transport_authority(
            before.safety_blackout_epoch,
            before.safety_blackout_generation,
        )
    else {
        return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
    };
    if after.process_incarnation == before.process_incarnation
        && after.session_incarnation == before.session_incarnation
        && after.project_epoch == before.project_epoch
        && after.project_revision == before.project_revision
        && after.project_checkpoint_hash == before.project_checkpoint_hash
        && after.project_publication_generation == before.project_publication_generation
        && after.output_epoch == before.output_epoch
        && after.output_generation == before.output_generation
        && after.safety_blackout_epoch == safety_blackout_epoch
        && after.safety_blackout_generation == safety_blackout_generation
    {
        Ok(())
    } else {
        Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
    }
}

fn validate_output_request_id(request_id: u64) -> Result<(), OutputControlValidationErrorV1> {
''',
    "protocol exact blackout release transition helper",
)

replace_exact(
    protocol,
    '''#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputControlCommandRequestV1 {
''',
    '''/// Exact external wire accepted by the canonical R4 Blackout Release source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlackoutReleaseCommandRequestV1 {
    pub operation_id: String,
    pub request_id: u64,
    pub expected_fence: OutputControlFenceV1,
    pub consent_token: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlackoutReleaseCommandRequestV1Wire {
    operation_id: String,
    request_id: u64,
    expected_fence: OutputControlFenceV1,
    consent_token: String,
}

impl BlackoutReleaseCommandRequestV1 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        if self.operation_id != OUTPUT_BLACKOUT_RELEASE_OPERATION_ID {
            return Err(OutputControlValidationErrorV1::UnexpectedOperationId);
        }
        validate_output_request_id(self.request_id)?;
        self.expected_fence.validate()?;
        validate_opaque_output_id(&self.consent_token)
    }

    pub fn into_output_control_request(self) -> OutputControlCommandRequestV1 {
        OutputControlCommandRequestV1 {
            operation_id: self.operation_id,
            request_id: self.request_id,
            expected_fence: self.expected_fence,
            consent_token: self.consent_token,
            action: OutputControlActionV1::ReleaseBlackout,
        }
    }

    pub fn canonical_shape_bytes(&self) -> Result<Vec<u8>, OutputControlValidationErrorV1> {
        self.validate()?;
        self.clone()
            .into_output_control_request()
            .canonical_shape_bytes()
    }

    pub fn argument_fingerprint_bytes(&self) -> Result<Vec<u8>, OutputControlValidationErrorV1> {
        self.validate()?;
        self.clone()
            .into_output_control_request()
            .argument_fingerprint_bytes()
    }
}

impl Serialize for BlackoutReleaseCommandRequestV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        BlackoutReleaseCommandRequestV1Wire {
            operation_id: self.operation_id.clone(),
            request_id: self.request_id,
            expected_fence: self.expected_fence.clone(),
            consent_token: self.consent_token.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BlackoutReleaseCommandRequestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BlackoutReleaseCommandRequestV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            expected_fence: wire.expected_fence,
            consent_token: wire.consent_token,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputControlCommandRequestV1 {
''',
    "release-only request wire",
)

replace_exact(
    protocol,
    '''fn validate_blackout_release_applied_fence_transition(
''',
    '''/// Exact terminal wire for the canonical R4 Blackout Release source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlackoutReleaseResponseV1 {
    Receipt(OutputControlReceiptV1),
    Rejected(OutputControlRejectionV1),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum BlackoutReleaseResponseV1Wire {
    Receipt { receipt: OutputControlReceiptV1 },
    Rejected { rejection: OutputControlRejectionV1 },
}

impl BlackoutReleaseResponseV1 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        match self {
            Self::Receipt(receipt) => {
                if receipt.operation_id != OUTPUT_BLACKOUT_RELEASE_OPERATION_ID {
                    return Err(OutputControlValidationErrorV1::UnexpectedOperationId);
                }
                receipt.validate()
            }
            Self::Rejected(rejection) => {
                if rejection.operation_id != OUTPUT_BLACKOUT_RELEASE_OPERATION_ID {
                    return Err(OutputControlValidationErrorV1::UnexpectedOperationId);
                }
                rejection.validate()
            }
        }
    }
}

impl From<OutputControlResponseV1> for BlackoutReleaseResponseV1 {
    fn from(value: OutputControlResponseV1) -> Self {
        match value {
            OutputControlResponseV1::Receipt(receipt) => Self::Receipt(receipt),
            OutputControlResponseV1::Rejected(rejection) => Self::Rejected(rejection),
        }
    }
}

impl Serialize for BlackoutReleaseResponseV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        match self {
            Self::Receipt(receipt) => BlackoutReleaseResponseV1Wire::Receipt {
                receipt: receipt.clone(),
            },
            Self::Rejected(rejection) => BlackoutReleaseResponseV1Wire::Rejected {
                rejection: rejection.clone(),
            },
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BlackoutReleaseResponseV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BlackoutReleaseResponseV1Wire::deserialize(deserializer)?;
        let value = match wire {
            BlackoutReleaseResponseV1Wire::Receipt { receipt } => Self::Receipt(receipt),
            BlackoutReleaseResponseV1Wire::Rejected { rejection } => Self::Rejected(rejection),
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

fn validate_blackout_release_applied_fence_transition(
''',
    "release-only terminal response wire",
)

replace_exact(
    protocol,
    '''        let mut invalid_noop = match response {
            OutputControlResponseV1::Receipt(receipt) => receipt,
            OutputControlResponseV1::Rejected(_) => unreachable!(),
        };
        invalid_noop.outcome = OutputControlReceiptOutcomeV1::NoOp;
        assert!(serde_json::to_value(invalid_noop).is_err());
''',
    '''        let mut invalid_noop = match response.clone() {
            OutputControlResponseV1::Receipt(receipt) => receipt,
            OutputControlResponseV1::Rejected(_) => unreachable!(),
        };
        invalid_noop.outcome = OutputControlReceiptOutcomeV1::NoOp;
        assert!(serde_json::to_value(invalid_noop).is_err());

        let mut output_drift = match response.clone() {
            OutputControlResponseV1::Receipt(receipt) => receipt,
            OutputControlResponseV1::Rejected(_) => unreachable!(),
        };
        output_drift.fence_after = output_drift.fence_before.clone();
        output_drift.fence_after.output_generation += 1;
        assert!(serde_json::to_value(output_drift).is_err());

        let mut skipped_safety_authority = match response {
            OutputControlResponseV1::Receipt(receipt) => receipt,
            OutputControlResponseV1::Rejected(_) => unreachable!(),
        };
        skipped_safety_authority.fence_after.safety_blackout_generation += 1;
        assert!(serde_json::to_value(skipped_safety_authority).is_err());
''',
    "protocol forged release receipt tests",
)

replace_exact(
    protocol,
    '''    #[test]
    fn output_consent_and_r4_command_wire_are_strict_and_exactly_bound() {
''',
    '''    #[test]
    fn blackout_release_command_wire_is_release_only_and_exactly_bound() {
        let request = BlackoutReleaseCommandRequestV1 {
            operation_id: OUTPUT_BLACKOUT_RELEASE_OPERATION_ID.to_string(),
            request_id: 24,
            expected_fence: output_fence(),
            consent_token: "AAAAAAAAAAAAAAAAAAAAAA".to_string(),
        };
        request.validate().unwrap();
        let encoded = serde_json::to_value(&request).unwrap();
        assert_eq!(
            encoded,
            serde_json::json!({
                "operation_id": OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
                "request_id": 24,
                "expected_fence": output_fence(),
                "consent_token": "AAAAAAAAAAAAAAAAAAAAAA",
            })
        );
        assert_eq!(
            serde_json::from_value::<BlackoutReleaseCommandRequestV1>(encoded.clone()).unwrap(),
            request
        );
        assert!(encoded.get("action").is_none());

        let mut forged_action = encoded.clone();
        forged_action["action"] = serde_json::json!({ "kind": "arm", "role": "both" });
        assert!(
            serde_json::from_value::<BlackoutReleaseCommandRequestV1>(forged_action).is_err()
        );
        for operation_id in [
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
            OUTPUT_STANDBY_TAKEOVER_OPERATION_ID,
        ] {
            let mut forged_operation = encoded.clone();
            forged_operation["operation_id"] = serde_json::json!(operation_id);
            assert!(
                serde_json::from_value::<BlackoutReleaseCommandRequestV1>(forged_operation)
                    .is_err()
            );
            let mut outbound = request.clone();
            outbound.operation_id = operation_id.to_string();
            assert!(serde_json::to_value(outbound).is_err());
        }

        let internal = request.clone().into_output_control_request();
        assert_eq!(internal.action, OutputControlActionV1::ReleaseBlackout);
        assert_eq!(
            request.canonical_shape_bytes().unwrap(),
            internal.canonical_shape_bytes().unwrap()
        );
        assert_eq!(
            request.argument_fingerprint_bytes().unwrap(),
            internal.argument_fingerprint_bytes().unwrap()
        );

        let fence_before = output_fence();
        let mut fence_after = fence_before.clone();
        fence_after.safety_blackout_generation += 1;
        let release_response =
            BlackoutReleaseResponseV1::from(OutputControlResponseV1::Receipt(
                OutputControlReceiptV1 {
                    operation_id: OUTPUT_BLACKOUT_RELEASE_OPERATION_ID.to_string(),
                    request_id: request.request_id,
                    shape_sha256: hash('d'),
                    argument_fingerprint: hash('e'),
                    audit_sequence: 1,
                    fence_before,
                    fence_after,
                    outcome: OutputControlReceiptOutcomeV1::Applied,
                },
            ));
        let release_json = serde_json::to_value(&release_response).unwrap();
        assert_eq!(
            serde_json::from_value::<BlackoutReleaseResponseV1>(release_json).unwrap(),
            release_response
        );

        let wrong_response =
            BlackoutReleaseResponseV1::from(OutputControlResponseV1::Rejected(
                OutputControlRejectionV1 {
                    operation_id: OUTPUT_OWNERSHIP_ARM_OPERATION_ID.to_string(),
                    request_id: request.request_id,
                    error: OutputControlErrorCodeV1::ConsentPending,
                },
            ));
        assert!(serde_json::to_value(wrong_response).is_err());
    }

    #[test]
    fn output_control_generic_wire_is_strict_and_exactly_bound() {
''',
    "release-only protocol test",
)

replace_exact(
    controller,
    '''const sameFence = (left: OutputControlFence, right: OutputControlFence): boolean =>
  left.process_incarnation === right.process_incarnation
  && left.session_incarnation === right.session_incarnation
  && left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.project_checkpoint_hash === right.project_checkpoint_hash
  && left.project_publication_generation === right.project_publication_generation
  && left.output_epoch === right.output_epoch
  && left.output_generation === right.output_generation
  && left.safety_blackout_epoch === right.safety_blackout_epoch
  && left.safety_blackout_generation === right.safety_blackout_generation;
''',
    '''const sameFence = (left: OutputControlFence, right: OutputControlFence): boolean =>
  left.process_incarnation === right.process_incarnation
  && left.session_incarnation === right.session_incarnation
  && left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.project_checkpoint_hash === right.project_checkpoint_hash
  && left.project_publication_generation === right.project_publication_generation
  && left.output_epoch === right.output_epoch
  && left.output_generation === right.output_generation
  && left.safety_blackout_epoch === right.safety_blackout_epoch
  && left.safety_blackout_generation === right.safety_blackout_generation;

const nextAuthorityPair = (epoch: number, generation: number): readonly [number, number] | null => {
  if (generation < Number.MAX_SAFE_INTEGER) return [epoch, generation + 1] as const;
  if (epoch < Number.MAX_SAFE_INTEGER) return [epoch + 1, 1] as const;
  return null;
};

const isExactBlackoutReleaseAppliedTransition = (
  before: OutputControlFence,
  after: OutputControlFence,
): boolean => {
  const next = nextAuthorityPair(
    before.safety_blackout_epoch,
    before.safety_blackout_generation,
  );
  return next !== null
    && after.process_incarnation === before.process_incarnation
    && after.session_incarnation === before.session_incarnation
    && after.project_epoch === before.project_epoch
    && after.project_revision === before.project_revision
    && after.project_checkpoint_hash === before.project_checkpoint_hash
    && after.project_publication_generation === before.project_publication_generation
    && after.output_epoch === before.output_epoch
    && after.output_generation === before.output_generation
    && after.safety_blackout_epoch === next[0]
    && after.safety_blackout_generation === next[1];
};
''',
    "renderer exact blackout release transition helper",
)

replace_exact(
    controller,
    '''  if ((receipt.outcome === "no_op" && !sameFence(receipt.fence_before, receipt.fence_after))
    || (receipt.outcome === "applied" && sameFence(receipt.fence_before, receipt.fence_after))) {
    throw new BlackoutReleaseProtocolError("receipt fence transition");
  }
''',
    '''  if ((receipt.outcome === "no_op" && !sameFence(receipt.fence_before, receipt.fence_after))
    || (receipt.outcome === "applied"
      && !isExactBlackoutReleaseAppliedTransition(receipt.fence_before, receipt.fence_after))) {
    throw new BlackoutReleaseProtocolError("receipt fence transition");
  }
''',
    "renderer receipt transition validation",
)

replace_all(
    controller,
    '"execute_output_control_v1"',
    '"execute_blackout_release_v1"',
    "renderer dedicated execute source",
)

replace_exact(
    controller,
    '''      consent_token: prepared.challenge.consent_token,
      action: { kind: "release_blackout" as const },
''',
    '''      consent_token: prepared.challenge.consent_token,
''',
    "renderer release-only request shape",
)

replace_exact(
    checker,
    '''        assert.equal(request.consent_token, opaqueId);
        assert.deepEqual(request.action, { kind: "release_blackout" });
        if (executeRequests.length === 1) {
''',
    '''        assert.equal(request.consent_token, opaqueId);
        assert.equal("action" in request, false);
        if (executeRequests.length === 1) {
''',
    "renderer harness release-only request",
)

replace_all(
    checker,
    '"execute_output_control_v1"',
    '"execute_blackout_release_v1"',
    "renderer harness dedicated execute source",
)

replace_exact(
    checker,
    '''await assert.rejects(mismatchedReceipt.release(prepared), /invalid terminal receipt/);
assert.equal(mismatchedReceiptCalls, 1);

const nonCanonicalChallenge = runtime.createBlackoutReleaseRuntimeController({
''',
    '''await assert.rejects(mismatchedReceipt.release(prepared), /invalid terminal receipt/);
assert.equal(mismatchedReceiptCalls, 1);

let forgedTransitionCalls = 0;
const forgedTransition = runtime.createBlackoutReleaseRuntimeController({
  invoke: async (command, args) => {
    assert.equal(command, "execute_blackout_release_v1");
    forgedTransitionCalls += 1;
    return {
      type: "receipt",
      receipt: {
        operation_id: runtime.blackoutReleaseOperationId,
        request_id: args.request.request_id,
        shape_sha256: hash("c"),
        argument_fingerprint: fingerprint,
        audit_sequence: 3,
        fence_before: fence,
        fence_after: {
          ...fence,
          output_generation: fence.output_generation + 1,
        },
        outcome: "applied",
      },
    };
  },
});
await assert.rejects(forgedTransition.release(prepared), /invalid receipt fence transition/);
assert.equal(forgedTransitionCalls, 1);

const nonCanonicalChallenge = runtime.createBlackoutReleaseRuntimeController({
''',
    "renderer forged transition harness",
)

replace_exact(
    main_rs,
    '''        AuthoredRequestV1, OutputConsentChallengeV1, OutputConsentPrepareRequestV1,
''',
    '''        AuthoredRequestV1, BlackoutReleaseCommandRequestV1, BlackoutReleaseResponseV1,
        OutputConsentChallengeV1, OutputConsentPrepareRequestV1,
''',
    "main release-only wire imports",
)

replace_exact(
    main_rs,
    '''#[tauri::command]
fn execute_output_control_v1(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    query_state: State<'_, ControlPlaneQueryState>,
    request: OutputControlCommandRequestV1,
) -> OutputControlResponseV1 {
    control_plane_runtime::execute_output_control(&app, &window, &state, &query_state, request)
}
''',
    '''#[tauri::command]
fn execute_blackout_release_v1(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    query_state: State<'_, ControlPlaneQueryState>,
    request: BlackoutReleaseCommandRequestV1,
) -> BlackoutReleaseResponseV1 {
    control_plane_runtime::execute_output_control(
        &app,
        &window,
        &state,
        &query_state,
        request.into_output_control_request(),
    )
    .into()
}
''',
    "main dedicated blackout release command",
)

replace_exact(
    main_rs,
    '''            execute_output_control_v1,
''',
    '''            execute_blackout_release_v1,
''',
    "main handler dedicated blackout release source",
)

replace_all(
    registry,
    '"execute_output_control_v1"',
    '"execute_blackout_release_v1"',
    "canonical registry dedicated blackout release source",
)

replace_exact(
    registry,
    '''        const ENGINE_COMMAND_COUNT: usize = 249;
''',
    '''        const ENGINE_COMMAND_COUNT: usize = 250;
''',
    "registry engine inventory count",
)
replace_exact(
    registry,
    '''        assert_eq!(registry.operations.len(), 1414);
''',
    '''        assert_eq!(registry.operations.len(), 1415);
''',
    "registry total inventory count",
)
replace_exact(
    registry,
    '''        const ENGINE_COUNT: usize = 249;
''',
    '''        const ENGINE_COUNT: usize = 250;
''',
    "canonical engine source count",
)
replace_exact(
    registry,
    '''        assert_eq!(LEGACY_SOURCE_TOTAL, 1414);
        assert_eq!(SOURCE_TOTAL, 1447);
''',
    '''        assert_eq!(LEGACY_SOURCE_TOTAL, 1415);
        assert_eq!(SOURCE_TOTAL, 1448);
''',
    "canonical total source counts",
)
replace_exact(
    registry,
    '''        assert_eq!(unclassified.len(), 1033);
''',
    '''        assert_eq!(unclassified.len(), 1034);
''',
    "canonical unclassified source count",
)

replace_all(
    invoke_manifest,
    '"execute_output_control_v1"',
    '"execute_blackout_release_v1"',
    "frontend manifest dedicated blackout release source",
)
replace_all(
    invoke_types,
    '"execute_output_control_v1"',
    '"execute_blackout_release_v1"',
    "frontend invoke type dedicated blackout release source",
)
