//! Fail-closed AI0 inventory for existing IO control surfaces.
//!
//! This module is metadata only.  It does not start an input, invoke a
//! callback, read a PIN, emit DMX, or provide an AI/MCP/HTTP adapter.

use protocol::control_plane::{
    OperationAuditRequirement, OperationAvailability, OperationCapability, OperationClass,
    OperationDescriptor, OperationIdempotency, OperationRisk, OperationSourceFamily,
    SchemaIdentity, CONTROL_PLANE_SCHEMA_VERSION,
};

use crate::{
    dmx_input::{DMX_INPUT_EVENT_VARIANT_NAMES, DMX_INPUT_PROTOCOL_VARIANT_NAMES},
    midi::{
        MIDI_CLOCK_EVENT_VARIANT_NAMES, MIDI_CONTROL_ACTION_VARIANT_NAMES,
        MIDI_CONTROL_EVENT_VARIANT_NAMES, MIDI_CONTROL_MESSAGE_VARIANT_NAMES,
    },
    osc::{OSC_CONTROL_ACTION_VARIANT_NAMES, OSC_INPUT_EVENT_VARIANT_NAMES},
    remote_ws::{RemoteClientRequest, RemoteInputEvent, RemoteWireOperation},
};

/// Deterministic, unavailable descriptors for every remote event, request,
/// and accepted wire operation.
pub fn control_plane_remote_descriptors() -> Vec<OperationDescriptor> {
    let mut descriptors = descriptors_for_variants(
        OperationSourceFamily::RemoteInputEvent,
        "input_event",
        RemoteInputEvent::CONTROL_PLANE_VARIANT_NAMES,
    );
    descriptors.extend(descriptors_for_variants(
        OperationSourceFamily::RemoteClientRequest,
        "client_request",
        RemoteClientRequest::CONTROL_PLANE_VARIANT_NAMES,
    ));
    descriptors.extend(RemoteWireOperation::CONTROL_PLANE_OPERATIONS.iter().map(
        |(variant_name, wire_type)| {
            unavailable_remote_descriptor(
                OperationSourceFamily::RemoteWireOperation,
                "wire_operation",
                &lower_snake_variant(variant_name),
                wire_type,
            )
        },
    ));
    descriptors
}

/// Deterministic, unavailable descriptors for the authored MIDI/OSC mapping
/// actions and the concrete ingress/event families those adapters expose.
/// DMX mappings deliberately reuse `OscControlAction` and emit
/// `OscInputEvent`, so those shared families are inventoried once under OSC;
/// only DMX's protocol selector and raw input event are DMX-specific entries.
pub fn control_plane_midi_osc_dmx_descriptors() -> Vec<OperationDescriptor> {
    let families = [
        (
            OperationSourceFamily::MidiControlMessage,
            "midi",
            "control_message",
            MIDI_CONTROL_MESSAGE_VARIANT_NAMES,
        ),
        (
            OperationSourceFamily::MidiControlAction,
            "midi",
            "control_action",
            MIDI_CONTROL_ACTION_VARIANT_NAMES,
        ),
        (
            OperationSourceFamily::MidiClockEvent,
            "midi",
            "clock_event",
            MIDI_CLOCK_EVENT_VARIANT_NAMES,
        ),
        (
            OperationSourceFamily::MidiControlEvent,
            "midi",
            "control_event",
            MIDI_CONTROL_EVENT_VARIANT_NAMES,
        ),
        (
            OperationSourceFamily::OscControlAction,
            "osc",
            "control_action",
            OSC_CONTROL_ACTION_VARIANT_NAMES,
        ),
        (
            OperationSourceFamily::OscInputEvent,
            "osc",
            "input_event",
            OSC_INPUT_EVENT_VARIANT_NAMES,
        ),
        (
            OperationSourceFamily::DmxInputProtocol,
            "dmx",
            "input_protocol",
            DMX_INPUT_PROTOCOL_VARIANT_NAMES,
        ),
        (
            OperationSourceFamily::DmxInputEvent,
            "dmx",
            "input_event",
            DMX_INPUT_EVENT_VARIANT_NAMES,
        ),
    ];
    families
        .into_iter()
        .flat_map(|(source_family, adapter, inventory_kind, names)| {
            descriptors_for_inventory_variants(source_family, adapter, inventory_kind, names)
        })
        .collect()
}

pub const fn midi_control_message_variant_count() -> usize {
    MIDI_CONTROL_MESSAGE_VARIANT_NAMES.len()
}

pub const fn midi_control_action_variant_count() -> usize {
    MIDI_CONTROL_ACTION_VARIANT_NAMES.len()
}

pub const fn midi_clock_event_variant_count() -> usize {
    MIDI_CLOCK_EVENT_VARIANT_NAMES.len()
}

pub const fn midi_control_event_variant_count() -> usize {
    MIDI_CONTROL_EVENT_VARIANT_NAMES.len()
}

pub const fn osc_control_action_variant_count() -> usize {
    OSC_CONTROL_ACTION_VARIANT_NAMES.len()
}

