//! Local-only control-plane inventory.
//!
//! This module intentionally provides discovery metadata, not an execution
//! transport.  It is compiled from the same `generate_handler!` source that
//! Tauri uses so a newly registered command is fail-closed as R5/Unavailable
//! until an explicit, reviewed R0 classification is added here.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::LazyLock,
};

use engine::control_plane_engine_command_descriptors;
use io::{control_plane_midi_osc_dmx_descriptors, control_plane_remote_descriptors};
use protocol::control_plane::{
    OperationAuditRequirement, OperationAvailability, OperationCapability, OperationClass,
    OperationDescriptor, OperationIdempotency, OperationRegistry, OperationRisk,
    OperationSourceFamily, SchemaIdentity, CONTROL_PLANE_SCHEMA_VERSION,
};
use protocol::control_plane_command::{
    CUE_LIST_DELETE_OPERATION_ID, CUE_LIST_REORDER_OPERATION_ID, EMPTY_CUE_CREATE_OPERATION_ID,
    OUTPUT_BLACKOUT_RELEASE_OPERATION_ID, OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID,
    OUTPUT_CONTROL_COMMAND_SCHEMA_VERSION, OUTPUT_DISPLAY_ADD_OPERATION_ID,
    OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID, OUTPUT_ENABLE_OPERATION_ID,
    OUTPUT_LEASE_ACQUIRE_OPERATION_ID, OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID,
    OUTPUT_LEASE_RECOVER_OPERATION_ID, OUTPUT_LEASE_RELINQUISH_OPERATION_ID,
    OUTPUT_LEASE_RENEW_OPERATION_ID, OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
    OUTPUT_STANDBY_TAKEOVER_OPERATION_ID, SAFETY_BLACKOUT_ENGAGE_OPERATION_ID,
    SET_EFFECT_ENABLED_OPERATION_ID, TIMELINE_FOLLOW_ABORT_AUTHORITY_QUERY_OPERATION_ID,
    TIMELINE_FOLLOW_ABORT_OPERATION_ID, TIMELINE_TRANSPORT_AUTHORITY_QUERY_OPERATION_ID,
    TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID,
};
use protocol::control_plane_registry_v2::{
    AdapterPolicy, CanonicalControlPlaneRegistry, CanonicalOperationDescriptor,
    CanonicalRegistryValidationError, CanonicalSourceFamily, ConsentPolicy, PayloadPolicy,
    RatePolicy, ReceiptPolicy, SourceDisposition, SourceInventoryDescriptor, SourceKey, SourceRole,
    TypedSchemaProjection,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_REGISTERED_OPERATIONS: usize = 2048;
const OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID: &str =
    "syndocal.query.output.display.add.authority.v1";
const MAIN_RS_SOURCE: &str = include_str!("main.rs");
const FRONTEND_INVOKE_MANIFEST: &str = include_str!("../../src/tauri-invoke-manifest.json");
const KEYBOARD_SHORTCUT_SOURCE_MANIFEST: &str =
    include_str!("../../src/keyboard-shortcut-source-manifest.json");
const KEYBOARD_SHORTCUT_SOURCE_MANIFEST_SCHEMA_VERSION: u16 = 1;
const KEYBOARD_APP_SHORTCUT_SOURCE_COUNT: usize = 30;
const KEYBOARD_PROJECT_FILE_SHORTCUT_SOURCE_COUNT: usize = 3;
/// The command source is parsed and validated exactly once.  Local discovery
/// calls only clone this immutable, validated value; they never parse source
/// text or make an external request on the invocation path.
static VALIDATED_REGISTRY: LazyLock<Result<OperationRegistry, ControlPlaneRegistryError>> =
    LazyLock::new(build_registry);
static VALIDATED_CANONICAL_REGISTRY: LazyLock<
    Result<CanonicalControlPlaneRegistry, ControlPlaneRegistryError>,
> = LazyLock::new(build_canonical_registry);

/// Server-side admission class for the exact production Tauri inventory.
/// `None` is the fail-closed answer for every unregistered command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TauriRouteAdmissionClass {
    ReadOnly,
    RendererTicketedProjectMutation,
    BackendAuthoritativeProjectMutation,
    ProjectReplacement,
    ProjectHistory,
    RuntimeMutation,
    FileExportMutation,
    SafetyMutation,
    RecoveryMaintenance,
    Retired,
}

static TAURI_ROUTE_ADMISSION_CLASSES: LazyLock<
    Result<BTreeMap<String, TauriRouteAdmissionClass>, ControlPlaneRegistryError>,
> = LazyLock::new(|| {
    let registered = registered_tauri_command_names_from_source(MAIN_RS_SOURCE)?;
    let inventory_fingerprint = format!("{:x}", Sha256::digest(registered.join("\n").as_bytes()));
    if registered.len() != 478
        || inventory_fingerprint
            != "79104a83edc9dd8608c29f2c99de26102fb0a107c19ea70e2c604a9ef03f7f7c"
    {
        return Err(ControlPlaneRegistryError::MissingRegistryDescriptor(
            format!(
                "D3 admission inventory changed (count {}, SHA-256 {inventory_fingerprint}); review every route class before admission",
                registered.len(),
            ),
        ));
    }
    let mut classes = BTreeMap::new();
    for command in registered {
        let class = classify_registered_tauri_route(&command).ok_or_else(|| {
            ControlPlaneRegistryError::MissingRegistryDescriptor(format!(
                "D3 admission class is missing for registered Tauri command '{command}'"
            ))
        })?;
        classes.insert(command, class);
    }
    Ok(classes)
});

pub fn tauri_route_admission_class(command: &str) -> Option<TauriRouteAdmissionClass> {
    TAURI_ROUTE_ADMISSION_CLASSES
        .as_ref()
        .ok()?
        .get(command)
        .copied()
}

fn classify_registered_tauri_route(command: &str) -> Option<TauriRouteAdmissionClass> {
    use TauriRouteAdmissionClass as Class;
    let class = if matches!(
        command,
        "commit_prepared_media_asset_relink"
            | "commit_prepared_media_assets"
            | "commit_prepared_video_file_layer"
            | "commit_prepared_still_image_layer"
            | "commit_prepared_local_media_layers"
            | "commit_prepared_bootstrap_vj_show"
            | "load_phase1_sample_project"
            | "run_phase1_smoke"
            | "set_blackout"
            | "set_all_blackout"
            | "set_video_blackout"
            | "set_video_output_blackout"
            | "take_over_standby"
            | "set_output_ownership_role"
            | "arm_output_ownership_role"
            | "load_effect_preset"
            | "load_effect_preset_for_target"
            | "load_sample_effect_preset"
            | "load_sample_effect_bundle"
            | "load_node_graph_preset_file"
            | "save_node_graph"
            | "remove_node_graph"
            | "remove_cue_list"
            | "add_timeline_cue_event"
            | "reconform_timeline_to_bpm"
            | "set_group_fixture_limits"
            | "set_video_cue_point"
            | "set_video_output_mapping_field"
            | "apply_builtin_video_isf_effect"
            | "apply_video_output_mapping_preset"
    ) {
        Class::Retired
    } else if matches!(
        command,
        "new_project"
            | "load_user_template"
            | "load_project"
            | "import_daslight_project"
            | "import_daslight_project_with_result"
            | "load_project_path"
            | "load_project_backup"
            | "load_startup_project"
    ) {
        Class::ProjectReplacement
    } else if matches!(
        command,
        "clear_project_history" | "undo_project_transaction" | "redo_project_transaction"
    ) {
        Class::ProjectHistory
    } else if matches!(command, "safety_blackout_engage_v1") {
        Class::SafetyMutation
    } else if matches!(
        command,
        "lock_project_operator_session"
            | "unlock_project_operator_session"
            | "adopt_project_publication_owner_v1"
            | "get_project_publication_receipt_v1"
            | "acknowledge_project_publication_receipt_v1"
            | "abandon_project_publication_v1"
            | "acknowledge_project_recovery_applied"
            | "load_project_checkpoint"
            | "begin_project_transaction"
            | "commit_project_transaction"
            | "cancel_project_transaction"
            | "query_project_transaction"
            | "adopt_project_transaction"
            | "acknowledge_project_transaction"
            | "register_project_transaction_owner"
            | "start_media_asset_operation"
            | "start_media_asset_availability_operation"
            | "cancel_media_asset_operation"
            | "get_media_asset_operation_terminal_result"
            | "get_video_clip_slot_operation_terminal_result"
            | "get_video_effect_catalog_operation_terminal_result"
            | "get_timeline_advanced_operation_terminal_result"
            | "get_timeline_follow_operation_terminal_result"
            | "get_timeline_follow_runtime"
            | "abort_timeline_follow"
            | "abort_timeline_follow_runtime_v1"
    ) {
        Class::RecoveryMaintenance
    } else if is_renderer_ticketed_project_mutation(command) {
        Class::RendererTicketedProjectMutation
    } else if is_backend_authoritative_project_mutation(command) {
        Class::BackendAuthoritativeProjectMutation
    } else if is_tauri_file_export_mutation(command) {
        Class::FileExportMutation
    } else if is_tauri_read_only_route(command) {
        Class::ReadOnly
    } else if is_tauri_runtime_mutation(command) {
        Class::RuntimeMutation
    } else {
        return None;
    };
    Some(class)
}

fn is_tauri_file_export_mutation(command: &str) -> bool {
    matches!(
        command,
        "install_application_update"
            | "save_project_v1"
            | "save_project_as_v1"
            | "save_user_template_v1"
            | "save_project_backup_v1"
            | "import_video_output_bitmap_mask"
            | "download_gdtf_from_url"
            | "download_gdtf_from_share"
            | "cache_gdtf_from_share"
            | "save_custom_fixture_profile"
            | "delete_project_backup"
            | "export_diagnostic_package"
            | "save_engine_telemetry_report"
            | "save_effect_preset"
            | "save_fixture_preset"
            | "save_midi_mappings"
            | "save_osc_mappings"
            | "save_node_graph_preset_file"
            | "save_stage_map_preset_file"
            | "save_video_output_mapping_preset_file"
    )
}

fn is_tauri_read_only_route(command: &str) -> bool {
    matches!(
        command,
        "capture_pane_window_placements"
            | "check_application_update"
            | "dmx_input_status"
            | "get_application_update_configuration"
            | "get_control_plane_canonical_registry"
            | "get_control_plane_operation_registry"
            | "get_control_plane_query_capabilities"
            | "get_control_plane_query_schema_catalog"
            | "get_debug_video_output_preview"
            | "get_debug_video_output_test_pattern"
            | "get_debug_video_preview"
            | "get_engine_telemetry_report"
            | "get_external_video_io_plans"
            | "get_external_video_transport_status"
            | "get_fixture_groups"
            | "get_fixture_profile_health"
            | "get_live_audio_input_capabilities"
            | "get_live_video_monitor_frame"
            | "get_media_asset_preview_frame"
            | "get_media_asset_thumbnail"
            | "get_operator_policy"
            | "get_operator_selection_context"
            | "get_output_ownership_status"
            | "get_project_authority_bundle"
            | "get_project_checkpoint"
            | "get_project_checkpoint_bundle"
            | "get_project_control_mappings"
            | "get_project_history_status"
            | "get_project_recovery_authority_status"
            | "get_snapshot"
            | "get_snapshot_delta"
            | "get_timeline_cue_audio_status"
            | "get_video_clip_slot_runtime"
            | "get_video_composition_plans"
            | "get_video_layer_thumbnail"
            | "get_video_layer_transition_runtime"
            | "get_video_output_render_plans"
            | "get_video_output_window_statuses"
            | "get_video_preview_diagnostics"
            | "get_video_runtime_status"
            | "get_visualizer_external_model_assets"
            | "get_visualizer_model_asset_cache_summary"
            | "get_visualizer_model_render_plans"
            | "get_visualizer_render_payload"
            | "get_visualizer_resolved_render_payload"
            | "get_visualizer_scene"
            | "get_vj_preview_transport"
            | "inspect_media_asset_availability"
            | "inspect_reserved_media_asset_availability"
            | "list_audio_input_devices"
            | "list_audio_output_devices"
            | "list_gdtf_fixture_cache"
            | "list_midi_inputs"
            | "list_midi_outputs"
            | "list_project_backups"
            | "list_serial_ports"
            | "list_show_lan_interfaces"
            | "list_verified_fixture_profiles"
            | "list_video_display_monitors"
            | "live_audio_input_backends"
            | "live_audio_input_levels"
            | "live_audio_input_status"
            | "midi_feedback_status"
            | "poll_control_plane_observation_events"
            | "poll_project_authority_bundle"
            | "preview_custom_fixture_profile"
            | "query_control_plane_output_ownership"
            | "query_control_plane_project_authority"
            | "query_control_plane_runtime_generations"
            | "query_display_add_lease_authority_v1"
            | "query_output_control_authority_v1"
            | "query_output_lease_authority_v1"
            | "query_timeline_follow_abort_authority_v1"
            | "query_timeline_transport_authority_v1"
            | "remote_access_urls"
            | "remote_control_status"
            | "search_gdtf_share"
            | "select_gdtf_file"
            | "select_standby_sync_directory"
            | "select_timeline_audio_clip_file"
            | "select_video_isf_file"
            | "select_video_source_file"
            | "select_video_source_files"
            | "standby_sync_status"
            | "video_audio_monitor_status"
            | "video_output_recording_status"
    )
}

