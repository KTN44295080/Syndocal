from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


protocol = Path("crates/protocol/src/control_plane_registry_v2.rs")
control = Path("app/src-tauri/src/control_plane.rs")

# A local Release workflow has separate Tauri calls for authority issuance,
# consent preparation/status, and the one direct execution source. These helper
# calls are neither independent canonical operations nor aliases of execution;
# give them an explicit structural disposition which resolves to the reviewed
# R4 operation without deriving a second adapter.
replace_exact(
    protocol,
    '''    InternalStepOf {
        target: SourceKey,
    },
    /// A keyboard shortcut has no invocation authority of its own: its code
''',
    '''    InternalStepOf {
        target: SourceKey,
    },
    /// A local Tauri command which is a required step in a larger canonical
    /// operation workflow. It resolves to the target operation for coverage,
    /// but never derives its own adapter or execution authority.
    WorkflowStepOf {
        target: SourceKey,
    },
    /// A keyboard shortcut has no invocation authority of its own: its code
''',
    "local workflow source disposition",
)
replace_exact(
    protocol,
    '''            Self::InternalStepOf { .. } => "internal_step_of",
            Self::StructuralRoute { .. } => "structural_route",
''',
    '''            Self::InternalStepOf { .. } => "internal_step_of",
            Self::WorkflowStepOf { .. } => "workflow_step_of",
            Self::StructuralRoute { .. } => "structural_route",
''',
    "workflow source disposition name",
)
replace_exact(
    protocol,
    '''            Self::AliasOfSource { target }
            | Self::InternalStepOf { target }
            | Self::StructuralRoute { target } => 96 + target.wire_byte_bound(),
''',
    '''            Self::AliasOfSource { target }
            | Self::InternalStepOf { target }
            | Self::WorkflowStepOf { target }
            | Self::StructuralRoute { target } => 96 + target.wire_byte_bound(),
''',
    "workflow source wire bound",
)
replace_exact(
    protocol,
    '''            SourceDisposition::InternalStepOf { target } => {
                target.validate()?;
                if self.role != SourceRole::EngineCommand {
                    return Err(
                        CanonicalRegistryValidationError::InvalidDispositionForFamily(
                            self.source_key.clone(),
                            self.disposition.kind_name(),
                        ),
                    );
                }
            }
            SourceDisposition::StructuralRoute { target } => {
''',
    '''            SourceDisposition::InternalStepOf { target } => {
                target.validate()?;
                if self.role != SourceRole::EngineCommand {
                    return Err(
                        CanonicalRegistryValidationError::InvalidDispositionForFamily(
                            self.source_key.clone(),
                            self.disposition.kind_name(),
                        ),
                    );
                }
            }
            SourceDisposition::WorkflowStepOf { target } => {
                target.validate()?;
                if self.role != SourceRole::LocalWindowCommand
                    || target.family != CanonicalSourceFamily::TauriCommand
                {
                    return Err(
                        CanonicalRegistryValidationError::InvalidDispositionForFamily(
                            self.source_key.clone(),
                            self.disposition.kind_name(),
                        ),
                    );
                }
            }
            SourceDisposition::StructuralRoute { target } => {
''',
    "validate local workflow source disposition",
)
replace_exact(
    protocol,
    '''                SourceDisposition::InternalStepOf { target } => {
                    if !sources.contains_key(target) {
                        return Err(CanonicalRegistryValidationError::MissingSourceTarget(
                            target.clone(),
                        ));
                    }
                }
                SourceDisposition::StructuralRoute { target } => {
''',
    '''                SourceDisposition::InternalStepOf { target } => {
                    if !sources.contains_key(target) {
                        return Err(CanonicalRegistryValidationError::MissingSourceTarget(
                            target.clone(),
                        ));
                    }
                }
                SourceDisposition::WorkflowStepOf { target } => {
                    if !sources.contains_key(target) {
                        return Err(CanonicalRegistryValidationError::MissingSourceTarget(
                            target.clone(),
                        ));
                    }
                }
                SourceDisposition::StructuralRoute { target } => {
''',
    "validate local workflow source target existence",
)
replace_exact(
    protocol,
    '''    /// Resolve a source through aliases, internal steps, and verified
    /// structural routes to its canonical operation. Structural sources still
''',
    '''    /// Resolve a source through aliases, internal steps, local workflow
    /// steps, and verified structural routes to its canonical operation.
    /// Structural sources still
''',
    "workflow source resolver documentation",
)
replace_exact(
    protocol,
    '''            SourceDisposition::InternalStepOf { target } => {
                resolve_canonical_operation_id(target, sources, canonical_ids, visiting)
            }
            SourceDisposition::StructuralRoute { target } => {
''',
    '''            SourceDisposition::InternalStepOf { target } => {
                resolve_canonical_operation_id(target, sources, canonical_ids, visiting)
            }
            SourceDisposition::WorkflowStepOf { target } => {
                resolve_canonical_operation_id(target, sources, canonical_ids, visiting)
            }
            SourceDisposition::StructuralRoute { target } => {
''',
    "resolve local workflow source to canonical operation",
)
replace_exact(
    protocol,
    '''    #[test]
    fn unclassified_and_internal_sources_cannot_be_exposed() {
''',
    '''    #[test]
    fn local_workflow_step_resolves_without_deriving_an_adapter() {
        let mut registry = valid_registry();
        let workflow = source(
            CanonicalSourceFamily::TauriCommand,
            "prepare_demo",
            SourceDisposition::WorkflowStepOf {
                target: SourceKey::new(CanonicalSourceFamily::TauriCommand, DIRECT_SOURCE_ID),
            },
        );
        registry.source_inventory.push(workflow.clone());
        registry
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        registry.populate_derived_adapters().unwrap();
        assert_eq!(registry.canonical_operations[0].derived_adapters.len(), 1);
        assert_eq!(
            registry
                .canonical_operation_for_source(&workflow.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(OPERATION_ID)
        );

        let mut invalid = valid_registry();
        invalid.source_inventory.push(source(
            CanonicalSourceFamily::EngineCommand,
            "bad_workflow_step",
            SourceDisposition::WorkflowStepOf {
                target: SourceKey::new(CanonicalSourceFamily::TauriCommand, DIRECT_SOURCE_ID),
            },
        ));
        invalid
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        assert!(matches!(
            invalid.validate(),
            Err(CanonicalRegistryValidationError::InvalidDispositionForFamily(_, "workflow_step_of"))
        ));
    }

    #[test]
    fn unclassified_and_internal_sources_cannot_be_exposed() {
''',
    "local workflow source focused protocol test",
)

