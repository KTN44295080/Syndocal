from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def replace_all(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count == 0:
        raise RuntimeError(f"{label}: expected at least one match")
    path.write_text(text.replace(old, new), encoding="utf-8")


protocol = Path("crates/protocol/src/control_plane_command.rs")
main = Path("app/src-tauri/src/main.rs")
controller = Path("app/src/blackoutReleaseRuntimeController.ts")
checker = Path("app/scripts/check-blackout-release-runtime.mjs")
manifest = Path("app/src/tauri-invoke-manifest.json")
invoke_types = Path("app/src/tauriInvokeCommands.ts")

# The production Release workflow must not accept the generic R4 action union at
# the consent-preparation wire. Keep the existing internal generic DTO for later
# Arm/Takeover slices, but expose a Release-only DTO with no action field.
replace_exact(
    protocol,
    '''#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputConsentPrepareRequestV1 {
''',
    '''#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlackoutReleaseConsentPrepareRequestV1 {
    pub operation_id: String,
    pub request_id: u64,
    pub expected_fence: OutputControlFenceV1,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlackoutReleaseConsentPrepareRequestV1Wire {
    operation_id: String,
    request_id: u64,
    expected_fence: OutputControlFenceV1,
}

impl BlackoutReleaseConsentPrepareRequestV1 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        if self.operation_id != OUTPUT_CONSENT_PREPARE_OPERATION_ID {
            return Err(OutputControlValidationErrorV1::UnexpectedOperationId);
        }
        validate_output_request_id(self.request_id)?;
        self.expected_fence.validate()
    }

    pub fn into_output_consent_prepare_request(self) -> OutputConsentPrepareRequestV1 {
        OutputConsentPrepareRequestV1 {
            operation_id: self.operation_id,
            request_id: self.request_id,
            expected_fence: self.expected_fence,
            action: OutputControlActionV1::ReleaseBlackout,
        }
    }

    pub fn argument_fingerprint_bytes(&self) -> Result<Vec<u8>, OutputControlValidationErrorV1> {
        self.validate()?;
        self.clone()
            .into_output_consent_prepare_request()
            .argument_fingerprint_bytes()
    }
}

impl Serialize for BlackoutReleaseConsentPrepareRequestV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        BlackoutReleaseConsentPrepareRequestV1Wire {
            operation_id: self.operation_id.clone(),
            request_id: self.request_id,
            expected_fence: self.expected_fence.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BlackoutReleaseConsentPrepareRequestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BlackoutReleaseConsentPrepareRequestV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            expected_fence: wire.expected_fence,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputConsentPrepareRequestV1 {
''',
    "release-only consent prepare request DTO",
)

# Fold the prepare-wire assertions into the existing exact R4 wire test so the
# focused workflow proves both preparation and execution with one stable test.
replace_exact(
    protocol,
    '''    fn blackout_release_command_wire_is_release_only_and_exactly_bound() {
        let request = BlackoutReleaseCommandRequestV1 {
''',
    '''    fn blackout_release_command_wire_is_release_only_and_exactly_bound() {
        let prepare = BlackoutReleaseConsentPrepareRequestV1 {
            operation_id: OUTPUT_CONSENT_PREPARE_OPERATION_ID.to_string(),
            request_id: 23,
            expected_fence: output_fence(),
        };
        prepare.validate().unwrap();
        let prepare_json = serde_json::to_value(&prepare).unwrap();
        assert_eq!(
            prepare_json,
            serde_json::json!({
                "operation_id": OUTPUT_CONSENT_PREPARE_OPERATION_ID,
                "request_id": 23,
                "expected_fence": output_fence(),
            })
        );
        assert!(prepare_json.get("action").is_none());
        let mut forged_prepare_action = prepare_json.clone();
        forged_prepare_action["action"] =
            serde_json::json!({ "kind": "arm", "role": "both" });
        assert!(
            serde_json::from_value::<BlackoutReleaseConsentPrepareRequestV1>(
                forged_prepare_action,
            )
            .is_err()
        );
        let internal_prepare = prepare.clone().into_output_consent_prepare_request();
        assert_eq!(internal_prepare.action, OutputControlActionV1::ReleaseBlackout);
        assert_eq!(
            prepare.argument_fingerprint_bytes().unwrap(),
            internal_prepare.argument_fingerprint_bytes().unwrap()
        );

        let request = BlackoutReleaseCommandRequestV1 {
''',
    "strict Release consent prepare assertions in exact wire test",
)

replace_exact(
    main,
    '''        AuthoredRequestV1, BlackoutReleaseCommandRequestV1, BlackoutReleaseResponseV1,
        OutputConsentChallengeV1, OutputConsentPrepareRequestV1,
''',
    '''        AuthoredRequestV1, BlackoutReleaseCommandRequestV1,
        BlackoutReleaseConsentPrepareRequestV1, BlackoutReleaseResponseV1,
        OutputConsentChallengeV1, OutputConsentPrepareRequestV1,
''',
    "main release-only consent prepare import",
)
replace_exact(
    main,
    '''#[tauri::command]
fn prepare_output_consent_v1(
    window: WebviewWindow,
    state: State<'_, AppState>,
    query_state: State<'_, ControlPlaneQueryState>,
    request: OutputConsentPrepareRequestV1,
) -> Result<OutputConsentChallengeV1, String> {
    control_plane_runtime::prepare_output_consent(&window, &state, &query_state, request)
}
''',
    '''#[tauri::command]
fn prepare_blackout_release_consent_v1(
    window: WebviewWindow,
    state: State<'_, AppState>,
    query_state: State<'_, ControlPlaneQueryState>,
    request: BlackoutReleaseConsentPrepareRequestV1,
) -> Result<OutputConsentChallengeV1, String> {
    request
        .validate()
        .map_err(|_| "Blackout Release consent request is invalid".to_string())?;
    control_plane_runtime::prepare_output_consent(
        &window,
        &state,
        &query_state,
        request.into_output_consent_prepare_request(),
    )
}
''',
    "dedicated Release consent prepare Tauri source",
)
replace_exact(
    main,
    '''            prepare_output_consent_v1,
''',
    '''            prepare_blackout_release_consent_v1,
''',
    "register dedicated Release consent prepare source",
)

replace_all(
    controller,
    '"prepare_output_consent_v1"',
    '"prepare_blackout_release_consent_v1"',
    "renderer dedicated Release consent prepare source",
)
replace_exact(
    controller,
    '''          expected_fence: authority.fence,
          action: { kind: "release_blackout" },
''',
    '''          expected_fence: authority.fence,
''',
    "renderer Release consent prepare has no action field",
)

replace_all(
    checker,
    '"prepare_output_consent_v1"',
    '"prepare_blackout_release_consent_v1"',
    "renderer harness dedicated Release consent prepare source",
)
replace_exact(
    checker,
    '''        assert.deepEqual(request.expected_fence, fence);
        assert.deepEqual(request.action, { kind: "release_blackout" });
''',
    '''        assert.deepEqual(request.expected_fence, fence);
        assert.equal("action" in request, false);
''',
    "harness rejects action field on Release consent prepare wire",
)

replace_all(
    manifest,
    '"prepare_output_consent_v1"',
    '"prepare_blackout_release_consent_v1"',
    "frontend manifest dedicated Release consent prepare source",
)
replace_all(
    invoke_types,
    '"prepare_output_consent_v1"',
    '"prepare_blackout_release_consent_v1"',
    "frontend type dedicated Release consent prepare source",
)