fn is_tauri_runtime_mutation(command: &str) -> bool {
    matches!(
        command,
        "acquire_output_lease_v2"
            | "add_display_output_v2"
            | "add_local_media_layers"
            | "add_still_image_layer"
            | "add_video_file_layer"
            | "analyze_timeline_audio_clip_path"
            | "arm_output_control_v2"
            | "begin_media_asset_preview"
            | "bootstrap_vj_show"
            | "cancel_queued_video_clip_slot_authoritative"
            | "clear_cue_live_modifier"
            | "clear_fixture_flags"
            | "clear_programmer"
            | "clear_vj_preview"
            | "close_open_video_output_windows"
            | "close_pane_window"
            | "close_video_output_window"
            | "connect_midi_clock"
            | "connect_midi_control"
            | "connect_midi_feedback"
            | "create_custom_fixture_profile"
            | "disconnect_midi_clock"
            | "disconnect_midi_control"
            | "disconnect_midi_feedback"
            | "disconnect_remote_client"
            | "discover_art_rdm_devices"
            | "discover_usb_rdm_devices"
            | "enable_output_control_v2"
            | "end_media_asset_preview"
            | "fade_video_layer_opacity"
            | "finalize_prepared_media_asset_relink"
            | "finalize_prepared_media_assets"
            | "force_transfer_output_lease_v2"
            | "import_gdtf"
            | "jump_video_cue_point"
            | "jump_video_cue_point_relative"
            | "launch_video_clip_slot_authoritative"
            | "launch_video_layer_transition_bus_authoritative"
            | "learn_dmx_control"
            | "learn_midi_control"
            | "learn_osc_control"
            | "load_custom_fixture_profile"
            | "load_gdtf_model_file"
            | "load_gdtf_wheel_media"
            | "load_midi_mappings"
            | "load_osc_mappings"
            | "load_stage_map_preset_file"
            | "load_verified_fixture_profile"
            | "open_pane_window"
            | "open_video_output_window"
            | "play_video_layer_audio_monitor"
            | "prepare_local_media_assets"
            | "prepare_media_asset_relink"
            | "prepare_reserved_media_asset_relink"
            | "prepare_reserved_media_assets"
            | "pulse_video_layer_isf_event"
            | "queue_video_clip_slot_authoritative"
            | "recover_output_lease_v2"
            | "refresh_video_layer_metadata"
            | "release_blackout_output_control_v2"
            | "release_cue"
            | "release_video_layer_transition_bus_authoritative"
            | "relink_media_asset"
            | "relinquish_output_lease_v2"
            | "renew_output_lease_v2"
            | "reset_engine_telemetry"
            | "rotate_dj_link_token"
            | "scale_timeline_loop"
            | "seek_direct_child_timeline"
            | "seek_timeline"
            | "seek_timeline_beat"
            | "seek_video_clip_slot_authoritative"
            | "seek_vj_preview"
            | "send_art_rdm_request"
            | "send_dmx_routes_test_frame"
            | "send_dmx_test_frame"
            | "send_midi_feedback"
            | "send_usb_rdm_request"
            | "set_auto_vj_armed"
            | "set_auto_vj_hold"
            | "set_bpm"
            | "set_cue_fade_paused"
            | "set_cue_live_modifier"
            | "set_direct_child_timeline_playing"
            | "set_display_output_window_open_v2"
            | "set_fixture_highlight"
            | "set_fixture_park"
            | "set_fixture_solo"
            | "set_group_highlight"
            | "set_group_park"
            | "set_group_solo"
            | "set_group_strobe"
            | "set_group_submaster"
            | "set_lighting_master"
            | "set_machine_timeline_cue_audio_settings"
            | "set_midi_feedback_auto"
            | "set_operator_feature_fader"
            | "set_operator_selection_context"
            | "set_playback_executor_level"
            | "set_playback_master"
            | "set_program_audio_handoff_config"
            | "set_programmer_attribute"
            | "set_programmer_fixture_attribute_batch"
            | "set_programmer_group_attribute"
            | "set_programmer_mode"
            | "set_timeline_loop_enabled"
            | "set_timeline_playing"
            | "set_timeline_transport_playing_runtime_v1"
            | "set_video_layer_audio_monitor_volume"
            | "set_video_master_opacity"
            | "set_vj_preview_playing"
            | "set_vj_preview_speed"
            | "stage_vj_preview_layer"
            | "start_art_rdm_full_discovery"
            | "start_dmx_input"
            | "start_live_audio_input"
            | "start_osc_input"
            | "start_remote_control"
            | "start_standby_sync"
            | "start_video_output_recording"
            | "stop_dmx_input"
            | "stop_live_audio_input"
            | "stop_osc_input"
            | "stop_remote_control"
            | "stop_standby_sync"
            | "stop_video_layer_audio_monitor"
            | "stop_video_output_recording"
            | "sync_ableton_link_clock"
            | "sync_external_video_transports"
            | "sync_ltc_timecode"
            | "sync_open_video_output_windows"
            | "sync_video_output_window"
            | "take_open_project_paths"
            | "take_over_output_control_v2"
            | "tap_bpm"
            | "trigger_cue"
            | "trigger_cue_list_next"
            | "trigger_cue_list_previous"
            | "trigger_next_cue"
            | "trigger_playback_executor"
            | "trigger_previous_cue"
            | "use_fixture_profile"
    )
}

fn is_backend_authoritative_project_mutation(command: &str) -> bool {
    matches!(
        command,
        "commit_prepared_media_assets_authoritative"
            | "commit_prepared_media_asset_relink_authoritative"
            | "commit_prepared_video_file_layer_authoritative"
            | "commit_prepared_still_image_layer_authoritative"
            | "commit_prepared_local_media_layers_authoritative"
            | "commit_prepared_bootstrap_vj_show_authoritative"
            | "create_video_clip_slot_authoritative"
            | "assign_video_clip_slot_asset_authoritative"
            | "update_video_clip_slot_authoritative"
            | "remove_video_clip_slot_authoritative"
            | "reorder_video_clip_slots_authoritative"
            | "duplicate_video_clip_slot_authoritative"
            | "set_default_video_clip_slot_authoritative"
            | "import_and_assign_video_clip_slots_authoritative"
            | "apply_video_effect_catalog_authoritative"
            | "apply_timeline_advanced_authoritative"
            | "set_project_control_mappings"
            | "set_video_layer_isf_effect"
            | "add_video_layer_isf_effect"
            | "add_builtin_video_isf_effect"
            | "move_video_layer_isf_effect"
            | "remove_video_layer_isf_effect"
            | "set_video_layer_isf_effect_enabled"
            | "reset_video_layer_isf_effect"
            | "set_video_layer_isf_control"
            | "set_effect_enabled"
            | "create_empty_cue"
            | "delete_cue_list"
            | "reorder_cue_lists"
    )
}

fn is_renderer_ticketed_project_mutation(command: &str) -> bool {
    matches!(
        command,
        "analyze_audio_file"
            | "clear_timeline_audio"
            | "add_timeline_audio_clip"
            | "update_timeline_audio_clip"
            | "remove_timeline_audio_clip"
            | "set_timeline_audio_master"
            | "set_timeline_metronome"
            | "patch_fixtures"
            | "remove_fixture"
            | "repair_fixture_profile"
            | "set_fixture_patch"
            | "set_fixture_limits"
            | "set_fixture_groups"
            | "create_fixture_group"
            | "rename_fixture_group"
            | "recolor_fixture_group"
            | "delete_fixture_group"
            | "undo_delete_fixture_group"
            | "set_attribute"
            | "set_fixture_attribute_batch"
            | "set_group_attribute"
            | "commit_programmer"
            | "set_fixture_transform"
            | "set_stage_map_config"
            | "set_output_config"
            | "set_dmx_outputs"
            | "create_cue_from_current"
            | "create_cue_list"
            | "rename_cue_list"
            | "set_cue_list"
            | "create_reference_palette"
            | "update_reference_palette"
            | "remove_reference_palette"
            | "apply_reference_palette"
            | "set_cue_palette_targets"
            | "create_playback_executor"
            | "update_playback_executor"
            | "remove_playback_executor"
            | "update_cue_from_current"
            | "update_cue_from_current_batch"
            | "set_cue_effect_targets"
            | "add_cue_owned_effect"
            | "set_cue_metadata"
            | "set_cue_child_timeline"
            | "set_cue_steps"
            | "set_cue_color"
            | "set_cue_live_modifier_defaults"
            | "set_group_color"
            | "move_cue"
            | "move_cue_between_scene_banks_batch"
            | "duplicate_cue"
            | "remove_cue"
            | "set_timeline_cue_event"
            | "add_timeline_scene_block"
            | "set_timeline_scene_block"
            | "remove_timeline_scene_block"
            | "snap_timeline_items"
            | "remove_timeline_event"
            | "add_timeline_layer"
            | "update_timeline_layer"
            | "remove_timeline_layer"
            | "reorder_timeline_layers"
            | "add_timeline_automation"
            | "add_timeline_group_automation"
            | "set_timeline_automation"
            | "add_timeline_video_automation"
            | "set_timeline_video_automation"
            | "set_timeline_automation_enabled"
            | "remove_timeline_automation"
            | "add_video_input_layer"
            | "duplicate_video_layer"
            | "remove_video_layer"
            | "set_video_layer_order"
            | "set_video_layer_label"
            | "set_video_layer_state"
            | "launch_video_clip"
            | "take_video_clip"
            | "set_auto_vj_config"
            | "set_video_ab_mix"
            | "stop_video_clip"
            | "add_video_cue_point"
            | "remove_video_cue_point"
            | "set_video_layer_blend_mode"
            | "add_video_composition"
            | "remove_video_composition"
            | "set_video_composition_layers"
            | "add_video_output"
            | "remove_video_output"
            | "set_video_output_config"
            | "set_video_output_enabled"
            | "set_video_output_routing"
            | "set_video_output_opacity"
            | "fade_video_output_opacity"
            | "set_video_output_mapping"
            | "save_video_output_mapping_preset"
            | "remove_video_output_mapping_preset"
            | "load_video_output_mapping_preset_file"
            | "add_lfo_effect"
            | "add_position_wave_effect"
            | "add_color_effect"
            | "add_chaser_effect"
            | "add_move_effect"
            | "add_value_effect"
            | "add_curve_effect"
            | "add_mapping_effect"
            | "add_color_mapping_effect"
            | "update_lfo_effect"
            | "update_position_wave_effect"
            | "update_color_effect"
            | "update_chaser_effect"
            | "update_move_effect"
            | "update_value_effect"
            | "update_curve_effect"
            | "update_mapping_effect"
            | "update_color_mapping_effect"
            | "set_node_graph_enabled"
            | "set_effect_video_target_position"
            | "move_effect"
            | "duplicate_effect"
            | "remove_effect"
            | "load_fixture_preset"
            | "load_fixture_preset_for_group"
            | "load_fixture_preset_for_all_matching"
            | "save_stage_map_preset"
            | "apply_stage_map_preset"
            | "remove_stage_map_preset"
            | "import_stage_map_preset"
            | "add_stage_object"
            | "set_stage_object"
            | "remove_stage_object"
            | "set_touch_surface"
            | "set_operator_policy"
            | "clear_operator_policy"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlPlaneRegistryError {
    HandlerMarkerMissing,
    MultipleHandlerMarkers,
    HandlerClosingBracketMissing,
    InvalidCommandName(String),
    DuplicateCommandName(String),
    InvalidFrontendInvokeManifest,
    DuplicateFrontendInvokeCommand(String),
    UnregisteredFrontendInvokeCommand(String),
    InvalidKeyboardShortcutSourceManifest,
    DuplicateKeyboardShortcutSource(String),
    TooManyOperations(usize),
    MissingRegistryDescriptor(String),
    UnexpectedRegistryDescriptor(String),
    DescriptorInvalid(String),
    CanonicalRegistryInvalid(String),
    MissingCanonicalSource(String),
    UnexpectedCanonicalSource(String),
    MissingWindowBinding,
}

impl fmt::Display for ControlPlaneRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HandlerMarkerMissing => {
                formatter.write_str("Tauri generate_handler marker is missing")
            }
            Self::MultipleHandlerMarkers => {
                formatter.write_str("exactly one production Tauri generate_handler is required")
            }
            Self::HandlerClosingBracketMissing => {
                formatter.write_str("Tauri generate_handler closing bracket is missing")
            }
            Self::InvalidCommandName(name) => {
                write!(formatter, "invalid Tauri command name: {name}")
            }
            Self::DuplicateCommandName(name) => {
                write!(formatter, "duplicate Tauri command name: {name}")
            }
            Self::InvalidFrontendInvokeManifest => {
                formatter.write_str("frontend invoke manifest is invalid or not byte-sorted")
            }
            Self::DuplicateFrontendInvokeCommand(name) => {
                write!(formatter, "duplicate frontend invoke command: {name}")
            }
            Self::UnregisteredFrontendInvokeCommand(name) => {
                write!(
                    formatter,
                    "frontend invoke command is not registered with Tauri: {name}"
                )
            }
            Self::InvalidKeyboardShortcutSourceManifest => formatter.write_str(
                "keyboard shortcut source manifest is malformed, non-canonical, or out of contract",
            ),
            Self::DuplicateKeyboardShortcutSource(name) => {
                write!(formatter, "duplicate keyboard shortcut source: {name}")
            }
            Self::TooManyOperations(count) => write!(
                formatter,
                "operation registry exceeds its {MAX_REGISTERED_OPERATIONS} entry bound: {count}"
            ),
            Self::MissingRegistryDescriptor(name) => write!(
                formatter,
                "registered Tauri command has no descriptor: {name}"
            ),
            Self::UnexpectedRegistryDescriptor(name) => {
                write!(formatter, "descriptor is not registered with Tauri: {name}")
            }
            Self::DescriptorInvalid(message) => {
                write!(formatter, "invalid control-plane descriptor: {message}")
            }
            Self::CanonicalRegistryInvalid(message) => {
                write!(
                    formatter,
                    "invalid canonical control-plane registry: {message}"
                )
            }
            Self::MissingCanonicalSource(key) => {
                write!(formatter, "canonical source inventory is missing: {key}")
            }
            Self::UnexpectedCanonicalSource(key) => {
                write!(
                    formatter,
                    "canonical source inventory is not expected by v2: {key}"
                )
            }
            Self::MissingWindowBinding => formatter
                .write_str("control-plane discovery requires a window-bound Tauri invocation"),
        }
    }
}

impl std::error::Error for ControlPlaneRegistryError {}

/// Returns the deterministic, bounded inventory visible to a local Tauri
/// window.  The `window_label` argument makes the security boundary explicit
/// without granting any external or headless adapter.
pub fn registry_for_window(
    window_label: &str,
) -> Result<OperationRegistry, ControlPlaneRegistryError> {
    if window_label.is_empty() {
        return Err(ControlPlaneRegistryError::MissingWindowBinding);
    }
    registry()
}

/// Returns the version-2 canonical/source inventory for an injected local
/// Tauri window. This is discovery metadata only; it creates no external
/// adapter and carries no caller-owner argument.
pub fn canonical_registry_for_window(
    window_label: &str,
) -> Result<CanonicalControlPlaneRegistry, ControlPlaneRegistryError> {
    if window_label.is_empty() {
        return Err(ControlPlaneRegistryError::MissingWindowBinding);
    }
    canonical_registry()
}

pub fn registry() -> Result<OperationRegistry, ControlPlaneRegistryError> {
    VALIDATED_REGISTRY.clone()
}

pub fn canonical_registry() -> Result<CanonicalControlPlaneRegistry, ControlPlaneRegistryError> {
    VALIDATED_CANONICAL_REGISTRY.clone()
}

fn build_registry() -> Result<OperationRegistry, ControlPlaneRegistryError> {
    let command_names = registered_tauri_command_names_from_source(MAIN_RS_SOURCE)?;
    let frontend_invoke_names = frontend_invoke_names_from_manifest(FRONTEND_INVOKE_MANIFEST)?;
    for name in &frontend_invoke_names {
        if command_names.binary_search(name).is_err() {
            return Err(ControlPlaneRegistryError::UnregisteredFrontendInvokeCommand(name.clone()));
        }
    }
    let engine_descriptors = control_plane_engine_command_descriptors();
    let remote_descriptors = control_plane_remote_descriptors();
    let midi_osc_dmx_descriptors = control_plane_midi_osc_dmx_descriptors();
    let total_operations = command_names.len()
        + engine_descriptors.len()
        + remote_descriptors.len()
        + midi_osc_dmx_descriptors.len()
        + frontend_invoke_names.len();
    if total_operations > MAX_REGISTERED_OPERATIONS {
        return Err(ControlPlaneRegistryError::TooManyOperations(
            total_operations,
        ));
    }
    let mut operations = command_names
        .iter()
        .map(|name| descriptor_for_command(name))
        .collect::<Vec<_>>();
    operations.extend(engine_descriptors);
    operations.extend(remote_descriptors);
    operations.extend(midi_osc_dmx_descriptors);
    operations.extend(
        frontend_invoke_names
            .iter()
            .map(|name| unavailable_frontend_invoke_descriptor(name)),
    );
    operations.sort_by(|left, right| {
        (left.source_family, &left.source_id).cmp(&(right.source_family, &right.source_id))
    });
    let registry = OperationRegistry {
        schema: SchemaIdentity::registry(),
        operations,
    };
    registry
        .validate()
        .map_err(|error| ControlPlaneRegistryError::DescriptorInvalid(error.to_string()))?;
    verify_registry_exact_set(&command_names, &registry)?;
    Ok(registry)
}