pub const fn osc_input_event_variant_count() -> usize {
    OSC_INPUT_EVENT_VARIANT_NAMES.len()
}

pub const fn dmx_input_protocol_variant_count() -> usize {
    DMX_INPUT_PROTOCOL_VARIANT_NAMES.len()
}

pub const fn dmx_input_event_variant_count() -> usize {
    DMX_INPUT_EVENT_VARIANT_NAMES.len()
}

pub const fn remote_input_event_variant_count() -> usize {
    RemoteInputEvent::CONTROL_PLANE_VARIANT_NAMES.len()
}

pub const fn remote_client_request_variant_count() -> usize {
    RemoteClientRequest::CONTROL_PLANE_VARIANT_NAMES.len()
}

pub const fn remote_wire_operation_count() -> usize {
    RemoteWireOperation::CONTROL_PLANE_OPERATIONS.len()
}

fn descriptors_for_variants(
    source_family: OperationSourceFamily,
    inventory_kind: &str,
    names: &[&str],
) -> Vec<OperationDescriptor> {
    names
        .iter()
        .map(|variant_name| {
            let source_id = lower_snake_variant(variant_name);
            unavailable_remote_descriptor(source_family, inventory_kind, &source_id, &source_id)
        })
        .collect()
}

fn descriptors_for_inventory_variants(
    source_family: OperationSourceFamily,
    adapter: &str,
    inventory_kind: &str,
    names: &[&str],
) -> Vec<OperationDescriptor> {
    names
        .iter()
        .map(|variant_name| {
            let source_id = lower_snake_variant(variant_name);
            unavailable_inventory_descriptor(
                source_family,
                adapter,
                inventory_kind,
                &source_id,
                &source_id,
            )
        })
        .collect()
}

fn unavailable_remote_descriptor(
    source_family: OperationSourceFamily,
    inventory_kind: &str,
    semantic_source_id: &str,
    raw_source_id: &str,
) -> OperationDescriptor {
    unavailable_inventory_descriptor(
        source_family,
        "remote",
        inventory_kind,
        semantic_source_id,
        raw_source_id,
    )
}

