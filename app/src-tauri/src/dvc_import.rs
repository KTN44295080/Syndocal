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
    ChildTimelineSummary, CueFixtureTarget, CueSummary, DmxModeSummary, DmxOutputConfig,
    DmxUniversePreview, EngineSnapshot, FixtureProfileSummary, GeometrySummary,
    PatchedFixtureSummary, ProjectFile, Rotation3, TimelineAudioClipSummary,
    TimelineCueEventSummary, TimelineLayerKind, TimelineLayerSummary, TimelineTrackKind, Vec3,
};
use roxmltree::{Document, Node};
use serde::Serialize;

const DVC_FILE_MAX_BYTES: u64 = 64 * 1024 * 1024;
const DVC_PATCH_MAX_BYTES: usize = 128 * 1024 * 1024;
const DVC_FIXTURE_DATA_MAX_BYTES: usize = 16 * 1024 * 1024;
const DVC_FIXTURE_RECORD_BYTES: usize = 27;
const DVC_REPORT_DETAIL_LIMIT: usize = 256;

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
    pub(crate) timeline_audio_clips: usize,
    pub(crate) timeline_scene_blocks: usize,
    pub(crate) effects_skipped: usize,
    pub(crate) unknown_channel_types: usize,
    pub(crate) missing_audio_files: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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
        }
    }
}

#[derive(Debug, Clone)]
struct DvcChannelBinding {
    attribute: String,
    raw_offsets: Vec<usize>,
    resolution: AttributeResolution,
}

#[derive(Debug, Clone)]
struct ParsedProfile {
    summary: FixtureProfileSummary,
    physical_channel_count: usize,
    bindings: Vec<DvcChannelBinding>,
}

#[derive(Debug, Clone, Copy)]
struct FixtureImportRef {
    fixture_id: u64,
    fixture_index: usize,
    profile_index: usize,
}

#[derive(Debug)]
struct FixtureDataValues {
    values: Vec<Option<u8>>,
    record_bytes: usize,
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

    let (profiles, mut fixtures, fixture_refs) = parse_patch(patch_root, root, &mut report)?;
    fit_fixture_positions(&mut fixtures);

    let mut snapshot = EngineSnapshot::default();
    snapshot.fixtures = fixtures;
    snapshot.group_colors = BTreeMap::new();
    let (mut cues, scene_indices, bank_count) = parse_scenes(
        root,
        &profiles,
        &fixture_refs,
        &mut snapshot.fixtures,
        &mut snapshot.group_colors,
        &mut report,
    )?;
    parse_super_scenes(root, &mut cues, &scene_indices, &mut snapshot, &mut report)?;
    snapshot.cues = cues;
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
        custom_profiles: profiles
            .iter()
            .map(|profile| profile.summary.clone())
            .collect(),
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

fn parse_patch(
    patch_root: Node<'_, '_>,
    dvc_root: Node<'_, '_>,
    report: &mut DvcImportReport,
) -> Result<
    (
        Vec<ParsedProfile>,
        Vec<PatchedFixtureSummary>,
        HashMap<String, FixtureImportRef>,
    ),
    String,
> {
    let fixture_groups = fixture_group_memberships(dvc_root);
    let mut profiles = Vec::new();
    let mut fixtures = Vec::new();
    let mut fixture_refs = HashMap::new();
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
    Ok((profiles, fixtures, fixture_refs))
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
        let default_value = channel_default_value(channel, &resolution);
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
            })
        })
        .collect()
}

