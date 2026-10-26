use super::*;
use crate::prepare_isf_shader;

const DATE: [f32; 4] = [2026.0, 10.0, 25.0, 0.0];
const IDENTITY: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() { gl_FragColor = IMG_THIS_PIXEL(inputImage); }"#;

fn frame(layer: u64, width: u32, height: u32, seed: u8) -> Arc<VideoFrame> {
    Arc::new(VideoFrame {
        layer_id: layer,
        width,
        height,
        pts_ms: layer * 17,
        duration_ms: 17,
        format: VideoPixelFormat::Rgba8,
        data: (0..width * height)
            .flat_map(|pixel| [seed.wrapping_add(pixel as u8), seed, 173, 255])
            .collect(),
    })
}

fn requests<'a>(
    frames: &[Arc<VideoFrame>],
    stages: &'a [IsfStageInputs<'a>],
) -> Vec<IndependentStack<'a>> {
    frames
        .iter()
        .enumerate()
        .map(|(index, frame)| IndependentStack {
            frame: Arc::clone(frame),
            stages,
            keys: (0..stages.len())
                .map(|stage| IsfInstanceKey::legacy(frame.layer_id, stage))
                .collect::<Vec<_>>()
                .into(),
            frame_index: 31 + index as u32,
            time_seconds: 7.0 + index as f32,
            time_delta_seconds: 0.017,
            date: DATE,
        })
        .collect()
}

fn gpu() -> IsfGpuRuntime {
    IsfGpuRuntime::new().expect("independent ISF stack ownership checks require a GPU")
}

