from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


protocol = Path("crates/protocol/src/control_plane_command.rs")
main = Path("app/src-tauri/src/main.rs")

replace_exact(
    protocol,
    '''#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputConsentChallengeV1 {
''',
    '''#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlackoutReleaseConsentChallengeV1 {
    pub operation_id: String,
    pub request_id: u64,
    pub target_operation_id: String,
    pub challenge_id: String,
    pub consent_token: String,
    pub display_code: String,
    pub argument_fingerprint: String,
    pub expires_at_unix_ms: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlackoutReleaseConsentChallengeV1Wire {
    operation_id: String,
    request_id: u64,
    target_operation_id: String,
    challenge_id: String,
    consent_token: String,
    display_code: String,
    argument_fingerprint: String,
    expires_at_unix_ms: u64,
}

impl BlackoutReleaseConsentChallengeV1 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        if self.operation_id != OUTPUT_CONSENT_PREPARE_OPERATION_ID
            || self.target_operation_id != OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
        {
            return Err(OutputControlValidationErrorV1::UnexpectedOperationId);
        }
        validate_output_request_id(self.request_id)?;
        validate_opaque_output_id(&self.challenge_id)?;
        validate_opaque_output_id(&self.consent_token)?;
        validate_lower_hex_fingerprint(&self.argument_fingerprint)?;
        if self.display_code.len() != 6
            || !self.display_code.bytes().all(|byte| byte.is_ascii_digit())
            || self.expires_at_unix_ms == 0
            || self.expires_at_unix_ms > MAX_SAFE_JAVASCRIPT_INTEGER
        {
            return Err(OutputControlValidationErrorV1::InvalidOpaqueId);
        }
        Ok(())
    }
}

impl TryFrom<OutputConsentChallengeV1> for BlackoutReleaseConsentChallengeV1 {
    type Error = OutputControlValidationErrorV1;

    fn try_from(value: OutputConsentChallengeV1) -> Result<Self, Self::Error> {
        let value = Self {
            operation_id: value.operation_id,
            request_id: value.request_id,
            target_operation_id: value.target_operation_id,
            challenge_id: value.challenge_id,
            consent_token: value.consent_token,
            display_code: value.display_code,
            argument_fingerprint: value.argument_fingerprint,
            expires_at_unix_ms: value.expires_at_unix_ms,
        };
        value.validate()?;
        Ok(value)
    }
}

impl Serialize for BlackoutReleaseConsentChallengeV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        BlackoutReleaseConsentChallengeV1Wire {
            operation_id: self.operation_id.clone(),
            request_id: self.request_id,
            target_operation_id: self.target_operation_id.clone(),
            challenge_id: self.challenge_id.clone(),
            consent_token: self.consent_token.clone(),
            display_code: self.display_code.clone(),
            argument_fingerprint: self.argument_fingerprint.clone(),
            expires_at_unix_ms: self.expires_at_unix_ms,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BlackoutReleaseConsentChallengeV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BlackoutReleaseConsentChallengeV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            target_operation_id: wire.target_operation_id,
            challenge_id: wire.challenge_id,
            consent_token: wire.consent_token,
            display_code: wire.display_code,
            argument_fingerprint: wire.argument_fingerprint,
            expires_at_unix_ms: wire.expires_at_unix_ms,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputConsentChallengeV1 {
''',
    "Release-only consent challenge response DTO",
)

replace_exact(
    protocol,
    '''        let internal_prepare = prepare.clone().into_output_consent_prepare_request();
        assert_eq!(internal_prepare.action, OutputControlActionV1::ReleaseBlackout);
        assert_eq!(
            prepare.argument_fingerprint_bytes().unwrap(),
            internal_prepare.argument_fingerprint_bytes().unwrap()
        );

        let request = BlackoutReleaseCommandRequestV1 {
''',
    '''        let internal_prepare = prepare.clone().into_output_consent_prepare_request();
        assert_eq!(internal_prepare.action, OutputControlActionV1::ReleaseBlackout);
        assert_eq!(
            prepare.argument_fingerprint_bytes().unwrap(),
            internal_prepare.argument_fingerprint_bytes().unwrap()
        );
        let release_challenge = BlackoutReleaseConsentChallengeV1 {
            operation_id: OUTPUT_CONSENT_PREPARE_OPERATION_ID.to_string(),
            request_id: 23,
            target_operation_id: OUTPUT_BLACKOUT_RELEASE_OPERATION_ID.to_string(),
            challenge_id: "AAAAAAAAAAAAAAAAAAAAAA".to_string(),
            consent_token: "AAAAAAAAAAAAAAAAAAAAAA".to_string(),
            display_code: "123456".to_string(),
            argument_fingerprint: hash('c'),
            expires_at_unix_ms: 1000,
        };
        let challenge_json = serde_json::to_value(&release_challenge).unwrap();
        assert_eq!(
            serde_json::from_value::<BlackoutReleaseConsentChallengeV1>(challenge_json.clone())
                .unwrap(),
            release_challenge
        );
        let mut forged_target = challenge_json;
        forged_target["target_operation_id"] = serde_json::json!(OUTPUT_OWNERSHIP_ARM_OPERATION_ID);
        assert!(
            serde_json::from_value::<BlackoutReleaseConsentChallengeV1>(forged_target).is_err()
        );

        let request = BlackoutReleaseCommandRequestV1 {
''',
    "strict Release consent challenge assertions",
)

replace_exact(
    main,
    '''        AuthoredRequestV1, BlackoutReleaseCommandRequestV1,
        BlackoutReleaseConsentPrepareRequestV1, BlackoutReleaseResponseV1,
        OutputConsentChallengeV1, OutputConsentPrepareRequestV1,
''',
    '''        AuthoredRequestV1, BlackoutReleaseCommandRequestV1,
        BlackoutReleaseConsentChallengeV1, BlackoutReleaseConsentPrepareRequestV1,
        BlackoutReleaseResponseV1, OutputConsentChallengeV1, OutputConsentPrepareRequestV1,
''',
    "main Release-only consent challenge import",
)
replace_exact(
    main,
    '''fn prepare_blackout_release_consent_v1(
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
    '''fn prepare_blackout_release_consent_v1(
    window: WebviewWindow,
    state: State<'_, AppState>,
    query_state: State<'_, ControlPlaneQueryState>,
    request: BlackoutReleaseConsentPrepareRequestV1,
) -> Result<BlackoutReleaseConsentChallengeV1, String> {
    request
        .validate()
        .map_err(|_| "Blackout Release consent request is invalid".to_string())?;
    let challenge = control_plane_runtime::prepare_output_consent(
        &window,
        &state,
        &query_state,
        request.into_output_consent_prepare_request(),
    )?;
    BlackoutReleaseConsentChallengeV1::try_from(challenge)
        .map_err(|_| "Blackout Release consent challenge is invalid".to_string())
}
''',
    "dedicated Release consent challenge result type",
)
