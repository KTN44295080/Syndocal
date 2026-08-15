from __future__ import annotations

import json
import re
from pathlib import Path


def read(path: str) -> str:
    return Path(path).read_text(encoding="utf-8")


def write(path: str, text: str) -> None:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8", newline="\n")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


def replace_count(text: str, old: str, new: str, expected: int, label: str) -> str:
    count = text.count(old)
    if count != expected:
        raise RuntimeError(f"{label}: expected {expected} matches, found {count}")
    return text.replace(old, new)


registry_path = "crates/protocol/src/control_plane_registry_v2.rs"
registry = read(registry_path)
registry = replace_once(
    registry,
    '''    /// The named S0 emergency blackout engagement. Its request has no target
    /// value, requires immutable audit, and is never a release capability.
    LocalWindowEmergencySafetyMutation,
''',
    '''    /// A typed local R4 output-disruptive mutation. It is not available
    /// during Full Lock and requires an exact terminal receipt plus backend
    /// physical, single-use consent bound to the complete output fence.
    LocalWindowOutputDisruptiveMutation,
    /// The named S0 emergency blackout engagement. Its request has no target
    /// value, requires immutable audit, and is never a release capability.
    LocalWindowEmergencySafetyMutation,
''',
    "adapter policy variant",
)
registry = replace_once(
    registry,
    '''pub enum ConsentPolicy {
    FailClosed,
    /// S0 does not consume an R4/R5 confirmation token; authority is the
''',
    '''pub enum ConsentPolicy {
    FailClosed,
    /// Backend-issued, physical-input-confirmed consent. The token is bound to
    /// one caller, operation, argument fingerprint and authority fence, and
    /// expires no later than fifteen seconds after preparation.
    PhysicalSingleUse15Seconds,
    /// S0 does not consume an R4/R5 confirmation token; authority is the
''',
    "consent policy variant",
)
release_validation = '''            AdapterPolicy::LocalWindowOutputDisruptiveMutation => {
                if self.class != OperationClass::Mutation
                    || self.risk != OperationRisk::R4
                    || self.idempotency != OperationIdempotency::Mutating
                    || self.audit != OperationAuditRequirement::Immutable
                    || self.capabilities
                        != vec![
                            OperationCapability::LocalWindowBound,
                            OperationCapability::OutputBlackoutRelease,
                        ]
                    || self.receipt_policy != ReceiptPolicy::ExactTerminalReceipt
                    || self.rate_policy != RatePolicy::TokenBucket4PerSecondBurst8
                    || self.payload_policy != PayloadPolicy::FailClosed
                    || self.consent_policy != ConsentPolicy::PhysicalSingleUse15Seconds
                {
                    return Err(
                        CanonicalRegistryValidationError::UnsafeCanonicalLocalOperation(
                            self.operation_id.clone(),
                        ),
                    );
                }
                for adapter in &self.derived_adapters {
                    adapter.validate_shape()?;
                    if adapter.adapter != AdapterKind::LocalTauriWindow {
                        return Err(
                            CanonicalRegistryValidationError::UnsupportedAdapterForPolicy(
                                self.operation_id.clone(),
                            ),
                        );
                    }
                }
            }
'''
registry = replace_once(
    registry,
    "            AdapterPolicy::LocalWindowEmergencySafetyMutation => {\n",
    release_validation + "            AdapterPolicy::LocalWindowEmergencySafetyMutation => {\n",
    "R4 metadata validation",
)
registry = replace_count(
    registry,
    "                | AdapterPolicy::LocalWindowEmergencySafetyMutation\n",
    "                | AdapterPolicy::LocalWindowOutputDisruptiveMutation\n"
    "                | AdapterPolicy::LocalWindowEmergencySafetyMutation\n",
    3,
    "adapter allowlists",
)
release_policy_test = '''    #[test]
    fn output_disruptive_release_requires_r4_physical_consent_receipt_rate_and_no_full_lock() {
        let operation_id = "syndocal.output.blackout.release.v1";
        let mut release = authoritative_mutation_operation();
        release.operation_id = operation_id.to_string();
        release.request_schema = SchemaIdentity {
            name: format!("{operation_id}.request"),
            version: 1,
        };
        release.response_schema = SchemaIdentity {
            name: format!("{operation_id}.response"),
            version: 1,
        };
        release.risk = OperationRisk::R4;
        release.capabilities = vec![
            OperationCapability::LocalWindowBound,
            OperationCapability::OutputBlackoutRelease,
        ];
        release.audit = OperationAuditRequirement::Immutable;
        release.adapter_policy = AdapterPolicy::LocalWindowOutputDisruptiveMutation;
        release.rate_policy = RatePolicy::TokenBucket4PerSecondBurst8;
        release.consent_policy = ConsentPolicy::PhysicalSingleUse15Seconds;
        let source_key = SourceKey::new(
            CanonicalSourceFamily::TauriCommand,
            "execute_output_control_v1",
        );
        release.derived_adapters.push(DerivedAdapterBinding {
            adapter: AdapterKind::LocalTauriWindow,
            binding_id: source_key.binding_id(),
            source_key,
        });
        release.validate().unwrap();

        for forged in [
            {
                let mut value = release.clone();
                value.risk = OperationRisk::R0;
                value
            },
            {
                let mut value = release.clone();
                value.capabilities[1] = OperationCapability::AuthoritativeRuntimeMutation;
                value
            },
            {
                let mut value = release.clone();
                value.capabilities.push(OperationCapability::AllowedDuringFullLock);
                value
            },
            {
                let mut value = release.clone();
                value.audit = OperationAuditRequirement::NotApplicable;
                value
            },
            {
                let mut value = release.clone();
                value.rate_policy = RatePolicy::FailClosed;
                value
            },
            {
                let mut value = release.clone();
                value.consent_policy = ConsentPolicy::FailClosed;
                value
            },
        ] {
            assert!(matches!(
                forged.validate(),
                Err(CanonicalRegistryValidationError::UnsafeCanonicalLocalOperation(_))
            ));
        }
        let mut external = release;
        external.derived_adapters[0].adapter = AdapterKind::ExternalMcp;
        assert!(matches!(
            external.validate(),
            Err(CanonicalRegistryValidationError::UnsupportedAdapterForPolicy(_))
        ));
    }

'''
registry = replace_once(
    registry,
    "    #[test]\n    fn emergency_s0_requires_named_capability_immutable_audit_and_no_consent() {\n",
    release_policy_test
    + "    #[test]\n    fn emergency_s0_requires_named_capability_immutable_audit_and_no_consent() {\n",
    "R4 policy test",
)
write(registry_path, registry)