#[test]
fn independent_stacks_keep_distinct_pixels_clocks_and_bounded_reusable_scratch() {
    let invert = prepare_isf_shader(
        r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
        void main(){ vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(1.0-c.rgb,c.a); }"#,
    )
    .unwrap();
    let clock = prepare_isf_shader(r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
        void main(){ vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(c.r,float(FRAMEINDEX)/255.0,TIME/255.0,c.a); }"#).unwrap();
    let stages = [
        IsfStageInputs {
            shader: &invert,
            controls: &[],
            images: &[],
        },
        IsfStageInputs {
            shader: &clock,
            controls: &[],
            images: &[],
        },
    ];
    let mut runtime = gpu();
    runtime.set_stack_phase_timing_enabled(false);
    let mut previous_buffers: Vec<wgpu::Buffer> = Vec::new();
    for round in 0..4 {
        let frames = (1..=3)
            .map(|layer| {
                frame(
                    layer,
                    if layer == 3 { 128 } else { 65 },
                    3,
                    (layer * 39 + round * 13) as u8,
                )
            })
            .collect::<Vec<_>>();
        let jobs = requests(&frames, &stages);
        let output = runtime
            .try_render_independent_stacks(&jobs)
            .unwrap()
            .unwrap();
        assert_eq!(output.len(), 3);
        for ((actual, input), job) in output.iter().zip(&frames).zip(&jobs) {
            assert_eq!(
                (
                    actual.layer_id,
                    actual.width,
                    actual.height,
                    actual.pts_ms,
                    actual.duration_ms
                ),
                (
                    input.layer_id,
                    input.width,
                    input.height,
                    input.pts_ms,
                    input.duration_ms
                )
            );
            for (actual, input) in actual.data.chunks_exact(4).zip(input.data.chunks_exact(4)) {
                assert_eq!(
                    actual,
                    [
                        255 - input[0],
                        job.frame_index as u8,
                        job.time_seconds as u8,
                        255
                    ]
                );
            }
            assert_eq!(
                Arc::strong_count(input),
                2,
                "completed ticket releases its input Arc"
            );
        }
        let buffers = runtime
            .resources
            .workspace
            .iter()
            .chain(&runtime.resources.batch_workspaces)
            .map(|workspace| {
                workspace
                    .readback
                    .as_ref()
                    .expect("CPU stack retains readback")
                    .clone()
            })
            .collect::<Vec<_>>();
        assert_eq!(buffers.len(), 3);
        for left in 0..buffers.len() {
            for right in left + 1..buffers.len() {
                assert_ne!(
                    buffers[left], buffers[right],
                    "pending uploads cannot alias scratch"
                );
            }
        }
        if !previous_buffers.is_empty() {
            assert!(buffers
                .iter()
                .all(|buffer| previous_buffers.contains(buffer)));
        }
        previous_buffers = buffers;
        assert_eq!(runtime.independent_batch_count(), round + 1);
        assert!(runtime.resources.phase_timing.is_none());
        assert!(runtime.resources.entries.is_empty());
        assert_eq!(
            runtime.frame_index, 0,
            "pass tickets do not commit caller clocks"
        );
    }
    let identity = prepare_isf_shader(IDENTITY).unwrap();
    let input = frame(7, 65, 3, 91);
    assert_eq!(
        runtime.apply(&input, &identity, &[], 0.0, 0.017).unwrap(),
        *input
    );
    assert!(
        runtime.resources.batch_workspaces.is_empty(),
        "single path releases extra reservation before feedback eviction"
    );
    assert!(runtime.resources.phase_timing.is_none());
    runtime.set_stack_phase_timing_enabled(true);
    runtime.apply(&input, &identity, &[], 0.0, 0.017).unwrap();
    assert!(runtime.resources.phase_timing.is_some());
}

#[test]
fn memory_pressure_and_resourceful_stages_decline_without_touching_feedback() {
    let history = prepare_isf_shader(include_str!("isf_fixtures/persistent.fs")).unwrap();
    let identity = prepare_isf_shader(IDENTITY).unwrap();
    let stages = [IsfStageInputs {
        shader: &identity,
        controls: &[],
        images: &[],
    }];
    let history_input = frame(99, 4, 4, 0);
    let mut runtime = gpu();
    runtime
        .apply(&history_input, &history, &[], 0.0, 0.017)
        .unwrap();
    let key = IsfInstanceKey::legacy(99, 0);
    let original_bytes = runtime.resources.entries[&key].bytes;
    let frames = vec![frame(1, 65, 3, 37), frame(2, 65, 3, 91)];
    let jobs = requests(&frames, &stages);
    let one_reservation = StackWorkspace::required_bytes(65, 3) + frames[0].data.len() as u64 * 2;
    // Simulate a nearly full accounting reservation without allocating 512 MiB
    // on the test GPU. The existing real history image must remain untouched.
    runtime.resources.entries.get_mut(&key).unwrap().bytes =
        MAX_CACHED_RESOURCE_BYTES - one_reservation;
    let before = runtime
        .resources
        .workspace
        .as_ref()
        .unwrap()
        .readback
        .as_ref()
        .expect("CPU stack retains readback")
        .clone();
    let clock = runtime.resources.clock;
    let index = runtime.frame_index;
    assert!(runtime.try_render_independent_stacks(&jobs).is_none());
    assert_eq!(runtime.resources.clock, clock);
    assert_eq!(runtime.frame_index, index);
    assert_eq!(
        runtime
            .resources
            .workspace
            .as_ref()
            .unwrap()
            .readback
            .as_ref()
            .expect("CPU stack retains readback"),
        &before
    );
    assert_eq!(runtime.independent_batch_count(), 0);
    assert!(runtime.resources.entries[&key].has_history);
    runtime.resources.entries.get_mut(&key).unwrap().bytes = original_bytes;
    let history_stages = [IsfStageInputs {
        shader: &history,
        controls: &[],
        images: &[],
    }];
    assert!(runtime
        .try_render_independent_stacks(&requests(&frames, &history_stages))
        .is_none());
    assert_eq!(runtime.independent_batch_count(), 0);
    let second = runtime
        .apply(&history_input, &history, &[], 0.0, 0.017)
        .unwrap();
    assert!(
        second
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [2, 0, 0, 255]),
        "decline neither advances nor evicts feedback"
    );
}

#[test]
fn padded_upload_pressure_declines_before_upload_without_changing_feedback() {
    // Literal independent oracle: 505 MiB cache; old packed-upload omission
    // reserves 6,782,976; actual 256-padded simultaneous payload is 12,976,128.
    assert_upload_decline(Some(256), 529_530_880, 6_782_976, 12_976_128);
}

#[test]
fn pitch_512_pressure_declines_before_upload_without_changing_feedback() {
    // Three 1x8192 jobs: old pitch-256 reservation 13,074,432;
    // actual pitch-512 known payload 19,267,584. Cache is 499 MiB.
    assert_eq!(523_239_424_u64 + 13_074_432, 536_313_856);
    assert_eq!(523_239_424_u64 + 19_267_584, 542_507_008);
    // New conservative reservation also includes CPU output: 19,365,888.
    assert_eq!(
        3 * (StackWorkspace::required_bytes(1, 8192)
            + 65_536
            + StackWorkspace::upload_staging_bytes(1, 8192, 512).unwrap()),
        19_365_888
    );
    assert_upload_decline(Some(512), 523_239_424, 13_074_432, 19_267_584);
}