fn channel_default_value(channel: Node<'_, '_>, resolution: &AttributeResolution) -> u16 {
    let preferred = channel
        .descendants()
        .filter(|node| node.has_tag_name("SSLPRESET"))
        .find(|node| node.attribute("SSLPRESETDEFAULTPRESET") == Some("1"))
        .or_else(|| {
            channel
                .descendants()
                .find(|node| node.has_tag_name("SSLPRESET"))
        });
    preferred
        .and_then(|preset| preset.attribute("SSLPRESETDMXDEFAULT"))
        .and_then(|value| value.parse::<u16>().ok())
        .map(|value| scale_dmx_value(value, resolution))
        .unwrap_or(0)
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

fn parse_scenes(
    root: Node<'_, '_>,
    profiles: &[ParsedProfile],
    fixture_refs: &HashMap<String, FixtureImportRef>,
    fixtures: &mut [PatchedFixtureSummary],
    group_colors: &mut BTreeMap<String, String>,
    report: &mut DvcImportReport,
) -> Result<(Vec<CueSummary>, HashMap<String, usize>, usize), String> {
    let Some(scenes_section) = direct_child(root, "SCENES") else {
        return Ok((Vec::new(), HashMap::new(), 0));
    };
    let banks = element_children(scenes_section)
        .filter(|node| node.has_tag_name("BANK"))
        .collect::<Vec<_>>();
    let mut cues = Vec::new();
    let mut scene_indices = HashMap::new();

    for (bank_index, bank) in banks.iter().copied().enumerate() {
        let bank_name =
            non_empty_label(bank.attribute("NAME"), &format!("Bank {}", bank_index + 1));
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
            let effect_count = scene
                .descendants()
                .filter(|node| node.has_tag_name("EFFECT"))
                .count();
            let mut notes = vec![format!(
                "Daslight bank={bank_name}; loop={}; speed={}; play_trigger={}; play_division={}; fade_out_ms={fade_out}",
                scene.attribute("LOOP").unwrap_or("0"),
                scene.attribute("SPEED").unwrap_or("1"),
                scene.attribute("PLAY_TRIGGER").unwrap_or("0"),
                scene.attribute("PLAY_DIVISION").unwrap_or("0"),
            )];
            if effect_count > 0 {
                notes.push(format!(
                    "Daslight effects={effect_count}; not converted by DVC-1"
                ));
                report.summary.effects_skipped =
                    report.summary.effects_skipped.saturating_add(effect_count);
                report.skipped.add(
                    effect_count,
                    format!("Cue: {scene_name}"),
                    "RACK/EFFECT numeric codes were not converted",
                );
            }
            let cue = CueSummary {
                id: cue_id,
                cue_list_id: protocol::DEFAULT_CUE_LIST_ID,
                cue_number: format!("{}.{}", bank_index + 1, scene_index + 1),
                label: scene_name,
                group_id: Some(bank_name.clone()),
                fade_ms: fade_in,
                notes: notes.join(" | "),
                color: scene.attribute("COLOR").and_then(dvc_argb_to_rgb),
                ..CueSummary::default()
            };
            let cue_index = cues.len();
            cues.push(cue);
            if scene_indices.insert(scene_uid.clone(), cue_index).is_some() {
                return Err(format!("Duplicate Daslight scene DASUID {scene_uid}"));
            }
        }
    }

    let mut expected_record_bytes = HashMap::<usize, usize>::new();
    for bank in banks.iter().copied() {
        let bank_name = non_empty_label(bank.attribute("NAME"), "Bank");
        for scene in element_children(bank).filter(|node| node.has_tag_name("SCENE")) {
            let scene_uid = required_attribute(scene, "DASUID", "SCENE")?;
            let cue_index = *scene_indices
                .get(scene_uid)
                .ok_or_else(|| format!("Scene index missing for {scene_uid}"))?;
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
                    let values = fixture_attribute_values(profile, &decoded.values)?;
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
    Ok((cues, scene_indices, banks.len()))
}

fn fixture_attribute_values(
    profile: &ParsedProfile,
    raw_values: &[Option<u8>],
) -> Result<Vec<AttributeValueSummary>, String> {
    let mut values = Vec::new();
    for binding in &profile.bindings {
        let value = match (&binding.resolution, binding.raw_offsets.as_slice()) {
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
        if let Some(value) = value {
            values.push(AttributeValueSummary {
                attribute: binding.attribute.clone(),
                value,
            });
        }
    }
    Ok(values)
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
    let super_scene_uids = scenes_section
        .descendants()
        .filter(|node| node.has_tag_name("SCENE"))
        .filter(|scene| {
            scene
                .descendants()
                .any(|node| node.has_tag_name("TIMELINES"))
        })
        .filter_map(|scene| scene.attribute("DASUID"))
        .collect::<HashSet<_>>();
    let mut next_layer_id = 1_u32;
    let mut next_event_id = 1_u64;
    let mut next_audio_clip_id = 1_u64;
    let mut first_timeline_bpm = None;

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
                kind,
            });
            if timeline
                .attribute("DASTLFOLDED")
                .is_some_and(|value| value != "0")
            {
                report.skipped.add(
                    1,
                    format!("Super Scene: {owner_label}"),
                    "Daslight folded lane state has no persisted Syndocal equivalent",
                );
            }
        }

        let timeline_bpm = timeline_nodes
            .iter()
            .flat_map(|timeline| timeline.descendants())
            .filter(|node| node.has_tag_name("BLOCK") && node.attribute("TYPE") == Some("2"))
            .filter_map(|node| parse_f32_attribute(node, "BPM"))
            .find(|bpm| bpm.is_finite() && *bpm > 0.0)
            .unwrap_or(120.0);
        if first_timeline_bpm.is_none() && timeline_bpm > 0.0 {
            first_timeline_bpm = Some(timeline_bpm);
        }
        let mut audio_clips = Vec::new();
        let mut events = Vec::new();
        let mut duration_ms = 0_u64;
        let mut authored_candidates = Vec::<(usize, f32)>::new();

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
                        if super_scene_uids.contains(source_uid) {
                            report.skipped.add(
                                1,
                                format!(
                                    "Scene block: {}",
                                    block.attribute("NAME").unwrap_or("Untitled")
                                ),
                                "Nested Super Scene reference was skipped (depth-1 model)",
                            );
                            continue;
                        }
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
                        let duration_beats =
                            f64::from(timeline_bpm) * block_duration as f64 / 60_000.0;
                        if conform {
                            let authored = (duration_beats * f64::from(speed)).clamp(
                                f64::from(protocol::MIN_CUE_AUTHORED_BEATS),
                                f64::from(protocol::MAX_CUE_AUTHORED_BEATS),
                            ) as f32;
                            authored_candidates.push((source_index, authored));
                            report.approximate.add(
                                1,
                                format!("Scene block: {}", block.attribute("NAME").unwrap_or("Untitled")),
                                format!(
                                    "Authored beat length inferred from {block_duration} ms at {timeline_bpm:.3} BPM"
                                ),
                            );
                        } else if allow_loop {
                            report.approximate.add(
                                1,
                                format!(
                                    "Scene block: {}",
                                    block.attribute("NAME").unwrap_or("Untitled")
                                ),
                                "ALLOWLOOP without CONFORM_TO_TEMPO remains a fixed one-pass block",
                            );
                        }
                        let fade_in = parse_u64_attribute(block, "FADEIN")
                            .unwrap_or(0)
                            .min(block_duration);
                        let fade_out = parse_u64_attribute(block, "FADEOUT")
                            .unwrap_or(0)
                            .min(block_duration.saturating_sub(fade_in));
                        events.push(TimelineCueEventSummary {
                            id: next_event_id,
                            cue_id: cues[source_index].id,
                            time_ms: start_ms,
                            time_beats: conform
                                .then_some(f64::from(timeline_bpm) * start_ms as f64 / 60_000.0),
                            track: TimelineTrackKind::Lighting,
                            layer_id: Some(layer_id),
                            duration_ms: block_duration,
                            duration_beats: conform.then_some(duration_beats),
                            conform_to_tempo: conform,
                            loop_fill: conform && allow_loop,
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
                                "{start_ms}..{end_ms} ms -> cue {}",
                                cues[source_index].label
                            ),
                        );
                        if parse_i64_attribute(block, "POSITION").unwrap_or(0) != 0 {
                            report.skipped.add(
                                1,
                                format!(
                                    "Scene block: {}",
                                    block.attribute("NAME").unwrap_or("Untitled")
                                ),
                                "Daslight source POSITION offset has no Scene Block field",
                            );
                        }
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

        authored_candidates.sort_by(|left, right| left.0.cmp(&right.0));
        for (source_index, authored) in authored_candidates {
            if cues[source_index].authored_beats.is_none() {
                cues[source_index].authored_beats = Some(authored);
            }
        }
        cues[owner_index].child_timeline = Some(ChildTimelineSummary {
            layers,
            events,
            automations: Vec::new(),
            video_automations: Vec::new(),
            audio: None,
            audio_clips,
            duration_ms,
        });
        cues[owner_index]
            .notes
            .push_str(" | Daslight Super Scene child timeline imported");
    }
    if let Some(bpm) = first_timeline_bpm {
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

fn fit_fixture_positions(fixtures: &mut [PatchedFixtureSummary]) {
    if fixtures.is_empty() {
        return;
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
    let width = (max_x - min_x).max(0.0);
    let depth = (max_z - min_z).max(0.0);
    let scale_x = (width > f32::EPSILON).then_some(20.0 / width);
    let scale_z = (depth > f32::EPSILON).then_some(20.0 / depth);
    let scale = match (scale_x, scale_z) {
        (Some(x), Some(z)) => x.min(z),
        (Some(x), None) => x,
        (None, Some(z)) => z,
        (None, None) => 1.0,
    };
    let center_x = (min_x + max_x) * 0.5;
    let center_z = (min_z + max_z) * 0.5;
    for fixture in fixtures {
        fixture.position.x = (fixture.position.x - center_x) * scale;
        fixture.position.z = (fixture.position.z - center_z) * scale;
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
    // Verified across all four read-only DVC-1 specimens: a BE channel count,
    // one BE u16 per physical channel (0xFFFF = unwritten), a BE record count,
    // then record_count opaque 27-byte metadata rows. The metadata rows are
    // deliberately validated but not interpreted as steps without evidence.
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
    Ok(FixtureDataValues {
        values,
        record_bytes,
    })
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
    for (tag, label) in [
        ("TOUCH", "Touch layout"),
        ("SHORTCUTS", "Shortcuts"),
        ("DEVICES", "Daslight hardware devices"),
    ] {
        let Some(section) = direct_child(root, tag) else {
            continue;
        };
        let count = element_children(section).count().max(1);
        report
            .unsupported
            .add(count, label, "Not supported in the DVC-1 tranche");
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
    use std::io::Write;

    use base64::Engine as _;
    use flate2::{write::ZlibEncoder, Compress, Compression, FlushCompress};

    use super::*;

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
        let patch = r#"<PATCH NBFIXTURE="1"><FIXTURES><SSLLIBRARY SSLFIXUID="profile-1" SSLNAME="Test/Dimmer.ssl2"><SSLPROPERTIES SSLBEAMOPENING="20"/><SSLMODES SSLNBMODE="1"><SSLMODE SSLMODEINDEX="0" SSLNBCHANNEL="1"><SSLCHANNEL SSLCHANNELTYPE="7" SSLCHANNELNAME="Dimmer" SSLCHANNELMSB="0" SSLCHANNELLSB="0"><SSLPRESETS><SSLPRESET SSLPRESETNAME="Dimmer" SSLPRESETDMXSTART="0" SSLPRESETDMXEND="255" SSLPRESETDMXDEFAULT="0" SSLPRESETDEFAULTPRESET="1"/></SSLPRESETS></SSLCHANNEL></SSLMODE></SSLMODES></SSLLIBRARY><FIXTURE DASUID="fixture-1" NAME="Dimmer 1" ADDRESS="1" UNIVERS="1" POSX="0" POSY="0" ANGLE="0"/></FIXTURES></PATCH>"#;
        let mut fixture_data = Vec::new();
        fixture_data.extend_from_slice(&1_u16.to_be_bytes());
        fixture_data.extend_from_slice(&255_u16.to_be_bytes());
        fixture_data.extend_from_slice(&1_u32.to_be_bytes());
        fixture_data.extend_from_slice(&[0_u8; DVC_FIXTURE_RECORD_BYTES]);
        let fixture_data =
            base64::engine::general_purpose::STANDARD.encode(sync_flush(&fixture_data));
        format!(
            r##"<DLMFILE TYPE="Daslight" VERSION="5" DASBUILD="test" VERSIONFILE="2"><PATCHS DATA="{}"/><FIXTUREGROUPS><FIXTUREGROUP DASUID="group-1" NAME="All"><FIXTURES><FIXTURE DASUID="fixture-1"/></FIXTURES></FIXTUREGROUP></FIXTUREGROUPS><SCENES><BANK DASUID="bank-1" NAME="Bank 1" COLOR="#ff112233"><SCENE DASUID="scene-1" NAME="Static" COLOR="#ff112233" FADE_IN="100" FADE_OUT="200" LOOP="0" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><FIXTUREDATAS NB="1"><FIXTUREDATA FIXTURE="fixture-1" DATA="{}"/></FIXTUREDATAS></SCENE><SCENE DASUID="scene-2" NAME="Super" COLOR="#ff445566" FADE_IN="0" FADE_OUT="0" LOOP="0" SPEED="1" PLAY_TRIGGER="0" PLAY_DIVISION="1"><FIXTUREDATAS NB="0"/><RACKS><RACK><TIMELINES><TIMELINE DASUID="lane-a" NAME="Audio" INDEX="0" DASTLLOCKED="0" DASTLMUTED="0" DASTLFOLDED="0"><BLOCKS><BLOCK TYPE="2" NAME="Track" START="0" END="1000" POSITION="0" FADEIN="0" FADEOUT="0" DASTLMEDIAPATH="C:/missing.mp3" BPM="120"/></BLOCKS></TIMELINE><TIMELINE DASUID="lane-l" NAME="Lighting" INDEX="1" DASTLLOCKED="0" DASTLMUTED="0" DASTLFOLDED="0"><BLOCKS><BLOCK TYPE="1" NAME="Static" START="0" END="1000" POSITION="0" FADEIN="100" FADEOUT="100" SPEED="1" ALLOWLOOP="1" CONFORM_TO_TEMPO="1" SCENEUUID="scene-1"/></BLOCKS></TIMELINE></TIMELINES></RACK></RACKS></SCENE></BANK></SCENES><SHORTCUTS/><TOUCH/><DEVICES/></DLMFILE>"##,
            qcompress(patch.as_bytes()),
            fixture_data,
        )
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
        let source = synthetic_dvc();
        let outcome = import_bytes(source.as_bytes(), "synthetic.dvc").unwrap();
        crate::validate_project_file(&outcome.project).unwrap();
        assert_eq!(outcome.project.snapshot.fixtures.len(), 1);
        assert_eq!(outcome.project.custom_profiles.len(), 1);
        assert_eq!(outcome.project.snapshot.fixtures[0].universe, 0);
        assert_eq!(outcome.project.snapshot.fixtures[0].address, 1);
        assert_eq!(outcome.project.snapshot.group_colors["Bank 1"], "#112233");
        assert_eq!(outcome.project.snapshot.cues.len(), 2);
        assert_eq!(
            outcome.project.snapshot.cues[0].targets[0].values[0].value,
            65_535
        );
        let child = outcome.project.snapshot.cues[1]
            .child_timeline
            .as_ref()
            .unwrap();
        assert_eq!(child.layers.len(), 2);
        assert_eq!(child.audio_clips.len(), 1);
        assert_eq!(child.events.len(), 1);
        assert!(child.events[0].conform_to_tempo);
        assert!(child.events[0].loop_fill);
        assert_eq!(outcome.report.summary.values_decoded, 1);
        assert_eq!(outcome.report.summary.values_skipped, 0);
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
    }
}