/// Builds v2 solely from the already validated exact v1 source inventory.
/// In particular, the v1 descriptor's source risk is deliberately not read as
/// canonical authority: only the explicit reviewed query table below creates
/// a canonical operation.
fn build_canonical_registry() -> Result<CanonicalControlPlaneRegistry, ControlPlaneRegistryError> {
    let legacy = registry()?;
    let keyboard_shortcut_sources =
        keyboard_shortcut_source_ids_from_manifest(KEYBOARD_SHORTCUT_SOURCE_MANIFEST)?;
    let mut canonical_operations = legacy
        .operations
        .iter()
        .filter(|descriptor| descriptor.source_family == OperationSourceFamily::TauriCommand)
        .filter_map(canonical_descriptor_for_source)
        .collect::<Vec<_>>();
    canonical_operations.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));

    let mut source_inventory = legacy
        .operations
        .iter()
        .map(canonical_source_inventory_descriptor)
        .collect::<Vec<_>>();
    source_inventory.extend(keyboard_shortcut_sources.iter().map(|(family, source_id)| {
        keyboard_shortcut_source_inventory_descriptor(*family, source_id)
    }));
    source_inventory.sort_by(|left, right| left.source_key.cmp(&right.source_key));

    let mut canonical = CanonicalControlPlaneRegistry {
        schema: CanonicalControlPlaneRegistry::schema_identity(),
        canonical_operations,
        source_inventory,
    };
    canonical
        .populate_derived_adapters()
        .map_err(canonical_registry_error)?;
    verify_canonical_registry_exact_sources(&legacy, &canonical)?;
    Ok(canonical)
}

fn canonical_source_inventory_descriptor(
    descriptor: &OperationDescriptor,
) -> SourceInventoryDescriptor {
    let source_family = CanonicalSourceFamily::from(descriptor.source_family);
    let source_key = SourceKey::new(source_family, descriptor.source_id.clone());
    let disposition = match source_family {
        CanonicalSourceFamily::TauriCommand => {
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
        CanonicalSourceFamily::FrontendInvoke => SourceDisposition::AliasOfSource {
            target: SourceKey::new(
                CanonicalSourceFamily::TauriCommand,
                descriptor.source_id.clone(),
            ),
        },
        CanonicalSourceFamily::EngineCommand
            if descriptor.source_id == "set_effect_enabled_published" =>
        {
            SourceDisposition::InternalStepOf {
                target: SourceKey::new(CanonicalSourceFamily::TauriCommand, "set_effect_enabled"),
            }
        }
        CanonicalSourceFamily::EngineCommand
            if descriptor.source_id == "set_timeline_playing_published" =>
        {
            SourceDisposition::InternalStepOf {
                target: SourceKey::new(
                    CanonicalSourceFamily::TauriCommand,
                    "set_timeline_transport_playing_runtime_v1",
                ),
            }
        }
        CanonicalSourceFamily::EngineCommand if descriptor.source_id == "abort_timeline_follow" => {
            SourceDisposition::InternalStepOf {
                target: SourceKey::new(
                    CanonicalSourceFamily::TauriCommand,
                    "abort_timeline_follow_runtime_v1",
                ),
            }
        }
        CanonicalSourceFamily::EngineCommand
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
            reason: effect_enabled_unclassified_reason(descriptor)
                .unwrap_or_else(|| unclassified_reason(source_family))
                .to_string(),
        },
    };
    SourceInventoryDescriptor {
        schema: SourceInventoryDescriptor::schema_identity(),
        binding_id: source_key.binding_id(),
        source_key,
        role: SourceRole::for_family(source_family),
        raw_request_schema: descriptor.request_schema.clone(),
        raw_response_schema: descriptor.response_schema.clone(),
        disposition,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyboardShortcutSourceManifest {
    schema_version: u16,
    keyboard_app: Vec<String>,
    keyboard_project_file: Vec<String>,
}

fn keyboard_shortcut_source_ids_from_manifest(
    source: &str,
) -> Result<Vec<(CanonicalSourceFamily, String)>, ControlPlaneRegistryError> {
    let manifest = serde_json::from_str::<KeyboardShortcutSourceManifest>(source)
        .map_err(|_| ControlPlaneRegistryError::InvalidKeyboardShortcutSourceManifest)?;
    if manifest.schema_version != KEYBOARD_SHORTCUT_SOURCE_MANIFEST_SCHEMA_VERSION {
        return Err(ControlPlaneRegistryError::InvalidKeyboardShortcutSourceManifest);
    }
    let canonical_bytes = serde_json::to_string_pretty(&manifest)
        .map_err(|_| ControlPlaneRegistryError::InvalidKeyboardShortcutSourceManifest)?;
    if source != format!("{canonical_bytes}\n") {
        return Err(ControlPlaneRegistryError::InvalidKeyboardShortcutSourceManifest);
    }

    let mut sources = Vec::with_capacity(
        KEYBOARD_APP_SHORTCUT_SOURCE_COUNT + KEYBOARD_PROJECT_FILE_SHORTCUT_SOURCE_COUNT,
    );
    append_keyboard_shortcut_source_ids(
        CanonicalSourceFamily::KeyboardApp,
        &manifest.keyboard_app,
        KEYBOARD_APP_SHORTCUT_SOURCE_COUNT,
        &mut sources,
    )?;
    append_keyboard_shortcut_source_ids(
        CanonicalSourceFamily::KeyboardProjectFile,
        &manifest.keyboard_project_file,
        KEYBOARD_PROJECT_FILE_SHORTCUT_SOURCE_COUNT,
        &mut sources,
    )?;
    Ok(sources)
}

fn append_keyboard_shortcut_source_ids(
    family: CanonicalSourceFamily,
    source_ids: &[String],
    expected_count: usize,
    sources: &mut Vec<(CanonicalSourceFamily, String)>,
) -> Result<(), ControlPlaneRegistryError> {
    if source_ids.len() != expected_count {
        return Err(ControlPlaneRegistryError::InvalidKeyboardShortcutSourceManifest);
    }
    let mut distinct = BTreeSet::new();
    for source_id in source_ids {
        let source_key = SourceKey::new(family, source_id.clone());
        if source_key.validate().is_err() {
            return Err(ControlPlaneRegistryError::InvalidKeyboardShortcutSourceManifest);
        }
        if !distinct.insert(source_id.as_str()) {
            return Err(ControlPlaneRegistryError::DuplicateKeyboardShortcutSource(
                source_key.binding_id(),
            ));
        }
        sources.push((family, source_id.clone()));
    }
    if source_ids
        .windows(2)
        .any(|pair| pair[0].as_str() >= pair[1].as_str())
    {
        return Err(ControlPlaneRegistryError::InvalidKeyboardShortcutSourceManifest);
    }
    Ok(())
}

fn keyboard_shortcut_source_inventory_descriptor(
    family: CanonicalSourceFamily,
    source_id: &str,
) -> SourceInventoryDescriptor {
    let source_key = SourceKey::new(family, source_id);
    let family_name = match family {
        CanonicalSourceFamily::KeyboardApp => "keyboard_app",
        CanonicalSourceFamily::KeyboardProjectFile => "keyboard_project_file",
        _ => unreachable!("only keyboard source families use this descriptor builder"),
    };
    let raw_schema = |direction| SchemaIdentity {
        name: format!("syndocal.inventory.{family_name}.{source_id}.{direction}"),
        version: KEYBOARD_SHORTCUT_SOURCE_MANIFEST_SCHEMA_VERSION,
    };
    // Shortcut sources intentionally have neither a canonical operation nor
    // an adapter. Some current shortcuts mutate project/runtime state, so
    // presentation-only would be both inaccurate and unsafe.
    SourceInventoryDescriptor {
        schema: SourceInventoryDescriptor::schema_identity(),
        binding_id: source_key.binding_id(),
        source_key,
        role: SourceRole::for_family(family),
        raw_request_schema: raw_schema("request"),
        raw_response_schema: raw_schema("response"),
        disposition: if family == CanonicalSourceFamily::KeyboardApp
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
    }
}

fn unclassified_reason(family: CanonicalSourceFamily) -> &'static str {
    match family {
        CanonicalSourceFamily::TauriCommand => "unreviewed tauri command",
        CanonicalSourceFamily::EngineCommand => "unreviewed engine command",
        CanonicalSourceFamily::RemoteInputEvent
        | CanonicalSourceFamily::RemoteClientRequest
        | CanonicalSourceFamily::RemoteWireOperation => "unreviewed remote ingress",
        CanonicalSourceFamily::MidiControlMessage
        | CanonicalSourceFamily::MidiControlAction
        | CanonicalSourceFamily::MidiClockEvent
        | CanonicalSourceFamily::MidiControlEvent => "unreviewed midi ingress",
        CanonicalSourceFamily::OscControlAction | CanonicalSourceFamily::OscInputEvent => {
            "unreviewed osc ingress"
        }
        CanonicalSourceFamily::DmxInputProtocol | CanonicalSourceFamily::DmxInputEvent => {
            "unreviewed dmx ingress"
        }
        CanonicalSourceFamily::FrontendInvoke => "unreviewed frontend invocation",
        CanonicalSourceFamily::KeyboardApp => "unreviewed keyboard app shortcut",
        CanonicalSourceFamily::KeyboardProjectFile => "unreviewed keyboard project-file shortcut",
    }
}

fn canonical_registry_error(error: CanonicalRegistryValidationError) -> ControlPlaneRegistryError {
    ControlPlaneRegistryError::CanonicalRegistryInvalid(error.to_string())
}

pub fn verify_canonical_registry_exact_sources(
    legacy: &OperationRegistry,
    canonical: &CanonicalControlPlaneRegistry,
) -> Result<(), ControlPlaneRegistryError> {
    let mut expected = legacy
        .operations
        .iter()
        .map(|descriptor| {
            SourceKey::new(
                CanonicalSourceFamily::from(descriptor.source_family),
                descriptor.source_id.clone(),
            )
        })
        .collect::<BTreeSet<_>>();
    expected.extend(
        keyboard_shortcut_source_ids_from_manifest(KEYBOARD_SHORTCUT_SOURCE_MANIFEST)?
            .into_iter()
            .map(|(family, source_id)| SourceKey::new(family, source_id)),
    );
    let actual = canonical
        .source_inventory
        .iter()
        .map(|source| source.source_key.clone())
        .collect::<BTreeSet<_>>();
    if let Some(missing) = expected.difference(&actual).next() {
        return Err(ControlPlaneRegistryError::MissingCanonicalSource(
            missing.binding_id(),
        ));
    }
    if let Some(unexpected) = actual.difference(&expected).next() {
        return Err(ControlPlaneRegistryError::UnexpectedCanonicalSource(
            unexpected.binding_id(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct ReviewedQueryOperation {
    operation_id: &'static str,
    class: OperationClass,
    domain_capability: OperationCapability,
}

#[derive(Debug, Clone, Copy)]
enum ReviewedCanonicalOperation {
    Query(ReviewedQueryOperation),
    SetEffectEnabled,
    SetTimelineTransportPlaying,
    AbortTimelineFollow,
    EngageSafetyBlackout,
    ReleaseBlackout,
    ArmOutputOwnership,
    TakeOverStandby,
    AddDisplayOutput,
    SetDisplayWindowOpen,
    EnableOutput,
    AcquireOutputLease,
    RenewOutputLease,
    RecoverOutputLease,
    RelinquishOutputLease,
    ForceTransferOutputLease,
    ReorderCueLists,
    DeleteCueList,
    CreateEmptyCue,
}

impl ReviewedCanonicalOperation {
    fn operation_id(self) -> &'static str {
        match self {
            Self::Query(query) => query.operation_id,
            Self::SetEffectEnabled => SET_EFFECT_ENABLED_OPERATION_ID,
            Self::SetTimelineTransportPlaying => TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID,
            Self::AbortTimelineFollow => TIMELINE_FOLLOW_ABORT_OPERATION_ID,
            Self::EngageSafetyBlackout => SAFETY_BLACKOUT_ENGAGE_OPERATION_ID,
            Self::ReleaseBlackout => OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
            Self::ArmOutputOwnership => OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
            Self::TakeOverStandby => OUTPUT_STANDBY_TAKEOVER_OPERATION_ID,
            Self::AddDisplayOutput => OUTPUT_DISPLAY_ADD_OPERATION_ID,
            Self::SetDisplayWindowOpen => OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID,
            Self::EnableOutput => OUTPUT_ENABLE_OPERATION_ID,
            Self::AcquireOutputLease => OUTPUT_LEASE_ACQUIRE_OPERATION_ID,
            Self::RenewOutputLease => OUTPUT_LEASE_RENEW_OPERATION_ID,
            Self::RecoverOutputLease => OUTPUT_LEASE_RECOVER_OPERATION_ID,
            Self::RelinquishOutputLease => OUTPUT_LEASE_RELINQUISH_OPERATION_ID,
            Self::ForceTransferOutputLease => OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID,
            Self::ReorderCueLists => CUE_LIST_REORDER_OPERATION_ID,
            Self::DeleteCueList => CUE_LIST_DELETE_OPERATION_ID,
            Self::CreateEmptyCue => EMPTY_CUE_CREATE_OPERATION_ID,
        }
    }
}

fn reviewed_canonical_operation(command: &str) -> Option<ReviewedCanonicalOperation> {
    if let Some(query) = reviewed_query_operation(command) {
        return Some(ReviewedCanonicalOperation::Query(query));
    }
    match command {
        "set_effect_enabled" => Some(ReviewedCanonicalOperation::SetEffectEnabled),
        "set_timeline_transport_playing_runtime_v1" => {
            Some(ReviewedCanonicalOperation::SetTimelineTransportPlaying)
        }
        "abort_timeline_follow_runtime_v1" => Some(ReviewedCanonicalOperation::AbortTimelineFollow),
        "safety_blackout_engage_v1" => Some(ReviewedCanonicalOperation::EngageSafetyBlackout),
        "release_blackout_output_control_v2" => Some(ReviewedCanonicalOperation::ReleaseBlackout),
        "arm_output_control_v2" => Some(ReviewedCanonicalOperation::ArmOutputOwnership),
        "take_over_output_control_v2" => Some(ReviewedCanonicalOperation::TakeOverStandby),
        "add_display_output_v2" => Some(ReviewedCanonicalOperation::AddDisplayOutput),
        "set_display_output_window_open_v2" => {
            Some(ReviewedCanonicalOperation::SetDisplayWindowOpen)
        }
        "enable_output_control_v2" => Some(ReviewedCanonicalOperation::EnableOutput),
        "acquire_output_lease_v2" => Some(ReviewedCanonicalOperation::AcquireOutputLease),
        "renew_output_lease_v2" => Some(ReviewedCanonicalOperation::RenewOutputLease),
        "recover_output_lease_v2" => Some(ReviewedCanonicalOperation::RecoverOutputLease),
        "relinquish_output_lease_v2" => Some(ReviewedCanonicalOperation::RelinquishOutputLease),
        "force_transfer_output_lease_v2" => {
            Some(ReviewedCanonicalOperation::ForceTransferOutputLease)
        }
        "reorder_cue_lists" => Some(ReviewedCanonicalOperation::ReorderCueLists),
        "delete_cue_list" => Some(ReviewedCanonicalOperation::DeleteCueList),
        "create_empty_cue" => Some(ReviewedCanonicalOperation::CreateEmptyCue),
        _ => None,
    }
}

fn canonical_descriptor_for_source(
    descriptor: &OperationDescriptor,
) -> Option<CanonicalOperationDescriptor> {
    let reviewed = reviewed_canonical_operation(&descriptor.source_id)?;
    let (class, capabilities, idempotency, adapter_policy, receipt_policy) = match reviewed {
        ReviewedCanonicalOperation::Query(query) => (
            query.class,
            vec![
                OperationCapability::ReadOnly,
                OperationCapability::LocalWindowBound,
                OperationCapability::AllowedDuringFullLock,
                query.domain_capability,
            ],
            OperationIdempotency::ReadOnly,
            AdapterPolicy::LocalWindowReadOnly,
            ReceiptPolicy::FailClosed,
        ),
        ReviewedCanonicalOperation::SetEffectEnabled => (
            OperationClass::Mutation,
            vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::AuthoritativeProjectMutation,
            ],
            OperationIdempotency::Mutating,
            AdapterPolicy::LocalWindowAuthoritativeMutation,
            ReceiptPolicy::ExactTerminalReceipt,
        ),
        ReviewedCanonicalOperation::ReorderCueLists => (
            OperationClass::Mutation,
            vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::AuthoritativeProjectMutation,
            ],
            OperationIdempotency::Mutating,
            AdapterPolicy::LocalWindowAuthoritativeMutation,
            ReceiptPolicy::ExactTerminalReceipt,
        ),
        ReviewedCanonicalOperation::DeleteCueList => (
            OperationClass::Mutation,
            vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::AuthoritativeProjectMutation,
            ],
            OperationIdempotency::Mutating,
            AdapterPolicy::LocalWindowAuthoritativeMutation,
            ReceiptPolicy::ExactTerminalReceipt,
        ),
        ReviewedCanonicalOperation::CreateEmptyCue => (
            OperationClass::Mutation,
            vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::AuthoritativeProjectMutation,
            ],
            OperationIdempotency::Mutating,
            AdapterPolicy::LocalWindowAuthoritativeMutation,
            ReceiptPolicy::ExactTerminalReceipt,
        ),
        ReviewedCanonicalOperation::SetTimelineTransportPlaying => (
            OperationClass::Mutation,
            vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::AuthoritativeRuntimeMutation,
            ],
            OperationIdempotency::Mutating,
            AdapterPolicy::LocalWindowAuthoritativeRuntimeMutation,
            ReceiptPolicy::ExactTerminalReceipt,
        ),
        ReviewedCanonicalOperation::AbortTimelineFollow => (
            OperationClass::Mutation,
            vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::AuthoritativeRuntimeMutation,
                OperationCapability::AllowedDuringFullLock,
            ],
            OperationIdempotency::Mutating,
            AdapterPolicy::LocalWindowRuntimeSafetyMutation,
            ReceiptPolicy::ExactTerminalReceipt,
        ),
        ReviewedCanonicalOperation::EngageSafetyBlackout => (
            OperationClass::Mutation,
            vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::SafetyBlackoutEngage,
                OperationCapability::AllowedDuringFullLock,
            ],
            OperationIdempotency::Mutating,
            AdapterPolicy::LocalWindowEmergencySafetyMutation,
            ReceiptPolicy::ExactTerminalReceipt,
        ),
        ReviewedCanonicalOperation::ReleaseBlackout
        | ReviewedCanonicalOperation::ArmOutputOwnership
        | ReviewedCanonicalOperation::TakeOverStandby
        | ReviewedCanonicalOperation::AddDisplayOutput
        | ReviewedCanonicalOperation::ForceTransferOutputLease => (
            OperationClass::Mutation,
            vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::OutputControl,
            ],
            OperationIdempotency::Mutating,
            AdapterPolicy::LocalWindowDangerousOutputControl,
            ReceiptPolicy::ExactTerminalReceipt,
        ),
        ReviewedCanonicalOperation::SetDisplayWindowOpen => (
            OperationClass::Mutation,
            vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::OutputControl,
            ],
            OperationIdempotency::Mutating,
            AdapterPolicy::LocalWindowOutputControl,
            ReceiptPolicy::ExactTerminalReceipt,
        ),
        ReviewedCanonicalOperation::EnableOutput
        | ReviewedCanonicalOperation::AcquireOutputLease
        | ReviewedCanonicalOperation::RenewOutputLease
        | ReviewedCanonicalOperation::RecoverOutputLease
        | ReviewedCanonicalOperation::RelinquishOutputLease => (
            OperationClass::Mutation,
            vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::OutputControl,
            ],
            OperationIdempotency::Mutating,
            AdapterPolicy::LocalWindowOutputControl,
            ReceiptPolicy::ExactTerminalReceipt,
        ),
    };
    Some(CanonicalOperationDescriptor {
        schema: CanonicalOperationDescriptor::schema_identity(),
        operation_id: reviewed.operation_id().to_string(),
        class,
        risk: if matches!(reviewed, ReviewedCanonicalOperation::EngageSafetyBlackout) {
            OperationRisk::S0
        } else if matches!(
            reviewed,
            ReviewedCanonicalOperation::ReleaseBlackout
                | ReviewedCanonicalOperation::ArmOutputOwnership
                | ReviewedCanonicalOperation::TakeOverStandby
                | ReviewedCanonicalOperation::AddDisplayOutput
                | ReviewedCanonicalOperation::SetDisplayWindowOpen
                | ReviewedCanonicalOperation::EnableOutput
                | ReviewedCanonicalOperation::AcquireOutputLease
                | ReviewedCanonicalOperation::RenewOutputLease
                | ReviewedCanonicalOperation::RecoverOutputLease
                | ReviewedCanonicalOperation::RelinquishOutputLease
                | ReviewedCanonicalOperation::ForceTransferOutputLease
        ) {
            OperationRisk::R4
        } else {
            OperationRisk::R0
        },
        capabilities,
        request_schema: descriptor.request_schema.clone(),
        response_schema: descriptor.response_schema.clone(),
        idempotency,
        audit: if matches!(
            reviewed,
            ReviewedCanonicalOperation::EngageSafetyBlackout
                | ReviewedCanonicalOperation::ReleaseBlackout
                | ReviewedCanonicalOperation::ArmOutputOwnership
                | ReviewedCanonicalOperation::TakeOverStandby
                | ReviewedCanonicalOperation::AddDisplayOutput
                | ReviewedCanonicalOperation::SetDisplayWindowOpen
                | ReviewedCanonicalOperation::EnableOutput
                | ReviewedCanonicalOperation::AcquireOutputLease
                | ReviewedCanonicalOperation::RenewOutputLease
                | ReviewedCanonicalOperation::RecoverOutputLease
                | ReviewedCanonicalOperation::RelinquishOutputLease
                | ReviewedCanonicalOperation::ForceTransferOutputLease
        ) {
            OperationAuditRequirement::Immutable
        } else {
            OperationAuditRequirement::NotApplicable
        },
        adapter_policy,
        receipt_policy,
        rate_policy: if matches!(
            reviewed,
            ReviewedCanonicalOperation::SetTimelineTransportPlaying
                | ReviewedCanonicalOperation::AbortTimelineFollow
                | ReviewedCanonicalOperation::EngageSafetyBlackout
                | ReviewedCanonicalOperation::ReleaseBlackout
                | ReviewedCanonicalOperation::ArmOutputOwnership
                | ReviewedCanonicalOperation::TakeOverStandby
                | ReviewedCanonicalOperation::AddDisplayOutput
                | ReviewedCanonicalOperation::SetDisplayWindowOpen
                | ReviewedCanonicalOperation::EnableOutput
                | ReviewedCanonicalOperation::AcquireOutputLease
                | ReviewedCanonicalOperation::RenewOutputLease
                | ReviewedCanonicalOperation::RecoverOutputLease
                | ReviewedCanonicalOperation::RelinquishOutputLease
                | ReviewedCanonicalOperation::ForceTransferOutputLease
        ) {
            RatePolicy::TokenBucket4PerSecondBurst8
        } else {
            RatePolicy::FailClosed
        },
        payload_policy: PayloadPolicy::FailClosed,
        consent_policy: if matches!(reviewed, ReviewedCanonicalOperation::EngageSafetyBlackout) {
            ConsentPolicy::NotRequiredForSafetyOnly
        } else if matches!(
            reviewed,
            ReviewedCanonicalOperation::EnableOutput
                | ReviewedCanonicalOperation::SetDisplayWindowOpen
                | ReviewedCanonicalOperation::AcquireOutputLease
                | ReviewedCanonicalOperation::RenewOutputLease
                | ReviewedCanonicalOperation::RecoverOutputLease
                | ReviewedCanonicalOperation::RelinquishOutputLease
        ) {
            ConsentPolicy::LocalExplicitAction
        } else if matches!(
            reviewed,
            ReviewedCanonicalOperation::ReleaseBlackout
                | ReviewedCanonicalOperation::ArmOutputOwnership
                | ReviewedCanonicalOperation::TakeOverStandby
                | ReviewedCanonicalOperation::AddDisplayOutput
                | ReviewedCanonicalOperation::ForceTransferOutputLease
        ) {
            ConsentPolicy::NativeDangerConfirmation
        } else {
            ConsentPolicy::FailClosed
        },
        derived_adapters: Vec::new(),
    })
}

