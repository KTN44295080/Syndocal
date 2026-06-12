use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use protocol::{
    AttributeControl, AttributeResolution, ChannelFunctionSummary, DmxModeSummary,
    FixtureProfileSummary, GeometrySummary, Vec3,
};
use roxmltree::{Document, Node};
use thiserror::Error;
use zip::ZipArchive;

#[derive(Debug, Error)]
pub enum GdtfError {
    #[error("failed to open GDTF file {path}: {source}")]
    Open {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to read GDTF zip archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("description.xml was not found in the GDTF archive")]
    MissingDescription,
    #[error("failed to read description.xml: {0}")]
    ReadDescription(std::io::Error),
    #[error("failed to read wheel media: {0}")]
    ReadWheelMedia(std::io::Error),
    #[error("failed to read model file: {0}")]
    ReadModelFile(std::io::Error),
    #[error("failed to parse description.xml: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("description.xml does not contain a FixtureType node")]
    MissingFixtureType,
}

pub fn load_profile(path: impl AsRef<Path>) -> Result<FixtureProfileSummary, GdtfError> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|source| GdtfError::Open {
        path: path.to_path_buf(),
        source,
    })?;
    let mut archive = ZipArchive::new(file)?;
    let mut description = String::new();

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().replace('\\', "/");
        if name.eq_ignore_ascii_case("description.xml") || name.ends_with("/description.xml") {
            entry
                .read_to_string(&mut description)
                .map_err(GdtfError::ReadDescription)?;
            break;
        }
    }

    if description.is_empty() {
        return Err(GdtfError::MissingDescription);
    }

    parse_description_xml(path.to_string_lossy().as_ref(), &description)
}

pub fn load_wheel_media(
    path: impl AsRef<Path>,
    media_name: &str,
) -> Result<Option<Vec<u8>>, GdtfError> {
    let Some(target_name) = normalized_wheel_media_path(media_name) else {
        return Ok(None);
    };
    let path = path.as_ref();
    let file = File::open(path).map_err(|source| GdtfError::Open {
        path: path.to_path_buf(),
        source,
    })?;
    let mut archive = ZipArchive::new(file)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().replace('\\', "/");
        if name.eq_ignore_ascii_case(&target_name)
            || name
                .to_ascii_lowercase()
                .ends_with(&format!("/{target_name}").to_ascii_lowercase())
        {
            let mut bytes = Vec::with_capacity(entry.size() as usize);
            entry
                .read_to_end(&mut bytes)
                .map_err(GdtfError::ReadWheelMedia)?;
            return Ok(Some(bytes));
        }
    }

    Ok(None)
}

pub fn load_model_file(
    path: impl AsRef<Path>,
    model_file: &str,
) -> Result<Option<Vec<u8>>, GdtfError> {
    let candidates = normalized_model_file_paths(model_file);
    if candidates.is_empty() {
        return Ok(None);
    }
    let path = path.as_ref();
    let file = File::open(path).map_err(|source| GdtfError::Open {
        path: path.to_path_buf(),
        source,
    })?;
    let mut archive = ZipArchive::new(file)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().replace('\\', "/");
        let normalized_name = name.to_ascii_lowercase();
        if candidates.iter().any(|target_name| {
            normalized_name == *target_name || normalized_name.ends_with(&format!("/{target_name}"))
        }) {
            let mut bytes = Vec::with_capacity(entry.size() as usize);
            entry
                .read_to_end(&mut bytes)
                .map_err(GdtfError::ReadModelFile)?;
            return Ok(Some(bytes));
        }
    }

    Ok(None)
}

fn normalized_wheel_media_path(media_name: &str) -> Option<String> {
    let name = media_name.trim().replace('\\', "/");
    if name.is_empty()
        || name.contains('/')
        || name.contains(':')
        || name == "."
        || name == ".."
        || name.starts_with('.')
    {
        return None;
    }
    let file_name = if name.to_ascii_lowercase().ends_with(".png") {
        name
    } else {
        format!("{name}.png")
    };
    Some(format!("wheels/{file_name}"))
}

fn normalized_model_file_paths(model_file: &str) -> Vec<String> {
    let normalized = model_file.trim().replace('\\', "/");
    if normalized.is_empty()
        || normalized.contains(':')
        || normalized.starts_with('/')
        || normalized.starts_with('.')
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Vec::new();
    }

    let lower = normalized.to_ascii_lowercase();
    let mut candidates = vec![lower.clone()];
    if !lower.starts_with("models/") {
        candidates.push(format!("models/{lower}"));
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

pub fn parse_description_xml(
    source_path: &str,
    xml: &str,
) -> Result<FixtureProfileSummary, GdtfError> {
    let document = Document::parse(xml.trim_start())?;
    let fixture = document
        .descendants()
        .find(|node| node.has_tag_name("FixtureType"))
        .ok_or(GdtfError::MissingFixtureType)?;

    let mut warnings = Vec::new();
    let manufacturer = attr(&fixture, "Manufacturer")
        .unwrap_or("Unknown")
        .to_string();
    let name = attr(&fixture, "Name")
        .unwrap_or("Unnamed Fixture")
        .to_string();
    let short_name = attr(&fixture, "ShortName").map(str::to_string);
    let fixture_type_id = attr(&fixture, "FixtureTypeID").map(str::to_string);
    let models = parse_models(&fixture);
    let geometries = parse_geometries(&fixture, &models, &mut warnings);
    let wheel_slots = parse_wheel_slots(&fixture);
    let dmx_modes = parse_dmx_modes(&fixture, &wheel_slots, &mut warnings);

    if dmx_modes.is_empty() {
        warnings.push("No DMXMode nodes were found".to_string());
    }

    Ok(FixtureProfileSummary {
        source_path: source_path.to_string(),
        manufacturer,
        name,
        short_name,
        fixture_type_id,
        dmx_modes,
        geometries,
        warnings,
    })
}

pub fn find_dmx_mode<'a>(
    profile: &'a FixtureProfileSummary,
    mode_name: Option<&str>,
) -> Option<&'a DmxModeSummary> {
    let Some(mode_name) = mode_name.map(str::trim).filter(|name| !name.is_empty()) else {
        return profile.dmx_modes.first();
    };
    profile.dmx_modes.iter().find(|mode| mode.name == mode_name)
}

pub fn dmx_mode_footprint(mode: &DmxModeSummary) -> Option<u16> {
    mode.controls
        .iter()
        .flat_map(|control| control.offsets.iter().copied())
        .max()
}