#[test]
fn unknown_upload_pitch_declines_before_upload_without_changing_feedback() {
    let history = prepare_isf_shader(include_str!("isf_fixtures/persistent.fs")).unwrap();
    let identity = prepare_isf_shader(IDENTITY).unwrap();
    let stages = [IsfStageInputs {
        shader: &identity,
        controls: &[],
        images: &[],
    }];
    let history_input = frame(99, 4, 4, 0);
    let mut runtime = gpu();
    runtime
        .apply(&history_input, &history, &[], 0.0, 0.017)
        .unwrap();
    let key = IsfInstanceKey::legacy(99, 0);
    let cache_bytes = runtime.resources.entries[&key].bytes;
    let frames = [
        frame(1, 65, 3, 17),
        frame(2, 65, 3, 53),
        frame(3, 65, 3, 91),
    ];
    let jobs = requests(&frames, &stages);

    // No simulated cache pressure: the exact same jobs must submit with a
    // known pitch, so substituting Some(512) for None cannot pass this test.
    runtime.resources.queue_upload_pitch = Some(512);
    let submits_before_admission = runtime.resources.stack_submit_count;
    let output = runtime
        .try_render_independent_stacks(&jobs)
        .unwrap()
        .unwrap();
    assert!(runtime.resources.stack_submit_count > submits_before_admission);
    assert!(output
        .iter()
        .zip(&frames)
        .all(|(actual, input)| actual == input.as_ref()));

    runtime.resources.queue_upload_pitch = None;
    let scratch = runtime
        .resources
        .workspace
        .iter()
        .chain(&runtime.resources.batch_workspaces)
        .map(|workspace| {
            workspace
                .readback
                .as_ref()
                .expect("CPU stack retains readback")
                .clone()
        })
        .collect::<Vec<_>>();
    let clock = runtime.resources.clock;
    let frame_index = runtime.frame_index;
    let batches = runtime.independent_batch_count();
    let uploads = runtime.resources.stack_upload_count;
    let submits = runtime.resources.stack_submit_count;
    assert!(runtime.try_render_independent_stacks(&jobs).is_none());
    assert_eq!(runtime.resources.stack_upload_count, uploads, "no upload");
    assert_eq!(
        runtime.resources.stack_submit_count, submits,
        "no submit, including cleanup"
    );
    assert_eq!(runtime.resources.entries.len(), 1);
    assert_eq!(runtime.resources.entries[&key].bytes, cache_bytes);
    assert!(runtime.resources.entries[&key].has_history);
    assert_eq!(runtime.resources.clock, clock);
    assert_eq!(runtime.frame_index, frame_index);
    assert_eq!(runtime.independent_batch_count(), batches);
    assert_eq!(
        runtime
            .resources
            .workspace
            .iter()
            .chain(&runtime.resources.batch_workspaces)
            .map(|workspace| workspace
                .readback
                .as_ref()
                .expect("CPU stack retains readback")
                .clone())
            .collect::<Vec<_>>(),
        scratch
    );
    assert!(frames.iter().all(|frame| Arc::strong_count(frame) == 2));
    let second = runtime
        .apply(&history_input, &history, &[], 0.0, 0.017)
        .unwrap();
    assert!(second
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [2, 0, 0, 255]));
}

fn assert_upload_decline(pitch: Option<u64>, cache: u64, old_three: u64, known_three: u64) {
    let history = prepare_isf_shader(include_str!("isf_fixtures/persistent.fs")).unwrap();
    let identity = prepare_isf_shader(IDENTITY).unwrap();
    let stages = [IsfStageInputs {
        shader: &identity,
        controls: &[],
        images: &[],
    }];
    let history_input = frame(99, 4, 4, 0);
    let mut runtime = gpu();
    runtime
        .apply(&history_input, &history, &[], 0.0, 0.017)
        .unwrap();
    let key = IsfInstanceKey::legacy(99, 0);
    let original_bytes = runtime.resources.entries[&key].bytes;
    assert!(cache + old_three < 536_870_912);
    assert!(cache + known_three > 536_870_912);
    runtime.resources.queue_upload_pitch = pitch;
    runtime.resources.entries.get_mut(&key).unwrap().bytes = cache;
    let frames = [
        frame(1, 1, 8192, 17),
        frame(2, 1, 8192, 53),
        frame(3, 1, 8192, 91),
    ];
    let jobs = requests(&frames, &stages);
    let buffer = runtime
        .resources
        .workspace
        .as_ref()
        .unwrap()
        .readback
        .as_ref()
        .expect("CPU stack retains readback")
        .clone();
    let clock = runtime.resources.clock;
    let frame_index = runtime.frame_index;
    let uploads = runtime.resources.stack_upload_count;
    let submits = runtime.resources.stack_submit_count;
    assert!(runtime.try_render_independent_stacks(&jobs).is_none());
    assert_eq!(runtime.resources.stack_upload_count, uploads, "no upload");
    assert_eq!(
        runtime.resources.stack_submit_count, submits,
        "no submit, including cleanup"
    );
    assert_eq!(runtime.resources.entries.len(), 1);
    assert_eq!(runtime.resources.entries[&key].bytes, cache);
    assert!(runtime.resources.entries[&key].has_history);
    assert_eq!(runtime.resources.clock, clock);
    assert_eq!(runtime.frame_index, frame_index);
    assert_eq!(runtime.independent_batch_count(), 0);
    assert_eq!(
        runtime
            .resources
            .workspace
            .as_ref()
            .unwrap()
            .readback
            .as_ref()
            .expect("CPU stack retains readback"),
        &buffer
    );
    assert!(runtime.resources.batch_workspaces.is_empty());
    assert!(frames.iter().all(|frame| Arc::strong_count(frame) == 2));
    runtime.resources.entries.get_mut(&key).unwrap().bytes = original_bytes;
    let second = runtime
        .apply(&history_input, &history, &[], 0.0, 0.017)
        .unwrap();
    assert!(second
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [2, 0, 0, 255]));
}