security_path = "app/src-tauri/src/control_plane_security.rs"
security = read(security_path)
security = replace_once(
    security,
    "const CONSENT_CHALLENGE_TTL: Duration = Duration::from_secs(30);",
    "const CONSENT_CHALLENGE_TTL: Duration = Duration::from_secs(15);",
    "consent TTL",
)
write(security_path, security)

control_path = "app/src-tauri/src/control_plane.rs"
control = read(control_path)
control = replace_once(
    control,
    '''use protocol::control_plane_command::{
    SAFETY_BLACKOUT_ENGAGE_OPERATION_ID, SET_EFFECT_ENABLED_OPERATION_ID,
''',
    '''use protocol::control_plane_command::{
    OUTPUT_BLACKOUT_RELEASE_OPERATION_ID, SAFETY_BLACKOUT_ENGAGE_OPERATION_ID,
    SET_EFFECT_ENABLED_OPERATION_ID,
''',
    "release operation import",
)
control = replace_once(
    control,
    '''        CanonicalSourceFamily::EngineCommand
            if descriptor.source_id == "safety_blackout_engage_published" =>
        {
            SourceDisposition::InternalStepOf {
                target: SourceKey::new(
                    CanonicalSourceFamily::TauriCommand,
                    "safety_blackout_engage_v1",
                ),
            }
        }
        _ => SourceDisposition::Unclassified {
''',
    '''        CanonicalSourceFamily::EngineCommand
            if descriptor.source_id == "safety_blackout_engage_published" =>
        {
            SourceDisposition::InternalStepOf {
                target: SourceKey::new(
                    CanonicalSourceFamily::TauriCommand,
                    "safety_blackout_engage_v1",
                ),
            }
        }
        CanonicalSourceFamily::EngineCommand
            if descriptor.source_id == "safety_blackout_release_published" =>
        {
            SourceDisposition::InternalStepOf {
                target: SourceKey::new(
                    CanonicalSourceFamily::TauriCommand,
                    "execute_output_control_v1",
                ),
            }
        }
        _ => SourceDisposition::Unclassified {
''',
    "engine release internal step",
)
control = replace_once(
    control,
    '''    AbortTimelineFollow,
    EngageSafetyBlackout,
''',
    '''    AbortTimelineFollow,
    ReleaseSafetyBlackout,
    EngageSafetyBlackout,
''',
    "reviewed operation enum",
)
control = replace_once(
    control,
    '''            Self::AbortTimelineFollow => TIMELINE_FOLLOW_ABORT_OPERATION_ID,
            Self::EngageSafetyBlackout => SAFETY_BLACKOUT_ENGAGE_OPERATION_ID,
''',
    '''            Self::AbortTimelineFollow => TIMELINE_FOLLOW_ABORT_OPERATION_ID,
            Self::ReleaseSafetyBlackout => OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
            Self::EngageSafetyBlackout => SAFETY_BLACKOUT_ENGAGE_OPERATION_ID,
''',
    "reviewed operation id",
)
control = replace_once(
    control,
    '''        "abort_timeline_follow_runtime_v1" => Some(ReviewedCanonicalOperation::AbortTimelineFollow),
        "safety_blackout_engage_v1" => Some(ReviewedCanonicalOperation::EngageSafetyBlackout),
''',
    '''        "abort_timeline_follow_runtime_v1" => Some(ReviewedCanonicalOperation::AbortTimelineFollow),
        "execute_output_control_v1" => Some(ReviewedCanonicalOperation::ReleaseSafetyBlackout),
        "safety_blackout_engage_v1" => Some(ReviewedCanonicalOperation::EngageSafetyBlackout),
''',
    "reviewed release source",
)
control = replace_once(
    control,
    '''        ReviewedCanonicalOperation::EngageSafetyBlackout => (
            OperationClass::Mutation,
''',
    '''        ReviewedCanonicalOperation::ReleaseSafetyBlackout => (
            OperationClass::Mutation,
            vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::OutputBlackoutRelease,
            ],
            OperationIdempotency::Mutating,
            AdapterPolicy::LocalWindowOutputDisruptiveMutation,
            ReceiptPolicy::ExactTerminalReceipt,
        ),
        ReviewedCanonicalOperation::EngageSafetyBlackout => (
            OperationClass::Mutation,
''',
    "release canonical descriptor",
)
control = replace_once(
    control,
    '''        risk: if matches!(reviewed, ReviewedCanonicalOperation::EngageSafetyBlackout) {
            OperationRisk::S0
        } else {
            OperationRisk::R0
        },
''',
    '''        risk: match reviewed {
            ReviewedCanonicalOperation::ReleaseSafetyBlackout => OperationRisk::R4,
            ReviewedCanonicalOperation::EngageSafetyBlackout => OperationRisk::S0,
            _ => OperationRisk::R0,
        },
''',
    "release risk",
)
control = replace_once(
    control,
    '''        audit: if matches!(reviewed, ReviewedCanonicalOperation::EngageSafetyBlackout) {
            OperationAuditRequirement::Immutable
        } else {
            OperationAuditRequirement::NotApplicable
        },
''',
    '''        audit: if matches!(
            reviewed,
            ReviewedCanonicalOperation::ReleaseSafetyBlackout
                | ReviewedCanonicalOperation::EngageSafetyBlackout
        ) {
            OperationAuditRequirement::Immutable
        } else {
            OperationAuditRequirement::NotApplicable
        },
''',
    "release audit",
)
control = replace_once(
    control,
    '''                | ReviewedCanonicalOperation::AbortTimelineFollow
                | ReviewedCanonicalOperation::EngageSafetyBlackout
''',
    '''                | ReviewedCanonicalOperation::AbortTimelineFollow
                | ReviewedCanonicalOperation::ReleaseSafetyBlackout
                | ReviewedCanonicalOperation::EngageSafetyBlackout
''',
    "release rate policy",
)
control = replace_once(
    control,
    '''        consent_policy: if matches!(reviewed, ReviewedCanonicalOperation::EngageSafetyBlackout) {
            ConsentPolicy::NotRequiredForSafetyOnly
        } else {
            ConsentPolicy::FailClosed
        },
''',
    '''        consent_policy: match reviewed {
            ReviewedCanonicalOperation::ReleaseSafetyBlackout => {
                ConsentPolicy::PhysicalSingleUse15Seconds
            }
            ReviewedCanonicalOperation::EngageSafetyBlackout => {
                ConsentPolicy::NotRequiredForSafetyOnly
            }
            _ => ConsentPolicy::FailClosed,
        },
''',
    "release consent policy",
)
control = replace_once(
    control,
    '''    if operation_id == "safety_blackout_engage_v1" {
        return unavailable_descriptor_for_semantic_operation(
            operation_id,
            SAFETY_BLACKOUT_ENGAGE_OPERATION_ID,
        );
    }
''',
    '''    if operation_id == "execute_output_control_v1" {
        return unavailable_descriptor_for_semantic_operation(
            operation_id,
            OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
        );
    }
    if operation_id == "safety_blackout_engage_v1" {
        return unavailable_descriptor_for_semantic_operation(
            operation_id,
            SAFETY_BLACKOUT_ENGAGE_OPERATION_ID,
        );
    }
''',
    "legacy release semantic descriptor",
)
control = replace_count(
    control,
    "const FRONTEND_INVOKE_COUNT: usize = 389;",
    "const FRONTEND_INVOKE_COUNT: usize = 393;",
    1,
    "legacy frontend count",
)
control = replace_count(
    control,
    "const FRONTEND_COUNT: usize = 389;",
    "const FRONTEND_COUNT: usize = 393;",
    1,
    "canonical frontend count",
)
for old, new, label in [
    ("assert_eq!(registry.operations.len(), 1410);", "assert_eq!(registry.operations.len(), 1414);", "legacy registry total"),
    ("assert_eq!(LEGACY_SOURCE_TOTAL, 1410);", "assert_eq!(LEGACY_SOURCE_TOTAL, 1414);", "canonical legacy total"),
    ("assert_eq!(SOURCE_TOTAL, 1443);", "assert_eq!(SOURCE_TOTAL, 1447);", "canonical source total"),
    ("assert_eq!(canonical.canonical_operations.len(), 14);", "assert_eq!(canonical.canonical_operations.len(), 15);", "canonical operation count"),
    ("assert_eq!(direct.len(), 14);", "assert_eq!(direct.len(), 15);", "direct source count"),
    ("assert_eq!(internal_steps.len(), 4);", "assert_eq!(internal_steps.len(), 5);", "internal source count"),
    ("assert_eq!(unclassified.len(), 1035);", "assert_eq!(unclassified.len(), 1033);", "unclassified source count"),
    ("assert_eq!(legacy.operations.len(), 1410);", "assert_eq!(legacy.operations.len(), 1414);", "legacy json registry total"),
    ("assert_eq!(operations.len(), 1410);", "assert_eq!(operations.len(), 1414);", "legacy json operation total"),
]:
    control = replace_count(control, old, new, 1, label)