pub fn profile_mode_footprint(
    profile: &FixtureProfileSummary,
    mode_name: Option<&str>,
) -> Option<u16> {
    find_dmx_mode(profile, mode_name).and_then(dmx_mode_footprint)
}

#[derive(Debug, Clone, PartialEq)]
struct ModelInfo {
    file: Option<String>,
    primitive: Option<String>,
    dimensions: Option<Vec3>,
}

fn parse_models(fixture: &Node<'_, '_>) -> HashMap<String, ModelInfo> {
    let mut models = HashMap::new();
    for model in fixture
        .descendants()
        .filter(|node| node.is_element() && node.has_tag_name("Model"))
    {
        let Some(name) = attr(&model, "Name")
            .map(str::trim)
            .filter(|name| !name.is_empty())
        else {
            continue;
        };
        let dimensions = match (
            attr(&model, "Length").and_then(parse_f32),
            attr(&model, "Width").and_then(parse_f32),
            attr(&model, "Height").and_then(parse_f32),
        ) {
            (Some(length), Some(width), Some(height)) => Some(Vec3 {
                x: length,
                y: height,
                z: width,
            }),
            _ => None,
        };
        models.insert(
            name.to_string(),
            ModelInfo {
                file: attr(&model, "File")
                    .or_else(|| attr(&model, "FileName"))
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                primitive: attr(&model, "PrimitiveType")
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                dimensions,
            },
        );
    }
    models
}

fn parse_geometries(
    fixture: &Node<'_, '_>,
    models: &HashMap<String, ModelInfo>,
    warnings: &mut Vec<String>,
) -> Vec<GeometrySummary> {
    let mut geometries = Vec::new();
    for container in fixture
        .children()
        .filter(|node| node.is_element() && node.has_tag_name("Geometries"))
    {
        for child in container.children().filter(Node::is_element) {
            parse_geometry_node(child, None, models, &mut geometries, warnings);
        }
    }
    geometries
}

fn parse_geometry_node(
    node: Node<'_, '_>,
    parent: Option<String>,
    models: &HashMap<String, ModelInfo>,
    geometries: &mut Vec<GeometrySummary>,
    warnings: &mut Vec<String>,
) {
    let kind = node.tag_name().name().to_string();
    let name = attr(&node, "Name")
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}#{}", kind, geometries.len() + 1));
    let matrix = attr(&node, "Matrix")
        .or_else(|| attr(&node, "Position"))
        .map(parse_matrix)
        .unwrap_or_else(identity_matrix);

    if attr(&node, "Matrix").is_none() && attr(&node, "Position").is_none() {
        warnings.push(format!(
            "Geometry {name} ({kind}) has no Matrix/Position; identity was used"
        ));
    }

    let model_name = attr(&node, "Model")
        .or_else(|| attr(&node, "ModelName"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let model = model_name.as_ref().and_then(|name| models.get(name));

    geometries.push(GeometrySummary {
        name: name.clone(),
        kind,
        parent: parent.clone(),
        matrix,
        model_name,
        model_file: model.and_then(|model| model.file.clone()),
        model_primitive: model.and_then(|model| model.primitive.clone()),
        model_dimensions: model.and_then(|model| model.dimensions),
        beam_type: attr(&node, "BeamType")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        beam_angle_deg: attr(&node, "BeamAngle").and_then(parse_f32),
        field_angle_deg: attr(&node, "FieldAngle").and_then(parse_f32),
        beam_radius: attr(&node, "BeamRadius").and_then(parse_f32),
    });

    for child in node.children().filter(Node::is_element) {
        parse_geometry_node(child, Some(name.clone()), models, geometries, warnings);
    }
}

fn parse_wheel_slots(fixture: &Node<'_, '_>) -> HashMap<String, WheelSlotInfo> {
    let mut slots = HashMap::new();
    for wheel in fixture
        .descendants()
        .filter(|node| node.is_element() && node.has_tag_name("Wheel"))
    {
        let Some(wheel_name) = attr(&wheel, "Name") else {
            continue;
        };
        for (index, slot) in wheel
            .children()
            .filter(|node| {
                node.is_element() && (node.has_tag_name("Slot") || node.has_tag_name("WheelSlot"))
            })
            .enumerate()
        {
            let Some(slot_name) = attr(&slot, "Name") else {
                continue;
            };
            let slot_index = (index + 1) as u32;
            let info = WheelSlotInfo {
                name: slot_name.to_string(),
                color: attr(&slot, "Color").and_then(parse_wheel_slot_color),
                media: attr(&slot, "MediaFileName").map(str::to_string),
            };
            slots.insert(format!("{wheel_name}.{slot_name}"), info.clone());
            slots.insert(format!("{wheel_name}.{slot_index}"), info.clone());
            slots.insert(slot_name.to_string(), info);
        }
    }
    slots
}

#[derive(Debug, Clone)]
struct WheelSlotInfo {
    name: String,
    color: Option<String>,
    media: Option<String>,
}

#[derive(Debug, Clone)]
struct RawChannelFunction {
    dmx_from: u16,
    explicit_dmx_to: Option<u16>,
    summary: ChannelFunctionSummary,
    wheel: Option<String>,
    channel_sets: Vec<RawChannelSet>,
}

#[derive(Debug, Clone)]
struct RawChannelSet {
    name: String,
    dmx_from: u16,
    explicit_dmx_to: Option<u16>,
    physical_from: Option<f32>,
    physical_to: Option<f32>,
    wheel_slot: Option<String>,
    wheel_slot_index: Option<u32>,
}

fn parse_dmx_modes(
    fixture: &Node<'_, '_>,
    wheel_slots: &HashMap<String, WheelSlotInfo>,
    warnings: &mut Vec<String>,
) -> Vec<DmxModeSummary> {
    fixture
        .descendants()
        .filter(|node| node.is_element() && node.has_tag_name("DMXMode"))
        .map(|mode| {
            let mode_name = attr(&mode, "Name").unwrap_or("Default").to_string();
            let controls = mode
                .descendants()
                .filter(|node| node.is_element() && node.has_tag_name("DMXChannel"))
                .filter_map(|channel| parse_dmx_channel(channel, wheel_slots, warnings))
                .collect();

            DmxModeSummary {
                name: mode_name,
                controls,
            }
        })
        .collect()
}

fn parse_dmx_channel(
    channel: Node<'_, '_>,
    wheel_slots: &HashMap<String, WheelSlotInfo>,
    warnings: &mut Vec<String>,
) -> Option<AttributeControl> {
    if !is_supported_dmx_break(&channel, warnings) {
        return None;
    }

    let offsets = attr(&channel, "Offset")
        .map(parse_u16_list)
        .unwrap_or_default();
    if offsets.is_empty() {
        warnings.push(format!(
            "DMXChannel {} has no Offset and was skipped",
            attr(&channel, "Name").unwrap_or("<unnamed>")
        ));
        return None;
    }

    let attribute = channel
        .descendants()
        .find_map(|node| attr(&node, "Attribute"))
        .unwrap_or_else(|| attr(&channel, "Name").unwrap_or("Unknown"))
        .to_string();
    let channel_name = attr(&channel, "Name")
        .map(str::to_string)
        .unwrap_or_else(|| attribute.clone());
    let geometry = attr(&channel, "Geometry").map(str::to_string);
    if offsets.len() > 2 {
        warnings.push(format!(
            "DMXChannel {channel_name} uses {} byte offsets; Rayard stores attributes as 16-bit values and scales them across all fixture bytes",
            offsets.len()
        ));
    }
    let resolution = if offsets.len() >= 2 {
        AttributeResolution::SixteenBit
    } else {
        AttributeResolution::EightBit
    };
    let default_value = parse_channel_default_value(channel, &resolution);
    let functions = parse_channel_functions(channel, &resolution, wheel_slots, warnings);

    Some(AttributeControl {
        attribute,
        channel_name,
        geometry,
        offsets,
        resolution,
        default_value,
        functions,
    })
}

fn is_supported_dmx_break(channel: &Node<'_, '_>, warnings: &mut Vec<String>) -> bool {
    let Some(dmx_break) = attr(channel, "DMXBreak").map(str::trim) else {
        return true;
    };
    if dmx_break.is_empty() || dmx_break == "1" {
        return true;
    }
    let channel_name = attr(channel, "Name").unwrap_or("<unnamed>");
    if dmx_break.eq_ignore_ascii_case("Overwrite") {
        warnings.push(format!(
            "DMXChannel {channel_name} uses DMXBreak=Overwrite; break 1 was assumed"
        ));
        return true;
    }
    warnings.push(format!(
        "DMXChannel {channel_name} uses unsupported DMXBreak {dmx_break} and was skipped"
    ));
    false
}

fn parse_channel_default_value(channel: Node<'_, '_>, resolution: &AttributeResolution) -> u16 {
    if let Some(value) =
        attr(&channel, "Default").and_then(|value| parse_optional_dmx_value(value, resolution))
    {
        return value;
    }

    let initial_function = attr(&channel, "InitialFunction")
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("None"));
    let channel_functions = channel
        .descendants()
        .filter(|node| node.is_element() && node.has_tag_name("ChannelFunction"))
        .collect::<Vec<_>>();
    let selected_function = initial_function
        .and_then(|reference| {
            channel_functions
                .iter()
                .copied()
                .find(|function| channel_function_matches_reference(*function, reference))
        })
        .or_else(|| channel_functions.first().copied());

    selected_function
        .and_then(|function| {
            attr(&function, "Default")
                .or_else(|| attr(&function, "DMXFrom"))
                .and_then(|value| parse_optional_dmx_value(value, resolution))
        })
        .unwrap_or(0)
}

