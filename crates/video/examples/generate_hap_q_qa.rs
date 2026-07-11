use std::{env, fs, io::Cursor, path::PathBuf};

use mp4::{AvcConfig, Bytes, FourCC, Mp4Config, Mp4Sample, Mp4Writer, TrackConfig};
use serde_json::json;

const WIDTH: u16 = 640;
const HEIGHT: u16 = 360;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/syndocal-hap-q-qa.mov"));
    let config = Mp4Config {
        major_brand: FourCC::from(*b"isom"),
        minor_version: 512,
        compatible_brands: vec![FourCC::from(*b"isom"), FourCC::from(*b"mp41")],
        timescale: 1000,
    };
    let mut writer = Mp4Writer::write_start(Cursor::new(Vec::new()), &config)?;
    writer.add_track(&TrackConfig::from(AvcConfig {
        width: WIDTH,
        height: HEIGHT,
        seq_param_set: vec![0x67, 0x42, 0x00, 0x1e],
        pic_param_set: vec![0x68, 0xce, 0x06, 0xe2],
    }))?;
    for (index, color) in [0xca80u16, 0x4d00u16].into_iter().enumerate() {
        let frame = hap_q_frame(color, if index == 0 { 180 } else { 110 });
        writer.write_sample(
            1,
            &Mp4Sample {
                start_time: index as u64 * 500,
                duration: 500,
                rendering_offset: 0,
                is_sync: true,
                bytes: Bytes::from(frame),
            },
        )?;
    }
    writer.write_end()?;
    let mut bytes = writer.into_writer().into_inner();
    let mut replacements = 0;
    for index in 0..bytes.len().saturating_sub(3) {
        if bytes[index..index + 4] == *b"avc1" {
            bytes[index..index + 4].copy_from_slice(b"HapY");
            replacements += 1;
        }
    }
    if replacements != 1 {
        return Err(format!("expected one video sample entry, found {replacements}").into());
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, bytes)?;
    let movie_path = output.canonicalize()?;
    let project_path = write_qa_project(&movie_path)?;
    println!("movie: {}", movie_path.display());
    println!("project: {}", project_path.display());
    Ok(())
}

fn write_qa_project(movie_path: &std::path::Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut project: serde_json::Value =
        serde_json::from_slice(&fs::read("samples/phase1-mini-show.sdc")?)?;
    let video = &mut project["snapshot"]["video"];
    let state = video["layers"][0]["state"].clone();
    let layers = video["layers"]
        .as_array_mut()
        .ok_or("sample project video.layers is not an array")?;
    layers.clear();
    for (index, (label, blend_mode, opacity, x, scale, hue)) in [
        ("HAP Q Base", "Normal", 1.0, 0.0, 1.0, 0.0),
        ("HAP Q Add", "Add", 0.35, -0.18, 0.72, 45.0),
        ("HAP Q Screen", "Screen", 0.4, 0.2, 0.62, -55.0),
    ]
    .into_iter()
    .enumerate()
    {
        let mut layer_state = state.clone();
        layer_state["opacity"] = json!(opacity);
        layer_state["playing"] = json!(true);
        layer_state["loop_end_ms"] = json!(1000);
        layer_state["bpm_sync"]["enabled"] = json!(false);
        layer_state["cue_points"] = json!([
            { "position_ms": 0, "label": "Start", "color": "#5dd64c" },
            { "position_ms": 500, "label": "Flip", "color": "#ff2b88" }
        ]);
        layer_state["cue_points_ms"] = json!([0, 500]);
        layer_state["transform"]["x"] = json!(x);
        layer_state["transform"]["scale_x"] = json!(scale);
        layer_state["transform"]["scale_y"] = json!(scale);
        layer_state["color"]["hue_deg"] = json!(hue);
        layers.push(json!({
            "id": index + 2,
            "label": label,
            "source": {
                "kind": "File",
                "path": movie_path.to_string_lossy(),
                "name": null,
                "codec": "HAP Q",
                "metadata": {
                    "duration_ms": 1000,
                    "width": WIDTH,
                    "height": HEIGHT,
                    "frame_rate": 2.0
                }
            },
            "blend_mode": blend_mode,
            "state": layer_state
        }));
    }
    let cue_targets: Vec<_> = layers
        .iter()
        .map(|layer| {
            json!({
                "layer_id": layer["id"],
                "state": layer["state"]
            })
        })
        .collect();
    video["compositions"][0]["layer_ids"] = json!([2, 3, 4]);
    project["snapshot"]["cues"][0]["video_targets"] = json!(cue_targets);
    project["snapshot"]["timeline"]["video_automations"][0]["layer_id"] = json!(2);

    let project_path = movie_path.with_extension("sdc");
    fs::write(&project_path, serde_json::to_vec_pretty(&project)?)?;
    Ok(project_path.canonicalize()?)
}

fn hap_q_frame(color: u16, luminance: u8) -> Vec<u8> {
    let blocks = usize::from(WIDTH.div_ceil(4)) * usize::from(HEIGHT.div_ceil(4));
    let mut payload = Vec::with_capacity(blocks * 16);
    for _ in 0..blocks {
        payload.push(luminance);
        payload.push(luminance);
        payload.extend_from_slice(&[0; 6]);
        payload.extend_from_slice(&color.to_le_bytes());
        payload.extend_from_slice(&color.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
    }
    let len = payload.len();
    let mut frame = vec![len as u8, (len >> 8) as u8, (len >> 16) as u8, 0xaf];
    frame.extend_from_slice(&payload);
    frame
}