release_source_assertions = '''        let blackout_release_direct = direct
            .iter()
            .find(|source| source.source_key.source_id == "execute_output_control_v1")
            .expect("R4 Blackout Release must be a direct canonical source");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&blackout_release_direct.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(OUTPUT_BLACKOUT_RELEASE_OPERATION_ID)
        );
        let blackout_release_alias = aliases
            .iter()
            .find(|source| source.source_key.source_id == "execute_output_control_v1")
            .expect("frontend R4 Blackout Release must retain its canonical alias row");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&blackout_release_alias.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(OUTPUT_BLACKOUT_RELEASE_OPERATION_ID)
        );
'''
control = replace_once(
    control,
    "        for (source_id, target_id) in [\n",
    release_source_assertions + "        for (source_id, target_id) in [\n",
    "release source assertions",
)
control = replace_once(
    control,
    '''            (
                "safety_blackout_engage_published",
                "safety_blackout_engage_v1",
            ),
''',
    '''            (
                "safety_blackout_engage_published",
                "safety_blackout_engage_v1",
            ),
            (
                "safety_blackout_release_published",
                "execute_output_control_v1",
            ),
''',
    "release internal-step assertion",
)
control = replace_once(
    control,
    '''            if operation.operation_id == SAFETY_BLACKOUT_ENGAGE_OPERATION_ID {
                assert_eq!(operation.risk, OperationRisk::S0);
                assert_eq!(operation.audit, OperationAuditRequirement::Immutable);
            } else {
                assert_eq!(operation.risk, OperationRisk::R0);
                assert_eq!(operation.audit, OperationAuditRequirement::NotApplicable);
            }
''',
    '''            if operation.operation_id == SAFETY_BLACKOUT_ENGAGE_OPERATION_ID {
                assert_eq!(operation.risk, OperationRisk::S0);
                assert_eq!(operation.audit, OperationAuditRequirement::Immutable);
            } else if operation.operation_id == OUTPUT_BLACKOUT_RELEASE_OPERATION_ID {
                assert_eq!(operation.risk, OperationRisk::R4);
                assert_eq!(operation.audit, OperationAuditRequirement::Immutable);
            } else {
                assert_eq!(operation.risk, OperationRisk::R0);
                assert_eq!(operation.audit, OperationAuditRequirement::NotApplicable);
            }
''',
    "release registry risk assertion",
)
release_policy_assertion = '''            } else if operation.operation_id == OUTPUT_BLACKOUT_RELEASE_OPERATION_ID {
                assert_eq!(operation.class, OperationClass::Mutation);
                assert_eq!(operation.idempotency, OperationIdempotency::Mutating);
                assert_eq!(
                    operation.adapter_policy,
                    AdapterPolicy::LocalWindowOutputDisruptiveMutation
                );
                assert_eq!(
                    operation.receipt_policy,
                    ReceiptPolicy::ExactTerminalReceipt
                );
                assert_eq!(
                    operation.rate_policy,
                    RatePolicy::TokenBucket4PerSecondBurst8
                );
                assert_eq!(
                    operation.consent_policy,
                    ConsentPolicy::PhysicalSingleUse15Seconds
                );
                assert_eq!(
                    operation.capabilities,
                    vec![
                        OperationCapability::LocalWindowBound,
                        OperationCapability::OutputBlackoutRelease,
                    ]
                );
                assert!(!operation
                    .capabilities
                    .contains(&OperationCapability::AllowedDuringFullLock));
'''
control = replace_once(
    control,
    "            } else if operation.operation_id == SAFETY_BLACKOUT_ENGAGE_OPERATION_ID {\n",
    release_policy_assertion
    + "            } else if operation.operation_id == SAFETY_BLACKOUT_ENGAGE_OPERATION_ID {\n",
    "release registry policy assertion",
)
write(control_path, control)