fn channel_function_matches_reference(function: Node<'_, '_>, reference: &str) -> bool {
    attr(&function, "Name")
        .or_else(|| attr(&function, "OriginalAttribute"))
        .or_else(|| attr(&function, "Attribute"))
        .is_some_and(|candidate| matches_node_reference(reference, candidate))
}

fn matches_node_reference(reference: &str, candidate: &str) -> bool {
    let reference = reference.trim();
    let candidate = candidate.trim();
    if reference.eq_ignore_ascii_case(candidate) {
        return true;
    }
    reference
        .rsplit('.')
        .next()
        .is_some_and(|last_segment| last_segment.trim().eq_ignore_ascii_case(candidate))
}

fn parse_channel_functions(
    channel: Node<'_, '_>,
    resolution: &AttributeResolution,
    wheel_slots: &HashMap<String, WheelSlotInfo>,
    warnings: &mut Vec<String>,
) -> Vec<ChannelFunctionSummary> {
    let channel_name = attr(&channel, "Name").unwrap_or("<unnamed>");
    let mut functions = channel
        .descendants()
        .filter(|node| node.is_element() && node.has_tag_name("ChannelFunction"))
        .enumerate()
        .filter_map(|(index, node)| {
            let attribute = attr(&node, "Attribute")
                .or_else(|| node.parent().and_then(|parent| attr(&parent, "Attribute")))
                .unwrap_or_else(|| attr(&channel, "Name").unwrap_or("Unknown"))
                .to_string();
            let name = attr(&node, "Name")
                .or_else(|| attr(&node, "OriginalAttribute"))
                .unwrap_or(&attribute)
                .to_string();
            let Some(dmx_from) =
                attr(&node, "DMXFrom").and_then(|value| parse_optional_dmx_value(value, resolution))
            else {
                let function_name = if name.trim().is_empty() {
                    format!("Function {}", index + 1)
                } else {
                    name.clone()
                };
                warnings.push(format!(
                    "DMXChannel {channel_name} ChannelFunction {function_name} has no parseable DMXFrom and was skipped"
                ));
                return None;
            };
            let dmx_to =
                attr(&node, "DMXTo").and_then(|value| parse_optional_dmx_value(value, resolution));
            let wheel_slot = attr(&node, "WheelSlot").map(str::to_string);
            let wheel = attr(&node, "Wheel")
                .or_else(|| attr(&node, "WheelName"))
                .map(str::to_string);
            let wheel_slot_index = attr(&node, "WheelSlotIndex")
                .and_then(parse_u32)
                .filter(|index| *index > 0);
            let (wheel_slot, wheel_slot_info) = channel_function_wheel_slot(
                wheel_slots,
                wheel.as_deref(),
                wheel_slot.as_deref(),
                wheel_slot_index,
            );
            let channel_sets = node
                .children()
                .filter(|child| child.is_element() && child.has_tag_name("ChannelSet"))
                .enumerate()
                .map(|(set_index, set)| RawChannelSet {
                    name: attr(&set, "Name")
                        .map(str::to_string)
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or_else(|| format!("{} Set {}", name, set_index + 1)),
                    dmx_from: attr(&set, "DMXFrom")
                        .and_then(|value| parse_optional_dmx_value(value, resolution))
                        .unwrap_or(dmx_from),
                    explicit_dmx_to: attr(&set, "DMXTo")
                        .and_then(|value| parse_optional_dmx_value(value, resolution)),
                    physical_from: attr(&set, "PhysicalFrom").and_then(parse_f32),
                    physical_to: attr(&set, "PhysicalTo").and_then(parse_f32),
                    wheel_slot: attr(&set, "WheelSlot").map(str::to_string),
                    wheel_slot_index: attr(&set, "WheelSlotIndex")
                        .and_then(parse_u32)
                        .filter(|index| *index > 0),
                })
                .collect();
            Some(RawChannelFunction {
                dmx_from,
                explicit_dmx_to: dmx_to,
                summary: ChannelFunctionSummary {
                    name: if name.is_empty() {
                        format!("Function {}", index + 1)
                    } else {
                        name
                    },
                    attribute,
                    parent_function: None,
                    dmx_from,
                    dmx_to: dmx_to.unwrap_or(u16::MAX),
                    physical_from: attr(&node, "PhysicalFrom").and_then(parse_f32),
                    physical_to: attr(&node, "PhysicalTo").and_then(parse_f32),
                    wheel_slot,
                    wheel_slot_name: wheel_slot_info.map(|slot| slot.name.clone()),
                    wheel_slot_color: wheel_slot_info.and_then(|slot| slot.color.clone()),
                    wheel_slot_media: wheel_slot_info.and_then(|slot| slot.media.clone()),
                },
                wheel,
                channel_sets,
            })
        })
        .collect::<Vec<_>>();

    functions.sort_by_key(|function| function.dmx_from);
    let mut summaries = Vec::new();
    for index in 0..functions.len() {
        let mut function = functions[index].clone();
        function.summary.dmx_to = function.explicit_dmx_to.unwrap_or_else(|| {
            functions
                .get(index + 1)
                .map(|next| next.dmx_from.saturating_sub(1))
                .unwrap_or(u16::MAX)
        });
        function.summary.dmx_to = function.summary.dmx_to.max(function.dmx_from);
        let channel_sets = expand_channel_sets(&function, wheel_slots);
        if channel_sets.is_empty() {
            summaries.push(function.summary);
        } else {
            summaries.extend(channel_sets);
        }
    }
    summaries
}