# This patch runs after both strict wire patches, so the reviewed direct source
# is execute_blackout_release_v1 and consent preparation is the dedicated
# prepare_blackout_release_consent_v1 source.
replace_exact(
    control,
    '''        CanonicalSourceFamily::TauriCommand => {
            if let Some(reviewed) = reviewed_canonical_operation(&descriptor.source_id) {
                SourceDisposition::Operation {
                    canonical_operation_id: reviewed.operation_id().to_string(),
                    projection: TypedSchemaProjection::Exact,
                }
            } else {
                SourceDisposition::Unclassified {
                    reason: unclassified_reason(source_family).to_string(),
                }
            }
        }
''',
    '''        CanonicalSourceFamily::TauriCommand => {
            if let Some(reviewed) = reviewed_canonical_operation(&descriptor.source_id) {
                SourceDisposition::Operation {
                    canonical_operation_id: reviewed.operation_id().to_string(),
                    projection: TypedSchemaProjection::Exact,
                }
            } else if matches!(
                descriptor.source_id.as_str(),
                "query_output_control_authority_v1"
                    | "prepare_blackout_release_consent_v1"
                    | "query_output_consent_status_v1"
            ) {
                SourceDisposition::WorkflowStepOf {
                    target: SourceKey::new(
                        CanonicalSourceFamily::TauriCommand,
                        "execute_blackout_release_v1",
                    ),
                }
            } else {
                SourceDisposition::Unclassified {
                    reason: unclassified_reason(source_family).to_string(),
                }
            }
        }
''',
    "classify R4 Release local workflow support commands",
)