fn reviewed_query_operation(command: &str) -> Option<ReviewedQueryOperation> {
    let (operation_id, class, domain_capability) = match command {
        "get_control_plane_operation_registry" => (
            "syndocal.query.control_plane.registry.v1",
            OperationClass::Discovery,
            OperationCapability::RegistryDiscovery,
        ),
        "get_control_plane_canonical_registry" => (
            "syndocal.query.control_plane.canonical_registry.v3",
            OperationClass::Discovery,
            OperationCapability::RegistryDiscovery,
        ),
        "get_control_plane_query_schema_catalog" => (
            "syndocal.query.control_plane.schemas.v1",
            OperationClass::Discovery,
            OperationCapability::RegistryDiscovery,
        ),
        "get_control_plane_query_capabilities" => (
            "syndocal.query.control_plane.capabilities.v1",
            OperationClass::Discovery,
            OperationCapability::RegistryDiscovery,
        ),
        "query_control_plane_project_authority" => (
            "syndocal.query.project.authority.v1",
            OperationClass::ProjectAuthority,
            OperationCapability::ProjectAuthorityRead,
        ),
        "query_control_plane_runtime_generations" => (
            "syndocal.query.runtime.generations.v1",
            OperationClass::RuntimeObservation,
            OperationCapability::RuntimeRead,
        ),
        "query_timeline_transport_authority_v1" => (
            TIMELINE_TRANSPORT_AUTHORITY_QUERY_OPERATION_ID,
            OperationClass::RuntimeObservation,
            OperationCapability::RuntimeRead,
        ),
        "query_timeline_follow_abort_authority_v1" => (
            TIMELINE_FOLLOW_ABORT_AUTHORITY_QUERY_OPERATION_ID,
            OperationClass::RuntimeObservation,
            OperationCapability::RuntimeRead,
        ),
        "query_output_control_authority_v1" => (
            OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID,
            OperationClass::RuntimeObservation,
            OperationCapability::RuntimeRead,
        ),
        "query_display_add_lease_authority_v1" => (
            OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
            OperationClass::RuntimeObservation,
            OperationCapability::RuntimeRead,
        ),
        "query_control_plane_output_ownership" => (
            "syndocal.query.output.ownership.v1",
            OperationClass::OutputOwnership,
            OperationCapability::OutputOwnershipRead,
        ),
        "list_video_display_monitors" => (
            "syndocal.query.video.display_monitors.v1",
            OperationClass::RuntimeObservation,
            OperationCapability::RuntimeRead,
        ),
        "poll_control_plane_observation_events" => (
            "syndocal.query.events.observations.v1",
            OperationClass::RuntimeObservation,
            OperationCapability::RuntimeRead,
        ),
        _ => return None,
    };
    Some(ReviewedQueryOperation {
        operation_id,
        class,
        domain_capability,
    })
}

fn descriptor_for_command(operation_id: &str) -> OperationDescriptor {
    if let Some(reviewed) = reviewed_query_operation(operation_id) {
        return OperationDescriptor {
            schema: SchemaIdentity::descriptor(),
            operation_id: reviewed.operation_id.to_string(),
            source_family: OperationSourceFamily::TauriCommand,
            source_id: operation_id.to_string(),
            class: reviewed.class,
            risk: OperationRisk::R0,
            capabilities: vec![
                OperationCapability::ReadOnly,
                OperationCapability::LocalWindowBound,
                // Registry discovery remains observable during Full Lock and has
                // no mutation capability.
                OperationCapability::AllowedDuringFullLock,
                reviewed.domain_capability,
            ],
            availability: OperationAvailability::LocalWindowOnly,
            idempotency: OperationIdempotency::ReadOnly,
            audit: OperationAuditRequirement::NotApplicable,
            request_schema: command_schema(reviewed.operation_id, "request"),
            response_schema: command_schema(reviewed.operation_id, "response"),
        };
    }
    if matches!(
        operation_id,
        "add_display_output_v2"
            | "set_display_output_window_open_v2"
            | "enable_output_control_v2"
            | "acquire_output_lease_v2"
            | "renew_output_lease_v2"
            | "recover_output_lease_v2"
            | "relinquish_output_lease_v2"
            | "force_transfer_output_lease_v2"
            | "reorder_cue_lists"
            | "delete_cue_list"
            | "create_empty_cue"
    ) {
        let source_id = operation_id;
        let reviewed = reviewed_canonical_operation(source_id)
            .expect("reviewed canonical command must have a descriptor mapping");
        return unavailable_descriptor_for_semantic_operation(source_id, reviewed.operation_id());
    }
    if operation_id == "set_effect_enabled" {
        // v1 has no safe executable-authoritative mutation descriptor. Keep
        // its raw inventory row fail-closed/R5 while preserving the precise
        // v2 schema identity needed for the reviewed direct projection.
        return unavailable_descriptor_for_semantic_operation(
            operation_id,
            SET_EFFECT_ENABLED_OPERATION_ID,
        );
    }
    if operation_id == "set_timeline_transport_playing_runtime_v1" {
        // v1 remains an inventory-only view. The v2 direct source is the
        // reviewed runtime mutation; its local rate/receipt policy is not
        // retroactively advertised by the legacy registry.
        return unavailable_descriptor_for_semantic_operation(
            operation_id,
            TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID,
        );
    }
    if operation_id == "abort_timeline_follow_runtime_v1" {
        return unavailable_descriptor_for_semantic_operation(
            operation_id,
            TIMELINE_FOLLOW_ABORT_OPERATION_ID,
        );
    }
    if operation_id == "safety_blackout_engage_v1" {
        return unavailable_descriptor_for_semantic_operation(
            operation_id,
            SAFETY_BLACKOUT_ENGAGE_OPERATION_ID,
        );
    }
    if operation_id == "release_blackout_output_control_v2" {
        return unavailable_descriptor_for_semantic_operation(
            operation_id,
            OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
        );
    }
    if operation_id == "arm_output_control_v2" {
        return unavailable_descriptor_for_semantic_operation(
            operation_id,
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
        );
    }
    if operation_id == "take_over_output_control_v2" {
        return unavailable_descriptor_for_semantic_operation(
            operation_id,
            OUTPUT_STANDBY_TAKEOVER_OPERATION_ID,
        );
    }
    unavailable_descriptor(operation_id)
}

fn unavailable_descriptor(operation_id: &str) -> OperationDescriptor {
    let semantic_operation_id = format!("syndocal.inventory.tauri.{operation_id}.v1");
    unavailable_descriptor_for_semantic_operation(operation_id, &semantic_operation_id)
}