fn expand_channel_sets(
    function: &RawChannelFunction,
    wheel_slots: &HashMap<String, WheelSlotInfo>,
) -> Vec<ChannelFunctionSummary> {
    let mut sets = function.channel_sets.clone();
    sets.sort_by_key(|set| set.dmx_from);
    let mut summaries = Vec::with_capacity(sets.len());
    for index in 0..sets.len() {
        let set = &sets[index];
        let dmx_to = set
            .explicit_dmx_to
            .unwrap_or_else(|| {
                sets.get(index + 1)
                    .map(|next| next.dmx_from.saturating_sub(1))
                    .unwrap_or(function.summary.dmx_to)
            })
            .max(set.dmx_from);
        let (wheel_slot, wheel_slot_info) =
            channel_set_wheel_slot(wheel_slots, function.wheel.as_deref(), set);
        summaries.push(ChannelFunctionSummary {
            name: set.name.clone(),
            attribute: function.summary.attribute.clone(),
            parent_function: Some(function.summary.name.clone()),
            dmx_from: set.dmx_from,
            dmx_to,
            physical_from: set.physical_from.or(function.summary.physical_from),
            physical_to: set.physical_to.or(function.summary.physical_to),
            wheel_slot,
            wheel_slot_name: wheel_slot_info.map(|slot| slot.name.clone()),
            wheel_slot_color: wheel_slot_info.and_then(|slot| slot.color.clone()),
            wheel_slot_media: wheel_slot_info.and_then(|slot| slot.media.clone()),
        });
    }
    summaries
}

fn channel_set_wheel_slot<'a>(
    wheel_slots: &'a HashMap<String, WheelSlotInfo>,
    wheel: Option<&str>,
    set: &RawChannelSet,
) -> (Option<String>, Option<&'a WheelSlotInfo>) {
    if let Some(reference) = set.wheel_slot.as_deref() {
        return (
            Some(reference.to_string()),
            wheel_slots.get(reference).or_else(|| {
                wheel.and_then(|wheel_name| wheel_slots.get(&format!("{wheel_name}.{reference}")))
            }),
        );
    }

    let Some(slot_index) = set.wheel_slot_index else {
        return (None, None);
    };
    let Some(wheel_name) = wheel else {
        return (Some(slot_index.to_string()), None);
    };
    let reference = format!("{wheel_name}.{slot_index}");
    let slot_info = wheel_slots.get(&reference);
    let display_reference = slot_info
        .map(|slot| format!("{wheel_name}.{}", slot.name))
        .unwrap_or_else(|| reference.clone());
    (Some(display_reference), slot_info)
}

fn channel_function_wheel_slot<'a>(
    wheel_slots: &'a HashMap<String, WheelSlotInfo>,
    wheel: Option<&str>,
    wheel_slot: Option<&str>,
    wheel_slot_index: Option<u32>,
) -> (Option<String>, Option<&'a WheelSlotInfo>) {
    if let Some(reference) = wheel_slot {
        let slot_info = wheel_slots.get(reference).or_else(|| {
            wheel.and_then(|wheel_name| wheel_slots.get(&format!("{wheel_name}.{reference}")))
        });
        let display_reference = if reference.contains('.') {
            reference.to_string()
        } else {
            slot_info
                .and_then(|slot| wheel.map(|wheel_name| format!("{wheel_name}.{}", slot.name)))
                .unwrap_or_else(|| reference.to_string())
        };
        return (Some(display_reference), slot_info);
    }

    let Some(slot_index) = wheel_slot_index else {
        return (None, None);
    };
    let Some(wheel_name) = wheel else {
        return (Some(slot_index.to_string()), None);
    };
    let reference = format!("{wheel_name}.{slot_index}");
    let slot_info = wheel_slots.get(&reference);
    let display_reference = slot_info
        .map(|slot| format!("{wheel_name}.{}", slot.name))
        .unwrap_or_else(|| reference.clone());
    (Some(display_reference), slot_info)
}

fn attr<'a, 'input>(node: &Node<'a, 'input>, name: &str) -> Option<&'a str> {
    node.attribute(name)
}

fn parse_matrix(value: &str) -> [f32; 16] {
    let mut matrix = identity_matrix();
    for (index, number) in value
        .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == ';' || ch == '{' || ch == '}')
        .filter_map(|part| part.parse::<f32>().ok())
        .take(16)
        .enumerate()
    {
        matrix[index] = number;
    }
    matrix
}

