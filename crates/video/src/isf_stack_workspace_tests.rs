use super::*;
use crate::prepare_isf_shader;

fn frame(layer_id: u64, width: u32, height: u32, seed: u8) -> VideoFrame {
    VideoFrame {
        layer_id,
        width,
        height,
        pts_ms: 0,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: (0..width * height)
            .flat_map(|pixel| [seed.wrapping_add(pixel as u8), seed, 173, 255])
            .collect(),
    }
}

fn gpu() -> IsfGpuRuntime {
    IsfGpuRuntime::new().expect("ISF scratch ownership tests require a GPU")
}

#[test]
fn scratch_reuse_preserves_pixels_across_layers_odd_pitch_and_resize() {
    let shader = prepare_isf_shader(
        r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
        void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage); }"#,
    )
    .unwrap();
    let mut runtime = gpu();
    let mut previous: Option<(u32, u32, wgpu::Buffer)> = None;
    for (layer, width, height, seed) in [
        (1, 65, 3, 20),
        (2, 65, 3, 71),
        (3, 257, 5, 113),
        (1, 65, 3, 157),
        (2, 65, 3, 203),
        (3, 64, 3, 31),
        (1, 64, 3, 83),
        (2, 128, 5, 137),
        (3, 65, 3, 191),
    ] {
        let input = frame(layer, width, height, seed);
        let actual = runtime.apply(&input, &shader, &[], 0.0, 0.016).unwrap();
        assert_eq!(actual, input);
        let workspace = runtime.resources.workspace.as_ref().unwrap();
        if let Some((old_width, old_height, old_buffer)) = previous {
            assert_eq!(
                old_buffer == workspace.readback,
                (width, height) == (old_width, old_height),
                "only an exact host size may reuse scratch"
            );
        }
        assert!(
            runtime
                .resources
                .entries
                .values()
                .map(|state| state.bytes)
                .sum::<u64>()
                + StackWorkspace::required_bytes(width, height)
                <= MAX_CACHED_RESOURCE_BYTES
        );
        previous = Some((width, height, workspace.readback.clone()));
    }
}

#[test]
#[ignore = "explicit matched mapped-GPU readback copy experiment; no wall-time assertion"]
fn mapped_readback_copy_timing_experiment() {
    let shader = prepare_isf_shader(
        r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
        void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage); }"#,
    )
    .unwrap();
    let input = frame(1, 2560, 1600, 37);
    let mut runtime = gpu();
    assert_eq!(
        runtime.apply(&input, &shader, &[], 0.0, 0.016).unwrap(),
        input
    );
    let readback = &runtime.resources.workspace.as_ref().unwrap().readback;
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
    runtime.device.poll(wgpu::PollType::wait()).unwrap();
    receiver.recv().unwrap().unwrap();
    let mapped = readback.slice(..).get_mapped_range();
    let row_bytes = (input.width * 4) as usize;
    let padded_row_bytes = StackWorkspace::bytes_per_row(input.width) as usize;
    assert_eq!(row_bytes, padded_row_bytes);
    let mut before = Vec::new();
    let mut after = Vec::new();
    for iteration in 0..20 {
        // Balance first/last position within each round; allocate and copy the
        // same mapped bytes in both paths, keeping verification/drop outside.
        for optimized in [false, true, true, false] {
            let started = std::time::Instant::now();
            let output = if optimized {
                copy_rgba_readback(&mapped, row_bytes, padded_row_bytes, input.data.len())
            } else {
                let mut data = Vec::with_capacity(input.data.len());
                for row in mapped.chunks_exact(padded_row_bytes) {
                    data.extend_from_slice(&row[..row_bytes]);
                }
                data
            };
            let elapsed = started.elapsed().as_nanos();
            assert_eq!(output, input.data);
            std::hint::black_box(&output);
            if iteration >= 4 {
                if optimized {
                    after.push(elapsed);
                } else {
                    before.push(elapsed);
                }
            }
        }
    }
    before.sort_unstable();
    after.sort_unstable();
    eprintln!(
        "ISF_READBACK_COPY size=2560x1600 samples={} before_p50_ns={} before_p95_ns={} after_p50_ns={} after_p95_ns={}",
        before.len(), before[16], before[30], after[16], after[30]
    );
    drop(mapped);
    readback.unmap();
}

#[test]
fn failed_readback_discards_scratch_and_does_not_publish_feedback() {
    let shader = prepare_isf_shader(include_str!("isf_fixtures/persistent.fs")).unwrap();
    let input = frame(23, 3, 2, 0);
    let mut runtime = gpu();
    let first = runtime.apply(&input, &shader, &[], 0.0, 0.016).unwrap();
    assert!(first
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [1, 0, 0, 255]));
    assert_eq!(runtime.resources.entries.len(), 1);
    // A real invalid GPU resource exercises both map failure and validation
    // scope cleanup without exhausting the adapter or injecting fake success.
    runtime
        .resources
        .workspace
        .as_ref()
        .unwrap()
        .readback
        .destroy();
    assert!(runtime.apply(&input, &shader, &[], 0.0, 0.016).is_err());
    assert!(runtime.resources.workspace.is_none());
    assert!(runtime.resources.entries.is_empty());
    let recovered = runtime.apply(&input, &shader, &[], 0.0, 0.016).unwrap();
    assert_eq!(
        recovered.data, first.data,
        "failed feedback must not become history"
    );
    assert!(runtime.resources.workspace.is_some());
}

#[test]
fn rejected_host_inputs_drop_scratch_and_allow_next_valid_frame() {
    let shader = prepare_isf_shader(
        r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
        void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage); }"#,
    )
    .unwrap();
    let mut runtime = gpu();
    let input = frame(1, 1, 1, 37);
    for oversized in [false, true] {
        runtime.apply(&input, &shader, &[], 0.0, 0.016).unwrap();
        let mut invalid = input.clone();
        if oversized {
            invalid.width = 8192;
            invalid.height = 8192;
        } else {
            invalid.data.clear();
        }
        let error = runtime
            .apply(&invalid, &shader, &[], 0.0, 0.016)
            .unwrap_err();
        assert!(
            error.message.contains(if oversized {
                "memory budget"
            } else {
                "byte length"
            }),
            "{error}"
        );
        assert!(runtime.resources.workspace.is_none());
        assert_eq!(
            runtime.apply(&input, &shader, &[], 0.0, 0.016).unwrap(),
            input
        );
    }
}