fn unavailable_descriptor_for_semantic_operation(
    source_id: &str,
    semantic_operation_id: &str,
) -> OperationDescriptor {
    OperationDescriptor {
        schema: SchemaIdentity::descriptor(),
        operation_id: semantic_operation_id.to_string(),
        source_family: OperationSourceFamily::TauriCommand,
        source_id: source_id.to_string(),
        class: OperationClass::Mutation,
        risk: OperationRisk::R5,
        capabilities: vec![OperationCapability::LocalWindowBound],
        availability: OperationAvailability::Unavailable,
        idempotency: OperationIdempotency::Mutating,
        audit: OperationAuditRequirement::RequiredBeforeExternalExecution,
        request_schema: command_schema(semantic_operation_id, "request"),
        response_schema: command_schema(semantic_operation_id, "response"),
    }
}

fn effect_enabled_unclassified_reason(descriptor: &OperationDescriptor) -> Option<&'static str> {
    match (descriptor.source_family, descriptor.source_id.as_str()) {
        (OperationSourceFamily::EngineCommand, "set_effect_enabled") => {
            Some("legacy effect-enabled engine ingress lacks a versioned authored fence")
        }
        (OperationSourceFamily::RemoteInputEvent, "set_effect_enabled")
        | (OperationSourceFamily::RemoteWireOperation, "setEffectEnabled") => {
            Some("legacy remote effect-enabled ingress is fail-closed pending versioned migration")
        }
        (OperationSourceFamily::MidiControlAction, "effect_enabled")
        | (OperationSourceFamily::MidiControlEvent, "set_effect_enabled") => {
            Some("legacy midi effect-enabled ingress is fail-closed pending versioned migration")
        }
        (OperationSourceFamily::OscControlAction, "effect_enabled")
        | (OperationSourceFamily::OscInputEvent, "set_effect_enabled") => {
            Some("legacy osc effect-enabled ingress is fail-closed pending versioned migration")
        }
        (OperationSourceFamily::EngineCommand, "set_timeline_playing") => {
            Some("legacy timeline set-playing engine ingress lacks a versioned runtime identity")
        }
        (OperationSourceFamily::RemoteInputEvent, "set_timeline_playing")
        | (OperationSourceFamily::RemoteWireOperation, "setTimelinePlaying") => Some(
            "legacy remote timeline set-playing ingress is fail-closed pending versioned migration",
        ),
        (OperationSourceFamily::MidiControlAction, "timeline_play")
        | (OperationSourceFamily::MidiControlEvent, "set_timeline_playing") => Some(
            "legacy midi timeline set-playing ingress is fail-closed pending versioned migration",
        ),
        (OperationSourceFamily::OscControlAction, "timeline_play")
        | (OperationSourceFamily::OscInputEvent, "set_timeline_playing") => Some(
            "legacy osc timeline set-playing ingress is fail-closed pending versioned migration",
        ),
        _ => None,
    }
}

