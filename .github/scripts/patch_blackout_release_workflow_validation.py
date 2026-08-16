from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


protocol = Path("crates/protocol/src/control_plane_registry_v2.rs")
control = Path("app/src-tauri/src/control_plane.rs")

replace_exact(
    protocol,
    '''    InternalStepDoesNotResolveToOperation(SourceKey),
    SchemaProjectionMismatch(SourceKey),
''',
    '''    InternalStepDoesNotResolveToOperation(SourceKey),
    WorkflowStepDoesNotResolveToOperation(SourceKey),
    SchemaProjectionMismatch(SourceKey),
''',
    "workflow step terminal-resolution error",
)
replace_exact(
    protocol,
    '''            Self::InternalStepDoesNotResolveToOperation(key) => write!(
                formatter,
                "internal source step does not terminate at an operation: {key:?}"
            ),
            Self::SchemaProjectionMismatch(key) => {
''',
    '''            Self::InternalStepDoesNotResolveToOperation(key) => write!(
                formatter,
                "internal source step does not terminate at an operation: {key:?}"
            ),
            Self::WorkflowStepDoesNotResolveToOperation(key) => write!(
                formatter,
                "local workflow source step does not terminate at an operation: {key:?}"
            ),
            Self::SchemaProjectionMismatch(key) => {
''',
    "workflow step terminal-resolution display",
)

# This patch runs after WorkflowStepOf exists. A mere target source is not
# enough: the workflow chain must terminate at an actual canonical operation.
replace_exact(
    protocol,
    '''                SourceDisposition::WorkflowStepOf { target } => {
                    if !sources.contains_key(target) {
                        return Err(CanonicalRegistryValidationError::MissingSourceTarget(
                            target.clone(),
                        ));
                    }
                }
                SourceDisposition::StructuralRoute { target } => {
''',
    '''                SourceDisposition::WorkflowStepOf { target } => {
                    if !sources.contains_key(target) {
                        return Err(CanonicalRegistryValidationError::MissingSourceTarget(
                            target.clone(),
                        ));
                    }
                    if resolve_canonical_operation_id(
                        &source.source_key,
                        &sources,
                        &canonical_ids,
                        &mut BTreeSet::new(),
                    )?
                    .is_none()
                    {
                        return Err(
                            CanonicalRegistryValidationError::WorkflowStepDoesNotResolveToOperation(
                                source.source_key.clone(),
                            ),
                        );
                    }
                }
                SourceDisposition::StructuralRoute { target } => {
''',
    "workflow source must terminate at canonical operation",
)

# Extend the existing focused test so the already-run exact test proves both
# the valid direct route and fail-closed rejection of an unclassified target.
replace_exact(
    protocol,
    '''        assert!(matches!(
            invalid.validate(),
            Err(CanonicalRegistryValidationError::InvalidDispositionForFamily(_, "workflow_step_of"))
        ));
    }

    #[test]
    fn unclassified_and_internal_sources_cannot_be_exposed() {
''',
    '''        assert!(matches!(
            invalid.validate(),
            Err(CanonicalRegistryValidationError::InvalidDispositionForFamily(_, "workflow_step_of"))
        ));

        let mut unresolved = valid_registry();
        unresolved.source_inventory.push(source(
            CanonicalSourceFamily::TauriCommand,
            "unclassified_helper",
            SourceDisposition::Unclassified {
                reason: "not reviewed".to_string(),
            },
        ));
        unresolved.source_inventory.push(source(
            CanonicalSourceFamily::TauriCommand,
            "prepare_unresolved",
            SourceDisposition::WorkflowStepOf {
                target: SourceKey::new(
                    CanonicalSourceFamily::TauriCommand,
                    "unclassified_helper",
                ),
            },
        ));
        unresolved
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        assert!(matches!(
            unresolved.validate(),
            Err(CanonicalRegistryValidationError::WorkflowStepDoesNotResolveToOperation(_))
        ));
    }

    #[test]
    fn unclassified_and_internal_sources_cannot_be_exposed() {
''',
    "workflow focused test rejects unresolved target",
)

# B is now a safer-direction-only shortcut in appShortcutActions.ts. Classify
# that raw keyboard source as a structural route to the canonical S0 operation;
# it has no adapter authority of its own and cannot express Blackout Release.
replace_exact(
    control,
    '''        disposition: if family == CanonicalSourceFamily::KeyboardApp
            && source_id == "toggle_timeline_playback_v1"
        {
            SourceDisposition::StructuralRoute {
                target: SourceKey::new(
                    CanonicalSourceFamily::TauriCommand,
                    "set_timeline_transport_playing_runtime_v1",
                ),
            }
        } else {
            SourceDisposition::Unclassified {
                reason: unclassified_reason(family).to_string(),
            }
        },
''',
    '''        disposition: if family == CanonicalSourceFamily::KeyboardApp
            && source_id == "toggle_timeline_playback_v1"
        {
            SourceDisposition::StructuralRoute {
                target: SourceKey::new(
                    CanonicalSourceFamily::TauriCommand,
                    "set_timeline_transport_playing_runtime_v1",
                ),
            }
        } else if family == CanonicalSourceFamily::KeyboardApp
            && source_id == "toggle_blackout_v1"
        {
            SourceDisposition::StructuralRoute {
                target: SourceKey::new(
                    CanonicalSourceFamily::TauriCommand,
                    "safety_blackout_engage_v1",
                ),
            }
        } else {
            SourceDisposition::Unclassified {
                reason: unclassified_reason(family).to_string(),
            }
        },
''',
    "keyboard blackout shortcut structurally routes to S0 engage",
)

# WorkflowStepOf moved three Tauri support sources out of Unclassified earlier.
# Moving the now-safe B shortcut changes only the structural/unclassified split.
replace_exact(
    control,
    "        assert_eq!(structural_routes.len(), 1);",
    "        assert_eq!(structural_routes.len(), 2);",
    "keyboard S0 structural-route count",
)
replace_exact(
    control,
    "        assert_eq!(unclassified.len(), 1031);",
    "        assert_eq!(unclassified.len(), 1030);",
    "keyboard S0 unclassified count",
)
replace_exact(
    control,
    '''        for source in &unclassified {
''',
    '''        let keyboard_blackout = structural_routes
            .iter()
            .find(|source| {
                source.source_key.family == CanonicalSourceFamily::KeyboardApp
                    && source.source_key.source_id == "toggle_blackout_v1"
            })
            .expect("missing engage-only keyboard blackout structural route");
        assert!(matches!(
            &keyboard_blackout.disposition,
            SourceDisposition::StructuralRoute { target }
                if *target == SourceKey::new(
                    CanonicalSourceFamily::TauriCommand,
                    "safety_blackout_engage_v1"
                )
        ));
        assert_eq!(
            canonical
                .canonical_operation_for_source(&keyboard_blackout.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(SAFETY_BLACKOUT_ENGAGE_OPERATION_ID)
        );

        for source in &unclassified {
''',
    "keyboard blackout route resolves to canonical S0 engage",
)