fn parse_wheel_slot_color(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.starts_with('#')
        && trimmed.len() == 7
        && trimmed.chars().skip(1).all(|ch| ch.is_ascii_hexdigit())
    {
        return Some(trimmed.to_ascii_lowercase());
    }

    let values = trimmed
        .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == ';' || ch == '{' || ch == '}')
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<f32>().ok())
        .collect::<Vec<_>>();
    if values.len() < 3 {
        return None;
    }
    cie_xyy_to_srgb_hex(values[0], values[1], values[2])
}

fn cie_xyy_to_srgb_hex(x: f32, y: f32, luminance: f32) -> Option<String> {
    if !x.is_finite() || !y.is_finite() || !luminance.is_finite() || y <= 0.0 || luminance <= 0.0 {
        return None;
    }

    let y_luma = if luminance > 1.0 {
        luminance / 100.0
    } else {
        luminance
    }
    .clamp(0.0, 1.0);
    let x_xyz = (x * y_luma) / y;
    let z_xyz = ((1.0 - x - y) * y_luma) / y;

    let red_linear = 3.2406 * x_xyz - 1.5372 * y_luma - 0.4986 * z_xyz;
    let green_linear = -0.9689 * x_xyz + 1.8758 * y_luma + 0.0415 * z_xyz;
    let blue_linear = 0.0557 * x_xyz - 0.2040 * y_luma + 1.0570 * z_xyz;

    Some(format!(
        "#{:02x}{:02x}{:02x}",
        srgb_byte(red_linear),
        srgb_byte(green_linear),
        srgb_byte(blue_linear)
    ))
}