fn unavailable_frontend_invoke_descriptor(source_id: &str) -> OperationDescriptor {
    let operation_id = format!("syndocal.inventory.frontend.invoke.{source_id}.v1");
    OperationDescriptor {
        schema: SchemaIdentity::descriptor(),
        operation_id: operation_id.clone(),
        source_family: OperationSourceFamily::FrontendInvoke,
        source_id: source_id.to_string(),
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

fn frontend_invoke_names_from_manifest(
    source: &str,
) -> Result<Vec<String>, ControlPlaneRegistryError> {
    let names = serde_json::from_str::<Vec<String>>(source)
        .map_err(|_| ControlPlaneRegistryError::InvalidFrontendInvokeManifest)?;
    let mut prior: Option<&str> = None;
    for name in &names {
        if !is_lower_snake_case(name) {
            return Err(ControlPlaneRegistryError::InvalidFrontendInvokeManifest);
        }
        if let Some(prior) = prior {
            if name == prior {
                return Err(ControlPlaneRegistryError::DuplicateFrontendInvokeCommand(
                    name.clone(),
                ));
            }
            if name.as_str() < prior {
                return Err(ControlPlaneRegistryError::InvalidFrontendInvokeManifest);
            }
        }
        prior = Some(name);
    }
    Ok(names)
}

fn command_schema(operation_id: &str, direction: &str) -> SchemaIdentity {
    SchemaIdentity {
        name: format!("{operation_id}.{direction}"),
        version: if matches!(
            operation_id,
            OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
                | OUTPUT_OWNERSHIP_ARM_OPERATION_ID
                | OUTPUT_STANDBY_TAKEOVER_OPERATION_ID
                | OUTPUT_DISPLAY_ADD_OPERATION_ID
                | OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID
                | OUTPUT_ENABLE_OPERATION_ID
                | OUTPUT_LEASE_ACQUIRE_OPERATION_ID
                | OUTPUT_LEASE_RENEW_OPERATION_ID
                | OUTPUT_LEASE_RECOVER_OPERATION_ID
                | OUTPUT_LEASE_RELINQUISH_OPERATION_ID
                | OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID
        ) {
            OUTPUT_CONTROL_COMMAND_SCHEMA_VERSION
        } else {
            CONTROL_PLANE_SCHEMA_VERSION
        },
    }
}

/// Parse the single production `generate_handler!` declaration.  This is a
/// compile-time embedded source snapshot, so release inventory never depends
/// on disk I/O.  It is deliberately strict: comments, qualified paths, and
/// duplicate names cause startup discovery to fail instead of inventing an
/// ambiguous operation identity.
pub fn registered_tauri_command_names_from_source(
    source: &str,
) -> Result<Vec<String>, ControlPlaneRegistryError> {
    const MARKER: &str = "tauri::generate_handler![";
    if source.match_indices(MARKER).count() > 1 {
        return Err(ControlPlaneRegistryError::MultipleHandlerMarkers);
    }
    let marker_offset = source
        .find(MARKER)
        .ok_or(ControlPlaneRegistryError::HandlerMarkerMissing)?;
    let body_start = marker_offset + MARKER.len();
    let remainder = &source[body_start..];
    let body_end = remainder
        .find("](invoke)")
        .or_else(|| remainder.find("],\r\n"))
        .or_else(|| remainder.find("],\n"))
        .or_else(|| remainder.find("];"))
        .or_else(|| remainder.find("]) "))
        .or_else(|| remainder.find("])\r\n"))
        .or_else(|| remainder.find("])\n"))
        .or_else(|| remainder.find("])"))
        .or_else(|| remainder.find("\n]"))
        .ok_or(ControlPlaneRegistryError::HandlerClosingBracketMissing)?;
    let mut distinct = BTreeSet::new();
    let mut names = Vec::new();
    for raw_line in remainder[..body_end].lines() {
        let name = raw_line.trim().trim_end_matches(',').trim();
        if name.is_empty() {
            continue;
        }
        if !is_lower_snake_case(name) {
            return Err(ControlPlaneRegistryError::InvalidCommandName(
                name.to_string(),
            ));
        }
        if !distinct.insert(name.to_string()) {
            return Err(ControlPlaneRegistryError::DuplicateCommandName(
                name.to_string(),
            ));
        }
        names.push(name.to_string());
    }
    if names.is_empty() {
        return Err(ControlPlaneRegistryError::HandlerMarkerMissing);
    }
    names.sort();
    Ok(names)
}

pub fn verify_registry_exact_set(
    command_names: &[String],
    registry: &OperationRegistry,
) -> Result<(), ControlPlaneRegistryError> {
    let command_names = command_names.iter().collect::<BTreeSet<_>>();
    let descriptor_source_ids = registry
        .operations
        .iter()
        .filter(|descriptor| descriptor.source_family == OperationSourceFamily::TauriCommand)
        .map(|descriptor| &descriptor.source_id)
        .collect::<BTreeSet<_>>();
    if let Some(missing) = command_names.difference(&descriptor_source_ids).next() {
        return Err(ControlPlaneRegistryError::MissingRegistryDescriptor(
            (*missing).to_string(),
        ));
    }
    if let Some(unexpected) = descriptor_source_ids.difference(&command_names).next() {
        return Err(ControlPlaneRegistryError::UnexpectedRegistryDescriptor(
            (*unexpected).to_string(),
        ));
    }
    Ok(())
}

fn is_lower_snake_case(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tauri_route_admission_table_is_exact_and_fail_closed() {
        let names = registered_tauri_command_names_from_source(MAIN_RS_SOURCE).unwrap();
        let mut counts = std::collections::BTreeMap::new();
        for name in &names {
            let class = tauri_route_admission_class(name)
                .unwrap_or_else(|| panic!("registered route is unclassified: {name}"));
            *counts.entry(class).or_insert(0usize) += 1;
        }
        assert_eq!(names.len(), 478);
        assert_eq!(
            counts[&TauriRouteAdmissionClass::RendererTicketedProjectMutation],
            133
        );
        assert_eq!(
            counts[&TauriRouteAdmissionClass::BackendAuthoritativeProjectMutation],
            29
        );
        assert_eq!(counts[&TauriRouteAdmissionClass::ReadOnly], 86);
        assert_eq!(counts[&TauriRouteAdmissionClass::ProjectReplacement], 8);
        assert_eq!(counts[&TauriRouteAdmissionClass::ProjectHistory], 3);
        assert_eq!(counts[&TauriRouteAdmissionClass::RuntimeMutation], 142);
        assert_eq!(counts[&TauriRouteAdmissionClass::FileExportMutation], 20);
        assert_eq!(counts[&TauriRouteAdmissionClass::SafetyMutation], 1);
        assert_eq!(counts[&TauriRouteAdmissionClass::RecoveryMaintenance], 26);
        assert_eq!(counts[&TauriRouteAdmissionClass::Retired], 30);
        assert_eq!(tauri_route_admission_class("patch_fixture"), None);
        assert_eq!(
            tauri_route_admission_class("future_unreviewed_mutation"),
            None
        );
    }

    #[test]
    fn compiled_handler_and_registry_have_the_exact_same_set() {
        let names = registered_tauri_command_names_from_source(MAIN_RS_SOURCE).unwrap();
        let registry = registry().unwrap();
        const R0_ALLOWLIST: [&str; 13] = [
            "syndocal.query.control_plane.registry.v1",
            "syndocal.query.control_plane.canonical_registry.v3",
            "syndocal.query.control_plane.capabilities.v1",
            "syndocal.query.control_plane.schemas.v1",
            "syndocal.query.events.observations.v1",
            "syndocal.query.output.ownership.v1",
            "syndocal.query.project.authority.v1",
            "syndocal.query.runtime.generations.v1",
            "syndocal.query.video.display_monitors.v1",
            OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID,
            OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
            TIMELINE_FOLLOW_ABORT_AUTHORITY_QUERY_OPERATION_ID,
            TIMELINE_TRANSPORT_AUTHORITY_QUERY_OPERATION_ID,
        ];
        assert_eq!(names.len(), 478);
        const ENGINE_COMMAND_COUNT: usize = 263;
        const REMOTE_INPUT_EVENT_COUNT: usize = 51;
        const REMOTE_CLIENT_REQUEST_COUNT: usize = 7;
        const REMOTE_WIRE_OPERATION_COUNT: usize = 58;
        const REMOTE_OPERATION_COUNT: usize =
            REMOTE_INPUT_EVENT_COUNT + REMOTE_CLIENT_REQUEST_COUNT + REMOTE_WIRE_OPERATION_COUNT;
        const MIDI_CONTROL_MESSAGE_COUNT: usize = 4;
        const MIDI_CONTROL_ACTION_COUNT: usize = 51;
        const MIDI_CLOCK_EVENT_COUNT: usize = 6;
        const MIDI_CONTROL_EVENT_COUNT: usize = 47;
        const OSC_CONTROL_ACTION_COUNT: usize = 47;
        const OSC_INPUT_EVENT_COUNT: usize = 47;
        const DMX_INPUT_PROTOCOL_COUNT: usize = 2;
        const DMX_INPUT_EVENT_COUNT: usize = 2;
        const MIDI_OSC_DMX_OPERATION_COUNT: usize = MIDI_CONTROL_MESSAGE_COUNT
            + MIDI_CONTROL_ACTION_COUNT
            + MIDI_CLOCK_EVENT_COUNT
            + MIDI_CONTROL_EVENT_COUNT
            + OSC_CONTROL_ACTION_COUNT
            + OSC_INPUT_EVENT_COUNT
            + DMX_INPUT_PROTOCOL_COUNT
            + DMX_INPUT_EVENT_COUNT;
        const FRONTEND_INVOKE_COUNT: usize = 417;
        assert_eq!(MIDI_OSC_DMX_OPERATION_COUNT, 206);
        assert_eq!(
            registry.operations.len(),
            478 + ENGINE_COMMAND_COUNT
                + REMOTE_OPERATION_COUNT
                + MIDI_OSC_DMX_OPERATION_COUNT
                + FRONTEND_INVOKE_COUNT
        );
        assert_eq!(registry.operations.len(), 1480);
        verify_registry_exact_set(&names, &registry).unwrap();
        let r0 = registry
            .operations
            .iter()
            .filter(|descriptor| descriptor.risk == OperationRisk::R0)
            .collect::<Vec<_>>();
        assert_eq!(r0.len(), R0_ALLOWLIST.len());
        assert_eq!(
            registry.operations.len() - r0.len(),
            465 + ENGINE_COMMAND_COUNT
                + REMOTE_OPERATION_COUNT
                + MIDI_OSC_DMX_OPERATION_COUNT
                + FRONTEND_INVOKE_COUNT
        );
        assert_eq!(
            r0.iter()
                .map(|descriptor| descriptor.operation_id.as_str())
                .collect::<BTreeSet<_>>(),
            R0_ALLOWLIST.into_iter().collect::<BTreeSet<_>>()
        );
        for descriptor in &r0 {
            assert_eq!(
                descriptor.source_family,
                OperationSourceFamily::TauriCommand
            );
            assert_eq!(descriptor.risk, OperationRisk::R0);
            assert_eq!(
                descriptor.availability,
                OperationAvailability::LocalWindowOnly
            );
            assert_eq!(descriptor.idempotency, OperationIdempotency::ReadOnly);
            assert_eq!(descriptor.audit, OperationAuditRequirement::NotApplicable);
            assert!(descriptor
                .capabilities
                .contains(&OperationCapability::AllowedDuringFullLock));
        }
        let tauri_unavailable = registry.operations.iter().filter(|descriptor| {
            descriptor.source_family == OperationSourceFamily::TauriCommand
                && !R0_ALLOWLIST.contains(&descriptor.operation_id.as_str())
        });
        assert_eq!(tauri_unavailable.clone().count(), 465);
        for descriptor in tauri_unavailable {
            assert_eq!(
                descriptor.risk,
                OperationRisk::R5,
                "{}",
                descriptor.operation_id
            );
            assert_eq!(
                descriptor.availability,
                OperationAvailability::Unavailable,
                "{}",
                descriptor.operation_id
            );
            assert_eq!(
                descriptor.idempotency,
                OperationIdempotency::Mutating,
                "{}",
                descriptor.operation_id
            );
            assert_eq!(
                descriptor.audit,
                OperationAuditRequirement::RequiredBeforeExternalExecution,
                "{}",
                descriptor.operation_id
            );
            assert_eq!(
                descriptor.capabilities,
                vec![OperationCapability::LocalWindowBound],
                "{}",
                descriptor.operation_id
            );
        }
        for (source_id, semantic_operation_id) in [
            (
                "release_blackout_output_control_v2",
                OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
            ),
            ("arm_output_control_v2", OUTPUT_OWNERSHIP_ARM_OPERATION_ID),
            ("enable_output_control_v2", OUTPUT_ENABLE_OPERATION_ID),
            (
                "take_over_output_control_v2",
                OUTPUT_STANDBY_TAKEOVER_OPERATION_ID,
            ),
            ("add_display_output_v2", OUTPUT_DISPLAY_ADD_OPERATION_ID),
        ] {
            let descriptor = registry
                .operations
                .iter()
                .find(|descriptor| {
                    descriptor.source_family == OperationSourceFamily::TauriCommand
                        && descriptor.source_id == source_id
                })
                .unwrap_or_else(|| panic!("missing OutputControl command descriptor {source_id}"));
            assert_eq!(descriptor.operation_id, semantic_operation_id);
            assert_eq!(descriptor.risk, OperationRisk::R5);
            assert_eq!(descriptor.availability, OperationAvailability::Unavailable);
            assert_eq!(descriptor.idempotency, OperationIdempotency::Mutating);
        }
        {
            let (source_id, semantic_operation_id) = (
                "query_output_control_authority_v1",
                OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID,
            );
            let descriptor = registry
                .operations
                .iter()
                .find(|descriptor| {
                    descriptor.source_family == OperationSourceFamily::TauriCommand
                        && descriptor.source_id == source_id
                })
                .unwrap_or_else(|| panic!("missing OutputControl query descriptor {source_id}"));
            assert_eq!(descriptor.operation_id, semantic_operation_id);
            assert_eq!(descriptor.risk, OperationRisk::R0);
            assert_eq!(
                descriptor.availability,
                OperationAvailability::LocalWindowOnly
            );
            assert_eq!(descriptor.idempotency, OperationIdempotency::ReadOnly);
        }
        let engine_unavailable = registry
            .operations
            .iter()
            .filter(|descriptor| descriptor.source_family == OperationSourceFamily::EngineCommand)
            .collect::<Vec<_>>();
        assert_eq!(engine_unavailable.len(), ENGINE_COMMAND_COUNT);
        for descriptor in engine_unavailable {
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
        }
        let remote_unavailable = registry
            .operations
            .iter()
            .filter(|descriptor| {
                matches!(
                    descriptor.source_family,
                    OperationSourceFamily::RemoteInputEvent
                        | OperationSourceFamily::RemoteClientRequest
                        | OperationSourceFamily::RemoteWireOperation
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(remote_unavailable.len(), REMOTE_OPERATION_COUNT);
        assert_eq!(
            remote_unavailable
                .iter()
                .filter(|descriptor| {
                    descriptor.source_family == OperationSourceFamily::RemoteInputEvent
                })
                .count(),
            REMOTE_INPUT_EVENT_COUNT
        );
        assert_eq!(
            remote_unavailable
                .iter()
                .filter(|descriptor| {
                    descriptor.source_family == OperationSourceFamily::RemoteClientRequest
                })
                .count(),
            REMOTE_CLIENT_REQUEST_COUNT
        );
        assert_eq!(
            remote_unavailable
                .iter()
                .filter(|descriptor| {
                    descriptor.source_family == OperationSourceFamily::RemoteWireOperation
                })
                .count(),
            REMOTE_WIRE_OPERATION_COUNT
        );
        for descriptor in remote_unavailable {
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
        }
        let midi_osc_dmx_families = [
            (
                OperationSourceFamily::MidiControlMessage,
                MIDI_CONTROL_MESSAGE_COUNT,
            ),
            (
                OperationSourceFamily::MidiControlAction,
                MIDI_CONTROL_ACTION_COUNT,
            ),
            (
                OperationSourceFamily::MidiClockEvent,
                MIDI_CLOCK_EVENT_COUNT,
            ),
            (
                OperationSourceFamily::MidiControlEvent,
                MIDI_CONTROL_EVENT_COUNT,
            ),
            (
                OperationSourceFamily::OscControlAction,
                OSC_CONTROL_ACTION_COUNT,
            ),
            (OperationSourceFamily::OscInputEvent, OSC_INPUT_EVENT_COUNT),
            (
                OperationSourceFamily::DmxInputProtocol,
                DMX_INPUT_PROTOCOL_COUNT,
            ),
            (OperationSourceFamily::DmxInputEvent, DMX_INPUT_EVENT_COUNT),
        ];
        let midi_osc_dmx_unavailable = registry
            .operations
            .iter()
            .filter(|descriptor| {
                midi_osc_dmx_families
                    .iter()
                    .any(|(family, _)| descriptor.source_family == *family)
            })
            .collect::<Vec<_>>();
        assert_eq!(midi_osc_dmx_unavailable.len(), MIDI_OSC_DMX_OPERATION_COUNT);
        for (family, expected_count) in midi_osc_dmx_families {
            assert_eq!(
                midi_osc_dmx_unavailable
                    .iter()
                    .filter(|descriptor| descriptor.source_family == family)
                    .count(),
                expected_count,
                "{family:?}"
            );
        }
        for descriptor in midi_osc_dmx_unavailable {
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
        }
        let frontend_manifest =
            frontend_invoke_names_from_manifest(FRONTEND_INVOKE_MANIFEST).unwrap();
        assert_eq!(frontend_manifest.len(), FRONTEND_INVOKE_COUNT);
        let frontend_unavailable = registry
            .operations
            .iter()
            .filter(|descriptor| descriptor.source_family == OperationSourceFamily::FrontendInvoke)
            .collect::<Vec<_>>();
        assert_eq!(frontend_unavailable.len(), FRONTEND_INVOKE_COUNT);
        assert_eq!(
            frontend_unavailable
                .iter()
                .map(|descriptor| descriptor.source_id.as_str())
                .collect::<BTreeSet<_>>(),
            frontend_manifest
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        );
        for descriptor in frontend_unavailable {
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
            assert_eq!(
                descriptor.operation_id,
                format!(
                    "syndocal.inventory.frontend.invoke.{}.v1",
                    descriptor.source_id
                )
            );
        }
    }

    #[test]
    fn canonical_registry_derives_the_exact_current_source_inventory() {
        let legacy = registry().unwrap();
        let canonical = canonical_registry().unwrap();
        canonical.validate().unwrap();
        verify_canonical_registry_exact_sources(&legacy, &canonical).unwrap();

        const TAURI_COUNT: usize = 478;
        const ENGINE_COUNT: usize = 263;
        const REMOTE_COUNT: usize = 116;
        const MIDI_OSC_DMX_COUNT: usize = 206;
        const FRONTEND_COUNT: usize = 417;
        const LEGACY_SOURCE_TOTAL: usize =
            TAURI_COUNT + ENGINE_COUNT + REMOTE_COUNT + MIDI_OSC_DMX_COUNT + FRONTEND_COUNT;
        const KEYBOARD_APP_COUNT: usize = 30;
        const KEYBOARD_PROJECT_FILE_COUNT: usize = 3;
        const SOURCE_TOTAL: usize =
            LEGACY_SOURCE_TOTAL + KEYBOARD_APP_COUNT + KEYBOARD_PROJECT_FILE_COUNT;
        assert_eq!(LEGACY_SOURCE_TOTAL, 1480);
        assert_eq!(SOURCE_TOTAL, 1513);
        assert_eq!(canonical.source_inventory.len(), SOURCE_TOTAL);
        assert_eq!(canonical.canonical_operations.len(), 31);

        let output_control_operations = canonical
            .canonical_operations
            .iter()
            .filter(|operation| {
                matches!(
                    operation.operation_id.as_str(),
                    OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
                        | OUTPUT_OWNERSHIP_ARM_OPERATION_ID
                        | OUTPUT_STANDBY_TAKEOVER_OPERATION_ID
                        | OUTPUT_DISPLAY_ADD_OPERATION_ID
                        | OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID
                        | OUTPUT_ENABLE_OPERATION_ID
                        | OUTPUT_LEASE_ACQUIRE_OPERATION_ID
                        | OUTPUT_LEASE_RENEW_OPERATION_ID
                        | OUTPUT_LEASE_RECOVER_OPERATION_ID
                        | OUTPUT_LEASE_RELINQUISH_OPERATION_ID
                        | OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(output_control_operations.len(), 11);
        for (source_id, operation_id) in [
            (
                "release_blackout_output_control_v2",
                OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
            ),
            ("arm_output_control_v2", OUTPUT_OWNERSHIP_ARM_OPERATION_ID),
            ("enable_output_control_v2", OUTPUT_ENABLE_OPERATION_ID),
            (
                "take_over_output_control_v2",
                OUTPUT_STANDBY_TAKEOVER_OPERATION_ID,
            ),
            ("add_display_output_v2", OUTPUT_DISPLAY_ADD_OPERATION_ID),
            (
                "set_display_output_window_open_v2",
                OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID,
            ),
            ("acquire_output_lease_v2", OUTPUT_LEASE_ACQUIRE_OPERATION_ID),
            ("renew_output_lease_v2", OUTPUT_LEASE_RENEW_OPERATION_ID),
            ("recover_output_lease_v2", OUTPUT_LEASE_RECOVER_OPERATION_ID),
            (
                "relinquish_output_lease_v2",
                OUTPUT_LEASE_RELINQUISH_OPERATION_ID,
            ),
            (
                "force_transfer_output_lease_v2",
                OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID,
            ),
        ] {
            let operation = output_control_operations
                .iter()
                .find(|operation| operation.operation_id == operation_id)
                .unwrap_or_else(|| {
                    panic!("missing canonical OutputControl operation {operation_id}")
                });
            assert_eq!(operation.risk, OperationRisk::R4);
            assert_eq!(operation.class, OperationClass::Mutation);
            assert_eq!(operation.audit, OperationAuditRequirement::Immutable);
            assert_eq!(
                operation.adapter_policy,
                if matches!(
                    operation_id,
                    OUTPUT_ENABLE_OPERATION_ID
                        | OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID
                        | OUTPUT_LEASE_ACQUIRE_OPERATION_ID
                        | OUTPUT_LEASE_RENEW_OPERATION_ID
                        | OUTPUT_LEASE_RECOVER_OPERATION_ID
                        | OUTPUT_LEASE_RELINQUISH_OPERATION_ID
                ) {
                    AdapterPolicy::LocalWindowOutputControl
                } else {
                    AdapterPolicy::LocalWindowDangerousOutputControl
                }
            );
            assert_eq!(
                operation.receipt_policy,
                ReceiptPolicy::ExactTerminalReceipt
            );
            assert_eq!(
                operation.request_schema.version,
                OUTPUT_CONTROL_COMMAND_SCHEMA_VERSION
            );
            assert_eq!(
                operation.response_schema.version,
                OUTPUT_CONTROL_COMMAND_SCHEMA_VERSION
            );
            assert_eq!(
                operation.consent_policy,
                if matches!(
                    operation_id,
                    OUTPUT_ENABLE_OPERATION_ID
                        | OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID
                        | OUTPUT_LEASE_ACQUIRE_OPERATION_ID
                        | OUTPUT_LEASE_RENEW_OPERATION_ID
                        | OUTPUT_LEASE_RECOVER_OPERATION_ID
                        | OUTPUT_LEASE_RELINQUISH_OPERATION_ID
                ) {
                    ConsentPolicy::LocalExplicitAction
                } else {
                    ConsentPolicy::NativeDangerConfirmation
                }
            );
            assert_eq!(
                operation.capabilities,
                vec![
                    OperationCapability::LocalWindowBound,
                    OperationCapability::OutputControl,
                ]
            );
            let direct = canonical
                .source_inventory
                .iter()
                .find(|source| {
                    source.source_key.family == CanonicalSourceFamily::TauriCommand
                        && source.source_key.source_id == source_id
                })
                .unwrap_or_else(|| panic!("missing direct OutputControl source {source_id}"));
            assert!(matches!(
                &direct.disposition,
                SourceDisposition::Operation {
                    canonical_operation_id,
                    projection: TypedSchemaProjection::Exact,
                } if canonical_operation_id == operation_id
            ));
            let alias = canonical
                .source_inventory
                .iter()
                .find(|source| {
                    source.source_key.family == CanonicalSourceFamily::FrontendInvoke
                        && source.source_key.source_id == source_id
                })
                .unwrap_or_else(|| panic!("missing frontend OutputControl alias {source_id}"));
            assert!(matches!(
                &alias.disposition,
                SourceDisposition::AliasOfSource { target }
                    if *target == direct.source_key
            ));
            assert_eq!(
                canonical
                    .canonical_operation_for_source(&alias.source_key)
                    .unwrap()
                    .map(|operation| operation.operation_id.as_str()),
                Some(operation_id)
            );
        }
        {
            let (source_id, operation_id) = (
                "query_output_control_authority_v1",
                OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID,
            );
            let source = canonical
                .source_inventory
                .iter()
                .find(|source| {
                    source.source_key.family == CanonicalSourceFamily::TauriCommand
                        && source.source_key.source_id == source_id
                })
                .unwrap_or_else(|| panic!("missing OutputControl query source {source_id}"));
            assert_eq!(
                canonical
                    .canonical_operation_for_source(&source.source_key)
                    .unwrap()
                    .map(|operation| operation.operation_id.as_str()),
                Some(operation_id)
            );
        }

        let canonical_query_source = canonical
            .source_inventory
            .iter()
            .find(|source| {
                source.source_key.family == CanonicalSourceFamily::TauriCommand
                    && source.source_key.source_id == "get_control_plane_canonical_registry"
            })
            .expect("canonical registry query source must remain inventoried");
        let canonical_query_operation_id = "syndocal.query.control_plane.canonical_registry.v3";
        assert_eq!(
            canonical_query_source.raw_request_schema,
            command_schema(canonical_query_operation_id, "request")
        );
        assert_eq!(
            canonical_query_source.raw_response_schema,
            command_schema(canonical_query_operation_id, "response")
        );
        let canonical_query_operation = canonical
            .canonical_operation_for_source(&canonical_query_source.source_key)
            .unwrap()
            .expect("canonical registry query must resolve to a canonical operation");
        assert_eq!(
            canonical_query_operation.request_schema,
            canonical_query_source.raw_request_schema
        );
        assert_eq!(
            canonical_query_operation.response_schema,
            canonical_query_source.raw_response_schema
        );

        let count_family = |family| {
            canonical
                .source_inventory
                .iter()
                .filter(|source| source.source_key.family == family)
                .count()
        };
        assert_eq!(
            count_family(CanonicalSourceFamily::TauriCommand),
            TAURI_COUNT
        );
        assert_eq!(
            count_family(CanonicalSourceFamily::EngineCommand),
            ENGINE_COUNT
        );
        assert_eq!(
            count_family(CanonicalSourceFamily::RemoteInputEvent)
                + count_family(CanonicalSourceFamily::RemoteClientRequest)
                + count_family(CanonicalSourceFamily::RemoteWireOperation),
            REMOTE_COUNT
        );
        assert_eq!(
            count_family(CanonicalSourceFamily::MidiControlMessage)
                + count_family(CanonicalSourceFamily::MidiControlAction)
                + count_family(CanonicalSourceFamily::MidiClockEvent)
                + count_family(CanonicalSourceFamily::MidiControlEvent)
                + count_family(CanonicalSourceFamily::OscControlAction)
                + count_family(CanonicalSourceFamily::OscInputEvent)
                + count_family(CanonicalSourceFamily::DmxInputProtocol)
                + count_family(CanonicalSourceFamily::DmxInputEvent),
            MIDI_OSC_DMX_COUNT
        );
        assert_eq!(
            count_family(CanonicalSourceFamily::FrontendInvoke),
            FRONTEND_COUNT
        );
        assert_eq!(
            count_family(CanonicalSourceFamily::KeyboardApp),
            KEYBOARD_APP_COUNT
        );
        assert_eq!(
            count_family(CanonicalSourceFamily::KeyboardProjectFile),
            KEYBOARD_PROJECT_FILE_COUNT
        );

        let expected_keyboard_sources =
            keyboard_shortcut_source_ids_from_manifest(KEYBOARD_SHORTCUT_SOURCE_MANIFEST)
                .unwrap()
                .into_iter()
                .map(|(family, source_id)| SourceKey::new(family, source_id))
                .collect::<BTreeSet<_>>();
        let actual_keyboard_sources = canonical
            .source_inventory
            .iter()
            .filter(|source| {
                matches!(
                    source.source_key.family,
                    CanonicalSourceFamily::KeyboardApp | CanonicalSourceFamily::KeyboardProjectFile
                )
            })
            .map(|source| source.source_key.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual_keyboard_sources, expected_keyboard_sources);

        let direct = canonical
            .source_inventory
            .iter()
            .filter(|source| matches!(&source.disposition, SourceDisposition::Operation { .. }))
            .collect::<Vec<_>>();
        let aliases = canonical
            .source_inventory
            .iter()
            .filter(|source| matches!(&source.disposition, SourceDisposition::AliasOfSource { .. }))
            .collect::<Vec<_>>();
        let unclassified = canonical
            .source_inventory
            .iter()
            .filter(|source| matches!(&source.disposition, SourceDisposition::Unclassified { .. }))
            .collect::<Vec<_>>();
        let support_phases = canonical
            .source_inventory
            .iter()
            .filter(|source| matches!(&source.disposition, SourceDisposition::SupportPhase { .. }))
            .collect::<Vec<_>>();
        let internal_steps = canonical
            .source_inventory
            .iter()
            .filter(|source| {
                matches!(
                    &source.disposition,
                    SourceDisposition::InternalStepOf { .. }
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
        assert_eq!(direct.len(), 31);
        assert_eq!(aliases.len(), FRONTEND_COUNT);
        assert_eq!(internal_steps.len(), 4);
        assert_eq!(structural_routes.len(), 1);
        assert_eq!(unclassified.len(), 1060);
        assert_eq!(support_phases.len(), 0);
        assert_eq!(
            direct.len()
                + aliases.len()
                + internal_steps.len()
                + structural_routes.len()
                + unclassified.len()
                + support_phases.len(),
            SOURCE_TOTAL
        );

        for (family, expected_count, role, reason) in [(
            CanonicalSourceFamily::KeyboardProjectFile,
            KEYBOARD_PROJECT_FILE_COUNT,
            SourceRole::KeyboardProjectFileShortcut,
            "unreviewed keyboard project-file shortcut",
        )] {
            let sources = canonical
                .source_inventory
                .iter()
                .filter(|source| source.source_key.family == family)
                .collect::<Vec<_>>();
            assert_eq!(sources.len(), expected_count);
            for source in sources {
                assert_eq!(source.role, role);
                assert!(source.source_key.source_id.ends_with("_v1"));
                assert!(matches!(
                    &source.disposition,
                    SourceDisposition::Unclassified { reason: actual } if actual == reason
                ));
                assert!(canonical
                    .canonical_operation_for_source(&source.source_key)
                    .unwrap()
                    .is_none());
            }
        }
        let timeline_shortcut = structural_routes[0];
        assert_eq!(
            timeline_shortcut.source_key.family,
            CanonicalSourceFamily::KeyboardApp
        );
        assert_eq!(
            timeline_shortcut.source_key.source_id,
            "toggle_timeline_playback_v1"
        );
        assert!(matches!(
            &timeline_shortcut.disposition,
            SourceDisposition::StructuralRoute { target }
                if *target == SourceKey::new(
                    CanonicalSourceFamily::TauriCommand,
                    "set_timeline_transport_playing_runtime_v1"
                )
        ));
        assert!(canonical
            .canonical_operation_for_source(&timeline_shortcut.source_key)
            .unwrap()
            .is_some());

        for source in &direct {
            assert_eq!(
                source.source_key.family,
                CanonicalSourceFamily::TauriCommand
            );
            assert_eq!(source.role, SourceRole::LocalWindowCommand);
            assert!(canonical
                .canonical_operation_for_source(&source.source_key)
                .unwrap()
                .is_some());
        }
        for source in &aliases {
            assert_eq!(
                source.source_key.family,
                CanonicalSourceFamily::FrontendInvoke
            );
            assert_eq!(source.role, SourceRole::FrontendInvocation);
            let SourceDisposition::AliasOfSource { target } = &source.disposition else {
                unreachable!("filtered aliases must retain their disposition");
            };
            assert_eq!(target.family, CanonicalSourceFamily::TauriCommand);
            assert_eq!(target.source_id, source.source_key.source_id);
            assert_eq!(
                canonical
                    .canonical_operation_for_source(&source.source_key)
                    .unwrap()
                    .map(|operation| operation.operation_id.as_str()),
                canonical
                    .canonical_operation_for_source(target)
                    .unwrap()
                    .map(|operation| operation.operation_id.as_str())
            );
        }
        let effect_enabled_direct = direct
            .iter()
            .find(|source| source.source_key.source_id == "set_effect_enabled")
            .expect("set_effect_enabled must be the ninth direct canonical source");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&effect_enabled_direct.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(SET_EFFECT_ENABLED_OPERATION_ID)
        );
        let effect_enabled_alias = aliases
            .iter()
            .find(|source| source.source_key.source_id == "set_effect_enabled")
            .expect("frontend set_effect_enabled must retain its alias row");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&effect_enabled_alias.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(SET_EFFECT_ENABLED_OPERATION_ID)
        );
        let timeline_authority_direct = direct
            .iter()
            .find(|source| source.source_key.source_id == "query_timeline_transport_authority_v1")
            .expect("runtime Timeline authority must be a direct canonical query source");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&timeline_authority_direct.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(TIMELINE_TRANSPORT_AUTHORITY_QUERY_OPERATION_ID)
        );
        assert_eq!(
            timeline_authority_direct.raw_request_schema.name.as_str(),
            "syndocal.query.runtime.timeline.transport.authority.v1.request"
        );
        assert_eq!(
            timeline_authority_direct.raw_response_schema.name.as_str(),
            "syndocal.query.runtime.timeline.transport.authority.v1.response"
        );
        let timeline_authority_alias = aliases
            .iter()
            .find(|source| source.source_key.source_id == "query_timeline_transport_authority_v1")
            .expect("frontend runtime Timeline authority must retain its alias row");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&timeline_authority_alias.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(TIMELINE_TRANSPORT_AUTHORITY_QUERY_OPERATION_ID)
        );
        let timeline_transport_direct = direct
            .iter()
            .find(|source| {
                source.source_key.source_id == "set_timeline_transport_playing_runtime_v1"
            })
            .expect("runtime Timeline transport must be a direct canonical source");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&timeline_transport_direct.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID)
        );
        let timeline_transport_alias = aliases
            .iter()
            .find(|source| {
                source.source_key.source_id == "set_timeline_transport_playing_runtime_v1"
            })
            .expect("frontend runtime Timeline transport must retain its alias row");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&timeline_transport_alias.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID)
        );
        let follow_abort_authority_direct = direct
            .iter()
            .find(|source| {
                source.source_key.source_id == "query_timeline_follow_abort_authority_v1"
            })
            .expect("Follow Abort authority must be a direct canonical query source");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&follow_abort_authority_direct.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(TIMELINE_FOLLOW_ABORT_AUTHORITY_QUERY_OPERATION_ID)
        );
        let follow_abort_direct = direct
            .iter()
            .find(|source| source.source_key.source_id == "abort_timeline_follow_runtime_v1")
            .expect("Follow Abort must be a direct canonical runtime-safety source");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&follow_abort_direct.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(TIMELINE_FOLLOW_ABORT_OPERATION_ID)
        );
        let follow_abort_alias = aliases
            .iter()
            .find(|source| source.source_key.source_id == "abort_timeline_follow_runtime_v1")
            .expect("frontend Follow Abort must retain its canonical alias row");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&follow_abort_alias.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(TIMELINE_FOLLOW_ABORT_OPERATION_ID)
        );
        let safety_blackout_direct = direct
            .iter()
            .find(|source| source.source_key.source_id == "safety_blackout_engage_v1")
            .expect("S0 safety blackout must be a direct canonical source");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&safety_blackout_direct.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(SAFETY_BLACKOUT_ENGAGE_OPERATION_ID)
        );
        let safety_blackout_alias = aliases
            .iter()
            .find(|source| source.source_key.source_id == "safety_blackout_engage_v1")
            .expect("frontend S0 safety blackout must retain its canonical alias row");
        assert_eq!(
            canonical
                .canonical_operation_for_source(&safety_blackout_alias.source_key)
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(SAFETY_BLACKOUT_ENGAGE_OPERATION_ID)
        );
        for (source_id, target_id) in [
            ("set_effect_enabled_published", "set_effect_enabled"),
            (
                "set_timeline_playing_published",
                "set_timeline_transport_playing_runtime_v1",
            ),
            ("abort_timeline_follow", "abort_timeline_follow_runtime_v1"),
            (
                "safety_blackout_engage_published",
                "safety_blackout_engage_v1",
            ),
        ] {
            let published_step = internal_steps
                .iter()
                .find(|source| source.source_key.source_id == source_id)
                .unwrap_or_else(|| panic!("missing internal step {source_id}"));
            assert_eq!(
                published_step.source_key.family,
                CanonicalSourceFamily::EngineCommand
            );
            assert!(matches!(
                &published_step.disposition,
                SourceDisposition::InternalStepOf { target }
                    if *target == SourceKey::new(CanonicalSourceFamily::TauriCommand, target_id)
            ));
        }
        for source in &unclassified {
            assert!(canonical
                .canonical_operation_for_source(&source.source_key)
                .unwrap()
                .is_none());
        }
        for operation in &canonical.canonical_operations {
            if operation.operation_id == SAFETY_BLACKOUT_ENGAGE_OPERATION_ID {
                assert_eq!(operation.risk, OperationRisk::S0);
                assert_eq!(operation.audit, OperationAuditRequirement::Immutable);
            } else if matches!(
                operation.operation_id.as_str(),
                OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
                    | OUTPUT_OWNERSHIP_ARM_OPERATION_ID
                    | OUTPUT_STANDBY_TAKEOVER_OPERATION_ID
                    | OUTPUT_DISPLAY_ADD_OPERATION_ID
                    | OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID
                    | OUTPUT_ENABLE_OPERATION_ID
                    | OUTPUT_LEASE_ACQUIRE_OPERATION_ID
                    | OUTPUT_LEASE_RENEW_OPERATION_ID
                    | OUTPUT_LEASE_RECOVER_OPERATION_ID
                    | OUTPUT_LEASE_RELINQUISH_OPERATION_ID
                    | OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID
            ) {
                assert_eq!(operation.risk, OperationRisk::R4);
                assert_eq!(operation.audit, OperationAuditRequirement::Immutable);
            } else {
                assert_eq!(operation.risk, OperationRisk::R0);
                assert_eq!(operation.audit, OperationAuditRequirement::NotApplicable);
            }
            assert_eq!(operation.derived_adapters.len(), 1);
            if operation.operation_id == SET_EFFECT_ENABLED_OPERATION_ID {
                assert_eq!(operation.class, OperationClass::Mutation);
                assert_eq!(operation.idempotency, OperationIdempotency::Mutating);
                assert_eq!(
                    operation.adapter_policy,
                    AdapterPolicy::LocalWindowAuthoritativeMutation
                );
                assert_eq!(
                    operation.receipt_policy,
                    ReceiptPolicy::ExactTerminalReceipt
                );
                assert_eq!(
                    operation.capabilities,
                    vec![
                        OperationCapability::LocalWindowBound,
                        OperationCapability::AuthoritativeProjectMutation,
                    ]
                );
            } else if matches!(
                operation.operation_id.as_str(),
                CUE_LIST_DELETE_OPERATION_ID
                    | CUE_LIST_REORDER_OPERATION_ID
                    | EMPTY_CUE_CREATE_OPERATION_ID
            ) {
                assert_eq!(operation.class, OperationClass::Mutation);
                assert_eq!(operation.idempotency, OperationIdempotency::Mutating);
                assert_eq!(
                    operation.adapter_policy,
                    AdapterPolicy::LocalWindowAuthoritativeMutation
                );
                assert_eq!(
                    operation.receipt_policy,
                    ReceiptPolicy::ExactTerminalReceipt
                );
                assert_eq!(operation.rate_policy, RatePolicy::FailClosed);
                assert_eq!(operation.consent_policy, ConsentPolicy::FailClosed);
                assert_eq!(
                    operation.capabilities,
                    vec![
                        OperationCapability::LocalWindowBound,
                        OperationCapability::AuthoritativeProjectMutation,
                    ]
                );
            } else if operation.operation_id == TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID {
                assert_eq!(operation.class, OperationClass::Mutation);
                assert_eq!(operation.idempotency, OperationIdempotency::Mutating);
                assert_eq!(
                    operation.adapter_policy,
                    AdapterPolicy::LocalWindowAuthoritativeRuntimeMutation
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
                    operation.capabilities,
                    vec![
                        OperationCapability::LocalWindowBound,
                        OperationCapability::AuthoritativeRuntimeMutation,
                    ]
                );
                assert!(!operation
                    .capabilities
                    .contains(&OperationCapability::AllowedDuringFullLock));
            } else if operation.operation_id == TIMELINE_FOLLOW_ABORT_OPERATION_ID {
                assert_eq!(operation.class, OperationClass::Mutation);
                assert_eq!(operation.idempotency, OperationIdempotency::Mutating);
                assert_eq!(
                    operation.adapter_policy,
                    AdapterPolicy::LocalWindowRuntimeSafetyMutation
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
                    operation.capabilities,
                    vec![
                        OperationCapability::LocalWindowBound,
                        OperationCapability::AuthoritativeRuntimeMutation,
                        OperationCapability::AllowedDuringFullLock,
                    ]
                );
            } else if operation.operation_id == SAFETY_BLACKOUT_ENGAGE_OPERATION_ID {
                assert_eq!(operation.class, OperationClass::Mutation);
                assert_eq!(operation.idempotency, OperationIdempotency::Mutating);
                assert_eq!(
                    operation.adapter_policy,
                    AdapterPolicy::LocalWindowEmergencySafetyMutation
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
                    ConsentPolicy::NotRequiredForSafetyOnly
                );
                assert_eq!(
                    operation.capabilities,
                    vec![
                        OperationCapability::LocalWindowBound,
                        OperationCapability::SafetyBlackoutEngage,
                        OperationCapability::AllowedDuringFullLock,
                    ]
                );
            } else if matches!(
                operation.operation_id.as_str(),
                OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
                    | OUTPUT_OWNERSHIP_ARM_OPERATION_ID
                    | OUTPUT_STANDBY_TAKEOVER_OPERATION_ID
                    | OUTPUT_DISPLAY_ADD_OPERATION_ID
                    | OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID
                    | OUTPUT_ENABLE_OPERATION_ID
                    | OUTPUT_LEASE_ACQUIRE_OPERATION_ID
                    | OUTPUT_LEASE_RENEW_OPERATION_ID
                    | OUTPUT_LEASE_RECOVER_OPERATION_ID
                    | OUTPUT_LEASE_RELINQUISH_OPERATION_ID
                    | OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID
            ) {
                assert_eq!(operation.class, OperationClass::Mutation);
                assert_eq!(operation.idempotency, OperationIdempotency::Mutating);
                assert_eq!(
                    operation.adapter_policy,
                    if matches!(
                        operation.operation_id.as_str(),
                        OUTPUT_ENABLE_OPERATION_ID
                            | OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID
                            | OUTPUT_LEASE_ACQUIRE_OPERATION_ID
                            | OUTPUT_LEASE_RENEW_OPERATION_ID
                            | OUTPUT_LEASE_RECOVER_OPERATION_ID
                            | OUTPUT_LEASE_RELINQUISH_OPERATION_ID
                    ) {
                        AdapterPolicy::LocalWindowOutputControl
                    } else {
                        AdapterPolicy::LocalWindowDangerousOutputControl
                    }
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
                    if matches!(
                        operation.operation_id.as_str(),
                        OUTPUT_ENABLE_OPERATION_ID
                            | OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID
                            | OUTPUT_LEASE_ACQUIRE_OPERATION_ID
                            | OUTPUT_LEASE_RENEW_OPERATION_ID
                            | OUTPUT_LEASE_RECOVER_OPERATION_ID
                            | OUTPUT_LEASE_RELINQUISH_OPERATION_ID
                    ) {
                        ConsentPolicy::LocalExplicitAction
                    } else {
                        ConsentPolicy::NativeDangerConfirmation
                    }
                );
                assert_eq!(
                    operation.capabilities,
                    vec![
                        OperationCapability::LocalWindowBound,
                        OperationCapability::OutputControl,
                    ]
                );
                assert!(!operation
                    .capabilities
                    .contains(&OperationCapability::AllowedDuringFullLock));
            } else {
                assert_eq!(
                    operation.idempotency,
                    OperationIdempotency::ReadOnly,
                    "{}",
                    operation.operation_id
                );
                assert_eq!(operation.adapter_policy, AdapterPolicy::LocalWindowReadOnly);
                assert_eq!(operation.receipt_policy, ReceiptPolicy::FailClosed);
            }
        }

        let fail_closed_effect_bindings = [
            (
                CanonicalSourceFamily::EngineCommand,
                "set_effect_enabled",
                "legacy effect-enabled engine ingress lacks a versioned authored fence",
            ),
            (
                CanonicalSourceFamily::RemoteInputEvent,
                "set_effect_enabled",
                "legacy remote effect-enabled ingress is fail-closed pending versioned migration",
            ),
            (
                CanonicalSourceFamily::RemoteWireOperation,
                "setEffectEnabled",
                "legacy remote effect-enabled ingress is fail-closed pending versioned migration",
            ),
            (
                CanonicalSourceFamily::MidiControlAction,
                "effect_enabled",
                "legacy midi effect-enabled ingress is fail-closed pending versioned migration",
            ),
            (
                CanonicalSourceFamily::MidiControlEvent,
                "set_effect_enabled",
                "legacy midi effect-enabled ingress is fail-closed pending versioned migration",
            ),
            (
                CanonicalSourceFamily::OscControlAction,
                "effect_enabled",
                "legacy osc effect-enabled ingress is fail-closed pending versioned migration",
            ),
            (
                CanonicalSourceFamily::OscInputEvent,
                "set_effect_enabled",
                "legacy osc effect-enabled ingress is fail-closed pending versioned migration",
            ),
        ];
        for (family, source_id, reason) in fail_closed_effect_bindings {
            let source = canonical
                .source_inventory
                .iter()
                .find(|source| {
                    source.source_key.family == family && source.source_key.source_id == source_id
                })
                .unwrap_or_else(|| {
                    panic!("missing legacy effect-enabled source {family:?}:{source_id}")
                });
            assert!(matches!(
                &source.disposition,
                SourceDisposition::Unclassified { reason: actual } if actual == reason
            ));
        }

        // These are the actual existing non-local timeline ingress bindings.
        // They remain deliberately unclassified until each adapter has its own
        // versioned identity and server-derived runtime authority contract.
        let fail_closed_timeline_transport_bindings = [
            (
                CanonicalSourceFamily::EngineCommand,
                "set_timeline_playing",
                "legacy timeline set-playing engine ingress lacks a versioned runtime identity",
            ),
            (
                CanonicalSourceFamily::RemoteInputEvent,
                "set_timeline_playing",
                "legacy remote timeline set-playing ingress is fail-closed pending versioned migration",
            ),
            (
                CanonicalSourceFamily::RemoteWireOperation,
                "setTimelinePlaying",
                "legacy remote timeline set-playing ingress is fail-closed pending versioned migration",
            ),
            (
                CanonicalSourceFamily::MidiControlAction,
                "timeline_play",
                "legacy midi timeline set-playing ingress is fail-closed pending versioned migration",
            ),
            (
                CanonicalSourceFamily::MidiControlEvent,
                "set_timeline_playing",
                "legacy midi timeline set-playing ingress is fail-closed pending versioned migration",
            ),
            (
                CanonicalSourceFamily::OscControlAction,
                "timeline_play",
                "legacy osc timeline set-playing ingress is fail-closed pending versioned migration",
            ),
            (
                CanonicalSourceFamily::OscInputEvent,
                "set_timeline_playing",
                "legacy osc timeline set-playing ingress is fail-closed pending versioned migration",
            ),
        ];
        for (family, source_id, reason) in fail_closed_timeline_transport_bindings {
            let source = canonical
                .source_inventory
                .iter()
                .find(|source| {
                    source.source_key.family == family && source.source_key.source_id == source_id
                })
                .unwrap_or_else(|| {
                    panic!("missing legacy timeline transport source {family:?}:{source_id}")
                });
            assert!(matches!(
                &source.disposition,
                SourceDisposition::Unclassified { reason: actual } if actual == reason
            ));
            assert!(canonical
                .canonical_operation_for_source(&source.source_key)
                .unwrap()
                .is_none());
        }

        let canonical_json = serde_json::to_value(&canonical).unwrap();
        for source in canonical_json["source_inventory"].as_array().unwrap() {
            let source = source.as_object().unwrap();
            assert!(!source.contains_key("risk"));
            assert!(!source.contains_key("availability"));
            assert!(!source.contains_key("adapter_policy"));
        }
    }

    #[test]
    fn canonical_registry_rejects_dropped_legacy_and_keyboard_sources() {
        let legacy = registry().unwrap();
        let mut canonical = canonical_registry().unwrap();
        let legacy_index = canonical
            .source_inventory
            .iter()
            .position(|source| source.source_key.family == CanonicalSourceFamily::TauriCommand)
            .unwrap();
        canonical.source_inventory.remove(legacy_index);
        assert!(matches!(
            verify_canonical_registry_exact_sources(&legacy, &canonical),
            Err(ControlPlaneRegistryError::MissingCanonicalSource(_))
        ));

        let mut canonical = canonical_registry().unwrap();
        let keyboard_index = canonical
            .source_inventory
            .iter()
            .position(|source| {
                source.source_key
                    == SourceKey::new(CanonicalSourceFamily::KeyboardApp, "new_project_v1")
            })
            .unwrap();
        canonical.source_inventory.remove(keyboard_index);
        assert!(matches!(
            verify_canonical_registry_exact_sources(&legacy, &canonical),
            Err(ControlPlaneRegistryError::MissingCanonicalSource(_))
        ));
    }

    #[test]
    fn legacy_v1_registry_json_and_count_remain_inventory_honest() {
        let legacy = registry().unwrap();
        assert_eq!(legacy.operations.len(), 1480);
        let encoded = serde_json::to_value(&legacy).unwrap();
        assert_eq!(encoded["schema"]["version"], CONTROL_PLANE_SCHEMA_VERSION);
        let operations = encoded["operations"].as_array().unwrap();
        assert_eq!(operations.len(), 1480);
        assert!(operations.iter().all(|operation| {
            operation["source_family"] != "keyboard_app"
                && operation["source_family"] != "keyboard_project_file"
        }));
    }

    #[test]
    fn keyboard_shortcut_manifest_rejects_malformed_duplicate_and_drifting_sources() {
        assert!(matches!(
            keyboard_shortcut_source_ids_from_manifest("{"),
            Err(ControlPlaneRegistryError::InvalidKeyboardShortcutSourceManifest)
        ));
        assert!(matches!(
            keyboard_shortcut_source_ids_from_manifest(
                "{\"schema_version\":1,\"keyboard_app\":[],\"keyboard_project_file\":[],\"unknown\":true}"
            ),
            Err(ControlPlaneRegistryError::InvalidKeyboardShortcutSourceManifest)
        ));

        let mut duplicate = serde_json::from_str::<KeyboardShortcutSourceManifest>(
            KEYBOARD_SHORTCUT_SOURCE_MANIFEST,
        )
        .unwrap();
        duplicate.keyboard_app[1] = duplicate.keyboard_app[0].clone();
        let duplicate = format!("{}\n", serde_json::to_string_pretty(&duplicate).unwrap());
        assert!(matches!(
            keyboard_shortcut_source_ids_from_manifest(&duplicate),
            Err(ControlPlaneRegistryError::DuplicateKeyboardShortcutSource(
                _
            ))
        ));

        let mut drifted = serde_json::from_str::<KeyboardShortcutSourceManifest>(
            KEYBOARD_SHORTCUT_SOURCE_MANIFEST,
        )
        .unwrap();
        drifted.keyboard_project_file.pop();
        let drifted = format!("{}\n", serde_json::to_string_pretty(&drifted).unwrap());
        assert!(matches!(
            keyboard_shortcut_source_ids_from_manifest(&drifted),
            Err(ControlPlaneRegistryError::InvalidKeyboardShortcutSourceManifest)
        ));
    }

    #[test]
    fn legacy_registry_only_adds_the_canonical_query_endpoint_row() {
        let legacy = registry().unwrap();
        let encoded = serde_json::to_value(&legacy).unwrap();
        let operations = encoded["operations"].as_array().unwrap();
        let endpoint_rows = operations
            .iter()
            .filter(|operation| {
                operation["source_family"] == "tauri_command"
                    && operation["source_id"] == "get_control_plane_canonical_registry"
            })
            .collect::<Vec<_>>();
        assert_eq!(endpoint_rows.len(), 1);
        let endpoint = endpoint_rows[0];
        assert_eq!(
            endpoint["operation_id"],
            "syndocal.query.control_plane.canonical_registry.v3"
        );
        assert_eq!(
            endpoint["request_schema"]["name"],
            "syndocal.query.control_plane.canonical_registry.v3.request"
        );
        assert_eq!(
            endpoint["response_schema"]["name"],
            "syndocal.query.control_plane.canonical_registry.v3.response"
        );
        assert_eq!(
            endpoint["request_schema"]["version"],
            CONTROL_PLANE_SCHEMA_VERSION
        );
        assert_eq!(
            endpoint["response_schema"]["version"],
            CONTROL_PLANE_SCHEMA_VERSION
        );
        assert_eq!(endpoint["risk"], "r0");
        assert_eq!(endpoint["availability"], "local_window_only");
        assert_eq!(endpoint["idempotency"], "read_only");
        assert_eq!(endpoint["audit"], "not_applicable");
        assert_eq!(
            endpoint["capabilities"],
            serde_json::json!([
                "read_only",
                "local_window_bound",
                "allowed_during_full_lock",
                "registry_discovery"
            ])
        );
    }

    #[test]
    fn legacy_set_effect_enabled_stays_r5_while_v2_owns_the_authoritative_mutation() {
        let legacy = registry().unwrap();
        let descriptor = legacy
            .operations
            .iter()
            .find(|descriptor| {
                descriptor.source_family == OperationSourceFamily::TauriCommand
                    && descriptor.source_id == "set_effect_enabled"
            })
            .expect("set_effect_enabled must remain in the exact v1 source inventory");
        assert_eq!(descriptor.operation_id, SET_EFFECT_ENABLED_OPERATION_ID);
        assert_eq!(descriptor.risk, OperationRisk::R5);
        assert_eq!(descriptor.availability, OperationAvailability::Unavailable);
        assert_eq!(descriptor.class, OperationClass::Mutation);
        assert_eq!(descriptor.idempotency, OperationIdempotency::Mutating);
        assert_eq!(
            descriptor.capabilities,
            vec![OperationCapability::LocalWindowBound]
        );
        assert_eq!(
            descriptor.audit,
            OperationAuditRequirement::RequiredBeforeExternalExecution
        );
        assert_eq!(
            descriptor.request_schema,
            command_schema(SET_EFFECT_ENABLED_OPERATION_ID, "request")
        );
        assert_eq!(
            descriptor.response_schema,
            command_schema(SET_EFFECT_ENABLED_OPERATION_ID, "response")
        );
    }

    #[test]
    fn duplicate_handler_command_is_rejected() {
        let source = "tauri::generate_handler![\n  get_snapshot,\n  get_snapshot,\n]";
        assert_eq!(
            registered_tauri_command_names_from_source(source),
            Err(ControlPlaneRegistryError::DuplicateCommandName(
                "get_snapshot".to_string()
            ))
        );
    }

    #[test]
    fn frontend_manifest_rejects_duplicate_unsorted_and_invalid_commands() {
        assert_eq!(
            frontend_invoke_names_from_manifest("[\"get_snapshot\",\"get_snapshot\"]"),
            Err(ControlPlaneRegistryError::DuplicateFrontendInvokeCommand(
                "get_snapshot".to_string()
            ))
        );
        assert_eq!(
            frontend_invoke_names_from_manifest("[\"set_bpm\",\"get_snapshot\"]"),
            Err(ControlPlaneRegistryError::InvalidFrontendInvokeManifest)
        );
        assert_eq!(
            frontend_invoke_names_from_manifest("[\"GetSnapshot\"]"),
            Err(ControlPlaneRegistryError::InvalidFrontendInvokeManifest)
        );
    }

    #[test]
    fn multiple_production_handler_blocks_are_rejected() {
        let source = "tauri::generate_handler![\n  get_snapshot,\n]\ntauri::generate_handler![\n  get_snapshot_delta,\n]";
        assert_eq!(
            registered_tauri_command_names_from_source(source),
            Err(ControlPlaneRegistryError::MultipleHandlerMarkers)
        );
    }

    #[test]
    fn missing_or_new_registry_descriptor_is_rejected() {
        let names = vec!["get_snapshot".to_string()];
        let mut registry = OperationRegistry {
            schema: SchemaIdentity::registry(),
            operations: Vec::new(),
        };
        assert_eq!(
            verify_registry_exact_set(&names, &registry),
            Err(ControlPlaneRegistryError::MissingRegistryDescriptor(
                "get_snapshot".to_string()
            ))
        );
        registry
            .operations
            .push(descriptor_for_command("get_snapshot"));
        registry
            .operations
            .push(unavailable_descriptor("new_command"));
        assert_eq!(
            verify_registry_exact_set(&names, &registry),
            Err(ControlPlaneRegistryError::UnexpectedRegistryDescriptor(
                "new_command".to_string()
            ))
        );
    }

    #[test]
    fn conservative_default_never_makes_a_mutation_externally_available() {
        let descriptor = unavailable_descriptor("set_blackout");
        assert_eq!(descriptor.risk, OperationRisk::R5);
        assert_eq!(descriptor.availability, OperationAvailability::Unavailable);
        assert_eq!(descriptor.idempotency, OperationIdempotency::Mutating);
        assert!(descriptor.validate().is_ok());
    }

    #[test]
    fn registry_requires_a_window_binding() {
        assert_eq!(
            registry_for_window(""),
            Err(ControlPlaneRegistryError::MissingWindowBinding)
        );
    }
}