commands_to_add = {
    "execute_output_control_v1",
    "prepare_output_consent_v1",
    "query_output_consent_status_v1",
    "query_output_control_authority_v1",
}
manifest_path = Path("app/src/tauri-invoke-manifest.json")
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
if not isinstance(manifest, list) or not all(isinstance(value, str) for value in manifest):
    raise RuntimeError("frontend manifest is malformed")
manifest_set = set(manifest)
if commands_to_add & manifest_set:
    raise RuntimeError("one or more R4 frontend commands were already present")
manifest = sorted(manifest_set | commands_to_add)
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n")

typed_path = "app/src/tauriInvokeCommands.ts"
typed = read(typed_path)
start_marker = "export const FRONTEND_TAURI_INVOKE_COMMANDS = [\n"
end_marker = "] as const;"
start = typed.find(start_marker)
end = typed.find(end_marker, start)
if start < 0 or end < 0:
    raise RuntimeError("typed frontend manifest declaration was not found")
array_start = start + len(start_marker)
entries = "".join(f'  "{command}",\n' for command in manifest)
typed = typed[:array_start] + entries + typed[end:]
write(typed_path, typed)

controller_path = "app/src/blackoutReleaseRuntimeController.ts"
controller = read(controller_path)
controller = replace_once(
    controller,
    ''' * Staged R4 renderer lane for releasing the runtime safety blackout.
 *
 * This controller deliberately does not participate in the generated frontend
 * Tauri invoke authority yet. The four commands below remain unreachable from
 * production UI until the canonical R4 registry row and legacy-release bypass
 * removal land in the same reviewed slice.
''',
    ''' * R4 renderer lane for releasing the runtime safety blackout.
 *
 * The finite command union below is included in the generated frontend invoke
 * authority only after the canonical R4 registry row is present.
''',
    "activate staged renderer controller",
)
write(controller_path, controller)