#[test]
fn encoding_failure_flushes_pending_uploads_before_single_recovery() {
    let identity = prepare_isf_shader(IDENTITY).unwrap();
    let mut invalid = prepare_isf_shader(IDENTITY).unwrap();
    invalid.wgsl = "this is not valid WGSL".to_string();
    let frames = vec![frame(1, 65, 3, 37), frame(2, 65, 3, 91)];
    let good = [IsfStageInputs {
        shader: &identity,
        controls: &[],
        images: &[],
    }];
    let bad = [IsfStageInputs {
        shader: &invalid,
        controls: &[],
        images: &[],
    }];
    let mut jobs = requests(&frames, &good);
    jobs[1].stages = &bad;
    let mut runtime = gpu();
    assert!(runtime
        .try_render_independent_stacks(&jobs)
        .unwrap()
        .is_err());
    assert_eq!(
        runtime.independent_batch_count(),
        0,
        "no encoded batch was submitted"
    );
    assert!(runtime.resources.poisoned_transaction.is_none());
    assert!(runtime.resources.workspace.is_none());
    assert!(runtime.resources.batch_workspaces.is_empty());
    for input in &frames {
        assert_eq!(Arc::strong_count(input), 2);
        assert_eq!(
            runtime.apply(input, &identity, &[], 0.0, 0.017).unwrap(),
            **input
        );
    }
    jobs[1].stages = &good;
    let output = runtime
        .try_render_independent_stacks(&jobs)
        .unwrap()
        .unwrap();
    assert!(output
        .iter()
        .zip(&frames)
        .all(|(actual, input)| actual == input.as_ref()));
}

#[test]
fn failed_batch_map_drains_every_ticket_and_preserves_unrelated_feedback() {
    let identity = prepare_isf_shader(IDENTITY).unwrap();
    let stages = [IsfStageInputs {
        shader: &identity,
        controls: &[],
        images: &[],
    }];
    let history = prepare_isf_shader(include_str!("isf_fixtures/persistent.fs")).unwrap();
    let history_input = frame(99, 4, 4, 0);
    let frames = vec![
        frame(1, 65, 3, 37),
        frame(2, 65, 3, 91),
        frame(3, 65, 3, 151),
    ];
    let jobs = requests(&frames, &stages);
    let mut runtime = gpu();
    runtime
        .apply(&history_input, &history, &[], 0.0, 0.017)
        .unwrap();
    runtime
        .try_render_independent_stacks(&jobs)
        .unwrap()
        .unwrap();
    runtime
        .resources
        .workspace
        .as_ref()
        .unwrap()
        .readback
        .as_ref()
        .expect("CPU stack retains readback")
        .destroy();
    assert!(runtime
        .try_render_independent_stacks(&jobs)
        .unwrap()
        .is_err());
    assert!(runtime.resources.workspace.is_none());
    assert!(runtime.resources.batch_workspaces.is_empty());
    assert!(runtime.resources.poisoned_transaction.is_none());
    assert_eq!(runtime.resources.entries.len(), 1);
    assert_eq!(runtime.independent_batch_count(), 2);
    assert!(frames.iter().all(|input| Arc::strong_count(input) == 2));
    let second = runtime
        .apply(&history_input, &history, &[], 0.0, 0.017)
        .unwrap();
    assert!(second
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [2, 0, 0, 255]));
    let recovered = runtime
        .try_render_independent_stacks(&jobs)
        .unwrap()
        .unwrap();
    assert!(recovered
        .iter()
        .zip(&frames)
        .all(|(actual, input)| actual == input.as_ref()));
}

