//! Fail-closed AI0 inventory for engine commands.
//!
//! This module only describes the `EngineCommand` declaration.  It does not
//! enqueue a command, inspect runtime state, or create a local/remote adapter.

use protocol::control_plane::{
    OperationAuditRequirement, OperationAvailability, OperationCapability, OperationClass,
    OperationDescriptor, OperationIdempotency, OperationRisk, OperationSourceFamily,
    SchemaIdentity, CONTROL_PLANE_SCHEMA_VERSION,
};

use super::EngineCommand;

/// Return one deterministic, unavailable descriptor for every `EngineCommand`
/// variant. The names come from the enum's declarative definition, so adding a
/// variant cannot omit it from the inventory.
pub fn control_plane_engine_command_descriptors() -> Vec<OperationDescriptor> {
    EngineCommand::CONTROL_PLANE_VARIANT_NAMES
        .iter()
        .map(|variant_name| unavailable_engine_command_descriptor(variant_name))
        .collect()
}

/// The source-of-truth command count. It is generated alongside the enum,
/// rather than parsed from source text or maintained as a separate list.
pub const fn engine_command_variant_count() -> usize {
    EngineCommand::CONTROL_PLANE_VARIANT_NAMES.len()
}

fn unavailable_engine_command_descriptor(variant_name: &str) -> OperationDescriptor {
    let source_id = lower_snake_engine_variant(variant_name);
    let operation_id = format!("syndocal.inventory.engine.{source_id}.v1");
    OperationDescriptor {
        schema: SchemaIdentity::descriptor(),
        operation_id: operation_id.clone(),
        source_family: OperationSourceFamily::EngineCommand,
        source_id,
        class: OperationClass::Mutation,
        risk: OperationRisk::R5,
        capabilities: vec![OperationCapability::InternalInventory],
        availability: OperationAvailability::Unavailable,
        idempotency: OperationIdempotency::Mutating,
        audit: OperationAuditRequirement::RequiredBeforeExternalExecution,
        request_schema: command_schema(&operation_id, "request"),
        response_schema: command_schema(&operation_id, "response"),
    }
}

fn command_schema(operation_id: &str, direction: &str) -> SchemaIdentity {
    SchemaIdentity {
        name: format!("{operation_id}.{direction}"),
        version: CONTROL_PLANE_SCHEMA_VERSION,
    }
}

/// Converts a Rust PascalCase variant into its stable lower_snake inventory
/// identity while preserving acronym runs (`SetDmxInputFrame` becomes
/// `set_dmx_input_frame`). Engine variants are ASCII identifiers by language
/// definition; a non-ASCII byte never reaches this function.
fn lower_snake_engine_variant(variant_name: &str) -> String {
    let characters = variant_name.as_bytes();
    let mut result = String::with_capacity(variant_name.len() + 8);
    for (index, byte) in characters.iter().copied().enumerate() {
        let prior = index
            .checked_sub(1)
            .and_then(|prior| characters.get(prior))
            .copied();
        let next = characters.get(index + 1).copied();
        let starts_new_word = byte.is_ascii_uppercase()
            && index > 0
            && (prior.is_some_and(|prior| prior.is_ascii_lowercase() || prior.is_ascii_digit())
                || (prior.is_some_and(|prior| prior.is_ascii_uppercase())
                    && next.is_some_and(|next| next.is_ascii_lowercase())));
        if starts_new_word {
            result.push('_');
        }
        result.push(byte.to_ascii_lowercase() as char);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn exact_engine_command_inventory_is_generated_once_and_conservative() {
        const EXPECTED_ENGINE_COMMAND_COUNT: usize = 260;
        let descriptors = control_plane_engine_command_descriptors();
        assert_eq!(
            engine_command_variant_count(),
            EXPECTED_ENGINE_COMMAND_COUNT
        );
        assert_eq!(descriptors.len(), EXPECTED_ENGINE_COMMAND_COUNT);
        assert_eq!(
            EngineCommand::CONTROL_PLANE_VARIANT_NAMES
                .iter()
                .collect::<BTreeSet<_>>()
                .len(),
            EXPECTED_ENGINE_COMMAND_COUNT,
            "the Rust enum itself rejects duplicate variant names; retain this proof against accidental inventory transformations"
        );
        assert_eq!(
            descriptors
                .iter()
                .map(|descriptor| descriptor.source_id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            EXPECTED_ENGINE_COMMAND_COUNT,
        );
        for descriptor in descriptors {
            assert_eq!(
                descriptor.source_family,
                OperationSourceFamily::EngineCommand
            );
            assert_eq!(descriptor.class, OperationClass::Mutation);
            assert_eq!(descriptor.risk, OperationRisk::R5);
            assert_eq!(descriptor.availability, OperationAvailability::Unavailable);
            assert_eq!(descriptor.idempotency, OperationIdempotency::Mutating);
            assert_eq!(
                descriptor.audit,
                OperationAuditRequirement::RequiredBeforeExternalExecution
            );
            assert_eq!(
                descriptor.capabilities,
                vec![OperationCapability::InternalInventory]
            );
            assert!(descriptor
                .operation_id
                .starts_with("syndocal.inventory.engine."));
            assert!(descriptor.operation_id.ends_with(".v1"));
            descriptor.validate().unwrap();
        }
    }

    #[test]
    fn pascal_case_acronyms_produce_stable_lower_snake_ids() {
        assert_eq!(
            lower_snake_engine_variant("SetDmxInputFrame"),
            "set_dmx_input_frame"
        );
        assert_eq!(
            lower_snake_engine_variant("SetAutoVjArmed"),
            "set_auto_vj_armed"
        );
        assert_eq!(
            lower_snake_engine_variant("PulseVideoLayerIsfEvent"),
            "pulse_video_layer_isf_event"
        );
    }
}
