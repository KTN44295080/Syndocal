use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::Read,
    path::Path,
};

use base64::Engine as _;
use flate2::{read::ZlibDecoder, Decompress, FlushDecompress, Status};
use protocol::{
    AttributeControl, AttributeResolution, AttributeValueSummary, ChannelFunctionSummary,
    ChaserDirection, ChaserEffectRequest, ChaserFeature, ChaserStep, ChildTimelineSummary,
    ColorEffectAlgorithm, ColorEffectBeamTarget, ColorEffectColor, ColorEffectInterpolation,
    ColorEffectRequest, ColorEffectSpatialBounceItem, ColorEffectSpatialCoordinateFrame,
    ColorEffectSpatialMappingShape, ColorEffectSpatialPattern, ColorEffectSpatialPlacement,
    ColorEffectSpatialPlacementTarget, ColorEffectSpatialRecipe, ColorEffectSpatialSamplingRule,
    ColorEffectSpatialSparkleRasterMode, ColorEffectStop, CueEffectTarget, CueFixtureTarget,
    CueListSummary, CueSummary, DaslightCurveSource, DaslightCustomCurvePoint,
    DaslightCustomCurveSource, DmxControlAction, DmxControlMapping, DmxModeSummary,
    DmxOutputConfig, DmxUniversePreview, EffectBeamTarget, EffectBlendMode, EffectClockSync,
    EffectParamsSnapshot, EngineSnapshot, FixtureProfileSummary, GeometrySummary, LfoEffectRequest,
    LfoShape, MidiControlAction, MidiControlFeedback, MidiControlMapping, MidiControlMessage,
    MidiFeedbackMessage, MoveCoordinateMode, MoveDirection, MoveEffectBeamTarget,
    MoveEffectRequest, MoveInterpolation, MovePathPoint, PatchedFixtureSummary, ProjectFile,
    Rotation3, StageMapConfig, TimelineAudioClipSummary, TimelineCueEventSummary,
    TimelineLayerKind, TimelineLayerSummary, TimelineTrackKind, TouchControlBinding,
    TouchControlKind, TouchControlSummary, TouchFeaturePresetTarget, TouchPageSummary,
    TouchSurfaceSummary, ValueEffectDirection, ValueEffectInterpolation, ValueEffectMode,
    ValueEffectPoint, ValueEffectRequest, Vec3, COLOR_EFFECT_SPATIAL_PARAMETER_MODEL_VERSION,
};
use roxmltree::{Document, Node};
use serde::Serialize;

const DVC_FILE_MAX_BYTES: u64 = 64 * 1024 * 1024;
const DVC_PATCH_MAX_BYTES: usize = 128 * 1024 * 1024;
const DVC_FIXTURE_DATA_MAX_BYTES: usize = 16 * 1024 * 1024;
const DVC_FIXTURE_RECORD_BYTES: usize = 27;
const DVC_BEAM_FEATURE_SLOTS: usize = 13;
const DVC_BEAM_MISMATCH_WARNING_LIMIT: usize = 16;
const DVC_REPORT_DETAIL_LIMIT: usize = 256;
const DASLIGHT_CURVE_SAMPLE_MS: u16 = 40;
// Keep this paired with app/src/fixtureVisuals.ts::mappingFixtureGridUnit,
// the nominal par/point cell used by mappingFixtureStageSize.
const SYNDOCAL_STANDARD_FIXTURE_GLYPH_WORLD_SIZE: f32 = 5.0;
const DVC_DEFAULT_FIXTURE_SIZE: f32 = 30.0;

// Feature slots identified empirically across the five local Daslight projects
// (525 payloads, 61 distinct row patterns): slots 0..2 mirror the unique
// ColorRed/Green/Blue channels and slot 11 mirrors the unique Dimmer channel.
// The remaining slots are not yet identified and are never used for import.
const DVC_BEAM_FEATURE_BINDINGS: [(usize, &str); 4] = [
    (0, "ColorRed"),
    (1, "ColorGreen"),
    (2, "ColorBlue"),
    (11, "Dimmer"),
];

#[derive(Debug)]
pub(crate) struct DvcImportOutcome {
    pub(crate) project: ProjectFile,
    pub(crate) report: DvcImportReport,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub(crate) struct DvcImportCategory {
    pub(crate) count: usize,
    pub(crate) details: Vec<DvcImportDetail>,
}

impl DvcImportCategory {
    fn add(&mut self, count: usize, item: impl Into<String>, message: impl Into<String>) {
        if count == 0 {
            return;
        }
        self.count = self.count.saturating_add(count);
        if self.details.len() < DVC_REPORT_DETAIL_LIMIT {
            self.details.push(DvcImportDetail {
                item: item.into(),
                message: message.into(),
            });
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct DvcImportDetail {
    pub(crate) item: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub(crate) struct DvcImportSummary {
    pub(crate) fixtures: usize,
    pub(crate) profiles: usize,
    pub(crate) fixture_groups: usize,
    pub(crate) groups: usize,
    pub(crate) cues: usize,
    pub(crate) values_decoded: usize,
    pub(crate) values_skipped: usize,
    pub(crate) beam_records: usize,
    pub(crate) beam_feature_checks: usize,
    pub(crate) beam_feature_mismatches: usize,
    pub(crate) timeline_audio_clips: usize,
    pub(crate) timeline_scene_blocks: usize,
    pub(crate) effects_converted: usize,
    pub(crate) effects_skipped: usize,
    pub(crate) unknown_channel_types: usize,
    pub(crate) missing_audio_files: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(crate) struct DvcImportReport {
    pub(crate) path: String,
    pub(crate) das_build: String,
    pub(crate) version_file: String,
    pub(crate) summary: DvcImportSummary,
    pub(crate) converted: DvcImportCategory,
    pub(crate) approximate: DvcImportCategory,
    pub(crate) skipped: DvcImportCategory,
    pub(crate) unsupported: DvcImportCategory,
    pub(crate) warnings: Vec<String>,
    pub(crate) midi_mappings: Vec<MidiControlMapping>,
    pub(crate) dmx_mappings: Vec<DmxControlMapping>,
}

impl DvcImportReport {
    fn new(path: &str, root: Node<'_, '_>) -> Self {
        Self {
            path: path.to_string(),
            das_build: root.attribute("DASBUILD").unwrap_or_default().to_string(),
            version_file: root
                .attribute("VERSIONFILE")
                .unwrap_or_default()
                .to_string(),
            summary: DvcImportSummary::default(),
            converted: DvcImportCategory::default(),
            approximate: DvcImportCategory::default(),
            skipped: DvcImportCategory::default(),
            unsupported: DvcImportCategory::default(),
            warnings: Vec::new(),
            midi_mappings: Vec::new(),
            dmx_mappings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct DvcPresetBinding {
    name: String,
    preset_type: Option<u16>,
}

#[derive(Debug, Clone)]
struct DvcChannelBinding {
    attribute: String,
    raw_offsets: Vec<usize>,
    resolution: AttributeResolution,
    raw_channel_index: usize,
    channel_type: u16,
    presets: Vec<DvcPresetBinding>,
}

#[derive(Debug, Clone)]
struct ParsedProfile {
    summary: FixtureProfileSummary,
    physical_channel_count: usize,
    bindings: Vec<DvcChannelBinding>,
}

#[derive(Debug, Clone)]
struct FixtureImportRef {
    fixture_id: u64,
    fixture_index: usize,
    profile_index: usize,
    supports_dimmer: bool,
    color_beam_count: u16,
    patch_beam_positions: HashMap<u16, DvcPatchCanvasPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DvcPatchCanvasPoint {
    x: i64,
    y: i64,
}

#[derive(Debug, Default)]
struct ParsedSceneEffects {
    targets: Vec<CueEffectTarget>,
    notes: Vec<String>,
}

#[derive(Debug)]
struct ConvertedDvcEffect {
    target: Option<CueEffectTarget>,
    generator: &'static str,
    note: String,
    approximations: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct DvcRackTargets {
    ordered_steps: Vec<Vec<u64>>,
    fixture_ids: Vec<u64>,
    beam_targets: Vec<ColorEffectBeamTarget>,
}

#[derive(Debug, Clone)]
struct DvcRackFeatureSpec {
    preset_type: u16,
    low: u16,
    high: u16,
    source: String,
}

#[derive(Debug)]
struct FixtureDataValues {
    values: Vec<Option<u8>>,
    record_bytes: usize,
    beam_features: Option<Vec<[f32; DVC_BEAM_FEATURE_SLOTS]>>,
    beam_feature_error: Option<String>,
}

pub(crate) fn import_path(path: &Path) -> Result<DvcImportOutcome, String> {
    validate_dvc_path(path)?;
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > DVC_FILE_MAX_BYTES {
        return Err(format!(
            "Daslight project is {} bytes; the limit is {DVC_FILE_MAX_BYTES} bytes",
            metadata.len()
        ));
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    import_bytes(&bytes, &path.to_string_lossy())
}

fn validate_dvc_path(path: &Path) -> Result<(), String> {
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("dvc"))
    {
        return Err(format!(
            "Daslight project files must use the .dvc extension: {}",
            path.to_string_lossy()
        ));
    }
    if !path.is_file() {
        return Err(format!(
            "Daslight project file was not found: {}",
            path.to_string_lossy()
        ));
    }
    Ok(())
}

fn import_bytes(bytes: &[u8], path_label: &str) -> Result<DvcImportOutcome, String> {
    if bytes.len() as u64 > DVC_FILE_MAX_BYTES {
        return Err(format!(
            "Daslight project is {} bytes; the limit is {DVC_FILE_MAX_BYTES} bytes",
            bytes.len()
        ));
    }
    let source = std::str::from_utf8(bytes)
        .map_err(|error| format!("Daslight project XML is not UTF-8: {error}"))?;
    let document = Document::parse(source)
        .map_err(|error| format!("Daslight project XML is invalid: {error}"))?;
    let root = document.root_element();
    validate_dvc_root(root)?;
    let mut report = DvcImportReport::new(path_label, root);

    let patch_data = direct_child(root, "PATCHS")
        .and_then(|node| node.attribute("DATA"))
        .ok_or_else(|| "Daslight project is missing PATCHS DATA".to_string())?;
    let patch_xml = decode_qcompress_base64(patch_data)?;
    let patch_source = std::str::from_utf8(&patch_xml)
        .map_err(|error| format!("Daslight PATCH XML is not UTF-8: {error}"))?;
    let patch_document = Document::parse(patch_source)
        .map_err(|error| format!("Daslight PATCH XML is invalid: {error}"))?;
    let patch_root = patch_document.root_element();
    if !patch_root.has_tag_name("PATCH") {
        return Err(format!(
            "Daslight PATCH payload has unexpected root <{}>",
            patch_root.tag_name().name()
        ));
    }

    let (profiles, mut fixtures, fixture_refs, fixture_sizes) =
        parse_patch(patch_root, root, &mut report)?;
    let stage_map = scale_fixture_layout(&mut fixtures, &fixture_sizes);

    let mut snapshot = EngineSnapshot::default();
    snapshot.fixtures = fixtures;
    snapshot.stage_map = stage_map;
    snapshot.group_colors = BTreeMap::new();
    let (mut cues, scene_indices, cue_lists) = parse_scenes(
        root,
        &profiles,
        &fixture_refs,
        &mut snapshot.fixtures,
        &mut snapshot.group_colors,
        &mut report,
    )?;
    let bank_count = cue_lists.len();
    if !cue_lists.is_empty() {
        snapshot.cue_lists = cue_lists;
    }
    parse_super_scenes(root, &mut cues, &scene_indices, &mut snapshot, &mut report)?;
    snapshot.cues = cues;
    snapshot.touch_surface = parse_touch_surface(
        root,
        &scene_indices,
        &snapshot.cues,
        &profiles,
        &fixture_refs,
        &snapshot.fixtures,
        &mut report,
    );
    let midi_mappings = parse_midi_shortcuts(root, &scene_indices, &snapshot.cues, &mut report);
    report.midi_mappings = midi_mappings;
    report.dmx_mappings = parse_dmx_shortcuts(root, &profiles, &fixture_refs, &mut report);
    configure_disabled_dmx_routes(&mut snapshot);

    let fixture_group_count = direct_child(root, "FIXTUREGROUPS")
        .map(|section| element_children(section).count())
        .unwrap_or(0);
    report.summary.fixtures = snapshot.fixtures.len();
    report.summary.profiles = profiles.len();
    report.summary.fixture_groups = fixture_group_count;
    report.summary.groups = bank_count;
    report.summary.cues = snapshot.cues.len();
    report.converted.add(
        profiles.len(),
        "Embedded fixture profiles",
        format!("{} self-contained profile(s)", profiles.len()),
    );
    report.converted.add(
        snapshot.fixtures.len(),
        "DMX patch",
        format!(
            "{} fixture(s), 1-based addresses preserved",
            snapshot.fixtures.len()
        ),
    );
    report.converted.add(
        fixture_group_count,
        "Fixture groups",
        format!("{fixture_group_count} Daslight fixture group(s)"),
    );
    report.converted.add(
        bank_count,
        "Scene banks",
        format!("{bank_count} bank group(s) with identity colors"),
    );
    report.converted.add(
        snapshot.cues.len(),
        "Cues",
        format!("{} Daslight scene(s)", snapshot.cues.len()),
    );

    count_ignored_sections(root, &mut report);
    let ignored_flags = root
        .descendants()
        .filter(|node| node.is_element())
        .flat_map(|node| node.attributes())
        .filter(|attribute| matches!(attribute.name(), "OUTMODE" | "FLAG"))
        .count();
    report.skipped.add(
        ignored_flags,
        "OUTMODE / FLAG",
        "Compressed fixture flags are intentionally not converted in DVC-1",
    );

    let project = ProjectFile {
        version: 1,
        app: "Syndocal".to_string(),
        operator_policy: None,
        custom_profiles: profiles
            .iter()
            .map(|profile| profile.summary.clone())
            .collect(),
        fixture_groups: Vec::new(),
        snapshot,
    };
    Ok(DvcImportOutcome { project, report })
}

fn validate_dvc_root(root: Node<'_, '_>) -> Result<(), String> {
    if !root.has_tag_name("DLMFILE") || root.attribute("TYPE") != Some("Daslight") {
        return Err("The selected file is not a Daslight project".to_string());
    }
    if root.attribute("VERSION") != Some("5") {
        return Err(format!(
            "Unsupported Daslight project version {}",
            root.attribute("VERSION").unwrap_or("unknown")
        ));
    }
    if root.attribute("VERSIONFILE") != Some("2") {
        return Err(format!(
            "Unsupported Daslight VERSIONFILE {}; DVC-1 is verified only for VERSIONFILE 2",
            root.attribute("VERSIONFILE").unwrap_or("unknown")
        ));
    }
    Ok(())
}

fn dvc_fixture_patch_beam_positions(fixture: Node<'_, '_>) -> HashMap<u16, DvcPatchCanvasPoint> {
    let mut positions = HashMap::new();
    if let (Some(x), Some(y)) = (
        fixture
            .attribute("POSX")
            .and_then(|value| value.parse::<i64>().ok()),
        fixture
            .attribute("POSY")
            .and_then(|value| value.parse::<i64>().ok()),
    ) {
        positions.insert(0, DvcPatchCanvasPoint { x, y });
    }

    // Daslight persists each sub-beam's already transformed, absolute Patch
    // position below the owning fixture. Prefer that authored coordinate over
    // the fixture origin for every explicitly listed beam, including index 0.
    for beam in element_children(fixture).filter(|node| node.has_tag_name("BEAM")) {
        let Some(index) = beam
            .attribute("INDEX")
            .and_then(|value| value.parse::<u16>().ok())
        else {
            continue;
        };
        positions.remove(&index);
        let (Some(x), Some(y)) = (
            beam.attribute("POSX")
                .and_then(|value| value.parse::<i64>().ok()),
            beam.attribute("POSY")
                .and_then(|value| value.parse::<i64>().ok()),
        ) else {
            continue;
        };
        positions.insert(index, DvcPatchCanvasPoint { x, y });
    }
    positions
}

fn parse_patch(
    patch_root: Node<'_, '_>,
    dvc_root: Node<'_, '_>,
    report: &mut DvcImportReport,
) -> Result<
    (
        Vec<ParsedProfile>,
        Vec<PatchedFixtureSummary>,
        HashMap<String, FixtureImportRef>,
        Vec<f32>,
    ),
    String,
> {
    let fixture_groups = fixture_group_memberships(dvc_root);
    let mut profiles = Vec::new();
    let mut fixtures = Vec::new();
    let mut fixture_refs = HashMap::new();
    let mut fixture_sizes = Vec::new();
    let mut profile_paths = HashSet::new();

    for (container_index, container) in element_children(patch_root)
        .filter(|node| node.has_tag_name("FIXTURES"))
        .enumerate()
    {
        let library = direct_child(container, "SSLLIBRARY").ok_or_else(|| {
            format!(
                "Daslight PATCH fixture container {} has no SSLLIBRARY",
                container_index + 1
            )
        })?;
        let mut profile = parse_profile(library, container_index, report)?;
        if !profile_paths.insert(profile.summary.source_path.clone()) {
            profile.summary.source_path =
                format!("{}-{}", profile.summary.source_path, container_index + 1);
            profile_paths.insert(profile.summary.source_path.clone());
        }
        let profile_index = profiles.len();
        for fixture_node in element_children(container).filter(|node| node.has_tag_name("FIXTURE"))
        {
            let fixture_uid = required_attribute(fixture_node, "DASUID", "FIXTURE")?.to_string();
            let fixture_id = u64::try_from(fixtures.len() + 1).unwrap_or(u64::MAX);
            let universe_one_based = parse_required_u16(fixture_node, "UNIVERS", "FIXTURE")?;
            if universe_one_based == 0 {
                return Err(format!(
                    "Daslight fixture {fixture_uid} has invalid 1-based universe 0"
                ));
            }
            let address = parse_required_u16(fixture_node, "ADDRESS", "FIXTURE")?;
            let label = non_empty_label(
                fixture_node.attribute("NAME"),
                &format!("Fixture {fixture_id}"),
            );
            if let Some(size) = parse_f32_attribute(fixture_node, "SIZE")
                .filter(|size| size.is_finite() && *size > f32::EPSILON)
            {
                fixture_sizes.push(size);
            }
            let attribute_values = profile
                .summary
                .dmx_modes
                .first()
                .map(|mode| {
                    mode.controls
                        .iter()
                        .map(|control| AttributeValueSummary {
                            attribute: control.attribute.clone(),
                            value: control.default_value,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let group_ids = fixture_groups
                .get(&fixture_uid)
                .cloned()
                .unwrap_or_default();
            let fixture_index = fixtures.len();
            let controls = profile
                .summary
                .dmx_modes
                .first()
                .map(|mode| mode.controls.clone())
                .unwrap_or_default();
            let supports_dimmer = controls
                .iter()
                .any(|control| normalize_dvc_attribute(&control.attribute) == "dimmer");
            let color_beam_count = u16::try_from(
                controls
                    .iter()
                    .filter(|control| {
                        normalize_dvc_attribute(&control.attribute)
                            .strip_prefix("colorred")
                            .is_some_and(|suffix| {
                                suffix.chars().all(|character| character.is_ascii_digit())
                            })
                    })
                    .count(),
            )
            .unwrap_or(u16::MAX);
            let patch_beam_positions = dvc_fixture_patch_beam_positions(fixture_node);
            fixtures.push(PatchedFixtureSummary {
                id: fixture_id,
                label,
                profile_source_path: profile.summary.source_path.clone(),
                profile_name: profile.summary.name.clone(),
                manufacturer: profile.summary.manufacturer.clone(),
                mode_name: profile
                    .summary
                    .dmx_modes
                    .first()
                    .map(|mode| mode.name.clone())
                    .unwrap_or_else(|| "Mode".to_string()),
                universe: universe_one_based - 1,
                address,
                group_ids,
                position: Vec3 {
                    x: parse_f32_attribute(fixture_node, "POSX").unwrap_or(0.0),
                    y: 0.0,
                    z: parse_f32_attribute(fixture_node, "POSY").unwrap_or(0.0),
                },
                rotation: Rotation3 {
                    pitch: 0.0,
                    yaw: parse_f32_attribute(fixture_node, "ANGLE").unwrap_or(0.0),
                    roll: 0.0,
                },
                geometries: profile.summary.geometries.clone(),
                controls,
                attribute_values,
                limits: protocol::FixtureLimits::default(),
                highlighted: false,
                soloed: false,
                parked: false,
            });
            if fixture_refs
                .insert(
                    fixture_uid.clone(),
                    FixtureImportRef {
                        fixture_id,
                        fixture_index,
                        profile_index,
                        supports_dimmer,
                        color_beam_count,
                        patch_beam_positions,
                    },
                )
                .is_some()
            {
                return Err(format!("Duplicate Daslight fixture DASUID {fixture_uid}"));
            }
        }
        profiles.push(profile);
    }

    let declared = patch_root
        .attribute("NBFIXTURE")
        .and_then(|value| value.parse::<usize>().ok());
    if declared.is_some_and(|count| count != fixtures.len()) {
        return Err(format!(
            "Daslight PATCH declares {} fixtures but contains {}",
            declared.unwrap_or_default(),
            fixtures.len()
        ));
    }
    Ok((profiles, fixtures, fixture_refs, fixture_sizes))
}

fn parse_profile(
    library: Node<'_, '_>,
    container_index: usize,
    report: &mut DvcImportReport,
) -> Result<ParsedProfile, String> {
    let library_name = required_attribute(library, "SSLNAME", "SSLLIBRARY")?;
    let fixture_uid = non_empty_label(
        library.attribute("SSLFIXUID"),
        &format!("profile-{}", container_index + 1),
    );
    let mode = library
        .descendants()
        .find(|node| node.has_tag_name("SSLMODE"))
        .ok_or_else(|| format!("Daslight profile {library_name} has no SSLMODE"))?;
    let channel_nodes = element_children(mode)
        .filter(|node| node.has_tag_name("SSLCHANNEL"))
        .collect::<Vec<_>>();
    let declared_channel_count = mode
        .attribute("SSLNBCHANNEL")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(channel_nodes.len());
    if declared_channel_count != channel_nodes.len() || channel_nodes.is_empty() {
        return Err(format!(
            "Daslight profile {library_name} mode declares {declared_channel_count} channels but contains {}",
            channel_nodes.len()
        ));
    }

    let mut consumed = HashSet::new();
    let mut attribute_counts = HashMap::<String, usize>::new();
    let mut controls = Vec::new();
    let mut bindings = Vec::new();
    for (channel_index, channel) in channel_nodes.iter().copied().enumerate() {
        if consumed.contains(&channel_index) {
            continue;
        }
        let channel_type = channel
            .attribute("SSLCHANNELTYPE")
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(u16::MAX);
        let channel_name = non_empty_label(
            channel.attribute("SSLCHANNELNAME"),
            &format!("Channel {}", channel_index + 1),
        );
        let (attribute_base, unknown) = dvc_channel_attribute(channel_type, &channel_name);
        if unknown {
            report.summary.unknown_channel_types =
                report.summary.unknown_channel_types.saturating_add(1);
            report.approximate.add(
                1,
                format!("Channel type {channel_type}"),
                format!("{library_name}: {channel_name} imported as Generic"),
            );
        }

        let msb = channel
            .attribute("SSLCHANNELMSB")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let lsb = channel
            .attribute("SSLCHANNELLSB")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let paired_offsets = (msb > 0
            && lsb > 0
            && msb != lsb
            && msb <= channel_nodes.len()
            && lsb <= channel_nodes.len())
        .then(|| vec![msb - 1, lsb - 1]);
        let (raw_offsets, resolution) = if let Some(offsets) = paired_offsets {
            consumed.extend(offsets.iter().copied());
            (offsets, AttributeResolution::SixteenBit)
        } else {
            if msb != 0 || lsb != 0 {
                report.approximate.add(
                    1,
                    format!("{library_name}: {channel_name}"),
                    "Invalid MSB/LSB pair imported as one 8-bit channel",
                );
            }
            consumed.insert(channel_index);
            (vec![channel_index], AttributeResolution::EightBit)
        };
        let attribute = unique_attribute(attribute_base, &mut attribute_counts);
        let functions = parse_channel_functions(channel, &attribute, &resolution);
        let presets = channel
            .descendants()
            .filter(|node| node.has_tag_name("SSLPRESET"))
            .map(|preset| DvcPresetBinding {
                name: non_empty_label(preset.attribute("SSLPRESETNAME"), &attribute),
                preset_type: preset
                    .attribute("SSLPRESETTYPE")
                    .and_then(|value| value.parse::<u16>().ok()),
            })
            .collect();
        // Daslight LIVE has a blackout baseline when no scene is active. The
        // profile's SSLPRESETDMXDEFAULT is an editor reference value, so it is
        // intentionally not used as the imported control's idle default.
        let default_value = 0;
        controls.push(AttributeControl {
            attribute: attribute.clone(),
            channel_name,
            geometry: None,
            offsets: raw_offsets
                .iter()
                .map(|offset| u16::try_from(offset + 1).unwrap_or(u16::MAX))
                .collect(),
            resolution: resolution.clone(),
            default_value,
            functions,
        });
        bindings.push(DvcChannelBinding {
            attribute,
            raw_offsets,
            resolution,
            raw_channel_index: channel_index,
            channel_type,
            presets,
        });
    }

    let (manufacturer, name) = profile_identity(library_name);
    let mode_index = mode.attribute("SSLMODEINDEX").unwrap_or("0");
    let source_path = format!("snapshot://fixture/{fixture_uid}/mode-{mode_index}");
    let beam_angle = library
        .descendants()
        .find(|node| node.has_tag_name("SSLPROPERTIES"))
        .and_then(|node| parse_f32_attribute(node, "SSLBEAMOPENING"))
        .filter(|angle| angle.is_finite() && *angle > 0.0);
    let geometries = beam_angle
        .map(|angle| {
            vec![GeometrySummary {
                name: "Beam".to_string(),
                kind: "Beam".to_string(),
                parent: None,
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
                model_name: None,
                model_file: None,
                model_primitive: None,
                model_dimensions: None,
                beam_type: Some("Beam".to_string()),
                beam_angle_deg: Some(angle),
                field_angle_deg: Some(angle),
                beam_radius: None,
            }]
        })
        .unwrap_or_default();
    Ok(ParsedProfile {
        summary: FixtureProfileSummary {
            source_path,
            manufacturer,
            name,
            short_name: None,
            fixture_type_id: Some(fixture_uid),
            dmx_modes: vec![DmxModeSummary {
                name: format!("Daslight Mode {mode_index}"),
                controls,
            }],
            geometries,
            warnings: Vec::new(),
        },
        physical_channel_count: channel_nodes.len(),
        bindings,
    })
}

fn parse_channel_functions(
    channel: Node<'_, '_>,
    attribute: &str,
    resolution: &AttributeResolution,
) -> Vec<ChannelFunctionSummary> {
    channel
        .descendants()
        .filter(|node| node.has_tag_name("SSLPRESET"))
        .filter_map(|preset| {
            let start = preset.attribute("SSLPRESETDMXSTART")?.parse::<u16>().ok()?;
            let end = preset.attribute("SSLPRESETDMXEND")?.parse::<u16>().ok()?;
            Some(ChannelFunctionSummary {
                name: non_empty_label(preset.attribute("SSLPRESETNAME"), attribute),
                attribute: attribute.to_string(),
                parent_function: None,
                dmx_from: scale_dmx_value(start, resolution),
                dmx_to: scale_dmx_value(end, resolution),
                physical_from: preset
                    .attribute("SSLPRESETPARAMMIN")
                    .and_then(|value| value.parse::<f32>().ok()),
                physical_to: preset
                    .attribute("SSLPRESETPARAMMAX")
                    .and_then(|value| value.parse::<f32>().ok()),
                wheel_slot: None,
                wheel_slot_name: None,
                wheel_slot_color: preset
                    .attribute("SSLPRESETCOLOR")
                    .and_then(daslight_numeric_color),
                wheel_slot_media: None,
                emitter: None,
            })
        })
        .collect()
}

fn scale_dmx_value(value: u16, resolution: &AttributeResolution) -> u16 {
    match resolution {
        AttributeResolution::EightBit => value.min(255).saturating_mul(257),
        AttributeResolution::SixteenBit => value,
    }
}

fn dvc_channel_attribute(channel_type: u16, channel_name: &str) -> (String, bool) {
    let attribute = match channel_type {
        0 => format!("Generic: {channel_name}"),
        1 => "Pan".to_string(),
        2 => "Tilt".to_string(),
        5 => "Generic: ColorMacro".to_string(),
        7 => "Dimmer".to_string(),
        8 => "Gobo".to_string(),
        15 => "ShutterStrobe".to_string(),
        18 => "Generic: PanTiltSpeed".to_string(),
        24 => "Generic: Smoke".to_string(),
        25 => "ColorRed".to_string(),
        26 => "ColorGreen".to_string(),
        27 => "ColorBlue".to_string(),
        31 => "ColorWhite".to_string(),
        37 => "Generic: AutoSound".to_string(),
        45 => "ColorAmber".to_string(),
        46 => "Generic: UV".to_string(),
        48 => "Generic: Lime".to_string(),
        _ => return (format!("Generic: {channel_name}"), true),
    };
    (attribute, false)
}

fn unique_attribute(base: String, counts: &mut HashMap<String, usize>) -> String {
    let count = counts.entry(base.clone()).or_default();
    *count += 1;
    if *count == 1 {
        base
    } else {
        format!("{base} {}", *count)
    }
}

fn normalize_dvc_attribute(attribute: &str) -> String {
    attribute
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn profile_identity(library_name: &str) -> (String, String) {
    let normalized = library_name.replace('\\', "/");
    let mut segments = normalized
        .split('/')
        .filter(|segment| !segment.trim().is_empty())
        .collect::<Vec<_>>();
    let file_name = segments.pop().unwrap_or("Daslight Fixture");
    let name = file_name.strip_suffix(".ssl2").unwrap_or(file_name).trim();
    let manufacturer = segments.first().copied().unwrap_or("Daslight").trim();
    (
        non_empty_label(Some(manufacturer), "Daslight"),
        non_empty_label(Some(name), "Daslight Fixture"),
    )
}

fn fixture_group_memberships(root: Node<'_, '_>) -> HashMap<String, Vec<String>> {
    let mut memberships = HashMap::<String, Vec<String>>::new();
    let Some(section) = direct_child(root, "FIXTUREGROUPS") else {
        return memberships;
    };
    for (group_index, group) in element_children(section)
        .filter(|node| node.has_tag_name("FIXTUREGROUP"))
        .enumerate()
    {
        let group_name = non_empty_label(
            group.attribute("NAME"),
            &format!("Fixture Group {}", group_index + 1),
        );
        let Some(fixtures) = direct_child(group, "FIXTURES") else {
            continue;
        };
        for fixture in element_children(fixtures).filter(|node| node.has_tag_name("FIXTURE")) {
            let Some(uid) = fixture.attribute("DASUID") else {
                continue;
            };
            let groups = memberships.entry(uid.to_string()).or_default();
            if !groups.contains(&group_name) {
                groups.push(group_name.clone());
            }
        }
    }
    memberships
}

fn fixture_group_identities(root: Node<'_, '_>) -> HashMap<String, String> {
    let mut identities = HashMap::new();
    let Some(section) = direct_child(root, "FIXTUREGROUPS") else {
        return identities;
    };
    for (group_index, group) in element_children(section)
        .filter(|node| node.has_tag_name("FIXTUREGROUP"))
        .enumerate()
    {
        let Some(uid) = group.attribute("DASUID") else {
            continue;
        };
        identities.insert(
            uid.to_string(),
            non_empty_label(
                group.attribute("NAME"),
                &format!("Fixture Group {}", group_index + 1),
            ),
        );
    }
    identities
}

fn parse_touch_feature_preset_binding(
    shortcut: Node<'_, '_>,
    action: Node<'_, '_>,
    profiles: &[ParsedProfile],
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<TouchControlBinding, String> {
    let target = action
        .attribute("TARGET")
        .ok_or_else(|| "Daslight Feature Preset target was missing".to_string())?;
    let target_parts = target.split(':').collect::<Vec<_>>();
    if target_parts.len() != 3 {
        return Err(format!(
            "Daslight Feature Preset target '{target}' has an unsupported shape"
        ));
    }

    let selected_fixture_uids = direct_child(action, "BEAMS")
        .into_iter()
        .flat_map(element_children)
        .filter(|node| node.has_tag_name("BEAM"))
        .filter_map(|beam| beam.attribute("FIXTURE"))
        .collect::<Vec<_>>();
    if selected_fixture_uids.is_empty() {
        return Err("Daslight Feature Preset has no selected BEAMS".to_string());
    }

    let mut targets = Vec::new();
    let mut seen_targets = HashSet::<(u64, String)>::new();
    for fixture_uid in selected_fixture_uids {
        let Some(fixture_ref) = fixture_refs.get(fixture_uid) else {
            continue;
        };
        let Some(profile) = profiles.get(fixture_ref.profile_index) else {
            continue;
        };
        let binding = if target == ":-1:4" {
            // Daslight exposes this target as "Generic: Dimmer". Live DMX
            // Levels verification shows that it applies to canonical Dimmer
            // presets, but not profile-specific presets such as "Dimmer
            // linear" even when both channels use SSLCHANNELTYPE=7.
            let candidates = profile
                .bindings
                .iter()
                .filter(|binding| {
                    binding.channel_type == 7
                        && binding
                            .presets
                            .iter()
                            .any(|preset| preset.name.eq_ignore_ascii_case("Dimmer"))
                })
                .collect::<Vec<_>>();
            match candidates.as_slice() {
                [binding] => Some(*binding),
                _ => None,
            }
        } else {
            let profile_uid = target_parts[0];
            let channel_index = target_parts[1].parse::<usize>().map_err(|_| {
                format!("Daslight Feature Preset target '{target}' has an invalid channel index")
            })?;
            let preset_index = target_parts[2].parse::<usize>().map_err(|_| {
                format!("Daslight Feature Preset target '{target}' has an invalid preset index")
            })?;
            if profile.summary.fixture_type_id.as_deref() != Some(profile_uid) {
                None
            } else {
                profile.bindings.iter().find(|binding| {
                    binding.raw_channel_index == channel_index
                        && binding.presets.get(preset_index).is_some()
                })
            }
        };
        let Some(binding) = binding else {
            continue;
        };
        let key = (fixture_ref.fixture_id, binding.attribute.clone());
        if seen_targets.insert(key.clone()) {
            targets.push(TouchFeaturePresetTarget {
                fixture_id: key.0,
                attribute: key.1,
            });
        }
    }
    if targets.is_empty() {
        return Err(format!(
            "Daslight Feature Preset target '{target}' resolved to no imported fixture attributes"
        ));
    }

    let settings = direct_child(shortcut, "SETTINGS");
    let parse_normalized = |attribute: &str, fallback: f32| -> Result<u16, String> {
        let value = settings
            .and_then(|node| node.attribute(attribute))
            .map(str::parse::<f32>)
            .transpose()
            .map_err(|_| format!("Daslight Feature Preset {attribute} was invalid"))?
            .unwrap_or(fallback);
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!(
                "Daslight Feature Preset {attribute}={value} is outside the supported 0..1 range"
            ));
        }
        Ok((value * f32::from(u16::MAX)).round() as u16)
    };
    let min_value = parse_normalized("MIN", 0.0)?;
    let max_value = parse_normalized("MAX", 1.0)?;
    if min_value > max_value {
        return Err("Daslight Feature Preset MIN is greater than MAX".to_string());
    }
    let inverted = settings
        .and_then(|node| node.attribute("INV"))
        .is_some_and(|value| value == "1");

    Ok(TouchControlBinding::FeaturePreset {
        targets,
        min_value,
        max_value,
        inverted,
    })
}

#[derive(Debug)]
struct ParsedDvcMidiEvent {
    message: MidiControlMessage,
    channel: u8,
    number: u8,
    value: u8,
    device: String,
}

fn parse_dvc_midi_event(data: &str) -> Result<ParsedDvcMidiEvent, String> {
    let mut parts = data.splitn(5, ':');
    let status = parts
        .next()
        .ok_or_else(|| "MIDI status was missing".to_string())?
        .parse::<u8>()
        .map_err(|_| "MIDI status was invalid".to_string())?;
    let channel = parts
        .next()
        .ok_or_else(|| "MIDI channel was missing".to_string())?
        .parse::<u8>()
        .map_err(|_| "MIDI channel was invalid".to_string())?;
    let number = parts
        .next()
        .ok_or_else(|| "MIDI control number was missing".to_string())?
        .parse::<u8>()
        .map_err(|_| "MIDI control number was invalid".to_string())?;
    let learned_value = parts
        .next()
        .ok_or_else(|| "MIDI learned value was missing".to_string())?
        .parse::<u8>()
        .map_err(|_| "MIDI learned value was invalid".to_string())?;
    if channel > 15 {
        return Err(format!("MIDI channel {channel} is outside 0..15"));
    }
    if number > 127 {
        return Err(format!("MIDI control number {number} is outside 0..127"));
    }
    if learned_value > 127 {
        return Err(format!(
            "MIDI learned value {learned_value} is outside 0..127"
        ));
    }
    let message = match status & 0xf0 {
        0x80 => MidiControlMessage::NoteOff,
        0x90 => MidiControlMessage::NoteOn,
        0xb0 => MidiControlMessage::ControlChange,
        0xc0 => MidiControlMessage::ProgramChange,
        _ => {
            return Err(format!(
                "MIDI status {status} is not a supported channel message"
            ))
        }
    };
    Ok(ParsedDvcMidiEvent {
        message,
        channel,
        number,
        value: learned_value,
        device: parts.next().unwrap_or_default().trim().to_string(),
    })
}

fn dvc_midi_feedback_message(event: ParsedDvcMidiEvent) -> MidiFeedbackMessage {
    MidiFeedbackMessage {
        message: event.message,
        channel: event.channel,
        number: event.number,
        value: event.value,
    }
}

fn dvc_midi_mapping(
    event: ParsedDvcMidiEvent,
    action: MidiControlAction,
    cue_id: Option<u64>,
) -> MidiControlMapping {
    MidiControlMapping {
        channel: Some(event.channel),
        message: event.message,
        number: event.number,
        action,
        fixture_id: None,
        attribute: None,
        group_id: None,
        cue_id,
        layer_id: None,
        output_id: None,
        video_param: None,
        cue_point_index: None,
        duration_ms: None,
        feedback: None,
        low: 0.0,
        high: 1.0,
    }
}

fn dvc_midi_direction_mapping(
    event: ParsedDvcMidiEvent,
    action: MidiControlAction,
    cue_id: u64,
    direction: &str,
) -> MidiControlMapping {
    let mut mapping = dvc_midi_mapping(event, action, Some(cue_id));
    mapping.attribute = Some(direction.to_string());
    mapping
}

fn dvc_midi_selected_feature_fader_mapping(
    event: ParsedDvcMidiEvent,
    target_index: usize,
) -> MidiControlMapping {
    let mut mapping = dvc_midi_mapping(event, MidiControlAction::SelectedFeatureFader, None);
    mapping.cue_point_index = Some(target_index);
    mapping.high = 65_535.0;
    mapping
}

fn parse_dvc_dmx_event(data: &str) -> Result<(u16, u16), String> {
    let (path, scalar_selector) = data
        .trim()
        .rsplit_once(':')
        .ok_or_else(|| format!("DMX mapping EVENT DATA '{data}' was malformed"))?;
    if scalar_selector != "5" {
        return Err(format!(
            "DMX mapping EVENT DATA '{data}' uses unverified scalar selector {scalar_selector}"
        ));
    }
    let segments = path.trim_matches('/').split('/').collect::<Vec<_>>();
    if segments.len() != 3 || !segments[0].eq_ignore_ascii_case("dmx") {
        return Err(format!("DMX mapping EVENT DATA '{data}' was malformed"));
    }
    let daslight_universe = segments[1]
        .parse::<u16>()
        .ok()
        .filter(|universe| *universe > 0)
        .ok_or_else(|| format!("DMX mapping EVENT DATA '{data}' has an invalid universe"))?;
    let channel = segments[2]
        .parse::<u16>()
        .ok()
        .filter(|channel| (1..=512).contains(channel))
        .ok_or_else(|| format!("DMX mapping EVENT DATA '{data}' has an invalid channel"))?;
    Ok((daslight_universe - 1, channel))
}

fn parse_dvc_dmx_range(settings: Node<'_, '_>) -> Result<(f32, f32), String> {
    for (field, expected) in [
        ("SMODE", "1"),
        ("CMODE", "1"),
        ("TMODE", "0"),
        ("LOOP", "0"),
        ("FLASH", "0"),
    ] {
        if settings.attribute(field) != Some(expected) {
            return Err(format!(
                "DMX mapping setting {field}={} is not verified",
                settings.attribute(field).unwrap_or("missing")
            ));
        }
    }
    let increment = settings
        .attribute("INC")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite());
    if !increment.is_some_and(|value| (value - 0.001).abs() <= f32::EPSILON) {
        return Err(format!(
            "DMX mapping setting INC={} is not verified",
            settings.attribute("INC").unwrap_or("missing")
        ));
    }
    let parse_level = |field: &str| {
        settings
            .attribute(field)
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
            .ok_or_else(|| format!("DMX mapping {field} must be a finite value from 0 to 1"))
    };
    let mut low = parse_level("MIN")? * 65_535.0;
    let mut high = parse_level("MAX")? * 65_535.0;
    match settings.attribute("INV").unwrap_or("0") {
        "0" => {}
        "1" => std::mem::swap(&mut low, &mut high),
        value => return Err(format!("DMX mapping INV={value} is not verified")),
    }
    Ok((low, high))
}

fn parse_dmx_shortcuts(
    root: Node<'_, '_>,
    profiles: &[ParsedProfile],
    fixture_refs: &HashMap<String, FixtureImportRef>,
    report: &mut DvcImportReport,
) -> Vec<DmxControlMapping> {
    let Some(shortcuts) = direct_child(root, "SHORTCUTS") else {
        return Vec::new();
    };
    let mut mappings = Vec::new();
    for (shortcut_index, shortcut) in element_children(shortcuts)
        .filter(|node| node.has_tag_name("SHORTCUT"))
        .enumerate()
    {
        if shortcut.attribute("TYPE") != Some("3") {
            continue;
        }
        let item = format!("Shortcut {}", shortcut_index + 1);
        let Some(event_data) =
            direct_child(shortcut, "EVENT").and_then(|event| event.attribute("DATA"))
        else {
            report
                .skipped
                .add(1, item, "DMX mapping EVENT DATA was missing");
            continue;
        };
        let (universe, channel) = match parse_dvc_dmx_event(event_data) {
            Ok(source) => source,
            Err(message) => {
                report.skipped.add(1, item, message);
                continue;
            }
        };
        let Some(action) = direct_child(shortcut, "ACTION") else {
            report
                .skipped
                .add(1, item, "DMX mapping ACTION was missing");
            continue;
        };
        if action.attribute("TYPE") != Some("210") {
            report.unsupported.add(
                1,
                item,
                format!(
                    "Daslight DMX action type {} is not yet mapped",
                    action.attribute("TYPE").unwrap_or("missing")
                ),
            );
            continue;
        }
        let Some((target_profile_uid, raw_channel_index)) = action
            .attribute("TARGET")
            .and_then(|target| target.rsplit_once(':'))
            .and_then(|(profile_uid, index)| {
                index
                    .parse::<usize>()
                    .ok()
                    .map(|index| (profile_uid, index))
            })
        else {
            report
                .skipped
                .add(1, item, "DMX Feature mapping TARGET was malformed");
            continue;
        };
        let Some(settings) = direct_child(shortcut, "SETTINGS") else {
            report
                .skipped
                .add(1, item, "DMX mapping SETTINGS were missing");
            continue;
        };
        let (low, high) = match parse_dvc_dmx_range(settings) {
            Ok(range) => range,
            Err(message) => {
                report.skipped.add(1, item, message);
                continue;
            }
        };
        let Some(beams) = direct_child(action, "BEAMS") else {
            report
                .skipped
                .add(1, item, "DMX Feature mapping BEAMS were missing");
            continue;
        };
        let mut target_count = 0_usize;
        for (beam_index, beam) in element_children(beams)
            .filter(|node| node.has_tag_name("BEAM"))
            .enumerate()
        {
            let target_item = format!("{item} target {}", beam_index + 1);
            if beam.attribute("BEAMID") != Some("0") {
                report.skipped.add(
                    1,
                    target_item,
                    "DMX Feature mapping for a non-primary beam is not yet proven",
                );
                continue;
            }
            let Some(fixture_ref) = beam
                .attribute("FIXTURE")
                .and_then(|fixture_uid| fixture_refs.get(fixture_uid))
            else {
                report.skipped.add(
                    1,
                    target_item,
                    "DMX Feature mapping referenced a missing imported fixture",
                );
                continue;
            };
            let Some(profile) = profiles.get(fixture_ref.profile_index) else {
                report
                    .skipped
                    .add(1, target_item, "DMX Feature mapping profile was missing");
                continue;
            };
            let expected_profile_fragment = format!("/{target_profile_uid}/");
            if !profile
                .summary
                .source_path
                .contains(&expected_profile_fragment)
            {
                report.skipped.add(
                    1,
                    target_item,
                    "DMX Feature mapping TARGET profile did not match its fixture profile",
                );
                continue;
            }
            let Some(binding) = profile
                .bindings
                .iter()
                .find(|binding| binding.raw_channel_index == raw_channel_index)
            else {
                report.skipped.add(
                    1,
                    target_item,
                    format!("DMX Feature mapping channel index {raw_channel_index} was missing"),
                );
                continue;
            };
            mappings.push(DmxControlMapping {
                universe,
                channel,
                action: DmxControlAction::FixtureAttribute,
                fixture_id: Some(fixture_ref.fixture_id),
                attribute: Some(binding.attribute.clone()),
                group_id: None,
                cue_id: None,
                layer_id: None,
                output_id: None,
                video_param: None,
                cue_point_index: None,
                duration_ms: None,
                low,
                high,
            });
            target_count = target_count.saturating_add(1);
        }
        if target_count == 0 {
            report
                .skipped
                .add(1, item, "DMX Feature mapping had no proven targets");
        }
    }
    report.converted.add(
        mappings.len(),
        "DMX control mappings",
        format!(
            "{} verified Daslight DMX Feature mapping target(s), with 1-based source universes normalized to internal universes",
            mappings.len()
        ),
    );
    mappings
}

fn parse_midi_shortcuts(
    root: Node<'_, '_>,
    scene_indices: &HashMap<String, usize>,
    cues: &[CueSummary],
    report: &mut DvcImportReport,
) -> Vec<MidiControlMapping> {
    let Some(shortcuts) = direct_child(root, "SHORTCUTS") else {
        return Vec::new();
    };
    let mut mappings = Vec::new();
    let mut device_affinity_count = 0_usize;
    let mut custom_feedback_count = 0_usize;
    let mut feedback_output_affinity_count = 0_usize;
    for (shortcut_index, shortcut) in element_children(shortcuts)
        .filter(|node| node.has_tag_name("SHORTCUT"))
        .enumerate()
    {
        let item = format!("Shortcut {}", shortcut_index + 1);
        let shortcut_type = shortcut.attribute("TYPE").unwrap_or_default();
        if shortcut_type == "2" || shortcut_type == "3" {
            continue;
        }
        if shortcut_type != "1" {
            report.unsupported.add(
                1,
                item,
                format!("Daslight shortcut type {shortcut_type} is not yet mapped"),
            );
            continue;
        }
        let Some(event_data) =
            direct_child(shortcut, "EVENT").and_then(|event| event.attribute("DATA"))
        else {
            report
                .skipped
                .add(1, item, "MIDI mapping EVENT DATA was missing");
            continue;
        };
        let event = match parse_dvc_midi_event(event_data) {
            Ok(event) => event,
            Err(message) => {
                report.skipped.add(1, item, message);
                continue;
            }
        };
        let has_device_affinity = !event.device.is_empty();
        let Some(action_node) = direct_child(shortcut, "ACTION") else {
            report
                .skipped
                .add(1, item, "MIDI mapping ACTION was missing");
            continue;
        };
        let action_type = action_node.attribute("TYPE").unwrap_or_default();
        let settings = direct_child(shortcut, "SETTINGS");
        let mut mapped = match action_type {
            // Verified in Daslight's mapping UI and the matching Touch action.
            "107" => {
                let Some(cue_id) = action_node
                    .attribute("TARGET")
                    .and_then(|target| scene_indices.get(target))
                    .and_then(|index| cues.get(*index))
                    .map(|cue| cue.id)
                else {
                    report.skipped.add(
                        1,
                        item,
                        "Daslight Scene Play mapping referenced a missing imported cue",
                    );
                    continue;
                };
                let action = if settings
                    .and_then(|node| node.attribute("FLASH"))
                    .is_some_and(|value| value == "1")
                {
                    MidiControlAction::FlashCue
                } else {
                    MidiControlAction::TriggerCue
                };
                dvc_midi_mapping(event, action, Some(cue_id))
            }
            // Verified by a local Daslight project and the matching Touch action.
            "55" => dvc_midi_mapping(event, MidiControlAction::TapBpm, None),
            // Daslight's embedded contiguous action table identifies these as
            // Scene Play Forwards / Backwards / Back & Forth. Preserve both
            // the target Scene and its launch direction.
            "108" | "109" | "110" => {
                let Some(cue_id) = action_node
                    .attribute("TARGET")
                    .and_then(|target| scene_indices.get(target))
                    .and_then(|index| cues.get(*index))
                    .map(|cue| cue.id)
                else {
                    report.skipped.add(
                        1,
                        item,
                        "Daslight directional Scene Play mapping referenced a missing imported cue",
                    );
                    continue;
                };
                let direction = match action_type {
                    "108" => "Forward",
                    "109" => "Reverse",
                    "110" => "Bounce",
                    _ => unreachable!(),
                };
                let flash = settings
                    .and_then(|node| node.attribute("FLASH"))
                    .is_some_and(|value| value == "1");
                if flash
                    && !matches!(
                        event.message,
                        MidiControlMessage::NoteOn | MidiControlMessage::ControlChange
                    )
                {
                    report.skipped.add(
                        1,
                        item,
                        "Directional Scene flash requires a Note On or Control Change input with a release value",
                    );
                    continue;
                }
                dvc_midi_direction_mapping(
                    event,
                    if flash {
                        MidiControlAction::FlashCueDirection
                    } else {
                        MidiControlAction::TriggerCueDirection
                    },
                    cue_id,
                    direction,
                )
            }
            // Daslight's embedded action table places Bank Next at 113. TARGET
            // is a Scene in that Bank, so preserve it as the Cue List anchor
            // instead of rounding this down to the global Next action.
            "113" => {
                let Some(cue_id) = action_node
                    .attribute("TARGET")
                    .and_then(|target| scene_indices.get(target))
                    .and_then(|index| cues.get(*index))
                    .map(|cue| cue.id)
                else {
                    report.skipped.add(
                        1,
                        item,
                        "Daslight Bank Next mapping referenced a missing imported cue",
                    );
                    continue;
                };
                dvc_midi_mapping(event, MidiControlAction::TriggerCueListNext, Some(cue_id))
            }
            // Daslight's embedded action table identifies 229 as Fader. Its
            // DVC payload carries only TARGETINDEX, so it intentionally follows
            // the current selected fixtures and visible feature-fader order
            // instead of being guessed as a fixed Dimmer attribute.
            "229" => {
                let Some(target_index) = action_node
                    .attribute("TARGETINDEX")
                    .and_then(|value| value.parse::<usize>().ok())
                else {
                    report.skipped.add(
                        1,
                        item,
                        "Daslight Fader mapping was missing a numeric target index",
                    );
                    continue;
                };
                dvc_midi_selected_feature_fader_mapping(event, target_index)
            }
            _ => {
                let target_index = action_node.attribute("TARGETINDEX").unwrap_or("unknown");
                report.unsupported.add(
                    1,
                    item,
                    format!(
                        "Daslight MIDI action type {action_type} (target index {target_index}) is not yet mapped"
                    ),
                );
                continue;
            }
        };
        if has_device_affinity {
            // The selected input remains a Setup > I/O concern in Syndocal.
            // Count this separately so the import report keeps that boundary visible.
            device_affinity_count = device_affinity_count.saturating_add(1);
        }
        if let Some(settings) = settings {
            let mut feedback = MidiControlFeedback::default();
            let mut output_has_device_affinity = settings
                .attribute("DEVICEOUT")
                .is_some_and(|value| !value.trim().is_empty());
            for (attribute, state) in [("OUT", "Off"), ("OUT1", "On"), ("OUT2", "Unknown")] {
                let Some(raw) = settings
                    .attribute(attribute)
                    .filter(|value| !value.trim().is_empty())
                else {
                    continue;
                };
                match parse_dvc_midi_event(raw) {
                    Ok(event) => {
                        output_has_device_affinity |= !event.device.is_empty();
                        let message = Some(dvc_midi_feedback_message(event));
                        match attribute {
                            "OUT" => feedback.off = message,
                            "OUT1" => feedback.on = message,
                            "OUT2" => feedback.unknown = message,
                            _ => unreachable!(),
                        }
                        custom_feedback_count = custom_feedback_count.saturating_add(1);
                    }
                    Err(message) => {
                        report
                            .skipped
                            .add(1, format!("{item} MIDI feedback {state}"), message)
                    }
                }
            }
            if feedback.off.is_some() || feedback.on.is_some() || feedback.unknown.is_some() {
                mapped.feedback = Some(feedback);
            }
            if output_has_device_affinity {
                feedback_output_affinity_count = feedback_output_affinity_count.saturating_add(1);
            }
        }
        mappings.push(mapped);
    }
    report.converted.add(
        mappings.len(),
        "MIDI mappings",
        format!("{} verified Daslight MIDI shortcut(s)", mappings.len()),
    );
    report.approximate.add(
        device_affinity_count,
        "MIDI input device affinity",
        "Mappings were restored; select the intended MIDI input in Setup > I/O",
    );
    report.converted.add(
        custom_feedback_count,
        "MIDI feedback states",
        "Daslight Off/On/Unknown output messages, channels, control numbers, and velocity colours were restored",
    );
    report.approximate.add(
        feedback_output_affinity_count,
        "MIDI feedback output device affinity",
        "Feedback messages were restored; select the intended MIDI output in Setup > I/O",
    );
    mappings
}

fn parse_touch_surface(
    root: Node<'_, '_>,
    scene_indices: &HashMap<String, usize>,
    cues: &[CueSummary],
    profiles: &[ParsedProfile],
    fixture_refs: &HashMap<String, FixtureImportRef>,
    fixtures: &[PatchedFixtureSummary],
    report: &mut DvcImportReport,
) -> TouchSurfaceSummary {
    const TOUCH_GRID_COLUMNS: u16 = 12;
    const TOUCH_GRID_ROWS: u16 = 8;

    let group_identities = fixture_group_identities(root);
    let mut bindings_by_control = HashMap::<String, TouchControlBinding>::new();
    let mut converted_mapping_count = 0_usize;
    if let Some(shortcuts) = direct_child(root, "SHORTCUTS") {
        for (shortcut_index, shortcut) in element_children(shortcuts)
            .filter(|node| node.has_tag_name("SHORTCUT"))
            .enumerate()
        {
            let item = format!("Shortcut {}", shortcut_index + 1);
            let shortcut_type = shortcut.attribute("TYPE").unwrap_or_default();
            if shortcut_type != "2" {
                continue;
            }
            let Some(event_data) =
                direct_child(shortcut, "EVENT").and_then(|event| event.attribute("DATA"))
            else {
                report
                    .skipped
                    .add(1, item, "Touch mapping EVENT DATA was missing");
                continue;
            };
            let control_uid = event_data.split(':').next().unwrap_or_default().trim();
            if control_uid.is_empty() {
                report
                    .skipped
                    .add(1, item, "Touch mapping control UUID was empty");
                continue;
            }
            let Some(action) = direct_child(shortcut, "ACTION") else {
                report
                    .skipped
                    .add(1, item, "Touch mapping ACTION was missing");
                continue;
            };
            let action_type = action.attribute("TYPE").unwrap_or_default();
            let binding = match action_type {
                // Verified in Daslight's TOUCH MAPPINGS window as Group Select.
                "30" => action
                    .attribute("TARGET")
                    .and_then(|target| group_identities.get(target))
                    .cloned()
                    .map(|group_id| TouchControlBinding::GroupSelect { group_id }),
                // A local Daslight project stores its Touch control named
                // "Tap Tempo" as action 55.
                "55" => Some(TouchControlBinding::TapTempo),
                // Verified in Daslight's TOUCH MAPPINGS window as Scene Play.
                "107" => action
                    .attribute("TARGET")
                    .and_then(|target| scene_indices.get(target))
                    .and_then(|index| cues.get(*index))
                    .map(|cue| TouchControlBinding::Cue { cue_id: cue.id }),
                // Verified in Daslight's TOUCH MAPPINGS and DMX LEVELS windows.
                // Action 223 is a Feature Preset over an explicit BEAMS set;
                // preserve the exact fixture/attribute expansion rather than
                // widening it to every fixture that happens to say Dimmer.
                "223" => match parse_touch_feature_preset_binding(
                    shortcut,
                    action,
                    profiles,
                    fixture_refs,
                ) {
                    Ok(binding) => Some(binding),
                    Err(message) => {
                        report.unsupported.add(1, item.clone(), message);
                        None
                    }
                },
                _ => {
                    report.unsupported.add(
                        1,
                        item.clone(),
                        format!("Daslight Touch action type {action_type} is not yet mapped"),
                    );
                    None
                }
            };
            let Some(binding) = binding else {
                if matches!(action_type, "30" | "107") {
                    report.skipped.add(
                        1,
                        item,
                        format!(
                            "Daslight Touch action type {action_type} referenced a missing target"
                        ),
                    );
                }
                continue;
            };
            if let TouchControlBinding::FeaturePreset { targets, .. } = &binding {
                if !targets.iter().all(|target| {
                    fixtures.iter().any(|fixture| {
                        fixture.id == target.fixture_id
                            && fixture
                                .controls
                                .iter()
                                .any(|control| control.attribute == target.attribute)
                    })
                }) {
                    report.skipped.add(
                        1,
                        item,
                        "Daslight Feature Preset referenced a missing imported fixture attribute",
                    );
                    continue;
                }
            }
            if bindings_by_control
                .insert(control_uid.to_string(), binding)
                .is_some()
            {
                report.approximate.add(
                    1,
                    item,
                    "Multiple Daslight mappings targeted one Touch control; the last mapping wins",
                );
            }
        }
    }

    let Some(touch) = direct_child(root, "TOUCH") else {
        return TouchSurfaceSummary::default();
    };
    let mut pages = Vec::new();
    let mut converted_control_count = 0_usize;
    let mut imported_control_uids = HashSet::new();
    for (page_index, page) in element_children(touch)
        .filter(|node| node.has_tag_name("TOUCHPAGE"))
        .enumerate()
    {
        let page_id = u64::try_from(page_index + 1).unwrap_or(u64::MAX);
        let page_label =
            non_empty_label(page.attribute("NAME"), &format!("Page {}", page_index + 1));
        let mut controls = Vec::new();
        for (control_index, control) in element_children(page)
            .filter(|node| node.has_tag_name("TOUCHCONTROL"))
            .enumerate()
        {
            let item = format!("Touch: {page_label} / Control {}", control_index + 1);
            let Some(control_uid) = control.attribute("DASUID") else {
                report
                    .skipped
                    .add(1, item, "Touch control UUID was missing");
                continue;
            };
            let kind = match control.attribute("TYPE").unwrap_or_default() {
                // Verified against rendered Daslight controls and local DVC files.
                "2" => TouchControlKind::Button,
                "3" => TouchControlKind::Fader,
                control_type => {
                    report.unsupported.add(
                        1,
                        item,
                        format!("Daslight Touch control type {control_type} is not yet mapped"),
                    );
                    continue;
                }
            };
            let coordinates =
                ["GRIDPOSX", "GRIDPOSY", "GRIDWIDTH", "GRIDHEIGHT"].map(|attribute| {
                    parse_u64_attribute(control, attribute)
                        .and_then(|value| u16::try_from(value).ok())
                });
            let [Some(x), Some(y), Some(w), Some(h)] = coordinates else {
                report
                    .skipped
                    .add(1, item, "Touch control grid geometry was invalid");
                continue;
            };
            if w == 0
                || h == 0
                || x.saturating_add(w) > TOUCH_GRID_COLUMNS
                || y.saturating_add(h) > TOUCH_GRID_ROWS
            {
                report
                    .skipped
                    .add(1, item, "Touch control was outside the 12x8 grid");
                continue;
            }
            let control_id = u64::try_from(converted_control_count + 1).unwrap_or(u64::MAX);
            let binding = bindings_by_control.remove(control_uid);
            if binding.is_some() {
                converted_mapping_count = converted_mapping_count.saturating_add(1);
            }
            controls.push(TouchControlSummary {
                id: control_id,
                kind,
                x,
                y,
                w,
                h,
                label: non_empty_label(
                    control.attribute("NAME"),
                    &format!("Control {}", control_index + 1),
                ),
                binding,
            });
            imported_control_uids.insert(control_uid.to_string());
            converted_control_count = converted_control_count.saturating_add(1);
        }
        pages.push(TouchPageSummary {
            id: page_id,
            label: page_label,
            controls,
        });
    }

    for control_uid in bindings_by_control.keys() {
        if !imported_control_uids.contains(control_uid) {
            report.skipped.add(
                1,
                format!("Touch mapping: {control_uid}"),
                "Touch mapping referenced a control that was not imported",
            );
        }
    }
    report.converted.add(
        pages.len(),
        "Touch pages",
        format!("{} Daslight Touch page(s)", pages.len()),
    );
    report.converted.add(
        converted_control_count,
        "Touch controls",
        format!("{converted_control_count} positioned Touch control(s)"),
    );
    report.converted.add(
        converted_mapping_count,
        "Touch mappings",
        format!("{converted_mapping_count} verified Touch binding(s)"),
    );
    TouchSurfaceSummary { pages }
}

fn parse_scenes(
    root: Node<'_, '_>,
    profiles: &[ParsedProfile],
    fixture_refs: &HashMap<String, FixtureImportRef>,
    fixtures: &mut [PatchedFixtureSummary],
    group_colors: &mut BTreeMap<String, String>,
    report: &mut DvcImportReport,
) -> Result<(Vec<CueSummary>, HashMap<String, usize>, Vec<CueListSummary>), String> {
    let Some(scenes_section) = direct_child(root, "SCENES") else {
        return Ok((Vec::new(), HashMap::new(), Vec::new()));
    };
    let banks = element_children(scenes_section)
        .filter(|node| node.has_tag_name("BANK"))
        .collect::<Vec<_>>();
    let mut cues = Vec::new();
    let mut scene_indices = HashMap::new();
    let mut cue_lists = Vec::with_capacity(banks.len());
    let mut next_effect_id = 1_u64;

    for (bank_index, bank) in banks.iter().copied().enumerate() {
        let bank_name =
            non_empty_label(bank.attribute("NAME"), &format!("Bank {}", bank_index + 1));
        let cue_list_id = u64::try_from(bank_index + 1).unwrap_or(u64::MAX);
        cue_lists.push(CueListSummary {
            id: cue_list_id,
            label: bank_name.clone(),
            active_cue_id: None,
        });
        if let Some(color) = bank.attribute("COLOR").and_then(dvc_argb_to_rgb) {
            group_colors.insert(bank_name.clone(), color);
        }
        for (scene_index, scene) in element_children(bank)
            .filter(|node| node.has_tag_name("SCENE"))
            .enumerate()
        {
            let cue_id = u64::try_from(cues.len() + 1).unwrap_or(u64::MAX);
            let scene_uid = required_attribute(scene, "DASUID", "SCENE")?.to_string();
            let scene_name = non_empty_label(
                scene.attribute("NAME"),
                &format!("Scene {}", scene_index + 1),
            );
            let fade_in = parse_u64_attribute(scene, "FADE_IN").unwrap_or(0);
            let fade_out = parse_u64_attribute(scene, "FADE_OUT").unwrap_or(0);
            let mut notes = vec![format!(
                "Daslight bank={bank_name}; loop={}; speed={}; play_trigger={}; play_division={}; fade_out_ms={fade_out}",
                scene.attribute("LOOP").unwrap_or("0"),
                scene.attribute("SPEED").unwrap_or("1"),
                scene.attribute("PLAY_TRIGGER").unwrap_or("0"),
                scene.attribute("PLAY_DIVISION").unwrap_or("0"),
            )];
            let parsed_effects = parse_scene_effects(
                scene,
                &scene_name,
                profiles,
                fixture_refs,
                &mut next_effect_id,
                report,
            );
            notes.extend(parsed_effects.notes);
            let cue = CueSummary {
                id: cue_id,
                cue_list_id,
                cue_number: format!("{}.{}", bank_index + 1, scene_index + 1),
                label: scene_name,
                group_id: Some(bank_name.clone()),
                fade_ms: fade_in,
                notes: notes.join(" | "),
                // Daslight presents scene titles with their bank identity.
                // Leaving the Cue color empty preserves Syndocal's existing
                // cue > bank > hash priority and lets imported scenes inherit.
                color: None,
                effect_targets: parsed_effects.targets,
                ..CueSummary::default()
            };
            let cue_index = cues.len();
            cues.push(cue);
            if scene_indices.contains_key(&scene_uid) {
                report.approximate.add(
                    1,
                    format!("Scene: {}", cues[cue_index].label),
                    format!(
                        "duplicate DASUID {scene_uid} imported as a separate Cue; timeline UUID references resolve to its first occurrence"
                    ),
                );
            } else {
                scene_indices.insert(scene_uid, cue_index);
            }
        }
    }

    let mut expected_record_bytes = HashMap::<usize, usize>::new();
    let mut cue_cursor = 0_usize;
    for bank in banks.iter().copied() {
        let bank_name = non_empty_label(bank.attribute("NAME"), "Bank");
        for scene in element_children(bank).filter(|node| node.has_tag_name("SCENE")) {
            let cue_index = cue_cursor;
            cue_cursor = cue_cursor.saturating_add(1);
            let Some(fixture_datas) = direct_child(scene, "FIXTUREDATAS") else {
                continue;
            };
            let data_nodes = element_children(fixture_datas)
                .filter(|node| node.has_tag_name("FIXTUREDATA"))
                .collect::<Vec<_>>();
            if data_nodes.is_empty() {
                continue;
            }
            let declared = fixture_datas
                .attribute("NB")
                .and_then(|value| value.parse::<usize>().ok());
            let mut scene_targets = Vec::new();
            let mut touched_fixture_indices = Vec::new();
            let mut beam_records = 0_usize;
            let mut beam_checks = 0_usize;
            let mut beam_mismatches: Vec<String> = Vec::new();
            let mut beam_uninterpreted: Vec<String> = Vec::new();
            let decode_result = (|| -> Result<(), String> {
                if declared.is_some_and(|count| count != data_nodes.len()) {
                    return Err(format!(
                        "FIXTUREDATAS declares {} entries but contains {}",
                        declared.unwrap_or_default(),
                        data_nodes.len()
                    ));
                }
                for data_node in data_nodes.iter().copied() {
                    let fixture_uid = required_attribute(data_node, "FIXTURE", "FIXTUREDATA")?;
                    let fixture_ref = fixture_refs.get(fixture_uid).ok_or_else(|| {
                        format!("FIXTUREDATA references unknown fixture {fixture_uid}")
                    })?;
                    let profile = profiles
                        .get(fixture_ref.profile_index)
                        .ok_or_else(|| format!("Fixture {fixture_uid} has no profile"))?;
                    let encoded = required_attribute(data_node, "DATA", "FIXTUREDATA")?;
                    let decoded = decode_fixture_data(encoded, profile.physical_channel_count)?;
                    if let Some(expected) = expected_record_bytes.get(&fixture_ref.profile_index) {
                        if *expected != decoded.record_bytes {
                            return Err(format!(
                                "profile record rows changed from {expected} to {} bytes",
                                decoded.record_bytes
                            ));
                        }
                    } else {
                        expected_record_bytes
                            .insert(fixture_ref.profile_index, decoded.record_bytes);
                    }
                    match (&decoded.beam_features, &decoded.beam_feature_error) {
                        (Some(features), _) => {
                            beam_records = beam_records.saturating_add(features.len());
                            let (checked, mismatches) =
                                beam_feature_mismatch_details(profile, &decoded.values, features);
                            beam_checks = beam_checks.saturating_add(checked);
                            for mismatch in mismatches {
                                beam_mismatches.push(format!("Fixture {fixture_uid}: {mismatch}"));
                            }
                        }
                        (None, error) => {
                            beam_uninterpreted.push(format!(
                                "Fixture {fixture_uid}: {}",
                                error.as_deref().unwrap_or("unrecognized row variant")
                            ));
                        }
                    }
                    let values = fixture_attribute_values(
                        profile,
                        &decoded.values,
                        decoded.beam_features.as_deref(),
                    )?;
                    if !values.is_empty() {
                        scene_targets.push(CueFixtureTarget {
                            fixture_id: fixture_ref.fixture_id,
                            values,
                        });
                    }
                    touched_fixture_indices.push(fixture_ref.fixture_index);
                }
                Ok(())
            })();

            match decode_result {
                Ok(()) => {
                    cues[cue_index].targets = scene_targets;
                    for fixture_index in touched_fixture_indices {
                        if let Some(fixture) = fixtures.get_mut(fixture_index) {
                            if !fixture.group_ids.contains(&bank_name) {
                                fixture.group_ids.push(bank_name.clone());
                            }
                        }
                    }
                    report.summary.values_decoded = report.summary.values_decoded.saturating_add(1);
                    report.summary.beam_records =
                        report.summary.beam_records.saturating_add(beam_records);
                    report.summary.beam_feature_checks = report
                        .summary
                        .beam_feature_checks
                        .saturating_add(beam_checks);
                    report.summary.beam_feature_mismatches = report
                        .summary
                        .beam_feature_mismatches
                        .saturating_add(beam_mismatches.len());
                    for mismatch in beam_mismatches {
                        if report.warnings.len() < DVC_BEAM_MISMATCH_WARNING_LIMIT {
                            report.warnings.push(format!(
                                "Cue '{}' beam feature disagrees with its channel value ({mismatch})",
                                cues[cue_index].label
                            ));
                        }
                    }
                    for uninterpreted in beam_uninterpreted {
                        if report.warnings.len() < DVC_BEAM_MISMATCH_WARNING_LIMIT {
                            report.warnings.push(format!(
                                "Cue '{}' beam rows were not interpreted ({uninterpreted}); channel values were kept",
                                cues[cue_index].label
                            ));
                        }
                    }
                    report.converted.add(
                        1,
                        format!("Cue: {}", cues[cue_index].label),
                        format!("{} fixture value payload(s)", data_nodes.len()),
                    );
                }
                Err(error) => {
                    cues[cue_index].targets.clear();
                    report.summary.values_skipped = report.summary.values_skipped.saturating_add(1);
                    report.skipped.add(
                        1,
                        format!("Cue: {}", cues[cue_index].label),
                        format!("Value data not converted (format unconfirmed): {error}"),
                    );
                }
            }
        }
    }
    Ok((cues, scene_indices, cue_lists))
}

fn parse_scene_effects(
    scene: Node<'_, '_>,
    scene_name: &str,
    profiles: &[ParsedProfile],
    fixture_refs: &HashMap<String, FixtureImportRef>,
    next_effect_id: &mut u64,
    report: &mut DvcImportReport,
) -> ParsedSceneEffects {
    let mut parsed = ParsedSceneEffects::default();
    let Some(racks) = direct_child(scene, "RACKS") else {
        return parsed;
    };

    for rack in element_children(racks).filter(|node| node.has_tag_name("RACK")) {
        let rack_type = rack
            .attribute("TYPE")
            .and_then(|value| value.parse::<u16>().ok());
        for effect in element_children(rack).filter(|node| node.has_tag_name("EFFECT")) {
            let effect_type = effect
                .attribute("TYPE")
                .and_then(|value| value.parse::<u16>().ok());
            let generator_id = effect
                .attribute("ID")
                .and_then(|value| value.parse::<u16>().ok());
            let generator = match (rack_type, effect_type, generator_id) {
                (Some(3), Some(6), Some(321)) => Some("Chaser #1"),
                (Some(3), Some(6), Some(322)) => Some("Chaser #2"),
                (Some(3), Some(6), Some(323)) => Some("Chaser #3"),
                (Some(3), Some(6), Some(324)) => Some("Chaser #4"),
                (Some(3), Some(6), Some(325)) => Some("Chaser random"),
                (Some(4), Some(4), Some(221)) => Some("Circle"),
                (Some(4), Some(4), Some(223)) => Some("Line"),
                (Some(4), Some(4), Some(224)) => Some("Polygon"),
                (Some(5), Some(3), Some(21)) => Some("Bounce"),
                (Some(5), Some(3), Some(22)) => Some("Burst"),
                (Some(5), Some(3), Some(23)) => Some("Butterfly"),
                (Some(5), Some(3), Some(30)) => Some("Knight Rider"),
                (Some(5), Some(3), Some(31)) => Some("Lines"),
                (Some(5), Some(3), Some(32)) => Some("Perlin"),
                (Some(5), Some(3), Some(33)) => Some("Media"),
                (Some(5), Some(3), Some(34)) => Some("Plasma"),
                (Some(5), Some(3), Some(35)) => Some("Rain"),
                (Some(5), Some(3), Some(29)) => Some("Fire"),
                (Some(5), Some(3), Some(36)) => Some("Rainbow"),
                (Some(5), Some(3), Some(40)) => Some("Sparkle"),
                (Some(5), Some(3), Some(41)) => Some("Tube"),
                (Some(5), Some(3), Some(42)) => Some("Spiral"),
                (Some(5), Some(3), Some(44)) => Some("Sweep"),
                (Some(5), Some(3), Some(45)) => Some("Text"),
                (Some(5), Some(3), Some(47)) => Some("Explosion"),
                (Some(5), Some(3), Some(48)) => Some("Starfield"),
                (Some(5), Some(3), Some(49)) => Some("Graph"),
                (Some(5), Some(3), Some(50)) => Some("Grid"),
                (Some(8), Some(5), Some(3)) => Some("Inverse Ramp"),
                (Some(8), Some(5), Some(4)) => Some("Pulse"),
                (Some(8), Some(5), Some(5)) => Some("Ramp"),
                (Some(8), Some(5), Some(6)) => Some("Random"),
                (Some(8), Some(5), Some(7)) => Some("Sinus"),
                (Some(8), Some(5), Some(8)) => Some("Sinus3"),
                (Some(8), Some(5), Some(9)) => Some("Square"),
                (Some(8), Some(5), Some(10)) => Some("Strobe"),
                (Some(8), Some(5), Some(11)) => Some("Tangeant"),
                (Some(8), Some(5), Some(12)) => Some("Triangle"),
                (Some(8), Some(5), Some(13)) => Some("Custom"),
                (Some(2), Some(2), Some(121)) => Some("Burst"),
                (Some(2), Some(2), Some(127)) => Some("Knight Rider"),
                (Some(2), Some(2), Some(129)) => Some("Plasma"),
                (Some(2), Some(2), Some(130)) => Some("Rainbow"),
                (Some(2), Some(2), Some(131)) => Some("Random fill"),
                (Some(2), Some(2), Some(133)) => Some("Sparkle"),
                (Some(2), Some(2), Some(128)) => Some("Perlin"),
                (Some(2), Some(2), Some(134)) => Some("Sweep"),
                (Some(7), Some(7), Some(621)) => Some("Rainbow"),
                (Some(7), Some(7), Some(622)) => Some("Burst"),
                (Some(7), Some(7), Some(623)) => Some("Plasma"),
                (Some(7), Some(7), Some(624)) => Some("Knight Rider"),
                (Some(7), Some(7), Some(625)) => Some("Sweep"),
                (Some(7), Some(7), Some(626)) => Some("Sparkles"),
                (Some(7), Some(7), Some(627)) => Some("Random fill"),
                (Some(7), Some(7), Some(628)) => Some("Perlin"),
                (Some(6), Some(8), Some(521)) => Some("Rainbow"),
                (Some(6), Some(8), Some(522)) => Some("Spiral"),
                (Some(6), Some(8), Some(523)) => Some("Burst"),
                (Some(6), Some(8), Some(524)) => Some("Butterfly"),
                (Some(6), Some(8), Some(525)) => Some("Plasma"),
                (Some(6), Some(8), Some(526)) => Some("Media"),
                (Some(6), Some(8), Some(527)) => Some("Knight Rider"),
                (Some(6), Some(8), Some(528)) => Some("Sweep"),
                (Some(6), Some(8), Some(529)) => Some("Sparkle"),
                (Some(6), Some(8), Some(530)) => Some("Perlin"),
                _ => None,
            };
            let item = format!(
                "Effect: {scene_name} ({})",
                generator.unwrap_or("unconfirmed generator")
            );

            let conversion = match (rack_type, effect_type, generator_id, generator) {
                (Some(rack_type), Some(effect_type), Some(generator_id), Some(_)) => {
                    convert_dvc_effect(
                        scene,
                        scene_name,
                        rack,
                        effect,
                        rack_type,
                        effect_type,
                        generator_id,
                        *next_effect_id,
                        profiles,
                        fixture_refs,
                    )
                    .map_err(|error| {
                        format!(
                            "RACK TYPE={rack_type} EFFECT TYPE={effect_type} ID={generator_id}: {error}"
                        )
                    })
                }
                _ => Err(format!(
                    "RACK TYPE={} EFFECT TYPE={} ID={} is not confirmed for DVC-3b",
                    rack_type
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "missing".to_string()),
                    effect_type
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "missing".to_string()),
                    generator_id
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "missing".to_string())
                )),
            };

            match conversion {
                Ok(converted) => {
                    let item = format!("Effect: {scene_name} ({})", converted.generator);
                    report.summary.effects_converted =
                        report.summary.effects_converted.saturating_add(1);
                    report
                        .converted
                        .add(1, item.clone(), converted.note.clone());
                    for approximation in &converted.approximations {
                        report
                            .approximate
                            .add(1, item.clone(), approximation.clone());
                    }
                    for warning in &converted.warnings {
                        if report.warnings.len() < DVC_REPORT_DETAIL_LIMIT {
                            report.warnings.push(format!(
                                "Effect '{scene_name}' ({}): {warning}",
                                converted.generator
                            ));
                        }
                    }
                    parsed.notes.push(format!(
                        "Daslight effect {} converted: {}",
                        converted.generator, converted.note
                    ));
                    if let Some(target) = converted.target {
                        parsed.targets.push(target);
                        *next_effect_id = (*next_effect_id).saturating_add(1);
                    }
                }
                Err(error) => {
                    report.summary.effects_skipped =
                        report.summary.effects_skipped.saturating_add(1);
                    report.skipped.add(1, item, error.clone());
                    parsed
                        .notes
                        .push(format!("Daslight effect skipped: {error}"));
                }
            }
        }
    }

    parsed
}

#[allow(clippy::too_many_arguments)]
fn convert_dvc_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    rack_type: u16,
    effect_type: u16,
    generator_id: u16,
    effect_id: u64,
    profiles: &[ParsedProfile],
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    match (rack_type, effect_type, generator_id) {
        (4, 4, 221..=225) => convert_dvc_move_exact_effect(
            scene,
            scene_name,
            rack,
            effect,
            generator_id,
            effect_id,
            fixture_refs,
        ),
        (3, 6, 321 | 322 | 323 | 324 | 325) => convert_dvc_chaser_effect(
            scene,
            scene_name,
            rack,
            effect,
            generator_id,
            effect_id,
            profiles,
            fixture_refs,
        ),
        (8, 5, 3) => convert_dvc_inverse_ramp_effect(
            scene,
            scene_name,
            rack,
            effect,
            effect_id,
            fixture_refs,
        ),
        (8, 5, 4) => convert_dvc_pulse_effect(
            scene,
            scene_name,
            rack,
            effect,
            effect_id,
            fixture_refs,
        ),
        (8, 5, generator_id @ (5 | 6 | 8 | 11 | 12)) => {
            convert_dvc_additional_curve_effect(
                scene,
                scene_name,
                rack,
                effect,
                generator_id,
                effect_id,
                fixture_refs,
            )
        }
        (8, 5, 7) => convert_dvc_sinus_effect(
            scene,
            scene_name,
            rack,
            effect,
            effect_id,
            fixture_refs,
        ),
        (8, 5, 9) => convert_dvc_square_effect(
            scene,
            scene_name,
            rack,
            effect,
            effect_id,
            fixture_refs,
        ),
        (8, 5, 10) => convert_dvc_strobe_effect(
            scene,
            scene_name,
            rack,
            effect,
            effect_id,
            fixture_refs,
        ),
        (8, 5, 13) => convert_dvc_custom_curve_effect(
            scene,
            scene_name,
            rack,
            effect,
            effect_id,
            fixture_refs,
        ),
        (7, 7, generator_id @ 621..=628) => convert_dvc_value_effect(
            scene,
            scene_name,
            rack,
            effect,
            generator_id,
            effect_id,
            profiles,
            fixture_refs,
        ),
        (5, 3, 45) => convert_dvc_text_source_noop(rack, effect, fixture_refs),
        (5, 3, 21 | 22 | 23 | 29 | 30 | 31 | 32 | 33 | 34 | 35 | 36 | 37 | 40 | 41 | 42 | 44 | 47 | 48 | 49 | 50)
        | (2, 2, 121 | 127 | 128 | 129 | 130 | 131 | 133 | 134)
        | (6, 8, 521..=530) => {
            convert_dvc_color_spatial_effect(
                scene,
                scene_name,
                rack,
                effect,
                generator_id,
                effect_id,
                fixture_refs,
            )
        }
        _ => Err(format!(
            "RACK TYPE={rack_type} EFFECT TYPE={effect_type} ID={generator_id} is not confirmed for DVC-3b"
        )),
    }
}

fn convert_dvc_text_source_noop(
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    const LABEL: &str = "COLOR MAPPINGS Text ID=45";
    const EXPECTED_TYPES: &[(u16, u16)] = &[
        (1, 4),
        (2, 2),
        (3, 6),
        (4, 0),
        (10, 3),
        (11, 0),
        (12, 2),
        (13, 10),
        (14, 2),
        (15, 0),
        (16, 0),
    ];

    require_exact_custom_attributes(rack, &["EXPAND_RACK", "TYPE"], &format!("{LABEL} RACK"))?;
    if rack.attribute("EXPAND_RACK") != Some("1") {
        return Err(format!(
            "{LABEL} RACK EXPAND_RACK must be 1, found {}",
            rack.attribute("EXPAND_RACK").unwrap_or("missing")
        ));
    }
    if rack.attribute("TYPE") != Some("5") {
        return Err(format!(
            "{LABEL} RACK TYPE must be 5, found {}",
            rack.attribute("TYPE").unwrap_or("missing")
        ));
    }
    require_exact_custom_attributes(
        effect,
        &["DURATION", "ID", "TYPE"],
        &format!("{LABEL} EFFECT"),
    )?;
    if effect.attribute("TYPE") != Some("3") || effect.attribute("ID") != Some("45") {
        return Err(format!(
            "{LABEL} EFFECT must use TYPE=3 ID=45, found TYPE={} ID={}",
            effect.attribute("TYPE").unwrap_or("missing"),
            effect.attribute("ID").unwrap_or("missing")
        ));
    }

    let selections = element_children(rack)
        .filter(|node| node.has_tag_name("SELECTIONS"))
        .count();
    if selections != 0 {
        return Err(format!(
            "{LABEL} external SELECTIONS remain fail-closed; found {selections} SELECTIONS container(s)"
        ));
    }
    let rack_children = element_children(rack).collect::<Vec<_>>();
    for (name, expected_count) in [("EFFECT", 1_usize), ("MAPPING", 1), ("BEAMS", 1)] {
        let actual_count = rack_children
            .iter()
            .filter(|node| node.has_tag_name(name))
            .count();
        if actual_count != expected_count {
            return Err(format!(
                "{LABEL} expected exactly one direct {name} container, found {actual_count}"
            ));
        }
    }
    if let Some(unexpected) = rack_children.iter().find(|node| {
        !node.has_tag_name("EFFECT") && !node.has_tag_name("MAPPING") && !node.has_tag_name("BEAMS")
    }) {
        return Err(format!(
            "{LABEL} RACK contains unexpected <{}> element",
            unexpected.tag_name().name()
        ));
    }

    let effect_children = element_children(effect).collect::<Vec<_>>();
    if effect_children.len() != 1 || !effect_children[0].has_tag_name("PARAMS") {
        return Err(format!(
            "{LABEL} EFFECT must contain exactly one direct PARAMS element"
        ));
    }
    let params_node = effect_children[0];
    require_exact_custom_attributes(params_node, &["NB"], &format!("{LABEL} PARAMS"))?;
    let declared = required_attribute(params_node, "NB", "PARAMS")?
        .parse::<usize>()
        .map_err(|error| format!("{LABEL} PARAMS NB is invalid: {error}"))?;
    let param_nodes = element_children(params_node).collect::<Vec<_>>();
    if let Some(unexpected) = param_nodes.iter().find(|node| !node.has_tag_name("PARAM")) {
        return Err(format!(
            "{LABEL} PARAMS contains unexpected <{}> element",
            unexpected.tag_name().name()
        ));
    }
    if declared != EXPECTED_TYPES.len() || param_nodes.len() != EXPECTED_TYPES.len() {
        return Err(format!(
            "{LABEL} expected PARAMS NB={} with {} PARAM elements, found NB={declared} with {}",
            EXPECTED_TYPES.len(),
            EXPECTED_TYPES.len(),
            param_nodes.len()
        ));
    }

    let mut params = HashMap::<u16, Node<'_, '_>>::new();
    for param in param_nodes {
        let id = required_attribute(param, "ID", "PARAM")?
            .parse::<u16>()
            .map_err(|error| format!("{LABEL} PARAM ID is invalid: {error}"))?;
        if params.insert(id, param).is_some() {
            return Err(format!("{LABEL} PARAM {id} is duplicated"));
        }
    }
    let mut actual_ids = params.keys().copied().collect::<Vec<_>>();
    actual_ids.sort_unstable();
    let mut expected_ids = EXPECTED_TYPES.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    expected_ids.sort_unstable();
    if actual_ids != expected_ids {
        return Err(format!(
            "{LABEL} expected PARAM IDs {expected_ids:?}, found {actual_ids:?}"
        ));
    }
    require_exact_dvc_param_types(effect, EXPECTED_TYPES)?;

    for (&id, param) in &params {
        let expected_attributes: &[&str] = if id == 1 {
            &["ID", "TYPE"]
        } else {
            &["ID", "TYPE", "VAL"]
        };
        require_exact_custom_attributes(
            *param,
            expected_attributes,
            &format!("{LABEL} PARAM {id}"),
        )?;
    }

    let palette_param = params[&1];
    require_strict_dvc_palette_shape(palette_param)?;
    let colors = direct_child(palette_param, "COLORS")
        .expect("strict palette shape established one COLORS child");
    require_exact_custom_attributes(colors, &["NB"], &format!("{LABEL} COLORS"))?;
    for (index, color) in element_children(colors).enumerate() {
        require_exact_custom_attributes(color, &["VAL"], &format!("{LABEL} COLOR {index}"))?;
    }
    let palette = dvc_color_palette_stops(palette_param)?;
    if !(2..=protocol::DASLIGHT_COLOR_PALETTE_MAX_STOPS).contains(&palette.len()) {
        return Err(format!(
            "{LABEL} palette requires 2..255 colors, found {}",
            palette.len()
        ));
    }

    let text = params[&10]
        .attribute("VAL")
        .ok_or_else(|| format!("{LABEL} Text PARAM 10 is missing VAL"))?;
    let numeric = |id: u16, parameter: &str| -> Result<f64, String> {
        let raw = params[&id]
            .attribute("VAL")
            .ok_or_else(|| format!("{LABEL} {parameter} PARAM {id} is missing VAL"))?;
        let value = raw
            .parse::<f64>()
            .map_err(|error| format!("{LABEL} {parameter} PARAM {id} is invalid: {error}"))?;
        if !value.is_finite() {
            return Err(format!(
                "{LABEL} {parameter} PARAM {id} must be finite, found {value}"
            ));
        }
        Ok(value)
    };
    let integer_range = |id: u16, parameter: &str, low: i64, high: i64| -> Result<i64, String> {
        let value = numeric(id, parameter)?;
        if value.fract().abs() > f64::EPSILON || value < low as f64 || value > high as f64 {
            return Err(format!(
                "{LABEL} {parameter} PARAM {id} must be an integer within {low}..{high}, found {value}"
            ));
        }
        Ok(value as i64)
    };
    let binary = |id: u16, parameter: &str| -> Result<bool, String> {
        match integer_range(id, parameter, 0, 1)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => unreachable!(),
        }
    };

    let grayscale = binary(2, "Grayscale")?;
    let transform = integer_range(3, "Transform", 0, 2)?;
    let rotation = integer_range(4, "Rotation", 0, 360)?;
    let size = integer_range(11, "Size", 5, 80)?;
    let anti_alias = binary(12, "Anti Alias")?;
    let direction = integer_range(13, "Direction", 0, 8)?;
    let write_vertically = binary(14, "Write Vertically")?;
    let vertical_offset = integer_range(15, "Vertical Offset", -100, 100)?;
    let horizontal_offset = integer_range(16, "Horizontal Offset", -100, 100)?;
    let (period_ms, period_note) = dvc_exact_generator_period(effect, "COLOR MAPPINGS", "Text")?;

    let rectangle = dvc_mapping_rectangle(rack, LABEL, true)?;
    let beams = direct_child(rack, "BEAMS").expect("strict rack shape established one BEAMS child");
    require_exact_custom_attributes(beams, &["NB"], &format!("{LABEL} BEAMS"))?;
    let beam_children = element_children(beams).collect::<Vec<_>>();
    if let Some(unexpected) = beam_children.iter().find(|node| !node.has_tag_name("BEAM")) {
        return Err(format!(
            "{LABEL} BEAMS contains unexpected <{}> element",
            unexpected.tag_name().name()
        ));
    }
    let declared_beams = required_attribute(beams, "NB", "BEAMS")?
        .parse::<usize>()
        .map_err(|error| format!("{LABEL} BEAMS NB is invalid: {error}"))?;
    if declared_beams != beam_children.len() {
        return Err(format!(
            "{LABEL} BEAMS declares {declared_beams} entries but contains {}",
            beam_children.len()
        ));
    }
    for (index, beam) in beam_children.iter().enumerate() {
        require_exact_custom_attributes(
            *beam,
            &["BEAMID", "FIXTURE", "IDSELECTION"],
            &format!("{LABEL} BEAM {index}"),
        )?;
    }
    let targets = dvc_rack_targets(rack, fixture_refs)?;
    if rectangle.native_empty_sentinel && !targets.beam_targets.is_empty() {
        return Err(format!(
            "{LABEL} native empty Rectangle sentinel is valid only with zero owned BEAMS"
        ));
    }
    if !targets.beam_targets.is_empty() {
        return Err(format!(
            "{LABEL} populated Text requires unresolved deterministic font and supported text-raster envelope"
        ));
    }

    Ok(ConvertedDvcEffect {
        target: None,
        generator: "Text",
        note: format!(
            "source_family=Color Mappings; implementation=SourceExactBoundary; evaluator=CTextEffect/0x140366940; source no-op preserved: Daslight BEAMS contains zero owned targets and no external SELECTIONS; palette_colors={}; Grayscale={}; Transform={transform}; Rotation={rotation}; TextEmpty={}; TextUtf8Bytes={}; Size={size}; Anti Alias={}; Direction={direction}; Write Vertically={}; Vertical Offset={vertical_offset}; Horizontal Offset={horizontal_offset}; Rectangle=({},{},{},{},{}); period_ms={period_ms}; {period_note}; no runtime effect was created",
            palette.len(),
            u8::from(grayscale),
            u8::from(text.is_empty()),
            text.len(),
            u8::from(anti_alias),
            u8::from(write_vertically),
            rectangle.x,
            rectangle.y,
            rectangle.sx,
            rectangle.sy,
            rectangle.angle_degrees,
        ),
        approximations: Vec::new(),
        warnings: Vec::new(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DvcMappingRectangle {
    x: i64,
    y: i64,
    sx: i64,
    sy: i64,
    angle_degrees: f32,
    /// Daslight serializes a not-yet-authored COLOR MAPPINGS Rectangle as
    /// `(0,0,-1,-1,0,LOCKED=0)`. It is accepted only for source-noop racks
    /// and never reaches a runtime placement.
    native_empty_sentinel: bool,
}

fn require_strict_dvc_bounce_structure(
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
) -> Result<(), String> {
    const LABEL: &str = "COLOR MAPPINGS Bounce ID=21";
    require_exact_custom_attributes(rack, &["EXPAND_RACK", "TYPE"], &format!("{LABEL} RACK"))?;
    if rack.attribute("EXPAND_RACK") != Some("1") || rack.attribute("TYPE") != Some("5") {
        return Err(format!(
            "{LABEL} RACK requires EXPAND_RACK=1 TYPE=5, found EXPAND_RACK={} TYPE={}",
            rack.attribute("EXPAND_RACK").unwrap_or("missing"),
            rack.attribute("TYPE").unwrap_or("missing")
        ));
    }
    require_exact_custom_attributes(
        effect,
        &["DURATION", "ID", "TYPE"],
        &format!("{LABEL} EFFECT"),
    )?;
    let rack_children = element_children(rack).collect::<Vec<_>>();
    for name in ["EFFECT", "MAPPING", "BEAMS"] {
        let count = rack_children
            .iter()
            .filter(|node| node.has_tag_name(name))
            .count();
        if count != 1 {
            return Err(format!(
                "{LABEL} expected exactly one direct {name} container, found {count}"
            ));
        }
    }
    if let Some(unexpected) = rack_children.iter().find(|node| {
        !node.has_tag_name("EFFECT") && !node.has_tag_name("MAPPING") && !node.has_tag_name("BEAMS")
    }) {
        return Err(format!(
            "{LABEL} RACK contains unexpected <{}> element",
            unexpected.tag_name().name()
        ));
    }
    let effect_children = element_children(effect).collect::<Vec<_>>();
    if effect_children.len() != 1 || !effect_children[0].has_tag_name("PARAMS") {
        return Err(format!(
            "{LABEL} EFFECT must contain exactly one direct PARAMS element"
        ));
    }
    let params = effect_children[0];
    require_exact_custom_attributes(params, &["NB"], &format!("{LABEL} PARAMS"))?;
    for param in element_children(params) {
        let id = required_attribute(param, "ID", "PARAM")?;
        let expected: &[&str] = if id == "1" {
            &["ID", "TYPE"]
        } else {
            &["ID", "TYPE", "VAL"]
        };
        require_exact_custom_attributes(param, expected, &format!("{LABEL} PARAM {id}"))?;
        if id == "1" {
            let colors = direct_child(param, "COLORS")
                .ok_or_else(|| format!("{LABEL} palette PARAM 1 is missing COLORS"))?;
            require_exact_custom_attributes(colors, &["NB"], &format!("{LABEL} COLORS"))?;
            for (index, color) in element_children(colors).enumerate() {
                require_exact_custom_attributes(
                    color,
                    &["VAL"],
                    &format!("{LABEL} COLOR {index}"),
                )?;
            }
        }
    }
    let beams = direct_child(rack, "BEAMS").expect("strict Bounce rack established BEAMS");
    require_exact_custom_attributes(beams, &["NB"], &format!("{LABEL} BEAMS"))?;
    let beam_children = element_children(beams).collect::<Vec<_>>();
    if let Some(unexpected) = beam_children.iter().find(|node| !node.has_tag_name("BEAM")) {
        return Err(format!(
            "{LABEL} BEAMS contains unexpected <{}> element",
            unexpected.tag_name().name()
        ));
    }
    let declared = required_attribute(beams, "NB", "BEAMS")?
        .parse::<usize>()
        .map_err(|error| format!("{LABEL} BEAMS NB is invalid: {error}"))?;
    if declared != beam_children.len() {
        return Err(format!(
            "{LABEL} BEAMS declares {declared} entries but contains {}",
            beam_children.len()
        ));
    }
    for (index, beam) in beam_children.iter().enumerate() {
        require_exact_custom_attributes(
            *beam,
            &["BEAMID", "FIXTURE", "IDSELECTION"],
            &format!("{LABEL} BEAM {index}"),
        )?;
    }
    Ok(())
}

fn dvc_mapping_rectangle(
    rack: Node<'_, '_>,
    generator: &str,
    require_exact_attributes: bool,
) -> Result<DvcMappingRectangle, String> {
    let mappings = element_children(rack)
        .filter(|node| node.has_tag_name("MAPPING"))
        .collect::<Vec<_>>();
    if mappings.len() != 1 {
        return Err(format!(
            "{generator} expected exactly one MAPPING, found {}",
            mappings.len()
        ));
    }
    let mapping = mappings[0];
    if require_exact_attributes {
        let mut actual_attributes = mapping
            .attributes()
            .map(|attribute| attribute.name())
            .collect::<Vec<_>>();
        actual_attributes.sort_unstable();
        let mut expected_attributes = vec![
            "ANGLE", "DASUID", "LOCKED", "NAME", "SX", "SY", "TYPE", "X", "Y",
        ];
        expected_attributes.sort_unstable();
        if actual_attributes != expected_attributes {
            return Err(format!(
                "{generator} expected Rectangle attributes {expected_attributes:?}, found {actual_attributes:?}"
            ));
        }
    }
    if required_attribute(mapping, "NAME", "MAPPING")? != "Rectangle" {
        return Err(format!(
            "{generator} expected MAPPING NAME=Rectangle, found {}",
            mapping.attribute("NAME").unwrap_or("missing")
        ));
    }
    let dasuid = required_attribute(mapping, "DASUID", "MAPPING Rectangle")?;
    if dasuid.trim().is_empty() {
        return Err(format!("{generator} Rectangle DASUID must not be empty"));
    }
    if required_attribute(mapping, "TYPE", "MAPPING Rectangle")? != "0" {
        return Err(format!(
            "{generator} expected Rectangle TYPE=0, found {}",
            mapping.attribute("TYPE").unwrap_or("missing")
        ));
    }
    let parse_integer = |attribute: &str| -> Result<i64, String> {
        required_attribute(mapping, attribute, "MAPPING Rectangle")?
            .parse::<i64>()
            .map_err(|error| format!("{generator} Rectangle {attribute} is invalid: {error}"))
    };
    let x = parse_integer("X")?;
    let y = parse_integer("Y")?;
    let sx = parse_integer("SX")?;
    let sy = parse_integer("SY")?;
    let angle_degrees = required_attribute(mapping, "ANGLE", "MAPPING Rectangle")?
        .parse::<f32>()
        .map_err(|error| format!("{generator} Rectangle ANGLE is invalid: {error}"))?;
    if !angle_degrees.is_finite() {
        return Err(format!(
            "{generator} Rectangle ANGLE must be finite, found {angle_degrees}"
        ));
    }
    let locked = required_attribute(mapping, "LOCKED", "MAPPING Rectangle")?;
    match locked {
        "0" | "1" => {}
        value => {
            return Err(format!(
                "{generator} Rectangle LOCKED must be 0 or 1, found {value}"
            ))
        }
    }
    let native_empty_sentinel =
        x == 0 && y == 0 && sx == -1 && sy == -1 && angle_degrees == 0.0 && locked == "0";
    if (sx <= 0 || sy <= 0) && !native_empty_sentinel {
        return Err(format!(
            "{generator} Rectangle SX and SY must be positive, found {sx} and {sy}"
        ));
    }
    Ok(DvcMappingRectangle {
        x,
        y,
        sx,
        sy,
        angle_degrees,
        native_empty_sentinel,
    })
}

fn dvc_color_spatial_placement(
    rectangle: DvcMappingRectangle,
    generator: &str,
    beam_targets: &[ColorEffectBeamTarget],
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ColorEffectSpatialPlacement, String> {
    if rectangle.native_empty_sentinel {
        return Err(format!(
            "{generator} native empty Rectangle sentinel is valid only with zero owned BEAMS"
        ));
    }
    let fixture_refs_by_id = fixture_refs
        .values()
        .map(|fixture_ref| (fixture_ref.fixture_id, fixture_ref))
        .collect::<HashMap<_, _>>();
    let mut seen_targets = HashSet::new();
    let mut target_coordinates = Vec::new();
    for target in beam_targets {
        if !seen_targets.insert((target.fixture_id, target.beam_index)) {
            continue;
        }
        let fixture_ref = fixture_refs_by_id.get(&target.fixture_id).ok_or_else(|| {
            format!(
                "{generator} target fixture {} is missing its imported Patch identity",
                target.fixture_id
            )
        })?;
        let point = fixture_ref
            .patch_beam_positions
            .get(&target.beam_index)
            .ok_or_else(|| {
                format!(
                    "{generator} target fixture {} beam {} has no exact integer Patch-canvas POSX/POSY",
                    target.fixture_id, target.beam_index
                )
            })?;
        target_coordinates.push(ColorEffectSpatialPlacementTarget {
            fixture_id: target.fixture_id,
            beam_index: target.beam_index,
            patch_x: point.x,
            patch_y: point.y,
        });
    }
    Ok(ColorEffectSpatialPlacement {
        source_coordinate_frame: ColorEffectSpatialCoordinateFrame::DaslightPatchCanvas,
        mapping_shape: ColorEffectSpatialMappingShape::Rectangle,
        x: rectangle.x,
        y: rectangle.y,
        sx: rectangle.sx,
        sy: rectangle.sy,
        mapping_angle_degrees: rectangle.angle_degrees,
        sampling_rule: ColorEffectSpatialSamplingRule::RotatedInclusionMaskAxisAlignedRaster,
        vertical_symmetry: false,
        horizontal_symmetry: false,
        raster_rotation_degrees: 0.0,
        target_coordinates,
    })
}

fn require_exact_dvc_mapping_recipe_schema(
    effect: Node<'_, '_>,
    params: &HashMap<u16, f64>,
    color_mappings: bool,
    recipe_types: &[(u16, u16)],
) -> Result<(), String> {
    let mut expected_ids = Vec::with_capacity(recipe_types.len() + 3);
    if color_mappings {
        expected_ids.push(2);
    }
    expected_ids.extend([3, 4]);
    expected_ids.extend(recipe_types.iter().map(|(id, _)| *id));
    require_exact_dvc_params(params, &expected_ids)?;

    let mut expected_types = Vec::with_capacity(recipe_types.len() + 4);
    expected_types.push((1, 4));
    if color_mappings {
        expected_types.push((2, 2));
    }
    expected_types.extend([(3, 6), (4, 0)]);
    expected_types.extend_from_slice(recipe_types);
    let params_node = direct_child(effect, "PARAMS")
        .ok_or_else(|| "confirmed mapping generator is missing PARAMS".to_string())?;
    let param_nodes = element_children(params_node).collect::<Vec<_>>();
    if let Some(unexpected) = param_nodes.iter().find(|node| !node.has_tag_name("PARAM")) {
        return Err(format!(
            "confirmed mapping generator PARAMS contains unexpected <{}> element",
            unexpected.tag_name().name()
        ));
    }
    let declared = required_attribute(params_node, "NB", "PARAMS")?
        .parse::<usize>()
        .map_err(|error| format!("PARAMS NB is invalid: {error}"))?;
    if declared != expected_types.len() || param_nodes.len() != expected_types.len() {
        return Err(format!(
            "confirmed mapping generator expected PARAMS NB={} with {} PARAM elements, found NB={declared} with {}",
            expected_types.len(),
            expected_types.len(),
            param_nodes.len()
        ));
    }
    require_exact_dvc_param_types(effect, &expected_types)?;
    let palette = param_nodes
        .iter()
        .find(|param| param.attribute("ID") == Some("1"))
        .ok_or_else(|| "confirmed mapping generator is missing palette PARAM 1".to_string())?;
    require_strict_dvc_palette_shape(*palette)
}

fn require_strict_dvc_palette_shape(param: Node<'_, '_>) -> Result<(), String> {
    let palette_children = element_children(param).collect::<Vec<_>>();
    if palette_children.len() != 1 || !palette_children[0].has_tag_name("COLORS") {
        return Err("palette PARAM 1 must contain exactly one direct COLORS element".to_string());
    }
    let colors = palette_children[0];
    let color_nodes = element_children(colors).collect::<Vec<_>>();
    if let Some(unexpected) = color_nodes.iter().find(|node| !node.has_tag_name("COLOR")) {
        return Err(format!(
            "palette COLORS contains unexpected <{}> element",
            unexpected.tag_name().name()
        ));
    }
    let declared = required_attribute(colors, "NB", "COLORS")?
        .parse::<usize>()
        .map_err(|error| format!("COLORS NB is invalid: {error}"))?;
    if declared != color_nodes.len() {
        return Err(format!(
            "COLORS declares {declared} entries but contains {}",
            color_nodes.len()
        ));
    }
    Ok(())
}

fn convert_dvc_move_exact_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    generator_id: u16,
    effect_id: u64,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    let (generator, interpolation, closed) = match generator_id {
        221 => ("Circle", MoveInterpolation::DaslightCircle, true),
        222 => ("Curve", MoveInterpolation::DaslightCurve, false),
        223 => ("Line", MoveInterpolation::DaslightLine, false),
        224 => ("Polygon", MoveInterpolation::DaslightPolygon, true),
        225 => ("Points", MoveInterpolation::DaslightPoints, false),
        _ => return Err(format!("unknown Move generator ID={generator_id}")),
    };
    let (points, phasing, symmetry) = dvc_move_effect_params(effect, generator)?;
    let targets = dvc_rack_targets(rack, fixture_refs)?;
    if targets.beam_targets.is_empty() {
        return Err(format!(
            "Move {generator} ID={generator_id} BEAMS resolved to no targets"
        ));
    }
    let coordinate_mode = match scene.attribute("ATTRIBUTEVALUE_MODE") {
        Some("0") => MoveCoordinateMode::Absolute,
        Some("1") => MoveCoordinateMode::Relative,
        Some(value) => {
            return Err(format!(
                "Move {generator} ID={generator_id} ATTRIBUTEVALUE_MODE must be 0 (Absolute) or 1 (Relative), found {value}"
            ));
        }
        None => {
            return Err(format!(
                "Move {generator} ID={generator_id} owning scene is missing ATTRIBUTEVALUE_MODE"
            ));
        }
    };
    let (period_ms, free_run_note) = dvc_exact_generator_period(effect, "MOVE FX", generator)?;
    let (clock_sync, clock_note, clock_warning) = dvc_scene_clock_sync(scene);
    let mut approximations = Vec::new();
    if let Some(clock_warning) = clock_warning {
        approximations.push(clock_warning);
    }
    let beam_count = targets.beam_targets.len();
    let selection_count = targets.ordered_steps.len();
    let beam_targets = targets
        .beam_targets
        .into_iter()
        .map(|target| MoveEffectBeamTarget {
            fixture_id: target.fixture_id,
            beam_index: target.beam_index,
            selection_index: target.selection_index,
        })
        .collect();
    let point_count = points.len();
    let request = MoveEffectRequest {
        label: format!("{scene_name} ({generator})"),
        fixture_ids: targets.fixture_ids,
        target_group_ids: Vec::new(),
        beam_targets,
        points,
        closed,
        interpolation,
        coordinate_mode,
        center_x: 0.5,
        center_y: 0.5,
        size_x: 1.0,
        size_y: 1.0,
        rotation_degrees: 0.0,
        period_ms,
        clock_sync,
        direction: MoveDirection::Forward,
        phase: 0.0,
        fixture_spread: phasing,
        symmetry,
        blend_mode: EffectBlendMode::Override,
    };
    engine::validate_move_effect_request(&request).map_err(|error| {
        format!("confirmed Move {generator} parameters are not representable: {error}")
    })?;
    Ok(ConvertedDvcEffect {
        target: Some(CueEffectTarget {
            effect_id,
            enabled: true,
            params: Some(EffectParamsSnapshot::Move(request)),
            transition_ms: None,
        }),
        generator,
        note: format!(
            "Daslight evaluator=ID {generator_id} {generator}; recovered_frame_quantum_ms=40; implementation=SyndocalCorrected; runtime_time=continuous; points={point_count}; beam_targets={beam_count}; selections={selection_count}; coordinate_mode={coordinate_mode:?}; raw_phasing={phasing}; symmetry={}; {free_run_note}; {clock_note}",
            u8::from(symmetry)
        ),
        approximations,
        warnings: Vec::new(),
    })
}

fn dvc_value_palette_points(palette: &[ColorEffectStop]) -> Result<Vec<ValueEffectPoint>, String> {
    if !(2..=32).contains(&palette.len()) {
        return Err(format!(
            "VALUE FX value palette requires 2..32 stops, found {}",
            palette.len()
        ));
    }
    palette
        .iter()
        .enumerate()
        .map(|(index, stop)| {
            let color = stop.color;
            if color.red != color.green || color.red != color.blue {
                return Err(format!(
                    "VALUE FX palette stop {} is not a Black..White value: {}/{}/{}",
                    index + 1,
                    color.red,
                    color.green,
                    color.blue
                ));
            }
            Ok(ValueEffectPoint {
                position: stop.position,
                value: color.red as f32 / u16::MAX as f32,
            })
        })
        .collect()
}

fn dvc_corrected_random_evaluator_note(
    recipe: &ColorEffectSpatialRecipe,
    generator_id: u16,
) -> Option<String> {
    match (recipe, generator_id) {
        (
            ColorEffectSpatialRecipe::Sparkle {
                rng_seed,
                number,
                lifetime_ms,
                source_lifespan,
                width,
                height,
                ..
            },
            626 | 133,
        ) => Some(format!(
            "evaluator=CSparklesEffect recovered retained-particle grammar; implementation=SyndocalCorrected; unavailable_qrand=stable_source_seed; rng_seed={rng_seed}; Number={number}; source_LifeSpan={source_lifespan:?}; lifetime_ms={lifetime_ms:?}; Width={width}; Height={height:?}; population=retained; spawn_rate_units=particles_per_40ms; fade=continuous; time_units=milliseconds"
        )),
        (
            ColorEffectSpatialRecipe::RandomFill {
                rng_seed,
                point_width,
                source_point_height,
                ..
            },
            627 | 131,
        ) => Some(format!(
            "evaluator=CRandomFillEffect recovered no-replacement grammar; implementation=SyndocalCorrected; unavailable_qrand=stable_source_seed; rng_seed={rng_seed}; PointWidth={point_width}; source_PointHeight={source_point_height:?}; source_PointHeight_evaluator_dead=true; cell_partition=div_ceil; end_coverage=partial_tail_included; transition=continuous_palette_to_palette"
        )),
        _ => None,
    }
}

fn normalize_dvc_spatial_recipe(
    recipe: &mut ColorEffectSpatialRecipe,
    strip_count: usize,
    mapping_raster: bool,
) {
    let strip_count = strip_count.max(1) as f32;
    let source_width = if mapping_raster { 100.0 } else { strip_count };
    match recipe {
        ColorEffectSpatialRecipe::KnightRider { size, .. } => {
            *size = *size * 100.0 / source_width;
        }
        ColorEffectSpatialRecipe::Burst {
            color_width,
            gradient,
            ..
        } => {
            *color_width = *color_width * 100.0 / source_width;
            *gradient *= 100.0;
        }
        ColorEffectSpatialRecipe::RandomFill { point_width, .. } => {
            *point_width = *point_width * 100.0 / source_width;
        }
        ColorEffectSpatialRecipe::Sparkle { width, height, .. } => {
            *width = *width * 100.0 / source_width;
            if let Some(height) = height {
                *height = *height * 100.0 / if mapping_raster { 100.0 } else { 1.0 };
            }
        }
        ColorEffectSpatialRecipe::Perlin {
            octaves,
            zoom,
            direction_degrees,
            ..
        } => {
            *octaves = octaves.saturating_sub(1);
            let source_span = if mapping_raster {
                99.0
            } else {
                (strip_count - 1.0).max(1.0)
            };
            *zoom /= source_span;
            *direction_degrees = (*direction_degrees - 1.0) * 360.0 / 99.0;
        }
        _ => {}
    }
}

fn convert_dvc_value_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    generator_id: u16,
    effect_id: u64,
    profiles: &[ParsedProfile],
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    let (params, palette) = dvc_color_palette_and_params(effect)?;
    let (generator, expected_ids, expected_types) = match generator_id {
        621 => (
            "Rainbow",
            &[3, 10, 11, 12][..],
            &[(1, 4), (3, 6), (10, 1), (11, 0), (12, 1)][..],
        ),
        622 => (
            "Burst",
            &[3, 10, 11][..],
            &[(1, 4), (3, 6), (10, 0), (11, 1)][..],
        ),
        623 => (
            "Plasma",
            &[3, 10, 11, 12, 13, 14, 15, 16, 17][..],
            &[
                (1, 4),
                (3, 6),
                (10, 0),
                (11, 0),
                (12, 0),
                (13, 0),
                (14, 0),
                (15, 0),
                (16, 0),
                (17, 0),
            ][..],
        ),
        624 => (
            "Knight Rider",
            &[3, 10, 11, 12, 13, 14][..],
            &[(1, 4), (3, 6), (10, 0), (11, 2), (12, 2), (13, 2), (14, 0)][..],
        ),
        625 => ("Sweep", &[3, 10][..], &[(1, 4), (3, 6), (10, 2)][..]),
        626 => (
            "Sparkles",
            &[3, 10, 11, 12][..],
            &[(1, 4), (3, 6), (10, 0), (11, 1), (12, 0)][..],
        ),
        627 => (
            "Random fill",
            &[3, 10, 11][..],
            &[(1, 4), (3, 6), (10, 0), (11, 0)][..],
        ),
        628 => (
            "Perlin",
            &[3, 10, 11, 12, 13, 14][..],
            &[(1, 4), (3, 6), (10, 0), (11, 0), (12, 0), (13, 0), (14, 0)][..],
        ),
        _ => {
            return Err(format!(
                "VALUE FX generator {generator_id} is not confirmed"
            ))
        }
    };
    require_exact_dvc_params(&params, expected_ids)?;
    require_exact_dvc_param_types(effect, expected_types)?;
    let mut recipe = match generator_id {
        621 => {
            let transform = dvc_param(&params, 3, "VALUE FX Rainbow Transform")?;
            if !matches!(transform, 0.0 | 1.0) {
                return Err(format!(
                    "VALUE FX Rainbow Transform PARAM 3 must be None(0) or Vertical symmetry(1), found {transform}"
                ));
            }
            let angle_degrees = dvc_finite_param(&params, 11, "VALUE FX Rainbow Angle")?;
            if angle_degrees.fract().abs() > f32::EPSILON {
                return Err(format!(
                    "VALUE FX Rainbow Angle PARAM 11 must be an integer, found {angle_degrees}"
                ));
            }
            ColorEffectSpatialRecipe::ColorRainbow {
                grayscale: false,
                vertical_symmetry: transform == 1.0,
                color_width: dvc_finite_param(&params, 10, "VALUE FX Color Width")?,
                angle_degrees,
                gradient: dvc_unit_param(&params, 12, "VALUE FX Gradient")? * 100.0,
            }
        }
        622 => ColorEffectSpatialRecipe::Burst {
            grayscale: false,
            vertical_symmetry: dvc_binary_param(&params, 3, "VALUE FX Burst Transform")?,
            color_width: dvc_integer_range_param(
                &params,
                10,
                "VALUE FX Burst Color Width",
                10,
                900,
            )?,
            gradient: dvc_finite_range_param(&params, 11, "VALUE FX Burst Gradient", 0.0, 1.0)?,
        },
        623 => ColorEffectSpatialRecipe::Plasma {
            grayscale: false,
            vertical_symmetry: dvc_binary_param(&params, 3, "VALUE FX Plasma Transform")?,
            size_x: dvc_integer_range_param(&params, 10, "VALUE FX Plasma Size X", 0, 20)?,
            param_x: dvc_integer_range_param(&params, 11, "VALUE FX Plasma Param X", 0, 20)?,
            size_y: dvc_integer_range_param(&params, 12, "VALUE FX Plasma Size Y", 0, 20)?,
            param_y: dvc_integer_range_param(&params, 13, "VALUE FX Plasma Param Y", 0, 20)?,
            speed_x: dvc_integer_range_param(&params, 14, "VALUE FX Plasma Speed X", -5, 5)?,
            param_sx: dvc_integer_range_param(&params, 15, "VALUE FX Plasma Param SX", -5, 5)?,
            speed_y: dvc_integer_range_param(&params, 16, "VALUE FX Plasma Speed Y", -5, 5)?,
            param_sy: dvc_integer_range_param(&params, 17, "VALUE FX Plasma Param SY", -5, 5)?,
        },
        624 => {
            let vertical_symmetry =
                dvc_binary_param(&params, 3, "VALUE FX Knight Rider Transform")?;
            ColorEffectSpatialRecipe::KnightRider {
                grayscale: false,
                vertical_symmetry,
                size: dvc_integer_range_param(&params, 10, "VALUE FX Knight Rider Size", 1, 100)?,
                one_way: dvc_binary_param(&params, 11, "VALUE FX Knight Rider One Way Only")?,
                fading: dvc_binary_param(&params, 12, "VALUE FX Knight Rider Fading")?,
                go_outside: dvc_binary_param(&params, 13, "VALUE FX Knight Rider Go Outside")?,
                gradient: dvc_integer_range_param(
                    &params,
                    14,
                    "VALUE FX Knight Rider Gradient",
                    0,
                    100,
                )?,
            }
        }
        625 => ColorEffectSpatialRecipe::Sweep {
            grayscale: false,
            vertical_symmetry: dvc_binary_param(&params, 3, "VALUE FX Sweep Transform")?,
            direction_change: dvc_binary_param(&params, 10, "VALUE FX Sweep Direction Change")?,
        },
        626 => {
            let vertical_symmetry = dvc_binary_param(&params, 3, "VALUE FX Sparkles Transform")?;
            let number =
                dvc_integer_range_param(&params, 10, "VALUE FX Sparkles Number", 1, 10)? as u16;
            let source_lifespan =
                dvc_finite_range_param(&params, 11, "VALUE FX Sparkles LifeSpan", 0.0, 0.9)?;
            let width =
                dvc_integer_range_param(&params, 12, "VALUE FX Sparkles Width", 1, 90)? as u16;
            let lifetime_ms = (100.0 / (1.0 - source_lifespan)).round() as u16;
            ColorEffectSpatialRecipe::Sparkle {
                grayscale: false,
                vertical_symmetry,
                raster_mode: ColorEffectSpatialSparkleRasterMode::Sparkle,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                number,
                lifetime_ms: Some(lifetime_ms),
                source_lifespan: Some(source_lifespan),
                width: f32::from(width),
                height: None,
            }
        }
        627 => {
            let vertical_symmetry = dvc_binary_param(&params, 3, "VALUE FX Random fill Transform")?;
            let point_width =
                dvc_integer_range_param(&params, 10, "VALUE FX Random fill Point Width", 1, 10)?
                    as u16;
            // Point Height is serialized for VALUE family 7 but the recovered
            // evaluator forces height to one and never consumes this value.
            let source_point_height =
                dvc_integer_range_param(&params, 11, "VALUE FX Random fill Point Height", 1, 10)?
                    as u16;
            ColorEffectSpatialRecipe::RandomFill {
                grayscale: false,
                vertical_symmetry,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                point_width: f32::from(point_width),
                source_point_height: Some(source_point_height),
            }
        }
        628 => ColorEffectSpatialRecipe::Perlin {
            grayscale: false,
            vertical_symmetry: dvc_binary_param(&params, 3, "VALUE FX Perlin Transform")?,
            horizontal_symmetry: false,
            rotation_degrees: 0.0,
            octaves: dvc_integer_range_param(&params, 10, "VALUE FX Perlin Octaves", 2, 10)? as u8,
            zoom: dvc_integer_range_param(&params, 11, "VALUE FX Perlin Zoom", 1, 100)?,
            direction_degrees: dvc_integer_range_param(
                &params,
                12,
                "VALUE FX Perlin Direction",
                1,
                100,
            )?,
            speed: dvc_integer_range_param(&params, 13, "VALUE FX Perlin Speed", 1, 10)?,
            amplitude: dvc_integer_range_param(&params, 14, "VALUE FX Perlin Amplitude", 5, 100)?,
        },
        _ => unreachable!(),
    };

    let targets = dvc_rack_targets(rack, fixture_refs)?;
    let validates_corrected_random_noop =
        targets.beam_targets.is_empty() && matches!(generator_id, 626 | 627);
    let points = if !targets.beam_targets.is_empty() || validates_corrected_random_noop {
        dvc_value_palette_points(&palette)?
    } else {
        Vec::new()
    };
    if targets.beam_targets.is_empty() {
        let corrected_random_validation = if validates_corrected_random_noop {
            let (_, period_note) = dvc_exact_generator_period(effect, "VALUE FX", generator)?;
            let evaluator_note = dvc_corrected_random_evaluator_note(&recipe, generator_id)
                .expect("VALUE 626/627 always has a corrected random recipe");
            format!("; {period_note}; {evaluator_note}")
        } else {
            String::new()
        };
        return Ok(ConvertedDvcEffect {
            target: None,
            generator,
            note: format!(
                "source_family=Value FX; source no-op preserved: Daslight BEAMS contains zero targets; palette_colors={}; PARAM IDs {expected_ids:?} and TYPEs validated{corrected_random_validation}; no runtime effect was created",
                palette.len(),
            ),
            approximations: Vec::new(),
            warnings: Vec::new(),
        });
    }

    let feature_spec = dvc_rack_feature_spec(rack, profiles)?;
    let mut beam_targets = Vec::new();
    let mut fixture_ids = Vec::new();
    let mut seen_fixture_ids = HashSet::new();
    let mut feature_attributes = Vec::new();
    let mut seen_features = HashSet::new();
    let mut omitted_feature_targets = 0_usize;
    for target in &targets.beam_targets {
        let Some(feature_attribute) =
            dvc_beam_feature_attribute(target, &feature_spec, profiles, fixture_refs)
        else {
            omitted_feature_targets = omitted_feature_targets.saturating_add(1);
            continue;
        };
        if seen_fixture_ids.insert(target.fixture_id) {
            fixture_ids.push(target.fixture_id);
        }
        if seen_features.insert(normalize_dvc_attribute(&feature_attribute)) {
            feature_attributes.push(feature_attribute.clone());
        }
        beam_targets.push(ColorEffectBeamTarget {
            fixture_id: target.fixture_id,
            beam_index: target.beam_index,
            selection_index: target.selection_index,
            feature_attribute: Some(feature_attribute),
        });
    }
    if beam_targets.is_empty() {
        return Err(format!(
            "BEAMS resolved to no VALUE FX targets exposing PRESET type {}",
            feature_spec.preset_type
        ));
    }

    let mut approximations = Vec::new();
    if omitted_feature_targets > 0 {
        approximations.push(format!(
            "{omitted_feature_targets} beam target(s) without PRESET type {} were omitted",
            feature_spec.preset_type
        ));
    }
    let (period_ms, period_note) = if matches!(generator_id, 622 | 624 | 625 | 626 | 627 | 628) {
        dvc_exact_generator_period(effect, "VALUE FX", generator)?
    } else {
        dvc_move_period(
            effect,
            scene,
            &format!("VALUE FX {generator}"),
            &mut approximations,
        )?
    };
    let (clock_sync, clock_note, clock_warning) = dvc_scene_clock_sync(scene);
    if let Some(clock_warning) = clock_warning {
        approximations.push(clock_warning);
    }
    let features = feature_attributes
        .iter()
        .map(|attribute| ChaserFeature {
            attribute: attribute.clone(),
            low: feature_spec.low,
            high: feature_spec.high,
        })
        .collect::<Vec<_>>();
    let primary = features
        .first()
        .ok_or_else(|| "VALUE FX resolved no feature attributes".to_string())?;
    let primary_attribute = primary.attribute.clone();
    let primary_low = primary.low;
    let primary_high = primary.high;
    let evaluator_note = match (&recipe, generator_id) {
        (_, 622) => "evaluator=CBurstEffect corrected analytic pixel-centre radius; equal cyclic palette segments; direct Gradient hold/interpolation".to_string(),
        (_, 624) => "evaluator=CKnightRiderEffect corrected duration-independent analytic geometry; source-over lanes".to_string(),
        (_, 625) => "evaluator=CSweepEffect corrected continuous-phase hard boundary".to_string(),
        (_, 626 | 627) => dvc_corrected_random_evaluator_note(&recipe, generator_id)
            .expect("VALUE 626/627 always has a corrected random recipe"),
        (_, 628) => "evaluator=CPerlinEffect corrected continuous lattice hash/cosine interpolation; implementation=SyndocalCorrected; Direction is activated as spatial phase".to_string(),
        _ => "evaluator=verified generator route".to_string(),
    };
    normalize_dvc_spatial_recipe(
        &mut recipe,
        beam_targets
            .iter()
            .map(|target| target.selection_index)
            .collect::<HashSet<_>>()
            .len(),
        false,
    );
    let request = ValueEffectRequest {
        label: format!("{scene_name} ({generator})"),
        fixture_ids,
        target_group_ids: Vec::new(),
        attribute: primary_attribute,
        features,
        points,
        spatial_pattern: Some(ColorEffectSpatialPattern {
            recipe,
            parameter_model_version: COLOR_EFFECT_SPATIAL_PARAMETER_MODEL_VERSION,
            beam_targets,
            placement: None,
        }),
        interpolation: ValueEffectInterpolation::Line,
        mode: ValueEffectMode::Absolute,
        direction: ValueEffectDirection::Forward,
        period_ms,
        clock_sync,
        low: primary_low,
        high: primary_high,
        phase: 0.0,
        fixture_spread: 0.0,
        blend_mode: EffectBlendMode::Override,
    };
    engine::validate_value_effect_request(&request).map_err(|error| {
        format!("confirmed VALUE FX {generator} parameters are not representable: {error}")
    })?;
    Ok(ConvertedDvcEffect {
        target: Some(CueEffectTarget {
            effect_id,
            enabled: true,
            params: Some(EffectParamsSnapshot::Value(request)),
            transition_ms: None,
        }),
        generator,
        note: format!(
            "source_family=Value FX; value_palette={}; preset_type={}; preset_range={}..{}; preset_source={}; beams={}; selections={}; {period_note}; {clock_note}; {evaluator_note}",
            palette.len(),
            feature_spec.preset_type,
            feature_spec.low,
            feature_spec.high,
            feature_spec.source,
            targets.beam_targets.len(),
            targets.ordered_steps.len(),
        ),
        approximations,
        warnings: Vec::new(),
    })
}

fn convert_dvc_color_spatial_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    generator_id: u16,
    effect_id: u64,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    let generator = match generator_id {
        21 => "Bounce",
        22 => "Burst",
        23 => "Butterfly",
        29 => "Fire",
        30 => "Knight Rider",
        31 => "Lines",
        32 => "Perlin",
        33 => "Media",
        34 => "Plasma",
        35 => "Rain",
        36 => "Rainbow",
        37 => "Random fill",
        40 => "Sparkle",
        41 => "Tube",
        42 => "Spiral",
        44 => "Sweep",
        47 => "Explosion",
        48 => "Starfield",
        49 => "Graph",
        50 => "Grid",
        121 => "Burst",
        127 => "Knight Rider",
        128 => "Perlin",
        129 => "Plasma",
        130 => "Rainbow",
        131 => "Random fill",
        133 => "Sparkle",
        134 => "Sweep",
        521 => "Rainbow",
        522 => "Spiral",
        523 => "Burst",
        524 => "Butterfly",
        525 => "Plasma",
        526 => "Media",
        527 => "Knight Rider",
        528 => "Sweep",
        529 => "Sparkle",
        530 => "Perlin",
        _ => {
            return Err(format!(
                "COLOR/MAPPINGS generator {generator_id} is not confirmed"
            ))
        }
    };
    let source_is_color_mappings = matches!(
        generator_id,
        21 | 22
            | 23
            | 29
            | 30
            | 31
            | 32
            | 33
            | 34
            | 35
            | 36
            | 37
            | 40
            | 41
            | 42
            | 44
            | 47
            | 48
            | 49
            | 50
    );
    let shared_mapping_generator_id = match generator_id {
        22 => 523,
        23 => 524,
        30 => 527,
        32 => 530,
        33 => 526,
        34 => 525,
        35 => 35,
        40 => 529,
        42 => 522,
        44 => 528,
        _ => generator_id,
    };
    if generator_id == 21 {
        require_strict_dvc_bounce_structure(rack, effect)?;
    }
    if matches!(generator_id, 33 | 526) {
        let source_family = if generator_id == 33 {
            "Color Mappings"
        } else {
            "Mappings"
        };
        let label = format!("{source_family} Media ID={generator_id}");
        let params_node =
            direct_child(effect, "PARAMS").ok_or_else(|| format!("{label} is missing PARAMS"))?;
        let expected_nb = if generator_id == 33 { "6" } else { "3" };
        if params_node.attribute("NB") != Some(expected_nb) {
            return Err(format!(
                "{label} expected PARAMS NB={expected_nb}, found {}",
                params_node.attribute("NB").unwrap_or("missing")
            ));
        }
        let param_nodes = element_children(params_node).collect::<Vec<_>>();
        if let Some(unexpected) = param_nodes.iter().find(|node| !node.has_tag_name("PARAM")) {
            return Err(format!(
                "{label} PARAMS contains unexpected <{}> element",
                unexpected.tag_name().name()
            ));
        }
        let mut ids = param_nodes
            .iter()
            .map(|param| {
                required_attribute(*param, "ID", "PARAM")?
                    .parse::<u16>()
                    .map_err(|error| format!("PARAM ID is invalid: {error}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        ids.sort_unstable();
        let expected_ids: &[u16] = if generator_id == 33 {
            &[1, 2, 3, 4, 10, 11]
        } else {
            &[3, 4, 10]
        };
        if ids.as_slice() != expected_ids {
            return Err(format!(
                "{label} expected PARAM IDs {expected_ids:?}, found {ids:?}"
            ));
        }
        if generator_id == 33 {
            require_exact_dvc_param_types(
                effect,
                &[(1, 4), (2, 2), (3, 6), (4, 0), (10, 8), (11, 1)],
            )?;
        } else {
            require_exact_dvc_param_types(effect, &[(3, 6), (4, 0), (10, 8)])?;
        }
        let numeric = |id: u16, label: &str| {
            let param = param_nodes
                .iter()
                .find(|param| {
                    param
                        .attribute("ID")
                        .and_then(|value| value.parse::<u16>().ok())
                        == Some(id)
                })
                .ok_or_else(|| format!("{label} is missing PARAM {id}"))?;
            required_attribute(*param, "VAL", "PARAM")?
                .parse::<f64>()
                .map_err(|error| format!("{label} value is invalid: {error}"))
        };
        let transform = numeric(3, &format!("{label} Transform"))?;
        if !matches!(transform, 0.0 | 1.0 | 2.0) {
            return Err(format!(
                "{label} Transform PARAM 3 must be None(0), Vertical symmetry(1), or Horizontal symmetry(2), found {transform}"
            ));
        }
        let rotation = numeric(4, &format!("{label} Rotation"))?;
        if rotation.fract().abs() > f64::EPSILON || !(0.0..=360.0).contains(&rotation) {
            return Err(format!(
                "{label} Rotation must be an integer within 0..360, found {rotation}"
            ));
        }
        let (palette_colors, grayscale, colorize) = if generator_id == 33 {
            let palette_param = param_nodes
                .iter()
                .find(|param| param.attribute("ID") == Some("1"))
                .ok_or_else(|| format!("{label} is missing palette PARAM 1"))?;
            require_strict_dvc_palette_shape(*palette_param)?;
            let palette = dvc_color_palette_stops(*palette_param)?;
            let grayscale = numeric(2, &format!("{label} Grayscale"))?;
            if !matches!(grayscale, 0.0 | 1.0) {
                return Err(format!(
                    "{label} Grayscale PARAM 2 must be 0 or 1, found {grayscale}"
                ));
            }
            let colorize = numeric(11, &format!("{label} Colorize"))?;
            if !colorize.is_finite() || !(0.0..=1.0).contains(&colorize) {
                return Err(format!(
                    "{label} Colorize PARAM 11 must be finite and within 0..1, found {colorize}"
                ));
            }
            (Some(palette.len()), Some(grayscale as u8), Some(colorize))
        } else {
            (None, None, None)
        };
        let path_param = param_nodes
            .iter()
            .find(|param| param.attribute("ID") == Some("10"))
            .ok_or_else(|| format!("{label} is missing Media Path PARAM 10"))?;
        let path = path_param
            .attribute("VAL")
            .ok_or_else(|| format!("{label} Media Path PARAM 10 is missing VAL"))?;
        if !path.is_empty() {
            return Err(format!(
                "{label} non-empty media paths require decoded embedded-media evidence and remain fail-closed"
            ));
        }
        let color_mapping_targets = if generator_id == 33 {
            let selections = element_children(rack)
                .filter(|node| node.has_tag_name("SELECTIONS"))
                .count();
            if selections != 0 {
                return Err(format!(
                    "{label} external SELECTIONS remain fail-closed; found {selections} SELECTIONS container(s)"
                ));
            }
            let beams =
                direct_child(rack, "BEAMS").ok_or_else(|| format!("{label} is missing BEAMS"))?;
            if beams.attribute("NB").is_none() {
                return Err(format!("{label} BEAMS is missing NB"));
            }
            Some(dvc_rack_targets(rack, fixture_refs)?)
        } else {
            None
        };
        let rectangle = dvc_mapping_rectangle(rack, &label, false)?;
        if rectangle.native_empty_sentinel
            && color_mapping_targets.is_some_and(|targets| !targets.beam_targets.is_empty())
        {
            return Err(format!(
                "{label} native empty Rectangle sentinel is valid only with zero owned BEAMS"
            ));
        }
        return Ok(ConvertedDvcEffect {
            target: None,
            generator,
            note: format!(
                "source_family={source_family}; implementation=source-no-op; evaluator=CMediaEffect/0x140364940; MediaPath is empty; palette_colors={palette_colors:?}; Grayscale={grayscale:?}; Transform={transform}; Rotation={rotation}; Colorize={colorize:?}; Rectangle=({},{},{},{},{}); timing=not-applicable; no runtime effect was created",
                rectangle.x,
                rectangle.y,
                rectangle.sx,
                rectangle.sy,
                rectangle.angle_degrees,
            ),
            approximations: Vec::new(),
            warnings: Vec::new(),
        });
    }
    if generator_id == 36 {
        let params_node = direct_child(effect, "PARAMS")
            .ok_or_else(|| "COLOR MAPPINGS Rainbow ID=36 is missing PARAMS".to_string())?;
        if params_node.attribute("NB") != Some("7") {
            return Err(format!(
                "COLOR MAPPINGS Rainbow ID=36 expected PARAMS NB=7, found {}",
                params_node.attribute("NB").unwrap_or("missing")
            ));
        }
    }
    let (params, stops) = dvc_color_palette_and_params(effect)?;
    let shared_color_mappings = source_is_color_mappings && generator_id != 36;
    let mapping_family_label = if shared_color_mappings {
        "COLOR MAPPINGS"
    } else {
        "MAPPINGS"
    };
    let generator_approximations = Vec::new();
    let mut recipe = match shared_mapping_generator_id {
        21 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                true,
                &[
                    (10, 6),
                    (11, 7),
                    (12, 0),
                    (13, 0),
                    (14, 0),
                    (16, 2),
                    (17, 2),
                    (18, 0),
                ],
            )?;
            if !(protocol::DASLIGHT_COLOR_PALETTE_MIN_STOPS
                ..=protocol::DASLIGHT_COLOR_PALETTE_MAX_STOPS)
                .contains(&stops.len())
            {
                return Err(format!(
                    "COLOR MAPPINGS Bounce ID=21 palette requires 1..255 colors before target/no-op classification, found {}",
                    stops.len()
                ));
            }
            let item =
                match dvc_integer_range_param(&params, 10, "COLOR MAPPINGS Bounce Item", 0, 1)?
                    as u8
                {
                    0 => ColorEffectSpatialBounceItem::Shape,
                    1 => ColorEffectSpatialBounceItem::Points,
                    _ => unreachable!(),
                };
            ColorEffectSpatialRecipe::Bounce {
                grayscale: dvc_binary_param(&params, 2, "COLOR MAPPINGS Bounce Grayscale")?,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                item,
                shape: dvc_integer_range_param(&params, 11, "COLOR MAPPINGS Bounce Shape", 0, 28)?
                    as u8,
                number: dvc_integer_range_param(&params, 12, "COLOR MAPPINGS Bounce Number", 1, 20)?
                    as u16,
                size: dvc_integer_range_param(&params, 13, "COLOR MAPPINGS Bounce Size", 1, 100)?
                    as u16,
                speed: dvc_integer_range_param(&params, 14, "COLOR MAPPINGS Bounce Speed", 0, 10)?
                    as u16,
                collide: dvc_binary_param(&params, 16, "COLOR MAPPINGS Bounce Collide")?,
                fill: dvc_binary_param(&params, 17, "COLOR MAPPINGS Bounce Fill")?,
                points: dvc_integer_range_param(&params, 18, "COLOR MAPPINGS Bounce Points", 2, 10)?
                    as u16,
            }
        }
        29 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                true,
                &[(10, 0), (11, 0), (12, 0), (13, 0)],
            )?;
            if !(2..=4).contains(&stops.len()) {
                return Err(format!(
                    "COLOR MAPPINGS Fire ID=29 palette requires 2..4 colors, found {}",
                    stops.len()
                ));
            }
            ColorEffectSpatialRecipe::Fire {
                grayscale: dvc_binary_param(&params, 2, "COLOR MAPPINGS Fire Grayscale")?,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                flames: dvc_integer_range_param(&params, 10, "COLOR MAPPINGS Fire Flames", 1, 100)?
                    as u16,
                width: dvc_integer_range_param(&params, 11, "COLOR MAPPINGS Fire Width", 10, 200)?
                    as u16,
                height: dvc_integer_range_param(&params, 12, "COLOR MAPPINGS Fire Height", 1, 100)?
                    as u16,
                hotspot: dvc_integer_range_param(
                    &params,
                    13,
                    "COLOR MAPPINGS Fire Hotspot",
                    10,
                    255,
                )? as u16,
            }
        }
        31 => {
            require_exact_dvc_mapping_recipe_schema(effect, &params, true, &[(10, 0)])?;
            if !(2..=protocol::DASLIGHT_COLOR_PALETTE_MAX_STOPS).contains(&stops.len()) {
                return Err(format!(
                    "COLOR MAPPINGS Lines ID=31 palette requires 2..255 colors, found {}",
                    stops.len()
                ));
            }
            ColorEffectSpatialRecipe::Lines {
                grayscale: dvc_binary_param(&params, 2, "COLOR MAPPINGS Lines Grayscale")?,
                size: dvc_integer_range_param(&params, 10, "COLOR MAPPINGS Lines Size", 2, 20)?
                    as u16,
            }
        }
        35 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                true,
                &[(10, 0), (11, 0), (12, 0), (13, 0), (14, 0)],
            )?;
            if !(protocol::DASLIGHT_COLOR_PALETTE_MIN_STOPS
                ..=protocol::DASLIGHT_COLOR_PALETTE_MAX_STOPS)
                .contains(&stops.len())
            {
                return Err(format!(
                    "COLOR MAPPINGS Rain ID=35 palette requires 1..255 colors, found {}",
                    stops.len()
                ));
            }
            ColorEffectSpatialRecipe::Rain {
                grayscale: dvc_binary_param(&params, 2, "COLOR MAPPINGS Rain Grayscale")?,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                speed: dvc_integer_range_param(&params, 10, "COLOR MAPPINGS Rain Speed", 0, 10)?
                    as u16,
                width: dvc_integer_range_param(&params, 11, "COLOR MAPPINGS Rain Width", 5, 10)?
                    as u16,
                height: dvc_integer_range_param(&params, 12, "COLOR MAPPINGS Rain Height", 10, 30)?
                    as u16,
                number: dvc_integer_range_param(&params, 13, "COLOR MAPPINGS Rain Number", 1, 100)?
                    as u16,
                trail: dvc_integer_range_param(&params, 14, "COLOR MAPPINGS Rain Trail", 1, 30)?
                    as u16,
            }
        }
        47 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                true,
                &[
                    (10, 7),
                    (11, 0),
                    (12, 0),
                    (13, 0),
                    (14, 0),
                    (15, 1),
                    (16, 0),
                    (17, 1),
                ],
            )?;
            if !(2..=protocol::DASLIGHT_COLOR_PALETTE_MAX_STOPS).contains(&stops.len()) {
                return Err(format!(
                    "COLOR MAPPINGS Explosion ID=47 palette requires 2..255 colors, found {}",
                    stops.len()
                ));
            }
            ColorEffectSpatialRecipe::Explosion {
                grayscale: dvc_binary_param(&params, 2, "COLOR MAPPINGS Explosion Grayscale")?,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                shape: dvc_integer_range_param(
                    &params,
                    10,
                    "COLOR MAPPINGS Explosion Shape",
                    0,
                    29,
                )? as u8,
                explosion_number: dvc_integer_range_param(
                    &params,
                    11,
                    "COLOR MAPPINGS Explosion Number",
                    1,
                    50,
                )? as u16,
                explosion_size: dvc_integer_range_param(
                    &params,
                    12,
                    "COLOR MAPPINGS Explosion Size",
                    0,
                    100,
                )? as u16,
                particle_number: dvc_integer_range_param(
                    &params,
                    13,
                    "COLOR MAPPINGS Explosion Particles",
                    1,
                    100,
                )? as u16,
                particle_size: dvc_integer_range_param(
                    &params,
                    14,
                    "COLOR MAPPINGS Explosion Particle Size",
                    1,
                    100,
                )? as u16,
                particle_life: dvc_finite_range_param(
                    &params,
                    15,
                    "COLOR MAPPINGS Explosion Life",
                    0.0,
                    0.9,
                )?,
                trail_size: dvc_integer_range_param(
                    &params,
                    16,
                    "COLOR MAPPINGS Explosion Trail",
                    1,
                    25,
                )? as u16,
                gravity: dvc_finite_range_param(
                    &params,
                    17,
                    "COLOR MAPPINGS Explosion Gravity",
                    0.0,
                    10.0,
                )?,
            }
        }
        48 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                true,
                &[(10, 7), (11, 0), (12, 0), (13, 0), (14, 1)],
            )?;
            if !(2..=protocol::DASLIGHT_COLOR_PALETTE_MAX_STOPS).contains(&stops.len()) {
                return Err(format!(
                    "COLOR MAPPINGS Starfield ID=48 palette requires 2..255 colors, found {}",
                    stops.len()
                ));
            }
            ColorEffectSpatialRecipe::Starfield {
                grayscale: dvc_binary_param(&params, 2, "COLOR MAPPINGS Starfield Grayscale")?,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                shape: dvc_integer_range_param(
                    &params,
                    10,
                    "COLOR MAPPINGS Starfield Shape",
                    0,
                    29,
                )? as u8,
                particles: dvc_integer_range_param(
                    &params,
                    11,
                    "COLOR MAPPINGS Starfield Particles",
                    1,
                    10,
                )? as u16,
                size: dvc_integer_range_param(&params, 12, "COLOR MAPPINGS Starfield Size", 1, 100)?
                    as u16,
                trail: dvc_integer_range_param(
                    &params,
                    13,
                    "COLOR MAPPINGS Starfield Trail",
                    1,
                    25,
                )? as u16,
                rotation: dvc_finite_range_param(
                    &params,
                    14,
                    "COLOR MAPPINGS Starfield Rotation",
                    -5.0,
                    5.0,
                )?,
            }
        }
        49 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                true,
                &[(10, 0), (11, 0), (12, 0), (13, 0), (14, 1), (15, 1)],
            )?;
            if !(2..=10).contains(&stops.len()) {
                return Err(format!(
                    "COLOR MAPPINGS Graph ID=49 palette requires 2..10 colors, found {}",
                    stops.len()
                ));
            }
            ColorEffectSpatialRecipe::Graph {
                grayscale: dvc_binary_param(&params, 2, "COLOR MAPPINGS Graph Grayscale")?,
                height: dvc_integer_range_param(&params, 10, "COLOR MAPPINGS Graph Height", 1, 100)?
                    as u16,
                width: dvc_integer_range_param(&params, 11, "COLOR MAPPINGS Graph Width", 1, 100)?
                    as u16,
                pitch: dvc_integer_range_param(&params, 12, "COLOR MAPPINGS Graph Pitch", 0, 100)?
                    as u16,
                frequency: dvc_integer_range_param(
                    &params,
                    13,
                    "COLOR MAPPINGS Graph Frequency",
                    0,
                    10,
                )? as u16,
                amplitude: dvc_finite_range_param(
                    &params,
                    14,
                    "COLOR MAPPINGS Graph Amplitude",
                    0.0,
                    2.0,
                )?,
                offset: dvc_finite_range_param(
                    &params,
                    15,
                    "COLOR MAPPINGS Graph Offset",
                    -1.0,
                    1.0,
                )?,
            }
        }
        50 => {
            require_exact_dvc_mapping_recipe_schema(effect, &params, true, &[(10, 0), (11, 0)])?;
            if !(2..=5).contains(&stops.len()) {
                return Err(format!(
                    "COLOR MAPPINGS Grid ID=50 palette requires 2..5 colors, found {}",
                    stops.len()
                ));
            }
            ColorEffectSpatialRecipe::Grid {
                grayscale: dvc_binary_param(&params, 2, "COLOR MAPPINGS Grid Grayscale")?,
                size: dvc_integer_range_param(&params, 10, "COLOR MAPPINGS Grid Size", 1, 5)?
                    as u16,
                width: dvc_integer_range_param(&params, 11, "COLOR MAPPINGS Grid Width", 2, 20)?
                    as u16,
            }
        }
        36 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                true,
                &[(10, 1), (11, 0), (12, 1)],
            )?;
            let grayscale = dvc_binary_param(&params, 2, "COLOR MAPPINGS Rainbow Grayscale")?;
            let transform = dvc_param(&params, 3, "COLOR MAPPINGS Rainbow Transform")?;
            if !matches!(transform, 0.0 | 1.0 | 2.0) {
                return Err(format!(
                    "COLOR MAPPINGS Rainbow Transform PARAM 3 must be None(0), Vertical symmetry(1), or Horizontal symmetry(2), found {transform}"
                ));
            }
            ColorEffectSpatialRecipe::Rainbow {
                grayscale,
                vertical_symmetry: transform == 1.0,
                horizontal_symmetry: transform == 2.0,
                rotation_degrees: dvc_integer_range_param(
                    &params,
                    4,
                    "COLOR MAPPINGS Rainbow Rotation",
                    0,
                    360,
                )?,
                color_width: dvc_unit_param(&params, 10, "COLOR MAPPINGS Rainbow Color Width")?
                    * 100.0,
                angle_degrees: dvc_integer_range_param(
                    &params,
                    11,
                    "COLOR MAPPINGS Rainbow Angle",
                    0,
                    360,
                )?,
                gradient: dvc_unit_param(&params, 12, "COLOR MAPPINGS Rainbow Gradient")? * 100.0,
            }
        }
        37 => {
            require_exact_dvc_mapping_recipe_schema(effect, &params, true, &[(10, 0), (11, 0)])?;
            ColorEffectSpatialRecipe::RandomFill {
                grayscale: dvc_binary_param(&params, 2, "COLOR MAPPINGS Random fill Grayscale")?,
                // COLOR MAPPINGS Transform and Rotation are placement-space
                // operations populated below, not one-dimensional strip flags.
                vertical_symmetry: false,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                point_width: dvc_integer_range_param(
                    &params,
                    10,
                    "COLOR MAPPINGS Random fill Point Width",
                    1,
                    10,
                )?,
                source_point_height: Some(dvc_integer_range_param(
                    &params,
                    11,
                    "COLOR MAPPINGS Random fill Point Height",
                    1,
                    10,
                )? as u16),
            }
        }
        127 => {
            require_exact_dvc_params(&params, &[2, 3, 10, 11, 12, 13, 14])?;
            require_exact_dvc_param_types(
                effect,
                &[
                    (1, 4),
                    (2, 2),
                    (3, 6),
                    (10, 0),
                    (11, 2),
                    (12, 2),
                    (13, 2),
                    (14, 0),
                ],
            )?;
            let grayscale = dvc_binary_param(&params, 2, "COLOR FX Knight Rider Grayscale")?;
            let transform = dvc_param(&params, 3, "COLOR FX Knight Rider Transform")?;
            if !matches!(transform, 0.0 | 1.0) {
                return Err(format!(
                    "COLOR FX Knight Rider Transform PARAM 3 must be None(0) or Vertical symmetry(1), found {transform}"
                ));
            }
            ColorEffectSpatialRecipe::KnightRider {
                grayscale,
                vertical_symmetry: transform == 1.0,
                size: dvc_integer_range_param(&params, 10, "COLOR FX Knight Rider Size", 1, 100)?,
                one_way: dvc_binary_param(&params, 11, "COLOR FX Knight Rider One Way Only")?,
                fading: dvc_binary_param(&params, 12, "COLOR FX Knight Rider Fading")?,
                go_outside: dvc_binary_param(&params, 13, "COLOR FX Knight Rider Go Outside")?,
                gradient: dvc_integer_range_param(
                    &params,
                    14,
                    "COLOR FX Knight Rider Gradient",
                    0,
                    100,
                )? as f32,
            }
        }
        128 => {
            require_exact_dvc_params(&params, &[2, 3, 10, 11, 12, 13, 14])?;
            require_exact_dvc_param_types(
                effect,
                &[
                    (1, 4),
                    (2, 2),
                    (3, 6),
                    (10, 0),
                    (11, 0),
                    (12, 0),
                    (13, 0),
                    (14, 0),
                ],
            )?;
            let transform = dvc_param(&params, 3, "COLOR FX Perlin Transform")?;
            if !matches!(transform, 0.0 | 1.0) {
                return Err(format!(
                    "COLOR FX Perlin Transform PARAM 3 must be None(0) or Vertical symmetry(1), found {transform}"
                ));
            }
            ColorEffectSpatialRecipe::Perlin {
                grayscale: dvc_binary_param(&params, 2, "COLOR FX Perlin Grayscale")?,
                vertical_symmetry: transform == 1.0,
                horizontal_symmetry: false,
                rotation_degrees: 0.0,
                octaves: dvc_integer_range_param(&params, 10, "COLOR FX Perlin Octaves", 2, 10)?
                    as u8,
                zoom: dvc_integer_range_param(&params, 11, "COLOR FX Perlin Zoom", 1, 100)?,
                direction_degrees: dvc_integer_range_param(
                    &params,
                    12,
                    "COLOR FX Perlin Direction",
                    1,
                    100,
                )?,
                speed: dvc_integer_range_param(&params, 13, "COLOR FX Perlin Speed", 1, 10)?,
                amplitude: dvc_integer_range_param(
                    &params,
                    14,
                    "COLOR FX Perlin Amplitude",
                    5,
                    100,
                )?,
            }
        }
        121 => {
            require_exact_dvc_params(&params, &[2, 3, 10, 11])?;
            require_exact_dvc_param_types(effect, &[(1, 4), (2, 2), (3, 6), (10, 0), (11, 1)])?;
            ColorEffectSpatialRecipe::Burst {
                grayscale: dvc_binary_param(&params, 2, "COLOR FX Burst Grayscale")?,
                vertical_symmetry: dvc_binary_param(&params, 3, "COLOR FX Burst Transform")?,
                color_width: dvc_integer_range_param(&params, 10, "COLOR FX Burst Width", 10, 900)?,
                gradient: dvc_unit_param(&params, 11, "COLOR FX Burst Gradient")?,
            }
        }
        131 => {
            require_exact_dvc_params(&params, &[2, 3, 10])?;
            require_exact_dvc_param_types(effect, &[(1, 4), (2, 2), (3, 6), (10, 0)])?;
            ColorEffectSpatialRecipe::RandomFill {
                grayscale: dvc_binary_param(&params, 2, "COLOR FX Random fill Grayscale")?,
                vertical_symmetry: dvc_binary_param(&params, 3, "COLOR FX Random fill Transform")?,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                point_width: dvc_integer_range_param(
                    &params,
                    10,
                    "COLOR FX Random fill Point Width",
                    1,
                    10,
                )?,
                source_point_height: None,
            }
        }
        133 => {
            require_exact_dvc_params(&params, &[2, 3, 10, 11, 12])?;
            require_exact_dvc_param_types(
                effect,
                &[(1, 4), (2, 2), (3, 6), (10, 0), (11, 1), (12, 0)],
            )?;
            let source_lifespan =
                dvc_finite_range_param(&params, 11, "COLOR FX Sparkle LifeSpan", 0.0, 0.9)?;
            ColorEffectSpatialRecipe::Sparkle {
                grayscale: dvc_binary_param(&params, 2, "COLOR FX Sparkle Grayscale")?,
                vertical_symmetry: dvc_binary_param(&params, 3, "COLOR FX Sparkle Transform")?,
                raster_mode: ColorEffectSpatialSparkleRasterMode::Sparkle,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                number: dvc_integer_range_param(&params, 10, "COLOR FX Sparkle Number", 1, 10)?
                    as u16,
                lifetime_ms: Some((100.0 / (1.0 - source_lifespan)).round() as u16),
                source_lifespan: Some(source_lifespan),
                width: dvc_integer_range_param(&params, 12, "COLOR FX Sparkle Width", 1, 90)?,
                height: None,
            }
        }
        134 => {
            require_exact_dvc_params(&params, &[2, 3, 10])?;
            require_exact_dvc_param_types(effect, &[(1, 4), (2, 2), (3, 6), (10, 2)])?;
            ColorEffectSpatialRecipe::Sweep {
                grayscale: dvc_binary_param(&params, 2, "COLOR FX Sweep Grayscale")?,
                vertical_symmetry: dvc_binary_param(&params, 3, "COLOR FX Sweep Transform")?,
                direction_change: dvc_binary_param(&params, 10, "COLOR FX Sweep Direction Change")?,
            }
        }
        129 => {
            require_exact_dvc_params(&params, &[2, 3, 10, 11, 12, 13, 14, 15, 16, 17])?;
            require_exact_dvc_param_types(
                effect,
                &[
                    (1, 4),
                    (2, 2),
                    (3, 6),
                    (10, 0),
                    (11, 0),
                    (12, 0),
                    (13, 0),
                    (14, 0),
                    (15, 0),
                    (16, 0),
                    (17, 0),
                ],
            )?;
            let grayscale = dvc_binary_param(&params, 2, "Plasma Grayscale")?;
            let transform = dvc_param(&params, 3, "Plasma Transform")?;
            if !matches!(transform, 0.0 | 1.0) {
                return Err(format!(
                    "Plasma Transform PARAM 3 must be None(0) or Vertical symmetry(1), found {transform}"
                ));
            }
            ColorEffectSpatialRecipe::Plasma {
                grayscale,
                vertical_symmetry: transform == 1.0,
                size_x: dvc_integer_range_param(&params, 10, "Plasma Size X", 0, 20)?,
                param_x: dvc_integer_range_param(&params, 11, "Plasma Param X", 0, 20)?,
                size_y: dvc_integer_range_param(&params, 12, "Plasma Size Y", 0, 20)?,
                param_y: dvc_integer_range_param(&params, 13, "Plasma Param Y", 0, 20)?,
                speed_x: dvc_integer_range_param(&params, 14, "Plasma Speed X", -5, 5)?,
                param_sx: dvc_integer_range_param(&params, 15, "Plasma Param SX", -5, 5)?,
                speed_y: dvc_integer_range_param(&params, 16, "Plasma Speed Y", -5, 5)?,
                param_sy: dvc_integer_range_param(&params, 17, "Plasma Param SY", -5, 5)?,
            }
        }
        130 => {
            require_exact_dvc_params(&params, &[2, 3, 10, 11, 12])?;
            require_exact_dvc_param_types(
                effect,
                &[(1, 4), (2, 2), (3, 6), (10, 1), (11, 0), (12, 1)],
            )?;
            let grayscale = dvc_binary_param(&params, 2, "Rainbow Grayscale")?;
            let transform = dvc_param(&params, 3, "Rainbow Transform")?;
            if !matches!(transform, 0.0 | 1.0) {
                return Err(format!(
                    "Rainbow Transform PARAM 3 must be None(0) or Vertical symmetry(1), found {transform}"
                ));
            }
            ColorEffectSpatialRecipe::ColorRainbow {
                grayscale,
                vertical_symmetry: transform == 1.0,
                color_width: dvc_unit_param(&params, 10, "Rainbow Color Width")?,
                angle_degrees: dvc_integer_range_param(&params, 11, "Rainbow Angle", 0, 360)?,
                gradient: dvc_unit_param(&params, 12, "Gradient")? * 100.0,
            }
        }
        521 => {
            require_exact_dvc_params(&params, &[3, 4, 10, 11, 12])?;
            require_exact_dvc_param_types(
                effect,
                &[(1, 4), (3, 6), (4, 0), (10, 1), (11, 0), (12, 1)],
            )?;
            let transform = dvc_param(&params, 3, "Transform")?;
            if !matches!(transform, 0.0 | 1.0 | 2.0) {
                return Err(format!(
                    "Rainbow Transform PARAM 3 must be None(0), Vertical symmetry(1), or Horizontal symmetry(2), found {transform}"
                ));
            }
            ColorEffectSpatialRecipe::Rainbow {
                grayscale: false,
                vertical_symmetry: transform == 1.0,
                horizontal_symmetry: transform == 2.0,
                rotation_degrees: dvc_integer_range_param(
                    &params,
                    4,
                    "MAPPINGS Rainbow Rotation",
                    0,
                    360,
                )?,
                color_width: dvc_unit_param(&params, 10, "MAPPINGS Rainbow Color Width")? * 100.0,
                angle_degrees: dvc_integer_range_param(
                    &params,
                    11,
                    "MAPPINGS Rainbow Angle",
                    0,
                    360,
                )?,
                gradient: dvc_unit_param(&params, 12, "Gradient")? * 100.0,
            }
        }
        522 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                shared_color_mappings,
                &[(10, 0), (11, 0), (12, 1)],
            )?;
            ColorEffectSpatialRecipe::Spiral {
                grayscale: if shared_color_mappings {
                    dvc_binary_param(&params, 2, "COLOR MAPPINGS Spiral Grayscale")?
                } else {
                    false
                },
                radius: dvc_integer_range_param(
                    &params,
                    10,
                    &format!("{mapping_family_label} Spiral Radius"),
                    0,
                    200,
                )?,
                arms: dvc_integer_range_param(
                    &params,
                    11,
                    &format!("{mapping_family_label} Spiral Arms"),
                    1,
                    10,
                )? as u16,
                gradient: dvc_unit_param(
                    &params,
                    12,
                    &format!("{mapping_family_label} Spiral Gradient"),
                )? * 100.0,
            }
        }
        523 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                shared_color_mappings,
                &[(10, 0), (11, 1)],
            )?;
            ColorEffectSpatialRecipe::Burst {
                grayscale: if shared_color_mappings {
                    dvc_binary_param(&params, 2, "COLOR MAPPINGS Burst Grayscale")?
                } else {
                    false
                },
                vertical_symmetry: false,
                color_width: dvc_integer_range_param(
                    &params,
                    10,
                    &format!("{mapping_family_label} Burst Color Width"),
                    10,
                    900,
                )?,
                gradient: dvc_unit_param(
                    &params,
                    11,
                    &format!("{mapping_family_label} Burst Gradient"),
                )?,
            }
        }
        524 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                shared_color_mappings,
                &[(10, 0), (11, 1), (12, 2)],
            )?;
            ColorEffectSpatialRecipe::Butterfly {
                grayscale: if shared_color_mappings {
                    dvc_binary_param(&params, 2, "COLOR MAPPINGS Butterfly Grayscale")?
                } else {
                    false
                },
                color_width: dvc_integer_range_param(
                    &params,
                    10,
                    &format!("{mapping_family_label} Butterfly Color Width"),
                    1,
                    100,
                )?,
                gradient: dvc_unit_param(
                    &params,
                    11,
                    &format!("{mapping_family_label} Butterfly Gradient"),
                )? * 100.0,
                clockwise: dvc_binary_param(
                    &params,
                    12,
                    &format!("{mapping_family_label} Butterfly Clockwise"),
                )?,
            }
        }
        525 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                shared_color_mappings,
                &[
                    (10, 0),
                    (11, 0),
                    (12, 0),
                    (13, 0),
                    (14, 0),
                    (15, 0),
                    (16, 0),
                    (17, 0),
                ],
            )?;
            ColorEffectSpatialRecipe::Plasma {
                grayscale: if shared_color_mappings {
                    dvc_binary_param(&params, 2, "COLOR MAPPINGS Plasma Grayscale")?
                } else {
                    false
                },
                vertical_symmetry: false,
                size_x: dvc_integer_range_param(
                    &params,
                    10,
                    &format!("{mapping_family_label} Plasma Size X"),
                    0,
                    20,
                )?,
                param_x: dvc_integer_range_param(
                    &params,
                    11,
                    &format!("{mapping_family_label} Plasma Param X"),
                    0,
                    20,
                )?,
                size_y: dvc_integer_range_param(
                    &params,
                    12,
                    &format!("{mapping_family_label} Plasma Size Y"),
                    0,
                    20,
                )?,
                param_y: dvc_integer_range_param(
                    &params,
                    13,
                    &format!("{mapping_family_label} Plasma Param Y"),
                    0,
                    20,
                )?,
                speed_x: dvc_integer_range_param(
                    &params,
                    14,
                    &format!("{mapping_family_label} Plasma Speed X"),
                    -5,
                    5,
                )?,
                param_sx: dvc_integer_range_param(
                    &params,
                    15,
                    &format!("{mapping_family_label} Plasma Param SX"),
                    -5,
                    5,
                )?,
                speed_y: dvc_integer_range_param(
                    &params,
                    16,
                    &format!("{mapping_family_label} Plasma Speed Y"),
                    -5,
                    5,
                )?,
                param_sy: dvc_integer_range_param(
                    &params,
                    17,
                    &format!("{mapping_family_label} Plasma Param SY"),
                    -5,
                    5,
                )?,
            }
        }
        527 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                shared_color_mappings,
                &[(10, 0), (11, 2), (12, 2), (13, 2), (14, 0)],
            )?;
            ColorEffectSpatialRecipe::KnightRider {
                grayscale: if shared_color_mappings {
                    dvc_binary_param(&params, 2, "COLOR MAPPINGS Knight Rider Grayscale")?
                } else {
                    false
                },
                vertical_symmetry: false,
                size: dvc_integer_range_param(
                    &params,
                    10,
                    &format!("{mapping_family_label} Knight Rider Size"),
                    1,
                    100,
                )?,
                one_way: dvc_binary_param(
                    &params,
                    11,
                    &format!("{mapping_family_label} Knight Rider One Way"),
                )?,
                fading: dvc_binary_param(
                    &params,
                    12,
                    &format!("{mapping_family_label} Knight Rider Fading"),
                )?,
                go_outside: dvc_binary_param(
                    &params,
                    13,
                    &format!("{mapping_family_label} Knight Rider Go Outside"),
                )?,
                gradient: dvc_integer_range_param(
                    &params,
                    14,
                    &format!("{mapping_family_label} Knight Rider Gradient"),
                    0,
                    100,
                )?,
            }
        }
        528 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                shared_color_mappings,
                &[(10, 2)],
            )?;
            ColorEffectSpatialRecipe::Sweep {
                grayscale: if shared_color_mappings {
                    dvc_binary_param(&params, 2, "COLOR MAPPINGS Sweep Grayscale")?
                } else {
                    false
                },
                vertical_symmetry: false,
                direction_change: dvc_binary_param(
                    &params,
                    10,
                    &format!("{mapping_family_label} Sweep Direction Change"),
                )?,
            }
        }
        529 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                shared_color_mappings,
                &[(10, 0), (11, 1), (12, 0), (13, 0)],
            )?;
            let source_lifespan = dvc_finite_range_param(
                &params,
                11,
                &format!("{mapping_family_label} Sparkle LifeSpan"),
                0.0,
                0.9,
            )?;
            ColorEffectSpatialRecipe::Sparkle {
                grayscale: if shared_color_mappings {
                    dvc_binary_param(&params, 2, "COLOR MAPPINGS Sparkle Grayscale")?
                } else {
                    false
                },
                vertical_symmetry: false,
                raster_mode: ColorEffectSpatialSparkleRasterMode::Sparkle,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                number: dvc_integer_range_param(
                    &params,
                    10,
                    &format!("{mapping_family_label} Sparkle Number"),
                    1,
                    10,
                )? as u16,
                lifetime_ms: Some((100.0 / (1.0 - source_lifespan)).round() as u16),
                source_lifespan: Some(source_lifespan),
                width: dvc_integer_range_param(
                    &params,
                    12,
                    &format!("{mapping_family_label} Sparkle Width"),
                    1,
                    90,
                )?,
                height: Some(dvc_integer_range_param(
                    &params,
                    13,
                    &format!("{mapping_family_label} Sparkle Height"),
                    1,
                    90,
                )?),
            }
        }
        41 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                true,
                &[(10, 0), (11, 1), (12, 0)],
            )?;
            if !(2..=protocol::DASLIGHT_COLOR_PALETTE_MAX_STOPS).contains(&stops.len()) {
                return Err(format!(
                    "COLOR MAPPINGS Tube ID=41 palette requires 2..255 colors, found {}",
                    stops.len()
                ));
            }
            let source_lifespan =
                dvc_finite_range_param(&params, 11, "COLOR MAPPINGS Tube LifeSpan", 0.0, 0.9)?;
            ColorEffectSpatialRecipe::Sparkle {
                grayscale: dvc_binary_param(&params, 2, "COLOR MAPPINGS Tube Grayscale")?,
                // COLOR MAPPINGS Transform and Rotation are placement-space
                // operations populated below, not one-dimensional strip flags.
                vertical_symmetry: false,
                raster_mode: ColorEffectSpatialSparkleRasterMode::TubeFullRasterHeight,
                rng_seed: dvc_corrected_rng_seed(scene, rack, effect, generator_id),
                number: dvc_integer_range_param(&params, 10, "COLOR MAPPINGS Tube Number", 1, 10)?
                    as u16,
                lifetime_ms: Some((100.0 / (1.0 - source_lifespan)).round() as u16),
                source_lifespan: Some(source_lifespan),
                width: dvc_integer_range_param(&params, 12, "COLOR MAPPINGS Tube Width", 1, 90)?,
                // Tube has no Height PARAM: the shared retained-particle
                // evaluator paints all 100 mapping rows.
                height: None,
            }
        }
        530 => {
            require_exact_dvc_mapping_recipe_schema(
                effect,
                &params,
                shared_color_mappings,
                &[(10, 0), (11, 0), (12, 0), (13, 0), (14, 0)],
            )?;
            let transform = dvc_param(
                &params,
                3,
                &format!("{mapping_family_label} Perlin Transform"),
            )?;
            if !matches!(transform, 0.0 | 1.0 | 2.0) {
                return Err(format!(
                    "{mapping_family_label} Perlin Transform PARAM 3 must be None(0), Vertical symmetry(1), or Horizontal symmetry(2), found {transform}"
                ));
            }
            let rotation_degrees = dvc_integer_range_param(
                &params,
                4,
                &format!("{mapping_family_label} Perlin Rotation"),
                0,
                360,
            )?;
            ColorEffectSpatialRecipe::Perlin {
                grayscale: if shared_color_mappings {
                    dvc_binary_param(&params, 2, "COLOR MAPPINGS Perlin Grayscale")?
                } else {
                    false
                },
                vertical_symmetry: transform == 1.0,
                horizontal_symmetry: transform == 2.0,
                rotation_degrees,
                octaves: dvc_integer_range_param(
                    &params,
                    10,
                    &format!("{mapping_family_label} Perlin Octaves"),
                    2,
                    10,
                )? as u8,
                zoom: dvc_integer_range_param(
                    &params,
                    11,
                    &format!("{mapping_family_label} Perlin Zoom"),
                    1,
                    100,
                )?,
                direction_degrees: dvc_integer_range_param(
                    &params,
                    12,
                    &format!("{mapping_family_label} Perlin Direction"),
                    1,
                    100,
                )?,
                speed: dvc_integer_range_param(
                    &params,
                    13,
                    &format!("{mapping_family_label} Perlin Speed"),
                    1,
                    10,
                )?,
                amplitude: dvc_integer_range_param(
                    &params,
                    14,
                    &format!("{mapping_family_label} Perlin Amplitude"),
                    5,
                    100,
                )?,
            }
        }
        _ => unreachable!(),
    };
    let source_is_mappings = matches!(generator_id, 521..=530);
    let placed_spatial_recipe = source_is_color_mappings || source_is_mappings;
    let placement_label = if source_is_color_mappings {
        format!("COLOR MAPPINGS {generator} ID={generator_id}")
    } else {
        format!("MAPPINGS {generator} ID={generator_id}")
    };
    let mut mapping_rectangle = placed_spatial_recipe
        .then(|| {
            dvc_mapping_rectangle(
                rack,
                &placement_label,
                matches!(
                    generator_id,
                    21 | 29 | 31 | 35 | 36 | 41 | 47 | 48 | 49 | 50
                ) || shared_mapping_generator_id == 530,
            )
        })
        .transpose()?;
    let source_mapping_rectangle = mapping_rectangle;
    if matches!(generator_id, 21 | 29 | 31 | 35 | 41 | 47 | 48 | 49 | 50) {
        let beam_containers = element_children(rack)
            .filter(|node| node.has_tag_name("BEAMS"))
            .collect::<Vec<_>>();
        if beam_containers.len() != 1 {
            return Err(format!(
                "{placement_label} expected exactly one direct BEAMS container, found {}",
                beam_containers.len()
            ));
        }
        let beams = beam_containers[0];
        let declared = beams
            .attribute("NB")
            .ok_or_else(|| format!("{placement_label} BEAMS is missing NB"))?
            .parse::<usize>()
            .map_err(|error| format!("{placement_label} BEAMS NB is invalid: {error}"))?;
        let beam_children = element_children(beams).collect::<Vec<_>>();
        if let Some(unexpected) = beam_children.iter().find(|node| !node.has_tag_name("BEAM")) {
            return Err(format!(
                "{placement_label} BEAMS contains unexpected <{}> element",
                unexpected.tag_name().name()
            ));
        }
        if declared != beam_children.len() {
            return Err(format!(
                "{placement_label} BEAMS declares {declared} entries but contains {}",
                beam_children.len()
            ));
        }
    } else if source_is_color_mappings {
        let beams = direct_child(rack, "BEAMS")
            .ok_or_else(|| format!("{placement_label} is missing BEAMS"))?;
        if beams.attribute("NB").is_none() {
            return Err(format!("{placement_label} BEAMS is missing NB"));
        }
    }
    let mut targets = dvc_rack_targets(rack, fixture_refs)?;
    if source_is_color_mappings {
        let selections = element_children(rack)
            .filter(|node| node.has_tag_name("SELECTIONS"))
            .count();
        if selections != 0 {
            return Err(format!(
                "{placement_label} external SELECTIONS remain fail-closed; found {selections} SELECTIONS container(s)"
            ));
        }
    }
    let source_noop = source_is_color_mappings && targets.beam_targets.is_empty();
    if generator_id == 21 && !source_noop && stops.len() < 2 {
        return Err(format!(
            "COLOR MAPPINGS Bounce ID=21 populated targets require 2..255 palette colors, found {}",
            stops.len()
        ));
    }
    if mapping_rectangle.is_some_and(|rectangle| rectangle.native_empty_sentinel) {
        if source_noop {
            // Native COLOR MAPPINGS serializes an unset Rectangle this way.
            // It is a verified source no-op, not an invalid runtime geometry.
            mapping_rectangle = None;
        } else {
            return Err(format!(
                "{placement_label} native empty Rectangle sentinel is valid only with zero owned BEAMS"
            ));
        }
    }
    if targets.beam_targets.is_empty() && !source_noop {
        return Err(format!("BEAMS resolved to no {generator} beam targets"));
    }
    if !source_noop {
        let unsupported_shape = match &recipe {
            ColorEffectSpatialRecipe::Bounce {
                item: ColorEffectSpatialBounceItem::Shape,
                shape,
                ..
            } if *shape != 0 => Some(*shape),
            ColorEffectSpatialRecipe::Explosion { shape, .. }
            | ColorEffectSpatialRecipe::Starfield { shape, .. }
                if *shape != 0 =>
            {
                Some(*shape)
            }
            _ => None,
        };
        if let Some(shape) = unsupported_shape {
            if generator_id == 21 {
                return Err(format!(
                    "COLOR MAPPINGS Bounce ID=21 populated Shape {shape} has unsupported XEEL glyph geometry; only Shape 0 is recovered"
                ));
            }
            return Err(format!(
                "COLOR MAPPINGS {generator} ID={generator_id} populated Shape {shape} is a proprietary glyph; only filled-ellipse Shape 0 is confirmed"
            ));
        }
    }
    if source_is_mappings {
        for target in &mut targets.beam_targets {
            target.feature_attribute = Some("Dimmer".to_string());
        }
    }
    let omitted_spatial_targets =
        retain_dvc_color_spatial_targets(&mut targets, fixture_refs, source_is_mappings);
    if matches!(generator_id, 21 | 29 | 31 | 35 | 41 | 47 | 48 | 49 | 50)
        && omitted_spatial_targets > 0
    {
        return Err(format!(
            "COLOR MAPPINGS {generator} ID={generator_id} requires every owned BEAMS target to expose a verified color segment; omitted {omitted_spatial_targets} target(s)"
        ));
    }
    if targets.beam_targets.is_empty() && !source_noop {
        return Err(format!(
            "BEAMS resolved to no {generator} targets with a verified color segment or Dimmer feature"
        ));
    }
    let selection_count = targets.ordered_steps.len();
    let beam_count = targets.beam_targets.len();
    let mut approximations = generator_approximations;
    if omitted_spatial_targets > 0 {
        approximations.push(format!(
            "{omitted_spatial_targets} beam target(s) without a verified color segment or Dimmer attribute were omitted"
        ));
    }
    let (period_ms, period_note) = if matches!(
        generator_id,
        21 | 29 | 31 | 35 | 37 | 41 | 47 | 48 | 49 | 50 | 121 | 127 | 128 | 131 | 133 | 134
    ) || shared_mapping_generator_id == 530
    {
        let period_source_family = if source_is_color_mappings {
            "COLOR MAPPINGS"
        } else if source_is_mappings {
            "MAPPINGS"
        } else {
            "COLOR FX"
        };
        dvc_exact_generator_period(effect, period_source_family, generator)?
    } else {
        dvc_move_period(effect, scene, generator, &mut approximations)?
    };
    let (clock_sync, clock_note, clock_warning) = dvc_scene_clock_sync(scene);
    if let Some(clock_warning) = clock_warning {
        approximations.push(clock_warning);
    }
    let recipe_note = match &recipe {
        ColorEffectSpatialRecipe::KnightRider {
            size,
            one_way,
            fading,
            go_outside,
            gradient,
            ..
        } => format!(
            "Size={size}; OneWay={}; Fading={}; GoOutside={}; Gradient={gradient}; evaluator=CKnightRiderEffect corrected analytic geometry",
            u8::from(*one_way),
            u8::from(*fading),
            u8::from(*go_outside)
        ),
        ColorEffectSpatialRecipe::Sweep {
            grayscale,
            vertical_symmetry,
            direction_change,
        } => {
            format!(
                "Grayscale={}; Transform={}; DirectionChange={}; evaluator=CSweepEffect corrected analytic route",
                u8::from(*grayscale),
                if *vertical_symmetry { "Vertical symmetry" } else { "None" },
                u8::from(*direction_change)
            )
        }
        ColorEffectSpatialRecipe::Burst {
            color_width,
            gradient,
            ..
        } => format!(
            "source_ColorWidth={color_width}; source_Gradient={gradient}; evaluator=CBurstEffect corrected analytic radius and cyclic palette"
        ),
        ColorEffectSpatialRecipe::RandomFill {
            grayscale,
            vertical_symmetry,
            rng_seed,
            point_width,
            source_point_height,
            ..
        } => {
            if generator_id == 37 {
                let transform = match params.get(&3).copied() {
                    Some(1.0) => "Vertical symmetry",
                    Some(2.0) => "Horizontal symmetry",
                    _ => "None",
                };
                let rotation = params.get(&4).copied().unwrap_or_default();
                format!("implementation=SyndocalCorrected; evaluator=CRandomFillEffect recovered no-replacement grammar; unavailable_qrand=stable_source_seed; rng_seed={rng_seed}; Grayscale={}; Transform={transform}; Rotation={rotation}; PointWidth={point_width}; source_PointHeight={source_point_height:?}; PointHeight_mode=placed_2d_live; source_raster=100x100; cell_ranking=single_flat_permutation; cell_partition=div_ceil; end_coverage=partial_right_and_bottom_tails_included; transition=continuous_palette_to_palette", u8::from(*grayscale))
            } else {
                format!("implementation=SyndocalCorrected; evaluator=CRandomFillEffect recovered no-replacement grammar; unavailable_qrand=stable_source_seed; rng_seed={rng_seed}; Grayscale={}; Transform={}; PointWidth={point_width}; source_PointHeight={source_point_height:?}; cell_partition=div_ceil; end_coverage=partial_tail_included; transition=continuous_palette_to_palette", u8::from(*grayscale), if *vertical_symmetry { "Vertical symmetry" } else { "None" })
            }
        }
        ColorEffectSpatialRecipe::Sparkle {
            grayscale,
            vertical_symmetry,
            raster_mode,
            rng_seed,
            number,
            lifetime_ms,
            source_lifespan,
            width,
            height,
            ..
        } => {
            let evaluator = if *raster_mode
                == ColorEffectSpatialSparkleRasterMode::TubeFullRasterHeight
            {
                "CTubeEffect shared retained-particle grammar"
            } else {
                "CSparklesEffect recovered retained-particle grammar"
            };
            let raster_height = if *raster_mode
                == ColorEffectSpatialSparkleRasterMode::TubeFullRasterHeight
            {
                "full_100_rows"
            } else {
                "authored_or_strip"
            };
            format!("implementation=SyndocalCorrected; evaluator={evaluator}; unavailable_qrand=stable_source_seed; rng_seed={rng_seed}; Grayscale={}; Transform={}; Number={number}; source_LifeSpan={source_lifespan:?}; lifetime_ms={lifetime_ms:?}; Width={width}; Height={height:?}; raster_height={raster_height}; population=retained; spawn_rate_units=particles_per_40ms; fade=continuous; time_units=milliseconds", u8::from(*grayscale), if *vertical_symmetry { "Vertical symmetry" } else { "None" })
        }
        ColorEffectSpatialRecipe::Spiral {
            grayscale,
            radius,
            arms,
            gradient,
        } => format!(
            "implementation=SyndocalCorrected; evaluator=CSpiralEffect/0x140366240 recovered conical angular phase and per-radius rotation; Grayscale={}; Radius={radius}; Arms={arms}; Gradient={gradient}",
            u8::from(*grayscale)
        ),
        ColorEffectSpatialRecipe::Butterfly {
            grayscale,
            color_width,
            gradient,
            clockwise,
        } => format!(
            "implementation=SyndocalCorrected; evaluator=CButterflyEffect/0x140362EB0 recovered paired 180-degree conical sectors; Grayscale={}; ColorWidth={color_width}; Gradient={gradient}; Clockwise={}",
            u8::from(*grayscale),
            u8::from(*clockwise)
        ),
        ColorEffectSpatialRecipe::Plasma {
            grayscale,
            vertical_symmetry,
            size_x,
            param_x,
            size_y,
            param_y,
            speed_x,
            param_sx,
            speed_y,
            param_sy,
        } => format!(
            "Grayscale={}; Transform={}; SizeX={size_x}; ParamX={param_x}; SizeY={size_y}; ParamY={param_y}; SpeedX={speed_x}; ParamSX={param_sx}; SpeedY={speed_y}; ParamSY={param_sy}",
            u8::from(*grayscale),
            if *vertical_symmetry { "Vertical symmetry" } else { "None" }
        ),
        ColorEffectSpatialRecipe::ColorRainbow {
            grayscale,
            vertical_symmetry,
            color_width,
            angle_degrees,
            gradient,
        } => format!(
            "Grayscale={}; Transform={}; ColorWidth={color_width}; Angle={angle_degrees}; Gradient=raw*100={gradient}",
            u8::from(*grayscale),
            if *vertical_symmetry { "Vertical symmetry" } else { "None" }
        ),
        ColorEffectSpatialRecipe::Rainbow {
            grayscale,
            vertical_symmetry,
            horizontal_symmetry,
            rotation_degrees,
            color_width,
            angle_degrees,
            gradient,
        } => format!(
            "Grayscale={}; Transform={}; Rotation={rotation_degrees}; ColorWidth={color_width}; Angle={angle_degrees}; Gradient={gradient}",
            u8::from(*grayscale),
            if *vertical_symmetry {
                "Vertical symmetry"
            } else if *horizontal_symmetry {
                "Horizontal symmetry"
            } else {
                "None"
            }
        ),
        ColorEffectSpatialRecipe::Perlin {
            grayscale,
            vertical_symmetry,
            horizontal_symmetry,
            rotation_degrees,
            octaves,
            zoom,
            direction_degrees,
            speed,
            amplitude,
        } => format!(
            "Grayscale={}; Transform={}; source_Rotation={rotation_degrees}; source_Octaves={octaves}; source_Zoom={zoom}; source_Direction={direction_degrees} (source 1..100 mapped linearly to 0..360 degrees; spatial-phase-active); source_Speed={speed}; source_Amplitude={amplitude}; evaluator=unified analytic Perlin; palette_wrap=true",
            u8::from(*grayscale),
            if *vertical_symmetry {
                "Vertical symmetry"
            } else if *horizontal_symmetry {
                "Horizontal symmetry"
            } else {
                "None"
            }
        ),
        ColorEffectSpatialRecipe::Grid {
            grayscale,
            size,
            width,
        } => format!(
            "implementation=SyndocalCorrected; evaluator=CGridEffect recovered allocation-free analytic 100x100 raster; Grayscale={}; Size={size}; Width={width}; background=palette0; paint_order=later-wins; time=continuous",
            u8::from(*grayscale)
        ),
        ColorEffectSpatialRecipe::Lines { grayscale, size } => format!(
            "implementation=SyndocalCorrected; evaluator=CLinesEffect recovered allocation-free analytic 100x100 raster; Grayscale={}; Size={size}; integer_stride_retained_when_B_ge_1=true; B_zero=float_fallback; background=palette0; paint_order=later-wins; time=continuous",
            u8::from(*grayscale)
        ),
        ColorEffectSpatialRecipe::Graph {
            grayscale,
            height,
            width,
            pitch,
            frequency,
            amplitude,
            offset,
        } => format!(
            "implementation=SyndocalCorrected; evaluator=CGraphEffect/0x1403638A0 recovered allocation-free analytic 100x100 raster; Grayscale={}; Height={height}; Width={width}; Pitch={pitch}; Frequency={frequency}; Amplitude={amplitude}; Offset={offset}; Height1=corrected_opaque_row; background=palette0; paint_order=later-wins; time=continuous",
            u8::from(*grayscale)
        ),
        ColorEffectSpatialRecipe::Rain {
            grayscale,
            rng_seed,
            speed,
            width,
            height,
            number,
            trail,
        } => format!(
            "implementation=SyndocalCorrected; evaluator=CRainEffect/0x140365660 recovered fixed 100x100 falling-particle raster; unavailable_qrand=stable_source_seed; rng_seed={rng_seed}; Grayscale={}; Speed={speed}; Width={width}; Height={height}; Number={number}; Trail={trail}; particle_table=first_100_pairs; vertical_wrap=true; horizontal_wrap=false; paint_order=later-wins_replacement; time=continuous; qGray=post-raster",
            u8::from(*grayscale)
        ),
        ColorEffectSpatialRecipe::Bounce {
            grayscale,
            rng_seed,
            item,
            shape,
            number,
            size,
            speed,
            collide,
            fill,
            points,
        } => format!(
            "implementation=SyndocalCorrected; evaluator=CBounceEffect retained update-first 100x100 raster; unavailable_qrand=stable_source_seed; rng_seed={rng_seed}; Grayscale={}; Item={item:?}; Shape={shape}; Number={number}; Size={size}; Speed={speed}; Collide={}; Fill={}; Points={points}; random_table=q15_even_lanes_5000; generation0=update_first; boundary=no_clamp_with_source_W_H_swap; collision=source_order_symmetric_impulse_dist2_zero_skip; shape0=XEEL_U+E900_even_odd_contours; points=closed_path; raster_edge=deterministic_pixel_center_corrected_not_Qt_antialias_SourceExact; palette=background0_foreground_1_plus_item_mod_N_minus_1; paint_order=later_wins; qGray=post_raster",
            u8::from(*grayscale),
            u8::from(*collide),
            u8::from(*fill)
        ),
        ColorEffectSpatialRecipe::Fire {
            grayscale,
            rng_seed,
            flames,
            width,
            height,
            hotspot,
        } => format!(
            "implementation=SyndocalCorrected; evaluator=CFireEffect recovered 102-row in-place heat raster; unavailable_qrand=stable_source_seed; rng_seed={rng_seed}; Grayscale={}; Flames={flames}; Width={width}; source_Height={height}; source_Height_evaluator_dead=true; corrected_Height_mode=live_heat_cutoff_rescale; Hotspot={hotspot}; random_table=q15_even_lanes_5000; injection=row1; diffusion=x-major_y-ascending_flattened_edge_spill; output_y=101-y; palette_lut=authored_nonwrapping_256; qGray=post-lut",
            u8::from(*grayscale)
        ),
        ColorEffectSpatialRecipe::Explosion {
            grayscale,
            rng_seed,
            shape,
            explosion_number,
            explosion_size,
            particle_number,
            particle_size,
            particle_life,
            trail_size,
            gravity,
        } => format!(
            "implementation=SyndocalCorrected; evaluator=CExplosionEffect retained-particle 100x100 raster; Grayscale={}; rng_seed={rng_seed}; Shape={shape}; ExplosionNumber={explosion_number}; ExplosionSize={explosion_size}; Particles={particle_number}; ParticleSize={particle_size}; Life={particle_life}; Trail={trail_size}; Gravity={gravity}; source_green_lifetime_corrected_to_alpha=true; children_before_parent=true; random_pairs=5000; frame_cap=750; qGray=post-raster",
            u8::from(*grayscale)
        ),
        ColorEffectSpatialRecipe::Starfield {
            grayscale,
            rng_seed,
            shape,
            particles,
            size,
            trail,
            rotation,
        } => format!(
            "implementation=SyndocalCorrected; evaluator=CStarfieldEffect retained-particle 100x100 raster; Grayscale={}; rng_seed={rng_seed}; Shape={shape}; Particles={particles}; Size={size}; Trail={trail}; Rotation={rotation}; palette_endpoint_clamp=true; source_green_lifetime_corrected_to_alpha=true; children_before_parent=true; random_pairs=5000; frame_cap=750; qGray=post-raster",
            u8::from(*grayscale)
        ),
    };
    let mut placement = mapping_rectangle
        .map(|rectangle| {
            dvc_color_spatial_placement(
                rectangle,
                &placement_label,
                &targets.beam_targets,
                fixture_refs,
            )
        })
        .transpose()?;
    if matches!(
        generator_id,
        21 | 29 | 31 | 35 | 37 | 41 | 47 | 48 | 49 | 50
    ) || matches!(shared_mapping_generator_id, 522..=529)
    {
        let transform = dvc_param(&params, 3, &format!("{mapping_family_label} Transform"))?;
        if !matches!(transform, 0.0 | 1.0 | 2.0) {
            return Err(format!(
                "{mapping_family_label} Transform PARAM 3 must be None(0), Vertical symmetry(1), or Horizontal symmetry(2), found {transform}"
            ));
        }
        let rotation = dvc_integer_range_param(
            &params,
            4,
            &format!("{mapping_family_label} Rotation"),
            0,
            360,
        )?;
        if let Some(placement) = placement.as_mut() {
            placement.vertical_symmetry = transform == 1.0;
            placement.horizontal_symmetry = transform == 2.0;
            placement.raster_rotation_degrees = rotation;
        } else if !source_noop {
            return Err(format!(
                "{placement_label} requires a positive Rectangle placement for owned BEAMS"
            ));
        }
    }
    let mapping_recipe = source_is_mappings;
    let source_family = if source_is_color_mappings {
        "Color Mappings"
    } else if mapping_recipe {
        "Mappings"
    } else {
        "Colour FX"
    };
    if source_noop {
        let rectangle = source_mapping_rectangle
            .as_ref()
            .expect("COLOR MAPPINGS always parses its Rectangle before preserving a no-op");
        return Ok(ConvertedDvcEffect {
            target: None,
            generator,
            note: format!(
                "source_family={source_family}; source no-op preserved: Daslight BEAMS contains zero targets and no external SELECTIONS; palette_source=PARAM TYPE=4 ID=1/COLORS/COLOR@VAL; palette_colors={}; Rectangle=({},{},{},{},{}); {period_note}; {clock_note}; {recipe_note}; no runtime effect was created",
                stops.len(),
                rectangle.x,
                rectangle.y,
                rectangle.sx,
                rectangle.sy,
                rectangle.angle_degrees,
            ),
            approximations,
            warnings: Vec::new(),
        });
    }
    normalize_dvc_spatial_recipe(&mut recipe, selection_count, placement.is_some());
    let request = ColorEffectRequest {
        label: format!("{scene_name} ({generator})"),
        fixture_ids: targets.fixture_ids,
        target_group_ids: Vec::new(),
        stops,
        algorithm: ColorEffectAlgorithm::Sequence,
        interpolation: ColorEffectInterpolation::Rgb,
        period_ms,
        clock_sync,
        phase: 0.0,
        fixture_spread: 0.0,
        blend_mode: if mapping_recipe {
            EffectBlendMode::Multiply
        } else {
            EffectBlendMode::Override
        },
        spatial_pattern: Some(Box::new(ColorEffectSpatialPattern {
            recipe,
            parameter_model_version: COLOR_EFFECT_SPATIAL_PARAMETER_MODEL_VERSION,
            beam_targets: targets.beam_targets,
            placement,
        })),
    };
    let note = format!(
        "source_family={source_family}; palette_source=PARAM TYPE=4 ID=1/COLORS/COLOR@VAL; palette_colors={}; beams={beam_count}; selections={selection_count}; {period_note}; {clock_note}; {recipe_note}",
        request.stops.len()
    );
    engine::validate_color_effect_request(&request).map_err(|error| {
        format!("confirmed {generator} parameters are not representable: {error}")
    })?;
    Ok(ConvertedDvcEffect {
        target: Some(CueEffectTarget {
            effect_id,
            enabled: true,
            params: Some(EffectParamsSnapshot::Color(request)),
            transition_ms: None,
        }),
        generator,
        note,
        approximations,
        warnings: Vec::new(),
    })
}

fn convert_dvc_chaser_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    generator_id: u16,
    effect_id: u64,
    profiles: &[ParsedProfile],
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    let generator = match generator_id {
        321 => "Chaser #1",
        322 => "Chaser #2",
        323 => "Chaser #3",
        324 => "Chaser #4",
        _ => "Chaser random",
    };
    let params = dvc_effect_params(effect)?;
    let expected_params: &[u16] = match generator_id {
        321 | 323 | 324 => &[10, 11, 12],
        322 => &[10],
        _ => &[11, 12, 13, 14, 15],
    };
    require_exact_dvc_params(&params, expected_params)?;
    let expected_types: &[(u16, u16)] = match generator_id {
        321 | 323 | 324 => &[(10, 2), (11, 2), (12, 0)],
        322 => &[(10, 2)],
        _ => &[(11, 2), (12, 0), (13, 1), (14, 0), (15, 0)],
    };
    require_exact_dvc_param_types(effect, expected_types)?;

    // Validate the complete serialized class schema before recognizing an
    // empty BEAMS rack as a source no-op. A malformed effect must not bypass
    // fail-closed validation merely because it currently targets nothing.
    let fading = dvc_binary_param(&params, if generator_id == 322 { 10 } else { 11 }, "Fading")?;
    let one_way = if matches!(generator_id, 321 | 323 | 324) {
        Some(dvc_binary_param(&params, 10, "One Way Only")?)
    } else {
        None
    };
    let pixels_on = if generator_id == 322 {
        1
    } else {
        dvc_integer_range_param(&params, 12, "Nb pixels on", 0, 1000)? as u64
    };
    let (flash_percent, random_sequence, authored_random_cycle_count) = if generator_id == 325 {
        let flash_percent = dvc_param(&params, 13, "Flash")?;
        if !flash_percent.is_finite() || !(0.0..=100.0).contains(&flash_percent) {
            return Err(format!(
                "Flash must be within 0..100, found {flash_percent}"
            ));
        }
        let random_sequence = dvc_param(&params, 14, "Random sequence")?;
        if !random_sequence.is_finite()
            || !(0.0..=255.0).contains(&random_sequence)
            || random_sequence.fract().abs() > f64::EPSILON
        {
            return Err(format!(
                "Random sequence PARAM 14 must be an integer from 0 to 255, found {random_sequence}"
            ));
        }
        let cycles = dvc_positive_integer_param(&params, 15, "Nb cycles")?;
        if cycles > u8::MAX as u64 {
            return Err(format!(
                "Nb cycles PARAM 15 must be an integer from 1 to 255, found {cycles}"
            ));
        }
        (flash_percent, random_sequence as u64, cycles as u8)
    } else {
        (100.0, 1, 1)
    };

    let targets = dvc_rack_targets(rack, fixture_refs)?;
    if targets.ordered_steps.is_empty() {
        return Ok(ConvertedDvcEffect {
            target: None,
            generator,
            note: format!(
                "source no-op preserved: Daslight BEAMS contains zero targets; PARAM IDs {expected_params:?}, TYPEs, and domains validated; no runtime effect was created"
            ),
            approximations: Vec::new(),
            warnings: Vec::new(),
        });
    }
    if pixels_on == 0 {
        return Err(format!(
            "{generator} Nb pixels on PARAM 12=0 is valid Daslight schema but cannot be represented for a populated Syndocal Chaser target; remains fail-closed"
        ));
    }

    let feature_spec = dvc_rack_feature_spec(rack, profiles)?;
    let original_step_count = targets.ordered_steps.len();
    if original_step_count == 0 {
        return Err("BEAMS resolved to no Chaser steps".to_string());
    }

    let mut approximations = Vec::new();
    match generator_id {
        321 => approximations.push(
            "Daslight Chaser #1 evaluator 0x140376540 is not frame-equivalent to Syndocal's Forward/Bounce/overlap compatibility route"
                .to_string(),
        ),
        323 => approximations.push(
            "Daslight Chaser #3 evaluator 0x1403775F0 is converted to ordered outside-in symmetric beam-pair steps; the recovered 40 ms frame grid is replaced by the continuous Syndocal Chaser clock"
                .to_string(),
        ),
        324 => approximations.push(
            "Daslight Chaser #4 evaluator 0x140378400 is converted to ordered center-out symmetric beam-pair steps; the recovered 40 ms frame grid is replaced by the continuous Syndocal Chaser clock"
                .to_string(),
        ),
        325 => {}
        _ => {}
    }
    let mut ordered_steps = targets.ordered_steps;
    let mut ordered_beam_steps = vec![Vec::new(); ordered_steps.len()];
    let mut feature_attributes = Vec::<String>::new();
    let mut seen_feature_attributes = HashSet::<String>::new();
    let mut omitted_feature_targets = 0_usize;
    for target in &targets.beam_targets {
        let Some(feature_attribute) =
            dvc_beam_feature_attribute(target, &feature_spec, profiles, fixture_refs)
        else {
            omitted_feature_targets = omitted_feature_targets.saturating_add(1);
            continue;
        };
        let normalized_feature = normalize_dvc_attribute(&feature_attribute);
        if seen_feature_attributes.insert(normalized_feature) {
            feature_attributes.push(feature_attribute.clone());
        }
        if let Some(step) = ordered_beam_steps.get_mut(target.selection_index as usize) {
            step.push(EffectBeamTarget {
                fixture_id: target.fixture_id,
                beam_index: target.beam_index,
                selection_index: target.selection_index,
                feature_attribute,
            });
        }
    }
    if feature_attributes.is_empty() {
        return Err(format!(
            "BEAMS resolved to no Chaser targets exposing PRESET type {}",
            feature_spec.preset_type
        ));
    }
    if omitted_feature_targets > 0 {
        approximations.push(format!(
            "{omitted_feature_targets} beam target(s) without PRESET type {} were omitted",
            feature_spec.preset_type
        ));
    }
    if matches!(generator_id, 323 | 324) {
        ordered_beam_steps =
            dvc_symmetric_chaser_beam_steps(ordered_beam_steps, generator_id == 324);
        ordered_steps = vec![Vec::new(); ordered_beam_steps.len()];
        if ordered_steps.len() == 1 {
            ordered_steps.push(Vec::new());
            ordered_beam_steps.push(ordered_beam_steps[0].clone());
            approximations.push(
                "the single symmetric beam pair was duplicated as an identical second step to preserve constant output within the Chaser engine's two-step minimum"
                    .to_string(),
            );
        }
    } else if ordered_steps.len() == 1 && generator_id != 322 {
        ordered_steps.push(Vec::new());
        ordered_beam_steps.push(Vec::new());
        approximations.push(
            "single target selection requires an added blackout gap in the Chaser engine"
                .to_string(),
        );
    }
    let maximum_pixels = ordered_steps.len().min(64) as u64;
    let active_step_count = pixels_on.min(maximum_pixels) as u16;
    if pixels_on > maximum_pixels {
        approximations.push(format!(
            "Nb pixels on {pixels_on} was clamped to the Chaser engine limit {maximum_pixels}"
        ));
    }

    let (direction, duty_cycle, generator_note, random_seed, random_cycle_count) = if matches!(
        generator_id,
        321 | 323 | 324
    ) {
        let one_way = one_way.unwrap_or(false);
        (
            if one_way {
                ChaserDirection::Forward
            } else {
                ChaserDirection::Bounce
            },
            1.0,
            format!(
                "param10=One Way Only({}); param11=Fading({}); topology={}",
                u8::from(one_way),
                u8::from(fading),
                match generator_id {
                    323 => "symmetric outside-in pairs",
                    324 => "symmetric center-out pairs",
                    _ => "source beam order",
                }
            ),
            (effect_id % u32::MAX as u64).max(1),
            1,
        )
    } else if generator_id == 322 {
        (
            ChaserDirection::BuildUpDown,
            1.0,
            format!(
                "param10=Fading({}); build-up then source-order clear cycle confirmed from Daslight LIVE DMX Levels",
                u8::from(fading)
            ),
            (effect_id % u32::MAX as u64).max(1),
            1,
        )
    } else {
        let mut duty_cycle = (flash_percent / 100.0) as f32;
        if duty_cycle <= 0.0 {
            duty_cycle = 0.001;
            approximations
                .push("Flash 0% was raised to the minimum non-zero Chaser duty cycle".to_string());
        }
        (
            ChaserDirection::Random,
            duty_cycle,
            format!(
                "random_sequence={random_sequence}; flash_percent={flash_percent}; cycles={authored_random_cycle_count}; implementation=SyndocalCorrected; correction=unavailable process-global qrand replaced by deterministic reload-stable permutation series"
            ),
            random_sequence,
            authored_random_cycle_count,
        )
    };

    let generator_slots = match generator_id {
        322 => ordered_steps.len().saturating_mul(2),
        323 | 324 if direction == ChaserDirection::Bounce && ordered_steps.len() > 1 => {
            ordered_steps.len().saturating_mul(2).saturating_sub(2)
        }
        325 => ordered_steps
            .len()
            .saturating_mul(random_cycle_count as usize),
        _ => ordered_steps.len(),
    };
    let (step_duration_ms, free_run_note) =
        dvc_chaser_step_duration(effect, scene, generator_slots, &mut approximations)?;
    let (clock_sync, clock_note, clock_warning) = dvc_scene_clock_sync(scene);
    if let Some(clock_warning) = clock_warning {
        approximations.push(clock_warning);
    }

    let steps = ordered_steps
        .into_iter()
        .zip(ordered_beam_steps)
        .map(|(_fixture_ids, beam_targets)| ChaserStep {
            fixture_ids: Vec::new(),
            target_group_ids: Vec::new(),
            beam_targets,
            level: u16::MAX,
        })
        .collect::<Vec<_>>();
    let features = feature_attributes
        .iter()
        .map(|attribute| ChaserFeature {
            attribute: attribute.clone(),
            low: feature_spec.low,
            high: feature_spec.high,
        })
        .collect::<Vec<_>>();
    let request = ChaserEffectRequest {
        label: format!("{scene_name} ({generator})"),
        steps,
        features,
        step_duration_ms,
        clock_sync,
        direction,
        wings: 1,
        active_step_count,
        duty_cycle,
        overlap: if fading { 1.0 } else { 0.0 },
        phase: 0.0,
        fixture_spread: 0.0,
        random_seed,
        random_cycle_count,
        blend_mode: EffectBlendMode::Override,
    };
    engine::validate_chaser_effect_request(&request)
        .map_err(|error| format!("confirmed Chaser parameters are not representable: {error}"))?;

    let feature_note = feature_attributes.join(",");
    let note = format!(
        "features={feature_note}; preset_type={}; preset_range={}..{}; preset_source={}; selections={original_step_count}; pixels_on={pixels_on}; {free_run_note}; {clock_note}; {generator_note}",
        feature_spec.preset_type,
        feature_spec.low,
        feature_spec.high,
        feature_spec.source,
    );
    Ok(ConvertedDvcEffect {
        target: Some(CueEffectTarget {
            effect_id,
            enabled: true,
            params: Some(EffectParamsSnapshot::Chaser(request)),
            transition_ms: None,
        }),
        generator,
        note,
        approximations,
        warnings: Vec::new(),
    })
}

/// Collapse the saved one-dimensional beam order into the mirrored pair order
/// recovered from `CChaserType3Effect` and `CChaserType4Effect`.
///
/// Type 3 visits both ends first and walks toward the centre. Type 4 uses the
/// exact reverse order. Odd target counts retain the centre beam once rather
/// than manufacturing a duplicate target.
fn dvc_symmetric_chaser_beam_steps(
    mut source_steps: Vec<Vec<EffectBeamTarget>>,
    center_out: bool,
) -> Vec<Vec<EffectBeamTarget>> {
    let source_count = source_steps.len();
    let pair_count = source_count.div_ceil(2);
    let mut paired_steps = Vec::with_capacity(pair_count);
    for left in 0..pair_count {
        let right = source_count - 1 - left;
        let mut pair = std::mem::take(&mut source_steps[left]);
        if right != left {
            pair.extend(std::mem::take(&mut source_steps[right]));
        }
        paired_steps.push(pair);
    }
    if center_out {
        paired_steps.reverse();
    }
    paired_steps
}

fn convert_dvc_custom_curve_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    effect_id: u64,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    require_exact_custom_attributes(effect, &["TYPE", "ID", "DURATION"], "Custom EFFECT")?;
    let (points, phasing) = dvc_custom_curve_params(effect)?;
    let duration_text = required_attribute(effect, "DURATION", "Custom EFFECT")?;
    let period_ms = duration_text
        .parse::<u32>()
        .map_err(|error| format!("Custom DURATION must be an unsigned 32-bit integer: {error}"))?;
    if period_ms < u32::from(DASLIGHT_CURVE_SAMPLE_MS) {
        return Err(format!(
            "Custom DURATION must be within {}..{}, found {period_ms}",
            DASLIGHT_CURVE_SAMPLE_MS,
            u32::MAX
        ));
    }

    if element_children(rack).any(|node| node.has_tag_name("SELECTIONS")) {
        return Err("Custom external SELECTIONS remain fail-closed".to_string());
    }
    let direct_beams = element_children(rack)
        .filter(|node| node.has_tag_name("BEAMS"))
        .collect::<Vec<_>>();
    if direct_beams.len() != 1 {
        return Err(format!(
            "Custom generator rack must contain exactly one direct BEAMS container, found {}",
            direct_beams.len()
        ));
    }
    let beams = direct_beams[0];
    require_exact_custom_attributes(beams, &["NB"], "Custom BEAMS")?;
    let beam_nodes = element_children(beams).collect::<Vec<_>>();
    if let Some(unexpected) = beam_nodes.iter().find(|node| !node.has_tag_name("BEAM")) {
        return Err(format!(
            "Custom BEAMS contains unexpected <{}> element",
            unexpected.tag_name().name()
        ));
    }
    let declared_beams = required_attribute(beams, "NB", "BEAMS")?
        .parse::<usize>()
        .map_err(|error| format!("Custom BEAMS NB is invalid: {error}"))?;
    if declared_beams != beam_nodes.len() {
        return Err(format!(
            "Custom BEAMS declares {declared_beams} entries but contains {}",
            beam_nodes.len()
        ));
    }
    for (index, beam) in beam_nodes.iter().enumerate() {
        require_exact_custom_attributes(
            *beam,
            &["FIXTURE", "BEAMID", "IDSELECTION"],
            &format!("Custom BEAM {}", index + 1),
        )?;
        required_attribute(*beam, "IDSELECTION", "Custom BEAM")?
            .parse::<u32>()
            .map_err(|error| {
                format!("Custom BEAM {} IDSELECTION is invalid: {error}", index + 1)
            })?;
    }

    let mut targets = dvc_rack_targets(rack, fixture_refs)?;
    let authored_beam_count = targets.beam_targets.len();
    let incompatible_targets = retain_dvc_dimmer_targets(&mut targets, fixture_refs);
    let dropped_beam_count = authored_beam_count.saturating_sub(targets.beam_targets.len());
    if incompatible_targets > 0 || dropped_beam_count > 0 {
        return Err(format!(
            "Custom BEAMS include {dropped_beam_count} beam target(s) across {incompatible_targets} fixture target(s) without a confirmed Dimmer attribute"
        ));
    }
    if targets.fixture_ids.is_empty() {
        return Err("BEAMS resolved to no Custom Dimmer fixture targets".to_string());
    }
    let beam_targets = dvc_effect_beam_targets(&targets, "Dimmer");
    if beam_targets.is_empty() {
        return Err("Custom requires explicit ordered Dimmer BEAMS".to_string());
    }

    let (clock_sync, clock_note, clock_warning) = dvc_scene_clock_sync(scene);
    let mut approximations = vec![
        "Syndocal evaluates the recovered right-point easing directly at continuous authored progress, removing Daslight's 40 ms storage/timer granularity and duration-remainder loss while preserving point order, values, easing, and adjacent-target lag"
            .to_string(),
    ];
    if let Some(clock_warning) = clock_warning {
        approximations.push(clock_warning);
    }
    let request = LfoEffectRequest {
        label: format!("{scene_name} (Custom)"),
        fixture_ids: targets.fixture_ids,
        target_group_ids: Vec::new(),
        attribute: "Dimmer".to_string(),
        video_targets: Vec::new(),
        shape: LfoShape::DaslightCustom,
        period_ms: u64::from(period_ms),
        clock_sync,
        low: 0,
        high: u16::MAX,
        phase: 0.0,
        fixture_spread: 0.0,
        beam_targets,
        blend_mode: EffectBlendMode::Override,
        daslight_curve: None,
        daslight_custom_curve: Some(DaslightCustomCurveSource {
            points,
            phasing,
            sample_ms: DASLIGHT_CURVE_SAMPLE_MS,
        }),
    };
    engine::validate_lfo_effect_request(&request)
        .map_err(|error| format!("confirmed Custom parameters are not representable: {error}"))?;
    let note = format!(
        "feature=Dimmer; shape=Custom; evaluator=CCustomCurveEffect@0x14036F1F0; implementation=SyndocalCorrected; duration_ms={period_ms}; recovered_sample_ms={DASLIGHT_CURVE_SAMPLE_MS}; runtime_time=continuous; points={}; right_point_easing=Linear|InCubic|OutCubic|InOutCubic|OutInCubic; fixture_lag=index*{phasing}; correction_reason=storage/timer granularity and duration remainder must not alter authored progress; {clock_note}",
        request
            .daslight_custom_curve
            .as_ref()
            .map_or(0, |source| source.points.len())
    );
    Ok(ConvertedDvcEffect {
        target: Some(CueEffectTarget {
            effect_id,
            enabled: true,
            params: Some(EffectParamsSnapshot::Lfo(request)),
            transition_ms: None,
        }),
        generator: "Custom",
        note,
        approximations,
        warnings: Vec::new(),
    })
}

fn dvc_custom_curve_params(
    effect: Node<'_, '_>,
) -> Result<(Vec<DaslightCustomCurvePoint>, f32), String> {
    let direct_params = element_children(effect)
        .filter(|node| node.has_tag_name("PARAMS"))
        .collect::<Vec<_>>();
    if direct_params.len() != 1 {
        return Err(format!(
            "Custom generator must contain exactly one direct PARAMS container, found {}",
            direct_params.len()
        ));
    }
    let params_node = direct_params[0];
    require_exact_custom_attributes(params_node, &["NB"], "Custom PARAMS")?;
    let param_nodes = element_children(params_node).collect::<Vec<_>>();
    if let Some(unexpected) = param_nodes.iter().find(|node| !node.has_tag_name("PARAM")) {
        return Err(format!(
            "Custom PARAMS contains unexpected <{}> element",
            unexpected.tag_name().name()
        ));
    }
    let declared = required_attribute(params_node, "NB", "PARAMS")?
        .parse::<usize>()
        .map_err(|error| format!("PARAMS NB is invalid: {error}"))?;
    if declared != 2 || param_nodes.len() != 2 {
        return Err(format!(
            "Custom PARAMS must declare and contain exactly 2 entries, found NB={declared} and {} entries",
            param_nodes.len()
        ));
    }

    let mut points = None;
    let mut phasing = None;
    let mut seen = HashSet::new();
    for param in param_nodes {
        let id = required_attribute(param, "ID", "PARAM")?
            .parse::<u16>()
            .map_err(|error| format!("PARAM ID is invalid: {error}"))?;
        if !seen.insert(id) {
            return Err(format!("Custom PARAM {id} is duplicated"));
        }
        let param_type = required_attribute(param, "TYPE", "PARAM")?
            .parse::<u16>()
            .map_err(|error| format!("Custom PARAM {id} TYPE is invalid: {error}"))?;
        match id {
            1 => {
                require_exact_custom_attributes(param, &["TYPE", "ID"], "Custom Points PARAM 1")?;
                if param_type != 5 {
                    return Err(format!(
                        "Custom Points PARAM 1 must use TYPE=5, found TYPE={param_type}"
                    ));
                }
                if param.attribute("VAL").is_some() {
                    return Err("Custom Points PARAM 1 must not contain VAL".to_string());
                }
                let children = element_children(param).collect::<Vec<_>>();
                if children.len() != 1 || !children[0].has_tag_name("POINTS") {
                    return Err(
                        "Custom Points PARAM 1 must contain exactly one POINTS element".to_string(),
                    );
                }
                let points_node = children[0];
                require_exact_custom_attributes(points_node, &["NB"], "Custom POINTS")?;
                let point_nodes = element_children(points_node).collect::<Vec<_>>();
                if let Some(unexpected) =
                    point_nodes.iter().find(|node| !node.has_tag_name("POINT"))
                {
                    return Err(format!(
                        "Custom POINTS contains unexpected <{}> element",
                        unexpected.tag_name().name()
                    ));
                }
                let declared_points = required_attribute(points_node, "NB", "POINTS")?
                    .parse::<usize>()
                    .map_err(|error| format!("Custom POINTS NB is invalid: {error}"))?;
                if declared_points != point_nodes.len() || !(2..=255).contains(&declared_points) {
                    return Err(format!(
                        "Custom POINTS must declare and contain between 2 and 255 points, found NB={declared_points} and {} points",
                        point_nodes.len()
                    ));
                }
                let mut parsed = Vec::with_capacity(point_nodes.len());
                let mut previous_x = None;
                for (index, point) in point_nodes.into_iter().enumerate() {
                    require_exact_custom_attributes(
                        point,
                        &["X", "Y"],
                        &format!("Custom POINT {}", index + 1),
                    )?;
                    if element_children(point).next().is_some() {
                        return Err(format!(
                            "Custom POINT {} must not contain child elements",
                            index + 1
                        ));
                    }
                    let x = required_attribute(point, "X", "POINT")?
                        .parse::<f32>()
                        .map_err(|error| {
                            format!("Custom POINT {} X is invalid: {error}", index + 1)
                        })?;
                    let raw_y = required_attribute(point, "Y", "POINT")?
                        .parse::<f32>()
                        .map_err(|error| {
                            format!("Custom POINT {} Y is invalid: {error}", index + 1)
                        })?;
                    if !x.is_finite() || !(0.0..=1.0).contains(&x) {
                        return Err(format!(
                            "Custom POINT {} X must be finite and within 0..1, found {x}",
                            index + 1
                        ));
                    }
                    if previous_x.is_some_and(|previous| x <= previous) {
                        return Err(
                            "Custom POINT X values must be strictly increasing in source order"
                                .to_string(),
                        );
                    }
                    let easing_code = (raw_y / 10.0).floor();
                    let value = raw_y - easing_code * 10.0;
                    if !raw_y.is_finite()
                        || !(0.0..=4.0).contains(&easing_code)
                        || easing_code.fract() != 0.0
                        || !(0.0..=1.0).contains(&value)
                    {
                        return Err(format!(
                            "Custom POINT {} Y must encode easing 0..4 and normalized value, found {raw_y}",
                            index + 1
                        ));
                    }
                    parsed.push(DaslightCustomCurvePoint { x, raw_y });
                    previous_x = Some(x);
                }
                points = Some(parsed);
            }
            2 => {
                require_exact_custom_attributes(
                    param,
                    &["TYPE", "ID", "VAL"],
                    "Custom Phasing PARAM 2",
                )?;
                if param_type != 1 || element_children(param).next().is_some() {
                    return Err(format!(
                        "Custom Phasing PARAM 2 must use TYPE=1 without child elements, found TYPE={param_type}"
                    ));
                }
                let value = required_attribute(param, "VAL", "PARAM 2")?
                    .parse::<f32>()
                    .map_err(|error| format!("Custom Phasing is invalid: {error}"))?;
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(format!("Custom Phasing must be within 0..1, found {value}"));
                }
                phasing = Some(value);
            }
            _ => {
                return Err(format!(
                    "Custom expected PARAM IDs [1, 2], found unexpected ID {id}"
                ))
            }
        }
    }
    if seen != HashSet::from([1, 2]) {
        return Err("Custom requires PARAM IDs [1, 2]".to_string());
    }
    Ok((points.unwrap_or_default(), phasing.unwrap_or_default()))
}

fn require_exact_custom_attributes(
    node: Node<'_, '_>,
    expected: &[&str],
    label: &str,
) -> Result<(), String> {
    let mut actual = node
        .attributes()
        .map(|attribute| attribute.name())
        .collect::<Vec<_>>();
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    if actual != expected {
        return Err(format!(
            "{label} attributes must be exactly {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn convert_dvc_additional_curve_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    generator_id: u16,
    effect_id: u64,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    let (generator, shape, evaluator) = match generator_id {
        5 => ("Ramp", LfoShape::Ramp, "CRampEffect@0x14036FAE0"),
        6 => ("Random", LfoShape::Random, "CRandomEffect@0x14036FE90"),
        8 => ("Sinus3", LfoShape::Sinus3, "CSinus3Effect@0x140370070"),
        11 => (
            "Tangeant",
            LfoShape::Tangeant,
            "CTangeantEffect@0x140370760",
        ),
        12 => (
            "Triangle",
            LfoShape::Triangle,
            "CTriangleEffect@0x140370930",
        ),
        _ => return Err(format!("unsupported additional Curve ID {generator_id}")),
    };
    let params = dvc_curve_effect_params(effect, generator)?;
    let rate = dvc_param(&params, 1, "Rate")?;
    let size = dvc_param(&params, 2, "Size")?;
    let phase = dvc_param(&params, 3, "Phase")?;
    let offset = dvc_param(&params, 4, "Offset")?;
    let phasing = dvc_param(&params, 5, "Phasing")?;
    let duration_ms = effect
        .attribute("DURATION")
        .ok_or_else(|| format!("{generator} EFFECT is missing DURATION"))?
        .parse::<f64>()
        .map_err(|error| format!("{generator} DURATION is invalid: {error}"))?;
    if !duration_ms.is_finite()
        || duration_ms < f64::from(DASLIGHT_CURVE_SAMPLE_MS)
        || duration_ms > u64::MAX as f64
    {
        return Err(format!(
            "DURATION must be a finite Curve source buffer of at least {DASLIGHT_CURVE_SAMPLE_MS} ms, found {duration_ms}"
        ));
    }
    let period_ms = duration_ms.round() as u64;
    let rng_seed =
        (generator_id == 6).then(|| dvc_corrected_rng_seed(scene, rack, effect, generator_id));
    let (low, high, source_range) = match generator_id {
        5 => {
            let raw_low = offset + size * 0.5 - 0.5;
            let raw_high = offset + size * 1.5 - 0.5;
            (
                normalized_dmx(raw_low),
                normalized_dmx(raw_high),
                format!("{raw_low:.3}..{raw_high:.3}"),
            )
        }
        6 => {
            let raw_low = size * offset;
            let raw_high = size * (offset + 0.99);
            (
                normalized_dmx(raw_low),
                normalized_dmx(raw_high),
                format!("{raw_low:.3}..{raw_high:.3}"),
            )
        }
        8 | 12 => {
            let raw_low = offset;
            let raw_high = offset + size;
            (
                normalized_dmx(raw_low),
                normalized_dmx(raw_high),
                format!("{raw_low:.3}..{raw_high:.3}"),
            )
        }
        11 => (
            0,
            u16::MAX,
            "unbounded tangent before native clamp".to_string(),
        ),
        _ => unreachable!(),
    };

    let mut targets = dvc_rack_targets(rack, fixture_refs)?;
    let incompatible_dimmer_targets = retain_dvc_dimmer_targets(&mut targets, fixture_refs);
    if targets.fixture_ids.is_empty() {
        return Err(format!("BEAMS resolved to no {generator} fixture targets"));
    }
    let correction = if generator_id == 6 {
        "Syndocal replaces the non-serialized process-global qrand table with a stable source-identity seed and evaluates the recovered intentional random steps without a 40 ms source-buffer hold"
    } else {
        "Syndocal evaluates the recovered generator continuously instead of holding Daslight's 40 ms work samples; authored Rate/Phase/Size/Offset, clamp regions, and beam order are preserved"
    };
    let mut approximations = vec![correction.to_string()];
    if incompatible_dimmer_targets > 0 {
        approximations.push(format!(
            "{incompatible_dimmer_targets} fixture target(s) without a Dimmer attribute or addressable color beam were omitted"
        ));
    }
    let (clock_sync, clock_note, clock_warning) = dvc_scene_clock_sync(scene);
    if let Some(clock_warning) = clock_warning {
        approximations.push(clock_warning);
    }
    let beam_targets = dvc_effect_beam_targets(&targets, "Dimmer");
    let request = LfoEffectRequest {
        label: format!("{scene_name} ({generator})"),
        fixture_ids: targets.fixture_ids,
        target_group_ids: Vec::new(),
        attribute: "Dimmer".to_string(),
        video_targets: Vec::new(),
        shape,
        period_ms,
        clock_sync,
        low,
        high,
        phase: phase as f32,
        fixture_spread: phasing as f32,
        beam_targets,
        blend_mode: EffectBlendMode::Override,
        daslight_curve: Some(DaslightCurveSource {
            rate: rate as f32,
            size: size as f32,
            offset: offset as f32,
            sample_ms: DASLIGHT_CURVE_SAMPLE_MS,
            rng_seed,
        }),
        daslight_custom_curve: None,
    };
    let evaluator_detail = match generator_id {
        5 => "centered=(Rate/2*progress-Phase)-floor(Rate/2*progress-Phase+0.5); source=clamp(Offset+centered*Size+Size-0.5,0,1)",
        6 => "step=floor(abs(Rate*pi*progress-Phase*TAU)); recovered_table=400*qrand()%100; source=clamp(Size*(bucket/100+Offset),0,1)",
        8 => "source=clamp(sin(Rate*pi*progress-Phase*TAU)^3*Size/2+Offset+Size/2,0,1)",
        11 => "source=clamp(tan(Rate*pi*progress-Phase*TAU)*Size/2+Offset+Size/2,0,1)",
        12 => "source=clamp(Triangle(Rate/2*progress-Phase+0.75)*Size+Offset,0,1)",
        _ => unreachable!(),
    };
    let correction_reason = if generator_id == 6 {
        "qrand process history is absent from DVC and timer granularity must not add an extra source-buffer hold"
    } else {
        "timer granularity must not stair-step output"
    };
    let rng_note = rng_seed
        .map(|seed| format!("; unavailable_qrand=stable_source_seed; rng_seed={seed}"))
        .unwrap_or_default();
    let note = format!(
        "feature=Dimmer; shape={generator}; evaluator={evaluator}; implementation=SyndocalCorrected; duration_ms={period_ms}; recovered_sample_ms={DASLIGHT_CURVE_SAMPLE_MS}; runtime_time=continuous; {evaluator_detail}; correction_reason={correction_reason}; rate={rate}; low={low}; high={high}; source_range={source_range} with native 0..1 clamp; source_phase={phase}; fixture_spread={phasing}; size={size}; offset={offset}{rng_note}; {clock_note}"
    );
    Ok(ConvertedDvcEffect {
        target: Some(CueEffectTarget {
            effect_id,
            enabled: true,
            params: Some(EffectParamsSnapshot::Lfo(request)),
            transition_ms: None,
        }),
        generator,
        note,
        approximations,
        warnings: Vec::new(),
    })
}

fn convert_dvc_inverse_ramp_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    effect_id: u64,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    let params = dvc_curve_effect_params(effect, "Inverse Ramp")?;
    let rate = dvc_param(&params, 1, "Rate")?;
    let size = dvc_param(&params, 2, "Size")?;
    let phase = dvc_param(&params, 3, "Phase")?;
    let offset = dvc_param(&params, 4, "Offset")?;
    let phasing = dvc_param(&params, 5, "Phasing")?;
    let duration_ms = effect
        .attribute("DURATION")
        .ok_or_else(|| "Inverse Ramp EFFECT is missing DURATION".to_string())?
        .parse::<f64>()
        .map_err(|error| format!("Inverse Ramp DURATION is invalid: {error}"))?;
    if !duration_ms.is_finite()
        || duration_ms < f64::from(DASLIGHT_CURVE_SAMPLE_MS)
        || duration_ms > u64::MAX as f64
    {
        return Err(format!(
            "DURATION must be a finite Curve source buffer of at least {DASLIGHT_CURVE_SAMPLE_MS} ms, found {duration_ms}"
        ));
    }
    let period_ms = duration_ms.round() as u64;
    // Daslight 5.0.6.2 CInverseRampEffect evaluates a centered saw and then
    // applies `Offset - centered * Size + Size - 0.5`, clamped to 0..1.
    let raw_low = offset + size * 0.5 - 0.5;
    let raw_high = offset + size * 1.5 - 0.5;
    let low = normalized_dmx(raw_high);
    let high = normalized_dmx(raw_low);
    let mut targets = dvc_rack_targets(rack, fixture_refs)?;
    let incompatible_dimmer_targets = retain_dvc_dimmer_targets(&mut targets, fixture_refs);
    if targets.fixture_ids.is_empty() {
        return Err("BEAMS resolved to no Inverse Ramp fixture targets".to_string());
    }

    let mut approximations = vec![
        "Syndocal evaluates Inverse Ramp continuously instead of holding Daslight's 40 ms work samples; Rate/Phase/Size/Offset, descending direction, clamp intervals, and beam order are preserved"
            .to_string(),
    ];
    if incompatible_dimmer_targets > 0 {
        approximations.push(format!(
            "{incompatible_dimmer_targets} fixture target(s) without a Dimmer attribute or addressable color beam were omitted"
        ));
    }
    let (clock_sync, clock_note, clock_warning) = dvc_scene_clock_sync(scene);
    if let Some(clock_warning) = clock_warning {
        approximations.push(clock_warning);
    }

    let beam_targets = dvc_effect_beam_targets(&targets, "Dimmer");
    let request = LfoEffectRequest {
        label: format!("{scene_name} (Inverse Ramp)"),
        fixture_ids: targets.fixture_ids,
        target_group_ids: Vec::new(),
        attribute: "Dimmer".to_string(),
        video_targets: Vec::new(),
        shape: LfoShape::Saw,
        period_ms,
        clock_sync,
        low,
        high,
        phase: phase as f32,
        fixture_spread: phasing as f32,
        beam_targets,
        blend_mode: EffectBlendMode::Override,
        daslight_curve: Some(DaslightCurveSource {
            rate: rate as f32,
            size: size as f32,
            offset: offset as f32,
            sample_ms: DASLIGHT_CURVE_SAMPLE_MS,
            rng_seed: None,
        }),
        daslight_custom_curve: None,
    };
    let note = format!(
        "feature=Dimmer; implementation=SyndocalCorrected; duration_ms={period_ms}; recovered_sample_ms={DASLIGHT_CURVE_SAMPLE_MS}; runtime_time=continuous; correction_reason=timer granularity must not stair-step output; rate={rate}; descending_ramp=Saw directed from low-field {low} to high-field {high}; source_range={raw_low:.3}..{raw_high:.3} with native 0..1 clamp; source_phase={phase}; fixture_spread={phasing}; size={size}; offset={offset}; {clock_note}"
    );
    Ok(ConvertedDvcEffect {
        target: Some(CueEffectTarget {
            effect_id,
            enabled: true,
            params: Some(EffectParamsSnapshot::Lfo(request)),
            transition_ms: None,
        }),
        generator: "Inverse Ramp",
        note,
        approximations,
        warnings: Vec::new(),
    })
}

fn convert_dvc_pulse_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    effect_id: u64,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    let params = dvc_curve_effect_params(effect, "Pulse")?;
    let rate = dvc_param(&params, 1, "Rate")?;
    let size = dvc_param(&params, 2, "Size")?;
    let phase = dvc_param(&params, 3, "Phase")?;
    let offset = dvc_param(&params, 4, "Offset")?;
    let phasing = dvc_param(&params, 5, "Phasing")?;
    let duration_ms = effect
        .attribute("DURATION")
        .ok_or_else(|| "Pulse EFFECT is missing DURATION".to_string())?
        .parse::<f64>()
        .map_err(|error| format!("Pulse DURATION is invalid: {error}"))?;
    if !duration_ms.is_finite()
        || duration_ms < f64::from(DASLIGHT_CURVE_SAMPLE_MS)
        || duration_ms > u64::MAX as f64
    {
        return Err(format!(
            "DURATION must be a finite Curve source buffer of at least {DASLIGHT_CURVE_SAMPLE_MS} ms, found {duration_ms}"
        ));
    }
    let period_ms = duration_ms.round() as u64;
    let sample_count = period_ms / u64::from(DASLIGHT_CURVE_SAMPLE_MS);
    if sample_count > u64::from(u32::MAX) {
        return Err(format!(
            "Pulse source buffer has {sample_count} samples, exceeding Daslight's 32-bit evaluator domain"
        ));
    }
    let mut targets = dvc_rack_targets(rack, fixture_refs)?;
    let incompatible_dimmer_targets = retain_dvc_dimmer_targets(&mut targets, fixture_refs);
    if targets.fixture_ids.is_empty() {
        return Err("BEAMS resolved to no Pulse fixture targets".to_string());
    }

    let mut approximations = vec![
        "Syndocal replaces Daslight's fixed 0.005 Pulse window and 40 ms hold with a normalized continuous window, so DURATION no longer changes authored Size and motion does not stair-step; carrier Rate/Phase, Offset, and cycle endpoints are preserved"
            .to_string(),
    ];
    if incompatible_dimmer_targets > 0 {
        approximations.push(format!(
            "{incompatible_dimmer_targets} fixture target(s) without a Dimmer attribute or addressable color beam were omitted"
        ));
    }
    let (clock_sync, clock_note, clock_warning) = dvc_scene_clock_sync(scene);
    if let Some(clock_warning) = clock_warning {
        approximations.push(clock_warning);
    }
    let beam_targets = dvc_effect_beam_targets(&targets, "Dimmer");
    let request = LfoEffectRequest {
        label: format!("{scene_name} (Pulse)"),
        fixture_ids: targets.fixture_ids,
        target_group_ids: Vec::new(),
        attribute: "Dimmer".to_string(),
        video_targets: Vec::new(),
        shape: LfoShape::Pulse,
        period_ms,
        clock_sync,
        low: 0,
        high: u16::MAX,
        phase: phase as f32,
        fixture_spread: phasing as f32,
        beam_targets,
        blend_mode: EffectBlendMode::Override,
        daslight_curve: Some(DaslightCurveSource {
            rate: rate as f32,
            size: size as f32,
            offset: offset as f32,
            sample_ms: DASLIGHT_CURVE_SAMPLE_MS,
            rng_seed: None,
        }),
        daslight_custom_curve: None,
    };
    let note = format!(
        "feature=Dimmer; shape=Pulse; source_evaluator=CPulseEffect@0x14036F8D0; implementation=SyndocalCorrected; duration_ms={period_ms}; recovered_sample_ms={DASLIGHT_CURVE_SAMPLE_MS}; recovered_sample_count={sample_count}; runtime_time=continuous; rate={rate}; carrier=sin(TAU*(2*Rate*progress-Phase)); Daslight_fixed_window_slope=0.005; Syndocal_window=1-abs(2*progress-1); correction_reason=DURATION must not scale authored Size and timer granularity must not stair-step output; source_range=clamp(Offset+0.5+carrier*Size*window,0,1); source_phase={phase}; fixture_spread={phasing}; size={size}; offset={offset}; {clock_note}"
    );
    Ok(ConvertedDvcEffect {
        target: Some(CueEffectTarget {
            effect_id,
            enabled: true,
            params: Some(EffectParamsSnapshot::Lfo(request)),
            transition_ms: None,
        }),
        generator: "Pulse",
        note,
        approximations,
        warnings: Vec::new(),
    })
}

fn convert_dvc_sinus_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    effect_id: u64,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    let params = dvc_curve_effect_params(effect, "Sinus")?;
    let rate = dvc_param(&params, 1, "Rate")?;
    let size = dvc_param(&params, 2, "Size")?;
    let phase = dvc_param(&params, 3, "Phase")?;
    let offset = dvc_param(&params, 4, "Offset")?;
    let phasing = dvc_param(&params, 5, "Phasing")?;
    let duration_ms = effect
        .attribute("DURATION")
        .ok_or_else(|| "Sinus EFFECT is missing DURATION".to_string())?
        .parse::<f64>()
        .map_err(|error| format!("Sinus DURATION is invalid: {error}"))?;
    if !duration_ms.is_finite()
        || duration_ms < f64::from(DASLIGHT_CURVE_SAMPLE_MS)
        || duration_ms > u64::MAX as f64
    {
        return Err(format!(
            "DURATION must be a finite Curve source buffer of at least {DASLIGHT_CURVE_SAMPLE_MS} ms, found {duration_ms}"
        ));
    }
    let period_ms = duration_ms.round() as u64;
    // Daslight 5.0.6.2 CSinusEffect evaluates
    // `sin(Rate*pi*t - Phase*2*pi) * Size/2 + Offset + Size/2`.
    let raw_low = offset;
    let raw_high = offset + size;
    let low = normalized_dmx(raw_low);
    let high = normalized_dmx(raw_high);
    let mut targets = dvc_rack_targets(rack, fixture_refs)?;
    let incompatible_dimmer_targets = retain_dvc_dimmer_targets(&mut targets, fixture_refs);
    if targets.fixture_ids.is_empty() {
        return Err("BEAMS resolved to no Sinus fixture targets".to_string());
    }

    let mut approximations = vec![
        "Syndocal evaluates Sinus continuously instead of holding Daslight's 40 ms work samples; Rate/Phase/Size/Offset, clamp intervals, and beam order are preserved"
            .to_string(),
    ];
    if incompatible_dimmer_targets > 0 {
        approximations.push(format!(
            "{incompatible_dimmer_targets} fixture target(s) without a Dimmer attribute or addressable color beam were omitted"
        ));
    }
    let (clock_sync, clock_note, clock_warning) = dvc_scene_clock_sync(scene);
    if let Some(clock_warning) = clock_warning {
        approximations.push(clock_warning);
    }
    let beam_targets = dvc_effect_beam_targets(&targets, "Dimmer");
    let request = LfoEffectRequest {
        label: format!("{scene_name} (Sinus)"),
        fixture_ids: targets.fixture_ids,
        target_group_ids: Vec::new(),
        attribute: "Dimmer".to_string(),
        video_targets: Vec::new(),
        shape: LfoShape::Sine,
        period_ms,
        clock_sync,
        low,
        high,
        phase: phase as f32,
        fixture_spread: phasing as f32,
        beam_targets,
        blend_mode: EffectBlendMode::Override,
        daslight_curve: Some(DaslightCurveSource {
            rate: rate as f32,
            size: size as f32,
            offset: offset as f32,
            sample_ms: DASLIGHT_CURVE_SAMPLE_MS,
            rng_seed: None,
        }),
        daslight_custom_curve: None,
    };
    let note = format!(
        "feature=Dimmer; implementation=SyndocalCorrected; duration_ms={period_ms}; recovered_sample_ms={DASLIGHT_CURVE_SAMPLE_MS}; runtime_time=continuous; correction_reason=timer granularity must not stair-step output; rate={rate}; low={low}; high={high}; source_range={raw_low:.3}..{raw_high:.3} with native 0..1 clamp; source_phase={phase}; fixture_spread={phasing}; offset={offset}; {clock_note}"
    );
    Ok(ConvertedDvcEffect {
        target: Some(CueEffectTarget {
            effect_id,
            enabled: true,
            params: Some(EffectParamsSnapshot::Lfo(request)),
            transition_ms: None,
        }),
        generator: "Sinus",
        note,
        approximations,
        warnings: Vec::new(),
    })
}

fn convert_dvc_square_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    effect_id: u64,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    let params = dvc_curve_effect_params(effect, "Square")?;
    let rate = dvc_param(&params, 1, "Rate")?;
    let size = dvc_param(&params, 2, "Size")?;
    let phase = dvc_param(&params, 3, "Phase")?;
    let offset = dvc_param(&params, 4, "Offset")?;
    let phasing = dvc_param(&params, 5, "Phasing")?;
    let duration_ms = effect
        .attribute("DURATION")
        .ok_or_else(|| "Square EFFECT is missing DURATION".to_string())?
        .parse::<f64>()
        .map_err(|error| format!("Square DURATION is invalid: {error}"))?;
    if !duration_ms.is_finite()
        || duration_ms < f64::from(DASLIGHT_CURVE_SAMPLE_MS)
        || duration_ms > u64::MAX as f64
    {
        return Err(format!(
            "DURATION must be a finite Curve source buffer of at least {DASLIGHT_CURVE_SAMPLE_MS} ms, found {duration_ms}"
        ));
    }
    let period_ms = duration_ms.round() as u64;
    let raw_low = offset;
    let raw_high = offset + size;
    let low = normalized_dmx(raw_low);
    let high = normalized_dmx(raw_high);
    let mut targets = dvc_rack_targets(rack, fixture_refs)?;
    let incompatible_dimmer_targets = retain_dvc_dimmer_targets(&mut targets, fixture_refs);
    if targets.fixture_ids.is_empty() {
        return Err("BEAMS resolved to no Square fixture targets".to_string());
    }

    let mut approximations = vec![
        "Syndocal replaces Square's 40 ms hold and floor(400/Rate) residue with continuous equal-width authored bands; Rate/Phase/Size/Offset and beam order are preserved"
            .to_string(),
    ];
    if incompatible_dimmer_targets > 0 {
        approximations.push(format!(
            "{incompatible_dimmer_targets} fixture target(s) without a Dimmer attribute or addressable color beam were omitted"
        ));
    }
    let (clock_sync, clock_note, clock_warning) = dvc_scene_clock_sync(scene);
    if let Some(clock_warning) = clock_warning {
        approximations.push(clock_warning);
    }
    let beam_targets = dvc_effect_beam_targets(&targets, "Dimmer");
    let request = LfoEffectRequest {
        label: format!("{scene_name} (Square)"),
        fixture_ids: targets.fixture_ids,
        target_group_ids: Vec::new(),
        attribute: "Dimmer".to_string(),
        video_targets: Vec::new(),
        shape: LfoShape::Square,
        period_ms,
        clock_sync,
        low,
        high,
        phase: phase as f32,
        fixture_spread: phasing as f32,
        beam_targets,
        blend_mode: EffectBlendMode::Override,
        daslight_curve: Some(DaslightCurveSource {
            rate: rate as f32,
            size: size as f32,
            offset: offset as f32,
            sample_ms: DASLIGHT_CURVE_SAMPLE_MS,
            rng_seed: None,
        }),
        daslight_custom_curve: None,
    };
    let note = format!(
        "feature=Dimmer; shape=Square; evaluator=CSquareEffect@0x140370420; implementation=SyndocalCorrected; duration_ms={period_ms}; recovered_sample_ms={DASLIGHT_CURVE_SAMPLE_MS}; runtime_time=continuous; recovered_grid_cells=400; recovered_half_period_cells=floor(400/Rate); Syndocal_band=floor(fract(progress-Phase)*Rate); correction_reason=timer granularity and integer residue must not change authored bands; rate={rate}; even_band=high; low={low}; high={high}; source_phase={phase}; fixture_spread={phasing}; size={size}; offset={offset}; {clock_note}"
    );
    Ok(ConvertedDvcEffect {
        target: Some(CueEffectTarget {
            effect_id,
            enabled: true,
            params: Some(EffectParamsSnapshot::Lfo(request)),
            transition_ms: None,
        }),
        generator: "Square",
        note,
        approximations,
        warnings: Vec::new(),
    })
}

fn convert_dvc_strobe_effect(
    scene: Node<'_, '_>,
    scene_name: &str,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    effect_id: u64,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<ConvertedDvcEffect, String> {
    let params = dvc_curve_effect_params(effect, "Strobe")?;
    let rate = dvc_param(&params, 1, "Rate")?;
    let size = dvc_param(&params, 2, "Size")?;
    let phase = dvc_param(&params, 3, "Phase")?;
    let offset = dvc_param(&params, 4, "Offset")?;
    let phasing = dvc_param(&params, 5, "Phasing")?;
    let duration_ms = effect
        .attribute("DURATION")
        .ok_or_else(|| "Strobe EFFECT is missing DURATION".to_string())?
        .parse::<f64>()
        .map_err(|error| format!("Strobe DURATION is invalid: {error}"))?;
    if !duration_ms.is_finite()
        || duration_ms < f64::from(DASLIGHT_CURVE_SAMPLE_MS)
        || duration_ms > u64::MAX as f64
    {
        return Err(format!(
            "DURATION must be a finite Curve source buffer of at least {DASLIGHT_CURVE_SAMPLE_MS} ms, found {duration_ms}"
        ));
    }
    let period_ms = duration_ms.round() as u64;
    // Daslight Strobe is one-sided: Size scales the peak by half range while
    // Offset moves the low/base. Thus Size=1 reaches half range and Size=2
    // reaches full range for the verified Offset=0 specimen.
    let raw_low = offset;
    let raw_high = offset + size * 0.5;
    let low = normalized_dmx(raw_low);
    let high = normalized_dmx(raw_high);
    let mut targets = dvc_rack_targets(rack, fixture_refs)?;
    let incompatible_dimmer_targets = retain_dvc_dimmer_targets(&mut targets, fixture_refs);
    if targets.fixture_ids.is_empty() {
        return Err("BEAMS resolved to no Strobe fixture targets".to_string());
    }

    let mut approximations = vec![
        "Syndocal uses the authored Strobe Rate and the common dimensionless pulse duty instead of Daslight's floor(25/Rate) interval and one-sample 40 ms flash width; Phase duty extension, Size/Offset, and beam order are preserved"
            .to_string(),
    ];
    if incompatible_dimmer_targets > 0 {
        approximations.push(format!(
            "{incompatible_dimmer_targets} fixture target(s) without a Dimmer attribute or addressable color beam were omitted"
        ));
    }
    let (clock_sync, clock_note, clock_warning) = dvc_scene_clock_sync(scene);
    if let Some(clock_warning) = clock_warning {
        approximations.push(clock_warning);
    }

    let beam_targets = dvc_effect_beam_targets(&targets, "Dimmer");
    let request = LfoEffectRequest {
        label: format!("{scene_name} (Strobe)"),
        fixture_ids: targets.fixture_ids,
        target_group_ids: Vec::new(),
        attribute: "Dimmer".to_string(),
        video_targets: Vec::new(),
        shape: LfoShape::Strobe,
        period_ms,
        clock_sync,
        low,
        high,
        phase: phase as f32,
        fixture_spread: phasing as f32,
        beam_targets,
        blend_mode: EffectBlendMode::Override,
        daslight_curve: Some(DaslightCurveSource {
            rate: rate as f32,
            size: size as f32,
            offset: offset as f32,
            sample_ms: DASLIGHT_CURVE_SAMPLE_MS,
            rng_seed: None,
        }),
        daslight_custom_curve: None,
    };
    let note = format!(
        "feature=Dimmer; shape=Strobe; implementation=SyndocalCorrected; duration_ms={period_ms}; recovered_sample_ms={DASLIGHT_CURVE_SAMPLE_MS}; runtime_time=continuous; recovered_interval=floor(25/Rate); Syndocal_interval_seconds=1/Rate; Syndocal_base_duty=0.2; extended_duty=max(0.2,Phase/2); correction_reason=integer timer division and one-sample flash width must not change authored Rate or duty; rate={rate}; low={low}; high={high}; source_phase={phase}; fixture_spread={phasing}; size={size}; offset={offset}; {clock_note}"
    );
    Ok(ConvertedDvcEffect {
        target: Some(CueEffectTarget {
            effect_id,
            enabled: true,
            params: Some(EffectParamsSnapshot::Lfo(request)),
            transition_ms: None,
        }),
        generator: "Strobe",
        note,
        approximations,
        warnings: Vec::new(),
    })
}

fn dvc_effect_params(effect: Node<'_, '_>) -> Result<HashMap<u16, f64>, String> {
    let params_node = direct_child(effect, "PARAMS")
        .ok_or_else(|| "confirmed generator is missing PARAMS".to_string())?;
    let param_nodes = element_children(params_node)
        .filter(|node| node.has_tag_name("PARAM"))
        .collect::<Vec<_>>();
    let declared = params_node
        .attribute("NB")
        .and_then(|value| value.parse::<usize>().ok());
    if declared.is_some_and(|count| count != param_nodes.len()) {
        return Err(format!(
            "PARAMS declares {} entries but contains {}",
            declared.unwrap_or_default(),
            param_nodes.len()
        ));
    }
    let mut params = HashMap::new();
    for param in param_nodes {
        let id = required_attribute(param, "ID", "PARAM")?
            .parse::<u16>()
            .map_err(|error| format!("PARAM ID is invalid: {error}"))?;
        let value = required_attribute(param, "VAL", "PARAM")?
            .parse::<f64>()
            .map_err(|error| format!("PARAM {id} value is invalid: {error}"))?;
        if params.insert(id, value).is_some() {
            return Err(format!("PARAM {id} is duplicated"));
        }
    }
    Ok(params)
}

fn dvc_curve_effect_params(
    effect: Node<'_, '_>,
    generator: &str,
) -> Result<HashMap<u16, f64>, String> {
    let params = dvc_effect_params(effect)?;
    require_exact_dvc_params(&params, &[1, 2, 3, 4, 5])?;
    require_exact_dvc_param_types(effect, &[(1, 0), (2, 1), (3, 1), (4, 1), (5, 1)])?;
    dvc_integer_range_param(&params, 1, &format!("{generator} Rate"), 1, 10)?;
    dvc_finite_range_param(&params, 2, &format!("{generator} Size"), 0.0, 2.0)?;
    dvc_unit_param(&params, 3, &format!("{generator} Phase"))?;
    dvc_finite_range_param(&params, 4, &format!("{generator} Offset"), -1.0, 1.0)?;
    dvc_unit_param(&params, 5, &format!("{generator} Phasing"))?;
    Ok(params)
}

fn dvc_color_palette_stops(param: Node<'_, '_>) -> Result<Vec<ColorEffectStop>, String> {
    if param.attribute("TYPE") != Some("4") {
        return Err(format!(
            "palette PARAM 1 must have TYPE=4, found {}",
            param.attribute("TYPE").unwrap_or("missing")
        ));
    }
    let colors = direct_child(param, "COLORS")
        .ok_or_else(|| "palette PARAM TYPE=4 ID=1 is missing COLORS".to_string())?;
    let color_nodes = element_children(colors)
        .filter(|node| node.has_tag_name("COLOR"))
        .collect::<Vec<_>>();
    let declared_colors = colors
        .attribute("NB")
        .and_then(|value| value.parse::<usize>().ok());
    if declared_colors.is_some_and(|count| count != color_nodes.len()) {
        return Err(format!(
            "COLORS declares {} entries but contains {}",
            declared_colors.unwrap_or_default(),
            color_nodes.len()
        ));
    }
    if !(protocol::DASLIGHT_COLOR_PALETTE_MIN_STOPS..=protocol::DASLIGHT_COLOR_PALETTE_MAX_STOPS)
        .contains(&color_nodes.len())
    {
        return Err(format!(
            "palette requires 1..255 COLOR entries, found {}",
            color_nodes.len()
        ));
    }
    let color_count = color_nodes.len();
    let mut stops = Vec::with_capacity(color_count);
    for (index, color) in color_nodes.into_iter().enumerate() {
        let raw = required_attribute(color, "VAL", "COLOR")?;
        let components = raw.split('/').collect::<Vec<_>>();
        if components.len() != 18 {
            return Err(format!(
                "palette COLOR {index} requires the verified 18-component VAL, found {}",
                components.len()
            ));
        }
        let mut rgb = [0_u16; 3];
        for (component_index, output) in rgb.iter_mut().enumerate() {
            let value = components[component_index]
                .parse::<f64>()
                .map_err(|error| {
                    format!("palette COLOR {index} component {component_index} is invalid: {error}")
                })?;
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(format!(
                    "palette COLOR {index} component {component_index} must be within 0..1, found {value}"
                ));
            }
            *output = (value * u16::MAX as f64).round() as u16;
        }
        stops.push(ColorEffectStop {
            position: if color_count == 1 {
                0.0
            } else {
                index as f32 / (color_count - 1) as f32
            },
            color: ColorEffectColor {
                red: rgb[0],
                green: rgb[1],
                blue: rgb[2],
            },
        });
    }
    Ok(stops)
}

fn dvc_color_palette_and_params(
    effect: Node<'_, '_>,
) -> Result<(HashMap<u16, f64>, Vec<ColorEffectStop>), String> {
    let params_node = direct_child(effect, "PARAMS")
        .ok_or_else(|| "confirmed palette generator is missing PARAMS".to_string())?;
    let param_nodes = element_children(params_node)
        .filter(|node| node.has_tag_name("PARAM"))
        .collect::<Vec<_>>();
    let declared = params_node
        .attribute("NB")
        .and_then(|value| value.parse::<usize>().ok());
    if declared.is_some_and(|count| count != param_nodes.len()) {
        return Err(format!(
            "PARAMS declares {} entries but contains {}",
            declared.unwrap_or_default(),
            param_nodes.len()
        ));
    }
    let mut seen = HashSet::new();
    let mut params = HashMap::new();
    let mut palette = None;
    for param in param_nodes {
        let id = required_attribute(param, "ID", "PARAM")?
            .parse::<u16>()
            .map_err(|error| format!("PARAM ID is invalid: {error}"))?;
        if !seen.insert(id) {
            return Err(format!("PARAM {id} is duplicated"));
        }
        if id == 1 {
            palette = Some(dvc_color_palette_stops(param)?);
        } else {
            let value = required_attribute(param, "VAL", "PARAM")?
                .parse::<f64>()
                .map_err(|error| format!("PARAM {id} value is invalid: {error}"))?;
            params.insert(id, value);
        }
    }
    let palette = palette.ok_or_else(|| {
        "palette source PARAM TYPE=4 ID=1/COLORS/COLOR@VAL was not found; generator remains Skipped"
            .to_string()
    })?;
    Ok((params, palette))
}

fn dvc_move_effect_params(
    effect: Node<'_, '_>,
    generator: &str,
) -> Result<(Vec<MovePathPoint>, f32, bool), String> {
    let params_node = direct_child(effect, "PARAMS")
        .ok_or_else(|| format!("{generator} generator is missing PARAMS"))?;
    let param_nodes = element_children(params_node).collect::<Vec<_>>();
    if let Some(unexpected) = param_nodes.iter().find(|node| !node.has_tag_name("PARAM")) {
        return Err(format!(
            "{generator} PARAMS contains unexpected <{}> element",
            unexpected.tag_name().name()
        ));
    }
    let declared = required_attribute(params_node, "NB", "PARAMS")?
        .parse::<usize>()
        .map_err(|error| format!("PARAMS NB is invalid: {error}"))?;
    if declared != param_nodes.len() {
        return Err(format!(
            "PARAMS declares {} entries but contains {}",
            declared,
            param_nodes.len()
        ));
    }

    let mut points = None;
    let mut phasing = None;
    let mut symmetry = None;
    let mut seen = HashSet::new();
    for param in param_nodes {
        let id = required_attribute(param, "ID", "PARAM")?
            .parse::<u16>()
            .map_err(|error| format!("PARAM ID is invalid: {error}"))?;
        if !seen.insert(id) {
            return Err(format!("PARAM {id} is duplicated"));
        }
        let param_type = required_attribute(param, "TYPE", "PARAM")?
            .parse::<u16>()
            .map_err(|error| format!("PARAM {id} TYPE is invalid: {error}"))?;
        match id {
            1 => {
                if param_type != 5 {
                    return Err(format!(
                        "Path PARAM 1 must use TYPE=5, found TYPE={param_type}"
                    ));
                }
                let points_node = direct_child(param, "POINTS")
                    .ok_or_else(|| format!("{generator} Path PARAM 1 is missing POINTS"))?;
                let point_nodes = element_children(points_node).collect::<Vec<_>>();
                if let Some(unexpected) =
                    point_nodes.iter().find(|node| !node.has_tag_name("POINT"))
                {
                    return Err(format!(
                        "{generator} POINTS contains unexpected <{}> element",
                        unexpected.tag_name().name()
                    ));
                }
                let declared_points = required_attribute(points_node, "NB", "POINTS")?
                    .parse::<usize>()
                    .map_err(|error| format!("POINTS NB is invalid: {error}"))?;
                if declared_points != point_nodes.len() {
                    return Err(format!(
                        "POINTS declares {declared_points} vertices but contains {}",
                        point_nodes.len()
                    ));
                }
                let mut parsed_points = Vec::with_capacity(point_nodes.len());
                for (point_index, point) in point_nodes.into_iter().enumerate() {
                    let x = required_attribute(point, "X", "POINT")?
                        .parse::<f32>()
                        .map_err(|error| {
                            format!("POINT {} X is invalid: {error}", point_index + 1)
                        })?;
                    let y = required_attribute(point, "Y", "POINT")?
                        .parse::<f32>()
                        .map_err(|error| {
                            format!("POINT {} Y is invalid: {error}", point_index + 1)
                        })?;
                    if !x.is_finite()
                        || !y.is_finite()
                        || !(0.0..=1.0).contains(&x)
                        || !(0.0..=1.0).contains(&y)
                    {
                        return Err(format!(
                            "POINT {} must be finite normalized Pan/Tilt coordinates within 0..1, found ({x},{y})",
                            point_index + 1
                        ));
                    }
                    parsed_points.push(MovePathPoint { x, y });
                }
                points = Some(parsed_points);
            }
            2 => {
                if param_type != 1 {
                    return Err(format!(
                        "Phasing PARAM 2 must use TYPE=1, found TYPE={param_type}"
                    ));
                }
                let value = required_attribute(param, "VAL", "PARAM 2")?
                    .parse::<f32>()
                    .map_err(|error| format!("Phasing PARAM 2 is invalid: {error}"))?;
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(format!(
                        "Phasing PARAM 2 must be within 0..1, found {value}"
                    ));
                }
                phasing = Some(value);
            }
            3 => {
                if param_type != 2 {
                    return Err(format!(
                        "Symmetry PARAM 3 must use TYPE=2, found TYPE={param_type}"
                    ));
                }
                symmetry = Some(match required_attribute(param, "VAL", "PARAM 3")? {
                    "0" => false,
                    "1" => true,
                    value => {
                        return Err(format!("Symmetry PARAM 3 must be 0 or 1, found {value}"));
                    }
                });
            }
            _ => {
                return Err(format!(
                    "confirmed {generator} generator expected PARAM IDs [1, 2, 3], found unexpected ID {id}"
                ));
            }
        }
    }
    if seen != HashSet::from([1, 2, 3]) {
        let mut actual = seen.into_iter().collect::<Vec<_>>();
        actual.sort_unstable();
        return Err(format!(
            "confirmed {generator} generator expected PARAM IDs [1, 2, 3], found {actual:?}"
        ));
    }

    let points = points.unwrap_or_default();
    let valid_point_count = match generator {
        "Line" => points.len() == 2,
        "Circle" => (2..=255).contains(&points.len()),
        _ => (2..=255).contains(&points.len()),
    };
    if !valid_point_count {
        let expected = match generator {
            "Line" => "exactly 2",
            "Circle" => "between 2 and 255",
            _ => "between 2 and 255",
        };
        return Err(format!(
            "{generator} POINTS must contain {expected} vertices, found {}",
            points.len()
        ));
    }
    Ok((
        points,
        phasing.unwrap_or_default(),
        symmetry.unwrap_or(false),
    ))
}

fn require_exact_dvc_params(params: &HashMap<u16, f64>, expected: &[u16]) -> Result<(), String> {
    let mut actual = params.keys().copied().collect::<Vec<_>>();
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    if actual != expected {
        return Err(format!(
            "confirmed generator expected PARAM IDs {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn require_exact_dvc_param_types(
    effect: Node<'_, '_>,
    expected: &[(u16, u16)],
) -> Result<(), String> {
    let params_node = direct_child(effect, "PARAMS")
        .ok_or_else(|| "confirmed generator is missing PARAMS".to_string())?;
    let params = element_children(params_node)
        .filter(|node| node.has_tag_name("PARAM"))
        .collect::<Vec<_>>();
    for (id, expected_type) in expected {
        let param = params
            .iter()
            .find(|param| {
                param
                    .attribute("ID")
                    .and_then(|value| value.parse::<u16>().ok())
                    == Some(*id)
            })
            .ok_or_else(|| format!("confirmed generator is missing PARAM {id}"))?;
        let actual_type = required_attribute(*param, "TYPE", "PARAM")?
            .parse::<u16>()
            .map_err(|error| format!("PARAM {id} TYPE is invalid: {error}"))?;
        if actual_type != *expected_type {
            return Err(format!(
                "confirmed generator expected PARAM {id} TYPE={expected_type}, found TYPE={actual_type}"
            ));
        }
    }
    Ok(())
}

fn dvc_param(params: &HashMap<u16, f64>, id: u16, label: &str) -> Result<f64, String> {
    params
        .get(&id)
        .copied()
        .ok_or_else(|| format!("{label} PARAM {id} is missing"))
}

fn dvc_finite_param(params: &HashMap<u16, f64>, id: u16, label: &str) -> Result<f32, String> {
    let value = dvc_param(params, id, label)?;
    if !value.is_finite() || value < f32::MIN as f64 || value > f32::MAX as f64 {
        return Err(format!("{label} PARAM {id} must be finite, found {value}"));
    }
    Ok(value as f32)
}

fn dvc_finite_range_param(
    params: &HashMap<u16, f64>,
    id: u16,
    label: &str,
    min: f64,
    max: f64,
) -> Result<f32, String> {
    let value = dvc_param(params, id, label)?;
    if !value.is_finite() || !(min..=max).contains(&value) {
        return Err(format!(
            "{label} PARAM {id} must be within {min}..{max}, found {value}"
        ));
    }
    Ok(value as f32)
}

fn dvc_integer_range_param(
    params: &HashMap<u16, f64>,
    id: u16,
    label: &str,
    min: i64,
    max: i64,
) -> Result<f32, String> {
    let value = dvc_param(params, id, label)?;
    if !value.is_finite() || value.fract() != 0.0 || value < min as f64 || value > max as f64 {
        return Err(format!(
            "{label} PARAM {id} must be an integer within {min}..{max}, found {value}"
        ));
    }
    Ok(value as f32)
}

fn dvc_unit_param(params: &HashMap<u16, f64>, id: u16, label: &str) -> Result<f32, String> {
    let value = dvc_finite_param(params, id, label)?;
    if !(0.0..=1.0).contains(&value) {
        return Err(format!(
            "{label} PARAM {id} must be within 0..1, found {value}"
        ));
    }
    Ok(value)
}

fn dvc_binary_param(params: &HashMap<u16, f64>, id: u16, label: &str) -> Result<bool, String> {
    match dvc_param(params, id, label)? {
        value if value == 0.0 => Ok(false),
        value if value == 1.0 => Ok(true),
        value => Err(format!("{label} PARAM {id} must be 0 or 1, found {value}")),
    }
}

fn dvc_positive_integer_param(
    params: &HashMap<u16, f64>,
    id: u16,
    label: &str,
) -> Result<u64, String> {
    let value = dvc_param(params, id, label)?;
    if !value.is_finite()
        || value < 1.0
        || value > u64::MAX as f64
        || value.fract().abs() > f64::EPSILON
    {
        return Err(format!(
            "{label} PARAM {id} must be a positive integer, found {value}"
        ));
    }
    Ok(value as u64)
}

fn dvc_rack_feature_spec(
    rack: Node<'_, '_>,
    profiles: &[ParsedProfile],
) -> Result<DvcRackFeatureSpec, String> {
    let preset_nodes = direct_child(rack, "PRESETS")
        .into_iter()
        .flat_map(element_children)
        .filter(|node| node.has_tag_name("PRESET"))
        .collect::<Vec<_>>();
    if preset_nodes.is_empty() {
        return Ok(DvcRackFeatureSpec {
            preset_type: 4,
            low: 0,
            high: u16::MAX,
            source: "implicit/default PRESET type 4 (Dimmer)".to_string(),
        });
    }

    let mut resolved = preset_nodes
        .iter()
        .copied()
        .map(|preset| dvc_single_rack_feature_spec(preset, profiles))
        .collect::<Result<Vec<_>, _>>()?;
    let first = resolved.remove(0);
    if resolved.iter().any(|feature| {
        feature.preset_type != first.preset_type
            || feature.low != first.low
            || feature.high != first.high
    }) {
        let descriptions = std::iter::once(&first)
            .chain(&resolved)
            .map(|feature| {
                format!(
                    "{} => type {} range {}..{}",
                    feature.source, feature.preset_type, feature.low, feature.high
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "confirmed scalar FX rack PRESET selectors resolve to different features: {descriptions}"
        ));
    }
    let sources = std::iter::once(first.source)
        .chain(resolved.into_iter().map(|feature| feature.source))
        .collect::<Vec<_>>();
    Ok(DvcRackFeatureSpec {
        preset_type: first.preset_type,
        low: first.low,
        high: first.high,
        source: sources.join(" + "),
    })
}

fn dvc_single_rack_feature_spec(
    preset: Node<'_, '_>,
    profiles: &[ParsedProfile],
) -> Result<DvcRackFeatureSpec, String> {
    let fixture_uid = preset.attribute("SSLFIXTURE").unwrap_or_default().trim();
    let raw_channel = preset.attribute("SSLCHANNEL").unwrap_or_default().trim();
    let raw_preset = preset.attribute("SSLPRESET").unwrap_or_default().trim();
    if fixture_uid.is_empty() && raw_channel.is_empty() && raw_preset.is_empty() {
        return Ok(DvcRackFeatureSpec {
            preset_type: 4,
            low: 0,
            high: u16::MAX,
            source: "empty PRESET placeholder; verified Daslight default type 4 (Dimmer)"
                .to_string(),
        });
    }

    let parse_boundary = |attribute: &str| -> Result<u16, String> {
        let value = required_attribute(preset, attribute, "PRESET")?
            .parse::<f64>()
            .map_err(|error| format!("PRESET {attribute} is invalid: {error}"))?;
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!(
                "PRESET {attribute} must be finite and within 0..1, found {value}"
            ));
        }
        Ok(normalized_dmx(value))
    };
    let low = parse_boundary("MIN")?;
    let high = parse_boundary("MAX")?;

    let (preset_type, source) = if fixture_uid.is_empty() && raw_channel == "-1" {
        let preset_type = raw_preset
            .parse::<u16>()
            .map_err(|error| format!("generic PRESET type is invalid: {error}"))?;
        (preset_type, format!("generic PRESET type {preset_type}"))
    } else if !fixture_uid.is_empty() {
        let channel_index = raw_channel
            .parse::<usize>()
            .map_err(|error| format!("profile PRESET SSLCHANNEL is invalid: {error}"))?;
        let preset_index = raw_preset
            .parse::<usize>()
            .map_err(|error| format!("profile PRESET SSLPRESET is invalid: {error}"))?;
        let matching_profiles = profiles
            .iter()
            .filter(|profile| profile.summary.fixture_type_id.as_deref() == Some(fixture_uid))
            .collect::<Vec<_>>();
        if matching_profiles.is_empty() {
            return Err(format!(
                "profile PRESET references absent profile '{fixture_uid}'"
            ));
        }
        let matching_channels = matching_profiles
            .iter()
            .filter_map(|profile| {
                profile
                    .bindings
                    .iter()
                    .find(|binding| binding.raw_channel_index == channel_index)
            })
            .collect::<Vec<_>>();
        if matching_channels.is_empty() {
            return Err(format!(
                "profile PRESET references absent channel {channel_index} on profile '{fixture_uid}'"
            ));
        }
        let mut preset_types = matching_channels
            .iter()
            .filter_map(|binding| {
                binding
                    .presets
                    .get(preset_index)
                    .and_then(|preset| preset.preset_type)
                    .or_else(|| dvc_preset_type_for_known_channel_type(binding.channel_type))
            })
            .collect::<Vec<_>>();
        preset_types.sort_unstable();
        preset_types.dedup();
        let [preset_type] = preset_types.as_slice() else {
            return Err(format!(
                "profile PRESET {fixture_uid}:{channel_index}:{preset_index} did not resolve to one SSLPRESETTYPE"
            ));
        };
        (
            *preset_type,
            format!(
                "profile PRESET {fixture_uid}:{channel_index}:{preset_index} -> type {preset_type}"
            ),
        )
    } else {
        return Err(format!(
            "PRESET selector SSLFIXTURE='{fixture_uid}' SSLCHANNEL='{raw_channel}' SSLPRESET='{raw_preset}' has an unsupported shape"
        ));
    };

    Ok(DvcRackFeatureSpec {
        preset_type,
        low,
        high,
        source,
    })
}

fn dvc_preset_type_for_known_channel_type(channel_type: u16) -> Option<u16> {
    match channel_type {
        7 => Some(4),
        25 => Some(65),
        26 => Some(66),
        27 => Some(67),
        45 => Some(81),
        _ => None,
    }
}

fn dvc_beam_feature_attribute(
    target: &ColorEffectBeamTarget,
    feature: &DvcRackFeatureSpec,
    profiles: &[ParsedProfile],
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Option<String> {
    let fixture_ref = fixture_refs
        .values()
        .find(|fixture_ref| fixture_ref.fixture_id == target.fixture_id)?;
    // Daslight's generic PRESET type 4 is a virtual beam Dimmer even when a
    // segmented RGB/RGBA profile has no physical per-cell dimmer channel. The
    // engine resolves this authored feature against the exact BEAMID and scales
    // only that segment's current colour.
    if feature.preset_type == 4 && target.beam_index < fixture_ref.color_beam_count {
        return Some("Dimmer".to_string());
    }
    let profile = profiles.get(fixture_ref.profile_index)?;
    let bindings = profile
        .bindings
        .iter()
        .filter(|binding| {
            binding
                .presets
                .iter()
                .any(|preset| preset.preset_type == Some(feature.preset_type))
                || (binding
                    .presets
                    .iter()
                    .all(|preset| preset.preset_type.is_none())
                    && dvc_known_channel_type_for_preset_type(feature.preset_type)
                        == Some(binding.channel_type))
        })
        .collect::<Vec<_>>();
    match bindings.as_slice() {
        [] => None,
        [binding] => (target.beam_index == 0
            || (feature.preset_type == 4 && fixture_ref.supports_dimmer))
            .then(|| binding.attribute.clone()),
        _ => bindings
            .get(target.beam_index as usize)
            .map(|binding| binding.attribute.clone()),
    }
}

fn dvc_known_channel_type_for_preset_type(preset_type: u16) -> Option<u16> {
    match preset_type {
        4 => Some(7),
        65 => Some(25),
        66 => Some(26),
        67 => Some(27),
        81 => Some(45),
        _ => None,
    }
}

fn dvc_rack_targets(
    rack: Node<'_, '_>,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> Result<DvcRackTargets, String> {
    let beams = direct_child(rack, "BEAMS")
        .ok_or_else(|| "confirmed generator rack is missing BEAMS".to_string())?;
    let beam_nodes = element_children(beams)
        .filter(|node| node.has_tag_name("BEAM"))
        .collect::<Vec<_>>();
    if let Some(raw_declared) = beams.attribute("NB") {
        let declared = raw_declared
            .parse::<usize>()
            .map_err(|error| format!("BEAMS NB is invalid: {error}"))?;
        if declared != beam_nodes.len() {
            return Err(format!(
                "BEAMS declares {declared} entries but contains {}",
                beam_nodes.len()
            ));
        }
    }

    let mut selection_indices = HashMap::<String, usize>::new();
    let mut ordered_steps = Vec::<Vec<u64>>::new();
    let mut fixture_ids = Vec::new();
    let mut seen_fixture_ids = HashSet::new();
    let mut beam_targets = Vec::with_capacity(beam_nodes.len());
    for (beam_index, beam) in beam_nodes.into_iter().enumerate() {
        let fixture_uid = required_attribute(beam, "FIXTURE", "BEAM")?;
        let fixture_ref = fixture_refs.get(fixture_uid).ok_or_else(|| {
            format!("BEAM fixture {fixture_uid} is not present in the imported patch")
        })?;
        let beam_id = required_attribute(beam, "BEAMID", "BEAM")?
            .parse::<u16>()
            .map_err(|error| format!("BEAMID is invalid: {error}"))?;
        if seen_fixture_ids.insert(fixture_ref.fixture_id) {
            fixture_ids.push(fixture_ref.fixture_id);
        }

        let selection_key = beam
            .attribute("IDSELECTION")
            .map(|value| format!("selection:{value}"))
            .unwrap_or_else(|| format!("beam:{beam_index}"));
        let selection_index = if let Some(index) = selection_indices.get(&selection_key) {
            *index
        } else {
            let index = ordered_steps.len();
            ordered_steps.push(Vec::new());
            selection_indices.insert(selection_key, index);
            index
        };
        let step = &mut ordered_steps[selection_index];
        if !step.contains(&fixture_ref.fixture_id) {
            step.push(fixture_ref.fixture_id);
        }
        beam_targets.push(ColorEffectBeamTarget {
            fixture_id: fixture_ref.fixture_id,
            beam_index: beam_id,
            selection_index: u32::try_from(selection_index).unwrap_or(u32::MAX),
            feature_attribute: None,
        });
    }

    Ok(DvcRackTargets {
        ordered_steps,
        fixture_ids,
        beam_targets,
    })
}

fn dvc_effect_beam_targets(targets: &DvcRackTargets, feature: &str) -> Vec<EffectBeamTarget> {
    targets
        .beam_targets
        .iter()
        .map(|target| EffectBeamTarget {
            fixture_id: target.fixture_id,
            beam_index: target.beam_index,
            selection_index: target.selection_index,
            feature_attribute: feature.to_string(),
        })
        .collect()
}

fn retain_dvc_dimmer_targets(
    targets: &mut DvcRackTargets,
    fixture_refs: &HashMap<String, FixtureImportRef>,
) -> usize {
    let capabilities = fixture_refs
        .values()
        .map(|fixture_ref| {
            (
                fixture_ref.fixture_id,
                (fixture_ref.supports_dimmer, fixture_ref.color_beam_count),
            )
        })
        .collect::<HashMap<_, _>>();
    let before = targets.fixture_ids.len();
    targets.beam_targets.retain(|target| {
        capabilities
            .get(&target.fixture_id)
            .is_some_and(|(supports_dimmer, color_beam_count)| {
                *supports_dimmer || target.beam_index < *color_beam_count
            })
    });
    let supported = targets
        .beam_targets
        .iter()
        .map(|target| target.fixture_id)
        .collect::<HashSet<_>>();
    targets
        .fixture_ids
        .retain(|fixture_id| supported.contains(fixture_id));
    for step in &mut targets.ordered_steps {
        step.retain(|fixture_id| supported.contains(fixture_id));
    }
    before.saturating_sub(targets.fixture_ids.len())
}

fn retain_dvc_color_spatial_targets(
    targets: &mut DvcRackTargets,
    fixture_refs: &HashMap<String, FixtureImportRef>,
    allow_dimmer_feature: bool,
) -> usize {
    let capabilities = fixture_refs
        .values()
        .map(|fixture_ref| {
            (
                fixture_ref.fixture_id,
                (fixture_ref.supports_dimmer, fixture_ref.color_beam_count),
            )
        })
        .collect::<HashMap<_, _>>();
    let before = targets.beam_targets.len();
    targets.beam_targets.retain(|target| {
        capabilities
            .get(&target.fixture_id)
            .is_some_and(|(supports_dimmer, color_beam_count)| {
                target.beam_index < *color_beam_count || (allow_dimmer_feature && *supports_dimmer)
            })
    });
    let retained_fixture_ids = targets
        .beam_targets
        .iter()
        .map(|target| target.fixture_id)
        .collect::<HashSet<_>>();
    targets
        .fixture_ids
        .retain(|fixture_id| retained_fixture_ids.contains(fixture_id));
    before.saturating_sub(targets.beam_targets.len())
}

fn dvc_chaser_step_duration(
    effect: Node<'_, '_>,
    _scene: Node<'_, '_>,
    step_count: usize,
    approximations: &mut Vec<String>,
) -> Result<(u64, String), String> {
    let duration_ms = effect
        .attribute("DURATION")
        .ok_or_else(|| "Chaser EFFECT is missing DURATION".to_string())?
        .parse::<f64>()
        .map_err(|error| format!("Chaser DURATION is invalid: {error}"))?;
    if !duration_ms.is_finite() || duration_ms <= 0.0 {
        return Err(format!(
            "Chaser DURATION must be finite and greater than 0, found {duration_ms}"
        ));
    }
    // EFFECT DURATION is already the authored generator period. SCENE SPEED is
    // the normalized live-speed dial (0.5 is neutral in Daslight), not a raw
    // duration divisor.
    let step_duration = duration_ms / step_count.max(1) as f64;
    if !step_duration.is_finite() || step_duration > u64::MAX as f64 {
        return Err("derived Chaser step duration exceeds the supported range".to_string());
    }
    let mut step_duration_ms = step_duration.round().max(1.0) as u64;
    if step_duration_ms < 10 {
        approximations.push(format!(
            "derived {step_duration_ms} ms step was clamped to the Chaser engine minimum 10 ms"
        ));
        step_duration_ms = 10;
    }
    Ok((
        step_duration_ms,
        format!("step_duration_ms=round(EFFECT DURATION / generator_slots)={step_duration_ms}"),
    ))
}

fn dvc_move_period(
    effect: Node<'_, '_>,
    _scene: Node<'_, '_>,
    generator: &str,
    approximations: &mut Vec<String>,
) -> Result<(u64, String), String> {
    let duration_ms = effect
        .attribute("DURATION")
        .ok_or_else(|| format!("{generator} EFFECT is missing DURATION"))?
        .parse::<f64>()
        .map_err(|error| format!("{generator} DURATION is invalid: {error}"))?;
    if !duration_ms.is_finite() || duration_ms <= 0.0 {
        return Err(format!(
            "{generator} DURATION must be finite and greater than 0, found {duration_ms}"
        ));
    }
    // EFFECT DURATION is already the authored generator period. SCENE SPEED is
    // Daslight's normalized live-speed dial and 0.5 represents neutral.
    let period = duration_ms;
    if !period.is_finite() || period > u64::MAX as f64 {
        return Err(format!(
            "derived {generator} period exceeds the supported range"
        ));
    }
    let mut period_ms = period.round().max(1.0) as u64;
    if period_ms < 10 {
        approximations.push(format!(
            "derived {period_ms} ms period was clamped to the Move engine minimum 10 ms"
        ));
        period_ms = 10;
    }
    Ok((
        period_ms,
        format!("period_ms=round(EFFECT DURATION)={period_ms}"),
    ))
}

fn dvc_exact_generator_period(
    effect: Node<'_, '_>,
    source_family: &str,
    generator: &str,
) -> Result<(u64, String), String> {
    let raw = effect
        .attribute("DURATION")
        .ok_or_else(|| format!("{source_family} {generator} EFFECT is missing DURATION"))?
        .trim();
    let duration_ms = raw.parse::<i32>().map_err(|_| {
        format!(
            "{source_family} {generator} DURATION must be a positive signed 32-bit integer, found {raw}"
        )
    })?;
    if duration_ms <= 0 {
        return Err(format!(
            "{source_family} {generator} DURATION must be greater than 0, found {duration_ms}"
        ));
    }

    // The recovered evaluator floors this value to a 40 ms work grid, but the
    // grid is an implementation artifact rather than authored timing. Keep the
    // positive integer serialized in the DVC and apply only Syndocal's common
    // 10 ms runtime floor. A DVC that Daslight already quantized before saving
    // remains quantized; no unavailable pre-save duration is invented here.
    let authored_duration_ms = duration_ms as u64;
    let period_ms = authored_duration_ms.max(10);
    let recovered_frame_count = (authored_duration_ms / 40).max(1);
    Ok((
        period_ms,
        format!(
            "authored_duration_ms={authored_duration_ms}; period_ms=max(authored_duration_ms,10)={period_ms}; recovered_frame_count=max(1,floor(EFFECT DURATION / 40))={recovered_frame_count}; correction_reason=40 ms work-grid quantization must not shorten authored timing"
        ),
    ))
}

fn dvc_scene_clock_sync(scene: Node<'_, '_>) -> (Option<EffectClockSync>, String, Option<String>) {
    let trigger = scene.attribute("PLAY_TRIGGER").unwrap_or("0").trim();
    if trigger != "2" {
        let warning = (!matches!(trigger, "" | "0")).then(|| {
            format!(
                "PLAY_TRIGGER={trigger} is not a confirmed BPM-sync mode; free-run timing retained"
            )
        });
        return (None, "clock_sync=none".to_string(), warning);
    }
    let division = scene
        .attribute("PLAY_DIVISION")
        .and_then(|value| value.parse::<f32>().ok());
    let Some(division) = division.filter(|value| value.is_finite() && *value > 0.0) else {
        return (
            None,
            "clock_sync=none".to_string(),
            Some(
                "PLAY_TRIGGER=2 had no positive PLAY_DIVISION; free-run timing retained"
                    .to_string(),
            ),
        );
    };
    let beats = 1.0 / division;
    (
        Some(EffectClockSync { beats }),
        format!("clock_sync beats=1/PLAY_DIVISION={beats}"),
        None,
    )
}

fn normalized_dmx(value: f64) -> u16 {
    (value.clamp(0.0, 1.0) * u16::MAX as f64).round() as u16
}

fn fixture_attribute_values(
    profile: &ParsedProfile,
    raw_values: &[Option<u8>],
    beam_features: Option<&[[f32; DVC_BEAM_FEATURE_SLOTS]]>,
) -> Result<Vec<AttributeValueSummary>, String> {
    let mut values = Vec::new();
    for binding in &profile.bindings {
        let mut value = match (&binding.resolution, binding.raw_offsets.as_slice()) {
            (AttributeResolution::EightBit, [offset]) => raw_values
                .get(*offset)
                .copied()
                .flatten()
                .map(|value| u16::from(value) * 257),
            (AttributeResolution::SixteenBit, [msb, lsb]) => {
                let msb = raw_values.get(*msb).copied().flatten();
                let lsb = raw_values.get(*lsb).copied().flatten();
                match (msb, lsb) {
                    (None, None) => None,
                    (Some(msb), Some(lsb)) => Some((u16::from(msb) << 8) | u16::from(lsb)),
                    _ => {
                        return Err(format!(
                            "16-bit attribute '{}' has only one written byte",
                            binding.attribute
                        ))
                    }
                }
            }
            _ => {
                return Err(format!(
                    "attribute '{}' has an invalid channel binding",
                    binding.attribute
                ))
            }
        };
        // Daslight's live DMX renderer converts the single-beam feature floats
        // back to 8-bit bytes by truncation. The channel section stores the
        // editor value (for example Amber 136/26), while the corresponding
        // half-float features produce the actual 135/25 output shown by DMX
        // Levels. Only the verified one-beam + unique-attribute case is used;
        // multi-beam payloads still keep their channel-section values because
        // their beam-to-channel mapping is not established.
        if let (AttributeResolution::EightBit, [feature_row]) =
            (&binding.resolution, beam_features.unwrap_or_default())
        {
            if let Some((slot, _)) = DVC_BEAM_FEATURE_BINDINGS
                .iter()
                .find(|(_, attribute)| *attribute == binding.attribute)
            {
                let feature = feature_row[*slot];
                if !beam_feature_is_unset(feature) {
                    let byte = u16::from(dvc_live_feature_byte(feature));
                    // A feature row is a live-value refinement, not permission
                    // to replace a contradictory channel payload. The known
                    // Daslight quantization drift is at most two DMX steps;
                    // larger disagreements keep the channel-section fallback
                    // and are reported by beam_feature_mismatch_details.
                    if value.is_some_and(|raw| (raw / 257).abs_diff(byte) <= 2) {
                        value = Some(byte * 257);
                    }
                }
            }
        }
        if let Some(value) = value {
            values.push(AttributeValueSummary {
                attribute: binding.attribute.clone(),
                value,
            });
        }
    }
    Ok(values)
}

fn dvc_live_feature_byte(feature: f32) -> u8 {
    // Daslight's observed conversion behaves as truncation with a tiny
    // boundary tolerance: the half-float for 12/255 is 11.999816 after
    // multiplication and still outputs 12, while 136/255 and 26/255 output
    // 135 and 25. 1e-3 is below one DMX sub-step and reproduces both cases.
    (feature.clamp(0.0, 1.0).mul_add(255.0, 1.0e-3))
        .floor()
        .clamp(0.0, 255.0) as u8
}

fn parse_super_scenes(
    root: Node<'_, '_>,
    cues: &mut [CueSummary],
    scene_indices: &HashMap<String, usize>,
    snapshot: &mut EngineSnapshot,
    report: &mut DvcImportReport,
) -> Result<(), String> {
    let Some(scenes_section) = direct_child(root, "SCENES") else {
        return Ok(());
    };
    let mut next_layer_id = 1_u32;
    let mut next_event_id = 1_u64;
    let mut next_audio_clip_id = 1_u64;
    let mut first_grid_bpm = None;

    for scene in scenes_section
        .descendants()
        .filter(|node| node.has_tag_name("SCENE"))
    {
        let Some(timelines) = scene
            .descendants()
            .find(|node| node.has_tag_name("TIMELINES"))
        else {
            continue;
        };
        let scene_uid = required_attribute(scene, "DASUID", "SCENE")?;
        let owner_index = *scene_indices
            .get(scene_uid)
            .ok_or_else(|| format!("Super Scene index missing for {scene_uid}"))?;
        let owner_label = cues[owner_index].label.clone();
        // Daslight only applies a block's CONFORM_TO_TEMPO flag while the
        // owning Super Scene itself uses BPM driving mode. PLAY_TRIGGER follows
        // the documented driving-mode order: 0=Off, 1=BeatGO, 2=BPM, 3=Pulse.
        let owner_bpm_driven = scene.attribute("PLAY_TRIGGER") == Some("2");
        let tempo_driven = if owner_bpm_driven {
            let play_division = parse_f32_attribute(scene, "PLAY_DIVISION").filter(|beats| {
                beats.is_finite()
                    && (protocol::MIN_CUE_AUTHORED_BEATS..=protocol::MAX_CUE_AUTHORED_BEATS)
                        .contains(beats)
            });
            if let Some(beats) = play_division {
                cues[owner_index].authored_beats = Some(beats);
                true
            } else {
                report.approximate.add(
                    1,
                    format!("Super Scene: {owner_label}"),
                    "PLAY_TRIGGER=2 had no supported positive PLAY_DIVISION; fixed-time child transport retained",
                );
                false
            }
        } else {
            false
        };
        let grid_bpm = timelines
            .ancestors()
            .find(|node| node.has_tag_name("RACK"))
            .and_then(|rack| parse_f32_attribute(rack, "GRID_BPM"))
            .filter(|bpm| bpm.is_finite() && *bpm > 0.0)
            .unwrap_or(snapshot.clock.bpm);
        if first_grid_bpm.is_none() {
            first_grid_bpm = Some(grid_bpm);
        }
        let timeline_nodes = element_children(timelines)
            .filter(|node| node.has_tag_name("TIMELINE"))
            .collect::<Vec<_>>();
        let mut layers = Vec::new();
        let mut layer_ids = HashMap::new();
        for (order, timeline) in timeline_nodes.iter().copied().enumerate() {
            let blocks = timeline
                .descendants()
                .filter(|node| node.has_tag_name("BLOCK"))
                .collect::<Vec<_>>();
            let has_audio = blocks
                .iter()
                .any(|block| block.attribute("TYPE") == Some("2"));
            let has_lighting = blocks
                .iter()
                .any(|block| block.attribute("TYPE") == Some("1"));
            let kind = if has_audio && !has_lighting {
                TimelineLayerKind::Audio
            } else {
                TimelineLayerKind::Lighting
            };
            if has_audio && has_lighting {
                report.approximate.add(
                    1,
                    format!("Super Scene: {owner_label}"),
                    "A mixed Daslight lane was imported as Lighting; audio blocks on it are skipped",
                );
            }
            let layer_id = next_layer_id;
            next_layer_id = next_layer_id.saturating_add(1);
            layer_ids.insert(timeline.id(), layer_id);
            layers.push(TimelineLayerSummary {
                id: layer_id,
                label: non_empty_label(timeline.attribute("NAME"), &format!("Layer {}", order + 1)),
                order: u32::try_from(order).unwrap_or(u32::MAX),
                muted: parse_bool_attribute(timeline, "DASTLMUTED"),
                locked: parse_bool_attribute(timeline, "DASTLLOCKED"),
                solo: false,
                expanded: !parse_bool_attribute(timeline, "DASTLFOLDED"),
                kind,
            });
        }

        let mut audio_clips = Vec::new();
        let mut events = Vec::new();
        let mut duration_ms = 0_u64;

        for timeline in timeline_nodes.iter().copied() {
            let layer_id = *layer_ids
                .get(&timeline.id())
                .ok_or_else(|| "Daslight timeline layer mapping is missing".to_string())?;
            let layer_kind = layers
                .iter()
                .find(|layer| layer.id == layer_id)
                .map(|layer| layer.kind)
                .unwrap_or_default();
            for block in timeline
                .descendants()
                .filter(|node| node.has_tag_name("BLOCK"))
            {
                let start_ms = parse_u64_attribute(block, "START").unwrap_or(0);
                let end_ms = parse_u64_attribute(block, "END").unwrap_or(start_ms);
                let block_duration = end_ms.saturating_sub(start_ms);
                duration_ms = duration_ms.max(end_ms);
                match block.attribute("TYPE") {
                    Some("2") => {
                        if layer_kind != TimelineLayerKind::Audio {
                            report.skipped.add(
                                1,
                                format!(
                                    "Audio block: {}",
                                    block.attribute("NAME").unwrap_or("Untitled")
                                ),
                                "Audio block was on a mixed non-Audio lane",
                            );
                            continue;
                        }
                        let path = block.attribute("DASTLMEDIAPATH").unwrap_or("").trim();
                        if path.is_empty() || block_duration == 0 {
                            report.skipped.add(
                                1,
                                format!(
                                    "Audio block: {}",
                                    block.attribute("NAME").unwrap_or("Untitled")
                                ),
                                "Audio path or duration was empty",
                            );
                            continue;
                        }
                        let fade_in = parse_u64_attribute(block, "FADEIN")
                            .unwrap_or(0)
                            .min(block_duration);
                        let fade_out = parse_u64_attribute(block, "FADEOUT")
                            .unwrap_or(0)
                            .min(block_duration.saturating_sub(fade_in));
                        let position = parse_i64_attribute(block, "POSITION").unwrap_or(0);
                        if position < 0 {
                            report.approximate.add(
                                1,
                                format!(
                                    "Audio block: {}",
                                    block.attribute("NAME").unwrap_or("Untitled")
                                ),
                                "Negative source offset was clamped to zero",
                            );
                        }
                        audio_clips.push(TimelineAudioClipSummary {
                            id: next_audio_clip_id,
                            layer_id,
                            path: path.to_string(),
                            start_ms,
                            offset_ms: u64::try_from(position.max(0)).unwrap_or(0),
                            duration_ms: block_duration,
                            gain: 1.0,
                            fade_in_ms: fade_in,
                            fade_out_ms: fade_out,
                        });
                        next_audio_clip_id = next_audio_clip_id.saturating_add(1);
                        report.summary.timeline_audio_clips =
                            report.summary.timeline_audio_clips.saturating_add(1);
                        report.converted.add(
                            1,
                            format!(
                                "Audio block: {}",
                                block.attribute("NAME").unwrap_or("Untitled")
                            ),
                            format!("{start_ms}..{end_ms} ms"),
                        );
                        if !Path::new(path).is_file() {
                            report.summary.missing_audio_files =
                                report.summary.missing_audio_files.saturating_add(1);
                            let warning = format!(
                                "Timeline audio file is unavailable and must be relinked: {path}"
                            );
                            report.warnings.push(warning.clone());
                            report.skipped.add(
                                1,
                                format!(
                                    "Audio block: {}",
                                    block.attribute("NAME").unwrap_or("Untitled")
                                ),
                                warning,
                            );
                        }
                    }
                    Some("1") => {
                        let Some(source_uid) = block.attribute("SCENEUUID") else {
                            report.skipped.add(
                                1,
                                format!(
                                    "Scene block: {}",
                                    block.attribute("NAME").unwrap_or("Untitled")
                                ),
                                "SCENEUUID was missing",
                            );
                            continue;
                        };
                        let Some(&source_index) = scene_indices.get(source_uid) else {
                            report.skipped.add(
                                1,
                                format!(
                                    "Scene block: {}",
                                    block.attribute("NAME").unwrap_or("Untitled")
                                ),
                                format!("SCENEUUID {source_uid} did not resolve"),
                            );
                            continue;
                        };
                        if block_duration == 0 {
                            report.skipped.add(
                                1,
                                format!(
                                    "Scene block: {}",
                                    block.attribute("NAME").unwrap_or("Untitled")
                                ),
                                "Scene block duration was zero",
                            );
                            continue;
                        }
                        let speed = parse_f32_attribute(block, "SPEED")
                            .filter(|speed| speed.is_finite() && *speed > 0.0)
                            .unwrap_or(1.0);
                        let conform = parse_bool_attribute(block, "CONFORM_TO_TEMPO");
                        let allow_loop = parse_bool_attribute(block, "ALLOWLOOP");
                        let fade_in = parse_u64_attribute(block, "FADEIN")
                            .unwrap_or(0)
                            .min(block_duration);
                        let fade_out = parse_u64_attribute(block, "FADEOUT")
                            .unwrap_or(0)
                            .min(block_duration.saturating_sub(fade_in));
                        let position = parse_i64_attribute(block, "POSITION").unwrap_or(0);
                        let source_offset_ms = position;
                        events.push(TimelineCueEventSummary {
                            id: next_event_id,
                            cue_id: cues[source_index].id,
                            time_ms: start_ms,
                            time_beats: None,
                            track: TimelineTrackKind::Lighting,
                            layer_id: Some(layer_id),
                            duration_ms: block_duration,
                            duration_beats: None,
                            conform_to_tempo: conform && tempo_driven,
                            loop_fill: allow_loop,
                            source_offset_ms,
                            rate: Some(speed),
                            fade_in_ms: fade_in,
                            fade_out_ms: fade_out,
                            loop_count: 1,
                            jump_to_event_id: None,
                        });
                        next_event_id = next_event_id.saturating_add(1);
                        report.summary.timeline_scene_blocks =
                            report.summary.timeline_scene_blocks.saturating_add(1);
                        report.converted.add(
                            1,
                            format!(
                                "Scene block: {}",
                                block.attribute("NAME").unwrap_or("Untitled")
                            ),
                            format!(
                                "{start_ms}..{end_ms} ms -> cue {}{}; speed={speed}; loop={allow_loop}; conform={conform}; parent_bpm_driven={owner_bpm_driven}; grid_bpm={grid_bpm:.3}",
                                cues[source_index].label,
                                if source_offset_ms != 0 {
                                    format!(" source position {source_offset_ms:+} ms")
                                } else {
                                    String::new()
                                }
                            ),
                        );
                    }
                    Some(other) => report.skipped.add(
                        1,
                        format!("Timeline block type {other}"),
                        "Unsupported Daslight timeline block type",
                    ),
                    None => {}
                }
            }
        }

        cues[owner_index].child_timeline = Some(ChildTimelineSummary {
            layers,
            events,
            automations: Vec::new(),
            video_automations: Vec::new(),
            audio: None,
            audio_clips,
            tempo_driven,
            metronome_enabled: false,
            count_in_beats: 4,
            duration_ms,
        });
        cues[owner_index]
            .notes
            .push_str(" | Daslight Super Scene child timeline imported");
    }
    if let Some(bpm) = first_grid_bpm {
        snapshot.clock.bpm = bpm;
    }
    Ok(())
}

fn configure_disabled_dmx_routes(snapshot: &mut EngineSnapshot) {
    let mut universes = snapshot
        .fixtures
        .iter()
        .map(|fixture| fixture.universe)
        .collect::<Vec<_>>();
    universes.sort_unstable();
    universes.dedup();
    if universes.is_empty() {
        universes.push(0);
    }
    snapshot.dmx_outputs = universes
        .iter()
        .copied()
        .map(|universe| {
            let mut output = DmxOutputConfig::default();
            output.enabled = false;
            output.universe = universe;
            output
        })
        .collect();
    snapshot.output = snapshot.dmx_outputs[0].clone();
    snapshot.dmx_previews = universes
        .into_iter()
        .map(|universe| DmxUniversePreview {
            universe,
            values: vec![0; 512],
        })
        .collect();
    snapshot.dmx_preview = vec![0; 512];
}

fn median_daslight_fixture_size(fixture_sizes: &[f32]) -> f32 {
    let mut sizes = fixture_sizes
        .iter()
        .copied()
        .filter(|size| size.is_finite() && *size > f32::EPSILON)
        .collect::<Vec<_>>();
    if sizes.is_empty() {
        return DVC_DEFAULT_FIXTURE_SIZE;
    }
    sizes.sort_by(f32::total_cmp);
    let midpoint = sizes.len() / 2;
    if sizes.len() % 2 == 0 {
        (sizes[midpoint - 1] + sizes[midpoint]) * 0.5
    } else {
        sizes[midpoint]
    }
}

fn scale_fixture_layout(
    fixtures: &mut [PatchedFixtureSummary],
    fixture_sizes: &[f32],
) -> StageMapConfig {
    if fixtures.is_empty() {
        return StageMapConfig::default();
    }
    let min_x = fixtures
        .iter()
        .map(|fixture| fixture.position.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = fixtures
        .iter()
        .map(|fixture| fixture.position.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_z = fixtures
        .iter()
        .map(|fixture| fixture.position.z)
        .fold(f32::INFINITY, f32::min);
    let max_z = fixtures
        .iter()
        .map(|fixture| fixture.position.z)
        .fold(f32::NEG_INFINITY, f32::max);
    let scale =
        SYNDOCAL_STANDARD_FIXTURE_GLYPH_WORLD_SIZE / median_daslight_fixture_size(fixture_sizes);
    let center_x = (min_x + max_x) * 0.5;
    let center_z = (min_z + max_z) * 0.5;
    let mut layout_half_extent = 0.0_f32;
    for fixture in fixtures {
        fixture.position.x = (fixture.position.x - center_x) * scale;
        fixture.position.z = (fixture.position.z - center_z) * scale;
        layout_half_extent = layout_half_extent
            .max(fixture.position.x.abs())
            .max(fixture.position.z.abs());
    }
    // A square locked reference preserves the uniform X/Z transform and adds
    // one standard glyph of breathing room beyond the outer fixture centers.
    let stage_half_extent = (layout_half_extent + SYNDOCAL_STANDARD_FIXTURE_GLYPH_WORLD_SIZE)
        .max(SYNDOCAL_STANDARD_FIXTURE_GLYPH_WORLD_SIZE);
    StageMapConfig {
        locked: true,
        min_x: -stage_half_extent,
        max_x: stage_half_extent,
        min_z: -stage_half_extent,
        max_z: stage_half_extent,
    }
}

fn decode_qcompress_base64(encoded: &str) -> Result<Vec<u8>, String> {
    let compressed = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|error| format!("PATCHS DATA base64 is invalid: {error}"))?;
    if compressed.len() < 6 {
        return Err("PATCHS DATA is shorter than its qCompress wrapper".to_string());
    }
    let expected = u32::from_be_bytes(
        compressed[0..4]
            .try_into()
            .map_err(|_| "PATCHS DATA length header is invalid".to_string())?,
    ) as usize;
    if expected > DVC_PATCH_MAX_BYTES {
        return Err(format!(
            "PATCHS DATA expands to {expected} bytes; the limit is {DVC_PATCH_MAX_BYTES} bytes"
        ));
    }
    let decoder = ZlibDecoder::new(&compressed[4..]);
    let mut output = Vec::with_capacity(expected.min(1024 * 1024));
    decoder
        .take((DVC_PATCH_MAX_BYTES + 1) as u64)
        .read_to_end(&mut output)
        .map_err(|error| format!("PATCHS DATA zlib stream is invalid: {error}"))?;
    if output.len() != expected {
        return Err(format!(
            "PATCHS DATA expected {expected} bytes after inflate but produced {}",
            output.len()
        ));
    }
    Ok(output)
}

fn decode_fixture_data(
    encoded: &str,
    expected_channels: usize,
) -> Result<FixtureDataValues, String> {
    let compressed = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|error| format!("FIXTUREDATA base64 is invalid: {error}"))?;
    let inflated = inflate_sync_flush_tolerant(&compressed, DVC_FIXTURE_DATA_MAX_BYTES)?;
    if inflated.len() < 6 {
        return Err("FIXTUREDATA is shorter than its header".to_string());
    }
    // Verified across the five read-only local Daslight projects (525 payloads,
    // DVC-2 census): a BE channel count, one BE u16 per physical channel
    // (0xFFFF = unwritten), a BE record count, then one fixed 27-byte row per
    // fixture BEAM (the record count matches the patch beam count: 8 for the
    // eight-segment mega bar rgba, 1 everywhere else - these are NOT steps; no
    // step container exists in any specimen). Each row is 13 big-endian IEEE
    // half-precision floats plus one trailing mode byte (0x00 and 0x02 are the
    // observed values): -1.0 marks an unset slot
    // and set slots hold normalized 0..1 feature values that mirror the
    // channel section (slot 0..2 = RGB, slot 11 = dimmer). In the verified
    // single-beam case Daslight truncates these half-floats for live DMX; the
    // channel section remains the fallback for every other mapping.
    let channel_count = u16::from_be_bytes([inflated[0], inflated[1]]) as usize;
    if channel_count != expected_channels {
        return Err(format!(
            "channel count {channel_count} does not match profile channel count {expected_channels}"
        ));
    }
    let values_end = 2usize
        .checked_add(channel_count.saturating_mul(2))
        .ok_or_else(|| "FIXTUREDATA channel section overflowed".to_string())?;
    let record_count_end = values_end
        .checked_add(4)
        .ok_or_else(|| "FIXTUREDATA record header overflowed".to_string())?;
    if inflated.len() < record_count_end {
        return Err("FIXTUREDATA ends before its record count".to_string());
    }
    let record_count = u32::from_be_bytes(
        inflated[values_end..record_count_end]
            .try_into()
            .map_err(|_| "FIXTUREDATA record count is invalid".to_string())?,
    ) as usize;
    if !(1..=64).contains(&record_count) {
        return Err(format!(
            "record count {record_count} is outside the verified 1..64 range"
        ));
    }
    let record_payload = inflated.len() - record_count_end;
    if record_payload % record_count != 0 {
        return Err(format!(
            "{} record bytes are not divisible by record count {record_count}",
            record_payload
        ));
    }
    let record_bytes = record_payload / record_count;
    if record_bytes != DVC_FIXTURE_RECORD_BYTES {
        return Err(format!(
            "record row is {record_bytes} bytes; only the verified {DVC_FIXTURE_RECORD_BYTES}-byte row is accepted"
        ));
    }
    let mut values = Vec::with_capacity(channel_count);
    for bytes in inflated[2..values_end].chunks_exact(2) {
        let value = u16::from_be_bytes([bytes[0], bytes[1]]);
        match value {
            u16::MAX => values.push(None),
            0..=255 => values.push(Some(value as u8)),
            _ => {
                return Err(format!(
                    "channel value {value} is neither an 8-bit DMX value nor 0xFFFF"
                ))
            }
        }
    }
    // Beam-row interpretation is deliberately NON-fatal: an unrecognized row
    // variant must never discard the channel-section payload - it only
    // downgrades to "rows not interpreted".
    let (beam_features, beam_feature_error) =
        match interpret_beam_features(&inflated[record_count_end..]) {
            Ok(features) => (Some(features), None),
            Err(error) => (None, Some(error)),
        };
    Ok(FixtureDataValues {
        values,
        record_bytes,
        beam_features,
        beam_feature_error,
    })
}

fn interpret_beam_features(rows: &[u8]) -> Result<Vec<[f32; DVC_BEAM_FEATURE_SLOTS]>, String> {
    let mut beam_features = Vec::with_capacity(rows.len() / DVC_FIXTURE_RECORD_BYTES);
    for record in rows.chunks_exact(DVC_FIXTURE_RECORD_BYTES) {
        let mode = record[DVC_FIXTURE_RECORD_BYTES - 1];
        if !matches!(mode, 0x00 | 0x02) {
            return Err(format!(
                "beam record mode byte 0x{mode:02X} is outside the verified 0x00/0x02 set"
            ));
        }
        let mut features = [0.0_f32; DVC_BEAM_FEATURE_SLOTS];
        for (slot, bytes) in record[..DVC_BEAM_FEATURE_SLOTS * 2]
            .chunks_exact(2)
            .enumerate()
        {
            let value = half_bits_to_f32(u16::from_be_bytes([bytes[0], bytes[1]]));
            if !beam_feature_is_unset(value) && !(0.0..=1.001).contains(&value) {
                return Err(format!(
                    "beam feature slot {slot} value {value} is outside the verified -1 / 0..1 range"
                ));
            }
            features[slot] = value;
        }
        beam_features.push(features);
    }
    Ok(beam_features)
}

fn half_bits_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0_f32 } else { 1.0 };
    let exponent = (bits >> 10) & 0x1F;
    let fraction = bits & 0x3FF;
    let magnitude = match exponent {
        0 => f32::from(fraction) / 1024.0 * 2.0_f32.powi(-14),
        31 => f32::NAN,
        _ => (1.0 + f32::from(fraction) / 1024.0) * 2.0_f32.powi(i32::from(exponent) - 15),
    };
    sign * magnitude
}

fn beam_feature_is_unset(value: f32) -> bool {
    (value + 1.0).abs() <= 1.0e-3
}

// Cross-check decoded beam feature slots against the authoritative channel
// section. Only unambiguous cases are compared: a single-beam payload, a slot
// whose attribute exists exactly once in the profile, an 8-bit single-offset
// binding, and a written header byte. Multi-beam rows target per-segment
// channels whose beam-to-channel mapping is not verified, so they are skipped.
fn beam_feature_mismatch_details(
    profile: &ParsedProfile,
    raw_values: &[Option<u8>],
    beam_features: &[[f32; DVC_BEAM_FEATURE_SLOTS]],
) -> (usize, Vec<String>) {
    let mut checked = 0;
    let mut mismatches = Vec::new();
    let [features] = beam_features else {
        return (0, mismatches);
    };
    for (slot, attribute) in DVC_BEAM_FEATURE_BINDINGS {
        let feature = features[slot];
        if beam_feature_is_unset(feature) {
            continue;
        }
        let mut bindings = profile
            .bindings
            .iter()
            .filter(|binding| binding.attribute == attribute);
        let (Some(binding), None) = (bindings.next(), bindings.next()) else {
            continue;
        };
        let (AttributeResolution::EightBit, [offset]) =
            (&binding.resolution, binding.raw_offsets.as_slice())
        else {
            continue;
        };
        let Some(raw) = raw_values.get(*offset).copied().flatten() else {
            continue;
        };
        checked += 1;
        let expected = (feature * 255.0).round() as i32;
        if (expected - i32::from(raw)).abs() > 2 {
            mismatches.push(format!(
                "{attribute}: channel byte {raw} vs beam feature {expected}"
            ));
        }
    }
    (checked, mismatches)
}

fn inflate_sync_flush_tolerant(input: &[u8], max_output: usize) -> Result<Vec<u8>, String> {
    if input.len() < 2 {
        return Err("zlib payload is too short".to_string());
    }
    let mut decompressor = Decompress::new(true);
    let mut output = Vec::new();
    let mut input_offset = 0usize;
    let mut buffer = [0_u8; 8192];
    let mut stream_end = false;
    loop {
        let before_in = decompressor.total_in();
        let before_out = decompressor.total_out();
        let status = decompressor
            .decompress(&input[input_offset..], &mut buffer, FlushDecompress::Sync)
            .map_err(|error| format!("zlib payload is invalid: {error}"))?;
        let consumed = usize::try_from(decompressor.total_in() - before_in).unwrap_or(usize::MAX);
        let written = usize::try_from(decompressor.total_out() - before_out).unwrap_or(usize::MAX);
        input_offset = input_offset.saturating_add(consumed).min(input.len());
        if output.len().saturating_add(written) > max_output {
            return Err(format!(
                "zlib payload expands beyond the {max_output}-byte safety limit"
            ));
        }
        output.extend_from_slice(&buffer[..written]);
        if status == Status::StreamEnd {
            stream_end = true;
            break;
        }
        if consumed == 0 && written == 0 {
            break;
        }
        if input_offset == input.len() && written < buffer.len() {
            break;
        }
    }
    if !stream_end && !input.ends_with(&[0x00, 0x00, 0xff, 0xff]) {
        return Err("unfinished zlib payload has no sync-flush terminator".to_string());
    }
    if input_offset != input.len() {
        return Err(format!(
            "zlib payload left {} unread byte(s)",
            input.len() - input_offset
        ));
    }
    Ok(output)
}

fn count_ignored_sections(root: Node<'_, '_>, report: &mut DvcImportReport) {
    if let Some(devices) = direct_child(root, "DEVICES") {
        let count = element_children(devices).count();
        if count > 0 {
            report.unsupported.add(
                count,
                "Daslight hardware devices",
                "Hardware bindings are device-specific; configure Syndocal output routes explicitly",
            );
        }
    }
}

fn dvc_argb_to_rgb(value: &str) -> Option<String> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 8 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", hex[2..].to_ascii_lowercase()))
}

fn daslight_numeric_color(value: &str) -> Option<String> {
    let color = value.parse::<u32>().ok()? & 0x00ff_ffff;
    Some(format!("#{color:06x}"))
}

fn direct_child<'a, 'input>(node: Node<'a, 'input>, tag: &str) -> Option<Node<'a, 'input>> {
    element_children(node).find(|child| child.has_tag_name(tag))
}

fn element_children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(|child| child.is_element())
}

/// Stable seed for corrected random DVC generators. Daslight's qrand state is
/// process history and is absent from the file, so identity comes only from
/// the source scene/rack/effect location and never from Syndocal's allocated
/// runtime effect ID.
fn dvc_corrected_rng_seed(
    scene: Node<'_, '_>,
    rack: Node<'_, '_>,
    effect: Node<'_, '_>,
    generator_id: u16,
) -> u32 {
    let rack_ordinal = rack
        .parent()
        .and_then(|parent| element_children(parent).position(|node| node.id() == rack.id()))
        .unwrap_or_default();
    let effect_ordinal = element_children(rack)
        .filter(|node| node.has_tag_name("EFFECT"))
        .position(|node| node.id() == effect.id())
        .unwrap_or_default();
    let identity = format!(
        "scene={}|{};rack={rack_ordinal}|{};effect={effect_ordinal}|{}|{generator_id}",
        scene.attribute("DASUID").unwrap_or_default(),
        scene.attribute("NAME").unwrap_or_default(),
        rack.attribute("TYPE").unwrap_or_default(),
        effect.attribute("TYPE").unwrap_or_default(),
    );
    let mut hash = 0x811C_9DC5_u32;
    for byte in identity.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    if hash == 0 {
        0xA341_316C
    } else {
        hash
    }
}

fn required_attribute<'a, 'input>(
    node: Node<'a, 'input>,
    attribute: &str,
    owner: &str,
) -> Result<&'a str, String> {
    node.attribute(attribute)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Daslight {owner} is missing {attribute}"))
}

fn parse_required_u16(node: Node<'_, '_>, attribute: &str, owner: &str) -> Result<u16, String> {
    required_attribute(node, attribute, owner)?
        .parse::<u16>()
        .map_err(|_| format!("Daslight {owner} has invalid {attribute}"))
}

fn parse_u64_attribute(node: Node<'_, '_>, attribute: &str) -> Option<u64> {
    node.attribute(attribute)?.trim().parse::<u64>().ok()
}

fn parse_i64_attribute(node: Node<'_, '_>, attribute: &str) -> Option<i64> {
    node.attribute(attribute)?.trim().parse::<i64>().ok()
}

fn parse_f32_attribute(node: Node<'_, '_>, attribute: &str) -> Option<f32> {
    node.attribute(attribute)?.trim().parse::<f32>().ok()
}

fn parse_bool_attribute(node: Node<'_, '_>, attribute: &str) -> bool {
    node.attribute(attribute)
        .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE"))
}

fn non_empty_label(value: Option<&str>, fallback: &str) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, io::Write, net::UdpSocket, time::Duration};

    use base64::Engine as _;
    use engine::{EngineCommand, EngineHandle};
    use flate2::{write::ZlibEncoder, Compress, Compression, FlushCompress};

    use super::*;

    fn first_json_difference(
        before: &serde_json::Value,
        after: &serde_json::Value,
        path: &str,
    ) -> Option<String> {
        match (before, after) {
            (serde_json::Value::Number(left), serde_json::Value::Number(right))
                if left.is_f64() || right.is_f64() =>
            {
                let left = left.as_f64().unwrap_or(f64::NAN);
                let right = right.as_f64().unwrap_or(f64::NAN);
                if left.is_finite() && right.is_finite() && (left - right).abs() <= 1.0e-9 {
                    None
                } else {
                    Some(format!("{path}: {left:?} -> {right:?}"))
                }
            }
            (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
                let keys = left
                    .keys()
                    .chain(right.keys())
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                for key in keys {
                    let next_path = format!("{path}.{key}");
                    match (left.get(key), right.get(key)) {
                        (Some(left), Some(right)) => {
                            if let Some(difference) = first_json_difference(left, right, &next_path)
                            {
                                return Some(difference);
                            }
                        }
                        (left, right) => {
                            return Some(format!("{next_path}: {left:?} -> {right:?}"));
                        }
                    }
                }
                None
            }
            (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
                if left.len() != right.len() {
                    return Some(format!("{path}.length: {} -> {}", left.len(), right.len()));
                }
                for (index, (left, right)) in left.iter().zip(right).enumerate() {
                    if let Some(difference) =
                        first_json_difference(left, right, &format!("{path}[{index}]"))
                    {
                        return Some(difference);
                    }
                }
                None
            }
            _ if before == after => None,
            _ => Some(format!("{path}: {before:?} -> {after:?}")),
        }
    }

    fn qcompress(value: &[u8]) -> String {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(value).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut wrapped = Vec::with_capacity(compressed.len() + 4);
        wrapped.extend_from_slice(&(value.len() as u32).to_be_bytes());
        wrapped.extend_from_slice(&compressed);
        base64::engine::general_purpose::STANDARD.encode(wrapped)
    }

    fn sync_flush(value: &[u8]) -> Vec<u8> {
        let mut compressor = Compress::new(Compression::default(), true);
        let mut output = vec![0_u8; value.len().saturating_mul(2).saturating_add(128)];
        let status = compressor
            .compress(value, &mut output, FlushCompress::Sync)
            .unwrap();
        assert!(matches!(status, Status::Ok | Status::BufError));
        output.truncate(compressor.total_out() as usize);
        output
    }

    fn synthetic_dvc() -> String {
        synthetic_dvc_with(255, 0x3C00)
    }

    fn synthetic_nested_dvc() -> String {
        let nested = r##"<SCENE DASUID="scene-3" NAME="Nested Super" COLOR="#ff778899" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><FIXTUREDATAS NB="0"/><RACKS><RACK GRID_BPM="96"><TIMELINES><TIMELINE DASUID="lane-nested" NAME="Nested Lighting" INDEX="0" DASTLLOCKED="0" DASTLMUTED="0" DASTLFOLDED="0"><BLOCKS><BLOCK TYPE="1" NAME="Super" START="0" END="1000" POSITION="0" FADEIN="0" FADEOUT="0" SPEED="1" ALLOWLOOP="0" CONFORM_TO_TEMPO="0" SCENEUUID="scene-2"/></BLOCKS></TIMELINE></TIMELINES></RACK></RACKS></SCENE>"##;
        synthetic_dvc().replace("</BANK></SCENES>", &format!("{nested}</BANK></SCENES>"))
    }

    fn synthetic_dvc_with(dimmer_byte: u16, dimmer_feature_bits: u16) -> String {
        let patch = r#"<PATCH NBFIXTURE="1"><FIXTURES><SSLLIBRARY SSLFIXUID="profile-1" SSLNAME="Test/Dimmer.ssl2"><SSLPROPERTIES SSLBEAMOPENING="20"/><SSLMODES SSLNBMODE="1"><SSLMODE SSLMODEINDEX="0" SSLNBCHANNEL="1"><SSLCHANNEL SSLCHANNELTYPE="7" SSLCHANNELNAME="Dimmer" SSLCHANNELMSB="0" SSLCHANNELLSB="0"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Dimmer" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL></SSLMODE></SSLMODES></SSLLIBRARY><FIXTURE DASUID="fixture-1" NAME="Dimmer 1" ADDRESS="1" UNIVERS="1" POSX="0" POSY="0" ANGLE="0"/></FIXTURES></PATCH>"#;
        let mut fixture_data = Vec::new();
        fixture_data.extend_from_slice(&1_u16.to_be_bytes());
        fixture_data.extend_from_slice(&dimmer_byte.to_be_bytes());
        fixture_data.extend_from_slice(&1_u32.to_be_bytes());
        let mut record = [0_u8; DVC_FIXTURE_RECORD_BYTES];
        for slot in 0..DVC_BEAM_FEATURE_SLOTS {
            let bits: u16 = if slot == 11 {
                dimmer_feature_bits
            } else {
                0xBC00
            };
            record[slot * 2..slot * 2 + 2].copy_from_slice(&bits.to_be_bytes());
        }
        fixture_data.extend_from_slice(&record);
        let fixture_data =
            base64::engine::general_purpose::STANDARD.encode(sync_flush(&fixture_data));
        format!(
            r##"<DLMFILE TYPE="Daslight" VERSION="5" DASBUILD="test" VERSIONFILE="2"><PATCHS DATA="{}"/><FIXTUREGROUPS><FIXTUREGROUP DASUID="group-1" NAME="All"><FIXTURES><FIXTURE DASUID="fixture-1"/></FIXTURES></FIXTUREGROUP></FIXTUREGROUPS><SCENES><BANK DASUID="bank-1" NAME="Bank 1" COLOR="#ff112233"><SCENE DASUID="scene-1" NAME="Static" COLOR="#ff112233" FADE_IN="100" FADE_OUT="200" LOOP="0" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><FIXTUREDATAS NB="1"><FIXTUREDATA FIXTURE="fixture-1" DATA="{}"/></FIXTUREDATAS></SCENE><SCENE DASUID="scene-2" NAME="Super" COLOR="#ff445566" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><FIXTUREDATAS NB="0"/><RACKS><RACK><TIMELINES><TIMELINE DASUID="lane-a" NAME="Audio" INDEX="0" DASTLLOCKED="0" DASTLMUTED="0" DASTLFOLDED="0"><BLOCKS><BLOCK TYPE="2" NAME="Track" START="0" END="1000" POSITION="0" FADEIN="0" FADEOUT="0" DASTLMEDIAPATH="C:/missing.mp3" BPM="120"/></BLOCKS></TIMELINE><TIMELINE DASUID="lane-l" NAME="Lighting" INDEX="1" DASTLLOCKED="0" DASTLMUTED="0" DASTLFOLDED="0"><BLOCKS><BLOCK TYPE="1" NAME="Static" START="0" END="1000" POSITION="0" FADEIN="100" FADEOUT="100" SPEED="1" ALLOWLOOP="1" CONFORM_TO_TEMPO="1" SCENEUUID="scene-1"/></BLOCKS></TIMELINE></TIMELINES></RACK></RACKS></SCENE></BANK></SCENES><SHORTCUTS/><TOUCH/><DEVICES/></DLMFILE>"##,
            qcompress(patch.as_bytes()),
            fixture_data,
        )
        .replace(
            "<RACK><TIMELINES>",
            "<RACK GRID_BPM=\"96\"><TIMELINES>",
        )
        .replace("BPM=\"120\"", "BPM=\"143\"")
        .replace(
            "<SHORTCUTS/><TOUCH/>",
            r#"<SHORTCUTS><SHORTCUT TYPE="2"><EVENT DATA="touch-control-group:0"/><ACTION TYPE="30" TARGET="group-1" TARGETINDEX="0"/><SETTINGS MIN="0" MAX="1" FLASH="1"/></SHORTCUT><SHORTCUT TYPE="2"><EVENT DATA="touch-control-cue:0"/><ACTION TYPE="107" TARGET="scene-1" TARGETINDEX="0"/><SETTINGS MIN="0" MAX="1" FLASH="1"/></SHORTCUT><SHORTCUT TYPE="2"><EVENT DATA="touch-control-tap:0"/><ACTION TYPE="55" TARGETINDEX="-1"/><SETTINGS MIN="0" MAX="1" FLASH="1"/></SHORTCUT></SHORTCUTS><TOUCH><TOUCHPAGE DASUID="touch-page-1" NAME="Page 1" TOUCHCONTROL="3"><TOUCHCONTROL DASUID="touch-control-group" NAME="All" TYPE="2" GRIDWIDTH="1" GRIDHEIGHT="1" GRIDPOSX="0" GRIDPOSY="0"/><TOUCHCONTROL DASUID="touch-control-cue" NAME="Static" TYPE="2" GRIDWIDTH="1" GRIDHEIGHT="1" GRIDPOSX="1" GRIDPOSY="0"/><TOUCHCONTROL DASUID="touch-control-tap" NAME="Tap Tempo" TYPE="2" GRIDWIDTH="1" GRIDHEIGHT="1" GRIDPOSX="2" GRIDPOSY="0"/></TOUCHPAGE></TOUCH>"#,
        )
    }

    fn synthetic_midi_shortcuts_dvc() -> String {
        synthetic_dvc().replacen(
            "<SHORTCUTS>",
            r#"<SHORTCUTS><SHORTCUT TYPE="1"><EVENT DATA="144:2:60:127:Controller:Port"/><ACTION TYPE="107" TARGET="scene-1" TARGETINDEX="0"/><SETTINGS MIN="0" MAX="1" FLASH="0" OUT="144:2:60:5:Controller" OUT1="144:2:60:1:Controller"/></SHORTCUT><SHORTCUT TYPE="1"><EVENT DATA="144:2:61:127:Controller"/><ACTION TYPE="107" TARGET="scene-2" TARGETINDEX="1"/><SETTINGS MIN="0" MAX="1" FLASH="1"/></SHORTCUT><SHORTCUT TYPE="1"><EVENT DATA="176:3:20:64:Controller"/><ACTION TYPE="55" TARGETINDEX="-1"/><SETTINGS MIN="0" MAX="1" FLASH="1"/></SHORTCUT><SHORTCUT TYPE="1"><EVENT DATA="144:2:62:127:Controller"/><ACTION TYPE="108" TARGET="scene-1" TARGETINDEX="0"/><SETTINGS MIN="0" MAX="1" FLASH="1"/></SHORTCUT><SHORTCUT TYPE="1"><EVENT DATA="144:2:63:127:Controller"/><ACTION TYPE="113" TARGET="scene-2" TARGETINDEX="1" TARGETINDEX2="0"/><SETTINGS MIN="0.2" MAX="0.8" FLASH="1"/></SHORTCUT><SHORTCUT TYPE="1"><EVENT DATA="144:2:64:127:Controller"/><ACTION TYPE="110" TARGET="scene-2" TARGETINDEX="1"/><SETTINGS MIN="0" MAX="1" FLASH="1"/></SHORTCUT><SHORTCUT TYPE="1"><EVENT DATA="144:2:65:127:Controller"/><ACTION TYPE="109" TARGET="scene-1" TARGETINDEX="0"/><SETTINGS MIN="0" MAX="1" FLASH="0"/></SHORTCUT><SHORTCUT TYPE="1"><EVENT DATA="176:3:21:64:Controller"/><ACTION TYPE="229" TARGETINDEX="1"/><SETTINGS MIN="0" MAX="1" FLASH="0"/></SHORTCUT><SHORTCUT TYPE="1"><EVENT DATA="144:2:66:127:Controller"/><ACTION TYPE="999" TARGET="scene-1" TARGETINDEX="0"/><SETTINGS MIN="0" MAX="1" FLASH="0"/></SHORTCUT>"#,
            1,
        )
    }

    fn synthetic_dmx_shortcuts_dvc() -> String {
        synthetic_dvc().replacen(
            "<SHORTCUTS>",
            r#"<SHORTCUTS><SHORTCUT TYPE="3"><EVENT DATA="/dmx/2/25:5"/><ACTION TYPE="210" TARGET="profile-1:0"><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0"/></BEAMS></ACTION><SETTINGS SMODE="1" CMODE="1" TMODE="0" MIN="0.25" MAX="0.75" INC="0.001" LOOP="0" FLASH="0" INV="0"/></SHORTCUT><SHORTCUT TYPE="3"><EVENT DATA="/dmx/1/1:4"/><ACTION TYPE="210" TARGET="profile-1:0"><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0"/></BEAMS></ACTION><SETTINGS SMODE="1" CMODE="1" TMODE="0" MIN="0" MAX="1" INC="0.001" LOOP="0" FLASH="0" INV="0"/></SHORTCUT><SHORTCUT TYPE="3"><EVENT DATA="/dmx/1/2:5"/><ACTION TYPE="211" TARGET="profile-1:0"><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0"/></BEAMS></ACTION><SETTINGS SMODE="1" CMODE="1" TMODE="0" MIN="0" MAX="1" INC="0.001" LOOP="0" FLASH="0" INV="0"/></SHORTCUT><SHORTCUT TYPE="3"><EVENT DATA="/dmx/1/3:5"/><ACTION TYPE="210" TARGET="profile-1:0"><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0"/></BEAMS></ACTION><SETTINGS SMODE="2" CMODE="1" TMODE="0" MIN="0" MAX="1" INC="0.001" LOOP="0" FLASH="0" INV="0"/></SHORTCUT><SHORTCUT TYPE="3"><EVENT DATA="/dmx/1/4:5"/><ACTION TYPE="210" TARGET="profile-1:0"><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0"/></BEAMS></ACTION><SETTINGS SMODE="1" CMODE="1" TMODE="0" MIN="0" MAX="1" INC="0.01" LOOP="0" FLASH="0" INV="0"/></SHORTCUT>"#,
            1,
        )
    }

    fn synthetic_layout_dvc() -> String {
        let patch = r#"<PATCH NBFIXTURE="3"><FIXTURES><SSLLIBRARY SSLFIXUID="profile-layout" SSLNAME="Test/Layout.ssl2"><SSLPROPERTIES SSLBEAMOPENING="20"/><SSLMODES SSLNBMODE="1"><SSLMODE SSLMODEINDEX="0" SSLNBCHANNEL="1"><SSLCHANNEL SSLCHANNELTYPE="7" SSLCHANNELNAME="Dimmer" SSLCHANNELMSB="0" SSLCHANNELLSB="0"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Dimmer" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL></SSLMODE></SSLMODES></SSLLIBRARY><FIXTURE DASUID="layout-1" NAME="Layout 1" ADDRESS="1" UNIVERS="1" SIZE="20" POSX="100" POSY="20" ANGLE="15"/><FIXTURE DASUID="layout-2" NAME="Layout 2" ADDRESS="2" UNIVERS="1" SIZE="30" POSX="220" POSY="-40" ANGLE="75"/><FIXTURE DASUID="layout-3" NAME="Layout 3" ADDRESS="3" UNIVERS="1" SIZE="40" POSX="400" POSY="80" ANGLE="195"/></FIXTURES></PATCH>"#;
        format!(
            r#"<DLMFILE TYPE="Daslight" VERSION="5" DASBUILD="test-layout" VERSIONFILE="2"><PATCHS DATA="{}"/><FIXTUREGROUPS/><SCENES/><SHORTCUTS/><TOUCH/><DEVICES/></DLMFILE>"#,
            qcompress(patch.as_bytes())
        )
    }

    fn synthetic_fx_dvc() -> String {
        let patch = r#"<PATCH NBFIXTURE="3"><FIXTURES><SSLLIBRARY SSLFIXUID="profile-1" SSLNAME="Test/Dimmer.ssl2"><SSLPROPERTIES SSLBEAMOPENING="20"/><SSLMODES SSLNBMODE="1"><SSLMODE SSLMODEINDEX="0" SSLNBCHANNEL="1"><SSLCHANNEL SSLCHANNELTYPE="7" SSLCHANNELNAME="Dimmer" SSLCHANNELMSB="0" SSLCHANNELLSB="0"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Dimmer" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL></SSLMODE></SSLMODES></SSLLIBRARY><FIXTURE DASUID="fixture-1" NAME="Dimmer 1" ADDRESS="1" UNIVERS="1" POSX="0" POSY="0" ANGLE="0"/><FIXTURE DASUID="fixture-2" NAME="Dimmer 2" ADDRESS="2" UNIVERS="1" POSX="1" POSY="0" ANGLE="0"/><FIXTURE DASUID="fixture-3" NAME="Dimmer 3" ADDRESS="3" UNIVERS="1" POSX="2" POSY="0" ANGLE="0"/></FIXTURES></PATCH>"#;
        format!(
            r##"<DLMFILE TYPE="Daslight" VERSION="5" DASBUILD="test-fx" VERSIONFILE="2"><PATCHS DATA="{}"/><FIXTUREGROUPS/><SCENES><BANK DASUID="bank-1" NAME="FX" COLOR="#ff112233"><SCENE DASUID="scene-chaser" NAME="Chaser 321" COLOR="#ff112233" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="0.5" PLAY_TRIGGER="0" PLAY_DIVISION="8"><FIXTUREDATAS NB="0"/><RACKS><RACK TYPE="3"><EFFECT TYPE="6" ID="321" DURATION="5000"><PARAMS NB="3"><PARAM TYPE="2" ID="10" VAL="1"/><PARAM TYPE="2" ID="11" VAL="1"/><PARAM TYPE="0" ID="12" VAL="2"/></PARAMS></EFFECT><BEAMS NB="3"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/><BEAM FIXTURE="fixture-3" BEAMID="0" IDSELECTION="3"/></BEAMS></RACK></RACKS></SCENE><SCENE DASUID="scene-curve" NAME="Curve 7" COLOR="#ff112233" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="8"><FIXTUREDATAS NB="0"/><RACKS><RACK TYPE="8"><EFFECT TYPE="5" ID="7" DURATION="5000"><PARAMS NB="5"><PARAM TYPE="0" ID="1" VAL="10"/><PARAM TYPE="1" ID="2" VAL="0.5"/><PARAM TYPE="1" ID="3" VAL="0.25"/><PARAM TYPE="1" ID="4" VAL="0.1"/><PARAM TYPE="1" ID="5" VAL="0"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE><SCENE DASUID="scene-random" NAME="Chaser 325" COLOR="#ff112233" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="8"><FIXTUREDATAS NB="0"/><RACKS><RACK TYPE="3"><EFFECT TYPE="6" ID="325" DURATION="1200"><PARAMS NB="5"><PARAM TYPE="2" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="1"/><PARAM TYPE="1" ID="13" VAL="50"/><PARAM TYPE="0" ID="14" VAL="1"/><PARAM TYPE="0" ID="15" VAL="2"/></PARAMS></EFFECT><BEAMS NB="3"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/><BEAM FIXTURE="fixture-3" BEAMID="0" IDSELECTION="3"/></BEAMS></RACK></RACKS></SCENE><SCENE DASUID="scene-skipped" NAME="Unconfirmed" COLOR="#ff112233" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="8"><FIXTUREDATAS NB="0"/><RACKS><RACK TYPE="2"><EFFECT TYPE="2" ID="133" DURATION="5000"><PARAMS NB="0"/></EFFECT></RACK><RACK TYPE="8"><EFFECT TYPE="5" ID="10" DURATION="5000"><PARAMS NB="0"/></EFFECT></RACK></RACKS></SCENE></BANK></SCENES><SHORTCUTS/><TOUCH/><DEVICES/></DLMFILE>"##,
            qcompress(patch.as_bytes())
        )
    }

    fn synthetic_dvc3a2() -> String {
        let patch = r#"<PATCH NBFIXTURE="2"><FIXTURES><SSLLIBRARY SSLFIXUID="profile-move" SSLNAME="Test/Move.ssl2"><SSLPROPERTIES SSLBEAMOPENING="20"/><SSLMODES SSLNBMODE="1"><SSLMODE SSLMODEINDEX="0" SSLNBCHANNEL="3"><SSLCHANNEL SSLCHANNELTYPE="1" SSLCHANNELNAME="Pan" SSLCHANNELMSB="0" SSLCHANNELLSB="0"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Pan" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL><SSLCHANNEL SSLCHANNELTYPE="2" SSLCHANNELNAME="Tilt" SSLCHANNELMSB="0" SSLCHANNELLSB="0"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Tilt" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL><SSLCHANNEL SSLCHANNELTYPE="7" SSLCHANNELNAME="Dimmer" SSLCHANNELMSB="0" SSLCHANNELLSB="0"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Dimmer" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL></SSLMODE></SSLMODES></SSLLIBRARY><FIXTURE DASUID="fixture-1" NAME="Mover 1" ADDRESS="1" UNIVERS="1" POSX="0" POSY="0" ANGLE="0"/><FIXTURE DASUID="fixture-2" NAME="Mover 2" ADDRESS="4" UNIVERS="1" POSX="1" POSY="0" ANGLE="0"/></FIXTURES></PATCH>"#;
        let source = format!(
            r##"<DLMFILE TYPE="Daslight" VERSION="5" DASBUILD="test-dvc3a2" VERSIONFILE="2"><PATCHS DATA="{}"/><FIXTUREGROUPS/><SCENES><BANK DASUID="bank-1" NAME="DVC-3a2" COLOR="#ff112233"><SCENE DASUID="scene-ramp" NAME="Inverse Ramp" COLOR="#ff112233" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><FIXTUREDATAS NB="0"/><RACKS><RACK TYPE="8"><EFFECT TYPE="5" ID="3" DURATION="5000"><PARAMS NB="5"><PARAM TYPE="0" ID="1" VAL="2"/><PARAM TYPE="1" ID="2" VAL="1.562"/><PARAM TYPE="1" ID="3" VAL="0.495"/><PARAM TYPE="1" ID="4" VAL="-0.848"/><PARAM TYPE="1" ID="5" VAL="0"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE><SCENE DASUID="scene-polygon" NAME="Move Polygon" COLOR="#ff445566" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="0.5" PLAY_TRIGGER="0" PLAY_DIVISION="1"><FIXTUREDATAS NB="0"/><RACKS><RACK TYPE="4"><EFFECT TYPE="4" ID="224" DURATION="2000"><PARAMS NB="3"><PARAM TYPE="5" ID="1"><POINTS NB="4"><POINT X="0.25" Y="0.5"/><POINT X="0.5" Y="0.75"/><POINT X="0.75" Y="0.5"/><POINT X="0.5" Y="0.25"/></POINTS></PARAM><PARAM TYPE="1" ID="2" VAL="0.02"/><PARAM TYPE="2" ID="3" VAL="1"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE></BANK></SCENES><SHORTCUTS/><TOUCH/><DEVICES/></DLMFILE>"##,
            qcompress(patch.as_bytes())
        );
        source.replace(
            r##"<SCENE DASUID="scene-polygon" NAME="Move Polygon" COLOR="#ff445566" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="0.5" PLAY_TRIGGER="0" PLAY_DIVISION="1">"##,
            r##"<SCENE DASUID="scene-polygon" NAME="Move Polygon" COLOR="#ff445566" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="0.5" PLAY_TRIGGER="0" PLAY_DIVISION="1" ATTRIBUTEVALUE_MODE="0">"##,
        )
    }

    fn synthetic_dvc3b() -> String {
        let patch = r#"<PATCH NBFIXTURE="2"><FIXTURES><SSLLIBRARY SSLFIXUID="profile-color" SSLNAME="Test/Two Segment RGBA.ssl2"><SSLPROPERTIES SSLBEAMOPENING="20"/><SSLMODES SSLNBMODE="1"><SSLMODE SSLMODEINDEX="0" SSLNBCHANNEL="8"><SSLCHANNEL SSLCHANNELTYPE="25" SSLCHANNELNAME="Red 1"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Red" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL><SSLCHANNEL SSLCHANNELTYPE="26" SSLCHANNELNAME="Green 1"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Green" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL><SSLCHANNEL SSLCHANNELTYPE="27" SSLCHANNELNAME="Blue 1"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Blue" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL><SSLCHANNEL SSLCHANNELTYPE="45" SSLCHANNELNAME="Amber 1"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Amber" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL><SSLCHANNEL SSLCHANNELTYPE="25" SSLCHANNELNAME="Red 2"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Red" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL><SSLCHANNEL SSLCHANNELTYPE="26" SSLCHANNELNAME="Green 2"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Green" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL><SSLCHANNEL SSLCHANNELTYPE="27" SSLCHANNELNAME="Blue 2"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Blue" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL><SSLCHANNEL SSLCHANNELTYPE="45" SSLCHANNELNAME="Amber 2"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Amber" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL></SSLMODE></SSLMODES></SSLLIBRARY><FIXTURE DASUID="fixture-1" NAME="Bar 1" ADDRESS="1" UNIVERS="1" POSX="0" POSY="0" ANGLE="0"><BEAM INDEX="0" POSX="0" POSY="0"/><BEAM INDEX="1" POSX="30" POSY="0"/></FIXTURE><FIXTURE DASUID="fixture-2" NAME="Bar 2" ADDRESS="9" UNIVERS="1" POSX="100" POSY="50" ANGLE="0"><BEAM INDEX="0" POSX="100" POSY="50"/><BEAM INDEX="1" POSX="130" POSY="50"/></FIXTURE></FIXTURES></PATCH>"#;
        let palette = r#"<PARAM TYPE="4" ID="1"><COLORS NB="3"><COLOR VAL="0/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM>"#;
        let beams = r#"<BEAMS NB="4"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-1" BEAMID="1" IDSELECTION="2"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="3"/><BEAM FIXTURE="fixture-2" BEAMID="1" IDSELECTION="4"/></BEAMS>"#;
        let scene = |uid: &str,
                     name: &str,
                     rack_type: u16,
                     effect_type: u16,
                     generator_id: u16,
                     numeric_params: &str,
                     param_count: usize| {
            let mapping = match generator_id {
                521 => {
                    r#"<MAPPING NAME="Rectangle" DASUID="mapping-521" TYPE="0" X="-10" Y="-10" SX="160" SY="70" ANGLE="17.5" LOCKED="0"/>"#
                }
                530 => {
                    r#"<MAPPING NAME="Rectangle" DASUID="mapping-530" TYPE="0" X="-10" Y="-10" SX="160" SY="70" ANGLE="17.5" LOCKED="0"/>"#
                }
                _ => "",
            };
            let duration = if generator_id == 127 { 1025 } else { 1000 };
            format!(
                r##"<SCENE DASUID="{uid}" NAME="{name}" COLOR="#ff112233" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="8"><FIXTUREDATAS NB="0"/><RACKS><RACK TYPE="{rack_type}"><EFFECT TYPE="{effect_type}" ID="{generator_id}" DURATION="{duration}"><PARAMS NB="{param_count}">{palette}{numeric_params}</PARAMS></EFFECT>{mapping}{beams}</RACK></RACKS></SCENE>"##
            )
        };
        let scenes = [
            scene(
                "scene-127",
                "Knight",
                2,
                2,
                127,
                r#"<PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="1"/><PARAM TYPE="0" ID="10" VAL="2"/><PARAM TYPE="2" ID="11" VAL="1"/><PARAM TYPE="2" ID="12" VAL="1"/><PARAM TYPE="2" ID="13" VAL="0"/><PARAM TYPE="0" ID="14" VAL="50"/>"#,
                8,
            ),
            scene(
                "scene-121",
                "Burst",
                2,
                2,
                121,
                r#"<PARAM TYPE="2" ID="2" VAL="0"/><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="10" VAL="50"/><PARAM TYPE="1" ID="11" VAL="1"/>"#,
                5,
            ),
            scene(
                "scene-131",
                "Random fill",
                2,
                2,
                131,
                r#"<PARAM TYPE="2" ID="2" VAL="0"/><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="10" VAL="1"/>"#,
                4,
            ),
            scene(
                "scene-133",
                "Sparkle",
                2,
                2,
                133,
                r#"<PARAM TYPE="2" ID="2" VAL="0"/><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="10" VAL="2"/><PARAM TYPE="1" ID="11" VAL="0.25"/><PARAM TYPE="0" ID="12" VAL="1"/>"#,
                6,
            ),
            scene(
                "scene-521",
                "Rainbow",
                6,
                8,
                521,
                r#"<PARAM TYPE="6" ID="3" VAL="1"/><PARAM TYPE="0" ID="4" VAL="171"/><PARAM TYPE="1" ID="10" VAL="0.25"/><PARAM TYPE="0" ID="11" VAL="0"/><PARAM TYPE="1" ID="12" VAL="1"/>"#,
                6,
            ),
            scene(
                "scene-530",
                "Perlin",
                6,
                8,
                530,
                r#"<PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="4" VAL="0"/><PARAM TYPE="0" ID="10" VAL="5"/><PARAM TYPE="0" ID="11" VAL="20"/><PARAM TYPE="0" ID="12" VAL="1"/><PARAM TYPE="0" ID="13" VAL="1"/><PARAM TYPE="0" ID="14" VAL="100"/>"#,
                8,
            ),
        ]
        .join("");
        format!(
            r##"<DLMFILE TYPE="Daslight" VERSION="5" DASBUILD="test-dvc3b" VERSIONFILE="2"><PATCHS DATA="{}"/><FIXTUREGROUPS/><SCENES><BANK DASUID="bank-1" NAME="DVC-3b" COLOR="#ff112233">{scenes}</BANK></SCENES><SHORTCUTS/><TOUCH/><DEVICES/></DLMFILE>"##,
            qcompress(patch.as_bytes())
        )
    }

    fn effect_test_fixture_refs() -> HashMap<String, FixtureImportRef> {
        HashMap::from([
            (
                "fixture-1".to_string(),
                FixtureImportRef {
                    fixture_id: 1,
                    fixture_index: 0,
                    profile_index: 0,
                    supports_dimmer: true,
                    color_beam_count: 1,
                    patch_beam_positions: HashMap::from([(
                        0,
                        DvcPatchCanvasPoint { x: 10, y: 20 },
                    )]),
                },
            ),
            (
                "fixture-2".to_string(),
                FixtureImportRef {
                    fixture_id: 2,
                    fixture_index: 1,
                    profile_index: 0,
                    supports_dimmer: true,
                    color_beam_count: 1,
                    patch_beam_positions: HashMap::from([(
                        0,
                        DvcPatchCanvasPoint { x: 110, y: 20 },
                    )]),
                },
            ),
        ])
    }

    fn effect_test_profiles() -> Vec<ParsedProfile> {
        let document = Document::parse(
            r#"<SSLLIBRARY SSLFIXUID="profile-1" SSLNAME="Test/Dimmer.ssl2"><SSLPROPERTIES SSLBEAMOPENING="20"/><SSLMODES SSLNBMODE="1"><SSLMODE SSLMODEINDEX="0" SSLNBCHANNEL="1"><SSLCHANNEL SSLCHANNELTYPE="7" SSLCHANNELNAME="Dimmer" SSLCHANNELMSB="0" SSLCHANNELLSB="0"><SSLPRESETS><SSLPRESET SSLPRESETTYPE="4" SSLPRESETNAME="Dimmer" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL></SSLMODE></SSLMODES></SSLLIBRARY>"#,
        )
        .unwrap();
        let mut report = DvcImportReport::new("effect-test-profile.ssl2", document.root_element());
        vec![parse_profile(document.root_element(), 0, &mut report).unwrap()]
    }

    #[test]
    fn dvc_concrete_channel_preset_resolves_and_fails_closed_for_missing_profile_or_channel() {
        let parse = |fixture: &str, channel: usize| {
            let source = format!(
                r#"<RACK><PRESETS><PRESET SSLFIXTURE="{fixture}" SSLCHANNEL="{channel}" SSLPRESET="0" MIN="0" MAX="1"/><PRESET SSLFIXTURE="" SSLCHANNEL="-1" SSLPRESET="4" MIN="0" MAX="1"/></PRESETS></RACK>"#
            );
            let document = Document::parse(&source).unwrap();
            dvc_rack_feature_spec(document.root_element(), &effect_test_profiles())
        };

        let resolved = parse("profile-1", 0).unwrap();
        assert_eq!(resolved.preset_type, 4);
        assert_eq!((resolved.low, resolved.high), (0, u16::MAX));
        assert!(resolved
            .source
            .contains("profile PRESET profile-1:0:0 -> type 4"));
        assert!(resolved.source.contains("generic PRESET type 4"));
        assert_eq!(
            parse("absent-profile", 0).unwrap_err(),
            "profile PRESET references absent profile 'absent-profile'"
        );
        assert_eq!(
            parse("profile-1", 33).unwrap_err(),
            "profile PRESET references absent channel 33 on profile 'profile-1'"
        );
    }

    fn value_fx_test_source(
        generator_id: u16,
        transform: u8,
        declared_params: usize,
        class_params: &str,
        targeted: bool,
    ) -> String {
        let beams = if targeted {
            r#"<BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS>"#
        } else {
            r#"<BEAMS NB="0"/>"#
        };
        format!(
            r#"<DLMFILE DASBUILD="25.0905.165.111" VERSIONFILE="2"><SCENE NAME="VALUE catalog" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="8"><RACKS><RACK TYPE="7"><EFFECT TYPE="7" ID="{generator_id}" DURATION="3000"><PARAMS NB="{declared_params}"><PARAM TYPE="4" ID="1"><COLORS NB="2"><COLOR VAL="1/1/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM><PARAM TYPE="6" ID="3" VAL="{transform}"/>{class_params}</PARAMS></EFFECT><PRESETS><PRESET SSLFIXTURE="" SSLCHANNEL="-1" SSLPRESET="4" MIN="0" MAX="1"><BEAMS/></PRESET></PRESETS>{beams}</RACK></RACKS></SCENE></DLMFILE>"#
        )
    }

    fn convert_value_fx_test_source(
        source: &str,
        generator_id: u16,
    ) -> Result<ConvertedDvcEffect, String> {
        let document = Document::parse(source).unwrap();
        let scene = document
            .descendants()
            .find(|node| node.has_tag_name("SCENE"))
            .unwrap();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        convert_dvc_effect(
            scene,
            "VALUE catalog",
            rack,
            effect,
            7,
            7,
            generator_id,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
    }

    fn segmented_red_effect_test_profiles() -> Vec<ParsedProfile> {
        let document = Document::parse(
            r#"<SSLLIBRARY SSLFIXUID="profile-red" SSLNAME="Test/Two Segment Red.ssl2"><SSLPROPERTIES SSLBEAMOPENING="20"/><SSLMODES SSLNBMODE="1"><SSLMODE SSLMODEINDEX="0" SSLNBCHANNEL="2"><SSLCHANNEL SSLCHANNELTYPE="25" SSLCHANNELNAME="Red1"><SSLPRESETS><SSLPRESET SSLPRESETTYPE="65" SSLPRESETNAME="Red1" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL><SSLCHANNEL SSLCHANNELTYPE="25" SSLCHANNELNAME="Red2"><SSLPRESETS><SSLPRESET SSLPRESETTYPE="65" SSLPRESETNAME="Red2" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL></SSLMODE></SSLMODES></SSLLIBRARY>"#,
        )
        .unwrap();
        let mut report =
            DvcImportReport::new("segmented-red-test-profile.ssl2", document.root_element());
        vec![parse_profile(document.root_element(), 0, &mut report).unwrap()]
    }

    fn segmented_red_effect_test_fixture_refs() -> HashMap<String, FixtureImportRef> {
        HashMap::from([(
            "fixture-red".to_string(),
            FixtureImportRef {
                fixture_id: 1,
                fixture_index: 0,
                profile_index: 0,
                supports_dimmer: false,
                color_beam_count: 2,
                patch_beam_positions: HashMap::from([
                    (0, DvcPatchCanvasPoint { x: 10, y: 20 }),
                    (1, DvcPatchCanvasPoint { x: 40, y: 20 }),
                ]),
            },
        )])
    }

    #[test]
    fn dvc_qcompress_decoder_roundtrips_and_checks_length() {
        let source = b"<PATCH NBFIXTURE=\"0\"/>";
        let encoded = qcompress(source);
        assert_eq!(decode_qcompress_base64(&encoded).unwrap(), source);

        let mut wrapped = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        wrapped[3] = wrapped[3].saturating_add(1);
        let invalid = base64::engine::general_purpose::STANDARD.encode(wrapped);
        assert!(decode_qcompress_base64(&invalid)
            .unwrap_err()
            .contains("expected"));
    }

    #[test]
    fn dvc_sync_flush_inflate_accepts_unfinished_zlib_stream() {
        let source = b"fixture-data-without-z-finish";
        let compressed = sync_flush(source);
        assert!(compressed.ends_with(&[0x00, 0x00, 0xff, 0xff]));
        assert_eq!(
            inflate_sync_flush_tolerant(&compressed, 1024).unwrap(),
            source
        );
    }

    #[test]
    fn dvc_argb_color_strips_alpha() {
        assert_eq!(dvc_argb_to_rgb("#ff12ABef").as_deref(), Some("#12abef"));
        assert_eq!(dvc_argb_to_rgb("#123456"), None);
    }

    #[test]
    fn dvc_channel_type_mapping_preserves_known_and_unknown_semantics() {
        assert_eq!(dvc_channel_attribute(1, "X"), ("Pan".to_string(), false));
        assert_eq!(
            dvc_channel_attribute(46, "UV"),
            ("Generic: UV".to_string(), false)
        );
        assert_eq!(
            dvc_channel_attribute(999, "Mystery"),
            ("Generic: Mystery".to_string(), true)
        );
    }

    #[test]
    fn dvc_synthetic_project_imports_patch_group_cues_and_super_scene() {
        let source = synthetic_dvc().replacen(
            r#"SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1""#,
            r#"SSLPRESETDMXDEFAULT="255" SSLPRESETDEFAULTPRESET="1""#,
            1,
        );
        let outcome = import_bytes(source.as_bytes(), "synthetic.dvc").unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        assert_eq!(outcome.project.snapshot.fixtures.len(), 1);
        assert_eq!(outcome.project.custom_profiles.len(), 1);
        assert_eq!(
            outcome.project.custom_profiles[0].dmx_modes[0].controls[0].default_value,
            0
        );
        assert_eq!(outcome.project.snapshot.fixtures[0].universe, 0);
        assert_eq!(outcome.project.snapshot.fixtures[0].address, 1);
        assert_eq!(outcome.project.snapshot.group_colors["Bank 1"], "#112233");
        assert_eq!(outcome.project.snapshot.cues.len(), 2);
        assert_eq!(outcome.project.snapshot.cue_lists.len(), 1);
        assert_eq!(outcome.project.snapshot.cue_lists[0].id, 1);
        assert_eq!(outcome.project.snapshot.cue_lists[0].label, "Bank 1");
        assert!(outcome
            .project
            .snapshot
            .cues
            .iter()
            .all(|cue| cue.cue_list_id == 1));
        assert!(outcome.project.snapshot.cues[0].color.is_none());
        assert!(outcome.project.snapshot.cues[1].color.is_none());
        assert_eq!(
            outcome.project.snapshot.cues[0].targets[0].values[0].value,
            65_535
        );
        let child = outcome.project.snapshot.cues[1]
            .child_timeline
            .as_ref()
            .unwrap();
        assert_eq!(child.layers.len(), 2);
        assert!(child.layers.iter().all(|layer| layer.expanded));
        assert_eq!(child.audio_clips.len(), 1);
        assert_eq!(child.events.len(), 1);
        assert!(!child.events[0].conform_to_tempo);
        assert!(child.events[0].loop_fill);
        assert_eq!(child.events[0].rate, Some(1.0));
        assert_eq!(child.events[0].source_offset_ms, 0);
        assert_eq!(outcome.project.snapshot.clock.bpm, 96.0);
        assert!(!outcome
            .report
            .approximate
            .details
            .iter()
            .any(|detail| { detail.message.contains("CONFORM_TO_TEMPO") }));
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item == "Scene block: Static"
                && detail.message.contains("parent_bpm_driven=false")
                && detail.message.contains("grid_bpm=96.000")
        }));
        assert_eq!(outcome.report.summary.values_decoded, 1);
        assert_eq!(outcome.report.summary.values_skipped, 0);
        assert_eq!(outcome.report.summary.beam_records, 1);
        assert_eq!(outcome.report.summary.beam_feature_checks, 1);
        assert_eq!(outcome.report.summary.beam_feature_mismatches, 0);
        let touch = &outcome.project.snapshot.touch_surface;
        assert_eq!(touch.pages.len(), 1);
        assert_eq!(touch.pages[0].label, "Page 1");
        assert_eq!(touch.pages[0].controls.len(), 3);
        assert!(matches!(
            touch.pages[0].controls[0].binding.as_ref(),
            Some(TouchControlBinding::GroupSelect { group_id }) if group_id == "All"
        ));
        assert!(matches!(
            touch.pages[0].controls[1].binding.as_ref(),
            Some(TouchControlBinding::Cue { cue_id: 1 })
        ));
        assert!(matches!(
            touch.pages[0].controls[2].binding.as_ref(),
            Some(TouchControlBinding::TapTempo)
        ));

        let folded_source = source.replacen("DASTLFOLDED=\"0\"", "DASTLFOLDED=\"1\"", 1);
        let folded = import_bytes(folded_source.as_bytes(), "synthetic-folded.dvc").unwrap();
        let folded_layers = &folded.project.snapshot.cues[1]
            .child_timeline
            .as_ref()
            .unwrap()
            .layers;
        assert!(!folded_layers[0].expanded);
        assert!(folded_layers[1].expanded);
        assert!(!folded
            .report
            .skipped
            .details
            .iter()
            .any(|detail| { detail.message.contains("folded lane state") }));

        let layout =
            import_bytes(synthetic_layout_dvc().as_bytes(), "synthetic-layout.dvc").unwrap();
        let positions = layout
            .project
            .snapshot
            .fixtures
            .iter()
            .map(|fixture| (fixture.position.x, fixture.position.z))
            .collect::<Vec<_>>();
        assert_eq!(positions, vec![(-25.0, 0.0), (-5.0, -10.0), (25.0, 10.0)]);
        assert_eq!(
            layout
                .project
                .snapshot
                .fixtures
                .iter()
                .map(|fixture| fixture.rotation.yaw)
                .collect::<Vec<_>>(),
            vec![15.0, 75.0, 195.0]
        );
        assert_eq!(
            layout.project.snapshot.stage_map,
            StageMapConfig {
                locked: true,
                min_x: -30.0,
                max_x: 30.0,
                min_z: -30.0,
                max_z: 30.0,
            }
        );
    }

    #[test]
    fn dvc_bpm_driven_super_scene_preserves_dynamic_conform_semantics() {
        let source = synthetic_dvc().replacen(
            r##"NAME="Super" COLOR="#ff445566" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="0""##,
            r##"NAME="Super" COLOR="#ff445566" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="2""##,
            1,
        );
        let outcome = import_bytes(source.as_bytes(), "synthetic-bpm-super.dvc").unwrap();
        crate::validate_project_file(&outcome.project).unwrap();

        assert_eq!(outcome.project.snapshot.clock.bpm, 96.0);
        let child = outcome.project.snapshot.cues[1]
            .child_timeline
            .as_ref()
            .unwrap();
        assert!(child.tempo_driven);
        assert!(child.events[0].conform_to_tempo);
        assert_eq!(child.events[0].rate, Some(1.0));
        assert_eq!(outcome.project.snapshot.cues[1].authored_beats, Some(1.0));
        assert!(!outcome.report.approximate.details.iter().any(|detail| {
            detail.item == "Scene block: Static" && detail.message.contains("CONFORM_TO_TEMPO")
        }));
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item == "Scene block: Static"
                && detail.message.contains("parent_bpm_driven=true")
        }));
    }

    #[test]
    fn dvc_bpm_driven_super_scene_with_invalid_division_falls_back_consistently() {
        let source = synthetic_dvc().replacen(
            r##"NAME="Super" COLOR="#ff445566" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1""##,
            r##"NAME="Super" COLOR="#ff445566" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="2" PLAY_DIVISION="0""##,
            1,
        );
        let outcome = import_bytes(source.as_bytes(), "synthetic-invalid-bpm-super.dvc").unwrap();
        crate::validate_project_file(&outcome.project).unwrap();

        let owner = &outcome.project.snapshot.cues[1];
        let child = owner.child_timeline.as_ref().unwrap();
        assert!(!child.tempo_driven);
        assert!(!child.events[0].conform_to_tempo);
        assert_eq!(owner.authored_beats, None);
        assert!(outcome.report.approximate.details.iter().any(|detail| {
            detail.item == "Super Scene: Super"
                && detail
                    .message
                    .contains("no supported positive PLAY_DIVISION")
        }));
    }

    #[test]
    fn dvc_super_scene_loop_off_preserves_final_frame_hold_semantics() {
        let source = synthetic_dvc().replacen("ALLOWLOOP=\"1\"", "ALLOWLOOP=\"0\"", 1);
        let outcome = import_bytes(source.as_bytes(), "synthetic-loop-off-super.dvc").unwrap();
        crate::validate_project_file(&outcome.project).unwrap();

        let child = outcome.project.snapshot.cues[1]
            .child_timeline
            .as_ref()
            .unwrap();
        assert!(!child.events[0].loop_fill);
        assert!(!outcome.report.approximate.details.iter().any(|detail| {
            detail.item == "Scene block: Static" && detail.message.contains("final source frame")
        }));
    }

    #[test]
    fn dvc_midi_shortcuts_import_scene_tap_and_flash_without_guessing_unknown_actions() {
        let outcome = import_bytes(
            synthetic_midi_shortcuts_dvc().as_bytes(),
            "synthetic-midi-shortcuts.dvc",
        )
        .unwrap();
        assert_eq!(outcome.report.midi_mappings.len(), 8);
        assert_eq!(
            outcome.report.midi_mappings[0],
            MidiControlMapping {
                channel: Some(2),
                message: MidiControlMessage::NoteOn,
                number: 60,
                action: MidiControlAction::TriggerCue,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: Some(1),
                layer_id: None,
                output_id: None,
                video_param: None,
                cue_point_index: None,
                duration_ms: None,
                feedback: Some(MidiControlFeedback {
                    off: Some(MidiFeedbackMessage {
                        message: MidiControlMessage::NoteOn,
                        channel: 2,
                        number: 60,
                        value: 5,
                    }),
                    on: Some(MidiFeedbackMessage {
                        message: MidiControlMessage::NoteOn,
                        channel: 2,
                        number: 60,
                        value: 1,
                    }),
                    unknown: None,
                }),
                low: 0.0,
                high: 1.0,
            }
        );
        assert!(matches!(
            outcome.report.midi_mappings[1],
            MidiControlMapping {
                channel: Some(2),
                message: MidiControlMessage::NoteOn,
                number: 61,
                action: MidiControlAction::FlashCue,
                cue_id: Some(2),
                ..
            }
        ));
        assert!(matches!(
            outcome.report.midi_mappings[2],
            MidiControlMapping {
                channel: Some(3),
                message: MidiControlMessage::ControlChange,
                number: 20,
                action: MidiControlAction::TapBpm,
                cue_id: None,
                ..
            }
        ));
        assert!(matches!(
            outcome.report.midi_mappings[3],
            MidiControlMapping {
                channel: Some(2),
                message: MidiControlMessage::NoteOn,
                number: 62,
                action: MidiControlAction::FlashCueDirection,
                cue_id: Some(1),
                ref attribute,
                ..
            } if attribute.as_deref() == Some("Forward")
        ));
        assert!(matches!(
            outcome.report.midi_mappings[4],
            MidiControlMapping {
                channel: Some(2),
                message: MidiControlMessage::NoteOn,
                number: 63,
                action: MidiControlAction::TriggerCueListNext,
                cue_id: Some(2),
                ..
            }
        ));
        assert!(matches!(
            outcome.report.midi_mappings[5],
            MidiControlMapping {
                number: 64,
                action: MidiControlAction::FlashCueDirection,
                cue_id: Some(2),
                ref attribute,
                ..
            } if attribute.as_deref() == Some("Bounce")
        ));
        assert!(matches!(
            outcome.report.midi_mappings[6],
            MidiControlMapping {
                number: 65,
                action: MidiControlAction::TriggerCueDirection,
                cue_id: Some(1),
                ref attribute,
                ..
            } if attribute.as_deref() == Some("Reverse")
        ));
        assert!(matches!(
            outcome.report.midi_mappings[7],
            MidiControlMapping {
                number: 21,
                action: MidiControlAction::SelectedFeatureFader,
                cue_point_index: Some(1),
                low: 0.0,
                high: 65_535.0,
                ..
            }
        ));
        assert!(outcome.report.unsupported.details.iter().any(|detail| {
            detail.item == "Shortcut 9"
                && detail
                    .message
                    .contains("MIDI action type 999 (target index 0)")
        }));
        assert!(!outcome.report.unsupported.details.iter().any(|detail| {
            detail.message.contains("shortcut type 1")
                || detail.message.contains("not a Touch mapping")
        }));
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item == "MIDI mappings"
                && detail.message == "8 verified Daslight MIDI shortcut(s)"
        }));
        assert!(outcome.report.approximate.details.iter().any(|detail| {
            detail.item == "MIDI input device affinity" && detail.message.contains("Setup > I/O")
        }));
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item == "MIDI feedback states"
                && detail.message.contains("velocity colours were restored")
        }));
        assert!(outcome.report.approximate.details.iter().any(|detail| {
            detail.item == "MIDI feedback output device affinity"
                && detail.message.contains("select the intended MIDI output")
        }));
    }

    #[test]
    fn dvc_dmx_shortcuts_import_verified_feature_mapping_and_skip_unproven_variants() {
        let outcome = import_bytes(
            synthetic_dmx_shortcuts_dvc().as_bytes(),
            "synthetic-dmx-shortcuts.dvc",
        )
        .unwrap();
        assert_eq!(
            outcome.report.dmx_mappings,
            vec![DmxControlMapping {
                universe: 1,
                channel: 25,
                action: DmxControlAction::FixtureAttribute,
                fixture_id: Some(1),
                attribute: Some("Dimmer".to_string()),
                group_id: None,
                cue_id: None,
                layer_id: None,
                output_id: None,
                video_param: None,
                cue_point_index: None,
                duration_ms: None,
                low: 16_383.75,
                high: 49_151.25,
            }]
        );
        assert!(outcome.report.skipped.details.iter().any(|detail| {
            detail.item == "Shortcut 2" && detail.message.contains("unverified scalar selector 4")
        }));
        assert!(outcome.report.unsupported.details.iter().any(|detail| {
            detail.item == "Shortcut 3" && detail.message.contains("DMX action type 211")
        }));
        assert!(outcome.report.skipped.details.iter().any(|detail| {
            detail.item == "Shortcut 4"
                && detail
                    .message
                    .contains("DMX mapping setting SMODE=2 is not verified")
        }));
        assert!(outcome.report.skipped.details.iter().any(|detail| {
            detail.item == "Shortcut 5"
                && detail
                    .message
                    .contains("DMX mapping setting INC=0.01 is not verified")
        }));
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item == "DMX control mappings"
                && detail.message
                    == "1 verified Daslight DMX Feature mapping target(s), with 1-based source universes normalized to internal universes"
        }));
    }

    #[test]
    fn dvc_scene_block_position_preserves_positive_and_negative_offsets() {
        let positive_source = synthetic_dvc().replacen(
            r#"TYPE="1" NAME="Static" START="0" END="1000" POSITION="0""#,
            r#"TYPE="1" NAME="Static" START="0" END="1000" POSITION="250""#,
            1,
        );
        let positive = import_bytes(positive_source.as_bytes(), "positive-position.dvc").unwrap();
        let positive_event = &positive.project.snapshot.cues[1]
            .child_timeline
            .as_ref()
            .unwrap()
            .events[0];
        assert_eq!(positive_event.source_offset_ms, 250);
        assert!(positive.report.converted.details.iter().any(|detail| {
            detail.item == "Scene block: Static"
                && detail.message.contains("source position +250 ms")
        }));
        assert!(!positive.report.skipped.details.iter().any(|detail| {
            detail.message == "Daslight source POSITION offset has no Scene Block field"
        }));

        let negative_source = synthetic_dvc().replacen(
            r#"TYPE="1" NAME="Static" START="0" END="1000" POSITION="0""#,
            r#"TYPE="1" NAME="Static" START="0" END="1000" POSITION="-250""#,
            1,
        );
        let negative = import_bytes(negative_source.as_bytes(), "negative-position.dvc").unwrap();
        let negative_event = &negative.project.snapshot.cues[1]
            .child_timeline
            .as_ref()
            .unwrap()
            .events[0];
        assert_eq!(negative_event.source_offset_ms, -250);
        assert!(negative.report.converted.details.iter().any(|detail| {
            detail.item == "Scene block: Static"
                && detail.message.contains("source position -250 ms")
        }));
        assert!(!negative
            .report
            .approximate
            .details
            .iter()
            .any(|detail| { detail.message == "Negative source offset was clamped to zero" }));
    }

    #[test]
    fn dvc_half_float_decoder_maps_sentinels_and_fractions() {
        assert_eq!(half_bits_to_f32(0xBC00), -1.0);
        assert_eq!(half_bits_to_f32(0x3C00), 1.0);
        assert_eq!(half_bits_to_f32(0x0000), 0.0);
        assert!((half_bits_to_f32(0x3999) - 0.7).abs() < 2.0e-3);
        assert!((half_bits_to_f32(0x3866) - 0.55).abs() < 2.0e-3);
        assert!((half_bits_to_f32(0x38CC) - 0.6).abs() < 2.0e-3);
    }

    #[test]
    fn dvc_beam_feature_mismatch_is_reported_not_fatal() {
        let source = synthetic_dvc_with(128, 0x3C00);
        let outcome = import_bytes(source.as_bytes(), "synthetic.dvc").unwrap();
        assert_eq!(outcome.report.summary.values_decoded, 1);
        assert_eq!(outcome.report.summary.beam_feature_checks, 1);
        assert_eq!(outcome.report.summary.beam_feature_mismatches, 1);
        assert!(outcome
            .report
            .warnings
            .iter()
            .any(|warning| warning.contains("beam feature disagrees")));
        assert_eq!(
            outcome.project.snapshot.cues[0].targets[0].values[0].value,
            128 * 257
        );
    }

    #[test]
    fn dvc_confirmed_fx_convert_to_cue_owned_chaser_and_sinus_params() {
        let outcome = import_bytes(synthetic_fx_dvc().as_bytes(), "synthetic-fx.dvc").unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        assert!(outcome.project.snapshot.effects.is_empty());
        assert_eq!(outcome.report.summary.effects_converted, 3);
        assert_eq!(outcome.report.summary.effects_skipped, 2);

        let chaser = &outcome.project.snapshot.cues[0].effect_targets[0];
        let Some(EffectParamsSnapshot::Chaser(chaser)) = &chaser.params else {
            panic!("CHASER 321 must be stored as cue-owned Chaser params");
        };
        assert_eq!(chaser.step_duration_ms, 1_667);
        assert_eq!(chaser.active_step_count, 2);
        assert_eq!(chaser.direction, ChaserDirection::Forward);
        assert_eq!(chaser.overlap, 1.0);
        assert_eq!(chaser.features[0].attribute, "Dimmer");
        assert_eq!(chaser.steps.len(), 3);

        let sinus = &outcome.project.snapshot.cues[1].effect_targets[0];
        let Some(EffectParamsSnapshot::Lfo(sinus)) = &sinus.params else {
            panic!("CURVE 7 must be stored as cue-owned LFO params");
        };
        assert_eq!(sinus.shape, LfoShape::Sine);
        assert_eq!(sinus.period_ms, 5_000);
        assert_eq!(sinus.low, 6_554);
        assert_eq!(sinus.high, 39_321);
        assert_eq!(sinus.phase, 0.25);
        assert_eq!(sinus.attribute, "Dimmer");
        assert_eq!(
            sinus.daslight_curve,
            Some(DaslightCurveSource {
                rate: 10.0,
                size: 0.5,
                offset: 0.1,
                sample_ms: 40,
                rng_seed: None,
            })
        );

        let random = &outcome.project.snapshot.cues[2].effect_targets[0];
        let Some(EffectParamsSnapshot::Chaser(random)) = &random.params else {
            panic!("CHASER 325 must be stored as cue-owned Chaser params");
        };
        assert_eq!(random.direction, ChaserDirection::Random);
        assert_eq!(random.step_duration_ms, 200);
        assert_eq!(random.duty_cycle, 0.5);
        assert_eq!(random.overlap, 0.0);
        assert_eq!(random.random_seed, 1);
        assert_eq!(random.random_cycle_count, 2);
        assert!(!outcome.report.approximate.details.iter().any(|detail| {
            detail.item.contains("Chaser 325") && detail.message.contains("NbCycles")
        }));

        assert!(outcome.project.snapshot.cues[3].effect_targets.is_empty());
        assert!(outcome
            .report
            .skipped
            .details
            .iter()
            .any(|detail| detail.message.contains("ID=133")));
        assert!(outcome
            .report
            .skipped
            .details
            .iter()
            .any(|detail| detail.message.contains("ID=10")));
    }

    #[test]
    fn dvc_random_chaser_rejects_out_of_range_sequence_and_cycle_values() {
        for (source, expected) in [
            (
                synthetic_fx_dvc().replacen(r#"ID="14" VAL="1""#, r#"ID="14" VAL="256""#, 1),
                "Random sequence PARAM 14 must be an integer from 0 to 255",
            ),
            (
                synthetic_fx_dvc().replacen(r#"ID="15" VAL="2""#, r#"ID="15" VAL="256""#, 1),
                "Nb cycles PARAM 15 must be an integer from 1 to 255",
            ),
        ] {
            let outcome = import_bytes(source.as_bytes(), "synthetic-random-range.dvc").unwrap();
            assert_eq!(outcome.report.summary.effects_converted, 2);
            assert_eq!(outcome.report.summary.effects_skipped, 3);
            assert!(outcome
                .report
                .skipped
                .details
                .iter()
                .any(|detail| detail.message.contains(expected)));
        }
    }

    #[test]
    fn dvc3a2_inverse_ramp_and_polygon_convert_and_recall() {
        let outcome = import_bytes(synthetic_dvc3a2().as_bytes(), "synthetic-dvc3a2.dvc").unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        assert_eq!(outcome.report.summary.effects_converted, 2);
        assert_eq!(outcome.report.summary.effects_skipped, 0);

        let ramp = &outcome.project.snapshot.cues[0].effect_targets[0];
        let Some(EffectParamsSnapshot::Lfo(ramp)) = &ramp.params else {
            panic!("CURVE 3 must be stored as cue-owned LFO params");
        };
        assert_eq!(ramp.shape, LfoShape::Saw);
        assert_eq!(ramp.period_ms, 5_000);
        assert_eq!(ramp.low, 65_207);
        assert_eq!(ramp.high, 0);
        assert_eq!(ramp.phase, 0.495);
        assert_eq!(
            ramp.daslight_curve,
            Some(DaslightCurveSource {
                rate: 2.0,
                size: 1.562,
                offset: -0.848,
                sample_ms: 40,
                rng_seed: None,
            })
        );
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item.contains("Inverse Ramp")
                && detail.message.contains("descending_ramp=Saw directed")
        }));
        assert!(!outcome.report.approximate.details.iter().any(|detail| {
            detail.item.contains("Inverse Ramp")
                && detail.message.contains("endpoints were clamped")
        }));

        let polygon = &outcome.project.snapshot.cues[1].effect_targets[0];
        let Some(EffectParamsSnapshot::Move(polygon)) = &polygon.params else {
            panic!("MOVE 224 must be stored as cue-owned Move params");
        };
        assert_eq!(
            polygon.points,
            vec![
                MovePathPoint { x: 0.25, y: 0.5 },
                MovePathPoint { x: 0.5, y: 0.75 },
                MovePathPoint { x: 0.75, y: 0.5 },
                MovePathPoint { x: 0.5, y: 0.25 },
            ]
        );
        assert!(polygon.closed);
        assert_eq!(polygon.interpolation, MoveInterpolation::DaslightPolygon);
        assert_eq!(polygon.direction, MoveDirection::Forward);
        assert_eq!(polygon.coordinate_mode, MoveCoordinateMode::Absolute);
        assert_eq!(polygon.period_ms, 2_000);
        assert!((polygon.fixture_spread - 0.02).abs() < f32::EPSILON);
        assert!(polygon.symmetry);
        assert!(!outcome.report.approximate.details.iter().any(|detail| {
            detail.item.contains("Polygon") && detail.message.contains("Symmetry")
        }));

        let cue_id = outcome.project.snapshot.cues[1].id;
        let mut snapshot_to_load = outcome.project.snapshot;
        snapshot_to_load.output.enabled = false;
        for output in &mut snapshot_to_load.dmx_outputs {
            output.enabled = false;
        }
        let engine = engine::EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine
            .send(engine::EngineCommand::TriggerCue(cue_id))
            .unwrap();
        let mut active_nonzero = false;
        for _ in 0..40 {
            let snapshot = engine.snapshot();
            active_nonzero = snapshot.active_cue_id == Some(cue_id)
                && snapshot.dmx_preview.iter().any(|value| *value != 0);
            if active_nonzero {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(
            active_nonzero,
            "cue recall must activate its owned Move effect"
        );
    }

    #[test]
    fn dvc_color_palette_parser_accepts_the_full_factory_cardinality_domain() {
        let parse = |count: usize| {
            let colors = (0..count)
                .map(|index| {
                    let value = index as f64 / count.max(1) as f64;
                    format!(r#"<COLOR VAL="{value}/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/>"#)
                })
                .collect::<String>();
            let xml = format!(
                r#"<EFFECT><PARAMS NB="1"><PARAM TYPE="4" ID="1"><COLORS NB="{count}">{colors}</COLORS></PARAM></PARAMS></EFFECT>"#
            );
            let document = Document::parse(&xml).unwrap();
            dvc_color_palette_and_params(document.root_element())
        };

        let (_, one) = parse(1).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].position, 0.0);

        let (_, maximum) = parse(255).unwrap();
        assert_eq!(maximum.len(), 255);
        assert_eq!(maximum.first().unwrap().position, 0.0);
        assert_eq!(maximum.last().unwrap().position, 1.0);

        assert!(parse(256).unwrap_err().contains("requires 1..255"));
    }

    #[test]
    fn dvc3b_spatial_routes_include_corrected_random_evaluators() {
        let xml = synthetic_dvc3b().replacen(
            r#"SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1""#,
            r#"SSLPRESETDMXDEFAULT="255" SSLPRESETDEFAULTPRESET="1""#,
            1,
        );
        let outcome = import_bytes(xml.as_bytes(), "synthetic-dvc3b.dvc").unwrap();
        assert_eq!(
            outcome.project.custom_profiles[0].dmx_modes[0].controls[0].default_value,
            0
        );
        assert_eq!(outcome.report.summary.effects_converted, 6);
        assert_eq!(outcome.report.summary.effects_skipped, 0);
        assert!(outcome
            .report
            .converted
            .details
            .iter()
            .filter(|detail| detail.item.starts_with("Effect:"))
            .all(|detail| detail
                .message
                .contains("palette_source=PARAM TYPE=4 ID=1/COLORS/COLOR@VAL")));
        assert_eq!(
            outcome
                .report
                .converted
                .details
                .iter()
                .filter(|detail| detail.message.contains("source_family=Mappings;"))
                .count(),
            2
        );
        assert_eq!(
            outcome
                .report
                .converted
                .details
                .iter()
                .filter(|detail| detail.message.contains("source_family=Colour FX;"))
                .count(),
            4
        );

        let requests = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Color(request)) => Some(request),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(requests.len(), 6);
        assert!(requests.iter().all(|request| request.stops.len() == 3));
        assert!(requests.iter().all(|request| {
            request.spatial_pattern.as_ref().is_some_and(|pattern| {
                pattern.beam_targets.len() == 4
                    && pattern
                        .beam_targets
                        .iter()
                        .map(|target| target.selection_index)
                        .eq(0..4)
            })
        }));
        assert!(requests.iter().any(|request| matches!(
            request
                .spatial_pattern
                .as_ref()
                .map(|pattern| &pattern.recipe),
            Some(ColorEffectSpatialRecipe::KnightRider {
                grayscale: true,
                vertical_symmetry: true,
                size: 50.0,
                ..
            })
        )));
        assert!(requests.iter().any(|request| matches!(
            request
                .spatial_pattern
                .as_ref()
                .map(|pattern| &pattern.recipe),
            Some(ColorEffectSpatialRecipe::Perlin {
                grayscale: false,
                vertical_symmetry: false,
                horizontal_symmetry: false,
                rotation_degrees: 0.0,
                octaves: 4,
                zoom,
                direction_degrees: 0.0,
                speed: 1.0,
                amplitude: 100.0,
            }) if (*zoom - 20.0 / 99.0).abs() <= f32::EPSILON
        )));
        assert!(requests.iter().any(|request| matches!(
            request
                .spatial_pattern
                .as_ref()
                .map(|pattern| &pattern.recipe),
            Some(ColorEffectSpatialRecipe::Burst {
                grayscale: false,
                vertical_symmetry: false,
                color_width: 1250.0,
                gradient: 100.0,
            })
        )));
        let knight = requests
            .iter()
            .find(|request| {
                matches!(
                    request
                        .spatial_pattern
                        .as_ref()
                        .map(|pattern| &pattern.recipe),
                    Some(ColorEffectSpatialRecipe::KnightRider { .. })
                )
            })
            .expect("synthetic 127 must retain the exact Knight Rider request");
        assert_eq!(
            knight.period_ms, 1_025,
            "corrected runtime must preserve the authored 1025 ms duration"
        );
        assert!(requests.iter().any(|request| matches!(
            request
                .spatial_pattern
                .as_ref()
                .map(|pattern| &pattern.recipe),
            Some(ColorEffectSpatialRecipe::Rainbow {
                grayscale: false,
                vertical_symmetry: true,
                rotation_degrees: 171.0,
                color_width: 25.0,
                ..
            })
        )));
        let mapping_pattern = requests
            .iter()
            .find_map(|request| {
                request.spatial_pattern.as_deref().filter(|pattern| {
                    matches!(&pattern.recipe, ColorEffectSpatialRecipe::Rainbow { .. })
                })
            })
            .expect("synthetic 521 must retain its spatial pattern");
        assert_eq!(
            mapping_pattern.placement,
            Some(ColorEffectSpatialPlacement {
                source_coordinate_frame: ColorEffectSpatialCoordinateFrame::DaslightPatchCanvas,
                mapping_shape: ColorEffectSpatialMappingShape::Rectangle,
                x: -10,
                y: -10,
                sx: 160,
                sy: 70,
                mapping_angle_degrees: 17.5,
                sampling_rule:
                    ColorEffectSpatialSamplingRule::RotatedInclusionMaskAxisAlignedRaster,
                vertical_symmetry: false,
                horizontal_symmetry: false,
                raster_rotation_degrees: 0.0,
                target_coordinates: vec![
                    ColorEffectSpatialPlacementTarget {
                        fixture_id: 1,
                        beam_index: 0,
                        patch_x: 0,
                        patch_y: 0,
                    },
                    ColorEffectSpatialPlacementTarget {
                        fixture_id: 1,
                        beam_index: 1,
                        patch_x: 30,
                        patch_y: 0,
                    },
                    ColorEffectSpatialPlacementTarget {
                        fixture_id: 2,
                        beam_index: 0,
                        patch_x: 100,
                        patch_y: 50,
                    },
                    ColorEffectSpatialPlacementTarget {
                        fixture_id: 2,
                        beam_index: 1,
                        patch_x: 130,
                        patch_y: 50,
                    },
                ],
            }),
            "521 must preserve the raw Rectangle and authored per-beam Patch coordinates"
        );
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item.contains("Random fill")
                && detail.message.contains("implementation=SyndocalCorrected")
                && detail.message.contains("stable_source_seed")
                && detail.message.contains("cell_partition=div_ceil")
                && detail
                    .message
                    .contains("end_coverage=partial_tail_included")
                && detail
                    .message
                    .contains("transition=continuous_palette_to_palette")
        }));
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item.contains("Sparkle")
                && detail.message.contains("lifetime_ms=Some(133)")
                && detail.message.contains("stable_source_seed")
                && detail.message.contains("population=retained")
                && detail.message.contains("fade=continuous")
                && detail.message.contains("time_units=milliseconds")
        }));
        assert!(requests.iter().any(|request| matches!(
            request
                .spatial_pattern
                .as_ref()
                .map(|pattern| &pattern.recipe),
            Some(ColorEffectSpatialRecipe::RandomFill {
                point_width: 25.0,
                source_point_height: None,
                ..
            })
        )));
        assert!(requests.iter().any(|request| matches!(
            request
                .spatial_pattern
                .as_ref()
                .map(|pattern| &pattern.recipe),
            Some(ColorEffectSpatialRecipe::Sparkle {
                number: 2,
                lifetime_ms: Some(133),
                source_lifespan: Some(0.25),
                width: 25.0,
                ..
            })
        )));
    }

    #[test]
    fn dvc3b_mappings_perlin_preserves_nonzero_exact_raster_rotation() {
        let xml = synthetic_dvc3b().replacen(
            r#"<PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="4" VAL="0"/><PARAM TYPE="0" ID="10" VAL="5"/>"#,
            r#"<PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="4" VAL="246"/><PARAM TYPE="0" ID="10" VAL="5"/>"#,
            1,
        );
        let outcome = import_bytes(xml.as_bytes(), "synthetic-dvc3b-rotation.dvc").unwrap();
        let perlin = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .find_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Color(request)) => request
                    .spatial_pattern
                    .as_ref()
                    .filter(|pattern| {
                        matches!(pattern.recipe, ColorEffectSpatialRecipe::Perlin { .. })
                    })
                    .map(|pattern| &pattern.recipe),
                _ => None,
            })
            .expect("MAPPINGS Perlin ID530 must convert");
        assert!(matches!(
            perlin,
            ColorEffectSpatialRecipe::Perlin {
                rotation_degrees: 246.0,
                ..
            }
        ));
        assert!(!outcome.report.skipped.details.iter().any(|detail| {
            detail.item.contains("Perlin") && detail.message.contains("rotation")
        }));
    }

    #[test]
    fn dvc3b_generator_without_verified_palette_source_stays_skipped() {
        let xml = synthetic_dvc3b().replacen(
            r#"<PARAM TYPE="4" ID="1"><COLORS NB="3"><COLOR VAL="0/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM>"#,
            r#"<PARAM TYPE="4" ID="1"><NOT_COLORS/></PARAM>"#,
            1,
        );
        let outcome = import_bytes(xml.as_bytes(), "synthetic-dvc3b-missing-palette.dvc").unwrap();
        assert_eq!(outcome.report.summary.effects_converted, 5);
        assert_eq!(outcome.report.summary.effects_skipped, 1);
        assert!(outcome.report.skipped.details.iter().any(|detail| detail
            .message
            .contains("palette PARAM TYPE=4 ID=1 is missing COLORS")));
    }

    #[test]
    fn dvc_rack_targets_preserve_xml_order_and_normalize_repeated_selection_ids() {
        let document = Document::parse(
            r#"<RACK><BEAMS NB="5"><BEAM FIXTURE="fixture-a" BEAMID="7" IDSELECTION="30"/><BEAM FIXTURE="fixture-b" BEAMID="2" IDSELECTION="10"/><BEAM FIXTURE="fixture-a" BEAMID="5" IDSELECTION="30"/><BEAM FIXTURE="fixture-b" BEAMID="9" IDSELECTION="20"/><BEAM FIXTURE="fixture-a" BEAMID="1" IDSELECTION="10"/></BEAMS></RACK>"#,
        )
        .unwrap();
        let fixture_refs = HashMap::from([
            (
                "fixture-a".to_string(),
                FixtureImportRef {
                    fixture_id: 101,
                    fixture_index: 0,
                    profile_index: 0,
                    supports_dimmer: false,
                    color_beam_count: 10,
                    patch_beam_positions: HashMap::new(),
                },
            ),
            (
                "fixture-b".to_string(),
                FixtureImportRef {
                    fixture_id: 202,
                    fixture_index: 1,
                    profile_index: 0,
                    supports_dimmer: false,
                    color_beam_count: 10,
                    patch_beam_positions: HashMap::new(),
                },
            ),
        ]);

        let targets = dvc_rack_targets(document.root_element(), &fixture_refs).unwrap();

        assert_eq!(targets.fixture_ids, vec![101, 202]);
        assert_eq!(
            targets.ordered_steps,
            vec![vec![101], vec![202, 101], vec![202]]
        );
        assert_eq!(
            targets
                .beam_targets
                .iter()
                .map(|target| (
                    target.fixture_id,
                    target.beam_index,
                    target.selection_index,
                ))
                .collect::<Vec<_>>(),
            vec![
                (101, 7, 0),
                (202, 2, 1),
                (101, 5, 0),
                (202, 9, 2),
                (101, 1, 1),
            ],
            "beam target Vec must retain XML fixture/BEAMID order while repeated and out-of-order raw IDSELECTION values normalize by first occurrence"
        );
    }

    #[test]
    fn dvc_move_unexpected_params_and_empty_points_stay_skipped() {
        let empty_document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="4"><EFFECT TYPE="4" ID="224" DURATION="1000"><PARAMS NB="3"><PARAM TYPE="5" ID="1"><POINTS NB="0"/></PARAM><PARAM TYPE="1" ID="2" VAL="0"/><PARAM TYPE="2" ID="3" VAL="0"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = empty_document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let empty_error = convert_dvc_effect(
            scene,
            "Empty Polygon",
            rack,
            effect,
            4,
            4,
            224,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap_err();
        assert!(empty_error.contains("POINTS must contain between 2 and 255 vertices"));

        let unexpected_document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="4"><EFFECT TYPE="4" ID="223" DURATION="1000"><PARAMS NB="4"><PARAM TYPE="5" ID="1"><POINTS NB="2"><POINT X="0" Y="0"/><POINT X="1" Y="1"/></POINTS></PARAM><PARAM TYPE="1" ID="2" VAL="0"/><PARAM TYPE="2" ID="3" VAL="0"/><PARAM TYPE="1" ID="4" VAL="0"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = unexpected_document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let unexpected_error = convert_dvc_effect(
            scene,
            "Unexpected Line",
            rack,
            effect,
            4,
            4,
            223,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap_err();
        assert!(unexpected_error.contains("unexpected ID 4"));
    }

    #[test]
    fn dvc_move_factory_ids_221_through_225_route_to_distinct_exact_evaluators() {
        let cases = [
            (221, "Circle", MoveInterpolation::DaslightCircle, true, 4),
            (222, "Curve", MoveInterpolation::DaslightCurve, false, 4),
            (223, "Line", MoveInterpolation::DaslightLine, false, 2),
            (224, "Polygon", MoveInterpolation::DaslightPolygon, true, 4),
            (225, "Points", MoveInterpolation::DaslightPoints, false, 4),
        ];
        for (generator_id, generator, expected, closed, point_count) in cases {
            let all_points = [
                r#"<POINT X="0.25" Y="0.5"/>"#,
                r#"<POINT X="0.5" Y="0.75"/>"#,
                r#"<POINT X="0.75" Y="0.5"/>"#,
                r#"<POINT X="0.5" Y="0.25"/>"#,
            ];
            let points = all_points[..point_count].join("");
            let source = format!(
                r#"<SCENE ATTRIBUTEVALUE_MODE="0" SPEED="0.5" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="4"><EFFECT TYPE="4" ID="{generator_id}" DURATION="1025"><PARAMS NB="3"><PARAM TYPE="5" ID="1"><POINTS NB="{point_count}">{points}</POINTS></PARAM><PARAM TYPE="1" ID="2" VAL="0.375"/><PARAM TYPE="2" ID="3" VAL="1"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#
            );
            let document = Document::parse(&source).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            let converted = convert_dvc_effect(
                scene,
                "Factory Move",
                rack,
                effect,
                4,
                4,
                generator_id,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
            .unwrap();
            let Some(EffectParamsSnapshot::Move(request)) = converted.target.unwrap().params else {
                panic!("MOVE {generator_id} must produce Move params");
            };
            assert_eq!(request.interpolation, expected, "{generator}");
            assert_eq!(request.closed, closed, "{generator}");
            assert_eq!(request.period_ms, 1_025, "{generator}");
            assert_eq!(request.fixture_spread, 0.375, "{generator}");
            assert!(request.symmetry, "{generator}");
            assert_eq!(request.beam_targets.len(), 2, "{generator}");
            assert!(converted.approximations.is_empty(), "{generator}");
        }
    }

    #[test]
    fn dvc_curve_and_polygon_schema_match_factory_types_and_domains() {
        let curve = r#"<EFFECT><PARAMS NB="5"><PARAM TYPE="0" ID="1" VAL="2"/><PARAM TYPE="1" ID="2" VAL="1"/><PARAM TYPE="1" ID="3" VAL="0.25"/><PARAM TYPE="1" ID="4" VAL="0"/><PARAM TYPE="1" ID="5" VAL="0.5"/></PARAMS></EFFECT>"#;
        let validate_curve = |xml: &str| {
            let document = Document::parse(xml).unwrap();
            dvc_curve_effect_params(document.root_element(), "Curve")
        };
        assert!(validate_curve(curve).is_ok());
        for invalid in [
            curve.replacen(r#"TYPE="0" ID="1""#, r#"TYPE="1" ID="1""#, 1),
            curve.replacen(r#"ID="1" VAL="2""#, r#"ID="1" VAL="2.5""#, 1),
            curve.replacen(r#"ID="2" VAL="1""#, r#"ID="2" VAL="2.01""#, 1),
            curve.replacen(r#"ID="3" VAL="0.25""#, r#"ID="3" VAL="1.01""#, 1),
            curve.replacen(r#"ID="4" VAL="0""#, r#"ID="4" VAL="-1.01""#, 1),
            curve.replacen(r#"ID="5" VAL="0.5""#, r#"ID="5" VAL="1.01""#, 1),
        ] {
            assert!(validate_curve(&invalid).is_err());
        }

        let polygon = |points: String, count: usize| {
            format!(
                r#"<EFFECT><PARAMS NB="3"><PARAM TYPE="5" ID="1"><POINTS NB="{count}">{points}</POINTS></PARAM><PARAM TYPE="1" ID="2" VAL="0"/><PARAM TYPE="2" ID="3" VAL="0"/></PARAMS></EFFECT>"#
            )
        };
        let two_points = polygon(r#"<POINT X="0" Y="0"/><POINT X="1" Y="1"/>"#.to_string(), 2);
        let document = Document::parse(&two_points).unwrap();
        assert_eq!(
            dvc_move_effect_params(document.root_element(), "Polygon")
                .unwrap()
                .0
                .len(),
            2
        );
        let points_256 = (0..256)
            .map(|index| format!(r#"<POINT X="{}" Y="0"/>"#, index as f32 / 255.0))
            .collect::<String>();
        let polygon_256 = polygon(points_256, 256);
        let document = Document::parse(&polygon_256).unwrap();
        assert!(dvc_move_effect_params(document.root_element(), "Polygon")
            .unwrap_err()
            .contains("between 2 and 255"));
    }

    #[test]
    fn dvc_empty_chasers_validate_class_schema_before_noop() {
        let convert = |generator_id: u16, params: &str, count: usize| {
            let xml = format!(
                r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="3"><EFFECT TYPE="6" ID="{generator_id}" DURATION="1000"><PARAMS NB="{count}">{params}</PARAMS></EFFECT><BEAMS NB="0"/></RACK></RACKS></SCENE>"#
            );
            let document = Document::parse(&xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "empty chaser",
                rack,
                effect,
                3,
                6,
                generator_id,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };
        for (generator_id, params, count, expected) in [
            (
                321,
                r#"<PARAM TYPE="2" ID="10" VAL="2"/><PARAM TYPE="2" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="1"/>"#,
                3,
                "must be 0 or 1",
            ),
            (
                322,
                r#"<PARAM TYPE="1" ID="10" VAL="1"/>"#,
                1,
                "expected PARAM 10 TYPE=2",
            ),
            (
                322,
                r#"<PARAM TYPE="2" ID="10" VAL="2"/>"#,
                1,
                "must be 0 or 1",
            ),
            (
                323,
                r#"<PARAM TYPE="2" ID="10" VAL="1"/><PARAM TYPE="1" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="1"/>"#,
                3,
                "expected PARAM 11 TYPE=2",
            ),
            (
                324,
                r#"<PARAM TYPE="2" ID="10" VAL="0"/><PARAM TYPE="2" ID="11" VAL="2"/><PARAM TYPE="0" ID="12" VAL="1"/>"#,
                3,
                "must be 0 or 1",
            ),
            (
                325,
                r#"<PARAM TYPE="2" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="1"/><PARAM TYPE="1" ID="13" VAL="50"/><PARAM TYPE="0" ID="14" VAL="1"/><PARAM TYPE="0" ID="15" VAL="256"/>"#,
                5,
                "integer from 1 to 255",
            ),
        ] {
            let error = convert(generator_id, params, count).unwrap_err();
            assert!(
                error.contains(expected),
                "expected {expected:?}, found {error:?}"
            );
        }
        for (generator_id, params, count) in [
            (
                321,
                r#"<PARAM TYPE="2" ID="10" VAL="1"/><PARAM TYPE="2" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="0"/>"#,
                3,
            ),
            (
                323,
                r#"<PARAM TYPE="2" ID="10" VAL="1"/><PARAM TYPE="2" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="1"/>"#,
                3,
            ),
            (
                324,
                r#"<PARAM TYPE="2" ID="10" VAL="0"/><PARAM TYPE="2" ID="11" VAL="1"/><PARAM TYPE="0" ID="12" VAL="1"/>"#,
                3,
            ),
            (
                325,
                r#"<PARAM TYPE="2" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="0"/><PARAM TYPE="1" ID="13" VAL="50"/><PARAM TYPE="0" ID="14" VAL="1"/><PARAM TYPE="0" ID="15" VAL="1"/>"#,
                5,
            ),
        ] {
            let converted = convert(generator_id, params, count).unwrap();
            assert!(converted.target.is_none());
            assert!(converted.note.contains("domains validated"));
        }
    }

    #[test]
    fn dvc_chaser_321_with_unexpected_binary_param_stays_skipped() {
        let document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="8"><RACKS><RACK TYPE="3"><EFFECT TYPE="6" ID="321" DURATION="1000"><PARAMS NB="3"><PARAM TYPE="2" ID="10" VAL="2"/><PARAM TYPE="2" ID="11" VAL="1"/><PARAM TYPE="0" ID="12" VAL="1"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let error = convert_dvc_effect(
            scene,
            "Invalid 321",
            rack,
            effect,
            3,
            6,
            321,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap_err();
        assert!(error.contains("must be 0 or 1"));
    }

    #[test]
    fn dvc_chaser_generic_preset_type_restores_segment_features_and_range() {
        let document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="8"><RACKS><RACK TYPE="3"><EFFECT TYPE="6" ID="321" DURATION="1000"><PARAMS NB="3"><PARAM TYPE="2" ID="10" VAL="1"/><PARAM TYPE="2" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="1"/></PARAMS></EFFECT><PRESETS><PRESET SSLFIXTURE="" SSLCHANNEL="-1" SSLPRESET="65" MIN="0.25" MAX="0.75"><BEAMS/></PRESET></PRESETS><BEAMS NB="2"><BEAM FIXTURE="fixture-red" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-red" BEAMID="1" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let converted = convert_dvc_effect(
            scene,
            "Segment Red",
            rack,
            effect,
            3,
            6,
            321,
            1,
            &segmented_red_effect_test_profiles(),
            &segmented_red_effect_test_fixture_refs(),
        )
        .unwrap();
        let Some(EffectParamsSnapshot::Chaser(request)) = converted.target.unwrap().params else {
            panic!("generic PRESET type 65 must convert to Chaser params");
        };
        assert_eq!(
            request
                .features
                .iter()
                .map(|feature| (feature.attribute.as_str(), feature.low, feature.high))
                .collect::<Vec<_>>(),
            vec![("ColorRed", 16_384, 49_151), ("ColorRed 2", 16_384, 49_151),]
        );
        assert_eq!(request.steps.len(), 2);
        assert_eq!(
            request.steps[0].beam_targets[0].feature_attribute,
            "ColorRed"
        );
        assert_eq!(
            request.steps[1].beam_targets[0].feature_attribute,
            "ColorRed 2"
        );
        assert!(request.steps.iter().all(|step| step.fixture_ids.is_empty()));
        assert!(converted.note.contains("preset_type=65"));
        assert!(converted.note.contains("preset_range=16384..49151"));
        assert_eq!(converted.approximations.len(), 1);
        assert!(converted.approximations[0].contains("evaluator 0x140376540"));
    }

    #[test]
    fn dvc_sinus_prefers_bpm_sync_and_preserves_phasing_and_segment_targets() {
        let document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="2" PLAY_DIVISION="4"><RACKS><RACK TYPE="8"><EFFECT TYPE="5" ID="7" DURATION="5000"><PARAMS NB="5"><PARAM TYPE="0" ID="1" VAL="10"/><PARAM TYPE="1" ID="2" VAL="1"/><PARAM TYPE="1" ID="3" VAL="0.25"/><PARAM TYPE="1" ID="4" VAL="0"/><PARAM TYPE="1" ID="5" VAL="0.2"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="1" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let converted = convert_dvc_effect(
            scene,
            "Synced Sinus",
            rack,
            effect,
            8,
            5,
            7,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        let Some(EffectParamsSnapshot::Lfo(request)) =
            converted.target.expect("runtime target").params
        else {
            panic!("CURVE 7 must convert to LFO params");
        };
        assert_eq!(request.clock_sync.unwrap().beats, 0.25);
        assert_eq!(request.period_ms, 5_000);
        assert_eq!(request.daslight_curve.as_ref().unwrap().rate, 10.0);
        assert_eq!(request.daslight_curve.as_ref().unwrap().sample_ms, 40);
        assert!((request.fixture_spread - 0.2).abs() < f32::EPSILON);
        assert_eq!(request.beam_targets.len(), 2);
        assert_eq!(request.beam_targets[0].beam_index, 1);
        assert_eq!(request.beam_targets[0].selection_index, 0);
        assert_eq!(request.beam_targets[0].feature_attribute, "Dimmer");
        assert!(!converted
            .approximations
            .iter()
            .any(|note| note == "segment selection approximated to fixture"));
        assert!(converted.warnings.is_empty());
        assert!(converted.note.contains("fixture_spread=0.2"));
        assert!(converted
            .approximations
            .iter()
            .any(|note| note.contains("evaluates Sinus continuously")));
    }

    #[test]
    fn dvc_strobe_conversion_scales_size_and_records_corrected_rate() {
        for (size, expected_high) in [(1.0, 32_768), (2.0, 65_535)] {
            let xml = format!(
                r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="8"><EFFECT TYPE="5" ID="10" DURATION="5000"><PARAMS NB="5"><PARAM TYPE="0" ID="1" VAL="2"/><PARAM TYPE="1" ID="2" VAL="{size}"/><PARAM TYPE="1" ID="3" VAL="0"/><PARAM TYPE="1" ID="4" VAL="0"/><PARAM TYPE="1" ID="5" VAL="0.4"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#
            );
            let document = Document::parse(&xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            let converted = convert_dvc_effect(
                scene,
                "Fl-Strobe",
                rack,
                effect,
                8,
                5,
                10,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
            .unwrap();
            let Some(EffectParamsSnapshot::Lfo(request)) =
                converted.target.expect("runtime target").params
            else {
                panic!("CURVE 10 must convert to cue-owned LFO params");
            };
            assert_eq!(request.shape, LfoShape::Strobe);
            assert_eq!(request.period_ms, 5_000);
            assert_eq!(request.low, 0);
            assert_eq!(request.high, expected_high);
            assert_eq!(request.phase, 0.0);
            assert!((request.fixture_spread - 0.4).abs() < f32::EPSILON);
            assert_eq!(
                request.daslight_curve,
                Some(DaslightCurveSource {
                    rate: 2.0,
                    size: size as f32,
                    offset: 0.0,
                    sample_ms: 40,
                    rng_seed: None,
                })
            );
            assert!(!converted
                .approximations
                .iter()
                .any(|note| note.contains("2% of the period")));
            assert!(converted.note.contains("implementation=SyndocalCorrected"));
            assert!(converted.note.contains("Syndocal_interval_seconds=1/Rate"));
            assert!(converted
                .approximations
                .iter()
                .any(|note| note.contains("common dimensionless pulse duty")));
            assert!(converted.note.contains("fixture_spread=0.4"));
        }
    }

    #[test]
    fn dvc_square_conversion_uses_corrected_equal_bands_and_preserves_beam_order() {
        let document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="8"><EFFECT TYPE="5" ID="9" DURATION="1000"><PARAMS NB="5"><PARAM TYPE="0" ID="1" VAL="3"/><PARAM TYPE="1" ID="2" VAL="0.75"/><PARAM TYPE="1" ID="3" VAL="0.25"/><PARAM TYPE="1" ID="4" VAL="0.1"/><PARAM TYPE="1" ID="5" VAL="0.6"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let converted = convert_dvc_effect(
            scene,
            "Quantized Square",
            rack,
            effect,
            8,
            5,
            9,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        let Some(EffectParamsSnapshot::Lfo(request)) =
            converted.target.expect("runtime target").params
        else {
            panic!("CURVE 9 must convert to cue-owned LFO params");
        };
        assert_eq!(request.shape, LfoShape::Square);
        assert_eq!(
            (request.low, request.high),
            (normalized_dmx(0.1), normalized_dmx(0.85))
        );
        assert_eq!(request.period_ms, 1_000);
        assert_eq!(request.phase, 0.25);
        assert_eq!(request.fixture_spread, 0.6);
        assert_eq!(
            request.daslight_curve,
            Some(DaslightCurveSource {
                rate: 3.0,
                size: 0.75,
                offset: 0.1,
                sample_ms: 40,
                rng_seed: None,
            })
        );
        assert_eq!(
            request
                .beam_targets
                .iter()
                .map(|target| target.fixture_id)
                .collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert!(converted.note.contains("CSquareEffect@0x140370420"));
        assert!(converted.note.contains("implementation=SyndocalCorrected"));
        assert!(converted.note.contains("Syndocal_band=floor"));
        assert_eq!(converted.approximations.len(), 1);
        assert!(converted.approximations[0].contains("equal-width authored bands"));
    }

    #[test]
    fn dvc_pulse_conversion_records_corrected_window_and_preserves_beam_order() {
        let document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="8"><EFFECT TYPE="5" ID="4" DURATION="16000"><PARAMS NB="5"><PARAM TYPE="0" ID="1" VAL="1"/><PARAM TYPE="1" ID="2" VAL="0.5"/><PARAM TYPE="1" ID="3" VAL="0.25"/><PARAM TYPE="1" ID="4" VAL="-0.1"/><PARAM TYPE="1" ID="5" VAL="0.6"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let converted = convert_dvc_effect(
            scene,
            "Exact Pulse",
            rack,
            effect,
            8,
            5,
            4,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        let Some(EffectParamsSnapshot::Lfo(request)) =
            converted.target.expect("runtime target").params
        else {
            panic!("CURVE 4 must convert to cue-owned LFO params");
        };
        assert_eq!(request.shape, LfoShape::Pulse);
        assert_eq!((request.low, request.high), (0, u16::MAX));
        assert_eq!(request.period_ms, 16_000);
        assert_eq!(request.phase, 0.25);
        assert_eq!(request.fixture_spread, 0.6);
        assert_eq!(
            request.daslight_curve,
            Some(DaslightCurveSource {
                rate: 1.0,
                size: 0.5,
                offset: -0.1,
                sample_ms: 40,
                rng_seed: None,
            })
        );
        assert_eq!(
            request
                .beam_targets
                .iter()
                .map(|target| target.fixture_id)
                .collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert!(converted.note.contains("CPulseEffect@0x14036F8D0"));
        assert!(converted.note.contains("Daslight_fixed_window_slope=0.005"));
        assert!(converted.note.contains("implementation=SyndocalCorrected"));
        assert!(converted.note.contains("sample_count=400"));
        assert_eq!(converted.approximations.len(), 1);
        assert!(converted.approximations[0].contains("DURATION no longer changes"));
    }

    #[test]
    fn dvc_inverse_ramp_preserves_normalized_phasing_with_timing_correction() {
        let document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="8"><EFFECT TYPE="5" ID="3" DURATION="5000"><PARAMS NB="5"><PARAM TYPE="0" ID="1" VAL="2"/><PARAM TYPE="1" ID="2" VAL="1"/><PARAM TYPE="1" ID="3" VAL="0"/><PARAM TYPE="1" ID="4" VAL="0"/><PARAM TYPE="1" ID="5" VAL="0.6"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let converted = convert_dvc_effect(
            scene,
            "Inverse",
            rack,
            effect,
            8,
            5,
            3,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        let Some(EffectParamsSnapshot::Lfo(request)) =
            converted.target.expect("runtime target").params
        else {
            panic!("CURVE 3 must convert to cue-owned LFO params");
        };
        assert!((request.fixture_spread - 0.6).abs() < f32::EPSILON);
        assert!(converted
            .approximations
            .iter()
            .all(|note| !note.contains("Phasing")));
        assert!(converted.note.contains("fixture_spread=0.6"));
        assert!(converted
            .approximations
            .iter()
            .any(|note| note.contains("evaluates Inverse Ramp continuously")));
    }

    #[test]
    fn dvc_plasma_conversion_parses_five_color_slash_palette_and_parameters() {
        let document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="2"><EFFECT TYPE="2" ID="129" DURATION="5000"><PARAMS NB="11"><PARAM TYPE="4" ID="1"><COLORS NB="5"><COLOR VAL="0.94902/0.0196078/0.266667/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0.2/0.1/0.3/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0.4/0.3/0.2/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0.6/0.5/0.4/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0.8/0.7/0.6/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM><PARAM TYPE="2" ID="2" VAL="0"/><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="10" VAL="1"/><PARAM TYPE="0" ID="11" VAL="2"/><PARAM TYPE="0" ID="12" VAL="1"/><PARAM TYPE="0" ID="13" VAL="2"/><PARAM TYPE="0" ID="14" VAL="-1"/><PARAM TYPE="0" ID="15" VAL="2"/><PARAM TYPE="0" ID="16" VAL="1"/><PARAM TYPE="0" ID="17" VAL="-1"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let converted = convert_dvc_effect(
            scene,
            "B-WineRed (2)",
            rack,
            effect,
            2,
            2,
            129,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        let Some(EffectParamsSnapshot::Color(request)) =
            converted.target.expect("runtime target").params
        else {
            panic!("COLOR 129 must convert to cue-owned Color params");
        };
        assert_eq!(request.stops.len(), 5);
        assert_eq!(
            request.stops[0].color,
            ColorEffectColor {
                red: normalized_dmx(0.94902),
                green: normalized_dmx(0.0196078),
                blue: normalized_dmx(0.266667),
            }
        );
        assert!(matches!(
            request
                .spatial_pattern
                .as_ref()
                .map(|pattern| &pattern.recipe),
            Some(ColorEffectSpatialRecipe::Plasma {
                grayscale: false,
                vertical_symmetry: false,
                size_x: 1.0,
                param_x: 2.0,
                size_y: 1.0,
                param_y: 2.0,
                speed_x: -1.0,
                param_sx: 2.0,
                speed_y: 1.0,
                param_sy: -1.0,
            })
        ));
        assert!(converted.approximations.is_empty());
    }

    #[test]
    fn dvc_color_rainbow_conversion_maps_width_angle_and_gradient_times_100() {
        let document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="2"><EFFECT TYPE="2" ID="130" DURATION="5000"><PARAMS NB="6"><PARAM TYPE="4" ID="1"><COLORS NB="5"><COLOR VAL="0/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0.2/0.1/0.3/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0.4/0.3/0.2/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0.6/0.5/0.4/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0.8/0.7/0.6/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM><PARAM TYPE="2" ID="2" VAL="0"/><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="1" ID="10" VAL="0.25"/><PARAM TYPE="0" ID="11" VAL="45"/><PARAM TYPE="1" ID="12" VAL="0.75"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let converted = convert_dvc_effect(
            scene,
            "PS-WineRed",
            rack,
            effect,
            2,
            2,
            130,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        let Some(EffectParamsSnapshot::Color(request)) =
            converted.target.expect("runtime target").params
        else {
            panic!("COLOR 130 must convert to cue-owned Color params");
        };
        assert!(matches!(
            request
                .spatial_pattern
                .as_ref()
                .map(|pattern| &pattern.recipe),
            Some(ColorEffectSpatialRecipe::ColorRainbow {
                grayscale: false,
                vertical_symmetry: false,
                color_width: 0.25,
                angle_degrees: 45.0,
                gradient: 75.0,
            })
        ));
        assert!(converted.approximations.is_empty());
    }

    #[test]
    fn dvc_color_fx_grayscale_and_all_verified_transforms_import_exactly() {
        let convert = |xml: &str, generator_id: u16| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_color_spatial_effect(
                scene,
                "Transform evidence",
                rack,
                effect,
                generator_id,
                1,
                &effect_test_fixture_refs(),
            )
            .unwrap()
        };
        let recipe = |converted: ConvertedDvcEffect| {
            assert!(converted.approximations.is_empty());
            let Some(EffectParamsSnapshot::Color(request)) =
                converted.target.expect("runtime target").params
            else {
                panic!("COLOR/MAPPINGS generator must convert to Color params");
            };
            request.spatial_pattern.expect("spatial pattern").recipe
        };
        let palette = r#"<PARAM TYPE="4" ID="1"><COLORS NB="2"><COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM>"#;

        let plasma_xml = format!(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="2"><EFFECT TYPE="2" ID="129" DURATION="1000"><PARAMS NB="11">{palette}<PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="1"/><PARAM TYPE="0" ID="10" VAL="1"/><PARAM TYPE="0" ID="11" VAL="2"/><PARAM TYPE="0" ID="12" VAL="1"/><PARAM TYPE="0" ID="13" VAL="2"/><PARAM TYPE="0" ID="14" VAL="-1"/><PARAM TYPE="0" ID="15" VAL="2"/><PARAM TYPE="0" ID="16" VAL="1"/><PARAM TYPE="0" ID="17" VAL="-1"/></PARAMS></EFFECT><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS></RACK></RACKS></SCENE>"#
        );
        assert!(matches!(
            recipe(convert(&plasma_xml, 129)),
            ColorEffectSpatialRecipe::Plasma {
                grayscale: true,
                vertical_symmetry: true,
                ..
            }
        ));

        let rainbow_xml = format!(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="2"><EFFECT TYPE="2" ID="130" DURATION="1000"><PARAMS NB="6">{palette}<PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="1"/><PARAM TYPE="1" ID="10" VAL="0.25"/><PARAM TYPE="0" ID="11" VAL="45"/><PARAM TYPE="1" ID="12" VAL="0.75"/></PARAMS></EFFECT><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS></RACK></RACKS></SCENE>"#
        );
        assert!(matches!(
            recipe(convert(&rainbow_xml, 130)),
            ColorEffectSpatialRecipe::ColorRainbow {
                grayscale: true,
                vertical_symmetry: true,
                ..
            }
        ));

        let sweep_xml = format!(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="2"><EFFECT TYPE="2" ID="134" DURATION="1000"><PARAMS NB="4">{palette}<PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="1"/><PARAM TYPE="2" ID="10" VAL="1"/></PARAMS></EFFECT><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS></RACK></RACKS></SCENE>"#
        );
        let converted = convert(&sweep_xml, 134);
        assert!(converted
            .note
            .contains("evaluator=CSweepEffect corrected analytic route"));
        assert!(matches!(
            recipe(converted),
            ColorEffectSpatialRecipe::Sweep {
                grayscale: true,
                vertical_symmetry: true,
                direction_change: true,
            }
        ));

        let mapping_xml = format!(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="2"><EFFECT TYPE="2" ID="521" DURATION="1000"><PARAMS NB="6">{palette}<PARAM TYPE="6" ID="3" VAL="2"/><PARAM TYPE="0" ID="4" VAL="0"/><PARAM TYPE="1" ID="10" VAL="0.5"/><PARAM TYPE="0" ID="11" VAL="90"/><PARAM TYPE="1" ID="12" VAL="1"/></PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="mapping-transform" TYPE="0" X="0" Y="0" SX="100" SY="100" ANGLE="0" LOCKED="0"/><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS></RACK></RACKS></SCENE>"#
        );
        assert!(matches!(
            recipe(convert(&mapping_xml, 521)),
            ColorEffectSpatialRecipe::Rainbow {
                grayscale: false,
                vertical_symmetry: false,
                horizontal_symmetry: true,
                color_width: 50.0,
                ..
            }
        ));
        let mapping_converted = convert(&mapping_xml, 521);
        let Some(EffectParamsSnapshot::Color(mapping_request)) =
            mapping_converted.target.expect("runtime target").params
        else {
            panic!("MAPPINGS 521 must convert to Color params");
        };
        assert_eq!(
            mapping_request
                .spatial_pattern
                .expect("MAPPINGS 521 spatial pattern")
                .placement,
            Some(ColorEffectSpatialPlacement {
                source_coordinate_frame: ColorEffectSpatialCoordinateFrame::DaslightPatchCanvas,
                mapping_shape: ColorEffectSpatialMappingShape::Rectangle,
                x: 0,
                y: 0,
                sx: 100,
                sy: 100,
                mapping_angle_degrees: 0.0,
                sampling_rule:
                    ColorEffectSpatialSamplingRule::RotatedInclusionMaskAxisAlignedRaster,
                vertical_symmetry: false,
                horizontal_symmetry: false,
                raster_rotation_degrees: 0.0,
                target_coordinates: vec![ColorEffectSpatialPlacementTarget {
                    fixture_id: 1,
                    beam_index: 0,
                    patch_x: 10,
                    patch_y: 20,
                }],
            })
        );
    }

    #[test]
    fn dvc_exact_color_and_mapping_routes_reject_factory_schema_drift() {
        let palette = r#"<PARAM TYPE="4" ID="1"><COLORS NB="2"><COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM>"#;
        let source = |rack_type: u16,
                      effect_type: u16,
                      generator_id: u16,
                      params: &str,
                      mapping: &str| {
            format!(
                r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="{rack_type}"><EFFECT TYPE="{effect_type}" ID="{generator_id}" DURATION="1000"><PARAMS NB="{}">{palette}{params}</PARAMS></EFFECT>{mapping}<BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS></RACK></RACKS></SCENE>"#,
                params.matches("<PARAM ").count() + 1
            )
        };
        let convert = |xml: &str, rack_type: u16, effect_type: u16, generator_id: u16| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "strict schema",
                rack,
                effect,
                rack_type,
                effect_type,
                generator_id,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };

        let plasma = source(
            2,
            2,
            129,
            r#"<PARAM TYPE="2" ID="2" VAL="0"/><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="10" VAL="1"/><PARAM TYPE="0" ID="11" VAL="2"/><PARAM TYPE="0" ID="12" VAL="1"/><PARAM TYPE="0" ID="13" VAL="2"/><PARAM TYPE="0" ID="14" VAL="-1"/><PARAM TYPE="0" ID="15" VAL="2"/><PARAM TYPE="0" ID="16" VAL="1"/><PARAM TYPE="0" ID="17" VAL="-1"/>"#,
            "",
        );
        assert!(convert(&plasma, 2, 2, 129).is_ok());
        for invalid in [
            plasma.replacen(r#"TYPE="0" ID="10""#, r#"TYPE="1" ID="10""#, 1),
            plasma.replacen(r#"ID="10" VAL="1""#, r#"ID="10" VAL="21""#, 1),
        ] {
            assert!(convert(&invalid, 2, 2, 129).is_err());
        }

        let rainbow = source(
            2,
            2,
            130,
            r#"<PARAM TYPE="2" ID="2" VAL="0"/><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="1" ID="10" VAL="0.25"/><PARAM TYPE="0" ID="11" VAL="45"/><PARAM TYPE="1" ID="12" VAL="0.75"/>"#,
            "",
        );
        assert!(convert(&rainbow, 2, 2, 130).is_ok());
        for invalid in [
            rainbow.replacen(r#"TYPE="0" ID="11""#, r#"TYPE="1" ID="11""#, 1),
            rainbow.replacen(r#"ID="10" VAL="0.25""#, r#"ID="10" VAL="1.01""#, 1),
            rainbow.replacen(r#"ID="11" VAL="45""#, r#"ID="11" VAL="361""#, 1),
        ] {
            assert!(convert(&invalid, 2, 2, 130).is_err());
        }

        let sweep = source(
            2,
            2,
            134,
            r#"<PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="1"/><PARAM TYPE="2" ID="10" VAL="1"/>"#,
            "",
        );
        assert!(convert(&sweep, 2, 2, 134).is_ok());
        for invalid in [
            sweep.replacen(r#"TYPE="2" ID="10""#, r#"TYPE="0" ID="10""#, 1),
            sweep.replacen(r#"ID="2" VAL="1""#, r#"ID="2" VAL="2""#, 1),
            sweep.replacen(r#"ID="3" VAL="1""#, r#"ID="3" VAL="2""#, 1),
            sweep.replacen(r#"ID="10" VAL="1""#, r#"ID="10" VAL="2""#, 1),
        ] {
            assert!(convert(&invalid, 2, 2, 134).is_err());
        }

        let mapping = source(
            6,
            8,
            521,
            r#"<PARAM TYPE="6" ID="3" VAL="1"/><PARAM TYPE="0" ID="4" VAL="171"/><PARAM TYPE="1" ID="10" VAL="0.25"/><PARAM TYPE="0" ID="11" VAL="0"/><PARAM TYPE="1" ID="12" VAL="1"/>"#,
            r#"<MAPPING NAME="Rectangle" DASUID="mapping-521" TYPE="0" X="0" Y="0" SX="100" SY="100" ANGLE="0" LOCKED="0"/>"#,
        );
        let converted = convert(&mapping, 6, 8, 521).unwrap();
        let Some(EffectParamsSnapshot::Color(request)) = converted.target.unwrap().params else {
            panic!("MAPPINGS 521 must convert to Color params");
        };
        assert!(matches!(
            request.spatial_pattern.unwrap().recipe,
            ColorEffectSpatialRecipe::Rainbow {
                color_width: 25.0,
                ..
            }
        ));
        for invalid in [
            mapping.replacen(r#"TYPE="1" ID="10""#, r#"TYPE="0" ID="10""#, 1),
            mapping.replacen(r#"ID="10" VAL="0.25""#, r#"ID="10" VAL="1.01""#, 1),
            mapping.replacen(r#"ID="4" VAL="171""#, r#"ID="4" VAL="171.5""#, 1),
        ] {
            assert!(convert(&invalid, 6, 8, 521).is_err());
        }
    }

    #[test]
    fn dvc_color_mappings_rainbow_imports_owned_beams_with_exact_patch_placement() {
        let xml = r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="5"><EFFECT TYPE="3" ID="36" DURATION="1000"><PARAMS NB="7"><PARAM TYPE="4" ID="1"><COLORS NB="8"><COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/1/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="1/1/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="1/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/1/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="1/1/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM><PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="2"/><PARAM TYPE="0" ID="4" VAL="45"/><PARAM TYPE="1" ID="10" VAL="0.25"/><PARAM TYPE="0" ID="11" VAL="90"/><PARAM TYPE="1" ID="12" VAL="0.75"/></PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="mapping-36" TYPE="0" X="0" Y="0" SX="200" SY="100" ANGLE="30" LOCKED="0"/><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#;
        let document = Document::parse(xml).unwrap();
        let scene = document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let converted = convert_dvc_effect(
            scene,
            "COLOR MAPPINGS owned beams",
            rack,
            effect,
            5,
            3,
            36,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        assert!(converted.approximations.is_empty());
        assert!(converted.note.contains("source_family=Color Mappings"));
        let Some(EffectParamsSnapshot::Color(request)) = converted
            .target
            .expect("owned beams create a runtime target")
            .params
        else {
            panic!("COLOR MAPPINGS 36 must convert to Color params");
        };
        assert_eq!(request.fixture_ids, vec![1, 2]);
        assert_eq!(request.stops.len(), 8);
        assert_eq!(request.blend_mode, EffectBlendMode::Override);
        let pattern = request.spatial_pattern.expect("spatial pattern");
        assert_eq!(
            pattern.recipe,
            ColorEffectSpatialRecipe::Rainbow {
                grayscale: true,
                vertical_symmetry: false,
                horizontal_symmetry: true,
                rotation_degrees: 45.0,
                color_width: 25.0,
                angle_degrees: 90.0,
                gradient: 75.0,
            }
        );
        assert_eq!(
            pattern.placement,
            Some(ColorEffectSpatialPlacement {
                source_coordinate_frame: ColorEffectSpatialCoordinateFrame::DaslightPatchCanvas,
                mapping_shape: ColorEffectSpatialMappingShape::Rectangle,
                x: 0,
                y: 0,
                sx: 200,
                sy: 100,
                mapping_angle_degrees: 30.0,
                sampling_rule:
                    ColorEffectSpatialSamplingRule::RotatedInclusionMaskAxisAlignedRaster,
                vertical_symmetry: false,
                horizontal_symmetry: false,
                raster_rotation_degrees: 0.0,
                target_coordinates: vec![
                    ColorEffectSpatialPlacementTarget {
                        fixture_id: 1,
                        beam_index: 0,
                        patch_x: 10,
                        patch_y: 20,
                    },
                    ColorEffectSpatialPlacementTarget {
                        fixture_id: 2,
                        beam_index: 0,
                        patch_x: 110,
                        patch_y: 20,
                    },
                ],
            })
        );

        let palette_variant = |count: usize| {
            let start_marker = "<COLORS NB=\"8\">";
            let start = xml.find(start_marker).unwrap();
            let content_start = start + start_marker.len();
            let end = content_start + xml[content_start..].find("</COLORS>").unwrap();
            let color = r#"<COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/>"#;
            format!(
                "{}<COLORS NB=\"{count}\">{}</COLORS>{}",
                &xml[..start],
                color.repeat(count),
                &xml[end + "</COLORS>".len()..]
            )
        };
        let convert_palette = |source: &str| {
            let document = Document::parse(source).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "COLOR MAPPINGS palette boundary",
                rack,
                effect,
                5,
                3,
                36,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };
        for count in [1, 8, 255] {
            let converted = convert_palette(&palette_variant(count)).unwrap();
            let Some(EffectParamsSnapshot::Color(request)) = converted.target.unwrap().params
            else {
                panic!("COLOR MAPPINGS 36 must preserve a legal {count}-color palette");
            };
            assert_eq!(request.stops.len(), count);
        }
        for count in [0, 256] {
            let error = convert_palette(&palette_variant(count)).unwrap_err();
            assert!(
                error.contains("palette requires 1..255"),
                "expected palette boundary error for {count}, found {error:?}"
            );
        }
        for (invalid_xml, expected_error) in [
            (
                xml.replacen("<PARAMS NB=\"7\">", "<PARAMS NB=\"8\">", 1),
                "expected PARAMS NB=7",
            ),
            (
                xml.replacen(
                    "LOCKED=\"0\"/>",
                    "LOCKED=\"0\" SOURCEEXTRA=\"unexpected\"/>",
                    1,
                ),
                "expected Rectangle attributes",
            ),
            (
                xml.replacen("<BEAMS NB=\"2\">", "<BEAMS>", 1),
                "BEAMS is missing NB",
            ),
        ] {
            let invalid_document = Document::parse(&invalid_xml).unwrap();
            let invalid_scene = invalid_document.root_element();
            let invalid_rack =
                direct_child(direct_child(invalid_scene, "RACKS").unwrap(), "RACK").unwrap();
            let invalid_effect = direct_child(invalid_rack, "EFFECT").unwrap();
            let error = convert_dvc_effect(
                invalid_scene,
                "COLOR MAPPINGS invalid schema",
                invalid_rack,
                invalid_effect,
                5,
                3,
                36,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
            .unwrap_err();
            assert!(
                error.contains(expected_error),
                "expected {expected_error:?}, found {error:?}"
            );
        }
    }

    #[test]
    fn dvc_color_mappings_shared_classes_use_owned_color_targets_and_shared_placement() {
        let palette = r#"<PARAM TYPE="4" ID="1"><COLORS NB="2"><COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM>"#;
        let common = r#"<PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="2"/><PARAM TYPE="0" ID="4" VAL="90"/>"#;
        let mapping = r#"<MAPPING NAME="Rectangle" DASUID="color-mappings-shared" TYPE="0" X="0" Y="0" SX="200" SY="100" ANGLE="30" LOCKED="0"/>"#;
        let source = |generator_id: u16, specific: &str, beams: &str| {
            let params = format!("{common}{specific}");
            format!(
                r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="5"><EFFECT TYPE="3" ID="{generator_id}" DURATION="1000"><PARAMS NB="{}">{palette}{params}</PARAMS></EFFECT>{mapping}{beams}</RACK></RACKS></SCENE>"#,
                params.matches("<PARAM ").count() + 1
            )
        };
        let convert = |xml: &str, generator_id: u16| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "COLOR MAPPINGS shared class",
                rack,
                effect,
                5,
                3,
                generator_id,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };
        let beams =
            r#"<BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS>"#;
        let cases = [
            (
                22,
                r#"<PARAM TYPE="0" ID="10" VAL="50"/><PARAM TYPE="1" ID="11" VAL="1"/>"#,
            ),
            (
                23,
                r#"<PARAM TYPE="0" ID="10" VAL="100"/><PARAM TYPE="1" ID="11" VAL="0.5"/><PARAM TYPE="2" ID="12" VAL="1"/>"#,
            ),
            (
                30,
                r#"<PARAM TYPE="0" ID="10" VAL="1"/><PARAM TYPE="2" ID="11" VAL="0"/><PARAM TYPE="2" ID="12" VAL="1"/><PARAM TYPE="2" ID="13" VAL="0"/><PARAM TYPE="0" ID="14" VAL="50"/>"#,
            ),
            (
                32,
                r#"<PARAM TYPE="0" ID="10" VAL="4"/><PARAM TYPE="0" ID="11" VAL="75"/><PARAM TYPE="0" ID="12" VAL="2"/><PARAM TYPE="0" ID="13" VAL="1"/><PARAM TYPE="0" ID="14" VAL="70"/>"#,
            ),
            (
                34,
                r#"<PARAM TYPE="0" ID="10" VAL="1"/><PARAM TYPE="0" ID="11" VAL="2"/><PARAM TYPE="0" ID="12" VAL="1"/><PARAM TYPE="0" ID="13" VAL="2"/><PARAM TYPE="0" ID="14" VAL="-1"/><PARAM TYPE="0" ID="15" VAL="2"/><PARAM TYPE="0" ID="16" VAL="1"/><PARAM TYPE="0" ID="17" VAL="-1"/>"#,
            ),
            (
                40,
                r#"<PARAM TYPE="0" ID="10" VAL="5"/><PARAM TYPE="1" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="1"/><PARAM TYPE="0" ID="13" VAL="1"/>"#,
            ),
            (
                42,
                r#"<PARAM TYPE="0" ID="10" VAL="30"/><PARAM TYPE="0" ID="11" VAL="1"/><PARAM TYPE="1" ID="12" VAL="1"/>"#,
            ),
            (44, r#"<PARAM TYPE="2" ID="10" VAL="0"/>"#),
        ];

        for (generator_id, specific) in cases {
            let converted = convert(&source(generator_id, specific, beams), generator_id).unwrap();
            assert!(converted.approximations.is_empty());
            assert!(converted.note.contains("source_family=Color Mappings;"));
            let Some(EffectParamsSnapshot::Color(request)) =
                converted.target.expect("owned color target").params
            else {
                panic!("COLOR MAPPINGS {generator_id} must convert to Color params");
            };
            assert_eq!(request.fixture_ids, vec![1]);
            assert_eq!(request.stops.len(), 2);
            assert_eq!(request.blend_mode, EffectBlendMode::Override);
            let pattern = request.spatial_pattern.expect("shared spatial pattern");
            assert_eq!(pattern.beam_targets.len(), 1);
            assert_eq!(pattern.beam_targets[0].feature_attribute, None);
            let placement = pattern.placement.expect("Rectangle placement");
            assert_eq!(
                placement.target_coordinates,
                vec![ColorEffectSpatialPlacementTarget {
                    fixture_id: 1,
                    beam_index: 0,
                    patch_x: 10,
                    patch_y: 20,
                }]
            );
            assert_eq!(placement.mapping_angle_degrees, 30.0);

            let grayscale = match &pattern.recipe {
                ColorEffectSpatialRecipe::Burst { grayscale, .. }
                | ColorEffectSpatialRecipe::Butterfly { grayscale, .. }
                | ColorEffectSpatialRecipe::KnightRider { grayscale, .. }
                | ColorEffectSpatialRecipe::Perlin { grayscale, .. }
                | ColorEffectSpatialRecipe::Plasma { grayscale, .. }
                | ColorEffectSpatialRecipe::Sparkle { grayscale, .. }
                | ColorEffectSpatialRecipe::Spiral { grayscale, .. }
                | ColorEffectSpatialRecipe::Sweep { grayscale, .. } => *grayscale,
                _ => false,
            };
            assert!(grayscale, "COLOR MAPPINGS {generator_id} lost Grayscale");
            match generator_id {
                22 => assert!(matches!(
                    pattern.recipe,
                    ColorEffectSpatialRecipe::Burst { .. }
                )),
                23 => assert!(matches!(
                    pattern.recipe,
                    ColorEffectSpatialRecipe::Butterfly { .. }
                )),
                30 => assert!(matches!(
                    pattern.recipe,
                    ColorEffectSpatialRecipe::KnightRider { .. }
                )),
                32 => assert!(matches!(
                    pattern.recipe,
                    ColorEffectSpatialRecipe::Perlin {
                        horizontal_symmetry: true,
                        rotation_degrees: 90.0,
                        ..
                    }
                )),
                34 => assert!(matches!(
                    pattern.recipe,
                    ColorEffectSpatialRecipe::Plasma { .. }
                )),
                40 => assert!(matches!(
                    pattern.recipe,
                    ColorEffectSpatialRecipe::Sparkle { .. }
                )),
                42 => assert!(matches!(
                    pattern.recipe,
                    ColorEffectSpatialRecipe::Spiral { .. }
                )),
                44 => assert!(matches!(
                    pattern.recipe,
                    ColorEffectSpatialRecipe::Sweep { .. }
                )),
                _ => unreachable!(),
            }
            if generator_id == 32 {
                assert!(!placement.vertical_symmetry);
                assert!(!placement.horizontal_symmetry);
                assert_eq!(placement.raster_rotation_degrees, 0.0);
            } else {
                assert!(!placement.vertical_symmetry);
                assert!(placement.horizontal_symmetry);
                assert_eq!(placement.raster_rotation_degrees, 90.0);
            }
        }

        for (generator_id, invalid_specific) in [
            (
                22,
                r#"<PARAM TYPE="0" ID="10" VAL="9"/><PARAM TYPE="1" ID="11" VAL="1"/>"#,
            ),
            (
                23,
                r#"<PARAM TYPE="0" ID="10" VAL="0"/><PARAM TYPE="1" ID="11" VAL="0.5"/><PARAM TYPE="2" ID="12" VAL="1"/>"#,
            ),
            (
                30,
                r#"<PARAM TYPE="0" ID="10" VAL="0"/><PARAM TYPE="2" ID="11" VAL="0"/><PARAM TYPE="2" ID="12" VAL="1"/><PARAM TYPE="2" ID="13" VAL="0"/><PARAM TYPE="0" ID="14" VAL="50"/>"#,
            ),
            (
                32,
                r#"<PARAM TYPE="0" ID="10" VAL="1"/><PARAM TYPE="0" ID="11" VAL="75"/><PARAM TYPE="0" ID="12" VAL="2"/><PARAM TYPE="0" ID="13" VAL="1"/><PARAM TYPE="0" ID="14" VAL="70"/>"#,
            ),
            (
                34,
                r#"<PARAM TYPE="0" ID="10" VAL="21"/><PARAM TYPE="0" ID="11" VAL="2"/><PARAM TYPE="0" ID="12" VAL="1"/><PARAM TYPE="0" ID="13" VAL="2"/><PARAM TYPE="0" ID="14" VAL="-1"/><PARAM TYPE="0" ID="15" VAL="2"/><PARAM TYPE="0" ID="16" VAL="1"/><PARAM TYPE="0" ID="17" VAL="-1"/>"#,
            ),
            (
                40,
                r#"<PARAM TYPE="0" ID="10" VAL="11"/><PARAM TYPE="1" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="1"/><PARAM TYPE="0" ID="13" VAL="1"/>"#,
            ),
            (
                42,
                r#"<PARAM TYPE="0" ID="10" VAL="201"/><PARAM TYPE="0" ID="11" VAL="1"/><PARAM TYPE="1" ID="12" VAL="1"/>"#,
            ),
            (44, r#"<PARAM TYPE="2" ID="10" VAL="2"/>"#),
        ] {
            assert!(
                convert(&source(generator_id, invalid_specific, beams), generator_id).is_err(),
                "COLOR MAPPINGS {generator_id} accepted an out-of-domain class parameter"
            );
        }

        let burst = source(22, cases[0].1, beams);
        for invalid in [
            burst.replacen(r#"TYPE="2" ID="2""#, r#"TYPE="0" ID="2""#, 1),
            burst.replacen(r#"ID="2" VAL="1""#, r#"ID="2" VAL="2""#, 1),
            burst.replacen(r#"ID="3" VAL="2""#, r#"ID="3" VAL="3""#, 1),
            burst.replacen(r#"ID="4" VAL="90""#, r#"ID="4" VAL="361""#, 1),
        ] {
            assert!(convert(&invalid, 22).is_err());
        }

        let missing_grayscale = burst
            .replacen(r#"<PARAM TYPE="2" ID="2" VAL="1"/>"#, "", 1)
            .replacen(r#"<PARAMS NB="6">"#, r#"<PARAMS NB="5">"#, 1);
        for (case, invalid, expected_error) in [
            (
                "missing PARAM 2",
                missing_grayscale,
                "expected PARAM IDs [2, 3, 4, 10, 11]",
            ),
            (
                "PARAMS NB mismatch",
                burst.replacen(r#"<PARAMS NB="6">"#, r#"<PARAMS NB="5">"#, 1),
                "PARAMS declares 5 entries but contains 6",
            ),
            (
                "unexpected palette child",
                burst.replacen(r#"<COLORS NB="2">"#, r#"<COLORS NB="2"><NOT_COLOR/>"#, 1),
                "palette COLORS contains unexpected <NOT_COLOR> element",
            ),
            (
                "COLORS NB mismatch",
                burst.replacen(r#"<COLORS NB="2">"#, r#"<COLORS NB="1">"#, 1),
                "COLORS declares 1 entries but contains 2",
            ),
        ] {
            let error = convert(&invalid, 22).unwrap_err();
            assert!(
                error.contains(expected_error),
                "{case}: expected {expected_error:?}, found {error:?}"
            );
        }

        let extra_rectangle_attribute = |generator_id, specific| {
            source(generator_id, specific, beams).replacen(
                r#"LOCKED="0"/>"#,
                r#"LOCKED="0" SOURCEEXTRA="unexpected"/>"#,
                1,
            )
        };
        let perlin_error = convert(&extra_rectangle_attribute(32, cases[3].1), 32).unwrap_err();
        assert!(perlin_error.contains("expected Rectangle attributes"));
        assert!(
            convert(&extra_rectangle_attribute(22, cases[0].1), 22).is_ok(),
            "only the shared Perlin/530 strict Rectangle mode should reject extra source attributes"
        );

        let empty = source(22, cases[0].1, r#"<BEAMS NB="0"/>"#);
        let no_op = convert(&empty, 22).unwrap();
        assert!(no_op.target.is_none());
        assert!(no_op.note.contains("source no-op preserved"));

        let external_selection = burst.replacen("<BEAMS NB=", "<SELECTIONS/><BEAMS NB=", 1);
        assert!(convert(&external_selection, 22)
            .unwrap_err()
            .contains("external SELECTIONS remain fail-closed"));
    }

    #[test]
    fn dvc_color_mappings_tube_is_full_height_shared_sparkle_and_fails_closed() {
        let source = || {
            r#"<SCENE DASUID="tube-scene" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="5" DASUID="tube-rack"><EFFECT TYPE="3" ID="41" DASUID="tube-effect" DURATION="1000"><PARAMS NB="7"><PARAM TYPE="4" ID="1"><COLORS NB="2"><COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM><PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="2"/><PARAM TYPE="0" ID="4" VAL="90"/><PARAM TYPE="0" ID="10" VAL="5"/><PARAM TYPE="1" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="1"/></PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="tube-rectangle" TYPE="0" X="0" Y="0" SX="200" SY="100" ANGLE="30" LOCKED="0"/><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS></RACK></RACKS></SCENE>"#.to_string()
        };
        let convert = |xml: &str| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "COLOR MAPPINGS Tube",
                rack,
                effect,
                5,
                3,
                41,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };

        let valid = source();
        let first = convert(&valid).unwrap();
        let repeated = convert(&valid).unwrap();
        assert!(first.approximations.is_empty());
        assert!(first.note.contains("source_family=Color Mappings;"));
        assert!(first
            .note
            .contains("evaluator=CTubeEffect shared retained-particle grammar"));
        assert!(first.note.contains("raster_height=full_100_rows"));
        assert_eq!(
            serde_json::to_value(&first.target).unwrap(),
            serde_json::to_value(&repeated.target).unwrap(),
            "Tube's corrected source-family seed must be reload-stable"
        );
        let Some(EffectParamsSnapshot::Color(request)) =
            first.target.expect("owned Tube target").params
        else {
            panic!("COLOR MAPPINGS 41 must create an owned Color target");
        };
        assert_eq!(request.fixture_ids, vec![1]);
        assert_eq!(request.stops.len(), 2);
        assert_eq!(request.period_ms, 1_000);
        assert_eq!(request.blend_mode, EffectBlendMode::Override);
        let pattern = request.spatial_pattern.expect("Tube spatial pattern");
        assert_eq!(pattern.beam_targets.len(), 1);
        assert_eq!(pattern.beam_targets[0].feature_attribute, None);
        assert!(matches!(
            pattern.recipe,
            ColorEffectSpatialRecipe::Sparkle {
                grayscale: true,
                vertical_symmetry: false,
                raster_mode: ColorEffectSpatialSparkleRasterMode::TubeFullRasterHeight,
                number: 5,
                lifetime_ms: Some(100),
                source_lifespan: Some(0.0),
                width: 1.0,
                height: None,
                ..
            }
        ));
        let placement = pattern.placement.expect("positive Rectangle placement");
        assert_eq!(placement.mapping_angle_degrees, 30.0);
        assert!(!placement.vertical_symmetry);
        assert!(placement.horizontal_symmetry);
        assert_eq!(placement.raster_rotation_degrees, 90.0);
        assert_eq!(
            placement.target_coordinates,
            vec![ColorEffectSpatialPlacementTarget {
                fixture_id: 1,
                beam_index: 0,
                patch_x: 10,
                patch_y: 20,
            }]
        );

        let importer_precision =
            convert(&valid.replacen(r#"ID="11" VAL="0""#, r#"ID="11" VAL="0.0049751224""#, 1))
                .unwrap();
        let Some(EffectParamsSnapshot::Color(precision_request)) = importer_precision
            .target
            .expect("precision Tube target")
            .params
        else {
            unreachable!();
        };
        assert!(matches!(
            precision_request
                .spatial_pattern
                .as_ref()
                .map(|pattern| &pattern.recipe),
            Some(ColorEffectSpatialRecipe::Sparkle {
                lifetime_ms: Some(101),
                source_lifespan: Some(value),
                ..
            }) if value.to_bits() == 0x3BA3_065A
        ));

        for (case, invalid, expected_error) in [
            (
                "PARAMS NB mismatch",
                valid.replacen(r#"<PARAMS NB="7">"#, r#"<PARAMS NB="6">"#, 1),
                "PARAMS declares 6 entries but contains 7",
            ),
            (
                "Number TYPE",
                valid.replacen(r#"TYPE="0" ID="10""#, r#"TYPE="1" ID="10""#, 1),
                "expected PARAM 10 TYPE=0, found TYPE=1",
            ),
            (
                "Number range",
                valid.replacen(r#"ID="10" VAL="5""#, r#"ID="10" VAL="11""#, 1),
                "integer within 1..10",
            ),
            (
                "LifeSpan range",
                valid.replacen(r#"ID="11" VAL="0""#, r#"ID="11" VAL="0.91""#, 1),
                "within 0..0.9",
            ),
            (
                "Width range",
                valid.replacen(r#"ID="12" VAL="1""#, r#"ID="12" VAL="91""#, 1),
                "integer within 1..90",
            ),
            (
                "minimum palette",
                valid
                    .replacen(r#"COLORS NB="2""#, r#"COLORS NB="1""#, 1)
                    .replacen(
                        r#"<COLOR VAL="0/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/>"#,
                        "",
                        1,
                    ),
                "palette requires 2..255 colors",
            ),
            (
                "strict Rectangle attributes",
                valid.replacen(r#"LOCKED="0"/>"#, r#"LOCKED="0" EXTRA="1"/>"#, 1),
                "expected Rectangle attributes",
            ),
            (
                "external selection",
                valid.replacen("<BEAMS NB=", "<SELECTIONS/><BEAMS NB=", 1),
                "external SELECTIONS remain fail-closed",
            ),
        ] {
            let error = convert(&invalid).unwrap_err();
            assert!(
                error.contains(expected_error),
                "{case}: expected {expected_error:?}, found {error:?}"
            );
        }

        let positive_empty = valid
            .replacen(r#"<BEAMS NB="1">"#, r#"<BEAMS NB="0">"#, 1)
            .replacen(
                r#"<BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/>"#,
                "",
                1,
            );
        let no_op = convert(&positive_empty).unwrap();
        assert!(no_op.target.is_none());
        assert!(no_op.note.contains("source no-op preserved"));
        assert!(no_op.note.contains("Rectangle=(0,0,200,100,30)"));

        let native_empty = positive_empty.replacen(
            r#"SX="200" SY="100" ANGLE="30""#,
            r#"SX="-1" SY="-1" ANGLE="0""#,
            1,
        );
        let native_no_op = convert(&native_empty).unwrap();
        assert!(native_no_op.target.is_none());
        assert!(native_no_op.note.contains("source no-op preserved"));
        assert!(native_no_op.note.contains("Rectangle=(0,0,-1,-1,0)"));

        let nonempty_native_empty = native_empty.replacen(
            r#"<BEAMS NB="0"></BEAMS>"#,
            r#"<BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS>"#,
            1,
        );
        assert!(convert(&nonempty_native_empty)
            .unwrap_err()
            .contains("native empty Rectangle sentinel is valid only with zero owned BEAMS"));
    }

    #[test]
    fn dvc_color_mappings_random_fill_routes_exact_2d_and_rejects_schema_drift() {
        let source = || {
            r#"<SCENE DASUID="random-fill-scene" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="5" DASUID="random-fill-rack"><EFFECT TYPE="3" ID="37" DASUID="random-fill-effect" DURATION="1000"><PARAMS NB="6"><PARAM TYPE="4" ID="1"><COLORS NB="2"><COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM><PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="2"/><PARAM TYPE="0" ID="4" VAL="90"/><PARAM TYPE="0" ID="10" VAL="6"/><PARAM TYPE="0" ID="11" VAL="7"/></PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="color-mappings-random-fill" TYPE="0" X="0" Y="0" SX="200" SY="100" ANGLE="30" LOCKED="0"/><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS></RACK></RACKS></SCENE>"#.to_string()
        };
        let convert = |xml: &str| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "COLOR MAPPINGS Random fill",
                rack,
                effect,
                5,
                3,
                37,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };

        let first = convert(&source()).unwrap();
        let repeated = convert(&source()).unwrap();
        assert!(first.approximations.is_empty());
        assert!(first.note.contains("source_family=Color Mappings;"));
        assert!(first.note.contains("implementation=SyndocalCorrected"));
        assert!(first.note.contains("unavailable_qrand=stable_source_seed"));
        assert!(first.note.contains("PointHeight_mode=placed_2d_live"));
        assert!(first.note.contains("source_raster=100x100"));
        assert!(first
            .note
            .contains("Transform=Horizontal symmetry; Rotation=90"));
        let first_target = first.target.unwrap();
        let repeated_target = repeated.target.unwrap();
        assert_eq!(
            serde_json::to_value(&repeated_target).unwrap(),
            serde_json::to_value(&first_target).unwrap(),
            "the source-family/generator-aware seed must be reload-stable"
        );
        let Some(EffectParamsSnapshot::Color(request)) = first_target.params else {
            panic!("COLOR MAPPINGS 37 must create an owned Color target");
        };
        assert_eq!(request.fixture_ids, vec![1]);
        assert_eq!(request.blend_mode, EffectBlendMode::Override);
        assert_eq!(request.stops.len(), 2);
        assert_eq!(request.period_ms, 1_000);
        let pattern = request.spatial_pattern.unwrap();
        assert_eq!(pattern.beam_targets.len(), 1);
        assert_eq!(pattern.beam_targets[0].feature_attribute, None);
        assert!(matches!(
            pattern.recipe,
            ColorEffectSpatialRecipe::RandomFill {
                grayscale: true,
                vertical_symmetry: false,
                point_width: 6.0,
                source_point_height: Some(7),
                rng_seed,
            } if rng_seed != 0
        ));
        let placement = pattern.placement.unwrap();
        assert_eq!(placement.mapping_angle_degrees, 30.0);
        assert!(!placement.vertical_symmetry);
        assert!(placement.horizontal_symmetry);
        assert_eq!(placement.raster_rotation_degrees, 90.0);
        assert_eq!(
            placement.target_coordinates,
            vec![ColorEffectSpatialPlacementTarget {
                fixture_id: 1,
                beam_index: 0,
                patch_x: 10,
                patch_y: 20,
            }]
        );
        let valid = source();
        for (case, invalid, expected_error) in [
            (
                "missing PARAM2",
                valid
                    .replacen(r#"<PARAM TYPE="2" ID="2" VAL="1"/>"#, "", 1)
                    .replacen(r#"<PARAMS NB="6">"#, r#"<PARAMS NB="5">"#, 1),
                "expected PARAM IDs [2, 3, 4, 10, 11]",
            ),
            (
                "PARAMS NB mismatch",
                valid.replacen(r#"<PARAMS NB="6">"#, r#"<PARAMS NB="5">"#, 1),
                "PARAMS declares 5 entries but contains 6",
            ),
            (
                "wrong PointHeight TYPE",
                valid.replacen(r#"TYPE="0" ID="11""#, r#"TYPE="1" ID="11""#, 1),
                "expected PARAM 11 TYPE=0, found TYPE=1",
            ),
            (
                "malformed palette child",
                valid.replacen("</COLORS></PARAM>", "</COLORS><EXTRA/></PARAM>", 1),
                "palette PARAM 1 must contain exactly one direct COLORS element",
            ),
            (
                "COLORS NB mismatch",
                valid.replacen(r#"<COLORS NB="2">"#, r#"<COLORS NB="1">"#, 1),
                "COLORS declares 1 entries but contains 2",
            ),
        ] {
            let error = convert(&invalid).unwrap_err();
            assert!(
                error.contains(expected_error),
                "{case}: expected {expected_error:?}, found {error:?}"
            );
        }

        for (attribute, invalid_value, expected_error) in [
            (r#"ID="2" VAL="1""#, r#"ID="2" VAL="2""#, "must be 0 or 1"),
            (r#"ID="3" VAL="2""#, r#"ID="3" VAL="3""#, "must be None(0)"),
            (
                r#"ID="4" VAL="90""#,
                r#"ID="4" VAL="90.5""#,
                "integer within 0..360",
            ),
            (
                r#"ID="10" VAL="6""#,
                r#"ID="10" VAL="0""#,
                "integer within 1..10",
            ),
            (
                r#"ID="11" VAL="7""#,
                r#"ID="11" VAL="11""#,
                "integer within 1..10",
            ),
        ] {
            let error = convert(&valid.replacen(attribute, invalid_value, 1)).unwrap_err();
            assert!(
                error.contains(expected_error),
                "expected {expected_error:?}, found {error:?}"
            );
        }

        let external_selection = valid.replacen("<BEAMS NB=", "<SELECTIONS/><BEAMS NB=", 1);
        assert!(convert(&external_selection)
            .unwrap_err()
            .contains("external SELECTIONS remain fail-closed"));
    }

    #[test]
    fn dvc_color_mappings_graph_grid_lines_import_strict_owned_placed_rasters() {
        let source = |generator_id: u16, palette_count: usize, beams: &str| {
            let color = r#"<COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/>"#;
            let (specific, nb) = match generator_id {
                49 => (
                    r#"<PARAM TYPE="0" ID="10" VAL="100"/><PARAM TYPE="0" ID="11" VAL="100"/><PARAM TYPE="0" ID="12" VAL="100"/><PARAM TYPE="0" ID="13" VAL="10"/><PARAM TYPE="1" ID="14" VAL="2"/><PARAM TYPE="1" ID="15" VAL="1"/>"#,
                    10,
                ),
                50 => (
                    r#"<PARAM TYPE="0" ID="10" VAL="5"/><PARAM TYPE="0" ID="11" VAL="20"/>"#,
                    6,
                ),
                _ => (r#"<PARAM TYPE="0" ID="10" VAL="20"/>"#, 5),
            };
            format!(
                r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="5"><EFFECT TYPE="3" ID="{generator_id}" DURATION="1"><PARAMS NB="{nb}"><PARAM TYPE="4" ID="1"><COLORS NB="{palette_count}">{}</COLORS></PARAM><PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="2"/><PARAM TYPE="0" ID="4" VAL="90"/>{specific}</PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="graph-grid-lines" TYPE="0" X="0" Y="0" SX="200" SY="100" ANGLE="30" LOCKED="0"/>{beams}</RACK></RACKS></SCENE>"#,
                color.repeat(palette_count)
            )
        };
        let convert_with_refs =
            |xml: &str, generator_id: u16, fixture_refs: &HashMap<String, FixtureImportRef>| {
                let document = Document::parse(xml).unwrap();
                let scene = document.root_element();
                let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
                let effect = direct_child(rack, "EFFECT").unwrap();
                convert_dvc_effect(
                    scene,
                    "COLOR MAPPINGS Graph/Grid/Lines",
                    rack,
                    effect,
                    5,
                    3,
                    generator_id,
                    1,
                    &effect_test_profiles(),
                    fixture_refs,
                )
            };
        let refs = effect_test_fixture_refs();
        let beams = r#"<BEAMS NB="2"><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="2"/></BEAMS>"#;

        for (generator_id, palette_count) in [(50, 5), (31, 255), (49, 10)] {
            let xml = source(generator_id, palette_count, beams);
            let converted = convert_with_refs(&xml, generator_id, &refs).unwrap();
            assert!(converted.approximations.is_empty());
            assert!(converted.note.contains("implementation=SyndocalCorrected"));
            assert!(converted.note.contains("source_family=Color Mappings"));
            assert!(converted
                .note
                .contains("period_ms=max(authored_duration_ms,10)=10"));
            let Some(EffectParamsSnapshot::Color(request)) =
                converted.target.expect("owned Color target").params
            else {
                panic!("COLOR MAPPINGS {generator_id} must create Color params");
            };
            assert_eq!(request.fixture_ids, vec![2, 1]);
            assert_eq!(request.stops.len(), palette_count);
            assert_eq!(request.period_ms, 10);
            assert_eq!(request.blend_mode, EffectBlendMode::Override);
            let pattern = request.spatial_pattern.unwrap();
            assert_eq!(
                pattern.parameter_model_version,
                COLOR_EFFECT_SPATIAL_PARAMETER_MODEL_VERSION
            );
            assert_eq!(
                pattern
                    .beam_targets
                    .iter()
                    .map(|target| (
                        target.fixture_id,
                        target.selection_index,
                        target.feature_attribute.clone()
                    ))
                    .collect::<Vec<_>>(),
                vec![(2, 0, None), (1, 1, None)]
            );
            match (generator_id, pattern.recipe) {
                (
                    50,
                    ColorEffectSpatialRecipe::Grid {
                        grayscale: true,
                        size: 5,
                        width: 20,
                    },
                ) => {}
                (
                    31,
                    ColorEffectSpatialRecipe::Lines {
                        grayscale: true,
                        size: 20,
                    },
                ) => {}
                (
                    49,
                    ColorEffectSpatialRecipe::Graph {
                        grayscale: true,
                        height: 100,
                        width: 100,
                        pitch: 100,
                        frequency: 10,
                        amplitude: 2.0,
                        offset: 1.0,
                    },
                ) => {}
                (_, recipe) => panic!("unexpected Graph/Grid/Lines recipe: {recipe:?}"),
            }
            let placement = pattern.placement.unwrap();
            assert_eq!(placement.mapping_angle_degrees, 30.0);
            assert!(!placement.vertical_symmetry);
            assert!(placement.horizontal_symmetry);
            assert_eq!(placement.raster_rotation_degrees, 90.0);
            assert_eq!(
                placement.target_coordinates,
                vec![
                    ColorEffectSpatialPlacementTarget {
                        fixture_id: 2,
                        beam_index: 0,
                        patch_x: 110,
                        patch_y: 20,
                    },
                    ColorEffectSpatialPlacementTarget {
                        fixture_id: 1,
                        beam_index: 0,
                        patch_x: 10,
                        patch_y: 20,
                    },
                ]
            );
        }

        for (generator_id, palette_count, expected) in [
            (50, 1, "Grid ID=50 palette requires 2..5"),
            (50, 6, "Grid ID=50 palette requires 2..5"),
            (31, 1, "Lines ID=31 palette requires 2..255"),
            (49, 1, "Graph ID=49 palette requires 2..10"),
            (49, 11, "Graph ID=49 palette requires 2..10"),
        ] {
            let error = convert_with_refs(
                &source(generator_id, palette_count, beams),
                generator_id,
                &refs,
            )
            .unwrap_err();
            assert!(
                error.contains(expected),
                "expected {expected:?}, found {error:?}"
            );
        }

        let valid_grid = source(50, 5, beams);
        for (invalid, expected) in [
            (
                valid_grid.replacen(r#"TYPE="0" ID="11""#, r#"TYPE="1" ID="11""#, 1),
                "expected PARAM 11 TYPE=0",
            ),
            (
                valid_grid.replacen(r#"ID="10" VAL="5""#, r#"ID="10" VAL="6""#, 1),
                "integer within 1..5",
            ),
            (
                valid_grid.replacen(r#"ID="11" VAL="20""#, r#"ID="11" VAL="21""#, 1),
                "integer within 2..20",
            ),
            (
                valid_grid.replacen(r#"DURATION="1""#, r#"DURATION="1.5""#, 1),
                "positive signed 32-bit integer",
            ),
            (
                valid_grid.replacen(r#"FIXTURE="fixture-2""#, r#"FIXTURE="missing""#, 1),
                "is not present in the imported patch",
            ),
            (
                valid_grid.replacen("<BEAMS NB=", "<SELECTIONS/><BEAMS NB=", 1),
                "external SELECTIONS remain fail-closed",
            ),
        ] {
            let error = convert_with_refs(&invalid, 50, &refs).unwrap_err();
            assert!(
                error.contains(expected),
                "expected {expected:?}, found {error:?}"
            );
        }

        let valid_graph = source(49, 10, beams);
        for (invalid, expected) in [
            (
                valid_graph.replacen(r#"TYPE="1" ID="14""#, r#"TYPE="0" ID="14""#, 1),
                "expected PARAM 14 TYPE=1",
            ),
            (
                valid_graph.replacen(r#"ID="10" VAL="100""#, r#"ID="10" VAL="0""#, 1),
                "integer within 1..100",
            ),
            (
                valid_graph.replacen(r#"ID="11" VAL="100""#, r#"ID="11" VAL="101""#, 1),
                "integer within 1..100",
            ),
            (
                valid_graph.replacen(r#"ID="12" VAL="100""#, r#"ID="12" VAL="-1""#, 1),
                "integer within 0..100",
            ),
            (
                valid_graph.replacen(r#"ID="13" VAL="10""#, r#"ID="13" VAL="11""#, 1),
                "integer within 0..10",
            ),
            (
                valid_graph.replacen(r#"ID="14" VAL="2""#, r#"ID="14" VAL="2.1""#, 1),
                "within 0..2",
            ),
            (
                valid_graph.replacen(r#"ID="15" VAL="1""#, r#"ID="15" VAL="1.1""#, 1),
                "within -1..1",
            ),
            (
                valid_graph.replacen(r#"LOCKED="0""#, r#"LOCKED="0" EXTRA="1""#, 1),
                "expected Rectangle attributes",
            ),
        ] {
            let error = convert_with_refs(&invalid, 49, &refs).unwrap_err();
            assert!(
                error.contains(expected),
                "expected {expected:?}, found {error:?}"
            );
        }

        for (generator_id, palette_count) in [(50, 5), (31, 2), (49, 2)] {
            let valid_empty = source(generator_id, palette_count, r#"<BEAMS NB="0"/>"#);
            let no_op = convert_with_refs(&valid_empty, generator_id, &refs).unwrap();
            assert!(no_op.target.is_none());
            assert!(no_op.note.contains("source no-op preserved"));
            for (invalid, expected) in [
                (
                    valid_empty.replacen(
                        r#"<BEAMS NB="0"/>"#,
                        r#"<BEAMS NB="0"><unexpected/></BEAMS>"#,
                        1,
                    ),
                    "BEAMS contains unexpected <unexpected> element",
                ),
                (
                    valid_empty.replacen(r#"<BEAMS NB="0"/>"#, r#"<BEAMS NB="1"/>"#, 1),
                    "BEAMS declares 1 entries but contains 0",
                ),
                (
                    valid_empty.replacen(r#"<BEAMS NB="0"/>"#, r#"<BEAMS NB="invalid"/>"#, 1),
                    "BEAMS NB is invalid",
                ),
                (
                    valid_empty.replacen("</RACK>", r#"<BEAMS NB="0"/></RACK>"#, 1),
                    "expected exactly one direct BEAMS container, found 2",
                ),
            ] {
                let error = convert_with_refs(&invalid, generator_id, &refs).unwrap_err();
                assert!(
                    error.contains(expected),
                    "ID={generator_id}: expected {expected:?}, found {error:?}"
                );
            }
        }
        let valid_empty = source(50, 5, r#"<BEAMS NB="0"/>"#);
        let invalid_empty = valid_empty.replacen(r#"ID="10" VAL="5""#, r#"ID="10" VAL="0""#, 1);
        assert!(convert_with_refs(&invalid_empty, 50, &refs).is_err());

        let mut refs_without_color = refs.clone();
        refs_without_color
            .get_mut("fixture-2")
            .unwrap()
            .color_beam_count = 0;
        let error = convert_with_refs(&valid_grid, 50, &refs_without_color).unwrap_err();
        assert!(error.contains("Grid ID=50 requires every owned BEAMS target"));
        let error = convert_with_refs(&valid_graph, 49, &refs_without_color).unwrap_err();
        assert!(error.contains("Graph ID=49 requires every owned BEAMS target"));
    }

    #[test]
    fn dvc_color_mappings_bounce_imports_strict_noop_shape_and_points_routes() {
        let source = |palette_count: usize, item: u8, shape: u8, beams: &str| {
            let color = r#"<COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/>"#;
            format!(
                r#"<SCENE DASUID="bounce-scene" NAME="Bounce Scene" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK EXPAND_RACK="1" TYPE="5"><EFFECT TYPE="3" ID="21" DURATION="5000"><PARAMS NB="12"><PARAM TYPE="0" ID="18" VAL="10"/><PARAM TYPE="4" ID="1"><COLORS NB="{palette_count}">{}</COLORS></PARAM><PARAM TYPE="2" ID="17" VAL="1"/><PARAM TYPE="0" ID="12" VAL="20"/><PARAM TYPE="6" ID="3" VAL="2"/><PARAM TYPE="0" ID="14" VAL="10"/><PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="10" VAL="{item}"/><PARAM TYPE="0" ID="4" VAL="360"/><PARAM TYPE="7" ID="11" VAL="{shape}"/><PARAM TYPE="2" ID="16" VAL="1"/><PARAM TYPE="0" ID="13" VAL="100"/></PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="bounce" TYPE="0" X="0" Y="0" SX="100" SY="100" ANGLE="30" LOCKED="0"/>{beams}</RACK></RACKS></SCENE>"#,
                color.repeat(palette_count)
            )
        };
        let convert = |xml: &str| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "COLOR MAPPINGS Bounce",
                rack,
                effect,
                5,
                3,
                21,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };
        let owned = r#"<BEAMS NB="2"><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="2"/></BEAMS>"#;

        for (item, shape) in [(0, 0), (1, 28)] {
            let converted = convert(&source(255, item, shape, owned)).unwrap();
            assert!(converted.approximations.is_empty());
            assert!(converted.note.contains("implementation=SyndocalCorrected"));
            assert!(converted.note.contains("generation0=update_first"));
            assert!(converted.note.contains("not_Qt_antialias_SourceExact"));
            let Some(EffectParamsSnapshot::Color(request)) =
                converted.target.expect("populated Bounce target").params
            else {
                panic!("Bounce must create Color params");
            };
            assert_eq!(request.fixture_ids, vec![2, 1]);
            assert_eq!(request.stops.len(), 255);
            assert_eq!(request.period_ms, 5_000);
            assert_eq!(request.blend_mode, EffectBlendMode::Override);
            let pattern = request.spatial_pattern.unwrap();
            assert_eq!(pattern.beam_targets.len(), 2);
            match pattern.recipe {
                ColorEffectSpatialRecipe::Bounce {
                    grayscale: true,
                    rng_seed,
                    item: actual_item,
                    shape: actual_shape,
                    number: 20,
                    size: 100,
                    speed: 10,
                    collide: true,
                    fill: true,
                    points: 10,
                } => {
                    assert_ne!(rng_seed, 0);
                    assert_eq!(
                        actual_item,
                        if item == 0 {
                            ColorEffectSpatialBounceItem::Shape
                        } else {
                            ColorEffectSpatialBounceItem::Points
                        }
                    );
                    assert_eq!(actual_shape, shape);
                }
                recipe => panic!("unexpected Bounce recipe: {recipe:?}"),
            }
            let placement = pattern.placement.unwrap();
            assert!(placement.horizontal_symmetry);
            assert_eq!(placement.raster_rotation_degrees, 360.0);
            assert_eq!(placement.target_coordinates.len(), 2);
        }

        let populated_shape = convert(&source(2, 0, 1, owned)).unwrap_err();
        assert!(populated_shape.contains("unsupported XEEL glyph geometry"));
        assert!(convert(&source(1, 1, 28, owned))
            .unwrap_err()
            .contains("populated targets require 2..255"));

        let empty = source(1, 0, 28, r#"<BEAMS NB="0"/>"#).replacen(
            r#"SX="100" SY="100" ANGLE="30""#,
            r#"SX="-1" SY="-1" ANGLE="0""#,
            1,
        );
        let no_op = convert(&empty).unwrap();
        assert!(no_op.target.is_none());
        assert!(no_op.note.contains("source no-op preserved"));
        assert!(no_op.note.contains("Shape=28"));

        let valid = source(2, 1, 28, owned);
        for (invalid, expected) in [
            (
                valid.replacen(r#"<PARAMS NB="12">"#, r#"<PARAMS NB="11">"#, 1),
                "PARAMS declares 11 entries but contains 12",
            ),
            (
                valid.replacen(r#"TYPE="6" ID="10""#, r#"TYPE="0" ID="10""#, 1),
                "expected PARAM 10 TYPE=6",
            ),
            (
                valid.replacen(r#"ID="11" VAL="28""#, r#"ID="11" VAL="29""#, 1),
                "integer within 0..28",
            ),
            (
                valid.replacen(r#"ID="12" VAL="20""#, r#"ID="12" VAL="0""#, 1),
                "integer within 1..20",
            ),
            (
                valid.replacen(r#"ID="13" VAL="100""#, r#"ID="13" VAL="0""#, 1),
                "integer within 1..100",
            ),
            (
                valid.replacen(r#"ID="14" VAL="10""#, r#"ID="14" VAL="11""#, 1),
                "integer within 0..10",
            ),
            (
                valid.replacen(r#"ID="16" VAL="1""#, r#"ID="16" VAL="2""#, 1),
                "must be 0 or 1",
            ),
            (
                valid.replacen(r#"ID="17" VAL="1""#, r#"ID="17" VAL="2""#, 1),
                "must be 0 or 1",
            ),
            (
                valid.replacen(r#"ID="18" VAL="10""#, r#"ID="18" VAL="11""#, 1),
                "integer within 2..10",
            ),
            (
                valid.replacen("<BEAMS NB=", "<SELECTIONS/><BEAMS NB=", 1),
                "RACK contains unexpected <SELECTIONS>",
            ),
            (
                valid.replacen(r#"<BEAMS NB="2">"#, r#"<BEAMS NB="3">"#, 1),
                "BEAMS declares 3 entries but contains 2",
            ),
            (
                valid.replacen(r#"IDSELECTION="1""#, r#"IDSELECTION="1" EXTRA="1""#, 1),
                "attributes must be exactly",
            ),
            (
                valid.replacen(r#"LOCKED="0""#, r#"LOCKED="0" EXTRA="1""#, 1),
                "expected Rectangle attributes",
            ),
        ] {
            let error = convert(&invalid).unwrap_err();
            assert!(
                error.contains(expected),
                "expected {expected:?}, found {error:?}"
            );
        }
        for (invalid, expected) in [
            (
                empty.replacen(r#"<BEAMS NB="0"/>"#, r#"<BEAMS NB="0"><JUNK/></BEAMS>"#, 1),
                "BEAMS contains unexpected <JUNK>",
            ),
            (
                empty.replacen(r#"<BEAMS NB="0"/>"#, r#"<BEAMS NB="invalid"/>"#, 1),
                "BEAMS NB is invalid",
            ),
            (
                empty.replacen("</RACK>", r#"<BEAMS NB="0"/></RACK>"#, 1),
                "expected exactly one direct BEAMS container, found 2",
            ),
        ] {
            let error = convert(&invalid).unwrap_err();
            assert!(
                error.contains(expected),
                "expected {expected:?}, found {error:?}"
            );
        }
    }

    #[test]
    fn dvc_color_mappings_rain_imports_strict_owned_raster_and_rejects_mutations() {
        let source = |palette_count: usize, beams: &str| {
            let color = r#"<COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/>"#;
            format!(
                r#"<SCENE DASUID="rain-scene" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="5" DASUID="rain-rack"><EFFECT TYPE="3" ID="35" DASUID="rain-effect" DURATION="5000"><PARAMS NB="9"><PARAM TYPE="4" ID="1"><COLORS NB="{palette_count}">{}</COLORS></PARAM><PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="2"/><PARAM TYPE="0" ID="4" VAL="90"/><PARAM TYPE="0" ID="13" VAL="100"/><PARAM TYPE="0" ID="10" VAL="10"/><PARAM TYPE="0" ID="11" VAL="10"/><PARAM TYPE="0" ID="12" VAL="30"/><PARAM TYPE="0" ID="14" VAL="30"/></PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="rain" TYPE="0" X="0" Y="0" SX="200" SY="100" ANGLE="30" LOCKED="0"/>{beams}</RACK></RACKS></SCENE>"#,
                color.repeat(palette_count)
            )
        };
        let convert = |xml: &str| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "COLOR MAPPINGS Rain",
                rack,
                effect,
                5,
                3,
                35,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };
        let owned = r#"<BEAMS NB="2"><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="2"/></BEAMS>"#;
        let valid = source(255, owned);
        let converted = convert(&valid).unwrap();
        assert!(converted.approximations.is_empty());
        assert!(converted.note.contains("evaluator=CRainEffect/0x140365660"));
        assert!(converted
            .note
            .contains("paint_order=later-wins_replacement"));
        let Some(EffectParamsSnapshot::Color(request)) =
            converted.target.expect("owned Rain target").params
        else {
            panic!("Rain must create Color params");
        };
        assert_eq!(request.fixture_ids, vec![2, 1]);
        assert_eq!(request.stops.len(), 255);
        assert_eq!(request.period_ms, 5000);
        assert_eq!(request.blend_mode, EffectBlendMode::Override);
        let pattern = request.spatial_pattern.unwrap();
        match pattern.recipe {
            ColorEffectSpatialRecipe::Rain {
                grayscale: true,
                speed: 10,
                width: 10,
                height: 30,
                number: 100,
                trail: 30,
                rng_seed,
            } => assert_ne!(rng_seed, 0),
            recipe => panic!("unexpected Rain recipe: {recipe:?}"),
        }
        let placement = pattern.placement.unwrap();
        assert!(placement.horizontal_symmetry);
        assert_eq!(placement.raster_rotation_degrees, 90.0);

        for (invalid, expected) in [
            (
                valid.replacen(r#"<PARAMS NB="9">"#, r#"<PARAMS NB="8">"#, 1),
                "PARAMS declares 8 entries but contains 9",
            ),
            (
                valid.replacen(r#"TYPE="0" ID="14""#, r#"TYPE="1" ID="14""#, 1),
                "expected PARAM 14 TYPE=0",
            ),
            (
                valid.replacen(r#"ID="10" VAL="10""#, r#"ID="10" VAL="11""#, 1),
                "integer within 0..10",
            ),
            (
                valid.replacen(r#"ID="11" VAL="10""#, r#"ID="11" VAL="4""#, 1),
                "integer within 5..10",
            ),
            (
                valid.replacen(r#"ID="12" VAL="30""#, r#"ID="12" VAL="31""#, 1),
                "integer within 10..30",
            ),
            (
                valid.replacen(r#"ID="13" VAL="100""#, r#"ID="13" VAL="0""#, 1),
                "integer within 1..100",
            ),
            (
                valid.replacen(r#"ID="14" VAL="30""#, r#"ID="14" VAL="31""#, 1),
                "integer within 1..30",
            ),
            (
                valid.replacen("<BEAMS NB=", "<SELECTIONS/><BEAMS NB=", 1),
                "external SELECTIONS remain fail-closed",
            ),
        ] {
            let error = convert(&invalid).unwrap_err();
            assert!(
                error.contains(expected),
                "expected {expected:?}, found {error:?}"
            );
        }
        let single_color = convert(&source(1, owned)).unwrap();
        let Some(EffectParamsSnapshot::Color(single_color)) = single_color
            .target
            .expect("single-color Rain target")
            .params
        else {
            panic!("single-color Rain must create Color params");
        };
        assert_eq!(single_color.stops.len(), 1);
        for palette_count in [0, 256] {
            let error = convert(&source(palette_count, owned)).unwrap_err();
            let expected = "palette requires 1..255";
            assert!(error.contains(expected), "{error}");
        }
        let native_empty = source(2, r#"<BEAMS NB="0"/>"#).replacen(
            r#"SX="200" SY="100" ANGLE="30""#,
            r#"SX="-1" SY="-1" ANGLE="0""#,
            1,
        );
        let no_op = convert(&native_empty).unwrap();
        assert!(no_op.target.is_none());
        assert!(no_op.note.contains("source no-op preserved"));
    }

    #[test]
    fn dvc_color_mappings_fire_imports_strict_owned_raster_and_rejects_mutations() {
        let source = |palette_count: usize, beams: &str| {
            let color = r#"<COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/>"#;
            format!(
                r#"<SCENE DASUID="fire-scene" NAME="Fire Scene" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="5"><EFFECT TYPE="3" ID="29" DURATION="5000"><PARAMS NB="8"><PARAM TYPE="4" ID="1"><COLORS NB="{palette_count}">{}</COLORS></PARAM><PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="2"/><PARAM TYPE="0" ID="4" VAL="360"/><PARAM TYPE="0" ID="10" VAL="100"/><PARAM TYPE="0" ID="11" VAL="200"/><PARAM TYPE="0" ID="12" VAL="100"/><PARAM TYPE="0" ID="13" VAL="255"/></PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="fire" TYPE="0" X="0" Y="0" SX="200" SY="100" ANGLE="30" LOCKED="0"/>{beams}</RACK></RACKS></SCENE>"#,
                color.repeat(palette_count)
            )
        };
        let convert = |xml: &str, rack_type: u16, effect_type: u16| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "COLOR MAPPINGS Fire",
                rack,
                effect,
                rack_type,
                effect_type,
                29,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };
        let owned = r#"<BEAMS NB="2"><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="2"/></BEAMS>"#;
        let valid = source(4, owned);
        let converted = convert(&valid, 5, 3).unwrap();
        assert!(converted.approximations.is_empty());
        assert!(converted.note.contains("evaluator=CFireEffect"));
        assert!(converted.note.contains("source_Height_evaluator_dead=true"));
        assert!(converted
            .note
            .contains("corrected_Height_mode=live_heat_cutoff_rescale"));
        let Some(EffectParamsSnapshot::Color(request)) =
            converted.target.expect("owned Fire target").params
        else {
            panic!("Fire must create Color params");
        };
        assert_eq!(request.fixture_ids, vec![2, 1]);
        assert_eq!(request.stops.len(), 4);
        assert_eq!(request.period_ms, 5_000);
        assert_eq!(request.blend_mode, EffectBlendMode::Override);
        let pattern = request.spatial_pattern.unwrap();
        assert!(matches!(
            pattern.recipe,
            ColorEffectSpatialRecipe::Fire {
                grayscale: true,
                flames: 100,
                width: 200,
                height: 100,
                hotspot: 255,
                rng_seed,
            } if rng_seed != 0
        ));
        let placement = pattern.placement.unwrap();
        assert!(placement.horizontal_symmetry);
        assert_eq!(placement.raster_rotation_degrees, 360.0);

        for (invalid, expected) in [
            (
                valid.replacen(r#"<PARAMS NB="8">"#, r#"<PARAMS NB="7">"#, 1),
                "PARAMS declares 7 entries but contains 8",
            ),
            (
                valid.replacen(r#"TYPE="0" ID="13""#, r#"TYPE="1" ID="13""#, 1),
                "expected PARAM 13 TYPE=0",
            ),
            (
                valid.replacen(r#"ID="10" VAL="100""#, r#"ID="10" VAL="0""#, 1),
                "integer within 1..100",
            ),
            (
                valid.replacen(r#"ID="11" VAL="200""#, r#"ID="11" VAL="201""#, 1),
                "integer within 10..200",
            ),
            (
                valid.replacen(r#"ID="12" VAL="100""#, r#"ID="12" VAL="0""#, 1),
                "integer within 1..100",
            ),
            (
                valid.replacen(r#"ID="13" VAL="255""#, r#"ID="13" VAL="9""#, 1),
                "integer within 10..255",
            ),
            (
                valid.replacen(r#"ID="3" VAL="2""#, r#"ID="3" VAL="3""#, 1),
                "Transform PARAM 3",
            ),
            (
                valid.replacen(r#"ID="4" VAL="360""#, r#"ID="4" VAL="361""#, 1),
                "integer within 0..360",
            ),
            (
                valid.replacen("<BEAMS NB=", "<SELECTIONS/><BEAMS NB=", 1),
                "external SELECTIONS remain fail-closed",
            ),
        ] {
            let error = convert(&invalid, 5, 3).unwrap_err();
            assert!(
                error.contains(expected),
                "expected {expected:?}, found {error:?}"
            );
        }
        for palette_count in [1, 5] {
            let error = convert(&source(palette_count, owned), 5, 3).unwrap_err();
            assert!(error.contains("palette requires 2..4"), "{error}");
        }
        for (rack_type, effect_type) in [(2, 3), (5, 2)] {
            let error = convert(&valid, rack_type, effect_type).unwrap_err();
            assert!(
                error.contains("is not confirmed for DVC-3b"),
                "wrong family must fail closed: {error}"
            );
        }
        let native_empty = source(4, r#"<BEAMS NB="0"/>"#).replacen(
            r#"SX="200" SY="100" ANGLE="30""#,
            r#"SX="-1" SY="-1" ANGLE="0""#,
            1,
        );
        let no_op = convert(&native_empty, 5, 3).unwrap();
        assert!(no_op.target.is_none());
        assert!(no_op.note.contains("source no-op preserved"));
    }

    #[test]
    fn dvc_color_mappings_explosion_starfield_import_strict_owned_particles_and_mutations() {
        let source = |generator_id: u16, palette_count: usize, shape: u8, beams: &str| {
            let color = r#"<COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/>"#;
            let (specific, nb) = match generator_id {
                47 => (
                    format!(
                        r#"<PARAM TYPE="7" ID="10" VAL="{shape}"/><PARAM TYPE="0" ID="11" VAL="5"/><PARAM TYPE="0" ID="12" VAL="5"/><PARAM TYPE="0" ID="13" VAL="10"/><PARAM TYPE="0" ID="14" VAL="10"/><PARAM TYPE="1" ID="15" VAL="0"/><PARAM TYPE="0" ID="16" VAL="10"/><PARAM TYPE="1" ID="17" VAL="0"/>"#
                    ),
                    12,
                ),
                48 => (
                    format!(
                        r#"<PARAM TYPE="7" ID="10" VAL="{shape}"/><PARAM TYPE="0" ID="11" VAL="1"/><PARAM TYPE="0" ID="12" VAL="10"/><PARAM TYPE="0" ID="13" VAL="10"/><PARAM TYPE="1" ID="14" VAL="0"/>"#
                    ),
                    9,
                ),
                _ => unreachable!(),
            };
            format!(
                r#"<SCENE DASUID="particle-scene" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="5" DASUID="particle-rack"><EFFECT TYPE="3" ID="{generator_id}" DASUID="particle-effect" DURATION="5000"><PARAMS NB="{nb}"><PARAM TYPE="4" ID="1"><COLORS NB="{palette_count}">{}</COLORS></PARAM><PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="4" VAL="0"/>{specific}</PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="particles" TYPE="0" X="0" Y="0" SX="100" SY="100" ANGLE="0" LOCKED="0"/>{beams}</RACK></RACKS></SCENE>"#,
                color.repeat(palette_count)
            )
        };
        let convert = |xml: &str, generator_id: u16| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "COLOR MAPPINGS particles",
                rack,
                effect,
                5,
                3,
                generator_id,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };
        let owned =
            r#"<BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS>"#;

        for (generator_id, maximum_palette) in [(47, 255), (48, 255)] {
            for palette_count in [2, maximum_palette] {
                let converted =
                    convert(&source(generator_id, palette_count, 0, owned), generator_id).unwrap();
                assert_eq!(
                    converted.generator,
                    if generator_id == 47 {
                        "Explosion"
                    } else {
                        "Starfield"
                    }
                );
                assert!(converted.approximations.is_empty());
                assert!(converted.note.contains("implementation=SyndocalCorrected"));
                assert!(converted.note.contains("children_before_parent=true"));
                assert!(converted.note.contains("random_pairs=5000"));
                let Some(EffectParamsSnapshot::Color(request)) =
                    converted.target.expect("populated particle mapping").params
                else {
                    panic!("particle mapping must create a Color request");
                };
                assert_eq!(request.period_ms, 5_000);
                assert_eq!(request.stops.len(), palette_count);
                let pattern = request.spatial_pattern.unwrap();
                assert_eq!(pattern.beam_targets.len(), 1);
                assert_eq!(
                    (
                        pattern.placement.as_ref().unwrap().sx,
                        pattern.placement.as_ref().unwrap().sy
                    ),
                    (100, 100)
                );
                match (generator_id, pattern.recipe) {
                    (
                        47,
                        ColorEffectSpatialRecipe::Explosion {
                            grayscale: true,
                            shape: 0,
                            explosion_number: 5,
                            explosion_size: 5,
                            particle_number: 10,
                            particle_size: 10,
                            particle_life: 0.0,
                            trail_size: 10,
                            gravity: 0.0,
                            rng_seed,
                        },
                    ) => assert_ne!(rng_seed, 0),
                    (
                        48,
                        ColorEffectSpatialRecipe::Starfield {
                            grayscale: true,
                            shape: 0,
                            particles: 1,
                            size: 10,
                            trail: 10,
                            rotation: 0.0,
                            rng_seed,
                        },
                    ) => assert_ne!(rng_seed, 0),
                    (_, recipe) => panic!("unexpected particle recipe: {recipe:?}"),
                }
            }
        }

        for generator_id in [47, 48] {
            for palette_count in [1, 256] {
                let error = convert(&source(generator_id, palette_count, 0, owned), generator_id)
                    .unwrap_err();
                let expected = if palette_count == 1 {
                    "palette requires 2..255"
                } else {
                    "palette requires 1..255"
                };
                assert!(error.contains(expected), "{error}");
            }
            let populated_shape =
                convert(&source(generator_id, 2, 1, owned), generator_id).unwrap_err();
            assert!(populated_shape.contains("populated Shape 1 is a proprietary glyph"));
            let shape_out_of_schema =
                convert(&source(generator_id, 2, 30, owned), generator_id).unwrap_err();
            assert!(shape_out_of_schema.contains("integer within 0..29"));

            let native_empty = source(generator_id, 2, 29, r#"<BEAMS NB="0"/>"#).replacen(
                r#"SX="100" SY="100""#,
                r#"SX="-1" SY="-1""#,
                1,
            );
            let no_op = convert(&native_empty, generator_id).unwrap();
            assert!(no_op.target.is_none());
            assert!(no_op.note.contains("source no-op preserved"));
            assert!(no_op.note.contains("Rectangle=(0,0,-1,-1,0)"));
            assert!(convert(
                &source(generator_id, 2, 0, owned).replacen(
                    r#"SX="100" SY="100""#,
                    r#"SX="-1" SY="-1""#,
                    1
                ),
                generator_id,
            )
            .unwrap_err()
            .contains("native empty Rectangle sentinel is valid only with zero owned BEAMS"));

            let valid = source(generator_id, 2, 0, owned);
            for (invalid, expected) in [
                (
                    valid.replacen(r#"TYPE="7" ID="10""#, r#"TYPE="0" ID="10""#, 1),
                    "expected PARAM 10 TYPE=7",
                ),
                (
                    valid.replacen("<BEAMS NB=", "<SELECTIONS/><BEAMS NB=", 1),
                    "external SELECTIONS remain fail-closed",
                ),
                (
                    valid.replacen(r#"<BEAMS NB="1">"#, r#"<BEAMS NB="2">"#, 1),
                    "BEAMS declares 2 entries but contains 1",
                ),
                (
                    valid.replacen("</RACK>", r#"<BEAMS NB="0"/></RACK>"#, 1),
                    "expected exactly one direct BEAMS container, found 2",
                ),
                (
                    valid.replacen("</PARAMS>", "<UNEXPECTED/></PARAMS>", 1),
                    "PARAMS contains unexpected <UNEXPECTED> element",
                ),
            ] {
                let error = convert(&invalid, generator_id).unwrap_err();
                assert!(
                    error.contains(expected),
                    "expected {expected:?}, found {error:?}"
                );
            }
        }

        let valid_explosion = source(47, 2, 0, owned);
        for (from, to, expected) in [
            (
                r#"ID="11" VAL="5""#,
                r#"ID="11" VAL="0""#,
                "integer within 1..50",
            ),
            (
                r#"ID="12" VAL="5""#,
                r#"ID="12" VAL="101""#,
                "integer within 0..100",
            ),
            (
                r#"ID="13" VAL="10""#,
                r#"ID="13" VAL="0""#,
                "integer within 1..100",
            ),
            (
                r#"ID="14" VAL="10""#,
                r#"ID="14" VAL="101""#,
                "integer within 1..100",
            ),
            (
                r#"ID="15" VAL="0""#,
                r#"ID="15" VAL="0.91""#,
                "within 0..0.9",
            ),
            (
                r#"ID="16" VAL="10""#,
                r#"ID="16" VAL="0""#,
                "integer within 1..25",
            ),
            (
                r#"ID="17" VAL="0""#,
                r#"ID="17" VAL="10.1""#,
                "within 0..10",
            ),
        ] {
            let error = convert(&valid_explosion.replacen(from, to, 1), 47).unwrap_err();
            assert!(
                error.contains(expected),
                "expected {expected:?}, found {error:?}"
            );
        }

        let valid_starfield = source(48, 2, 0, owned);
        for (from, to, expected) in [
            (
                r#"ID="11" VAL="1""#,
                r#"ID="11" VAL="11""#,
                "integer within 1..10",
            ),
            (
                r#"ID="12" VAL="10""#,
                r#"ID="12" VAL="0""#,
                "integer within 1..100",
            ),
            (
                r#"ID="13" VAL="10""#,
                r#"ID="13" VAL="26""#,
                "integer within 1..25",
            ),
            (r#"ID="14" VAL="0""#, r#"ID="14" VAL="5.1""#, "within -5..5"),
        ] {
            let error = convert(&valid_starfield.replacen(from, to, 1), 48).unwrap_err();
            assert!(
                error.contains(expected),
                "expected {expected:?}, found {error:?}"
            );
        }
    }

    #[test]
    fn dvc_color_mappings_media_empty_path_is_strict_noop_and_nonempty_fails_closed() {
        let source = |path: &str| {
            format!(
                r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="5"><EFFECT TYPE="3" ID="33" DURATION="1000"><PARAMS NB="6"><PARAM TYPE="4" ID="1"><COLORS NB="2"><COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM><PARAM TYPE="2" ID="2" VAL="1"/><PARAM TYPE="6" ID="3" VAL="2"/><PARAM TYPE="0" ID="4" VAL="90"/><PARAM TYPE="8" ID="10" VAL="{path}"/><PARAM TYPE="1" ID="11" VAL="0.5"/></PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="color-mappings-media" TYPE="0" X="0" Y="0" SX="200" SY="100" ANGLE="30" LOCKED="0"/><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS></RACK></RACKS></SCENE>"#
            )
        };
        let convert = |xml: &str| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "COLOR MAPPINGS Media",
                rack,
                effect,
                5,
                3,
                33,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };

        let empty = source("");
        let converted = convert(&empty).unwrap();
        assert!(converted.target.is_none());
        assert!(converted.approximations.is_empty());
        assert!(converted.note.contains("source_family=Color Mappings;"));
        assert!(converted.note.contains("palette_colors=Some(2)"));
        assert!(converted.note.contains("Grayscale=Some(1)"));
        assert!(converted.note.contains("Colorize=Some(0.5)"));
        assert!(converted.note.contains("MediaPath is empty"));
        assert!(converted.note.contains("timing=not-applicable"));

        let non_runtime_timing = empty
            .replacen(r#"SPEED="1""#, r#"SPEED="not-a-speed""#, 1)
            .replacen(r#"DURATION="1000""#, r#"DURATION="not-a-duration""#, 1);
        let timing_opaque_noop = convert(&non_runtime_timing).unwrap();
        assert!(timing_opaque_noop.target.is_none());
        assert!(timing_opaque_noop.approximations.is_empty());
        assert!(timing_opaque_noop.note.contains("timing=not-applicable"));

        assert!(convert(&source("embedded/image.png"))
            .unwrap_err()
            .contains("non-empty media paths"));
        assert!(
            convert(&empty.replacen(r#" ID="10" VAL="""#, r#" ID="10""#, 1))
                .unwrap_err()
                .contains("Media Path PARAM 10 is missing VAL")
        );
        assert!(
            convert(&empty.replacen(r#"TYPE="8" ID="10""#, r#"TYPE="2" ID="10""#, 1))
                .unwrap_err()
                .contains("expected PARAM 10 TYPE=8")
        );
        for invalid in [
            empty.replacen(r#"ID="2" VAL="1""#, r#"ID="2" VAL="2""#, 1),
            empty.replacen(r#"ID="3" VAL="2""#, r#"ID="3" VAL="3""#, 1),
            empty.replacen(r#"ID="4" VAL="90""#, r#"ID="4" VAL="90.5""#, 1),
            empty.replacen(r#"ID="11" VAL="0.5""#, r#"ID="11" VAL="1.1""#, 1),
        ] {
            assert!(convert(&invalid).is_err());
        }
        assert!(
            convert(&empty.replacen("<BEAMS NB=", "<SELECTIONS/><BEAMS NB=", 1))
                .unwrap_err()
                .contains("external SELECTIONS remain fail-closed")
        );
    }

    #[test]
    fn dvc_mappings_media_empty_path_preserves_legacy_non_runtime_timing_acceptance() {
        let source = |duration: &str, speed: &str| {
            format!(
                r#"<SCENE SPEED="{speed}" PLAY_TRIGGER="not-a-trigger" PLAY_DIVISION="0"><RACKS><RACK TYPE="6"><EFFECT TYPE="8" ID="526" DURATION="{duration}"><PARAMS NB="3"><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="4" VAL="0"/><PARAM TYPE="8" ID="10" VAL=""/></PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="mappings-media" TYPE="0" X="0" Y="0" SX="100" SY="100" ANGLE="0" LOCKED="0"/><BEAMS NB="0"/></RACK></RACKS></SCENE>"#
            )
        };
        let convert = |xml: &str| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "MAPPINGS Media timing",
                rack,
                effect,
                6,
                8,
                526,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };

        for (case, duration, speed) in [
            ("malformed", "not-a-duration", "not-a-speed"),
            ("zero and negative", "0", "-999"),
        ] {
            let converted = convert(&source(duration, speed)).unwrap();
            assert!(converted.target.is_none(), "{case}");
            assert!(converted.approximations.is_empty(), "{case}");
            assert!(converted.note.contains("source_family=Mappings;"), "{case}");
            assert!(converted.note.contains("timing=not-applicable"), "{case}");
        }
    }

    #[test]
    fn dvc_color_mappings_external_selections_and_unknown_family5_stay_fail_closed() {
        let document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="5"><EFFECT TYPE="3" ID="36" DURATION="1000"><PARAMS NB="7"><PARAM TYPE="4" ID="1"><COLORS NB="8"><COLOR VAL="1/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/1/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="1/1/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="1/0/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/1/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="1/1/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM><PARAM TYPE="2" ID="2" VAL="0"/><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="4" VAL="0"/><PARAM TYPE="1" ID="10" VAL="0"/><PARAM TYPE="0" ID="11" VAL="0"/><PARAM TYPE="1" ID="12" VAL="1"/></PARAMS></EFFECT><MAPPING NAME="Rectangle" DASUID="mapping-36" TYPE="0" X="0" Y="0" SX="100" SY="100" ANGLE="0" LOCKED="0"/><SELECTIONS/><BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = document.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let error = convert_dvc_effect(
            scene,
            "COLOR MAPPINGS selections",
            rack,
            effect,
            5,
            3,
            36,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap_err();
        assert!(error.contains("external SELECTIONS remain fail-closed"));

        let unknown = convert_dvc_effect(
            scene,
            "unknown family 5",
            rack,
            effect,
            5,
            3,
            999,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap_err();
        assert!(unknown.contains("RACK TYPE=5 EFFECT TYPE=3 ID=999 is not confirmed"));
    }

    #[test]
    fn dvc_chaser_322_preserves_empty_noop_and_converts_populated_build_clear_cycle() {
        let document = Document::parse(
            r#"<DLMFILE DASBUILD="test" VERSIONFILE="2"><SCENE NAME="SS-Blue" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="3"><EFFECT TYPE="6" ID="322" DURATION="5000"><PARAMS NB="1"><PARAM TYPE="2" ID="10" VAL="1"/></PARAMS></EFFECT><BEAMS NB="0"/></RACK></RACKS></SCENE></DLMFILE>"#,
        )
        .unwrap();
        let scene = document
            .descendants()
            .find(|node| node.has_tag_name("SCENE"))
            .unwrap();
        let mut report = DvcImportReport::new("synthetic-chaser-322.dvc", document.root_element());
        let mut next_effect_id = 1;
        let parsed = parse_scene_effects(
            scene,
            "SS-Blue",
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
            &mut next_effect_id,
            &mut report,
        );
        assert!(parsed.targets.is_empty());
        assert_eq!(report.summary.effects_converted, 1);
        assert_eq!(report.summary.effects_skipped, 0);
        assert_eq!(next_effect_id, 1);
        assert!(report.converted.details.iter().any(|detail| {
            detail.item == "Effect: SS-Blue (Chaser #2)"
                && detail.message.contains("source no-op preserved")
        }));
        assert!(parsed
            .notes
            .iter()
            .any(|note| note.contains("source no-op preserved")));

        let populated = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK TYPE="3"><EFFECT TYPE="6" ID="322" DURATION="5000"><PARAMS NB="1"><PARAM TYPE="2" ID="10" VAL="1"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-2" BEAMID="0" IDSELECTION="2"/></BEAMS></RACK></RACKS></SCENE>"#,
        )
        .unwrap();
        let scene = populated.root_element();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let converted = convert_dvc_effect(
            scene,
            "Future populated 322",
            rack,
            effect,
            3,
            6,
            322,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        let Some(EffectParamsSnapshot::Chaser(chaser)) = converted.target.unwrap().params else {
            panic!("CHASER 322 must be stored as cue-owned Chaser params");
        };
        assert_eq!(chaser.direction, ChaserDirection::BuildUpDown);
        assert_eq!(chaser.steps.len(), 2);
        assert_eq!(chaser.step_duration_ms, 1_250);
        assert_eq!(chaser.active_step_count, 1);
        assert_eq!(chaser.duty_cycle, 1.0);
        assert_eq!(chaser.overlap, 1.0);
        assert!(chaser.steps.iter().all(|step| step.fixture_ids.is_empty()));
        assert_eq!(chaser.steps[0].beam_targets[0].fixture_id, 1);
        assert_eq!(chaser.steps[0].beam_targets[0].selection_index, 0);
        assert_eq!(chaser.steps[1].beam_targets[0].fixture_id, 2);
        assert_eq!(chaser.steps[1].beam_targets[0].selection_index, 1);
        assert!(converted.approximations.is_empty());
        assert!(converted.note.contains("build-up then source-order clear"));
    }

    #[test]
    fn dvc_value_621_preserves_empty_noop_and_restores_targeted_feature_generator() {
        let source = r#"<DLMFILE DASBUILD="25.0905.165.111" VERSIONFILE="2"><SCENE NAME="SS-Blue" SPEED="0.5" PLAY_TRIGGER="0" PLAY_DIVISION="8"><RACKS><RACK TYPE="7"><EFFECT TYPE="7" ID="621" DURATION="5000"><PARAMS NB="5"><PARAM TYPE="4" ID="1"><COLORS NB="3"><COLOR VAL="1/1/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0.498039/0.498039/0.498039/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="1" ID="10" VAL="0"/><PARAM TYPE="0" ID="11" VAL="0"/><PARAM TYPE="1" ID="12" VAL="1"/></PARAMS></EFFECT><PRESETS><PRESET SSLFIXTURE="" SSLCHANNEL="-1" SSLPRESET="4" MIN="0" MAX="1"><BEAMS/></PRESET></PRESETS><BEAMS NB="0"/></RACK></RACKS></SCENE></DLMFILE>"#;
        let document = Document::parse(source).unwrap();
        let scene = document
            .descendants()
            .find(|node| node.has_tag_name("SCENE"))
            .unwrap();
        let mut report = DvcImportReport::new("value-621-noop.dvc", document.root_element());
        let mut next_effect_id = 1;
        let parsed = parse_scene_effects(
            scene,
            "SS-Blue",
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
            &mut next_effect_id,
            &mut report,
        );

        assert!(parsed.targets.is_empty());
        assert_eq!(report.summary.effects_converted, 1);
        assert_eq!(report.summary.effects_skipped, 0);
        assert_eq!(next_effect_id, 1);
        assert!(report.converted.details.iter().any(|detail| {
            detail.item == "Effect: SS-Blue (Rainbow)"
                && detail.message.contains("source_family=Value FX")
                && detail.message.contains("source no-op preserved")
                && detail.message.contains("palette_colors=3")
        }));

        let targeted = source.replacen(
            r#"<BEAMS NB="0"/>"#,
            r#"<BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS>"#,
            1,
        );
        let targeted = Document::parse(&targeted).unwrap();
        let scene = targeted
            .descendants()
            .find(|node| node.has_tag_name("SCENE"))
            .unwrap();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let converted = convert_dvc_effect(
            scene,
            "Targeted VALUE FX",
            rack,
            effect,
            7,
            7,
            621,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        let EffectParamsSnapshot::Value(value) = converted.target.unwrap().params.unwrap() else {
            panic!("targeted VALUE FX must retain its Value body");
        };
        assert_eq!(value.fixture_ids, vec![1]);
        assert_eq!(value.attribute, "Dimmer");
        assert_eq!(value.features.len(), 1);
        assert_eq!(value.features[0].low, 0);
        assert_eq!(value.features[0].high, u16::MAX);
        assert_eq!(value.points.len(), 3);
        assert_eq!(value.points[0].value, 1.0);
        assert_eq!(value.points[2].value, 0.0);
        let pattern = value.spatial_pattern.unwrap();
        assert!(matches!(
            pattern.recipe,
            ColorEffectSpatialRecipe::ColorRainbow {
                grayscale: false,
                vertical_symmetry: false,
                color_width: 0.0,
                angle_degrees: 0.0,
                gradient: 100.0,
            }
        ));
        assert_eq!(pattern.beam_targets.len(), 1);
        assert_eq!(pattern.beam_targets[0].fixture_id, 1);
        assert_eq!(pattern.beam_targets[0].beam_index, 0);
        assert_eq!(pattern.beam_targets[0].selection_index, 0);
        assert_eq!(
            pattern.beam_targets[0].feature_attribute.as_deref(),
            Some("Dimmer")
        );
        assert!(converted.approximations.is_empty());
        assert!(converted.note.contains("source_family=Value FX"));
        assert!(converted.note.contains("preset_type=4"));

        let malformed_source = source.replacen(r#"<BEAMS NB="0"/>"#, r#"<BEAMS NB="1"/>"#, 1);
        let malformed = Document::parse(&malformed_source).unwrap();
        let scene = malformed
            .descendants()
            .find(|node| node.has_tag_name("SCENE"))
            .unwrap();
        let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let error = convert_dvc_effect(
            scene,
            "Malformed VALUE FX",
            rack,
            effect,
            7,
            7,
            621,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap_err();
        assert!(error.contains("BEAMS declares 1 entries but contains 0"));
    }

    #[test]
    fn dvc_value_625_imports_static_serializer_shape_and_fails_closed_on_drift() {
        let source = r#"<DLMFILE DASBUILD="25.0905.165.111" VERSIONFILE="2"><SCENE NAME="Sweep" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="8"><RACKS><RACK TYPE="7"><EFFECT TYPE="7" ID="625" DURATION="3000"><PARAMS NB="3"><PARAM TYPE="4" ID="1"><COLORS NB="3"><COLOR VAL="1/1/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0.5/0.5/0.5/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="0/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="2" ID="10" VAL="1"/></PARAMS></EFFECT><PRESETS><PRESET SSLFIXTURE="" SSLCHANNEL="-1" SSLPRESET="4" MIN="0" MAX="1"><BEAMS/></PRESET></PRESETS><BEAMS NB="0"/></RACK></RACKS></SCENE></DLMFILE>"#;
        let document = Document::parse(source).unwrap();
        let scene = document
            .descendants()
            .find(|node| node.has_tag_name("SCENE"))
            .unwrap();
        let mut report = DvcImportReport::new("value-625-noop.dvc", document.root_element());
        let mut next_effect_id = 1;
        let parsed = parse_scene_effects(
            scene,
            "Sweep",
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
            &mut next_effect_id,
            &mut report,
        );
        assert!(parsed.targets.is_empty());
        assert_eq!(report.summary.effects_converted, 1);
        assert_eq!(report.summary.effects_skipped, 0);
        assert!(report.converted.details.iter().any(|detail| {
            detail.item == "Effect: Sweep (Sweep)"
                && detail.message.contains("PARAM IDs [3, 10]")
                && detail.message.contains("TYPEs validated")
        }));

        let convert_targeted = |xml: &str| {
            let document = Document::parse(xml).unwrap();
            let scene = document
                .descendants()
                .find(|node| node.has_tag_name("SCENE"))
                .unwrap();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "Targeted Sweep",
                rack,
                effect,
                7,
                7,
                625,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };
        let targeted = source.replacen(
            r#"<BEAMS NB="0"/>"#,
            r#"<BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS>"#,
            1,
        );
        let converted = convert_targeted(&targeted).unwrap();
        assert_eq!(converted.generator, "Sweep");
        assert!(converted.approximations.is_empty());
        let EffectParamsSnapshot::Value(value) = converted.target.unwrap().params.unwrap() else {
            panic!("targeted VALUE Sweep must retain its Value body");
        };
        assert_eq!(value.label, "Targeted Sweep (Sweep)");
        assert_eq!(value.points.len(), 3);
        let pattern = value.spatial_pattern.expect("Sweep spatial pattern");
        assert!(matches!(
            pattern.recipe,
            ColorEffectSpatialRecipe::Sweep {
                grayscale: false,
                vertical_symmetry: false,
                direction_change: true,
            }
        ));
        assert_eq!(pattern.beam_targets.len(), 1);

        let fixed_direction = targeted.replacen(
            r#"<PARAM TYPE="2" ID="10" VAL="1"/>"#,
            r#"<PARAM TYPE="2" ID="10" VAL="0"/>"#,
            1,
        );
        let converted = convert_targeted(&fixed_direction).unwrap();
        let EffectParamsSnapshot::Value(value) = converted.target.unwrap().params.unwrap() else {
            panic!("fixed-direction Sweep must retain its Value body");
        };
        assert!(matches!(
            value.spatial_pattern.unwrap().recipe,
            ColorEffectSpatialRecipe::Sweep {
                grayscale: false,
                vertical_symmetry: false,
                direction_change: false,
            }
        ));

        let wrong_type = targeted.replacen(
            r#"<PARAM TYPE="2" ID="10" VAL="1"/>"#,
            r#"<PARAM TYPE="1" ID="10" VAL="1"/>"#,
            1,
        );
        assert!(convert_targeted(&wrong_type)
            .unwrap_err()
            .contains("expected PARAM 10 TYPE=2, found TYPE=1"));

        let transformed = targeted.replacen(
            r#"<PARAM TYPE="6" ID="3" VAL="0"/>"#,
            r#"<PARAM TYPE="6" ID="3" VAL="1"/>"#,
            1,
        );
        let converted = convert_targeted(&transformed).unwrap();
        let EffectParamsSnapshot::Value(value) = converted.target.unwrap().params.unwrap() else {
            panic!("transformed Sweep must retain its Value body");
        };
        assert!(matches!(
            value.spatial_pattern.unwrap().recipe,
            ColorEffectSpatialRecipe::Sweep {
                grayscale: false,
                vertical_symmetry: true,
                direction_change: true,
            }
        ));

        let extra_param = targeted
            .replacen(r#"<PARAMS NB="3">"#, r#"<PARAMS NB="4">"#, 1)
            .replacen(
                "</PARAMS>",
                r#"<PARAM TYPE="2" ID="11" VAL="0"/></PARAMS>"#,
                1,
            );
        assert!(convert_targeted(&extra_param)
            .unwrap_err()
            .contains("expected PARAM IDs [3, 10], found [3, 10, 11]"));
    }

    #[test]
    fn dvc_value_623_plasma_imports_exact_schema_domains_and_one_row_transform() {
        let class_params = r#"<PARAM TYPE="0" ID="10" VAL="1"/><PARAM TYPE="0" ID="11" VAL="2"/><PARAM TYPE="0" ID="12" VAL="1"/><PARAM TYPE="0" ID="13" VAL="2"/><PARAM TYPE="0" ID="14" VAL="-1"/><PARAM TYPE="0" ID="15" VAL="2"/><PARAM TYPE="0" ID="16" VAL="1"/><PARAM TYPE="0" ID="17" VAL="-1"/>"#;

        for (transform, expected_symmetry) in [(0, false), (1, true)] {
            let source = value_fx_test_source(623, transform, 10, class_params, true);
            let converted = convert_value_fx_test_source(&source, 623).unwrap();
            assert_eq!(converted.generator, "Plasma");
            assert!(converted.approximations.is_empty());
            let EffectParamsSnapshot::Value(value) = converted.target.unwrap().params.unwrap()
            else {
                panic!("VALUE Plasma must retain its Value body");
            };
            let ColorEffectSpatialRecipe::Plasma {
                grayscale,
                vertical_symmetry,
                size_x,
                param_x,
                size_y,
                param_y,
                speed_x,
                param_sx,
                speed_y,
                param_sy,
            } = value.spatial_pattern.unwrap().recipe
            else {
                panic!("VALUE Plasma must use the Plasma recipe");
            };
            assert!(!grayscale);
            assert_eq!(vertical_symmetry, expected_symmetry);
            assert_eq!(
                (size_x, param_x, size_y, param_y, speed_x, param_sx, speed_y, param_sy,),
                (1.0, 2.0, 1.0, 2.0, -1.0, 2.0, 1.0, -1.0)
            );
        }

        let no_op = value_fx_test_source(623, 0, 10, class_params, false);
        let converted = convert_value_fx_test_source(&no_op, 623).unwrap();
        assert!(converted.target.is_none());
        assert!(converted.note.contains("source no-op preserved"));

        let targeted = value_fx_test_source(623, 0, 10, class_params, true);
        let wrong_type = targeted.replacen(
            r#"<PARAM TYPE="0" ID="17" VAL="-1"/>"#,
            r#"<PARAM TYPE="1" ID="17" VAL="-1"/>"#,
            1,
        );
        assert!(convert_value_fx_test_source(&wrong_type, 623)
            .unwrap_err()
            .contains("expected PARAM 17 TYPE=0, found TYPE=1"));

        let extra_param = targeted
            .replacen(r#"<PARAMS NB="10">"#, r#"<PARAMS NB="11">"#, 1)
            .replacen(
                "</PARAMS>",
                r#"<PARAM TYPE="0" ID="18" VAL="0"/></PARAMS>"#,
                1,
            );
        assert!(convert_value_fx_test_source(&extra_param, 623)
            .unwrap_err()
            .contains("expected PARAM IDs [3, 10, 11, 12, 13, 14, 15, 16, 17], found [3, 10, 11, 12, 13, 14, 15, 16, 17, 18]"));

        let out_of_range = targeted.replacen(
            r#"<PARAM TYPE="0" ID="14" VAL="-1"/>"#,
            r#"<PARAM TYPE="0" ID="14" VAL="-6"/>"#,
            1,
        );
        assert!(convert_value_fx_test_source(&out_of_range, 623)
            .unwrap_err()
            .contains(
                "VALUE FX Plasma Speed X PARAM 14 must be an integer within -5..5, found -6"
            ));

        let fractional = targeted.replacen(
            r#"<PARAM TYPE="0" ID="10" VAL="1"/>"#,
            r#"<PARAM TYPE="0" ID="10" VAL="1.5"/>"#,
            1,
        );
        assert!(convert_value_fx_test_source(&fractional, 623)
            .unwrap_err()
            .contains(
                "VALUE FX Plasma Size X PARAM 10 must be an integer within 0..20, found 1.5"
            ));

        let one_ulp_fractional = targeted.replacen(
            r#"<PARAM TYPE="0" ID="10" VAL="1"/>"#,
            r#"<PARAM TYPE="0" ID="10" VAL="1.0000000000000002"/>"#,
            1,
        );
        assert!(convert_value_fx_test_source(&one_ulp_fractional, 623)
            .unwrap_err()
            .contains("VALUE FX Plasma Size X PARAM 10 must be an integer within 0..20, found 1.0000000000000002"));

        let invalid_transform = value_fx_test_source(623, 2, 10, class_params, true);
        assert!(convert_value_fx_test_source(&invalid_transform, 623)
            .unwrap_err()
            .contains("VALUE FX Plasma Transform PARAM 3 must be 0 or 1, found 2"));
    }

    #[test]
    fn dvc_value_knight_rider_imports_exact_recipe_and_rejects_schema_drift() {
        let class_params = r#"<PARAM TYPE="0" ID="10" VAL="1"/><PARAM TYPE="2" ID="11" VAL="0"/><PARAM TYPE="2" ID="12" VAL="1"/><PARAM TYPE="2" ID="13" VAL="0"/><PARAM TYPE="0" ID="14" VAL="50"/>"#;

        for (transform, expected_symmetry) in [(0, false), (1, true)] {
            let source = value_fx_test_source(624, transform, 7, class_params, true);
            let converted = convert_value_fx_test_source(&source, 624).unwrap();
            assert!(converted.approximations.is_empty());
            assert!(converted.note.contains(
                "authored_duration_ms=3000; period_ms=max(authored_duration_ms,10)=3000; recovered_frame_count=max(1,floor(EFFECT DURATION / 40))=75"
            ));
            let EffectParamsSnapshot::Value(value) = converted.target.unwrap().params.unwrap()
            else {
                panic!("VALUE Knight Rider must retain its Value body");
            };
            let ColorEffectSpatialRecipe::KnightRider {
                grayscale,
                vertical_symmetry,
                size,
                one_way,
                fading,
                go_outside,
                gradient,
            } = value.spatial_pattern.unwrap().recipe
            else {
                panic!("VALUE ID 624 must use the KnightRider recipe");
            };
            assert!(!grayscale);
            assert_eq!(vertical_symmetry, expected_symmetry);
            assert_eq!(
                (size, one_way, fading, go_outside, gradient),
                (100.0, false, true, false, 50.0)
            );
        }

        let no_op = value_fx_test_source(624, 0, 7, class_params, false);
        let converted = convert_value_fx_test_source(&no_op, 624).unwrap();
        assert!(converted.target.is_none());
        assert!(converted.note.contains("source no-op preserved"));

        let invalid_transform = value_fx_test_source(624, 2, 7, class_params, true);
        assert!(convert_value_fx_test_source(&invalid_transform, 624)
            .unwrap_err()
            .contains("VALUE FX Knight Rider Transform PARAM 3 must be 0 or 1, found 2"));

        let source = value_fx_test_source(624, 0, 7, class_params, true);
        let wrong_type = source.replacen(
            r#"<PARAM TYPE="0" ID="10""#,
            r#"<PARAM TYPE="1" ID="10""#,
            1,
        );
        assert!(convert_value_fx_test_source(&wrong_type, 624)
            .unwrap_err()
            .contains("expected PARAM 10 TYPE=0, found TYPE=1"));

        let extra_param = source
            .replacen(r#"<PARAMS NB="7">"#, r#"<PARAMS NB="8">"#, 1)
            .replacen(
                "</PARAMS>",
                r#"<PARAM TYPE="0" ID="99" VAL="0"/></PARAMS>"#,
                1,
            );
        let extra_error = convert_value_fx_test_source(&extra_param, 624).unwrap_err();
        assert!(extra_error.contains("expected PARAM IDs"));
        assert!(extra_error.contains("99"));

        let out_of_range = source.replacen(
            r#"<PARAM TYPE="0" ID="10" VAL="1"/>"#,
            r#"<PARAM TYPE="0" ID="10" VAL="101"/>"#,
            1,
        );
        assert!(convert_value_fx_test_source(&out_of_range, 624)
            .unwrap_err()
            .contains(
                "VALUE FX Knight Rider Size PARAM 10 must be an integer within 1..100, found 101"
            ));
    }

    #[test]
    fn dvc_value_burst_imports_exact_recipe_and_rejects_schema_drift() {
        let class_params = r#"<PARAM TYPE="0" ID="10" VAL="50"/><PARAM TYPE="1" ID="11" VAL="1"/>"#;
        for transform in [0, 1] {
            let source = value_fx_test_source(622, transform, 4, class_params, true);
            let converted = convert_value_fx_test_source(&source, 622).unwrap();
            assert!(converted.approximations.is_empty());
            assert!(converted.note.contains(
                "authored_duration_ms=3000; period_ms=max(authored_duration_ms,10)=3000; recovered_frame_count=max(1,floor(EFFECT DURATION / 40))=75"
            ));
            let EffectParamsSnapshot::Value(value) = converted.target.unwrap().params.unwrap()
            else {
                panic!("VALUE Burst must retain its Value body");
            };
            assert_eq!(value.period_ms, 3_000);
            assert!(matches!(
                value.spatial_pattern.unwrap().recipe,
                ColorEffectSpatialRecipe::Burst {
                    grayscale: false,
                    vertical_symmetry,
                    color_width: 5000.0,
                    gradient: 100.0,
                } if vertical_symmetry == (transform == 1)
            ));
        }

        let invalid_transform = value_fx_test_source(622, 2, 4, class_params, true);
        assert!(convert_value_fx_test_source(&invalid_transform, 622)
            .unwrap_err()
            .contains("VALUE FX Burst Transform PARAM 3 must be 0 or 1, found 2"));
        let source = value_fx_test_source(622, 0, 4, class_params, true);
        assert!(convert_value_fx_test_source(
            &source.replacen(
                r#"<PARAM TYPE="0" ID="10" VAL="50"/>"#,
                r#"<PARAM TYPE="1" ID="10" VAL="50"/>"#,
                1,
            ),
            622,
        )
        .unwrap_err()
        .contains("expected PARAM 10 TYPE=0, found TYPE=1"));
        assert!(convert_value_fx_test_source(
            &source.replacen(
                r#"<PARAM TYPE="0" ID="10" VAL="50"/>"#,
                r#"<PARAM TYPE="0" ID="10" VAL="901"/>"#,
                1,
            ),
            622,
        )
        .unwrap_err()
        .contains(
            "VALUE FX Burst Color Width PARAM 10 must be an integer within 10..900, found 901"
        ));
    }

    #[test]
    fn dvc_value_random_generators_use_corrected_deterministic_recipes() {
        let cases = [
            (
                626,
                "Sparkles",
                5,
                r#"<PARAM TYPE="0" ID="10" VAL="5"/><PARAM TYPE="1" ID="11" VAL="0"/><PARAM TYPE="0" ID="12" VAL="1"/>"#,
                r#"<PARAM TYPE="1" ID="11" VAL="0"/>"#,
                r#"<PARAM TYPE="1" ID="11" VAL="1"/>"#,
                "VALUE FX Sparkles LifeSpan PARAM 11 must be within 0..0.9, found 1",
            ),
            (
                627,
                "Random fill",
                4,
                r#"<PARAM TYPE="0" ID="10" VAL="1"/><PARAM TYPE="0" ID="11" VAL="1"/>"#,
                r#"<PARAM TYPE="0" ID="11" VAL="1"/>"#,
                r#"<PARAM TYPE="0" ID="11" VAL="11"/>"#,
                "VALUE FX Random fill Point Height PARAM 11 must be an integer within 1..10, found 11",
            ),
        ];

        for (
            generator_id,
            generator,
            declared_params,
            class_params,
            valid_range_param,
            invalid_range_param,
            range_error,
        ) in cases
        {
            for transform in [0, 1] {
                let source = value_fx_test_source(
                    generator_id,
                    transform,
                    declared_params,
                    class_params,
                    true,
                );
                let first = convert_value_fx_test_source(&source, generator_id).unwrap();
                let second = convert_value_fx_test_source(&source, generator_id).unwrap();
                assert!(first.approximations.is_empty());
                assert!(first.note.contains("implementation=SyndocalCorrected"));
                assert!(first.note.contains("stable_source_seed"));
                if generator_id == 626 {
                    for contract in [
                        "population=retained",
                        "fade=continuous",
                        "time_units=milliseconds",
                    ] {
                        assert!(
                            first.note.contains(contract),
                            "missing {contract}: {}",
                            first.note
                        );
                    }
                } else {
                    for contract in [
                        "cell_partition=div_ceil",
                        "end_coverage=partial_tail_included",
                        "transition=continuous_palette_to_palette",
                    ] {
                        assert!(
                            first.note.contains(contract),
                            "missing {contract}: {}",
                            first.note
                        );
                    }
                }
                let EffectParamsSnapshot::Value(value) = first.target.unwrap().params.unwrap()
                else {
                    panic!("VALUE random generator must retain its Value body");
                };
                let EffectParamsSnapshot::Value(repeated) = second.target.unwrap().params.unwrap()
                else {
                    panic!("repeated VALUE random generator must retain its Value body");
                };
                assert_eq!(
                    serde_json::to_value(&value.spatial_pattern).unwrap(),
                    serde_json::to_value(&repeated.spatial_pattern).unwrap(),
                    "source identity must produce a reload-stable seed"
                );
                match value.spatial_pattern.unwrap().recipe {
                    ColorEffectSpatialRecipe::Sparkle {
                        vertical_symmetry,
                        number: 5,
                        lifetime_ms: Some(100),
                        source_lifespan: Some(0.0),
                        width: 100.0,
                        rng_seed,
                        ..
                    } if generator_id == 626 => {
                        assert_eq!(vertical_symmetry, transform == 1);
                        assert_ne!(rng_seed, 0);
                    }
                    ColorEffectSpatialRecipe::RandomFill {
                        vertical_symmetry,
                        point_width: 100.0,
                        source_point_height: Some(1),
                        rng_seed,
                        ..
                    } if generator_id == 627 => {
                        assert_eq!(vertical_symmetry, transform == 1);
                        assert_ne!(rng_seed, 0);
                    }
                    recipe => panic!("unexpected corrected VALUE recipe: {recipe:?}"),
                }
            }

            let invalid_transform =
                value_fx_test_source(generator_id, 2, declared_params, class_params, true);
            assert!(
                convert_value_fx_test_source(&invalid_transform, generator_id)
                    .unwrap_err()
                    .contains(&format!(
                        "VALUE FX {generator} Transform PARAM 3 must be 0 or 1, found 2"
                    ))
            );

            let source = value_fx_test_source(generator_id, 0, declared_params, class_params, true);
            let wrong_type = source.replacen(
                r#"<PARAM TYPE="0" ID="10""#,
                r#"<PARAM TYPE="1" ID="10""#,
                1,
            );
            assert!(convert_value_fx_test_source(&wrong_type, generator_id)
                .unwrap_err()
                .contains("expected PARAM 10 TYPE=0, found TYPE=1"));

            let extra_param = source
                .replacen(
                    &format!(r#"<PARAMS NB="{declared_params}">"#),
                    &format!(r#"<PARAMS NB="{}">"#, declared_params + 1),
                    1,
                )
                .replacen(
                    "</PARAMS>",
                    r#"<PARAM TYPE="0" ID="99" VAL="0"/></PARAMS>"#,
                    1,
                );
            let extra_error = convert_value_fx_test_source(&extra_param, generator_id).unwrap_err();
            assert!(extra_error.contains("expected PARAM IDs"));
            assert!(extra_error.contains("99"));

            let invalid = source.replacen(valid_range_param, invalid_range_param, 1);
            assert!(convert_value_fx_test_source(&invalid, generator_id)
                .unwrap_err()
                .contains(range_error));

            let no_op_source =
                value_fx_test_source(generator_id, 0, declared_params, class_params, false);
            let no_op = convert_value_fx_test_source(&no_op_source, generator_id).unwrap();
            assert!(no_op.target.is_none());
            assert!(no_op.note.contains("source no-op preserved"));
            assert!(no_op.note.contains("authored_duration_ms=3000"));
            assert!(no_op.note.contains(if generator_id == 626 {
                "population=retained"
            } else {
                "cell_partition=div_ceil"
            }));

            let invalid_no_op = no_op_source.replacen(valid_range_param, invalid_range_param, 1);
            assert!(convert_value_fx_test_source(&invalid_no_op, generator_id)
                .unwrap_err()
                .contains(range_error));
            let invalid_duration =
                no_op_source.replacen(r#"DURATION="3000""#, r#"DURATION="0""#, 1);
            assert!(
                convert_value_fx_test_source(&invalid_duration, generator_id)
                    .unwrap_err()
                    .contains("DURATION must be greater than 0")
            );
            let one_stop = no_op_source
                .replacen(r#"<COLORS NB="2">"#, r#"<COLORS NB="1">"#, 1)
                .replacen(
                    r#"<COLOR VAL="0/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/>"#,
                    "",
                    1,
                );
            assert!(convert_value_fx_test_source(&one_stop, generator_id)
                .unwrap_err()
                .contains("VALUE FX value palette requires 2..32 stops, found 1"));
        }
    }

    #[test]
    fn dvc_value_perlin_imports_corrected_recipe_and_rejects_schema_drift() {
        let class_params = r#"<PARAM TYPE="0" ID="10" VAL="4"/><PARAM TYPE="0" ID="11" VAL="75"/><PARAM TYPE="0" ID="12" VAL="2"/><PARAM TYPE="0" ID="13" VAL="1"/><PARAM TYPE="0" ID="14" VAL="70"/>"#;
        for transform in [0, 1] {
            let source = value_fx_test_source(628, transform, 7, class_params, true);
            let converted = convert_value_fx_test_source(&source, 628).unwrap();
            assert!(converted.approximations.is_empty());
            assert!(converted.note.contains(
                "evaluator=CPerlinEffect corrected continuous lattice hash/cosine interpolation"
            ));
            assert!(converted
                .note
                .contains("Direction is activated as spatial phase"));
            let EffectParamsSnapshot::Value(value) = converted.target.unwrap().params.unwrap()
            else {
                panic!("VALUE Perlin must retain its Value body");
            };
            assert_eq!(value.period_ms, 3_000);
            assert!(matches!(
                value.spatial_pattern.unwrap().recipe,
                ColorEffectSpatialRecipe::Perlin {
                    grayscale: false,
                    vertical_symmetry,
                    horizontal_symmetry: false,
                    rotation_degrees: 0.0,
                    octaves: 3,
                    zoom: 75.0,
                    direction_degrees,
                    speed: 1.0,
                    amplitude: 70.0,
                } if vertical_symmetry == (transform == 1)
                    && (direction_degrees - 360.0 / 99.0).abs() <= f32::EPSILON
            ));
        }

        let invalid_transform = value_fx_test_source(628, 2, 7, class_params, true);
        assert!(convert_value_fx_test_source(&invalid_transform, 628)
            .unwrap_err()
            .contains("VALUE FX Perlin Transform PARAM 3 must be 0 or 1, found 2"));
        let source = value_fx_test_source(628, 0, 7, class_params, true);
        assert!(convert_value_fx_test_source(
            &source.replacen(
                r#"<PARAM TYPE="0" ID="10" VAL="4"/>"#,
                r#"<PARAM TYPE="1" ID="10" VAL="4"/>"#,
                1,
            ),
            628,
        )
        .unwrap_err()
        .contains("expected PARAM 10 TYPE=0, found TYPE=1"));
        assert!(convert_value_fx_test_source(
            &source.replacen(
                r#"<PARAM TYPE="0" ID="14" VAL="70"/>"#,
                r#"<PARAM TYPE="0" ID="14" VAL="101"/>"#,
                1,
            ),
            628,
        )
        .unwrap_err()
        .contains(
            "VALUE FX Perlin Amplitude PARAM 14 must be an integer within 5..100, found 101"
        ));
    }

    #[test]
    fn dvc_corrected_value_duration_preserves_authored_time_with_common_minimum() {
        let bases = [
            (
                624,
                value_fx_test_source(
                    624,
                    0,
                    7,
                    r#"<PARAM TYPE="0" ID="10" VAL="1"/><PARAM TYPE="2" ID="11" VAL="0"/><PARAM TYPE="2" ID="12" VAL="1"/><PARAM TYPE="2" ID="13" VAL="0"/><PARAM TYPE="0" ID="14" VAL="50"/>"#,
                    true,
                ),
            ),
            (
                625,
                value_fx_test_source(625, 0, 3, r#"<PARAM TYPE="2" ID="10" VAL="1"/>"#, true),
            ),
        ];

        for (generator_id, base) in bases {
            for (duration, expected_period, expected_frames) in [
                ("1", 10, 1),
                ("9", 10, 1),
                ("10", 10, 1),
                ("39", 39, 1),
                ("40", 40, 1),
                ("41", 41, 1),
                ("79", 79, 1),
                ("80", 80, 2),
            ] {
                let source =
                    base.replacen("DURATION=\"3000\"", &format!("DURATION=\"{duration}\""), 1);
                let converted = convert_value_fx_test_source(&source, generator_id).unwrap();
                assert!(converted.note.contains(&format!(
                    "authored_duration_ms={duration}; period_ms=max(authored_duration_ms,10)={expected_period}; recovered_frame_count=max(1,floor(EFFECT DURATION / 40))={expected_frames}"
                )));
                let EffectParamsSnapshot::Value(value) = converted.target.unwrap().params.unwrap()
                else {
                    panic!("exact VALUE generator must retain its Value body");
                };
                assert_eq!(value.period_ms, expected_period);
            }

            for (duration, error) in [
                ("0", "must be greater than 0"),
                ("-1", "must be greater than 0"),
                ("1.5", "must be a positive signed 32-bit integer"),
                ("2147483648", "must be a positive signed 32-bit integer"),
            ] {
                let source =
                    base.replacen("DURATION=\"3000\"", &format!("DURATION=\"{duration}\""), 1);
                assert!(convert_value_fx_test_source(&source, generator_id)
                    .unwrap_err()
                    .contains(error));
            }
        }
    }

    #[test]
    fn dvc_cue_owned_fx_recall_activates_and_release_stops_output() {
        let outcome = import_bytes(synthetic_fx_dvc().as_bytes(), "synthetic-fx.dvc").unwrap();
        let cue_id = outcome.project.snapshot.cues[0].id;
        let mut snapshot_to_load = outcome.project.snapshot;
        snapshot_to_load.output.enabled = false;
        for output in &mut snapshot_to_load.dmx_outputs {
            output.enabled = false;
        }
        let engine = engine::EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine
            .send(engine::EngineCommand::TriggerCue(cue_id))
            .unwrap();

        let mut active_nonzero = false;
        for _ in 0..40 {
            let snapshot = engine.snapshot();
            active_nonzero = snapshot.active_cue_id == Some(cue_id)
                && snapshot.dmx_preview.iter().any(|value| *value != 0);
            if active_nonzero {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(active_nonzero, "cue recall must activate its owned Chaser");

        engine
            .send(engine::EngineCommand::ReleaseCue(cue_id))
            .unwrap();
        let mut released = false;
        for _ in 0..40 {
            let snapshot = engine.snapshot();
            released = snapshot.active_cue_id.is_none()
                && snapshot.dmx_preview.iter().all(|value| *value == 0);
            if released {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(released, "cue release must stop its owned Chaser output");
    }

    #[test]
    fn dvc_synthetic_direct_super_scene_trigger_changes_rendered_dmx_across_blocks() {
        let outcome = import_bytes(synthetic_dvc().as_bytes(), "synthetic-direct-super.dvc")
            .expect("synthetic Daslight Super Scene must import");
        let mut snapshot_to_load = outcome.project.snapshot;
        let parent_index = snapshot_to_load
            .cues
            .iter()
            .position(|cue| cue.child_timeline.is_some())
            .expect("synthetic import must contain a Super Scene cue");
        let parent_cue_id = snapshot_to_load.cues[parent_index].id;
        let parent_group_id = snapshot_to_load.cues[parent_index]
            .group_id
            .clone()
            .expect("synthetic Super Scene must retain its imported bank");
        let first_cue_id = snapshot_to_load.cues[parent_index]
            .child_timeline
            .as_ref()
            .and_then(|child| child.events.first())
            .map(|event| event.cue_id)
            .expect("synthetic Super Scene must contain a child Scene Block");
        let mut second_cue = snapshot_to_load
            .cues
            .iter()
            .find(|cue| cue.id == first_cue_id)
            .cloned()
            .expect("synthetic child Scene Block cue must exist");
        second_cue.id = snapshot_to_load
            .cues
            .iter()
            .map(|cue| cue.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        second_cue.label = "Synthetic child low".to_string();
        second_cue.targets[0].values[0].value = 16_384;
        let second_cue_id = second_cue.id;
        snapshot_to_load.cues.push(second_cue);

        let child = snapshot_to_load.cues[parent_index]
            .child_timeline
            .as_mut()
            .unwrap();
        let mut first_event = child.events[0].clone();
        first_event.time_ms = 100;
        first_event.time_beats = None;
        first_event.duration_ms = 250;
        first_event.duration_beats = None;
        first_event.conform_to_tempo = false;
        first_event.loop_fill = false;
        first_event.loop_count = 1;
        first_event.source_offset_ms = 0;
        let mut second_event = first_event.clone();
        second_event.id = second_event.id.saturating_add(1);
        second_event.cue_id = second_cue_id;
        second_event.time_ms = 400;
        child.events = vec![first_event, second_event];
        child.duration_ms = 650;
        child.audio = None;
        child.audio_clips.clear();

        snapshot_to_load.output.enabled = false;
        for output in &mut snapshot_to_load.dmx_outputs {
            output.enabled = false;
        }
        let engine = engine::EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine
            .send(engine::EngineCommand::TriggerCue(parent_cue_id))
            .unwrap();

        let mut first_value = None;
        for _ in 0..50 {
            let value = engine.snapshot().dmx_preview[0];
            if value == u8::MAX {
                first_value = Some(value);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let first_value = first_value.expect("the first direct child block must render full DMX");

        let mut second_value = None;
        for _ in 0..60 {
            let value = engine.snapshot().dmx_preview[0];
            if value == 64 {
                second_value = Some(value);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let second_value = second_value.expect("the later direct child block must replace DMX");
        assert_ne!(first_value, second_value);
        let rendered = engine.snapshot();
        assert_eq!(rendered.active_cue_id, Some(parent_cue_id));
        assert_eq!(
            rendered.active_group_cue_ids.get(&parent_group_id),
            Some(&parent_cue_id)
        );
    }

    #[test]
    fn dvc_nested_super_scene_reference_imports_and_reaches_leaf_dmx() {
        let outcome = import_bytes(
            synthetic_nested_dvc().as_bytes(),
            "synthetic-nested-super.dvc",
        )
        .expect("nested Daslight Super Scene must import");
        crate::validate_project_file(&outcome.project).unwrap();
        assert!(!outcome
            .report
            .skipped
            .details
            .iter()
            .any(|detail| detail.message.contains("Nested Timeline reference")));

        let mut snapshot = outcome.project.snapshot;
        let nested = snapshot
            .cues
            .iter()
            .find(|cue| cue.label == "Nested Super")
            .expect("nested owner cue");
        let nested_cue_id = nested.id;
        let referenced_cue_id = nested
            .child_timeline
            .as_ref()
            .and_then(|child| child.events.first())
            .map(|event| event.cue_id)
            .expect("nested owner block");
        assert!(snapshot
            .cues
            .iter()
            .find(|cue| cue.id == referenced_cue_id)
            .and_then(|cue| cue.child_timeline.as_ref())
            .is_some());

        snapshot.output.enabled = false;
        for output in &mut snapshot.dmx_outputs {
            output.enabled = false;
        }
        let engine = engine::EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot).unwrap();
        engine
            .send(engine::EngineCommand::TriggerCue(nested_cue_id))
            .unwrap();
        let mut rendered_leaf = false;
        for _ in 0..50 {
            if engine.snapshot().dmx_preview[0] == u8::MAX {
                rendered_leaf = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            rendered_leaf,
            "nested Super Scene leaf must render full DMX"
        );
    }

    #[test]
    fn dvc_local_golden_super_scene_direct_trigger_plays_child_timeline() {
        // Supervisor acceptance for direct Super Scene playback (#48): the real
        // show's Super Scene cue, triggered directly (matrix/GO path, not via a
        // main-timeline event), must execute its child timeline blocks.
        let path = Path::new(r"C:\Users\kouty\Documents\Daslight 5\Projects\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        let parent = outcome
            .project
            .snapshot
            .cues
            .iter()
            .find(|cue| {
                cue.child_timeline
                    .as_ref()
                    .is_some_and(|child| !child.events.is_empty())
            })
            .expect("golden project should contain a Super Scene cue with child blocks");
        let parent_cue_id = parent.id;
        let block_count = parent.child_timeline.as_ref().unwrap().events.len();
        assert!(
            block_count >= 2,
            "golden Super Scene should hold multiple child blocks (found {block_count})"
        );

        let mut snapshot_to_load = outcome.project.snapshot.clone();
        snapshot_to_load.output.enabled = false;
        for output in &mut snapshot_to_load.dmx_outputs {
            output.enabled = false;
        }
        let engine = engine::EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine
            .send(engine::EngineCommand::TriggerCue(parent_cue_id))
            .unwrap();

        // The first real blocks land a few seconds in; sample up to 12s and
        // require at least two distinct non-zero preview states (blocks
        // actually replacing each other), which the pre-#48 engine never
        // produced (30s of all-zero).
        let mut activated = false;
        for _ in 0..40 {
            if engine.snapshot().active_cue_id == Some(parent_cue_id) {
                activated = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(
            activated,
            "triggering the Super Scene should make it active"
        );

        let mut distinct_nonzero: Vec<Vec<u8>> = Vec::new();
        for _ in 0..120 {
            let snapshot = engine.snapshot();
            assert_eq!(
                snapshot.active_cue_id,
                Some(parent_cue_id),
                "the Super Scene must stay active while its child timeline plays"
            );
            let preview = snapshot.dmx_preview;
            if preview.iter().any(|value| *value != 0) && !distinct_nonzero.contains(&preview) {
                distinct_nonzero.push(preview);
                if distinct_nonzero.len() >= 2 {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(
            distinct_nonzero.len() >= 2,
            "direct Super Scene trigger should render at least two distinct non-zero \
             DMX states from its child blocks (saw {})",
            distinct_nonzero.len()
        );

        engine
            .send(engine::EngineCommand::ReleaseCue(parent_cue_id))
            .unwrap();
        let mut released = false;
        for _ in 0..40 {
            let snapshot = engine.snapshot();
            if snapshot.active_cue_id.is_none()
                && snapshot.dmx_preview.iter().all(|value| *value == 0)
            {
                released = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(
            released,
            "releasing the Super Scene should stop child playback and restore zero output"
        );
    }

    #[test]
    fn dvc_local_golden_shin_seek_matches_daslight_at_ten_seconds() {
        let path = Path::new(r"C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        let shin = outcome
            .project
            .snapshot
            .cues
            .iter()
            .find(|cue| cue.label == "Shin" && cue.child_timeline.is_some())
            .expect("golden project should contain the Shin Timeline");
        let cue_id = shin.id;
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let artnet_port = receiver.local_addr().unwrap().port();
        let mut snapshot_to_load = outcome.project.snapshot;
        snapshot_to_load.output = DmxOutputConfig {
            enabled: true,
            protocol: protocol::DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port: artnet_port,
            universe: 0,
            ..DmxOutputConfig::default()
        };
        snapshot_to_load.dmx_outputs = vec![snapshot_to_load.output.clone()];
        let engine = engine::EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine
            .send(engine::EngineCommand::TriggerCue(cue_id))
            .unwrap();
        engine
            .send(engine::EngineCommand::SetDirectChildTimelinePlaying {
                cue_id,
                playing: false,
            })
            .unwrap();
        engine
            .send(engine::EngineCommand::SeekDirectChildTimeline {
                cue_id,
                position_ms: 10_000,
            })
            .unwrap();

        let mut seeked = None;
        for _ in 0..100 {
            let snapshot = engine.snapshot();
            if snapshot
                .direct_child_timeline_transports
                .iter()
                .any(|transport| {
                    transport.cue_id == cue_id
                        && transport.position_ms == 10_000
                        && !transport.playing
                })
            {
                seeked = Some(snapshot.dmx_preview);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let frame = seeked.expect("Shin seek should publish a stable paused frame");
        let mut daslight_at_ten = vec![0_u8; 512];
        for (channel, value) in [
            (6, 255),
            (7, 135),
            (8, 25),
            (10, 255),
            (11, 255),
            (12, 135),
            (13, 25),
            (15, 255),
            (16, 255),
            (17, 135),
            (18, 25),
            (20, 255),
            (21, 255),
            (22, 135),
            (23, 25),
            (25, 255),
            (26, 255),
            (27, 135),
            (28, 25),
            (30, 255),
            (31, 255),
            (32, 135),
            (33, 25),
            (35, 255),
            (41, 255),
            (42, 135),
            (43, 25),
            (45, 255),
            (54, 255),
            (55, 135),
            (56, 25),
            (60, 255),
            (61, 255),
            (62, 255),
            (63, 135),
            (64, 25),
            (68, 255),
            (69, 255),
            (70, 255),
            (71, 135),
            (72, 25),
            (74, 255),
            (76, 255),
            (77, 135),
            (78, 25),
            (80, 255),
            (82, 255),
            (83, 135),
            (84, 25),
            (86, 255),
            (87, 255),
            (88, 135),
            (89, 25),
            (91, 255),
            (92, 255),
            (93, 135),
            (94, 25),
            (96, 255),
            (97, 255),
            (98, 135),
            (99, 25),
            (101, 255),
            (109, 255),
            (512, 255),
        ] {
            daslight_at_ten[channel - 1] = value;
        }
        assert_eq!(frame, daslight_at_ten, "Daslight DMX Levels at 10.000 s");

        let mut buffer = [0_u8; 600];
        let packet = loop {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = io::artnet::parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.universe == 0 && packet.data.get(108) == Some(&255) {
                break packet;
            }
        };
        assert_eq!(
            packet.data, frame,
            "Art-Net must carry the paused seek frame"
        );

        engine
            .send(engine::EngineCommand::SeekDirectChildTimeline {
                cue_id,
                position_ms: 20_000,
            })
            .unwrap();
        let mut frame_at_twenty = None;
        for _ in 0..100 {
            let snapshot = engine.snapshot();
            if snapshot
                .direct_child_timeline_transports
                .iter()
                .any(|transport| {
                    transport.cue_id == cue_id
                        && transport.position_ms == 20_000
                        && !transport.playing
                })
            {
                frame_at_twenty = Some(snapshot.dmx_preview);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let frame_at_twenty =
            frame_at_twenty.expect("Shin 20-second seek should publish a stable paused frame");
        // Daslight's on-screen DMX Levels cells are refreshed independently,
        // so a screenshot is not packet-atomic while an FX generator is live.
        // Keep the observed frame as a bounded visual-parity check while the
        // Chaser values below remain exact packet assertions.
        let daslight_levels_capture = [
            255, 176, 116, 0, 255, 255, 180, 125, 0, 255, 255, 190, 143, 0, 255, 255, 180, 130, 0,
            255, 255, 110, 0, 0, 255, 255, 128, 9, 0, 255, 255, 128, 8, 0, 255, 255, 113, 8, 0,
            255, 255, 139, 80, 0, 255, 255, 133, 72, 0, 0, 0, 0, 255,
        ];
        let capture_absolute_error = frame_at_twenty[..daslight_levels_capture.len()]
            .iter()
            .zip(daslight_levels_capture)
            .map(|(actual, observed)| actual.abs_diff(observed) as u64)
            .sum::<u64>();
        // The corrected COLOR 127 route now evaluates its duration-independent
        // analytic geometry at the exact authored phase instead of replaying
        // the recovered 40 ms frame table. Against this non-atomic UI capture,
        // the deterministic analytic frame differs by 1,168 across 53 cells;
        // retain a tight 1,200 bound while the packet-atomic Chaser assertions
        // below stay exact.
        assert!(
            capture_absolute_error <= 1_200,
            "20.000 s Par-OrangeStrobe visual capture error {capture_absolute_error} exceeded the packet-skew allowance; actual={:?}",
            &frame_at_twenty[..daslight_levels_capture.len()]
        );
        assert_eq!(
            [
                frame_at_twenty[85],
                frame_at_twenty[90],
                frame_at_twenty[95],
                frame_at_twenty[100],
            ],
            [0, 170, 85, 0],
            "Daslight Chaser dimmers at 20.000 s"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
        let paused_preview = engine.snapshot().dmx_preview;
        assert_eq!(
            [
                paused_preview[85],
                paused_preview[90],
                paused_preview[95],
                paused_preview[100],
            ],
            [0, 170, 85, 0],
            "paused Timeline FX must stay on the exact source frame"
        );
    }

    #[test]
    fn dvc_local_golden_project_triggers_cue_and_renders_dmx() {
        let path = Path::new(r"C:\Users\kouty\Documents\Daslight 5\Projects\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        let cue_id = outcome
            .project
            .snapshot
            .cues
            .iter()
            .find(|cue| {
                cue.targets
                    .iter()
                    .any(|target| target.values.iter().any(|value| value.value > 0))
            })
            .expect("golden project should contain a cue with non-zero imported values")
            .id;

        let mut snapshot_to_load = outcome.project.snapshot.clone();
        snapshot_to_load.output.enabled = false;
        for output in &mut snapshot_to_load.dmx_outputs {
            output.enabled = false;
        }
        let engine = engine::EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine
            .send(engine::EngineCommand::TriggerCue(cue_id))
            .unwrap();

        let mut nonzero = 0_usize;
        for _ in 0..40 {
            let snapshot = engine.snapshot();
            if snapshot.active_cue_id == Some(cue_id) {
                nonzero = snapshot
                    .dmx_preview
                    .iter()
                    .filter(|value| **value != 0)
                    .count();
                if nonzero > 0 {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(
            nonzero > 0,
            "imported cue {cue_id} should render non-zero DMX preview bytes"
        );
    }

    #[test]
    fn dvc_local_golden_spatial_color_cue_renders_and_moves() {
        let path = Path::new(r"C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        let cue_id = outcome
            .project
            .snapshot
            .cues
            .iter()
            .find(|cue| cue.label == "B-WineRed")
            .expect("full Shinkan project should contain the B-WineRed Knight Rider cue")
            .id;

        let mut snapshot_to_load = outcome.project.snapshot.clone();
        snapshot_to_load.output.enabled = false;
        for output in &mut snapshot_to_load.dmx_outputs {
            output.enabled = false;
        }
        let engine = engine::EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine
            .send(engine::EngineCommand::TriggerCue(cue_id))
            .unwrap();

        let mut first: Option<Vec<u8>> = None;
        for _ in 0..40 {
            let snapshot = engine.snapshot();
            if snapshot.active_cue_id == Some(cue_id) {
                let preview = snapshot.dmx_preview;
                if preview.iter().filter(|value| **value != 0).count() >= 3 {
                    first = Some(preview);
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        let first =
            first.expect("imported Knight Rider cue must light multiple bar segment channels");

        // The Knight Rider window sweeps the 64-beam strip; the rendered frame
        // must change over a fraction of the scene duration.
        let mut moved = false;
        for _ in 0..12 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            if engine.snapshot().dmx_preview != first {
                moved = true;
                break;
            }
        }
        assert!(moved, "spatial color pattern output must sweep over time");
    }

    #[test]
    fn dvc_local_golden_knight_rider_keeps_palette_floor_opaque() {
        let path = Path::new(r"C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        let cue_id = outcome
            .project
            .snapshot
            .cues
            .iter()
            .find(|cue| cue.label == "Par-OrangeStrobe")
            .expect("full Shinkan project should contain Par-OrangeStrobe")
            .id;
        let mut snapshot_to_load = outcome.project.snapshot.clone();
        snapshot_to_load.output.enabled = false;
        for output in &mut snapshot_to_load.dmx_outputs {
            output.enabled = false;
        }
        let engine = engine::EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        engine
            .send(engine::EngineCommand::TriggerCue(cue_id))
            .unwrap();

        let red_offsets = [0, 5, 10, 15, 20, 25, 30, 35, 40, 45];
        let green_offsets = [1, 6, 11, 16, 21, 26, 31, 36, 41, 46];
        let dimmer_offsets = [4, 9, 14, 19, 24, 29, 34, 39, 44, 52];
        let mut sampled = None;
        for _ in 0..40 {
            let snapshot = engine.snapshot();
            if snapshot.active_cue_id == Some(cue_id)
                && red_offsets
                    .iter()
                    .all(|offset| snapshot.dmx_preview[*offset] == 255)
            {
                sampled = Some(snapshot.dmx_preview);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        let sampled = sampled.expect("Knight Rider should retain the first palette colour");
        assert!(green_offsets.iter().all(|offset| sampled[*offset] >= 108));
        assert!(dimmer_offsets.iter().all(|offset| sampled[*offset] == 255));
    }

    #[test]
    fn dvc_local_golden_static_colors_match_daslight_live_truncation() {
        let path = Path::new(r"C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        let cue_rgb = |group: &str, label: &str| {
            outcome
                .project
                .snapshot
                .cues
                .iter()
                .find(|cue| cue.group_id.as_deref() == Some(group) && cue.label == label)
                .unwrap()
                .targets
                .iter()
                .map(|target| {
                    ["colorred", "colorgreen", "colorblue"].map(|attribute| {
                        target
                            .values
                            .iter()
                            .find(|value| normalize_dvc_attribute(&value.attribute) == attribute)
                            .map(|value| value.value / 257)
                            .unwrap_or_default()
                    })
                })
                .collect::<Vec<_>>()
        };

        // Captured from Daslight Tools > DMX Levels. The editor channel bytes
        // are 255/136/26 and 12/232/177, while live output truncates their
        // normalized half-float beam features to 255/135/25 and 12/231/176.
        let amber = cue_rgb("SaberSpot", "Amber");
        assert_eq!(amber.len(), 4);
        assert!(amber.iter().all(|rgb| *rgb == [255, 135, 25]));
        let aqua = cue_rgb("Side-Par", "SP-Aqua");
        assert_eq!(aqua.len(), 6);
        assert!(aqua.iter().all(|rgb| *rgb == [12, 231, 176]));
    }

    #[test]
    fn dvc_live_feature_bytes_match_captured_dmx_levels() {
        assert_eq!(dvc_live_feature_byte(1.0), 255);
        assert_eq!(dvc_live_feature_byte(0.533_203_1), 135);
        assert_eq!(dvc_live_feature_byte(0.101_928_71), 25);
        assert_eq!(dvc_live_feature_byte(0.047_058_105), 12);
        assert_eq!(dvc_live_feature_byte(0.909_667_97), 231);
        assert_eq!(dvc_live_feature_byte(0.693_847_66), 176);
        assert_eq!(dvc_live_feature_byte(0.705_566_4), 179);
    }

    #[test]
    fn dvc_local_golden_fx_cue_renders_chaser_via_go_path() {
        let path = Path::new(r"C:\Users\kouty\Documents\Daslight 5\Projects\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        let fx_cue = outcome
            .project
            .snapshot
            .cues
            .iter()
            .find(|cue| cue.label == "Fl-StrobeChase")
            .expect("golden project should contain the Fl-StrobeChase FX cue")
            .clone();

        let mut snapshot_to_load = outcome.project.snapshot.clone();
        snapshot_to_load.output.enabled = false;
        for output in &mut snapshot_to_load.dmx_outputs {
            output.enabled = false;
        }
        let engine = engine::EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine.load_project_snapshot(snapshot_to_load).unwrap();
        // Reach the FX cue through the same transport command the app GO button
        // sends, not through direct TriggerCue.
        let list_id = fx_cue.cue_list_id;
        for _ in 0..64 {
            engine
                .send(engine::EngineCommand::TriggerCueListNext(list_id))
                .unwrap();
            std::thread::sleep(std::time::Duration::from_millis(40));
            if engine.snapshot().active_cue_id == Some(fx_cue.id) {
                break;
            }
        }
        let snapshot = engine.snapshot();
        assert_eq!(
            snapshot.active_cue_id,
            Some(fx_cue.id),
            "GO transport should reach the Fl-StrobeChase cue"
        );

        let mut first_nonzero: Option<Vec<u8>> = None;
        for _ in 0..40 {
            let preview = engine.snapshot().dmx_preview.clone();
            if preview.iter().any(|value| *value != 0) {
                first_nonzero = Some(preview);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        let first = first_nonzero.expect("owned Chaser must render non-zero DMX after GO recall");

        // step_duration_ms for this cue is 3333; sample past one step boundary
        // and require the rendered frame to differ (the chase must move).
        let mut moved = false;
        for _ in 0..5 {
            std::thread::sleep(std::time::Duration::from_millis(900));
            if engine.snapshot().dmx_preview != first {
                moved = true;
                break;
            }
        }
        assert!(
            moved,
            "owned Chaser output must change across step boundaries"
        );
    }

    #[test]
    fn dvc_local_golden_project_matches_verified_counts_when_present() {
        let path = Path::new(r"C:\Users\kouty\Documents\Daslight 5\Projects\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        assert_eq!(outcome.report.summary.fixtures, 41);
        assert_eq!(outcome.report.summary.profiles, 15);
        assert_eq!(outcome.report.summary.groups, 13);
        assert_eq!(outcome.report.summary.cues, 30);
        assert_eq!(outcome.report.summary.values_decoded, 22);
        assert_eq!(outcome.report.summary.values_skipped, 0);
        assert_eq!(outcome.report.summary.beam_records, 221);
        assert_eq!(outcome.report.summary.beam_feature_checks, 109);
        assert_eq!(outcome.report.summary.beam_feature_mismatches, 0);
        assert_eq!(outcome.report.summary.effects_converted, 7);
        assert_eq!(outcome.report.summary.effects_skipped, 0);
        let min_x = outcome
            .project
            .snapshot
            .fixtures
            .iter()
            .map(|fixture| fixture.position.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = outcome
            .project
            .snapshot
            .fixtures
            .iter()
            .map(|fixture| fixture.position.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_z = outcome
            .project
            .snapshot
            .fixtures
            .iter()
            .map(|fixture| fixture.position.z)
            .fold(f32::INFINITY, f32::min);
        let max_z = outcome
            .project
            .snapshot
            .fixtures
            .iter()
            .map(|fixture| fixture.position.z)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min_x - -172.666_67).abs() < 0.001);
        assert!((max_x - 172.666_67).abs() < 0.001);
        assert!((min_z - -95.833_336).abs() < 0.001);
        assert!((max_z - 95.833_336).abs() < 0.001);
        let stage_map = outcome.project.snapshot.stage_map;
        assert!(stage_map.locked);
        assert!((stage_map.min_x - -177.666_67).abs() < 0.001);
        assert!((stage_map.max_x - 177.666_67).abs() < 0.001);
        assert!((stage_map.min_z - -177.666_67).abs() < 0.001);
        assert!((stage_map.max_z - 177.666_67).abs() < 0.001);
        let chaser_count = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter(|target| matches!(target.params, Some(EffectParamsSnapshot::Chaser(_))))
            .count();
        let curve_count = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter(|target| matches!(target.params, Some(EffectParamsSnapshot::Lfo(_))))
            .count();
        let move_count = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter(|target| matches!(target.params, Some(EffectParamsSnapshot::Move(_))))
            .count();
        assert_eq!(chaser_count, 5);
        assert_eq!(curve_count, 1);
        assert_eq!(move_count, 1);
        let strobe = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .find_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Lfo(request))
                    if request.label == "Fl-Strobe (Strobe)" =>
                {
                    Some(request)
                }
                _ => None,
            })
            .expect("Documents golden must convert Fl-Strobe to an owned LFO");
        assert_eq!(strobe.shape, LfoShape::Strobe);
        assert_eq!(strobe.period_ms, 5_000);
        assert_eq!(strobe.low, 0);
        assert_eq!(strobe.high, u16::MAX);
        assert_eq!(strobe.daslight_curve.as_ref().unwrap().rate, 2.0);
        assert_eq!(strobe.daslight_curve.as_ref().unwrap().sample_ms, 40);
        assert!(!outcome.report.approximate.details.iter().any(|detail| {
            detail.item == "Effect: Fl-Strobe (Strobe)"
                && detail.message.contains("2% of the period")
        }));
        assert!(!outcome
            .report
            .approximate
            .details
            .iter()
            .any(|detail| { detail.message == "segment selection approximated to fixture" }));
    }

    #[test]
    fn dvc_local_full_shinkan_super_scene_grid_and_dormant_conform_are_exact_when_present() {
        let path = Path::new(r"C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local full Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();

        assert_eq!(outcome.project.snapshot.clock.bpm, 120.0);
        let child_timelines = outcome
            .project
            .snapshot
            .cues
            .iter()
            .filter_map(|cue| cue.child_timeline.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(child_timelines.len(), 2);
        assert!(child_timelines.iter().all(|child| !child.tempo_driven));
        let child_events = child_timelines
            .iter()
            .flat_map(|child| child.events.iter())
            .collect::<Vec<_>>();
        assert_eq!(child_events.len(), 229);
        assert!(child_events
            .iter()
            .all(|event| !event.conform_to_tempo && event.loop_fill));
        assert!(!outcome
            .report
            .approximate
            .details
            .iter()
            .any(|detail| { detail.message.contains("CONFORM_TO_TEMPO") }));
        let reported_scene_blocks = outcome
            .report
            .converted
            .details
            .iter()
            .filter(|detail| detail.item.starts_with("Scene block: "))
            .collect::<Vec<_>>();
        assert!(!reported_scene_blocks.is_empty());
        assert!(reported_scene_blocks.iter().all(|detail| {
            detail.message.contains("parent_bpm_driven=false")
                && detail.message.contains("grid_bpm=120.000")
        }));
    }

    #[test]
    fn dvc_local_full_shinkan_move_fx_match_verified_generators_when_present() {
        let path = Path::new(r"C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local full Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        assert_eq!(outcome.report.summary.effects_converted, 30);
        assert_eq!(outcome.report.summary.effects_skipped, 0);
        assert_eq!(
            outcome.report.skipped.count, 0,
            "full Shinkan random-state COLOR generators must use corrected deterministic evaluators: {:#?}",
            outcome.report.skipped
        );
        assert_eq!(
            outcome
                .report
                .converted
                .details
                .iter()
                .filter(
                    |detail| detail.message.contains("implementation=SyndocalCorrected")
                        && detail.message.contains("stable_source_seed")
                )
                .count(),
            2
        );
        assert_eq!(outcome.report.unsupported.count, 3);
        assert!(outcome.report.unsupported.details.iter().all(|detail| {
            detail.item == "Daslight hardware devices"
                && detail
                    .message
                    .contains("configure Syndocal output routes explicitly")
        }));
        assert!(!outcome
            .report
            .skipped
            .details
            .iter()
            .any(|detail| { detail.message.contains("folded lane state") }));
        assert!(outcome
            .project
            .snapshot
            .cues
            .iter()
            .filter_map(|cue| cue.child_timeline.as_ref())
            .flat_map(|child| &child.layers)
            .all(|layer| !layer.expanded));
        let touch = &outcome.project.snapshot.touch_surface;
        assert_eq!(touch.pages.len(), 1);
        assert_eq!(touch.pages[0].label, "Page 1");
        assert_eq!(touch.pages[0].controls.len(), 1);
        let control = &touch.pages[0].controls[0];
        assert_eq!(control.kind, TouchControlKind::Button);
        assert_eq!((control.x, control.y, control.w, control.h), (0, 0, 1, 1));
        assert!(matches!(
            control.binding.as_ref(),
            Some(TouchControlBinding::GroupSelect { group_id }) if group_id == "saber spot rgbw"
        ));
        let imported_scene_blocks = outcome
            .project
            .snapshot
            .cues
            .iter()
            .filter_map(|cue| cue.child_timeline.as_ref())
            .flat_map(|child| &child.events)
            .collect::<Vec<_>>();
        let positive_scene_block_offsets = imported_scene_blocks
            .iter()
            .filter(|event| event.source_offset_ms > 0)
            .count();
        let negative_scene_block_offsets = imported_scene_blocks
            .iter()
            .filter(|event| event.source_offset_ms < 0)
            .count();
        assert_eq!(imported_scene_blocks.len(), 229);
        assert_eq!(positive_scene_block_offsets, 29);
        assert_eq!(negative_scene_block_offsets, 9);
        assert!(!outcome
            .report
            .approximate
            .details
            .iter()
            .any(|detail| { detail.message == "Negative source offset was clamped to zero" }));
        assert!(!outcome.report.skipped.details.iter().any(|detail| {
            detail.message == "Daslight source POSITION offset has no Scene Block field"
        }));
        let spatial_counts =
            outcome
                .project
                .snapshot
                .cues
                .iter()
                .flat_map(|cue| &cue.effect_targets)
                .filter_map(|target| match target.params.as_ref() {
                    Some(EffectParamsSnapshot::Color(request)) => request
                        .spatial_pattern
                        .as_ref()
                        .map(|pattern| match &pattern.recipe {
                            ColorEffectSpatialRecipe::KnightRider { .. } => 127,
                            ColorEffectSpatialRecipe::Sweep { .. } => 134,
                            ColorEffectSpatialRecipe::Burst { .. } => 121,
                            ColorEffectSpatialRecipe::RandomFill { .. } => 131,
                            ColorEffectSpatialRecipe::Sparkle { .. } => 133,
                            ColorEffectSpatialRecipe::Plasma { .. } => 129,
                            ColorEffectSpatialRecipe::ColorRainbow { .. } => 130,
                            ColorEffectSpatialRecipe::Rainbow { .. } => 521,
                            ColorEffectSpatialRecipe::Spiral { .. } => 522,
                            ColorEffectSpatialRecipe::Butterfly { .. } => 524,
                            ColorEffectSpatialRecipe::Perlin { .. } => 530,
                            ColorEffectSpatialRecipe::Grid { .. } => 50,
                            ColorEffectSpatialRecipe::Lines { .. } => 31,
                            ColorEffectSpatialRecipe::Graph { .. } => 49,
                            ColorEffectSpatialRecipe::Rain { .. } => 35,
                            ColorEffectSpatialRecipe::Bounce { .. } => 21,
                            ColorEffectSpatialRecipe::Fire { .. } => 29,
                            ColorEffectSpatialRecipe::Explosion { .. } => 47,
                            ColorEffectSpatialRecipe::Starfield { .. } => 48,
                        }),
                    _ => None,
                })
                .fold(HashMap::<u16, usize>::new(), |mut counts, generator| {
                    *counts.entry(generator).or_default() += 1;
                    counts
                });
        assert_eq!(spatial_counts.get(&127), Some(&5));
        assert_eq!(spatial_counts.get(&121), Some(&1));
        // DVC-RNG-CORRECTED routes the specimen's recovered Random Fill body
        // instead of preserving the former qrand-history fail-closed result.
        assert_eq!(spatial_counts.get(&131), Some(&1));
        // The same specimen also contains one Sparkle body that now uses the
        // recovered retained-particle grammar with a stable source seed.
        assert_eq!(spatial_counts.get(&133), Some(&1));
        assert_eq!(spatial_counts.get(&129), Some(&2));
        assert_eq!(spatial_counts.get(&130), Some(&3));
        assert!(!outcome.report.skipped.details.iter().any(|detail| {
            detail.message.contains("ID=129")
                || detail.item.contains("ID=129")
                || detail.message.contains("ID=131")
                || detail.item.contains("ID=131")
                || detail.message.contains("ID=133")
                || detail.item.contains("ID=133")
        }));
        assert!(!outcome
            .report
            .skipped
            .details
            .iter()
            .any(|detail| { detail.message.contains("ID=130") || detail.item.contains("ID=130") }));
        assert_eq!(
            outcome
                .report
                .converted
                .details
                .iter()
                .filter(|detail| detail.message.contains("source no-op preserved"))
                .count(),
            3
        );
        assert!(!outcome
            .report
            .approximate
            .details
            .iter()
            .any(|detail| detail.message == "segment selection approximated to fixture"));

        let bar_strobe = outcome
            .project
            .snapshot
            .cues
            .iter()
            .find(|cue| cue.label == "Bar-StrobeAMber")
            .and_then(|cue| {
                cue.effect_targets
                    .iter()
                    .find_map(|target| match target.params.as_ref() {
                        Some(EffectParamsSnapshot::Lfo(request)) => Some(request),
                        _ => None,
                    })
            })
            .expect("full Shinkan golden must preserve Bar-StrobeAMber Curve FX");
        assert_eq!(bar_strobe.beam_targets.len(), 64);
        assert_eq!(
            bar_strobe
                .beam_targets
                .iter()
                .map(|target| target.beam_index)
                .collect::<HashSet<_>>(),
            HashSet::from([0, 1, 2, 3, 4, 5, 6, 7])
        );
        assert_eq!(
            bar_strobe
                .beam_targets
                .iter()
                .map(|target| target.selection_index)
                .collect::<HashSet<_>>()
                .len(),
            48
        );

        let bar_side_chaser = outcome
            .project
            .snapshot
            .cues
            .iter()
            .filter(|cue| cue.label == "New Scene")
            .find_map(|cue| {
                cue.effect_targets
                    .iter()
                    .find_map(|target| match target.params.as_ref() {
                        Some(EffectParamsSnapshot::Chaser(request))
                            if request
                                .steps
                                .iter()
                                .map(|step| step.beam_targets.len())
                                .sum::<usize>()
                                == 16 =>
                        {
                            Some(request)
                        }
                        _ => None,
                    })
            })
            .expect("full Shinkan golden must preserve the 16-cell Bar-Side Chaser");
        assert_eq!(bar_side_chaser.steps.len(), 16);
        assert!(bar_side_chaser
            .steps
            .iter()
            .all(|step| step.fixture_ids.is_empty() && step.beam_targets.len() == 1));
        assert_eq!(
            bar_side_chaser
                .steps
                .iter()
                .flat_map(|step| &step.beam_targets)
                .map(|target| target.beam_index)
                .collect::<HashSet<_>>(),
            HashSet::from([0, 1, 2, 3, 4, 5, 6, 7])
        );

        let moves = outcome
            .project
            .snapshot
            .cues
            .iter()
            .filter_map(|cue| {
                cue.effect_targets.iter().find_map(|target| {
                    let Some(EffectParamsSnapshot::Move(request)) = &target.params else {
                        return None;
                    };
                    Some((cue.label.as_str(), request))
                })
            })
            .collect::<HashMap<_, _>>();
        assert_eq!(moves.len(), 3);

        let left_to_right = moves
            .get("Left2Right")
            .expect("full Shinkan golden must contain Left2Right");
        assert_eq!(left_to_right.points.len(), 2);
        assert!(!left_to_right.closed);
        assert_eq!(left_to_right.interpolation, MoveInterpolation::DaslightLine);
        assert_eq!(left_to_right.direction, MoveDirection::Forward);
        assert!((left_to_right.fixture_spread - 0.01).abs() < f32::EPSILON);

        let polygon = moves
            .get("M-PolyLoop")
            .expect("full Shinkan golden must contain M-PolyLoop");
        assert_eq!(
            polygon.points,
            vec![
                MovePathPoint { x: 0.25, y: 0.5 },
                MovePathPoint { x: 0.5, y: 0.75 },
                MovePathPoint { x: 0.75, y: 0.5 },
                MovePathPoint { x: 0.5, y: 0.25 },
            ]
        );
        assert!(polygon.closed);
        assert_eq!(polygon.interpolation, MoveInterpolation::DaslightPolygon);
        assert_eq!(polygon.direction, MoveDirection::Forward);
        assert!((polygon.fixture_spread - 0.02).abs() < f32::EPSILON);

        let center_div = moves
            .get("M-CenterDivLoop")
            .expect("full Shinkan golden must contain M-CenterDivLoop");
        assert_eq!(center_div.points.len(), 2);
        assert!(!center_div.closed);
        assert_eq!(center_div.interpolation, MoveInterpolation::DaslightLine);
        assert_eq!(center_div.direction, MoveDirection::Forward);
        assert!((center_div.fixture_spread - 0.176).abs() < f32::EPSILON);
    }

    #[test]
    fn dvc_local_full_shinkan_random_chaser_preserves_sequence_and_cycles_when_present() {
        let path = Path::new(r"C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local full Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        let random_chasers = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| cue.effect_targets.iter())
            .filter_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Chaser(request))
                    if request.direction == ChaserDirection::Random =>
                {
                    Some(request)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(!random_chasers.is_empty());
        assert!(random_chasers
            .iter()
            .all(|request| request.random_seed == 0 && request.random_cycle_count == 1));
        assert!(!outcome.report.approximate.details.iter().any(|detail| {
            detail.item.contains("Chaser random")
                && (detail.message.contains("RandomSeq") || detail.message.contains("NbCycles"))
        }));
    }

    #[test]
    fn dvc_local_full_shinkan_super_scenes_are_referentially_complete_and_survive_sdc_reload() {
        let path = Path::new(r"C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local full Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }

        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        let cue_ids = outcome
            .project
            .snapshot
            .cues
            .iter()
            .map(|cue| cue.id)
            .collect::<HashSet<_>>();
        let super_scenes = outcome
            .project
            .snapshot
            .cues
            .iter()
            .filter_map(|cue| cue.child_timeline.as_ref().map(|child| (cue, child)))
            .collect::<Vec<_>>();
        assert_eq!(super_scenes.len(), 2);
        assert_eq!(
            super_scenes
                .iter()
                .map(|(cue, _)| cue.label.as_str())
                .collect::<HashSet<_>>(),
            HashSet::from(["Shin", "Unr"])
        );

        let mut event_ids = HashSet::new();
        let mut event_count = 0_usize;
        let mut audio_clip_count = 0_usize;
        for (owner, child) in &super_scenes {
            assert!(
                !child.events.is_empty(),
                "{} must contain Scene Blocks",
                owner.label
            );
            assert!(
                child.duration_ms > 0,
                "{} must have a positive duration",
                owner.label
            );
            let layer_kinds = child
                .layers
                .iter()
                .map(|layer| (layer.id, layer.kind))
                .collect::<HashMap<_, _>>();
            for event in &child.events {
                assert!(
                    event_ids.insert(event.id),
                    "Scene Block id {} is duplicated across imported Super Scenes",
                    event.id
                );
                assert!(
                    cue_ids.contains(&event.cue_id),
                    "{} block {} references missing Cue {}",
                    owner.label,
                    event.id,
                    event.cue_id
                );
                assert_ne!(
                    event.cue_id, owner.id,
                    "{} block {} recursively references its owner",
                    owner.label, event.id
                );
                let referenced = outcome
                    .project
                    .snapshot
                    .cues
                    .iter()
                    .find(|cue| cue.id == event.cue_id)
                    .unwrap();
                assert!(
                    referenced.child_timeline.is_none(),
                    "{} block {} creates unsupported nested Super Scene playback",
                    owner.label,
                    event.id
                );
                let layer_id = event.layer_id.unwrap_or_else(|| {
                    panic!("{} block {} has no typed layer", owner.label, event.id)
                });
                let layer_kind = layer_kinds.get(&layer_id).unwrap_or_else(|| {
                    panic!(
                        "{} block {} references missing layer {}",
                        owner.label, event.id, layer_id
                    )
                });
                assert!(
                    matches!(
                        (event.track.clone(), layer_kind),
                        (TimelineTrackKind::Lighting, TimelineLayerKind::Lighting)
                            | (TimelineTrackKind::Video, TimelineLayerKind::Video)
                    ),
                    "{} block {} track/layer type mismatch",
                    owner.label,
                    event.id
                );
                assert!(
                    event.time_ms.saturating_add(event.duration_ms) <= child.duration_ms,
                    "{} block {} exceeds its child timeline duration",
                    owner.label,
                    event.id
                );
                if let Some(jump_id) = event.jump_to_event_id {
                    assert!(
                        child.events.iter().any(|candidate| candidate.id == jump_id),
                        "{} block {} jumps outside its child timeline",
                        owner.label,
                        event.id
                    );
                }
                event_count += 1;
            }
            for clip in &child.audio_clips {
                assert_eq!(
                    layer_kinds.get(&clip.layer_id),
                    Some(&TimelineLayerKind::Audio),
                    "{} audio clip {} must reference an Audio layer",
                    owner.label,
                    clip.id
                );
                assert!(
                    clip.start_ms.saturating_add(clip.duration_ms) <= child.duration_ms,
                    "{} audio clip {} exceeds its child timeline duration",
                    owner.label,
                    clip.id
                );
                audio_clip_count += 1;
            }
        }
        assert_eq!(event_count, 229);
        assert_eq!(
            audio_clip_count + outcome.project.snapshot.timeline.audio_clips.len(),
            2,
            "all imported Show and Super Scene audio clips must remain represented"
        );

        // Exercise the same JSON write/read boundary used by `.sdc` Save and
        // Open, then prove every imported authored field remains byte-semantic.
        let json = crate::project_json_for_write(&outcome.project).unwrap();
        let roundtrip: ProjectFile = serde_json::from_str(&json).unwrap();
        crate::validate_project_file(&roundtrip).unwrap();
        let authored_sections = [
            (
                "embedded profiles",
                serde_json::to_value(&outcome.project.custom_profiles).unwrap(),
                serde_json::to_value(&roundtrip.custom_profiles).unwrap(),
            ),
            (
                "fixtures",
                serde_json::to_value(&outcome.project.snapshot.fixtures).unwrap(),
                serde_json::to_value(&roundtrip.snapshot.fixtures).unwrap(),
            ),
            (
                "Scenes",
                serde_json::to_value(&outcome.project.snapshot.cues).unwrap(),
                serde_json::to_value(&roundtrip.snapshot.cues).unwrap(),
            ),
            (
                "Timeline",
                serde_json::to_value(&outcome.project.snapshot.timeline).unwrap(),
                serde_json::to_value(&roundtrip.snapshot.timeline).unwrap(),
            ),
            (
                "Touch",
                serde_json::to_value(&outcome.project.snapshot.touch_surface).unwrap(),
                serde_json::to_value(&roundtrip.snapshot.touch_surface).unwrap(),
            ),
        ];
        for (label, before, after) in authored_sections {
            let difference = first_json_difference(&before, &after, label);
            assert!(
                difference.is_none(),
                "imported {label} JSON must survive reload; first difference: {}",
                difference.unwrap_or_else(|| "unknown".to_string())
            );
        }
    }

    #[test]
    fn dvc_local_homecoming_mapping_and_burst_goldens_match_when_present() {
        let path = Path::new(r"C:\Users\kouty\Desktop\homecoming2026\homecoming2606.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local homecoming Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        let recipes = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Color(request)) => request
                    .spatial_pattern
                    .as_ref()
                    .map(|pattern| &pattern.recipe),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            recipes
                .iter()
                .filter(|recipe| matches!(recipe, ColorEffectSpatialRecipe::Burst { .. }))
                .count(),
            3
        );
        assert_eq!(
            recipes
                .iter()
                .filter(|recipe| matches!(recipe, ColorEffectSpatialRecipe::Rainbow { .. }))
                .count(),
            1
        );
        assert_eq!(
            recipes
                .iter()
                .filter(|recipe| matches!(recipe, ColorEffectSpatialRecipe::Perlin { .. }))
                .count(),
            1
        );
        assert_eq!(outcome.report.summary.effects_skipped, 0);
    }

    // Real saved Daslight 5.0.6.2 specimen authored on 2026-08-11. Each scene
    // keeps the same concrete Dimmer binding and ordered 32-beam selection;
    // only the Curve generator ID changes.
    #[test]
    fn dvc_local_golden_unrouted_curve_catalog_imports_all_five_saved_generators() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/CurveCatalog-Unrouted.dvc"
        ));
        assert!(path.is_file(), "repo-portable Curve specimen is missing");
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();

        let requests = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Lfo(request)) => Some(request),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            requests.len(),
            5,
            "all captured Curve generators must import"
        );

        let mut canonical_beam_order = None;
        for (generator, shape) in [
            ("Ramp", LfoShape::Ramp),
            ("Random", LfoShape::Random),
            ("Sinus3", LfoShape::Sinus3),
            ("Tangeant", LfoShape::Tangeant),
            ("Triangle", LfoShape::Triangle),
        ] {
            let request = requests
                .iter()
                .find(|request| request.label == format!("New Scene ({generator})"))
                .unwrap_or_else(|| panic!("saved Curve {generator} must retain its label"));
            assert_eq!(request.shape, shape);
            assert_eq!(request.attribute, "Dimmer");
            assert_eq!(request.period_ms, 5_000);
            assert_eq!(request.beam_targets.len(), 32);
            assert_eq!(
                request
                    .beam_targets
                    .iter()
                    .map(|target| target.selection_index)
                    .collect::<Vec<_>>(),
                (0..32).collect::<Vec<_>>(),
                "{generator} must preserve captured beam selection order"
            );
            assert!(request
                .beam_targets
                .iter()
                .all(|target| target.feature_attribute == "Dimmer"));
            let beam_order = request
                .beam_targets
                .iter()
                .map(|target| (target.fixture_id, target.beam_index, target.selection_index))
                .collect::<Vec<_>>();
            if let Some(canonical) = &canonical_beam_order {
                assert_eq!(&beam_order, canonical, "{generator} target order drifted");
            } else {
                canonical_beam_order = Some(beam_order);
            }
            let source = request
                .daslight_curve
                .as_ref()
                .unwrap_or_else(|| panic!("{generator} must retain Curve provenance"));
            assert_eq!(source.rate, 2.0);
            assert_eq!(source.size, 1.0);
            assert_eq!(source.offset, 0.0);
            assert_eq!(source.sample_ms, 40);
            if generator == "Random" {
                assert!(
                    source.rng_seed.is_some(),
                    "Random must carry a stable source seed"
                );
            } else {
                assert_eq!(source.rng_seed, None);
            }
        }
        assert!(outcome.report.summary.effects_converted >= 5);
        for evaluator in [
            "CRampEffect@0x14036FAE0",
            "CRandomEffect@0x14036FE90",
            "CSinus3Effect@0x140370070",
            "CTangeantEffect@0x140370760",
            "CTriangleEffect@0x140370930",
        ] {
            assert!(outcome
                .report
                .converted
                .details
                .iter()
                .any(|detail| detail.message.contains(evaluator)));
        }
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item.contains("Random")
                && detail
                    .message
                    .contains("unavailable_qrand=stable_source_seed")
        }));
        for generator in ["Ramp", "Random", "Sinus3", "Tangeant", "Triangle"] {
            assert!(!outcome
                .report
                .skipped
                .details
                .iter()
                .any(|detail| detail.item.contains(&format!("({generator})"))));
        }
    }

    #[test]
    fn dvc_local_golden_custom_curve_preserves_saved_points_and_beam_order() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/CurveCatalog-Custom.dvc"
        ));
        assert!(
            path.is_file(),
            "repo-portable Custom Curve specimen is missing"
        );
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();

        let request = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Lfo(request))
                    if request.shape == LfoShape::DaslightCustom =>
                {
                    Some(request)
                }
                _ => None,
            })
            .next()
            .unwrap_or_else(|| {
                panic!(
                    "captured Custom generator must import; skipped={:?}",
                    outcome.report.skipped.details
                )
            });
        assert_eq!(request.label, "New Scene (Custom)");
        assert_eq!(request.attribute, "Dimmer");
        assert_eq!(request.period_ms, 5_000);
        assert_eq!(request.phase, 0.0);
        assert_eq!(request.fixture_spread, 0.0);
        assert!(request.video_targets.is_empty());
        assert!(request.daslight_curve.is_none());
        let source = request
            .daslight_custom_curve
            .as_ref()
            .expect("Custom source profile must be retained");
        assert_eq!(source.sample_ms, 40);
        assert_eq!(source.phasing, 0.0);
        assert_eq!(
            source.points,
            vec![
                DaslightCustomCurvePoint { x: 0.0, raw_y: 0.5 },
                DaslightCustomCurvePoint { x: 1.0, raw_y: 0.5 },
            ]
        );
        assert_eq!(request.beam_targets.len(), 32);
        assert_eq!(
            request
                .beam_targets
                .iter()
                .map(|target| target.selection_index)
                .collect::<Vec<_>>(),
            (0..32).collect::<Vec<_>>()
        );
        assert!(request
            .beam_targets
            .iter()
            .all(|target| target.feature_attribute == "Dimmer"));
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item.contains("Custom")
                && detail.message.contains("CCustomCurveEffect@0x14036F1F0")
                && detail.message.contains("right_point_easing")
        }));
        assert!(!outcome
            .report
            .skipped
            .details
            .iter()
            .any(|detail| detail.item.contains("Custom")));
    }

    #[test]
    fn dvc_custom_curve_real_schema_mutations_fail_closed() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/CurveCatalog-Custom.dvc"
        ));
        let source = fs::read_to_string(path).unwrap();
        let mutate_custom = |needle: &str, replacement: &str| {
            let start = source
                .find(r#"<EFFECT TYPE="5" ID="13""#)
                .expect("saved Custom effect");
            let (prefix, custom_and_rest) = source.split_at(start);
            format!(
                "{prefix}{}",
                custom_and_rest.replacen(needle, replacement, 1)
            )
        };
        let custom_skip = |label: &str, mutated: String| {
            let outcome = import_bytes(mutated.as_bytes(), "mutated-custom.dvc").unwrap();
            outcome
                .report
                .skipped
                .details
                .iter()
                .find(|detail| detail.item.contains("Custom"))
                .map(|detail| detail.message.clone())
                .unwrap_or_else(|| {
                    panic!(
                        "{label}: mutated Custom must be skipped; converted={:?}",
                        outcome.report.converted.details
                    )
                })
        };

        assert!(custom_skip(
            "missing selection",
            mutate_custom(r#" IDSELECTION="1""#, "")
        )
        .contains("IDSELECTION"));
        assert!(custom_skip(
            "external selections",
            mutate_custom(r#"<BEAMS NB="32">"#, r#"<SELECTIONS/><BEAMS NB="32">"#)
        )
        .contains("external SELECTIONS"));
        assert!(custom_skip(
            "short duration",
            mutate_custom(r#"ID="13" DURATION="5000""#, r#"ID="13" DURATION="39""#,)
        )
        .contains("within 40"));
        assert!(custom_skip(
            "overflow duration",
            mutate_custom(
                r#"ID="13" DURATION="5000""#,
                r#"ID="13" DURATION="4294967296""#,
            )
        )
        .contains("unsigned 32-bit"));
        assert!(custom_skip(
            "duplicate params",
            mutate_custom("</PARAMS>", "</PARAMS><PARAMS NB=\"0\"/>")
        )
        .contains("exactly one direct PARAMS"));
        assert!(custom_skip(
            "duplicate beams",
            mutate_custom(r#"<BEAMS NB="32">"#, r#"<BEAMS NB="0"/><BEAMS NB="32">"#,)
        )
        .contains("exactly one direct BEAMS"));
        for (label, needle, replacement) in [
            (
                "effect attribute",
                r#"<EFFECT TYPE="5" ID="13" DURATION="5000">"#,
                r#"<EFFECT TYPE="5" ID="13" DURATION="5000" EXTRA="1">"#,
            ),
            (
                "params attribute",
                r#"<PARAMS NB="2">"#,
                r#"<PARAMS NB="2" EXTRA="1">"#,
            ),
            (
                "points param attribute",
                r#"<PARAM TYPE="5" ID="1">"#,
                r#"<PARAM TYPE="5" ID="1" EXTRA="1">"#,
            ),
            (
                "points attribute",
                r#"<POINTS NB="2">"#,
                r#"<POINTS NB="2" EXTRA="1">"#,
            ),
            (
                "point attribute",
                r#"<POINT X="0" Y="0.5"/>"#,
                r#"<POINT X="0" Y="0.5" EXTRA="1"/>"#,
            ),
            (
                "phasing attribute",
                r#"<PARAM TYPE="1" ID="2" VAL="0"/>"#,
                r#"<PARAM TYPE="1" ID="2" VAL="0" EXTRA="1"/>"#,
            ),
            (
                "beams attribute",
                r#"<BEAMS NB="32">"#,
                r#"<BEAMS NB="32" EXTRA="1">"#,
            ),
            (
                "beam attribute",
                r#"BEAMID="0" IDSELECTION="1"/>"#,
                r#"BEAMID="0" IDSELECTION="1" EXTRA="1"/>"#,
            ),
        ] {
            assert!(
                custom_skip(label, mutate_custom(needle, replacement))
                    .contains("attributes must be exactly"),
                "{label} must fail closed"
            );
        }
    }

    #[test]
    fn dvc_custom_curve_schema_rejects_malformed_point_lists() {
        let parse = |body: &str| {
            let xml = format!(
                r#"<EFFECT><PARAMS NB="2"><PARAM TYPE="5" ID="1"><POINTS NB="2">{body}</POINTS></PARAM><PARAM TYPE="1" ID="2" VAL="0.25"/></PARAMS></EFFECT>"#
            );
            let document = Document::parse(&xml).unwrap();
            dvc_custom_curve_params(document.root_element()).unwrap_err()
        };
        assert!(parse(r#"<POINT X="0.5" Y="0"/><POINT X="0.5" Y="1"/>"#)
            .contains("strictly increasing"));
        assert!(
            parse(r#"<POINT X="0" Y="0"/><POINT X="1" Y="11.5"/>"#).contains("must encode easing")
        );
        assert!(
            parse(r#"<POINT X="0" Y="0"><EXTRA/></POINT><POINT X="1" Y="1"/>"#)
                .contains("must not contain child")
        );

        let document = Document::parse(
            r#"<EFFECT><PARAMS NB="2"><PARAM TYPE="5" ID="1"><POINTS NB="3"><POINT X="0" Y="0"/><POINT X="1" Y="1"/></POINTS></PARAM><PARAM TYPE="1" ID="2" VAL="0"/></PARAMS></EFFECT>"#,
        )
        .unwrap();
        assert!(dvc_custom_curve_params(document.root_element())
            .unwrap_err()
            .contains("declare and contain"));

        let points_xml = (0..255)
            .map(|index| {
                let progress = index as f32 / 254.0;
                format!(
                    r#"<POINT X="{progress}" Y="{}"/>"#,
                    (index % 5) as f32 * 10.0 + progress
                )
            })
            .collect::<String>();
        let xml = format!(
            r#"<EFFECT><PARAMS NB="2"><PARAM TYPE="5" ID="1"><POINTS NB="255">{points_xml}</POINTS></PARAM><PARAM TYPE="1" ID="2" VAL="1"/></PARAMS></EFFECT>"#
        );
        let document = Document::parse(&xml).unwrap();
        let (points, phasing) = dvc_custom_curve_params(document.root_element()).unwrap();
        assert_eq!(points.len(), 255);
        assert_eq!(points.first().unwrap().x, 0.0);
        assert_eq!(points.last().unwrap().x, 1.0);
        assert_eq!(points.last().unwrap().raw_y, 41.0);
        assert_eq!(phasing, 1.0);
    }

    #[test]
    fn dvc_custom_curve_rejects_a_dropped_beam_inside_a_retained_fixture() {
        let document = Document::parse(
            r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACK TYPE="8"><EFFECT TYPE="5" ID="13" DURATION="1000"><PARAMS NB="2"><PARAM TYPE="5" ID="1"><POINTS NB="2"><POINT X="0" Y="0"/><POINT X="1" Y="1"/></POINTS></PARAM><PARAM TYPE="1" ID="2" VAL="0"/></PARAMS></EFFECT><BEAMS NB="2"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/><BEAM FIXTURE="fixture-1" BEAMID="1" IDSELECTION="2"/></BEAMS></RACK></SCENE>"#,
        )
        .unwrap();
        let scene = document.root_element();
        let rack = direct_child(scene, "RACK").unwrap();
        let effect = direct_child(rack, "EFFECT").unwrap();
        let fixture_refs = HashMap::from([(
            "fixture-1".to_string(),
            FixtureImportRef {
                fixture_id: 1,
                fixture_index: 0,
                profile_index: 0,
                supports_dimmer: false,
                color_beam_count: 1,
                patch_beam_positions: HashMap::new(),
            },
        )]);
        let error =
            convert_dvc_custom_curve_effect(scene, "Mixed beam", rack, effect, 1, &fixture_refs)
                .unwrap_err();
        assert!(error.contains("1 beam target(s) across 0 fixture target(s)"));
    }

    // Real saved Daslight 5.0.6.2 specimen authored on 2026-08-11. Both
    // effects use the same ordered 32-beam Dimmer binding; only the symmetric
    // Chaser generator and its boolean controls differ.
    #[test]
    fn dvc_local_golden_symmetric_chasers_preserve_recovered_pair_topology() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ChaserCatalog-323-324.dvc"
        ));
        assert!(
            path.is_file(),
            "repo-portable symmetric Chaser specimen is missing"
        );
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();

        let requests = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Chaser(request))
                    if request.label.contains("Chaser #3")
                        || request.label.contains("Chaser #4") =>
                {
                    Some(request)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(requests.len(), 2);

        let type3 = requests
            .iter()
            .find(|request| request.label.contains("Chaser #3"))
            .expect("saved Chaser #3 must import");
        assert_eq!(type3.direction, ChaserDirection::Forward);
        assert_eq!(type3.overlap, 0.0);
        assert_eq!(type3.active_step_count, 1);
        assert_eq!(type3.steps.len(), 16);
        assert_eq!(type3.step_duration_ms, 313);
        assert_eq!(
            type3.steps[0]
                .beam_targets
                .iter()
                .map(|target| target.selection_index)
                .collect::<Vec<_>>(),
            vec![0, 31],
            "Chaser #3 must start at both authored ends"
        );
        assert_eq!(
            type3.steps[15]
                .beam_targets
                .iter()
                .map(|target| target.selection_index)
                .collect::<Vec<_>>(),
            vec![15, 16],
            "Chaser #3 must finish at the authored centre pair"
        );

        let type4 = requests
            .iter()
            .find(|request| request.label.contains("Chaser #4"))
            .expect("saved Chaser #4 must import");
        assert_eq!(type4.direction, ChaserDirection::Bounce);
        assert_eq!(type4.overlap, 1.0);
        assert_eq!(type4.active_step_count, 1);
        assert_eq!(type4.steps.len(), 16);
        assert_eq!(type4.step_duration_ms, 167);
        assert_eq!(
            type4.steps[0]
                .beam_targets
                .iter()
                .map(|target| target.selection_index)
                .collect::<Vec<_>>(),
            vec![15, 16],
            "Chaser #4 must start at the authored centre pair"
        );
        assert_eq!(
            type4.steps[15]
                .beam_targets
                .iter()
                .map(|target| target.selection_index)
                .collect::<Vec<_>>(),
            vec![0, 31],
            "Chaser #4 must finish at both authored ends"
        );
        for evaluator in [
            "Chaser #3 evaluator 0x1403775F0",
            "Chaser #4 evaluator 0x140378400",
        ] {
            assert!(outcome
                .report
                .approximate
                .details
                .iter()
                .any(|detail| detail.message.contains(evaluator)));
        }
        for generator in ["Chaser #3", "Chaser #4"] {
            assert!(!outcome
                .report
                .skipped
                .details
                .iter()
                .any(|detail| detail.item.contains(generator)));
        }
    }

    #[test]
    fn dvc_local_golden_mapping_catalog_routes_shared_2d_rasters_and_empty_media_noop() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/MappingCatalog-522-529.dvc"
        ));
        assert!(path.is_file(), "repo-portable MAPPINGS specimen is missing");
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();

        let requests = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Color(request))
                    if request.spatial_pattern.as_ref().is_some_and(|pattern| {
                        pattern.placement.is_some()
                            && matches!(
                                pattern.recipe,
                                ColorEffectSpatialRecipe::Rainbow { .. }
                                    | ColorEffectSpatialRecipe::Burst { .. }
                                    | ColorEffectSpatialRecipe::Spiral { .. }
                                    | ColorEffectSpatialRecipe::Butterfly { .. }
                                    | ColorEffectSpatialRecipe::Plasma { .. }
                                    | ColorEffectSpatialRecipe::KnightRider { .. }
                                    | ColorEffectSpatialRecipe::Sweep { .. }
                                    | ColorEffectSpatialRecipe::Sparkle { .. }
                            )
                    }) =>
                {
                    Some(request)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            requests.len(),
            9,
            "all placed MAPPINGS raster routes must import"
        );

        for request in &requests {
            assert_eq!(request.blend_mode, EffectBlendMode::Multiply);
            let pattern = request.spatial_pattern.as_ref().unwrap();
            let placement = pattern.placement.as_ref().unwrap();
            assert_eq!(
                placement.mapping_shape,
                ColorEffectSpatialMappingShape::Rectangle
            );
            assert!(!placement.target_coordinates.is_empty());
            assert!(!placement.vertical_symmetry);
            assert!(!placement.horizontal_symmetry);
            assert_eq!(placement.raster_rotation_degrees, 0.0);
            assert!(pattern
                .beam_targets
                .iter()
                .all(|target| target.feature_attribute.as_deref() == Some("Dimmer")));
        }

        let recipes = requests
            .iter()
            .map(|request| &request.spatial_pattern.as_ref().unwrap().recipe)
            .collect::<Vec<_>>();
        assert!(recipes.iter().any(|recipe| matches!(
            recipe,
            ColorEffectSpatialRecipe::Burst {
                grayscale: false,
                vertical_symmetry: false,
                color_width,
                gradient,
            } if *color_width == 50.0 && *gradient == 100.0
        )));
        assert!(recipes.iter().any(|recipe| matches!(
            recipe,
            ColorEffectSpatialRecipe::Spiral {
                grayscale: false,
                radius,
                arms: 1,
                gradient,
            } if *radius == 30.0 && *gradient == 100.0
        )));
        assert!(recipes.iter().any(|recipe| matches!(
            recipe,
            ColorEffectSpatialRecipe::Butterfly {
                grayscale: false,
                color_width,
                gradient,
                clockwise: true,
            } if *color_width == 100.0 && *gradient == 50.0
        )));
        assert!(
            recipes.iter().any(|recipe| matches!(
                recipe,
                ColorEffectSpatialRecipe::Sparkle {
                    grayscale: false,
                    vertical_symmetry: false,
                    number: 5,
                    lifetime_ms: Some(100),
                    source_lifespan: Some(source_lifespan),
                    width,
                    height: Some(height),
                    ..
                } if *source_lifespan == 0.0 && *width == 1.0 && *height == 1.0
            )),
            "imported recipes: {recipes:#?}; report: {:#?}",
            outcome.report
        );
        assert!(recipes.iter().any(|recipe| matches!(
            recipe,
            ColorEffectSpatialRecipe::Plasma {
                grayscale: false,
                vertical_symmetry: false,
                size_x,
                param_x,
                size_y,
                param_y,
                speed_x,
                param_sx,
                speed_y,
                param_sy,
            } if *size_x == 1.0
                && *param_x == 2.0
                && *size_y == 1.0
                && *param_y == 2.0
                && *speed_x == -1.0
                && *param_sx == 2.0
                && *speed_y == 1.0
                && *param_sy == -1.0
        )));
        assert!(recipes.iter().any(|recipe| matches!(
            recipe,
            ColorEffectSpatialRecipe::KnightRider {
                grayscale: false,
                vertical_symmetry: false,
                size,
                one_way: false,
                fading: true,
                go_outside: false,
                gradient,
            } if *size == 1.0 && *gradient == 50.0
        )));
        assert!(recipes.iter().any(|recipe| matches!(
            recipe,
            ColorEffectSpatialRecipe::Sweep {
                grayscale: false,
                vertical_symmetry: false,
                direction_change: false,
            }
        )));

        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item.contains("Media")
                && detail.message.contains("source-no-op")
                && detail.message.contains("MediaPath is empty")
        }));
        for implemented in ["ID=522", "ID=524", "ID=529"] {
            assert!(!outcome
                .report
                .skipped
                .details
                .iter()
                .any(|detail| detail.message.contains(implemented)));
        }
    }

    #[test]
    fn dvc_symmetric_chaser_pairing_retains_an_odd_centre_once() {
        let source = (0..5_u32)
            .map(|selection_index| {
                vec![EffectBeamTarget {
                    fixture_id: 1,
                    beam_index: selection_index as u16,
                    selection_index,
                    feature_attribute: "Dimmer".to_string(),
                }]
            })
            .collect::<Vec<_>>();
        let outside_in = dvc_symmetric_chaser_beam_steps(source.clone(), false);
        let center_out = dvc_symmetric_chaser_beam_steps(source, true);
        let selections = |steps: &[Vec<EffectBeamTarget>]| {
            steps
                .iter()
                .map(|step| {
                    step.iter()
                        .map(|target| target.selection_index)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            selections(&outside_in),
            vec![vec![0, 4], vec![1, 3], vec![2]]
        );
        assert_eq!(
            selections(&center_out),
            vec![vec![2], vec![1, 3], vec![0, 4]]
        );
    }

    // Real saved Daslight specimen authored on 2026-08-10 (Fable, elevated
    // permissions) to turn the DVC-V3 static proof into a product-to-product
    // golden: a scratch project holding a targeted VALUE Sweep (ID625,
    // Direction Change ON) and a VALUE Plasma (ID623) at Daslight constructor
    // defaults, each with a Dimmer feature bound through the real UI (saved as
    // a PRESETS block with SSLPRESET=4 / SSLCHANNEL=-1 / MIN=0 / MAX=1). The
    // repo copy under qa/specimens is the durable golden; this test prefers it
    // and therefore runs on any checkout of this repository.
    #[test]
    fn dvc_local_golden_value_sweep_and_plasma_import_from_saved_specimen() {
        let repo_specimen = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ValueCatalog-Sweep-Plasma.dvc"
        ));
        let machine_specimen = Path::new(
            r"C:\Users\kouty\Documents\Daslight 5\Projects\specimens\ValueCatalog-Sweep-Plasma.dvc",
        );
        let path = if repo_specimen.is_file() {
            repo_specimen
        } else if machine_specimen.is_file() {
            machine_specimen
        } else {
            eprintln!("Skipping VALUE catalog Daslight golden: no specimen available");
            return;
        };
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        // The specimen also carries two laser-targeted VALUE scenes whose
        // f3200a profile exposes no PRESET type 4; they must stay skipped
        // (real-file coverage of the fail-closed feature-resolution path).
        assert_eq!(
            outcome
                .report
                .skipped
                .details
                .iter()
                .filter(|detail| detail
                    .message
                    .contains("no VALUE FX targets exposing PRESET type 4"))
                .count(),
            2,
            "laser-targeted VALUE scenes without a Dimmer preset must stay fail-closed"
        );
        let recipes = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter_map(|target| match target.params.as_ref() {
                // VALUE FX serializes as EffectParamsSnapshot::Value; the
                // generator recipe lives on its spatial_pattern.
                Some(EffectParamsSnapshot::Value(request)) => request
                    .spatial_pattern
                    .as_ref()
                    .map(|pattern| &pattern.recipe),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            recipes.iter().any(|recipe| matches!(
                recipe,
                ColorEffectSpatialRecipe::Sweep {
                    direction_change: true,
                    ..
                }
            )),
            "saved VALUE Sweep specimen (ID625) must import as Sweep with direction_change=true; found {recipes:?}"
        );
        assert!(
            recipes.iter().any(|recipe| matches!(
                recipe,
                ColorEffectSpatialRecipe::Plasma {
                    grayscale: false,
                    vertical_symmetry: false,
                    size_x,
                    param_x,
                    size_y,
                    param_y,
                    speed_x,
                    param_sx,
                    speed_y,
                    param_sy,
                } if *size_x == 1.0
                    && *param_x == 2.0
                    && *size_y == 1.0
                    && *param_y == 2.0
                    && *speed_x == -1.0
                    && *param_sx == 2.0
                    && *speed_y == 1.0
                    && *param_sy == -1.0
            )),
            "saved VALUE Plasma specimen (ID623) must import with Daslight default fields; found {recipes:?}"
        );
    }

    #[test]
    fn dvc_local_golden_value_burst_knight_rider_and_sweep_import_from_saved_specimen() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ValueCatalog-Burst-KnightRider.dvc"
        ));
        assert!(path.is_file(), "repo-portable VALUE specimen is missing");
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();

        let values = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Value(request)) => Some(request),
                _ => None,
            })
            .collect::<Vec<_>>();
        let burst = values
            .iter()
            .find(|request| {
                matches!(
                    request
                        .spatial_pattern
                        .as_ref()
                        .map(|pattern| &pattern.recipe),
                    Some(ColorEffectSpatialRecipe::Burst { .. })
                )
            })
            .expect("saved ID622 Burst must import");
        let burst_pattern = burst.spatial_pattern.as_ref().unwrap();
        let burst_strip_count = burst_pattern
            .beam_targets
            .iter()
            .map(|target| target.selection_index)
            .collect::<HashSet<_>>()
            .len()
            .max(1) as f32;
        assert!(matches!(
            &burst_pattern.recipe,
            ColorEffectSpatialRecipe::Burst {
                grayscale: false,
                vertical_symmetry: false,
                color_width,
                gradient,
            } if (*color_width - 5_000.0 / burst_strip_count).abs() <= f32::EPSILON
                && *gradient == 100.0
        ));

        let knight = values
            .iter()
            .find(|request| {
                matches!(
                    request
                        .spatial_pattern
                        .as_ref()
                        .map(|pattern| &pattern.recipe),
                    Some(ColorEffectSpatialRecipe::KnightRider { .. })
                )
            })
            .expect("saved ID624 Knight Rider must import");
        let knight_pattern = knight.spatial_pattern.as_ref().unwrap();
        let knight_strip_count = knight_pattern
            .beam_targets
            .iter()
            .map(|target| target.selection_index)
            .collect::<HashSet<_>>()
            .len()
            .max(1) as f32;
        assert!(matches!(
            &knight_pattern.recipe,
            ColorEffectSpatialRecipe::KnightRider {
                grayscale: false,
                vertical_symmetry: false,
                size,
                one_way: false,
                fading: true,
                go_outside: false,
                gradient,
            } if (*size - 100.0 / knight_strip_count).abs() <= f32::EPSILON
                && *gradient == 50.0
        ));

        let sweep = values
            .iter()
            .find(|request| {
                matches!(
                    request
                        .spatial_pattern
                        .as_ref()
                        .map(|pattern| &pattern.recipe),
                    Some(ColorEffectSpatialRecipe::Sweep {
                        vertical_symmetry: true,
                        direction_change: false,
                        ..
                    })
                )
            })
            .expect("saved ID625 Sweep must import");
        assert!(matches!(
            sweep
                .spatial_pattern
                .as_ref()
                .map(|pattern| &pattern.recipe),
            Some(ColorEffectSpatialRecipe::Sweep {
                grayscale: false,
                vertical_symmetry: true,
                direction_change: false,
            })
        ));

        for request in [burst, knight, sweep] {
            let targets = &request
                .spatial_pattern
                .as_ref()
                .expect("corrected VALUE recipe must retain spatial targeting")
                .beam_targets;
            assert!(
                !targets.is_empty(),
                "dual PRESET binding must retain beam targets"
            );
            assert!(targets
                .iter()
                .all(|target| target.feature_attribute.as_deref() == Some("Dimmer")));
        }
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.message.contains(
                "profile PRESET 69bdd010-d626-11ea-b9df-7da99bfefe5c-3afb4fbb:33:0 -> type 4",
            ) && detail.message.contains("generic PRESET type 4")
        }));
    }

    #[test]
    fn dvc_local_golden_color_mappings_rain_from_remaining7_specimen() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ColorMappings-Remaining7.dvc"
        ));
        let source = fs::read_to_string(path).unwrap();
        let document = Document::parse(&source).unwrap();
        let rack = document
            .descendants()
            .find(|node| {
                node.has_tag_name("RACK")
                    && node.attribute("TYPE") == Some("5")
                    && direct_child(*node, "EFFECT")
                        .is_some_and(|effect| effect.attribute("ID") == Some("35"))
            })
            .expect("Remaining7 must contain Rain ID=35");
        let effect = direct_child(rack, "EFFECT").unwrap();
        let params = direct_child(effect, "PARAMS").unwrap();
        assert_eq!(params.attribute("NB"), Some("9"));
        assert_eq!(
            element_children(params)
                .map(|param| (
                    param.attribute("TYPE").unwrap(),
                    param.attribute("ID").unwrap(),
                    param.attribute("VAL"),
                ))
                .collect::<Vec<_>>(),
            vec![
                ("4", "1", None),
                ("2", "2", Some("0")),
                ("6", "3", Some("0")),
                ("0", "4", Some("0")),
                ("0", "13", Some("50")),
                ("0", "10", Some("1")),
                ("0", "11", Some("5")),
                ("0", "12", Some("10")),
                ("0", "14", Some("10")),
            ]
        );
        let mapping = direct_child(rack, "MAPPING").unwrap();
        assert_eq!(
            (
                mapping.attribute("X"),
                mapping.attribute("Y"),
                mapping.attribute("SX"),
                mapping.attribute("SY"),
                mapping.attribute("ANGLE"),
            ),
            (Some("0"), Some("0"), Some("-1"), Some("-1"), Some("0"))
        );
        let beams = direct_child(rack, "BEAMS").unwrap();
        assert_eq!(beams.attribute("NB"), Some("0"));
        assert_eq!(element_children(beams).count(), 0);
        let scene = rack
            .ancestors()
            .find(|node| node.has_tag_name("SCENE"))
            .unwrap();
        let converted = convert_dvc_effect(
            scene,
            "Remaining7 Rain",
            rack,
            effect,
            5,
            3,
            35,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        assert!(converted.target.is_none());
        assert!(converted.note.contains("source no-op preserved"));
        assert!(converted.note.contains("Rectangle=(0,0,-1,-1,0)"));
        assert!(converted.note.contains("evaluator=CRainEffect/0x140365660"));
        assert!(converted.approximations.is_empty());
    }

    #[test]
    fn dvc_local_golden_color_mappings_bounce_from_remaining7_specimen() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ColorMappings-Remaining7.dvc"
        ));
        let source = fs::read_to_string(path).unwrap();
        assert_eq!(source.as_bytes().len(), 88_780);
        let document = Document::parse(&source).unwrap();
        let rack = document
            .descendants()
            .find(|node| {
                node.has_tag_name("RACK")
                    && node.attribute("TYPE") == Some("5")
                    && direct_child(*node, "EFFECT")
                        .is_some_and(|effect| effect.attribute("ID") == Some("21"))
            })
            .expect("Remaining7 must contain Bounce ID=21");
        let effect = direct_child(rack, "EFFECT").unwrap();
        let params = direct_child(effect, "PARAMS").unwrap();
        assert_eq!(params.attribute("NB"), Some("12"));
        assert_eq!(
            element_children(params)
                .map(|param| (
                    param.attribute("TYPE").unwrap(),
                    param.attribute("ID").unwrap(),
                    param.attribute("VAL"),
                ))
                .collect::<Vec<_>>(),
            vec![
                ("4", "1", None),
                ("2", "2", Some("0")),
                ("6", "3", Some("0")),
                ("0", "4", Some("0")),
                ("6", "10", Some("0")),
                ("7", "11", Some("0")),
                ("0", "12", Some("6")),
                ("0", "13", Some("40")),
                ("0", "14", Some("1")),
                ("2", "16", Some("0")),
                ("2", "17", Some("0")),
                ("0", "18", Some("3")),
            ]
        );
        let palette = direct_child(
            element_children(params)
                .find(|param| param.attribute("ID") == Some("1"))
                .unwrap(),
            "COLORS",
        )
        .unwrap();
        assert_eq!(palette.attribute("NB"), Some("7"));
        let mapping = direct_child(rack, "MAPPING").unwrap();
        assert_eq!(
            (
                mapping.attribute("X"),
                mapping.attribute("Y"),
                mapping.attribute("SX"),
                mapping.attribute("SY"),
                mapping.attribute("ANGLE"),
            ),
            (Some("0"), Some("0"), Some("-1"), Some("-1"), Some("0"))
        );
        let beams = direct_child(rack, "BEAMS").unwrap();
        assert_eq!(beams.attribute("NB"), Some("0"));
        assert_eq!(element_children(beams).count(), 0);
        let scene = rack
            .ancestors()
            .find(|node| node.has_tag_name("SCENE"))
            .unwrap();
        let seed = dvc_corrected_rng_seed(scene, rack, effect, 21);
        assert_ne!(seed, 0);
        let converted = convert_dvc_effect(
            scene,
            "Remaining7 Bounce",
            rack,
            effect,
            5,
            3,
            21,
            77,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        assert!(converted.target.is_none());
        assert!(converted.note.contains("source no-op preserved"));
        assert!(converted.note.contains("Rectangle=(0,0,-1,-1,0)"));
        assert!(converted.note.contains("Item=Shape"));
        assert!(converted.note.contains("Shape=0"));
        assert!(converted.note.contains(&format!("rng_seed={seed}")));
        assert!(converted.approximations.is_empty());

        let mut report = DvcImportReport::new(path.to_str().unwrap(), document.root_element());
        let mut next_effect_id = 1;
        let parsed = parse_scene_effects(
            scene,
            scene.attribute("NAME").unwrap_or("Remaining7"),
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
            &mut next_effect_id,
            &mut report,
        );
        assert!(parsed.targets.is_empty());
        assert_eq!(
            next_effect_id, 1,
            "source no-op must allocate no runtime effect ID"
        );
        assert!(report.converted.details.iter().any(|detail| {
            detail.item.ends_with("(Bounce)")
                && detail.message.contains("source no-op preserved")
                && detail.message.contains("implementation=SyndocalCorrected")
        }));
        assert!(!report.skipped.details.iter().any(|detail| {
            detail.item.ends_with("(Bounce)") && detail.message.contains("not confirmed")
        }));
    }

    #[test]
    fn dvc_color_mappings_text_source_noop_is_strict_and_populated_text_fails_closed() {
        const COLOR: &str = r#"<COLOR VAL="0/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/>"#;
        const PALETTE_2: &str = r#"<PARAM TYPE="4" ID="1"><COLORS NB="2"><COLOR VAL="0/0/0/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/><COLOR VAL="1/1/1/0/0/0/0/0/1/1/1/1/0/0/0/0/0/1"/></COLORS></PARAM>"#;
        const CLASS_PARAMS: &str = r#"<PARAM TYPE="2" ID="2" VAL="0"/><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="0" ID="4" VAL="0"/><PARAM TYPE="3" ID="10" VAL="Text"/><PARAM TYPE="0" ID="11" VAL="8"/><PARAM TYPE="2" ID="12" VAL="0"/><PARAM TYPE="10" ID="13" VAL="1"/><PARAM TYPE="2" ID="14" VAL="0"/><PARAM TYPE="0" ID="15" VAL="0"/><PARAM TYPE="0" ID="16" VAL="0"/>"#;
        const REORDERED_CLASS_PARAMS: &str = r#"<PARAM TYPE="0" ID="16" VAL="0"/><PARAM TYPE="0" ID="11" VAL="8"/><PARAM TYPE="3" ID="10" VAL="Text"/><PARAM TYPE="2" ID="14" VAL="0"/><PARAM TYPE="0" ID="15" VAL="0"/><PARAM TYPE="10" ID="13" VAL="1"/><PARAM TYPE="2" ID="12" VAL="0"/><PARAM TYPE="0" ID="4" VAL="0"/><PARAM TYPE="6" ID="3" VAL="0"/><PARAM TYPE="2" ID="2" VAL="0"/>"#;
        const POSITIVE_MAPPING: &str = r#"<MAPPING NAME="Rectangle" DASUID="text-mapping" TYPE="0" X="0" Y="0" SX="100" SY="100" ANGLE="0" LOCKED="0"/>"#;
        const SENTINEL_MAPPING: &str = r#"<MAPPING NAME="Rectangle" DASUID="text-mapping" TYPE="0" X="0" Y="0" SX="-1" SY="-1" ANGLE="0" LOCKED="0"/>"#;
        const EMPTY_BEAMS: &str = r#"<BEAMS NB="0"/>"#;
        const POPULATED_BEAMS: &str =
            r#"<BEAMS NB="1"><BEAM FIXTURE="fixture-1" BEAMID="0" IDSELECTION="1"/></BEAMS>"#;

        let source = |palette: &str, class_params: &str, mapping: &str, beams: &str| {
            format!(
                r#"<SCENE SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><RACKS><RACK EXPAND_RACK="1" TYPE="5"><EFFECT TYPE="3" ID="45" DURATION="5000"><PARAMS NB="11">{palette}{class_params}</PARAMS></EFFECT>{mapping}{beams}</RACK></RACKS></SCENE>"#
            )
        };
        let convert = |xml: &str| {
            let document = Document::parse(xml).unwrap();
            let scene = document.root_element();
            let rack = direct_child(direct_child(scene, "RACKS").unwrap(), "RACK").unwrap();
            let effect = direct_child(rack, "EFFECT").unwrap();
            convert_dvc_effect(
                scene,
                "Text source boundary",
                rack,
                effect,
                5,
                3,
                45,
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
        };

        let baseline = source(PALETTE_2, CLASS_PARAMS, POSITIVE_MAPPING, EMPTY_BEAMS);
        let converted = convert(&baseline).unwrap();
        assert!(converted.target.is_none());
        assert!(converted.approximations.is_empty());
        assert!(converted
            .note
            .contains("implementation=SourceExactBoundary"));
        assert!(converted.note.contains("source no-op preserved"));
        assert!(converted.note.contains("TextEmpty=0"));
        assert!(converted.note.contains("Anti Alias=0"));
        assert!(converted.note.contains("Write Vertically=0"));
        assert!(convert(&source(
            PALETTE_2,
            REORDERED_CLASS_PARAMS,
            SENTINEL_MAPPING,
            EMPTY_BEAMS
        ))
        .is_ok());
        assert!(convert(&baseline.replace(r#"VAL="Text""#, r#"VAL="""#)).is_ok());
        let long_unicode = "照明".repeat(2049);
        assert!(convert(&baseline.replace("Text", &long_unicode)).is_ok());
        assert!(convert(
            &source(PALETTE_2, CLASS_PARAMS, SENTINEL_MAPPING, EMPTY_BEAMS).replacen(
                r#"ID="11" VAL="8""#,
                r#"ID="11" VAL="81""#,
                1
            )
        )
        .is_err());

        let palette_1 =
            format!(r#"<PARAM TYPE="4" ID="1"><COLORS NB="1">{COLOR}</COLORS></PARAM>"#);
        let palette_256 = format!(
            r#"<PARAM TYPE="4" ID="1"><COLORS NB="256">{}</COLORS></PARAM>"#,
            COLOR.repeat(256)
        );
        let palette_255 = format!(
            r#"<PARAM TYPE="4" ID="1"><COLORS NB="255">{}</COLORS></PARAM>"#,
            COLOR.repeat(255)
        );
        let maximum_params = CLASS_PARAMS
            .replace(r#"ID="2" VAL="0""#, r#"ID="2" VAL="1""#)
            .replace(r#"ID="3" VAL="0""#, r#"ID="3" VAL="2""#)
            .replace(r#"ID="4" VAL="0""#, r#"ID="4" VAL="360""#)
            .replace(r#"ID="11" VAL="8""#, r#"ID="11" VAL="80""#)
            .replace(r#"ID="12" VAL="0""#, r#"ID="12" VAL="1""#)
            .replace(r#"ID="13" VAL="1""#, r#"ID="13" VAL="8""#)
            .replace(r#"ID="14" VAL="0""#, r#"ID="14" VAL="1""#)
            .replace(r#"ID="15" VAL="0""#, r#"ID="15" VAL="100""#)
            .replace(r#"ID="16" VAL="0""#, r#"ID="16" VAL="100""#);
        assert!(convert(&source(
            &palette_255,
            &maximum_params,
            POSITIVE_MAPPING,
            EMPTY_BEAMS
        ))
        .is_ok());
        let minimum_params = CLASS_PARAMS
            .replace(r#"ID="11" VAL="8""#, r#"ID="11" VAL="5""#)
            .replace(r#"ID="13" VAL="1""#, r#"ID="13" VAL="0""#)
            .replace(r#"ID="15" VAL="0""#, r#"ID="15" VAL="-100""#)
            .replace(r#"ID="16" VAL="0""#, r#"ID="16" VAL="-100""#);
        assert!(convert(&source(
            PALETTE_2,
            &minimum_params,
            SENTINEL_MAPPING,
            EMPTY_BEAMS
        ))
        .is_ok());
        let invalid = vec![
            (
                "NB",
                baseline.replacen(r#"PARAMS NB="11""#, r#"PARAMS NB="10""#, 1),
            ),
            ("wrong ID", baseline.replacen(r#"ID="16""#, r#"ID="17""#, 1)),
            (
                "duplicate ID",
                baseline.replacen(r#"ID="16""#, r#"ID="15""#, 1),
            ),
            (
                "wrong Text TYPE",
                baseline.replacen(r#"TYPE="3" ID="10""#, r#"TYPE="0" ID="10""#, 1),
            ),
            (
                "missing Text TYPE",
                baseline.replacen(r#"TYPE="3" ID="10""#, r#"ID="10""#, 1),
            ),
            (
                "missing Text VAL",
                baseline.replacen(r#" ID="10" VAL="Text""#, r#" ID="10""#, 1),
            ),
            (
                "palette 1",
                source(&palette_1, CLASS_PARAMS, POSITIVE_MAPPING, EMPTY_BEAMS),
            ),
            (
                "palette 256",
                source(&palette_256, CLASS_PARAMS, POSITIVE_MAPPING, EMPTY_BEAMS),
            ),
            (
                "size 4",
                baseline.replacen(r#"ID="11" VAL="8""#, r#"ID="11" VAL="4""#, 1),
            ),
            (
                "size 81",
                baseline.replacen(r#"ID="11" VAL="8""#, r#"ID="11" VAL="81""#, 1),
            ),
            (
                "direction -1",
                baseline.replacen(r#"ID="13" VAL="1""#, r#"ID="13" VAL="-1""#, 1),
            ),
            (
                "direction 9",
                baseline.replacen(r#"ID="13" VAL="1""#, r#"ID="13" VAL="9""#, 1),
            ),
            (
                "vertical offset -101",
                baseline.replacen(r#"ID="15" VAL="0""#, r#"ID="15" VAL="-101""#, 1),
            ),
            (
                "vertical offset 101",
                baseline.replacen(r#"ID="15" VAL="0""#, r#"ID="15" VAL="101""#, 1),
            ),
            (
                "horizontal offset -101",
                baseline.replacen(r#"ID="16" VAL="0""#, r#"ID="16" VAL="-101""#, 1),
            ),
            (
                "horizontal offset 101",
                baseline.replacen(r#"ID="16" VAL="0""#, r#"ID="16" VAL="101""#, 1),
            ),
            (
                "grayscale",
                baseline.replacen(r#"ID="2" VAL="0""#, r#"ID="2" VAL="2""#, 1),
            ),
            (
                "anti alias",
                baseline.replacen(r#"ID="12" VAL="0""#, r#"ID="12" VAL="2""#, 1),
            ),
            (
                "vertical",
                baseline.replacen(r#"ID="14" VAL="0""#, r#"ID="14" VAL="2""#, 1),
            ),
            (
                "transform",
                baseline.replacen(r#"ID="3" VAL="0""#, r#"ID="3" VAL="3""#, 1),
            ),
            (
                "rotation negative",
                baseline.replacen(r#"ID="4" VAL="0""#, r#"ID="4" VAL="-1""#, 1),
            ),
            (
                "rotation high",
                baseline.replacen(r#"ID="4" VAL="0""#, r#"ID="4" VAL="361""#, 1),
            ),
            (
                "nonfinite numeric",
                baseline.replacen(r#"ID="11" VAL="8""#, r#"ID="11" VAL="NaN""#, 1),
            ),
            (
                "missing mapping",
                source(PALETTE_2, CLASS_PARAMS, "", EMPTY_BEAMS),
            ),
            (
                "duplicate mapping",
                source(
                    PALETTE_2,
                    CLASS_PARAMS,
                    &format!("{POSITIVE_MAPPING}{POSITIVE_MAPPING}"),
                    EMPTY_BEAMS,
                ),
            ),
            (
                "mapping attrs",
                baseline.replacen(r#"NAME="Rectangle""#, r#"NAME="Rectangle" EXTRA="1""#, 1),
            ),
            (
                "invalid rectangle",
                baseline.replacen(r#"SX="100""#, r#"SX="0""#, 1),
            ),
            (
                "missing beams",
                source(PALETTE_2, CLASS_PARAMS, POSITIVE_MAPPING, ""),
            ),
            (
                "beams NB",
                baseline.replacen(r#"BEAMS NB="0""#, r#"BEAMS NB="1""#, 1),
            ),
            (
                "beams body",
                baseline.replacen(r#"<BEAMS NB="0"/>"#, r#"<BEAMS NB="1"><NOPE/></BEAMS>"#, 1),
            ),
            (
                "duplicate beams",
                source(
                    PALETTE_2,
                    CLASS_PARAMS,
                    POSITIVE_MAPPING,
                    &format!("{EMPTY_BEAMS}{EMPTY_BEAMS}"),
                ),
            ),
            (
                "external selections",
                baseline.replacen(EMPTY_BEAMS, &format!("<SELECTIONS/>{EMPTY_BEAMS}"), 1),
            ),
            (
                "duration missing",
                baseline.replacen(r#" DURATION="5000""#, "", 1),
            ),
            (
                "duration zero",
                baseline.replacen(r#"DURATION="5000""#, r#"DURATION="0""#, 1),
            ),
            (
                "duration fractional",
                baseline.replacen(r#"DURATION="5000""#, r#"DURATION="1.5""#, 1),
            ),
            (
                "rack attrs",
                baseline.replacen(
                    r#"EXPAND_RACK="1" TYPE="5""#,
                    r#"EXPAND_RACK="1" TYPE="5" EXTRA="1""#,
                    1,
                ),
            ),
            (
                "rack expand value",
                baseline.replacen(r#"EXPAND_RACK="1""#, r#"EXPAND_RACK="0""#, 1),
            ),
            (
                "rack type value",
                baseline.replacen(r#"TYPE="5""#, r#"TYPE="6""#, 1),
            ),
            (
                "effect attrs",
                baseline.replacen(
                    r#"ID="45" DURATION="5000""#,
                    r#"ID="45" DURATION="5000" EXTRA="1""#,
                    1,
                ),
            ),
            (
                "effect type value",
                baseline.replacen(r#"EFFECT TYPE="3""#, r#"EFFECT TYPE="4""#, 1),
            ),
            (
                "effect ID value",
                baseline.replacen(r#"ID="45""#, r#"ID="44""#, 1),
            ),
            (
                "params attrs",
                baseline.replacen(r#"PARAMS NB="11""#, r#"PARAMS NB="11" EXTRA="1""#, 1),
            ),
            (
                "param attrs",
                baseline.replacen(
                    r#"TYPE="3" ID="10" VAL="Text""#,
                    r#"TYPE="3" ID="10" VAL="Text" EXTRA="1""#,
                    1,
                ),
            ),
        ];
        for (case, xml) in invalid {
            assert!(
                convert(&xml).is_err(),
                "Text source accepted invalid {case}"
            );
        }

        let populated_error = convert(&source(
            PALETTE_2,
            CLASS_PARAMS,
            POSITIVE_MAPPING,
            POPULATED_BEAMS,
        ))
        .unwrap_err();
        assert_eq!(
            populated_error,
            "COLOR MAPPINGS Text ID=45 populated Text requires unresolved deterministic font and supported text-raster envelope"
        );
        let sentinel_error = convert(&source(
            PALETTE_2,
            CLASS_PARAMS,
            SENTINEL_MAPPING,
            POPULATED_BEAMS,
        ))
        .unwrap_err();
        assert!(sentinel_error
            .contains("native empty Rectangle sentinel is valid only with zero owned BEAMS"));
    }

    #[test]
    fn dvc_local_golden_color_mappings_text_from_remaining7_specimen() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ColorMappings-Remaining7.dvc"
        ));
        let source = fs::read_to_string(path).unwrap();
        let document = Document::parse(&source).unwrap();
        let rack = document
            .descendants()
            .find(|node| {
                node.has_tag_name("RACK")
                    && node.attribute("TYPE") == Some("5")
                    && direct_child(*node, "EFFECT")
                        .is_some_and(|effect| effect.attribute("ID") == Some("45"))
            })
            .expect("Remaining7 must contain Text ID=45");
        assert_eq!(rack.attribute("EXPAND_RACK"), Some("1"));
        let effect = direct_child(rack, "EFFECT").unwrap();
        let params = direct_child(effect, "PARAMS").unwrap();
        assert_eq!(params.attribute("NB"), Some("11"));
        assert_eq!(
            element_children(params)
                .map(|param| (
                    param.attribute("TYPE").unwrap(),
                    param.attribute("ID").unwrap(),
                    param.attribute("VAL"),
                ))
                .collect::<Vec<_>>(),
            vec![
                ("4", "1", None),
                ("2", "2", Some("0")),
                ("6", "3", Some("0")),
                ("0", "4", Some("0")),
                ("3", "10", Some("Text")),
                ("0", "11", Some("8")),
                ("2", "12", Some("0")),
                ("10", "13", Some("1")),
                ("2", "14", Some("0")),
                ("0", "15", Some("0")),
                ("0", "16", Some("0")),
            ]
        );
        let mapping = direct_child(rack, "MAPPING").unwrap();
        assert_eq!(
            (
                mapping.attribute("NAME"),
                mapping.attribute("X"),
                mapping.attribute("Y"),
                mapping.attribute("SX"),
                mapping.attribute("SY"),
                mapping.attribute("ANGLE"),
                mapping.attribute("LOCKED"),
            ),
            (
                Some("Rectangle"),
                Some("0"),
                Some("0"),
                Some("-1"),
                Some("-1"),
                Some("0"),
                Some("0"),
            )
        );
        let beams = direct_child(rack, "BEAMS").unwrap();
        assert_eq!(beams.attribute("NB"), Some("0"));
        assert_eq!(element_children(beams).count(), 0);

        let scene = rack
            .ancestors()
            .find(|node| node.has_tag_name("SCENE"))
            .unwrap();
        let converted = convert_dvc_effect(
            scene,
            "Remaining7 Text",
            rack,
            effect,
            5,
            3,
            45,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        assert!(converted.target.is_none());
        assert!(converted.note.contains("source no-op preserved"));
        assert!(converted.note.contains("TextEmpty=0"));
        assert!(converted.note.contains("Anti Alias=0"));
        assert!(converted.note.contains("Write Vertically=0"));
        assert!(converted.note.contains("Vertical Offset=0"));
        assert!(converted.note.contains("Horizontal Offset=0"));
        assert!(converted.note.contains("Rectangle=(0,0,-1,-1,0)"));
        assert!(converted.approximations.is_empty());

        let mut report = DvcImportReport::new(path.to_str().unwrap(), document.root_element());
        let mut next_effect_id = 1;
        let parsed = parse_scene_effects(
            scene,
            scene.attribute("NAME").unwrap_or("Remaining7"),
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
            &mut next_effect_id,
            &mut report,
        );
        assert!(parsed.targets.is_empty());
        assert_eq!(next_effect_id, 1);
        assert!(report.converted.details.iter().any(|detail| {
            detail.item.ends_with("(Text)")
                && detail
                    .message
                    .contains("implementation=SourceExactBoundary")
                && detail.message.contains("source no-op preserved")
        }));
        assert!(!report.skipped.details.iter().any(|detail| {
            detail.item.ends_with("(Text)") && detail.message.contains("not confirmed for DVC-3b")
        }));
    }

    #[test]
    fn dvc_local_golden_color_mappings_fire_from_remaining7_specimen() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ColorMappings-Remaining7.dvc"
        ));
        let source = fs::read_to_string(path).unwrap();
        let document = Document::parse(&source).unwrap();
        let rack = document
            .descendants()
            .find(|node| {
                node.has_tag_name("RACK")
                    && node.attribute("TYPE") == Some("5")
                    && direct_child(*node, "EFFECT")
                        .is_some_and(|effect| effect.attribute("ID") == Some("29"))
            })
            .expect("Remaining7 must contain Fire ID=29");
        let effect = direct_child(rack, "EFFECT").unwrap();
        let params = direct_child(effect, "PARAMS").unwrap();
        assert_eq!(params.attribute("NB"), Some("8"));
        assert_eq!(
            element_children(params)
                .map(|param| (
                    param.attribute("TYPE").unwrap(),
                    param.attribute("ID").unwrap(),
                    param.attribute("VAL"),
                ))
                .collect::<Vec<_>>(),
            vec![
                ("4", "1", None),
                ("2", "2", Some("0")),
                ("6", "3", Some("0")),
                ("0", "4", Some("0")),
                ("0", "10", Some("20")),
                ("0", "11", Some("20")),
                ("0", "12", Some("50")),
                ("0", "13", Some("250")),
            ]
        );
        let palette = element_children(
            element_children(params)
                .find(|param| param.attribute("ID") == Some("1"))
                .unwrap(),
        )
        .find(|node| node.has_tag_name("COLORS"))
        .unwrap();
        assert_eq!(palette.attribute("NB"), Some("4"));
        let mapping = direct_child(rack, "MAPPING").unwrap();
        assert_eq!(
            (
                mapping.attribute("X"),
                mapping.attribute("Y"),
                mapping.attribute("SX"),
                mapping.attribute("SY"),
                mapping.attribute("ANGLE"),
            ),
            (Some("0"), Some("0"), Some("-1"), Some("-1"), Some("0"))
        );
        let beams = direct_child(rack, "BEAMS").unwrap();
        assert_eq!(beams.attribute("NB"), Some("0"));
        assert_eq!(element_children(beams).count(), 0);
        let scene = rack
            .ancestors()
            .find(|node| node.has_tag_name("SCENE"))
            .unwrap();
        assert_eq!(
            dvc_corrected_rng_seed(scene, rack, effect, 29),
            0xDCE5_15F0,
            "Remaining7 Fire identity seed must stay stable"
        );
        let converted = convert_dvc_effect(
            scene,
            "Remaining7 Fire",
            rack,
            effect,
            5,
            3,
            29,
            1,
            &effect_test_profiles(),
            &effect_test_fixture_refs(),
        )
        .unwrap();
        assert!(converted.target.is_none());
        assert!(converted.note.contains("source no-op preserved"));
        assert!(converted.note.contains("Rectangle=(0,0,-1,-1,0)"));
        assert!(converted.note.contains("evaluator=CFireEffect"));
        assert!(converted.note.contains("rng_seed=3706000880"));
        assert!(converted.approximations.is_empty());
    }

    #[test]
    fn dvc_local_golden_color_mappings_explosion_starfield_from_remaining7_specimen() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ColorMappings-Remaining7.dvc"
        ));
        assert!(
            path.is_file(),
            "particle specimen must exist at {}",
            path.display()
        );
        let source = fs::read_to_string(path).unwrap();
        let document = Document::parse(&source).unwrap();
        for (generator_id, expected) in [
            (
                "47",
                vec![
                    ("4", "1", None),
                    ("2", "2", Some("0")),
                    ("6", "3", Some("0")),
                    ("0", "4", Some("0")),
                    ("7", "10", Some("0")),
                    ("0", "11", Some("5")),
                    ("0", "12", Some("5")),
                    ("0", "13", Some("10")),
                    ("0", "14", Some("10")),
                    ("1", "15", Some("0")),
                    ("0", "16", Some("10")),
                    ("1", "17", Some("0")),
                ],
            ),
            (
                "48",
                vec![
                    ("4", "1", None),
                    ("2", "2", Some("0")),
                    ("6", "3", Some("0")),
                    ("0", "4", Some("0")),
                    ("7", "10", Some("0")),
                    ("0", "11", Some("1")),
                    ("0", "12", Some("10")),
                    ("0", "13", Some("10")),
                    ("1", "14", Some("0")),
                ],
            ),
        ] {
            let rack = document
                .descendants()
                .find(|node| {
                    node.has_tag_name("RACK")
                        && node.attribute("TYPE") == Some("5")
                        && direct_child(*node, "EFFECT")
                            .is_some_and(|effect| effect.attribute("ID") == Some(generator_id))
                })
                .unwrap_or_else(|| panic!("Remaining7 must contain ID={generator_id}"));
            let scene = rack
                .ancestors()
                .find(|node| node.has_tag_name("SCENE"))
                .expect("particle rack must belong to a SCENE");
            let effect = direct_child(rack, "EFFECT").unwrap();
            let params = direct_child(effect, "PARAMS").unwrap();
            assert_eq!(
                params.attribute("NB").unwrap().parse::<usize>().unwrap(),
                expected.len()
            );
            assert_eq!(
                element_children(params)
                    .map(|param| (
                        param.attribute("TYPE").unwrap(),
                        param.attribute("ID").unwrap(),
                        param.attribute("VAL")
                    ))
                    .collect::<Vec<_>>(),
                expected
            );
            let mapping = direct_child(rack, "MAPPING").unwrap();
            assert_eq!(
                (
                    mapping.attribute("NAME"),
                    mapping.attribute("TYPE"),
                    mapping.attribute("X"),
                    mapping.attribute("Y"),
                    mapping.attribute("SX"),
                    mapping.attribute("SY"),
                    mapping.attribute("ANGLE"),
                    mapping.attribute("LOCKED"),
                ),
                (
                    Some("Rectangle"),
                    Some("0"),
                    Some("0"),
                    Some("0"),
                    Some("-1"),
                    Some("-1"),
                    Some("0"),
                    Some("0"),
                )
            );
            let beams = direct_child(rack, "BEAMS").unwrap();
            assert_eq!(beams.attribute("NB"), Some("0"));
            assert_eq!(element_children(beams).count(), 0);
            assert_eq!(
                element_children(rack)
                    .filter(|node| node.has_tag_name("SELECTIONS"))
                    .count(),
                0
            );
            let converted = convert_dvc_effect(
                scene,
                "Remaining7 particles",
                rack,
                effect,
                5,
                3,
                generator_id.parse().unwrap(),
                1,
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
            )
            .unwrap();
            assert!(
                converted.target.is_none(),
                "saved empty rack allocates no runtime effect"
            );
            assert!(converted.note.contains("source no-op preserved"));
            assert!(converted.note.contains("Rectangle=(0,0,-1,-1,0)"));
            assert!(converted.approximations.is_empty());
            let report_path = path.display().to_string();
            let mut report = DvcImportReport::new(&report_path, document.root_element());
            let mut next_effect_id = 1;
            let parsed = parse_scene_effects(
                scene,
                scene.attribute("NAME").unwrap_or("Remaining7"),
                &effect_test_profiles(),
                &effect_test_fixture_refs(),
                &mut next_effect_id,
                &mut report,
            );
            assert!(parsed.targets.is_empty());
            assert_eq!(next_effect_id, 1, "source no-ops allocate no runtime IDs");
            let generator = if generator_id == "47" {
                "Explosion"
            } else {
                "Starfield"
            };
            assert!(report.converted.details.iter().any(|detail| {
                detail.item.ends_with(&format!("({generator})"))
                    && detail.message.contains("source no-op preserved")
            }));
        }
    }

    #[test]
    fn dvc_local_golden_color_mappings_tube_from_remaining7_specimen() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ColorMappings-Remaining7.dvc"
        ));
        assert!(
            path.is_file(),
            "repo-portable Tube specimen must exist at {}",
            path.display()
        );

        let source = fs::read_to_string(path).unwrap();
        let document = Document::parse(&source).unwrap();
        let rack = document
            .descendants()
            .find(|node| {
                node.has_tag_name("RACK")
                    && node.attribute("TYPE") == Some("5")
                    && direct_child(*node, "EFFECT").is_some_and(|effect| {
                        effect.attribute("TYPE") == Some("3")
                            && effect.attribute("ID") == Some("41")
                    })
            })
            .expect("saved specimen must contain COLOR MAPPINGS 5/3/41 Tube");
        let effect = direct_child(rack, "EFFECT").expect("Tube must contain EFFECT");
        let params = direct_child(effect, "PARAMS").expect("Tube must contain PARAMS");
        assert_eq!(params.attribute("NB"), Some("7"));
        assert_eq!(
            element_children(params)
                .filter(|param| param.has_tag_name("PARAM"))
                .map(|param| {
                    (
                        param.attribute("TYPE").unwrap(),
                        param.attribute("ID").unwrap(),
                        param.attribute("VAL"),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("4", "1", None),
                ("2", "2", Some("0")),
                ("6", "3", Some("0")),
                ("0", "4", Some("0")),
                ("0", "10", Some("5")),
                ("1", "11", Some("0")),
                ("0", "12", Some("1")),
            ],
            "saved Tube PARAM schema must remain byte-semantic"
        );
        let palette_param = element_children(params)
            .find(|param| param.has_tag_name("PARAM") && param.attribute("ID") == Some("1"))
            .expect("Tube must contain palette PARAM 1");
        let palette =
            direct_child(palette_param, "COLORS").expect("Tube palette must contain COLORS");
        assert_eq!(palette.attribute("NB"), Some("2"));
        assert_eq!(
            element_children(palette)
                .filter(|color| color.has_tag_name("COLOR"))
                .count(),
            2
        );
        let rectangle = direct_child(rack, "MAPPING").expect("Tube must contain Rectangle");
        assert_eq!(rectangle.attribute("NAME"), Some("Rectangle"));
        assert_eq!(rectangle.attribute("TYPE"), Some("0"));
        assert_eq!(rectangle.attribute("X"), Some("1994"));
        assert_eq!(rectangle.attribute("Y"), Some("166"));
        assert_eq!(rectangle.attribute("SX"), Some("513"));
        assert_eq!(rectangle.attribute("SY"), Some("52"));
        assert_eq!(rectangle.attribute("ANGLE"), Some("0"));
        assert_eq!(rectangle.attribute("LOCKED"), Some("0"));
        let beams = direct_child(rack, "BEAMS").expect("Tube must contain BEAMS");
        assert_eq!(beams.attribute("NB"), Some("0"));
        assert_eq!(
            element_children(beams)
                .filter(|beam| beam.has_tag_name("BEAM"))
                .count(),
            0
        );
        assert!(
            direct_child(rack, "SELECTIONS").is_none(),
            "Tube source must not invent external selections"
        );

        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        assert_eq!(
            outcome
                .report
                .converted
                .details
                .iter()
                .filter(|detail| {
                    detail.item == "Effect: New Scene (Tube)"
                        && detail.message.contains("source_family=Color Mappings;")
                        && detail.message.contains("source no-op preserved")
                        && detail.message.contains("palette_colors=2")
                        && detail.message.contains("Rectangle=(1994,166,513,52,0)")
                        && detail
                            .message
                            .contains("evaluator=CTubeEffect shared retained-particle grammar")
                        && detail.message.contains("raster_height=full_100_rows")
                        && detail.message.contains("no runtime effect was created")
                })
                .count(),
            1,
            "saved empty Tube rack must validate as one source no-op"
        );
        assert!(outcome
            .report
            .skipped
            .details
            .iter()
            .all(|detail| { !detail.message.contains("RACK TYPE=5 EFFECT TYPE=3 ID=41") }));
        assert!(outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .all(|target| !matches!(
                target
                    .params
                    .as_ref()
                    .and_then(|params| match params {
                        EffectParamsSnapshot::Color(request) => request.spatial_pattern.as_ref(),
                        _ => None,
                    })
                    .map(|pattern| &pattern.recipe),
                Some(ColorEffectSpatialRecipe::Sparkle {
                    raster_mode: ColorEffectSpatialSparkleRasterMode::TubeFullRasterHeight,
                    ..
                })
            )));
    }

    #[test]
    fn dvc_local_golden_color_mappings_noop_521_placement_and_move_circle_from_saved_specimen() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ValueCatalog-Sweep-Plasma.dvc"
        ));
        assert!(
            path.is_file(),
            "repo-portable Daslight specimen must exist at {}",
            path.display()
        );

        let source = fs::read_to_string(path).unwrap();
        let document = Document::parse(&source).unwrap();
        let color_rack = document
            .descendants()
            .find(|node| {
                node.has_tag_name("RACK")
                    && node.attribute("TYPE") == Some("5")
                    && direct_child(*node, "EFFECT").is_some_and(|effect| {
                        effect.attribute("TYPE") == Some("3")
                            && effect.attribute("ID") == Some("36")
                    })
            })
            .expect("saved specimen must contain COLOR MAPPINGS 5/3/36");
        let color_effect =
            direct_child(color_rack, "EFFECT").expect("COLOR MAPPINGS 5/3/36 must contain EFFECT");
        let color_params = direct_child(color_effect, "PARAMS")
            .expect("COLOR MAPPINGS 5/3/36 must contain PARAMS");
        assert_eq!(color_params.attribute("NB"), Some("7"));
        assert_eq!(
            element_children(color_params)
                .filter(|param| param.has_tag_name("PARAM"))
                .map(|param| {
                    (
                        param.attribute("ID").unwrap(),
                        param.attribute("TYPE").unwrap(),
                        param.attribute("VAL"),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("1", "4", None),
                ("2", "2", Some("0")),
                ("3", "6", Some("0")),
                ("4", "0", Some("0")),
                ("10", "1", Some("0")),
                ("11", "0", Some("0")),
                ("12", "1", Some("1")),
            ],
            "saved COLOR MAPPINGS Rainbow must retain its exact seven PARAM types and values"
        );
        let palette_param = element_children(color_params)
            .find(|param| param.has_tag_name("PARAM") && param.attribute("ID") == Some("1"))
            .expect("COLOR MAPPINGS 5/3/36 must contain palette PARAM 1");
        let palette = direct_child(palette_param, "COLORS")
            .expect("COLOR MAPPINGS palette PARAM 1 must contain COLORS");
        assert_eq!(palette.attribute("NB"), Some("8"));
        assert_eq!(
            element_children(palette)
                .filter(|color| color.has_tag_name("COLOR"))
                .count(),
            8
        );
        let mapping = direct_child(color_rack, "MAPPING")
            .expect("COLOR MAPPINGS 5/3/36 must contain its Rectangle");
        assert_eq!(mapping.attribute("NAME"), Some("Rectangle"));
        assert_eq!(mapping.attribute("TYPE"), Some("0"));
        assert_eq!(mapping.attribute("X"), Some("1994"));
        assert_eq!(mapping.attribute("Y"), Some("166"));
        assert_eq!(mapping.attribute("SX"), Some("513"));
        assert_eq!(mapping.attribute("SY"), Some("52"));
        assert_eq!(mapping.attribute("ANGLE"), Some("0"));
        assert_eq!(mapping.attribute("LOCKED"), Some("0"));
        let color_beams =
            direct_child(color_rack, "BEAMS").expect("COLOR MAPPINGS 5/3/36 must contain BEAMS");
        assert_eq!(color_beams.attribute("NB"), Some("0"));
        assert_eq!(
            element_children(color_beams)
                .filter(|node| node.has_tag_name("BEAM"))
                .count(),
            0,
            "saved family 5 body must retain empty rack BEAMS; Rectangle overlap does not create runtime-owned targets"
        );
        assert_eq!(
            element_children(color_rack)
                .filter(|node| node.has_tag_name("SELECTIONS"))
                .count(),
            0,
            "saved family 5 body has no external SELECTIONS target list"
        );

        let mapping_521_rack = document
            .descendants()
            .find(|node| {
                node.has_tag_name("RACK")
                    && node.attribute("TYPE") == Some("6")
                    && direct_child(*node, "EFFECT").is_some_and(|effect| {
                        effect.attribute("TYPE") == Some("8")
                            && effect.attribute("ID") == Some("521")
                    })
            })
            .expect("saved specimen must contain MAPPINGS 6/8/521");
        let mapping_521 = direct_child(mapping_521_rack, "MAPPING")
            .expect("saved MAPPINGS 521 must contain its Rectangle");
        assert_eq!(mapping_521.attribute("NAME"), Some("Rectangle"));
        assert_eq!(mapping_521.attribute("TYPE"), Some("0"));
        assert_eq!(mapping_521.attribute("X"), Some("2630"));
        assert_eq!(mapping_521.attribute("Y"), Some("-140"));
        assert_eq!(mapping_521.attribute("SX"), Some("140"));
        assert_eq!(mapping_521.attribute("SY"), Some("50"));
        assert_eq!(mapping_521.attribute("ANGLE"), Some("0"));
        assert_eq!(mapping_521.attribute("LOCKED"), Some("0"));
        let mapping_521_fixture_uid = "6dfdc081-800e-4b09-a02e-afd1fcd102c8";
        let mapping_521_beams =
            direct_child(mapping_521_rack, "BEAMS").expect("saved MAPPINGS 521 must contain BEAMS");
        assert_eq!(mapping_521_beams.attribute("NB"), Some("4"));
        assert_eq!(
            element_children(mapping_521_beams)
                .filter(|node| node.has_tag_name("BEAM"))
                .map(|beam| {
                    (
                        beam.attribute("FIXTURE").unwrap(),
                        beam.attribute("BEAMID").unwrap().parse::<u16>().unwrap(),
                        beam.attribute("IDSELECTION")
                            .unwrap()
                            .parse::<u16>()
                            .unwrap(),
                    )
                })
                .collect::<Vec<_>>(),
            (0_u16..=3)
                .map(|beam_index| (mapping_521_fixture_uid, beam_index, beam_index + 1))
                .collect::<Vec<_>>(),
            "saved MAPPINGS 521 target identity and selection order must remain byte-semantic"
        );

        let move_rack = document
            .descendants()
            .find(|node| {
                node.has_tag_name("RACK")
                    && node.attribute("TYPE") == Some("4")
                    && direct_child(*node, "EFFECT").is_some_and(|effect| {
                        effect.attribute("TYPE") == Some("4")
                            && effect.attribute("ID") == Some("221")
                    })
            })
            .expect("saved specimen must contain Move 4/4/221");
        let move_effect =
            direct_child(move_rack, "EFFECT").expect("Move 4/4/221 must contain EFFECT");
        let move_params =
            direct_child(move_effect, "PARAMS").expect("Move 4/4/221 must contain PARAMS");
        assert_eq!(move_params.attribute("NB"), Some("3"));
        assert_eq!(
            element_children(move_params)
                .filter(|param| param.has_tag_name("PARAM"))
                .map(|param| {
                    (
                        param.attribute("ID").unwrap(),
                        param.attribute("TYPE").unwrap(),
                        param.attribute("VAL"),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("1", "5", None),
                ("2", "1", Some("0")),
                ("3", "2", Some("0"))
            ],
            "saved Move Circle must retain TYPE5/ID1, TYPE1/ID2=0, and TYPE2/ID3=0"
        );
        let path_param = element_children(move_params)
            .find(|param| param.has_tag_name("PARAM") && param.attribute("ID") == Some("1"))
            .expect("Move 4/4/221 must contain Path PARAM 1");
        assert_eq!(path_param.attribute("TYPE"), Some("5"));
        let points = direct_child(path_param, "POINTS")
            .expect("Move Circle Path PARAM 1 must contain POINTS");
        assert_eq!(points.attribute("NB"), Some("4"));
        assert_eq!(
            element_children(points)
                .filter(|point| point.has_tag_name("POINT"))
                .map(|point| {
                    (
                        point.attribute("X").unwrap().parse::<f32>().unwrap(),
                        point.attribute("Y").unwrap().parse::<f32>().unwrap(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![(0.25, 0.5), (0.5, 0.75), (0.75, 0.5), (0.5, 0.25)],
            "saved Move Circle must retain its four confirmed cardinal POINTS"
        );
        let move_beams = direct_child(move_rack, "BEAMS").expect("Move 4/4/221 must contain BEAMS");
        assert_eq!(move_beams.attribute("NB"), Some("6"));
        let fixture_uid = "3adf563d-e44a-4ee8-aac5-248b91469fa1";
        let raw_order = element_children(move_beams)
            .filter(|node| node.has_tag_name("BEAM"))
            .map(|beam| {
                (
                    beam.attribute("FIXTURE").unwrap().to_string(),
                    beam.attribute("BEAMID").unwrap().parse::<u16>().unwrap(),
                    beam.attribute("IDSELECTION")
                        .unwrap()
                        .parse::<u16>()
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            raw_order,
            (1_u16..=6)
                .map(|index| (fixture_uid.to_string(), index, index))
                .collect::<Vec<_>>(),
            "saved Move Circle BEAMID/IDSELECTION order must remain byte-semantic"
        );

        let fixture_refs = HashMap::from([(
            fixture_uid.to_string(),
            FixtureImportRef {
                fixture_id: 42,
                fixture_index: 0,
                profile_index: 0,
                supports_dimmer: false,
                color_beam_count: 8,
                patch_beam_positions: HashMap::new(),
            },
        )]);
        let targets = dvc_rack_targets(move_rack, &fixture_refs).unwrap();
        assert_eq!(targets.fixture_ids, vec![42]);
        assert!(targets
            .beam_targets
            .iter()
            .any(|target| target.beam_index != 0));
        assert_eq!(targets.ordered_steps, vec![vec![42]; 6]);
        assert_eq!(
            targets
                .beam_targets
                .iter()
                .map(|target| {
                    (
                        target.fixture_id,
                        target.beam_index,
                        target.selection_index,
                        target.feature_attribute.as_deref(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (42, 1, 0, None),
                (42, 2, 1, None),
                (42, 3, 2, None),
                (42, 4, 3, None),
                (42, 5, 4, None),
                (42, 6, 5, None),
            ],
            "dvc_rack_targets must retain BEAMID order and normalize IDSELECTION by first occurrence"
        );

        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        assert_eq!(
            outcome
                .report
                .converted
                .details
                .iter()
                .filter(|detail| {
                    detail.message.contains("source_family=Color Mappings;")
                        && detail.message.contains("source no-op preserved")
                        && detail.message.contains("palette_colors=8")
                        && detail.message.contains("Rectangle=(1994,166,513,52,0)")
                        && detail.message.contains("Grayscale=0")
                        && detail.message.contains("no runtime effect was created")
                })
                .count(),
            1,
            "saved empty COLOR MAPPINGS rack must import as one validated source no-op"
        );
        assert!(!outcome
            .report
            .skipped
            .details
            .iter()
            .any(|detail| { detail.message.contains("RACK TYPE=5 EFFECT TYPE=3 ID=36") }));
        assert!(
            outcome
                .project
                .snapshot
                .cues
                .iter()
                .flat_map(|cue| &cue.effect_targets)
                .filter_map(|target| match target.params.as_ref() {
                    Some(EffectParamsSnapshot::Color(request)) =>
                        request.spatial_pattern.as_deref(),
                    _ => None,
                })
                .all(|pattern| {
                    pattern.placement.as_ref().is_none_or(|placement| {
                        !(placement.x == 1994
                            && placement.y == 166
                            && placement.sx == 513
                            && placement.sy == 52)
                    })
                }),
            "the saved ID36 no-target rack must not create an empty runtime effect"
        );

        let placed_521 = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Color(request)) => request.spatial_pattern.as_deref(),
                _ => None,
            })
            .find(|pattern| {
                pattern.placement.as_ref().is_some_and(|placement| {
                    placement.x == 2630
                        && placement.y == -140
                        && placement.sx == 140
                        && placement.sy == 50
                })
            })
            .expect("saved MAPPINGS 521 must retain its Rectangle placement");
        assert!(matches!(
            placed_521.recipe,
            ColorEffectSpatialRecipe::Rainbow {
                grayscale: false,
                vertical_symmetry: false,
                horizontal_symmetry: false,
                rotation_degrees: 0.0,
                color_width: 0.0,
                angle_degrees: 0.0,
                gradient: 100.0,
            }
        ));
        assert_eq!(
            placed_521
                .beam_targets
                .iter()
                .map(|target| (
                    target.fixture_id,
                    target.beam_index,
                    target.selection_index,
                    target.feature_attribute.as_deref(),
                ))
                .collect::<Vec<_>>(),
            (0_u16..=3)
                .map(|beam_index| {
                    (
                        placed_521.beam_targets[0].fixture_id,
                        beam_index,
                        u32::from(beam_index),
                        Some("Dimmer"),
                    )
                })
                .collect::<Vec<_>>(),
            "521 must retain source BEAMID and IDSELECTION order"
        );
        let placement_521 = placed_521.placement.as_ref().unwrap();
        assert_eq!(
            (
                placement_521.source_coordinate_frame,
                placement_521.mapping_shape,
                placement_521.mapping_angle_degrees,
                placement_521.sampling_rule,
            ),
            (
                ColorEffectSpatialCoordinateFrame::DaslightPatchCanvas,
                ColorEffectSpatialMappingShape::Rectangle,
                0.0,
                ColorEffectSpatialSamplingRule::RotatedInclusionMaskAxisAlignedRaster,
            )
        );
        assert_eq!(
            placement_521
                .target_coordinates
                .iter()
                .map(|target| (
                    target.fixture_id,
                    target.beam_index,
                    target.patch_x,
                    target.patch_y,
                ))
                .collect::<Vec<_>>(),
            vec![
                (placed_521.beam_targets[0].fixture_id, 0, 2640, -130),
                (placed_521.beam_targets[0].fixture_id, 1, 2670, -130),
                (placed_521.beam_targets[0].fixture_id, 2, 2700, -130),
                (placed_521.beam_targets[0].fixture_id, 3, 2730, -130),
            ],
            "521 must retain authored absolute Patch coordinates rather than normalize stage X/Z"
        );

        let circles = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .filter_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Move(request))
                    if request.interpolation == MoveInterpolation::DaslightCircle =>
                {
                    Some(request)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            circles.len(),
            1,
            "saved Move Circle rack must import as exactly one cue-owned Move request"
        );
        let circle = circles[0];
        assert_eq!(circle.label, "New Scene (Circle)");
        assert_eq!(circle.fixture_ids.len(), 1);
        assert!(circle.target_group_ids.is_empty());
        assert_eq!(
            circle.points,
            vec![
                MovePathPoint { x: 0.25, y: 0.5 },
                MovePathPoint { x: 0.5, y: 0.75 },
                MovePathPoint { x: 0.75, y: 0.5 },
                MovePathPoint { x: 0.5, y: 0.25 },
            ],
            "imported Circle path must equal the four raw XML POINTS in source order"
        );
        assert!(circle.closed);
        assert_eq!(circle.interpolation, MoveInterpolation::DaslightCircle);
        assert_eq!(circle.coordinate_mode, MoveCoordinateMode::Absolute);
        assert_eq!(circle.center_x, 0.5);
        assert_eq!(circle.center_y, 0.5);
        assert_eq!(circle.size_x, 1.0);
        assert_eq!(circle.size_y, 1.0);
        assert_eq!(circle.rotation_degrees, 0.0);
        assert_eq!(circle.period_ms, 5_000);
        assert_eq!(circle.clock_sync, None);
        assert_eq!(circle.direction, MoveDirection::Forward);
        assert_eq!(circle.phase, 0.0);
        assert_eq!(circle.fixture_spread, 0.0);
        assert!(!circle.symmetry);
        assert_eq!(circle.blend_mode, EffectBlendMode::Override);
        assert_eq!(
            circle
                .beam_targets
                .iter()
                .map(|target| (
                    target.fixture_id,
                    target.beam_index,
                    target.selection_index,
                ))
                .collect::<Vec<_>>(),
            raw_order
                .iter()
                .enumerate()
                .map(|(selection_index, (_fixture_uid, beam_index, _raw_selection))| (
                    circle.fixture_ids[0],
                    *beam_index,
                    u32::try_from(selection_index).unwrap(),
                ))
                .collect::<Vec<_>>(),
            "imported Move beam targets must preserve raw XML fixture/BEAMID order and first-seen IDSELECTION order exactly"
        );
        assert_eq!(
            outcome
                .report
                .converted
                .details
                .iter()
                .filter(|detail| {
                    detail.item == "Effect: New Scene (Circle)"
                        && detail.message.contains("Daslight evaluator=ID 221 Circle")
                        && detail.message.contains("beam_targets=6; selections=6")
                        && detail.message.contains("coordinate_mode=Absolute")
                        && detail.message.contains("raw_phasing=0")
                        && detail.message.contains("symmetry=0")
                })
                .count(),
            1,
            "saved Move Circle rack must be reported once as an exact converted Circle"
        );
        assert!(!outcome
            .report
            .skipped
            .details
            .iter()
            .any(|detail| { detail.message.contains("RACK TYPE=4 EFFECT TYPE=4 ID=221") }));
        assert!(!outcome
            .report
            .approximate
            .details
            .iter()
            .any(|detail| detail.item == "Effect: New Scene (Circle)"));
    }

    #[test]
    fn dvc_unknown_move_generator_stays_fail_closed() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ValueCatalog-Sweep-Plasma.dvc"
        ));
        let source = fs::read_to_string(path).unwrap();
        let unknown_source = source.replacen(
            r#"<EFFECT TYPE="4" ID="221" DURATION="5000">"#,
            r#"<EFFECT TYPE="4" ID="226" DURATION="5000">"#,
            1,
        );
        assert_ne!(unknown_source, source);

        let outcome = import_bytes(unknown_source.as_bytes(), "unknown-move-226.dvc").unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        assert!(outcome.report.skipped.details.iter().any(|detail| {
            detail.message == "RACK TYPE=4 EFFECT TYPE=4 ID=226 is not confirmed for DVC-3b"
        }));
        assert!(!outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .any(|target| matches!(
                target.params.as_ref(),
                Some(EffectParamsSnapshot::Move(request))
                    if request.label.ends_with("(Circle)")
            )));
    }

    #[test]
    fn dvc_move_circle_nonzero_raw_phasing_imports_exactly() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../qa/specimens/ValueCatalog-Sweep-Plasma.dvc"
        ));
        let source = fs::read_to_string(path).unwrap();
        let effect_offset = source
            .find(r#"<EFFECT TYPE="4" ID="221" DURATION="5000">"#)
            .expect("saved specimen must contain Move Circle ID=221");
        let phasing = r#"<PARAM TYPE="1" ID="2" VAL="0"/>"#;
        let relative_offset = source[effect_offset..]
            .find(phasing)
            .expect("saved Move Circle must contain zero Phasing PARAM 2");
        let phasing_offset = effect_offset + relative_offset;
        let mut nonzero_source = source.clone();
        nonzero_source.replace_range(
            phasing_offset..phasing_offset + phasing.len(),
            r#"<PARAM TYPE="1" ID="2" VAL="0.5"/>"#,
        );

        let outcome =
            import_bytes(nonzero_source.as_bytes(), "circle-nonzero-phasing.dvc").unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        assert!(!outcome
            .report
            .skipped
            .details
            .iter()
            .any(|detail| { detail.item == "Effect: New Scene (Circle)" }));
        let circle = outcome
            .project
            .snapshot
            .cues
            .iter()
            .flat_map(|cue| &cue.effect_targets)
            .find_map(|target| match target.params.as_ref() {
                Some(EffectParamsSnapshot::Move(request))
                    if request.interpolation == MoveInterpolation::DaslightCircle =>
                {
                    Some(request)
                }
                _ => None,
            })
            .expect("nonzero-phasing Circle must import through the exact evaluator");
        assert_eq!(circle.fixture_spread, 0.5);
    }

    #[test]
    fn dvc_local_homecoming_chaser_presets_restore_rgb_features_when_present() {
        let path = Path::new(r"C:\Users\kouty\Desktop\homecoming2026\homecoming2606.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local homecoming Chaser feature golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        for (label, expected_attribute, expected_type) in [
            ("saber_chase-red", "ColorRed", 65_u16),
            ("saber_chase-green", "ColorGreen", 66_u16),
            ("saber_chase-blue", "ColorBlue", 67_u16),
            // This saved scene intentionally has stale PRESET/BEAMS from two
            // Strongpoint fixtures. RACK/BEAMS still names four Saber targets,
            // proving PRESET selects the feature type rather than replacing
            // the effect target list.
            ("strobe_wave-red", "ColorRed", 65_u16),
        ] {
            let request = outcome
                .project
                .snapshot
                .cues
                .iter()
                .find(|cue| cue.label == label)
                .and_then(|cue| {
                    cue.effect_targets
                        .iter()
                        .find_map(|target| match target.params.as_ref() {
                            Some(EffectParamsSnapshot::Chaser(request)) => Some(request),
                            _ => None,
                        })
                })
                .unwrap_or_else(|| panic!("homecoming must preserve {label} Chaser"));
            assert_eq!(
                request
                    .features
                    .iter()
                    .map(|feature| feature.attribute.as_str())
                    .collect::<Vec<_>>(),
                vec![expected_attribute]
            );
            assert!(request
                .features
                .iter()
                .all(|feature| { feature.low == 0 && feature.high == u16::MAX }));
            assert_eq!(
                request
                    .steps
                    .iter()
                    .flat_map(|step| &step.beam_targets)
                    .count(),
                4
            );
            assert!(request
                .steps
                .iter()
                .flat_map(|step| &step.beam_targets)
                .all(|target| target.feature_attribute == expected_attribute));
            assert!(outcome.report.converted.details.iter().any(|detail| {
                detail.item == format!("Effect: {label} (Chaser #1)")
                    && detail
                        .message
                        .contains(&format!("preset_type={expected_type}"))
            }));
        }
    }

    #[test]
    fn dvc_local_documents_backbar_chaser_restores_segment_amber_when_present() {
        let path = Path::new(r"C:\Users\kouty\Documents\Daslight 5\Projects\Shinkan2026.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local BackBar Amber Chaser golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        let request = outcome
            .project
            .snapshot
            .cues
            .iter()
            .find(|cue| cue.label == "BackBar-Amber")
            .and_then(|cue| {
                cue.effect_targets
                    .iter()
                    .find_map(|target| match target.params.as_ref() {
                        Some(EffectParamsSnapshot::Chaser(request)) => Some(request),
                        _ => None,
                    })
            })
            .expect("Documents Shinkan must preserve BackBar-Amber Chaser");
        assert_eq!(
            request
                .features
                .iter()
                .map(|feature| feature.attribute.clone())
                .collect::<HashSet<_>>(),
            HashSet::from([
                "ColorAmber".to_string(),
                "ColorAmber 2".to_string(),
                "ColorAmber 3".to_string(),
                "ColorAmber 4".to_string(),
                "ColorAmber 5".to_string(),
                "ColorAmber 6".to_string(),
                "ColorAmber 7".to_string(),
                "ColorAmber 8".to_string(),
            ])
        );
        let beam_targets = request
            .steps
            .iter()
            .flat_map(|step| &step.beam_targets)
            .collect::<Vec<_>>();
        assert_eq!(request.steps.len(), 48);
        assert_eq!(beam_targets.len(), 48);
        assert!(request
            .steps
            .iter()
            .all(|step| !step.beam_targets.is_empty()));
        assert!(beam_targets.iter().all(|target| {
            let expected = if target.beam_index == 0 {
                "ColorAmber".to_string()
            } else {
                format!("ColorAmber {}", target.beam_index + 1)
            };
            target.feature_attribute == expected
        }));
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item == "Effect: BackBar-Amber (Chaser #1)"
                && detail.message.contains("preset_type=81")
                && detail.message.contains("profile PRESET")
        }));
        assert!(outcome.report.approximate.details.iter().any(|detail| {
            detail.item == "Effect: BackBar-Amber (Chaser #1)"
                && detail
                    .message
                    .contains("3 beam target(s) without PRESET type 81 were omitted")
        }));
    }

    #[test]
    fn dvc_local_homecoming_touch_tap_tempo_is_bound_when_present() {
        let path = Path::new(r"C:\Users\kouty\Desktop\homecoming2026\homecoming2606.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local homecoming Touch golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        let control = outcome
            .project
            .snapshot
            .touch_surface
            .pages
            .iter()
            .flat_map(|page| &page.controls)
            .find(|control| control.label == "Tap Tempo")
            .expect("homecoming golden must preserve its Tap Tempo Touch control");
        assert_eq!(control.kind, TouchControlKind::Button);
        assert!(matches!(
            control.binding.as_ref(),
            Some(TouchControlBinding::TapTempo)
        ));
    }

    #[test]
    fn dvc_local_homecoming_midi_scene_tap_and_flash_shortcuts_import_when_present() {
        let path = Path::new(r"C:\Users\kouty\Desktop\homecoming2026\homecoming2606.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local homecoming MIDI golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        assert_eq!(outcome.report.midi_mappings.len(), 17);
        assert_eq!(
            outcome
                .report
                .midi_mappings
                .iter()
                .filter(|mapping| mapping.action == MidiControlAction::TriggerCue)
                .count(),
            12
        );
        assert_eq!(
            outcome
                .report
                .midi_mappings
                .iter()
                .filter(|mapping| mapping.action == MidiControlAction::FlashCue)
                .count(),
            1
        );
        assert_eq!(
            outcome
                .report
                .midi_mappings
                .iter()
                .filter(|mapping| mapping.action == MidiControlAction::FlashCueDirection)
                .count(),
            2
        );
        assert!(outcome.report.midi_mappings.iter().any(|mapping| {
            mapping.action == MidiControlAction::FlashCueDirection
                && mapping.attribute.as_deref() == Some("Forward")
        }));
        assert!(outcome.report.midi_mappings.iter().any(|mapping| {
            mapping.action == MidiControlAction::FlashCueDirection
                && mapping.attribute.as_deref() == Some("Bounce")
        }));
        assert_eq!(
            outcome
                .report
                .midi_mappings
                .iter()
                .filter(|mapping| mapping.action == MidiControlAction::TriggerCueListNext)
                .count(),
            1
        );
        assert_eq!(
            outcome.project.snapshot.cue_lists.len(),
            outcome.report.summary.groups
        );
        assert!(outcome.project.snapshot.cues.iter().all(|cue| {
            outcome.project.snapshot.cue_lists.iter().any(|list| {
                list.id == cue.cue_list_id && Some(list.label.as_str()) == cue.group_id.as_deref()
            })
        }));
        assert_eq!(
            outcome
                .report
                .midi_mappings
                .iter()
                .filter(|mapping| mapping.action == MidiControlAction::TapBpm)
                .count(),
            1
        );
        let unsupported_midi_actions = outcome
            .report
            .unsupported
            .details
            .iter()
            .filter(|detail| detail.message.contains("Daslight MIDI action type"))
            .map(|detail| detail.message.as_str())
            .collect::<Vec<_>>();
        assert!(unsupported_midi_actions.is_empty());
    }

    #[test]
    fn dvc_local_homecoming_laser_touch_faders_map_verified_feature_presets_when_present() {
        let path = Path::new(r"C:\Users\kouty\Desktop\homecoming2026\homecoming2606-Laser.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local homecoming Laser Touch golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        let controls = outcome
            .project
            .snapshot
            .touch_surface
            .pages
            .iter()
            .flat_map(|page| &page.controls)
            .filter(|control| control.label == "Dimmer" || control.label == "Dimmer linear")
            .collect::<Vec<_>>();
        assert_eq!(controls.len(), 2);
        assert!(controls
            .iter()
            .all(|control| control.kind == TouchControlKind::Fader));
        assert!(controls.iter().any(|control| {
            control.label == "Dimmer"
                && (control.x, control.y, control.w, control.h) == (0, 0, 1, 4)
                && matches!(
                    control.binding.as_ref(),
                    Some(TouchControlBinding::FeaturePreset {
                        targets,
                        min_value: 0,
                        max_value: u16::MAX,
                        inverted: false,
                    }) if targets.iter().map(|target| target.fixture_id).collect::<Vec<_>>()
                        == vec![8, 11, 9, 10, 1, 2]
                        && targets.iter().all(|target| target.attribute == "Dimmer")
                )
        }));
        assert!(controls.iter().any(|control| {
            control.label == "Dimmer linear"
                && (control.x, control.y, control.w, control.h) == (0, 4, 1, 4)
                && matches!(
                    control.binding.as_ref(),
                    Some(TouchControlBinding::FeaturePreset {
                        targets,
                        min_value: 0,
                        max_value: u16::MAX,
                        inverted: false,
                    }) if targets.iter().map(|target| target.fixture_id).collect::<Vec<_>>()
                        == vec![5, 6, 4, 7]
                        && targets.iter().all(|target| target.attribute == "Dimmer")
                )
        }));
        assert!(!outcome
            .report
            .unsupported
            .details
            .iter()
            .any(|detail| detail.item.starts_with("Shortcut")
                && detail.message.contains("Feature Preset")));

        let feature_targets = |label: &str| {
            let binding = controls
                .iter()
                .find(|control| control.label == label)
                .and_then(|control| control.binding.as_ref())
                .expect("verified Touch fader must remain bound");
            let TouchControlBinding::FeaturePreset { targets, .. } = binding else {
                panic!("verified Touch fader must use a Feature Preset binding");
            };
            targets.clone()
        };
        let dimmer_targets = feature_targets("Dimmer");
        let linear_targets = feature_targets("Dimmer linear");
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine
            .send(EngineCommand::LoadProjectSnapshot(
                outcome.project.snapshot.clone(),
            ))
            .unwrap();
        for _ in 0..40 {
            if engine.snapshot().telemetry.queue_depth == 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        engine
            .send(EngineCommand::SetFixtureAttributeBatch {
                targets: dimmer_targets.clone(),
                value: u16::MAX,
            })
            .unwrap();
        let mut snapshot = engine.snapshot();
        for _ in 0..40 {
            snapshot = engine.snapshot();
            if snapshot.telemetry.queue_depth == 0 && snapshot.dmx_preview.get(0) == Some(&255) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let lit_addresses = snapshot
            .dmx_preview
            .iter()
            .enumerate()
            .filter(|(_, value)| **value != 0)
            .map(|(index, _)| index + 1)
            .collect::<Vec<_>>();
        assert_eq!(lit_addresses, vec![1, 14, 173, 178, 183, 188]);

        engine
            .send(EngineCommand::SetFixtureAttributeBatch {
                targets: dimmer_targets,
                value: 0,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetFixtureAttributeBatch {
                targets: linear_targets,
                value: u16::MAX,
            })
            .unwrap();
        for _ in 0..40 {
            snapshot = engine.snapshot();
            if snapshot.telemetry.queue_depth == 0 && snapshot.dmx_preview.get(65) == Some(&255) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let lit_addresses = snapshot
            .dmx_preview
            .iter()
            .enumerate()
            .filter(|(_, value)| **value != 0)
            .map(|(index, _)| index + 1)
            .collect::<Vec<_>>();
        assert_eq!(lit_addresses, vec![66, 100, 134, 168]);
    }

    #[test]
    fn dvc_local_homecoming_laser_midi_faders_follow_operator_selection_when_present() {
        let path = Path::new(r"C:\Users\kouty\Desktop\homecoming2026\homecoming2606-Laser.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local homecoming Laser MIDI golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        assert_eq!(outcome.report.midi_mappings.len(), 2);
        assert!(outcome.report.midi_mappings.iter().any(|mapping| {
            mapping.action == MidiControlAction::SelectedFeatureFader
                && mapping.number == 8
                && mapping.cue_point_index == Some(0)
                && mapping.low == 0.0
                && mapping.high == 65_535.0
                && mapping.feedback.as_ref().is_some_and(|feedback| {
                    feedback.off.as_ref().is_some_and(|message| {
                        message.message == MidiControlMessage::ControlChange
                            && message.channel == 0
                            && message.number == 8
                            && message.value == 0
                    }) && feedback.on.as_ref().is_some_and(|message| {
                        message.message == MidiControlMessage::ControlChange
                            && message.channel == 0
                            && message.number == 8
                            && message.value == 1
                    })
                })
        }));
        assert!(outcome.report.midi_mappings.iter().any(|mapping| {
            mapping.action == MidiControlAction::SelectedFeatureFader
                && mapping.number == 9
                && mapping.cue_point_index == Some(1)
                && mapping.low == 0.0
                && mapping.high == 65_535.0
                && mapping.feedback.as_ref().is_some_and(|feedback| {
                    feedback.off.as_ref().is_some_and(|message| {
                        message.message == MidiControlMessage::ControlChange
                            && message.channel == 0
                            && message.number == 9
                            && message.value == 0
                    }) && feedback.on.as_ref().is_some_and(|message| {
                        message.message == MidiControlMessage::ControlChange
                            && message.channel == 0
                            && message.number == 9
                            && message.value == 1
                    })
                })
        }));
        assert!(!outcome
            .report
            .unsupported
            .details
            .iter()
            .any(|detail| detail.message.contains("Daslight MIDI action type 229")));
    }

    #[test]
    fn dvc_local_value_fx_noop_specimen_roundtrips_without_runtime_output_when_present() {
        let path = Path::new(r"C:\Users\kouty\Desktop\Shinkan-Left\Codex-Chaser322-Probe.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local VALUE FX DVC golden: {} is unavailable",
                path.display()
            );
            return;
        }

        let outcome = import_path(path).unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        assert!(outcome.report.converted.details.iter().any(|detail| {
            detail.item == "Effect: SS-Blue (Rainbow)"
                && detail.message.contains("source_family=Value FX")
                && detail.message.contains("source no-op preserved")
                && detail.message.contains("palette_colors=3")
        }));
        assert!(!outcome
            .report
            .skipped
            .details
            .iter()
            .any(|detail| { detail.message.contains("RACK TYPE=7 EFFECT TYPE=7 ID=621") }));
        assert!(!outcome.project.snapshot.cues.iter().any(|cue| {
            cue.effect_targets.iter().any(|target| {
                matches!(target.params.as_ref(), Some(EffectParamsSnapshot::Value(_)))
            })
        }));

        let json = serde_json::to_string(&outcome.project).unwrap();
        let roundtrip: ProjectFile = serde_json::from_str(&json).unwrap();
        crate::validate_project_file(&roundtrip).unwrap();
        assert!(!roundtrip.snapshot.cues.iter().any(|cue| {
            cue.effect_targets.iter().any(|target| {
                matches!(target.params.as_ref(), Some(EffectParamsSnapshot::Value(_)))
            })
        }));
    }

    #[test]
    fn dvc_local_project_inventory_reports_every_available_import_boundary() {
        let candidates = [
            r"C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc",
            r"C:\Users\kouty\Desktop\INMDAISUKI\DFS2026.dvc",
            r"C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc",
            r"C:\Users\kouty\Desktop\Shinkan-Left\Left1.dvc",
            r"C:\Users\kouty\Desktop\Shinkan-Left\Codex-Chaser322-Probe.dvc",
            r"C:\Users\kouty\Documents\Daslight 5\Projects\Sin.dvc",
            r"C:\Users\kouty\Documents\Daslight 5\Projects\Shinkan2026.dvc",
            r"C:\Users\kouty\Documents\Daslight 5\Projects\Panel.dvc",
            r"C:\Users\kouty\Desktop\homecoming2026\homecoming2606.dvc",
            r"C:\Users\kouty\Desktop\homecoming2026\homecoming2606-scenes.dvc",
            r"C:\Users\kouty\Desktop\homecoming2026\homecoming2606-Laser.dvc",
        ];
        let mut audited = 0_usize;
        for candidate in candidates {
            let path = Path::new(candidate);
            if !path.is_file() {
                continue;
            }
            let outcome = import_path(path).unwrap_or_else(|error| {
                panic!(
                    "local DVC inventory failed to import {}: {error}",
                    path.display()
                )
            });
            crate::validate_project_file(&outcome.project).unwrap_or_else(|error| {
                panic!(
                    "local DVC inventory produced an invalid project for {}: {error}",
                    path.display()
                )
            });
            audited = audited.saturating_add(1);
            eprintln!(
                "DVC_AUDIT|{}|fixtures={}|cues={}|effects={}/{}|midi={}|approximate={}|skipped={}|unsupported={}|warnings={}",
                path.display(),
                outcome.report.summary.fixtures,
                outcome.report.summary.cues,
                outcome.report.summary.effects_converted,
                outcome.report.summary.effects_skipped,
                outcome.report.midi_mappings.len(),
                outcome.report.approximate.count,
                outcome.report.skipped.count,
                outcome.report.unsupported.count,
                outcome.report.warnings.len(),
            );
            for (category, details) in [
                ("APPROXIMATE", &outcome.report.approximate.details),
                ("SKIPPED", &outcome.report.skipped.details),
                ("UNSUPPORTED", &outcome.report.unsupported.details),
            ] {
                for detail in details {
                    eprintln!(
                        "DVC_AUDIT_DETAIL|{}|{}|{}|{}",
                        path.display(),
                        category,
                        detail.item,
                        detail.message,
                    );
                }
            }
        }
        if audited == 0 {
            eprintln!("Skipping local DVC inventory: no known project files are available");
        }
    }

    #[test]
    fn dvc_local_panel_restores_verified_dmx_rgb_mappings_when_present() {
        let path = Path::new(r"C:\Users\kouty\Documents\Daslight 5\Projects\Panel.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local Panel DVC mapping audit: {} is unavailable",
                path.display()
            );
            return;
        }
        let outcome = import_path(path).unwrap();
        let restored = outcome
            .report
            .dmx_mappings
            .iter()
            .map(|mapping| {
                (
                    mapping.universe,
                    mapping.channel,
                    mapping.fixture_id,
                    mapping.attribute.as_deref(),
                    mapping.low,
                    mapping.high,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            restored,
            vec![
                (0, 25, Some(1), Some("ColorRed"), 0.0, 65_535.0),
                (0, 26, Some(1), Some("ColorGreen"), 0.0, 65_535.0),
                (0, 27, Some(1), Some("ColorBlue"), 0.0, 65_535.0),
                (0, 36, Some(2), Some("ColorRed"), 0.0, 65_535.0),
                (0, 37, Some(2), Some("ColorGreen"), 0.0, 65_535.0),
                (0, 38, Some(2), Some("ColorBlue"), 0.0, 65_535.0),
                (0, 47, Some(3), Some("ColorBlue"), 0.0, 65_535.0),
                (0, 48, Some(3), Some("ColorRed"), 0.0, 65_535.0),
                (0, 49, Some(3), Some("ColorGreen"), 0.0, 65_535.0),
            ]
        );
        assert_eq!(outcome.report.unsupported.count, 0);
        assert_eq!(outcome.report.skipped.count, 0);
    }

    #[test]
    fn dvc_local_homecoming_inverse_ramp_matches_verified_generator_when_present() {
        let path = Path::new(r"C:\Users\kouty\Desktop\homecoming2026\homecoming2606.dvc");
        if !path.is_file() {
            eprintln!(
                "Skipping local homecoming Daslight golden: {} is unavailable",
                path.display()
            );
            return;
        }
        let source = fs::read_to_string(path).unwrap();
        let document = Document::parse(&source).unwrap();
        let scene = document
            .descendants()
            .filter(|node| node.has_tag_name("SCENE"))
            .find(|node| node.attribute("NAME") == Some("all_rampFlash"))
            .expect("homecoming golden must contain all_rampFlash");
        let rack = direct_child(scene, "RACKS")
            .and_then(|racks| {
                element_children(racks)
                    .find(|rack| rack.has_tag_name("RACK") && rack.attribute("TYPE") == Some("8"))
            })
            .expect("all_rampFlash must contain its CURVE rack");
        let effect = element_children(rack)
            .find(|effect| {
                effect.has_tag_name("EFFECT")
                    && effect.attribute("TYPE") == Some("5")
                    && effect.attribute("ID") == Some("3")
            })
            .expect("all_rampFlash must contain CURVE ID=3");
        let mut fixture_refs = HashMap::new();
        for beam in direct_child(rack, "BEAMS")
            .into_iter()
            .flat_map(element_children)
            .filter(|node| node.has_tag_name("BEAM"))
        {
            let uid = required_attribute(beam, "FIXTURE", "BEAM").unwrap();
            if fixture_refs.contains_key(uid) {
                continue;
            }
            let fixture_id = fixture_refs.len() as u64 + 1;
            fixture_refs.insert(
                uid.to_string(),
                FixtureImportRef {
                    fixture_id,
                    fixture_index: fixture_id as usize - 1,
                    profile_index: 0,
                    supports_dimmer: true,
                    color_beam_count: 1,
                    patch_beam_positions: HashMap::new(),
                },
            );
        }
        let converted = convert_dvc_effect(
            scene,
            "all_rampFlash",
            rack,
            effect,
            8,
            5,
            3,
            1,
            &effect_test_profiles(),
            &fixture_refs,
        )
        .unwrap();
        let Some(EffectParamsSnapshot::Lfo(ramp)) =
            converted.target.expect("runtime target").params
        else {
            panic!("homecoming golden must convert all_rampFlash to an LFO");
        };
        assert_eq!(converted.generator, "Inverse Ramp");
        assert!(converted.note.contains("descending_ramp=Saw directed"));
        assert!(!converted
            .approximations
            .iter()
            .any(|note| note.contains("endpoints were clamped")));
        assert_eq!(ramp.shape, LfoShape::Saw);
        assert_eq!(ramp.period_ms, 5_000);
        assert_eq!(ramp.low, 65_207);
        assert_eq!(ramp.high, 0);
        assert_eq!(ramp.phase, 0.495);
        assert_eq!(ramp.daslight_curve.as_ref().unwrap().rate, 2.0);
        assert_eq!(ramp.daslight_curve.as_ref().unwrap().sample_ms, 40);
    }
}
