use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use protocol::{
    AttributeControl, AttributeResolution, ChannelFunctionSummary, DmxModeSummary,
    FixtureProfileSummary, GeometrySummary,
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
    let geometries = parse_geometries(&fixture, &mut warnings);
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
    mode_name
        .and_then(|name| profile.dmx_modes.iter().find(|mode| mode.name == name))
        .or_else(|| profile.dmx_modes.first())
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

fn parse_geometries(fixture: &Node<'_, '_>, warnings: &mut Vec<String>) -> Vec<GeometrySummary> {
    let mut geometries = Vec::new();
    for container in fixture
        .children()
        .filter(|node| node.is_element() && node.has_tag_name("Geometries"))
    {
        for child in container.children().filter(Node::is_element) {
            parse_geometry_node(child, None, &mut geometries, warnings);
        }
    }
    geometries
}

fn parse_geometry_node(
    node: Node<'_, '_>,
    parent: Option<String>,
    geometries: &mut Vec<GeometrySummary>,
    warnings: &mut Vec<String>,
) {
    let kind = node.tag_name().name().to_string();
    if !kind.starts_with("Geometry") {
        return;
    }

    let name = attr(&node, "Name")
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}#{}", kind, geometries.len() + 1));
    let matrix = attr(&node, "Matrix")
        .or_else(|| attr(&node, "Position"))
        .map(parse_matrix)
        .unwrap_or_else(identity_matrix);

    if attr(&node, "Matrix").is_none() && attr(&node, "Position").is_none() {
        warnings.push(format!(
            "Geometry {name} has no Matrix/Position; identity was used"
        ));
    }

    geometries.push(GeometrySummary {
        name: name.clone(),
        kind,
        parent: parent.clone(),
        matrix,
    });

    for child in node.children().filter(Node::is_element) {
        parse_geometry_node(child, Some(name.clone()), geometries, warnings);
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
        for slot in wheel.children().filter(|node| {
            node.is_element() && (node.has_tag_name("Slot") || node.has_tag_name("WheelSlot"))
        }) {
            let Some(slot_name) = attr(&slot, "Name") else {
                continue;
            };
            let info = WheelSlotInfo {
                name: slot_name.to_string(),
                color: attr(&slot, "Color").map(str::to_string),
            };
            slots.insert(format!("{wheel_name}.{slot_name}"), info.clone());
            slots.insert(slot_name.to_string(), info);
        }
    }
    slots
}