panel_path = "app/src/components/LightingRuntimeControlsPanel.tsx"
panel = read(panel_path)
panel = replace_once(
    panel,
    'import type { EngineSnapshot, MidiInputSummary, SubmasterSummary } from "../types";\n',
    'import type { EngineSnapshot, MidiInputSummary, SubmasterSummary } from "../types";\n'
    'import { BlackoutReleaseControl } from "./BlackoutReleaseControl";\n',
    "release control import",
)
panel = replace_once(
    panel,
    '''  onBlackout: (enabled: boolean) => void | Promise<void>;
  onAllBlackout: (enabled: boolean) => void | Promise<void>;
''',
    '''  onBlackout: (enabled: boolean) => void | Promise<void>;
  onBlackoutReleased: () => void | Promise<void>;
  onAllBlackout: (enabled: boolean) => void | Promise<void>;
''',
    "release refresh prop",
)
panel = replace_once(
    panel,
    '        <button onClick={() => void props.onBlackout(false)}>DMX Clear</button>\n',
    '        <BlackoutReleaseControl onReleased={props.onBlackoutReleased} />\n',
    "remove legacy clear button",
)
write(panel_path, panel)

app_path = "app/src/App.tsx"
app = read(app_path)
app = replace_once(
    app,
    '''      } else {
        // Safety release is intentionally not part of the S0 lane. This
        // legacy local release remains outside Full Lock until the R4
        // ownership/consent contract replaces it.
        await invoke("set_blackout", { enabled: false });
      }
      await refreshSnapshot();
''',
    '''      } else {
        setMessage("DMX blackout release requires physical confirmation in Runtime controls.");
        return;
      }
      await refreshSnapshot();
''',
    "remove App legacy release bypass",
)
match_count = len(re.findall(r"(?m)^(\s*)onBlackout=\{setBlackout\}\s*$", app))
if match_count != 1:
    raise RuntimeError(f"Runtime panel prop: expected one onBlackout binding, found {match_count}")
app = re.sub(
    r"(?m)^(\s*)onBlackout=\{setBlackout\}\s*$",
    lambda match: (
        f"{match.group(1)}onBlackout={{setBlackout}}\n"
        f"{match.group(1)}onBlackoutReleased={{refreshSnapshot}}"
    ),
    app,
    count=1,
)
write(app_path, app)

package_path = "app/package.json"
package = read(package_path)
package = replace_once(
    package,
    '    "check:safety-blackout-runtime": "node scripts/check-safety-blackout-runtime.mjs",\n',
    '    "check:safety-blackout-runtime": "node scripts/check-safety-blackout-runtime.mjs",\n'
    '    "check:blackout-release-runtime": "node scripts/check-blackout-release-runtime.mjs",\n',
    "register R4 harness",
)
write(package_path, package)

print("R4 Blackout Release canonical patch applied")