#[test]
fn poisoned_cleanup_retains_input_and_scratch_and_blocks_all_gpu_recovery() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<IsfGpuRuntime>();
    let shader = prepare_isf_shader(IDENTITY).unwrap();
    let stages = [IsfStageInputs {
        shader: &shader,
        controls: &[],
        images: &[],
    }];
    let input = frame(1, 65, 3, 37);
    let mut runtime = gpu();
    runtime.apply(&input, &shader, &[], 0.0, 0.017).unwrap();
    let keys = vec![IsfInstanceKey::legacy(1, 0)];
    let mut pending = [PendingStack::new(
        &input,
        Some(Arc::clone(&input)),
        Cow::Borrowed(&keys),
        StackClocks {
            time_seconds: 0.0,
            time_delta_seconds: 0.017,
            frame_index: 17,
            date: DATE,
        },
        runtime.resources.workspace.take(),
    )];
    // Explicitly inject the exceptional failed-drain state: a real device loss
    // is not induced. Assert owner retention and refusal of both entry points.
    runtime.resources.poisoned_transaction = Some(PoisonedStackTransaction::new(
        &mut pending,
        "injected cleanup wait failure".to_string(),
    ));
    assert_eq!(Arc::strong_count(&input), 2);
    let clock = runtime.resources.clock;
    let error = runtime
        .render_pass_stack(&input, &stages, 0.0, 0.017, 17, DATE, &keys)
        .unwrap_err();
    assert!(error.message.contains("owner retained"));
    let frames = vec![Arc::clone(&input), frame(2, 65, 3, 91)];
    assert!(runtime
        .try_render_independent_stacks(&requests(&frames, &stages))
        .unwrap()
        .is_err());
    assert_eq!(runtime.independent_batch_count(), 0);
    assert_eq!(runtime.resources.clock, clock);
    assert!(runtime.resources.workspace.is_none());
    drop(frames);
    assert_eq!(Arc::strong_count(&input), 2, "failed owner remains pinned");
    drop(runtime);
    assert_eq!(Arc::strong_count(&input), 1);
}

#[test]
fn queried_upload_pitch_keeps_actual_two_and_three_job_batches_admitted() {
    let mut runtime = gpu();
    let pitch = runtime
        .resources
        .queue_upload_pitch
        .expect("actual headless adapter pitch must be established");
    let alignment = pitch.max(256);
    assert!(alignment >= pitch && alignment >= 256);
    assert_eq!(alignment % 256, 0);
    assert_eq!(
        StackWorkspace::upload_staging_bytes(1, 8192, pitch),
        Some(alignment * 8192)
    );
    eprintln!(
        "ISF_UPLOAD_PITCH backend={:?} adapter={} raw={} reservation_alignment={}",
        runtime.adapter_info.backend, runtime.adapter_info.name, pitch, alignment
    );
    let shader = prepare_isf_shader(IDENTITY).unwrap();
    let stages = [IsfStageInputs {
        shader: &shader,
        controls: &[],
        images: &[],
    }];
    for count in [2, 3] {
        let frames = (1..=count)
            .map(|id| frame(id, 65, 3, id as u8))
            .collect::<Vec<_>>();
        let submits = runtime.resources.stack_submit_count;
        let output = runtime
            .try_render_independent_stacks(&requests(&frames, &stages))
            .expect("known pitch must admit a normal batch")
            .unwrap();
        assert!(runtime.resources.stack_submit_count > submits);
        assert!(runtime.independent_batch_count() > 0);
        assert!(output
            .iter()
            .zip(&frames)
            .all(|(out, input)| out == input.as_ref()));
    }
}

#[test]
fn isf_upload_pitch_derivation_requires_locked_wgpu_25_0_2() {
    let lock = include_str!("../../../Cargo.lock");
    for name in ["wgpu", "wgpu-core", "wgpu-hal"] {
        let versions = lock
            .split("[[package]]")
            .filter_map(|entry| {
                let mut lines = entry.lines().filter(|line| !line.is_empty());
                (lines.next()? == format!("name = \"{name}\""))
                    .then(|| lines.next().unwrap_or_default())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            versions,
            ["version = \"25.0.2\""],
            "re-review backend pitch derivation and queue staging on a wgpu dependency change"
        );
    }
}