replace_exact(
    control,
    '''        let structural_routes = canonical
            .source_inventory
            .iter()
            .filter(|source| {
                matches!(
                    &source.disposition,
                    SourceDisposition::StructuralRoute { .. }
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(direct.len(), 15);
        assert_eq!(aliases.len(), FRONTEND_COUNT);
        assert_eq!(internal_steps.len(), 5);
        assert_eq!(structural_routes.len(), 1);
        assert_eq!(unclassified.len(), 1034);
        assert_eq!(
            direct.len()
                + aliases.len()
                + internal_steps.len()
                + structural_routes.len()
                + unclassified.len(),
            SOURCE_TOTAL
        );
''',
    '''        let workflow_steps = canonical
            .source_inventory
            .iter()
            .filter(|source| {
                matches!(
                    &source.disposition,
                    SourceDisposition::WorkflowStepOf { .. }
                )
            })
            .collect::<Vec<_>>();
        let structural_routes = canonical
            .source_inventory
            .iter()
            .filter(|source| {
                matches!(
                    &source.disposition,
                    SourceDisposition::StructuralRoute { .. }
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(direct.len(), 15);
        assert_eq!(aliases.len(), FRONTEND_COUNT);
        assert_eq!(internal_steps.len(), 5);
        assert_eq!(workflow_steps.len(), 3);
        assert_eq!(structural_routes.len(), 1);
        assert_eq!(unclassified.len(), 1031);
        assert_eq!(
            direct.len()
                + aliases.len()
                + internal_steps.len()
                + workflow_steps.len()
                + structural_routes.len()
                + unclassified.len(),
            SOURCE_TOTAL
        );
''',
    "R4 workflow source inventory counts",
)

replace_exact(
    control,
    '''        for source in &unclassified {
            assert!(canonical
                .canonical_operation_for_source(&source.source_key)
                .unwrap()
                .is_none());
        }
''',
    '''        for source_id in [
            "query_output_control_authority_v1",
            "prepare_blackout_release_consent_v1",
            "query_output_consent_status_v1",
        ] {
            let workflow = workflow_steps
                .iter()
                .find(|source| source.source_key.source_id == source_id)
                .unwrap_or_else(|| panic!("missing R4 Release workflow step {source_id}"));
            assert_eq!(workflow.source_key.family, CanonicalSourceFamily::TauriCommand);
            assert!(matches!(
                &workflow.disposition,
                SourceDisposition::WorkflowStepOf { target }
                    if *target == SourceKey::new(
                        CanonicalSourceFamily::TauriCommand,
                        "execute_blackout_release_v1"
                    )
            ));
            assert_eq!(
                canonical
                    .canonical_operation_for_source(&workflow.source_key)
                    .unwrap()
                    .map(|operation| operation.operation_id.as_str()),
                Some(OUTPUT_BLACKOUT_RELEASE_OPERATION_ID)
            );
            assert_eq!(
                canonical
                    .canonical_operations
                    .iter()
                    .find(|operation| operation.operation_id == OUTPUT_BLACKOUT_RELEASE_OPERATION_ID)
                    .unwrap()
                    .derived_adapters
                    .len(),
                1,
                "workflow support commands must not derive extra adapters"
            );
        }
        for source in &unclassified {
            assert!(canonical
                .canonical_operation_for_source(&source.source_key)
                .unwrap()
                .is_none());
        }
''',
    "R4 workflow sources resolve to Release without extra adapter",
)