fn srgb_byte(linear: f32) -> u8 {
    let value = linear.clamp(0.0, 1.0);
    let srgb = if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (srgb.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

fn parse_u16_list(value: &str) -> Vec<u16> {
    value
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u16>().ok())
        .collect()
}

fn parse_f32(value: &str) -> Option<f32> {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

fn parse_u32(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok()
}

fn parse_optional_dmx_value(value: &str, resolution: &AttributeResolution) -> Option<u16> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("None") {
        return None;
    }
    parse_dmx_value_checked(trimmed, resolution)
}

#[cfg(test)]
fn parse_dmx_value(value: &str, resolution: &AttributeResolution) -> u16 {
    parse_dmx_value_checked(value, resolution).unwrap_or(0)
}

fn parse_dmx_value_checked(value: &str, resolution: &AttributeResolution) -> Option<u16> {
    let mut parts = value.split('/');
    let raw = parts.next()?.trim().parse::<u128>().ok()?;
    let byte_count = match parts.next() {
        Some(part) => Some(part.trim().parse::<u8>().ok()?),
        None => None,
    };
    if parts.next().is_some() {
        return None;
    }

    Some(match byte_count {
        Some(0 | 1) => expand_8bit_to_16bit(raw),
        Some(2) => raw.min(u16::MAX as u128) as u16,
        Some(bytes) => scale_dmx_value_to_16bit(raw, bytes),
        None => match resolution {
            AttributeResolution::EightBit => expand_8bit_to_16bit(raw),
            AttributeResolution::SixteenBit => raw.min(u16::MAX as u128) as u16,
        },
    })
}

fn expand_8bit_to_16bit(value: u128) -> u16 {
    let clamped = value.min(u8::MAX as u128) as u16;
    clamped * 257
}

fn scale_dmx_value_to_16bit(value: u128, byte_count: u8) -> u16 {
    let bits = (byte_count as u32).saturating_mul(8);
    let source_max = if bits >= u128::BITS {
        u128::MAX
    } else {
        (1u128 << bits) - 1
    };
    if source_max <= u16::MAX as u128 {
        return value.min(u16::MAX as u128) as u16;
    }
    let clamped = value.min(source_max);
    ((clamped * u16::MAX as u128 + source_max / 2) / source_max) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };
    use zip::{write::SimpleFileOptions, ZipWriter};

    const SAMPLE_XML: &str = r##"
        <?xml version="1.0" encoding="UTF-8"?>
        <GDTF DataVersion="1.2">
          <FixtureType Name="Mini Spot" ShortName="MS" Manufacturer="Rayard" FixtureTypeID="abc">
            <Geometries>
              <Geometry Name="Base" Matrix="{1,0,0,0}{0,1,0,0}{0,0,1,0}{0,0,0,1}">
                <GeometryBeam Name="Head" Position="{1,0,0,2}{0,1,0,3}{0,0,1,4}{0,0,0,1}" />
              </Geometry>
            </Geometries>
            <Wheels>
              <Wheel Name="GoboWheel">
                <Slot Name="Slot 1" Color="#ff8800" MediaFileName="breakup_01" />
                <Slot Name="Slot 2" Color="{0.3127,0.3290,100.0}" MediaFileName="dots_02" />
              </Wheel>
            </Wheels>
            <DMXModes>
              <DMXMode Name="Standard">
                <DMXChannels>
                  <DMXChannel Name="Base_Dimmer" Geometry="Base" Offset="1">
                    <LogicalChannel Attribute="Dimmer">
                      <ChannelFunction Name="Dimmer low" Attribute="Dimmer" DMXFrom="128/1" />
                      <ChannelFunction Name="Dimmer high" Attribute="Dimmer" DMXFrom="200/1" DMXTo="255/1" PhysicalFrom="0.75" PhysicalTo="1.0" />
                    </LogicalChannel>
                  </DMXChannel>
                  <DMXChannel Name="Head_Pan" Geometry="Head" Offset="2,3">
                    <LogicalChannel Attribute="Pan">
                      <ChannelFunction Attribute="Pan" DMXFrom="0/2" />
                    </LogicalChannel>
                  </DMXChannel>
                  <DMXChannel Name="Head_Gobo" Geometry="Head" Offset="4">
                    <LogicalChannel Attribute="Gobo1">
                      <ChannelFunction Name="Open" Attribute="Gobo1" DMXFrom="0/1" />
                      <ChannelFunction Name="Gobo Select" Attribute="Gobo1" DMXFrom="64/1" DMXTo="127/1" Wheel="GoboWheel" PhysicalFrom="0.0" PhysicalTo="1.0">
                        <ChannelSet Name="Breakup" DMXFrom="64/1" WheelSlotIndex="1" />
                        <ChannelSet Name="Dots" DMXFrom="96/1" PhysicalFrom="0.5" PhysicalTo="1.0" WheelSlotIndex="2" />
                      </ChannelFunction>
                    </LogicalChannel>
                  </DMXChannel>
                </DMXChannels>
              </DMXMode>
            </DMXModes>
          </FixtureType>
        </GDTF>
    "##;

    #[test]
    fn parses_modes_channels_and_geometry() {
        let profile = parse_description_xml("fixture.gdtf", SAMPLE_XML).unwrap();

        assert_eq!(profile.manufacturer, "Rayard");
        assert_eq!(profile.name, "Mini Spot");
        assert_eq!(profile.geometries.len(), 2);
        assert_eq!(profile.dmx_modes[0].controls.len(), 3);
        assert_eq!(profile.dmx_modes[0].controls[0].attribute, "Dimmer");
        assert_eq!(profile.dmx_modes[0].controls[0].default_value, 128 * 257);
        assert_eq!(profile.dmx_modes[0].controls[0].functions.len(), 2);
        assert_eq!(
            profile.dmx_modes[0].controls[0].functions[0].dmx_to,
            200 * 257 - 1
        );
        assert_eq!(
            profile.dmx_modes[0].controls[0].functions[1].physical_to,
            Some(1.0)
        );
        assert_eq!(
            profile.dmx_modes[0].controls[1].resolution,
            AttributeResolution::SixteenBit
        );
        assert_eq!(profile.dmx_modes[0].controls[2].functions.len(), 3);
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[1].name,
            "Breakup"
        );
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[1].parent_function,
            Some("Gobo Select".to_string())
        );
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[1].dmx_to,
            96 * 257 - 1
        );
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[1].wheel_slot,
            Some("GoboWheel.Slot 1".to_string())
        );
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[1].wheel_slot_name,
            Some("Slot 1".to_string())
        );
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[1].wheel_slot_color,
            Some("#ff8800".to_string())
        );
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[1].wheel_slot_media,
            Some("breakup_01".to_string())
        );
        assert_eq!(profile.dmx_modes[0].controls[2].functions[2].name, "Dots");
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[2].dmx_to,
            127 * 257
        );
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[2].physical_from,
            Some(0.5)
        );
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[2].wheel_slot_name,
            Some("Slot 2".to_string())
        );
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[2].wheel_slot_color,
            Some("#ffffff".to_string())
        );
        assert_eq!(
            profile.dmx_modes[0].controls[2].functions[2].wheel_slot_media,
            Some("dots_02".to_string())
        );
        assert_eq!(profile_mode_footprint(&profile, Some("Standard")), Some(4));
    }

    #[test]
    fn requested_dmx_mode_must_exist_when_named() {
        let profile = parse_description_xml("fixture.gdtf", SAMPLE_XML).unwrap();

        assert_eq!(find_dmx_mode(&profile, None).unwrap().name, "Standard");
        assert_eq!(find_dmx_mode(&profile, Some("")).unwrap().name, "Standard");
        assert!(find_dmx_mode(&profile, Some("Missing")).is_none());
        assert_eq!(profile_mode_footprint(&profile, Some("Missing")), None);
    }

    #[test]
    fn parses_non_geometry_prefixed_gdtf_geometry_nodes() {
        let xml = r#"
            <GDTF>
              <FixtureType Name="Moving Head" Manufacturer="Rayard">
                <Models>
                  <Model Name="LensModel" File="models/lens.glb" PrimitiveType="Cylinder"
                         Length="0.2" Width="0.12" Height="0.08" />
                </Models>
                <Geometries>
                  <Geometry Name="Base">
                    <Axis Name="PanAxis" Matrix="{1,0,0,1}{0,1,0,2}{0,0,1,3}{0,0,0,1}">
                      <Axis Name="TiltAxis">
                        <Beam Name="Lens" Model="LensModel"
                              Position="{1,0,0,4}{0,1,0,5}{0,0,1,6}{0,0,0,1}"
                              BeamType="Wash" BeamAngle="12.5" FieldAngle="18" BeamRadius="0.07" />
                      </Axis>
                    </Axis>
                  </Geometry>
                </Geometries>
                <DMXModes>
                  <DMXMode Name="Standard">
                    <DMXChannels />
                  </DMXMode>
                </DMXModes>
              </FixtureType>
            </GDTF>
        "#;

        let profile = parse_description_xml("moving-head.gdtf", xml).unwrap();
        let names = profile
            .geometries
            .iter()
            .map(|geometry| geometry.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["Base", "PanAxis", "TiltAxis", "Lens"]);
        assert_eq!(profile.geometries[1].kind, "Axis");
        assert_eq!(profile.geometries[1].parent, Some("Base".to_string()));
        assert_eq!(profile.geometries[2].kind, "Axis");
        assert_eq!(profile.geometries[2].parent, Some("PanAxis".to_string()));
        assert_eq!(profile.geometries[3].kind, "Beam");
        assert_eq!(profile.geometries[3].parent, Some("TiltAxis".to_string()));
        assert_eq!(profile.geometries[1].matrix[3], 1.0);
        assert_eq!(profile.geometries[3].matrix[3], 4.0);
        assert_eq!(
            profile.geometries[3].model_name.as_deref(),
            Some("LensModel")
        );
        assert_eq!(
            profile.geometries[3].model_file.as_deref(),
            Some("models/lens.glb")
        );
        assert_eq!(
            profile.geometries[3].model_primitive.as_deref(),
            Some("Cylinder")
        );
        assert_eq!(
            profile.geometries[3].model_dimensions,
            Some(Vec3 {
                x: 0.2,
                y: 0.08,
                z: 0.12
            })
        );
        assert_eq!(profile.geometries[3].beam_type.as_deref(), Some("Wash"));
        assert_eq!(profile.geometries[3].beam_angle_deg, Some(12.5));
        assert_eq!(profile.geometries[3].field_angle_deg, Some(18.0));
        assert_eq!(profile.geometries[3].beam_radius, Some(0.07));
    }

    #[test]
    fn channel_function_wheel_slot_index_resolves_slot_metadata() {
        let xml = r##"
            <GDTF>
              <FixtureType Name="Color Wheel" Manufacturer="Rayard">
                <Wheels>
                  <Wheel Name="ColorWheel">
                    <WheelSlot Name="Red" Color="#ff0000" MediaFileName="red_filter" />
                    <WheelSlot Name="Blue" Color="#0000ff" MediaFileName="blue_filter.png" />
                  </Wheel>
                </Wheels>
                <DMXModes>
                  <DMXMode Name="Standard">
                    <DMXChannels>
                      <DMXChannel Name="Color" Offset="1">
                        <LogicalChannel Attribute="Color1">
                          <ChannelFunction Name="Red Slot" Attribute="Color1" DMXFrom="10/1" Wheel="ColorWheel" WheelSlotIndex="1" />
                          <ChannelFunction Name="Blue Slot" Attribute="Color1" DMXFrom="20/1" WheelName="ColorWheel" WheelSlotIndex="2" />
                        </LogicalChannel>
                      </DMXChannel>
                    </DMXChannels>
                  </DMXMode>
                </DMXModes>
              </FixtureType>
            </GDTF>
        "##;

        let profile = parse_description_xml("color.gdtf", xml).unwrap();
        let functions = &profile.dmx_modes[0].controls[0].functions;

        assert_eq!(functions[0].wheel_slot, Some("ColorWheel.Red".to_string()));
        assert_eq!(functions[0].wheel_slot_name, Some("Red".to_string()));
        assert_eq!(functions[0].wheel_slot_color, Some("#ff0000".to_string()));
        assert_eq!(
            functions[0].wheel_slot_media,
            Some("red_filter".to_string())
        );
        assert_eq!(functions[1].wheel_slot, Some("ColorWheel.Blue".to_string()));
        assert_eq!(functions[1].wheel_slot_name, Some("Blue".to_string()));
        assert_eq!(functions[1].wheel_slot_color, Some("#0000ff".to_string()));
        assert_eq!(
            functions[1].wheel_slot_media,
            Some("blue_filter.png".to_string())
        );
    }

    #[test]
    fn parses_gdtf_dmx_values_into_internal_16bit_values() {
        assert_eq!(parse_dmx_value("0/1", &AttributeResolution::EightBit), 0);
        assert_eq!(
            parse_dmx_value("255/1", &AttributeResolution::EightBit),
            u16::MAX
        );
        assert_eq!(
            parse_dmx_value("128/1", &AttributeResolution::EightBit),
            128 * 257
        );
        assert_eq!(
            parse_dmx_value("4660/2", &AttributeResolution::SixteenBit),
            0x1234
        );
        assert_eq!(
            parse_dmx_value("8388608/3", &AttributeResolution::SixteenBit),
            32_768
        );
        assert_eq!(
            parse_dmx_value("2147483648/4", &AttributeResolution::SixteenBit),
            32_768
        );
        assert_eq!(
            parse_dmx_value("4294967295/4", &AttributeResolution::SixteenBit),
            u16::MAX
        );
        assert_eq!(
            parse_dmx_value("128", &AttributeResolution::EightBit),
            128 * 257
        );
        assert_eq!(
            parse_dmx_value("32768", &AttributeResolution::SixteenBit),
            32_768
        );
        assert_eq!(
            parse_optional_dmx_value("not-a-dmx-value", &AttributeResolution::EightBit),
            None
        );
        assert_eq!(
            parse_optional_dmx_value("12/1/extra", &AttributeResolution::EightBit),
            None
        );
    }

    #[test]
    fn parses_multibyte_dmx_channels_as_scaled_16bit_controls() {
        let xml = r#"
            <GDTF>
              <FixtureType Name="Extended Resolution" Manufacturer="Rayard">
                <DMXModes>
                  <DMXMode Name="Standard">
                    <DMXChannels>
                      <DMXChannel Name="Animation" Offset="1,2,3" Default="8388608/3">
                        <LogicalChannel Attribute="Animation">
                          <ChannelFunction Name="Animation" Attribute="Animation" DMXFrom="0/3" DMXTo="16777215/3" />
                        </LogicalChannel>
                      </DMXChannel>
                    </DMXChannels>
                  </DMXMode>
                </DMXModes>
              </FixtureType>
            </GDTF>
        "#;

        let profile = parse_description_xml("extended.gdtf", xml).unwrap();
        let control = &profile.dmx_modes[0].controls[0];

        assert_eq!(control.offsets, vec![1, 2, 3]);
        assert_eq!(control.resolution, AttributeResolution::SixteenBit);
        assert_eq!(control.default_value, 32_768);
        assert_eq!(control.functions[0].dmx_to, u16::MAX);
        assert!(profile
            .warnings
            .iter()
            .any(|warning| warning.contains("scales them across all fixture bytes")));
    }

    #[test]
    fn skips_channel_functions_without_parseable_dmx_from() {
        let xml = r#"
            <GDTF>
              <FixtureType Name="Bad Function Ranges" Manufacturer="Rayard">
                <DMXModes>
                  <DMXMode Name="Standard">
                    <DMXChannels>
                      <DMXChannel Name="Color" Offset="1">
                        <LogicalChannel Attribute="Color1">
                          <ChannelFunction Name="Missing From" Attribute="Color1" />
                          <ChannelFunction Name="Bad From" Attribute="Color1" DMXFrom="abc" />
                          <ChannelFunction Name="Good Slot" Attribute="Color1" DMXFrom="10/1" DMXTo="20/1" />
                        </LogicalChannel>
                      </DMXChannel>
                    </DMXChannels>
                  </DMXMode>
                </DMXModes>
              </FixtureType>
            </GDTF>
        "#;

        let profile = parse_description_xml("bad-functions.gdtf", xml).unwrap();
        let functions = &profile.dmx_modes[0].controls[0].functions;

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "Good Slot");
        assert_eq!(functions[0].dmx_from, 10 * 257);
        assert_eq!(functions[0].dmx_to, 20 * 257);
        assert!(profile.warnings.iter().any(
            |warning| warning.contains("ChannelFunction Missing From has no parseable DMXFrom")
        ));
        assert!(profile
            .warnings
            .iter()
            .any(|warning| warning.contains("ChannelFunction Bad From has no parseable DMXFrom")));
    }

    #[test]
    fn uses_channel_function_default_for_initial_channel_state() {
        let xml = SAMPLE_XML.replace(
            r#"<ChannelFunction Name="Dimmer low" Attribute="Dimmer" DMXFrom="128/1" />"#,
            r#"<ChannelFunction Name="Dimmer low" Attribute="Dimmer" DMXFrom="128/1" Default="140/1" />"#,
        );

        let profile = parse_description_xml("fixture.gdtf", &xml).unwrap();

        assert_eq!(profile.dmx_modes[0].controls[0].default_value, 140 * 257);
    }

    #[test]
    fn uses_initial_function_default_when_channel_default_is_absent() {
        let xml = SAMPLE_XML.replace(
            r#"<DMXChannel Name="Head_Pan" Geometry="Head" Offset="2,3">
                    <LogicalChannel Attribute="Pan">
                      <ChannelFunction Attribute="Pan" DMXFrom="0/2" />
                    </LogicalChannel>
                  </DMXChannel>"#,
            r#"<DMXChannel Name="Head_Pan" Geometry="Head" Offset="2,3" InitialFunction="Pan.Fast">
                    <LogicalChannel Attribute="Pan">
                      <ChannelFunction Name="Slow" Attribute="Pan" DMXFrom="0/2" Default="1000/2" />
                      <ChannelFunction Name="Fast" Attribute="Pan" DMXFrom="10000/2" Default="20000/2" />
                    </LogicalChannel>
                  </DMXChannel>"#,
        );

        let profile = parse_description_xml("fixture.gdtf", &xml).unwrap();

        assert_eq!(profile.dmx_modes[0].controls[1].default_value, 20_000);
    }

    #[test]
    fn channel_default_overrides_initial_function_default() {
        let xml = SAMPLE_XML.replace(
            r#"<DMXChannel Name="Head_Pan" Geometry="Head" Offset="2,3">
                    <LogicalChannel Attribute="Pan">
                      <ChannelFunction Attribute="Pan" DMXFrom="0/2" />
                    </LogicalChannel>
                  </DMXChannel>"#,
            r#"<DMXChannel Name="Head_Pan" Geometry="Head" Offset="2,3" Default="32768/2" InitialFunction="Pan.Fast">
                    <LogicalChannel Attribute="Pan">
                      <ChannelFunction Name="Slow" Attribute="Pan" DMXFrom="0/2" Default="1000/2" />
                      <ChannelFunction Name="Fast" Attribute="Pan" DMXFrom="10000/2" Default="20000/2" />
                    </LogicalChannel>
                  </DMXChannel>"#,
        );

        let profile = parse_description_xml("fixture.gdtf", &xml).unwrap();

        assert_eq!(profile.dmx_modes[0].controls[1].default_value, 32_768);
    }

    #[test]
    fn warns_and_scales_high_resolution_dmx_channels_to_internal_16bit() {
        let xml = SAMPLE_XML.replace(
            r#"<DMXChannel Name="Head_Pan" Geometry="Head" Offset="2,3">"#,
            r#"<DMXChannel Name="Head_Pan" Geometry="Head" Offset="2,3,4" Default="8388608/3">"#,
        );

        let profile = parse_description_xml("fixture.gdtf", &xml).unwrap();
        let control = &profile.dmx_modes[0].controls[1];

        assert_eq!(control.resolution, AttributeResolution::SixteenBit);
        assert_eq!(control.offsets, vec![2, 3, 4]);
        assert_eq!(control.default_value, 32_768);
        assert_eq!(profile_mode_footprint(&profile, Some("Standard")), Some(4));
        assert!(profile
            .warnings
            .iter()
            .any(|warning| warning.contains("3 byte offsets")));
    }

    #[test]
    fn skips_unsupported_dmx_breaks_instead_of_mapping_them_to_primary_patch() {
        let xml = SAMPLE_XML.replace(
            r#"<DMXChannel Name="Head_Pan" Geometry="Head" Offset="2,3">"#,
            r#"<DMXChannel Name="Head_Pan" Geometry="Head" Offset="2,3" DMXBreak="2">"#,
        );

        let profile = parse_description_xml("fixture.gdtf", &xml).unwrap();

        assert_eq!(profile.dmx_modes[0].controls.len(), 2);
        assert!(!profile.dmx_modes[0]
            .controls
            .iter()
            .any(|control| control.attribute == "Pan"));
        assert!(profile
            .warnings
            .iter()
            .any(|warning| warning.contains("unsupported DMXBreak 2")));
    }

    #[test]
    fn treats_dmx_break_overwrite_as_primary_break_with_warning() {
        let xml = SAMPLE_XML.replace(
            r#"<DMXChannel Name="Head_Pan" Geometry="Head" Offset="2,3">"#,
            r#"<DMXChannel Name="Head_Pan" Geometry="Head" Offset="2,3" DMXBreak="Overwrite">"#,
        );

        let profile = parse_description_xml("fixture.gdtf", &xml).unwrap();

        assert_eq!(profile.dmx_modes[0].controls.len(), 3);
        assert!(profile.dmx_modes[0]
            .controls
            .iter()
            .any(|control| control.attribute == "Pan"));
        assert!(profile
            .warnings
            .iter()
            .any(|warning| warning.contains("DMXBreak=Overwrite")));
    }

    #[test]
    fn parses_wheel_slot_color_formats() {
        assert_eq!(
            parse_wheel_slot_color("#Ff8800"),
            Some("#ff8800".to_string())
        );
        assert_eq!(
            parse_wheel_slot_color("{0.3127, 0.3290, 100.0}"),
            Some("#ffffff".to_string())
        );
        assert_eq!(parse_wheel_slot_color("not-a-color"), None);
    }

    #[test]
    fn loads_description_xml_from_gdtf_zip() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rayard-test-{suffix}.gdtf"));

        {
            let file = File::create(&path).unwrap();
            let mut archive = ZipWriter::new(file);
            archive
                .start_file("description.xml", SimpleFileOptions::default())
                .unwrap();
            archive.write_all(SAMPLE_XML.as_bytes()).unwrap();
            archive
                .start_file("wheels/breakup_01.png", SimpleFileOptions::default())
                .unwrap();
            archive.write_all(&[0x89, b'P', b'N', b'G']).unwrap();
            archive
                .start_file("models/lens.glb", SimpleFileOptions::default())
                .unwrap();
            archive.write_all(&[b'g', b'l', b'T', b'F']).unwrap();
            archive.finish().unwrap();
        }

        let profile = load_profile(&path).unwrap();
        let media = load_wheel_media(&path, "breakup_01").unwrap();
        let media_with_extension = load_wheel_media(&path, "breakup_01.png").unwrap();
        let invalid_media = load_wheel_media(&path, "../breakup_01").unwrap();
        let model = load_model_file(&path, "models/lens.glb").unwrap();
        let model_without_folder = load_model_file(&path, "lens.glb").unwrap();
        let invalid_relative_model = load_model_file(&path, "../lens.glb").unwrap();
        let invalid_absolute_model = load_model_file(&path, "/models/lens.glb").unwrap();
        let invalid_drive_model = load_model_file(&path, "C:/models/lens.glb").unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(profile.name, "Mini Spot");
        assert_eq!(profile.dmx_modes[0].controls[0].offsets, vec![1]);
        assert_eq!(media, Some(vec![0x89, b'P', b'N', b'G']));
        assert_eq!(media_with_extension, Some(vec![0x89, b'P', b'N', b'G']));
        assert_eq!(invalid_media, None);
        assert_eq!(model, Some(vec![b'g', b'l', b'T', b'F']));
        assert_eq!(model_without_folder, Some(vec![b'g', b'l', b'T', b'F']));
        assert_eq!(invalid_relative_model, None);
        assert_eq!(invalid_absolute_model, None);
        assert_eq!(invalid_drive_model, None);
    }
}