fn unavailable_inventory_descriptor(
    source_family: OperationSourceFamily,
    adapter: &str,
    inventory_kind: &str,
    semantic_source_id: &str,
    raw_source_id: &str,
) -> OperationDescriptor {
    let operation_id =
        format!("syndocal.inventory.{adapter}.{inventory_kind}.{semantic_source_id}.v1");
    OperationDescriptor {
        schema: SchemaIdentity::descriptor(),
        operation_id: operation_id.clone(),
        source_family,
        source_id: raw_source_id.to_string(),
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

fn lower_snake_variant(variant_name: &str) -> String {
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
    fn all_remote_families_are_exact_unique_and_fail_closed() {
        const INPUT_EVENT_COUNT: usize = 51;
        const CLIENT_REQUEST_COUNT: usize = 7;
        const WIRE_OPERATION_COUNT: usize = 58;
        let descriptors = control_plane_remote_descriptors();
        assert_eq!(remote_input_event_variant_count(), INPUT_EVENT_COUNT);
        assert_eq!(remote_client_request_variant_count(), CLIENT_REQUEST_COUNT);
        assert_eq!(remote_wire_operation_count(), WIRE_OPERATION_COUNT);
        assert_eq!(
            descriptors.len(),
            INPUT_EVENT_COUNT + CLIENT_REQUEST_COUNT + WIRE_OPERATION_COUNT
        );
        assert_eq!(
            descriptors
                .iter()
                .map(|descriptor| (descriptor.source_family, descriptor.source_id.as_str()))
                .collect::<BTreeSet<_>>()
                .len(),
            descriptors.len()
        );
        let expected_wire_source_ids = RemoteWireOperation::CONTROL_PLANE_OPERATIONS
            .iter()
            .map(|(_, wire_type)| *wire_type)
            .collect::<BTreeSet<_>>();
        let actual_wire_source_ids = descriptors
            .iter()
            .filter(|descriptor| {
                descriptor.source_family == OperationSourceFamily::RemoteWireOperation
            })
            .map(|descriptor| descriptor.source_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(expected_wire_source_ids.len(), WIRE_OPERATION_COUNT);
        assert_eq!(actual_wire_source_ids, expected_wire_source_ids);
        for (variant_name, wire_type) in RemoteWireOperation::CONTROL_PLANE_OPERATIONS {
            let semantic_source_id = lower_snake_variant(variant_name);
            let descriptor = descriptors
                .iter()
                .find(|descriptor| {
                    descriptor.source_family == OperationSourceFamily::RemoteWireOperation
                        && descriptor.source_id == *wire_type
                })
                .expect("every parser wire selector must have one registry descriptor");
            assert_eq!(
                descriptor.operation_id,
                format!("syndocal.inventory.remote.wire_operation.{semantic_source_id}.v1")
            );
        }
        for descriptor in descriptors {
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
                .starts_with("syndocal.inventory.remote."));
            descriptor.validate().unwrap();
        }
    }

    #[test]
    fn remote_acronyms_remain_stable_lower_snake_ids() {
        assert_eq!(
            lower_snake_variant("GetExternalVideoIoPlans"),
            "get_external_video_io_plans"
        );
        assert_eq!(
            lower_snake_variant("SyncAbletonLinkClock"),
            "sync_ableton_link_clock"
        );
    }

    #[test]
    fn midi_osc_dmx_families_are_exact_unique_and_fail_closed() {
        const MIDI_CONTROL_MESSAGE_COUNT: usize = 4;
        const MIDI_CONTROL_ACTION_COUNT: usize = 51;
        const MIDI_CLOCK_EVENT_COUNT: usize = 6;
        const MIDI_CONTROL_EVENT_COUNT: usize = 47;
        const OSC_CONTROL_ACTION_COUNT: usize = 47;
        const OSC_INPUT_EVENT_COUNT: usize = 47;
        const DMX_INPUT_PROTOCOL_COUNT: usize = 2;
        const DMX_INPUT_EVENT_COUNT: usize = 2;
        const TOTAL_COUNT: usize = MIDI_CONTROL_MESSAGE_COUNT
            + MIDI_CONTROL_ACTION_COUNT
            + MIDI_CLOCK_EVENT_COUNT
            + MIDI_CONTROL_EVENT_COUNT
            + OSC_CONTROL_ACTION_COUNT
            + OSC_INPUT_EVENT_COUNT
            + DMX_INPUT_PROTOCOL_COUNT
            + DMX_INPUT_EVENT_COUNT;

        assert_eq!(
            midi_control_message_variant_count(),
            MIDI_CONTROL_MESSAGE_COUNT
        );
        assert_eq!(
            midi_control_action_variant_count(),
            MIDI_CONTROL_ACTION_COUNT
        );
        assert_eq!(midi_clock_event_variant_count(), MIDI_CLOCK_EVENT_COUNT);
        assert_eq!(midi_control_event_variant_count(), MIDI_CONTROL_EVENT_COUNT);
        assert_eq!(osc_control_action_variant_count(), OSC_CONTROL_ACTION_COUNT);
        assert_eq!(osc_input_event_variant_count(), OSC_INPUT_EVENT_COUNT);
        assert_eq!(dmx_input_protocol_variant_count(), DMX_INPUT_PROTOCOL_COUNT);
        assert_eq!(dmx_input_event_variant_count(), DMX_INPUT_EVENT_COUNT);
        assert_eq!(TOTAL_COUNT, 206);

        let families = [
            (
                OperationSourceFamily::MidiControlMessage,
                "midi",
                "control_message",
                MIDI_CONTROL_MESSAGE_VARIANT_NAMES,
            ),
            (
                OperationSourceFamily::MidiControlAction,
                "midi",
                "control_action",
                MIDI_CONTROL_ACTION_VARIANT_NAMES,
            ),
            (
                OperationSourceFamily::MidiClockEvent,
                "midi",
                "clock_event",
                MIDI_CLOCK_EVENT_VARIANT_NAMES,
            ),
            (
                OperationSourceFamily::MidiControlEvent,
                "midi",
                "control_event",
                MIDI_CONTROL_EVENT_VARIANT_NAMES,
            ),
            (
                OperationSourceFamily::OscControlAction,
                "osc",
                "control_action",
                OSC_CONTROL_ACTION_VARIANT_NAMES,
            ),
            (
                OperationSourceFamily::OscInputEvent,
                "osc",
                "input_event",
                OSC_INPUT_EVENT_VARIANT_NAMES,
            ),
            (
                OperationSourceFamily::DmxInputProtocol,
                "dmx",
                "input_protocol",
                DMX_INPUT_PROTOCOL_VARIANT_NAMES,
            ),
            (
                OperationSourceFamily::DmxInputEvent,
                "dmx",
                "input_event",
                DMX_INPUT_EVENT_VARIANT_NAMES,
            ),
        ];
        let expected = families
            .into_iter()
            .flat_map(|(family, adapter, kind, names)| {
                names.iter().map(move |name| {
                    let source_id = lower_snake_variant(name);
                    (
                        family,
                        source_id.clone(),
                        format!("syndocal.inventory.{adapter}.{kind}.{source_id}.v1"),
                    )
                })
            })
            .collect::<BTreeSet<_>>();
        let descriptors = control_plane_midi_osc_dmx_descriptors();
        let actual = descriptors
            .iter()
            .map(|descriptor| {
                (
                    descriptor.source_family,
                    descriptor.source_id.clone(),
                    descriptor.operation_id.clone(),
                )
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(descriptors.len(), TOTAL_COUNT);
        assert_eq!(expected.len(), TOTAL_COUNT);
        assert_eq!(actual, expected);

        for descriptor in descriptors {
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
            descriptor.validate().unwrap();
        }
    }
}