#[derive(Debug, Clone)]
struct WheelSlotInfo {
    name: String,
    color: Option<String>,
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
    let resolution = if offsets.len() >= 2 {
        AttributeResolution::SixteenBit
    } else {
        AttributeResolution::EightBit
    };
    let default_value = attr(&channel, "Default")
        .or_else(|| {
            channel
                .descendants()
                .find(|node| node.has_tag_name("ChannelFunction"))
                .and_then(|node| attr(&node, "DMXFrom"))
        })
        .map(|value| parse_dmx_value(value, &resolution))
        .unwrap_or(0);
    let functions = parse_channel_functions(channel, &resolution, wheel_slots);

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

fn parse_channel_functions(
    channel: Node<'_, '_>,
    resolution: &AttributeResolution,
    wheel_slots: &HashMap<String, WheelSlotInfo>,
) -> Vec<ChannelFunctionSummary> {
    let mut functions = channel
        .descendants()
        .filter(|node| node.is_element() && node.has_tag_name("ChannelFunction"))
        .enumerate()
        .map(|(index, node)| {
            let attribute = attr(&node, "Attribute")
                .or_else(|| node.parent().and_then(|parent| attr(&parent, "Attribute")))
                .unwrap_or_else(|| attr(&channel, "Name").unwrap_or("Unknown"))
                .to_string();
            let name = attr(&node, "Name")
                .or_else(|| attr(&node, "OriginalAttribute"))
                .unwrap_or(&attribute)
                .to_string();
            let dmx_from = attr(&node, "DMXFrom")
                .map(|value| parse_dmx_value(value, resolution))
                .unwrap_or(0);
            let dmx_to = attr(&node, "DMXTo").map(|value| parse_dmx_value(value, resolution));
            let wheel_slot = attr(&node, "WheelSlot").map(str::to_string);
            let wheel_slot_info = wheel_slot
                .as_deref()
                .and_then(|reference| wheel_slots.get(reference));
            (
                dmx_from,
                dmx_to,
                ChannelFunctionSummary {
                    name: if name.is_empty() {
                        format!("Function {}", index + 1)
                    } else {
                        name
                    },
                    attribute,
                    dmx_from,
                    dmx_to: dmx_to.unwrap_or(u16::MAX),
                    physical_from: attr(&node, "PhysicalFrom").and_then(parse_f32),
                    physical_to: attr(&node, "PhysicalTo").and_then(parse_f32),
                    wheel_slot,
                    wheel_slot_name: wheel_slot_info.map(|slot| slot.name.clone()),
                    wheel_slot_color: wheel_slot_info.and_then(|slot| slot.color.clone()),
                },
            )
        })
        .collect::<Vec<_>>();

    functions.sort_by_key(|(dmx_from, _, _)| *dmx_from);
    let mut summaries = Vec::with_capacity(functions.len());
    for index in 0..functions.len() {
        let (dmx_from, explicit_dmx_to, mut summary) = functions[index].clone();
        summary.dmx_to = explicit_dmx_to.unwrap_or_else(|| {
            functions
                .get(index + 1)
                .map(|(next_from, _, _)| next_from.saturating_sub(1))
                .unwrap_or(u16::MAX)
        });
        summary.dmx_to = summary.dmx_to.max(dmx_from);
        summaries.push(summary);
    }
    summaries
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

fn parse_dmx_value(value: &str, resolution: &AttributeResolution) -> u16 {
    let mut parts = value.split('/');
    let raw = parts
        .next()
        .and_then(|part| part.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let byte_count = parts.next().and_then(|part| part.trim().parse::<u8>().ok());

    match byte_count {
        Some(0 | 1) => expand_8bit_to_16bit(raw),
        Some(_) => raw.min(u16::MAX as u32) as u16,
        None => match resolution {
            AttributeResolution::EightBit => expand_8bit_to_16bit(raw),
            AttributeResolution::SixteenBit => raw.min(u16::MAX as u32) as u16,
        },
    }
}

fn expand_8bit_to_16bit(value: u32) -> u16 {
    let clamped = value.min(u8::MAX as u32) as u16;
    clamped * 257
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
          <FixtureType Name="Mini Spot" ShortName="MS" Manufacturer="KDMX" FixtureTypeID="abc">
            <Geometries>
              <Geometry Name="Base" Matrix="{1,0,0,0}{0,1,0,0}{0,0,1,0}{0,0,0,1}">
                <GeometryBeam Name="Head" Position="{1,0,0,2}{0,1,0,3}{0,0,1,4}{0,0,0,1}" />
              </Geometry>
            </Geometries>
            <Wheels>
              <Wheel Name="GoboWheel">
                <Slot Name="Slot 1" Color="#ff8800" />
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
                      <ChannelFunction Name="Breakup" Attribute="Gobo1" DMXFrom="64/1" DMXTo="127/1" WheelSlot="GoboWheel.Slot 1" />
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

        assert_eq!(profile.manufacturer, "KDMX");
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
        assert_eq!(profile_mode_footprint(&profile, Some("Standard")), Some(4));
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
            parse_dmx_value("128", &AttributeResolution::EightBit),
            128 * 257
        );
        assert_eq!(
            parse_dmx_value("32768", &AttributeResolution::SixteenBit),
            32_768
        );
    }

    #[test]
    fn loads_description_xml_from_gdtf_zip() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("kdmx-test-{suffix}.gdtf"));

        {
            let file = File::create(&path).unwrap();
            let mut archive = ZipWriter::new(file);
            archive
                .start_file("description.xml", SimpleFileOptions::default())
                .unwrap();
            archive.write_all(SAMPLE_XML.as_bytes()).unwrap();
            archive.finish().unwrap();
        }

        let profile = load_profile(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(profile.name, "Mini Spot");
        assert_eq!(profile.dmx_modes[0].controls[0].offsets, vec![1]);
    }
}
