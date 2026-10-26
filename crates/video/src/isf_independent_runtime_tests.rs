use super::*;
use crate::prepare_isf_shader;
use std::time::Instant;

const DATE: [f32; 4] = [2026.0, 10.0, 25.0, 64.0];
const INCREMENT: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() {
    vec4 pixel = IMG_THIS_PIXEL(inputImage);
    gl_FragColor = vec4(pixel.r + 1.0 / 255.0, pixel.gba);
}"#;
const CLOCK_RED: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() {
    vec4 pixel = IMG_THIS_PIXEL(inputImage);
    pixel.r = float(FRAMEINDEX) / 255.0;
    gl_FragColor = pixel;
}"#;
const CLOCK_GREEN: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() {
    vec4 pixel = IMG_THIS_PIXEL(inputImage);
    pixel.g = float(FRAMEINDEX) / 255.0;
    gl_FragColor = pixel;
}"#;
const TIME_DATE: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main() {
    vec4 pixel = IMG_THIS_PIXEL(inputImage);
    gl_FragColor = vec4(pixel.rg, (TIME + TIMEDELTA) / 255.0, DATE.w / 255.0);
}"#;

fn gpu() -> IsfGpuRuntime {
    IsfGpuRuntime::new().expect("independent-chain regression requires a GPU adapter")
}

fn input(layer_id: u64, width: u32, height: u32, seed: u8) -> Arc<VideoFrame> {
    Arc::new(VideoFrame {
        layer_id,
        width,
        height,
        pts_ms: layer_id * 1_000,
        duration_ms: 16 + layer_id,
        format: VideoPixelFormat::Rgba8,
        data: (0..width * height)
            .flat_map(|pixel| {
                [
                    seed.wrapping_add(pixel as u8),
                    seed,
                    173,
                    if pixel % 5 == 0 { 127 } else { 255 },
                ]
            })
            .collect(),
    })
}

fn stage(shader: &PreparedIsfShader) -> IsfStageInputs<'_> {
    IsfStageInputs {
        shader,
        controls: &[],
        images: &[],
    }
}

fn instance_keys(inputs: &[Arc<VideoFrame>], counts: &[usize]) -> Vec<Vec<IsfInstanceKey>> {
    assert_eq!(inputs.len(), counts.len());
    inputs
        .iter()
        .zip(counts)
        .map(|(input, count)| {
            (0..*count)
                .map(|index| IsfInstanceKey::legacy(input.layer_id, index))
                .collect()
        })
        .collect()
}

fn chains<'a>(
    inputs: &[Arc<VideoFrame>],
    stages: &'a [Vec<IsfStageInputs<'a>>],
    keys: &'a [Vec<IsfInstanceKey>],
) -> Vec<IndependentChain<'a>> {
    assert_eq!(inputs.len(), stages.len());
    assert_eq!(inputs.len(), keys.len());
    inputs
        .iter()
        .zip(stages)
        .zip(keys)
        .map(|((input, stages), keys)| IndependentChain {
            frame: Arc::clone(input),
            stages,
            keys,
            time_seconds: input.pts_ms as f32 / 1_000.0,
            time_delta_seconds: input.duration_ms as f32 / 1_000.0,
            date: DATE,
        })
        .collect()
}

fn sequential(
    runtime: &mut IsfGpuRuntime,
    jobs: &[IndependentChain<'_>],
) -> Vec<(VideoFrame, Vec<(usize, IsfRuntimeError)>)> {
    jobs.iter()
        .map(|job| {
            runtime.apply_stack_stage_local_with_inputs_and_keys(
                &job.frame,
                job.stages,
                job.time_seconds,
                job.time_delta_seconds,
                job.date,
                job.keys,
            )
        })
        .collect()
}

fn originals(inputs: &[Arc<VideoFrame>]) -> Vec<VideoFrame> {
    inputs.iter().map(|input| input.as_ref().clone()).collect()
}

fn assert_inputs(inputs: &[Arc<VideoFrame>], original: &[VideoFrame], owners: usize) {
    for (input, original) in inputs.iter().zip(original) {
        assert_eq!(
            input.as_ref(),
            original,
            "shared decoder pixels/metadata changed"
        );
        assert_eq!(
            Arc::strong_count(input),
            owners,
            "completed pending work retained a decoder input"
        );
    }
}

#[test]
fn isf_independent_g2_wrapped_clock_does_not_revive_a_previous_epoch_reservation() {
    let mut runtime = gpu();
    let original = runtime.reserve_independent_clocks(3);
    assert_eq!(original.at(0), 0);
    assert!(runtime.native_clocks_current(&original));
    // Bound the full u32 cycle to two canonical advances, rather than render
    // billions of frames, while keeping the old reservation alive throughout.
    runtime.advance_frame_index(u32::MAX);
    let crossing = runtime.reserve_independent_clocks(3);
    assert_eq!(
        [crossing.at(0), crossing.at(1), crossing.at(2)],
        [u32::MAX, 0, 1]
    );
    runtime.advance_frame_index(1);
    assert_eq!(runtime.frame_index, 0);
    assert!(!runtime.native_clocks_current(&original));
    assert!(!runtime.native_clocks_current(&crossing));
    let current = runtime.reserve_independent_clocks(3);
    assert_eq!(current.at(0), original.at(0));
    assert!(runtime.native_clocks_current(&current));
    runtime.commit_independent_clocks(current);
    assert_eq!(runtime.frame_index, 3);
    assert!(!runtime.native_clocks_current(&original));
}

#[test]
fn isf_independent_batch_matches_sequential_pixels_metadata_and_shared_ownership() {
    let shader = prepare_isf_shader(INCREMENT).unwrap();
    let stages = vec![vec![stage(&shader); 3]; 3];
    let mut batched = gpu();
    let mut reference = gpu();
    // Different colors/layers expose scratch aliasing; odd pitch and resizing
    // also exercise the same host result contract as ordinary stack calls.
    for (width, height, count) in [(65, 3, 3), (65, 3, 2), (128, 5, 3), (3, 2, 3)] {
        let inputs = vec![
            input(7, width, height, 11),
            input(8, width, height, 53),
            input(9, width, height, 97),
        ];
        let original = originals(&inputs);
        let keys = instance_keys(&inputs, &[3, 3, 3]);
        let jobs = chains(&inputs, &stages, &keys);
        let expected = sequential(&mut reference, &jobs[..count]);
        let actual = batched
            .apply_independent_chains(&jobs[..count])
            .expect("two or three independent resource-neutral chains must be admitted");
        assert_eq!(actual, expected);
        // The successful shared interval describes every stage in this batch.
        assert_eq!(batched.last_stack_stage_count, count * 3);
        assert!(batched.last_stack_render_us > 0);
        assert!(actual.iter().all(|(_, failures)| failures.is_empty()));
        assert_eq!(batched.frame_index, reference.frame_index);
        for ((output, _), source) in actual.iter().zip(&inputs) {
            assert_eq!(output.data[0], source.data[0] + 3);
            assert_eq!(output.layer_id, source.layer_id);
            assert_eq!(output.pts_ms, source.pts_ms);
            assert_eq!(output.duration_ms, source.duration_ms);
        }
        assert_inputs(&inputs, &original, 2);
        drop(jobs);
        assert_inputs(&inputs, &original, 1);
    }
}

#[test]
fn isf_independent_batch_keeps_per_chain_frame_index_time_delta_and_date() {
    let red = prepare_isf_shader(CLOCK_RED).unwrap();
    let green = prepare_isf_shader(CLOCK_GREEN).unwrap();
    let time_date = prepare_isf_shader(TIME_DATE).unwrap();
    let stages = vec![vec![stage(&red), stage(&green), stage(&time_date)]; 3];
    let inputs = vec![input(1, 3, 2, 31), input(2, 3, 2, 61), input(3, 3, 2, 91)];
    let keys = instance_keys(&inputs, &[3, 3, 3]);
    let mut jobs = chains(&inputs, &stages, &keys);
    for (index, job) in jobs.iter_mut().enumerate() {
        job.time_seconds = 10.0 + index as f32;
        job.time_delta_seconds = 1.0 + index as f32;
        job.date[3] = 40.0 + index as f32;
    }
    let mut batched = gpu();
    let mut reference = gpu();
    batched.frame_index = 7;
    reference.frame_index = 7;
    for round in 0..3 {
        let expected = sequential(&mut reference, &jobs);
        let actual = batched.apply_independent_chains(&jobs).unwrap();
        assert_eq!(actual, expected);
        for (index, (output, failures)) in actual.iter().enumerate() {
            assert!(failures.is_empty(), "{failures:?}");
            let clock = (7 + round * 3 + index) as u8;
            let pixel = [clock, clock, 11 + index as u8 * 2, 40 + index as u8];
            assert!(output.data.chunks_exact(4).all(|actual| actual == pixel));
        }
        assert_eq!(batched.frame_index, (10 + round * 3) as u32);
        assert_eq!(batched.frame_index, reference.frame_index);
    }
}

#[test]
fn isf_independent_batch_middle_stage_failure_keeps_neighbors_and_clock() {
    let increment = prepare_isf_shader(INCREMENT).unwrap();
    let red = prepare_isf_shader(CLOCK_RED).unwrap();
    let green = prepare_isf_shader(CLOCK_GREEN).unwrap();
    let mut invalid = increment.clone();
    invalid.wgsl = "not valid WGSL".into();
    let stages = vec![
        vec![stage(&increment); 3],
        vec![stage(&red), stage(&invalid), stage(&green)],
        vec![stage(&increment); 3],
    ];
    let inputs = vec![input(1, 3, 2, 11), input(2, 3, 2, 53), input(3, 3, 2, 97)];
    let original = originals(&inputs);
    let keys = instance_keys(&inputs, &[3, 3, 3]);
    let jobs = chains(&inputs, &stages, &keys);
    let mut batched = gpu();
    let mut reference = gpu();
    for round in 0..2 {
        let expected = sequential(&mut reference, &jobs);
        let actual = batched.apply_independent_chains(&jobs).unwrap();
        // Compare complete error values, not just their count: recovery must
        // identify exactly the same bad stage as the canonical sequential call.
        assert_eq!(actual, expected);
        assert!(actual[0].1.is_empty());
        assert!(actual[2].1.is_empty());
        assert_eq!(actual[1].1.len(), 1);
        assert_eq!(actual[1].1[0].0, 1);
        let failed_chain_clock = (round * 3 + 1) as u8;
        assert_eq!(
            &actual[1].0.data[..3],
            &[failed_chain_clock, failed_chain_clock, 173]
        );
        assert_eq!(actual[0].0.data[0], 14);
        assert_eq!(actual[2].0.data[0], 100);
        // Recovery reports the final complete chain, like ordinary sequential
        // execution, rather than the failed group's partial map wait.
        assert_eq!(batched.last_stack_stage_count, 3);
        assert!(batched.last_stack_render_us > 0);
        assert_eq!(batched.frame_index, (round * 3 + 3) as u32);
        assert_eq!(batched.frame_index, reference.frame_index);
        assert_inputs(&inputs, &original, 2);
    }
    drop(jobs);
    assert_inputs(&inputs, &original, 1);
}

#[test]
fn isf_independent_batch_all_failed_chains_keep_input_and_advance_once() {
    let red = prepare_isf_shader(CLOCK_RED).unwrap();
    let mut invalid = red.clone();
    invalid.wgsl = "not valid WGSL".into();
    let stages = vec![vec![stage(&invalid); 3]; 3];
    let inputs = vec![input(1, 2, 2, 11), input(2, 2, 2, 53), input(3, 2, 2, 97)];
    let original = originals(&inputs);
    let keys = instance_keys(&inputs, &[3, 3, 3]);
    let jobs = chains(&inputs, &stages, &keys);
    let mut batched = gpu();
    let mut reference = gpu();
    let expected = sequential(&mut reference, &jobs);
    let actual = batched.apply_independent_chains(&jobs).unwrap();
    assert_eq!(actual, expected);
    for ((output, failures), input) in actual.iter().zip(&inputs) {
        assert_eq!(output, input.as_ref());
        assert_eq!(
            failures.iter().map(|(index, _)| *index).collect::<Vec<_>>(),
            [0, 1, 2]
        );
    }
    assert_eq!(batched.frame_index, 3);
    let recovery_stages = vec![vec![stage(&red)]; 3];
    let recovery_keys = instance_keys(&inputs, &[1, 1, 1]);
    let recovery = chains(&inputs, &recovery_stages, &recovery_keys);
    let expected = sequential(&mut reference, &recovery);
    let actual = batched.apply_independent_chains(&recovery).unwrap();
    assert_eq!(actual, expected);
    for (index, (output, failures)) in actual.iter().enumerate() {
        assert!(failures.is_empty());
        assert_eq!(output.data[0], 3 + index as u8);
    }
    assert_eq!(batched.frame_index, 6);
    assert_eq!(batched.frame_index, reference.frame_index);
    drop(recovery);
    drop(jobs);
    assert_inputs(&inputs, &original, 1);
}

#[test]
fn isf_independent_batch_declines_unsupported_without_running_or_retaining_jobs() {
    let increment = prepare_isf_shader(INCREMENT).unwrap();
    let persistent = prepare_isf_shader(include_str!("isf_fixtures/persistent.fs")).unwrap();
    let inputs = vec![input(1, 3, 2, 11), input(2, 3, 2, 53)];
    let original = originals(&inputs);
    let keys = instance_keys(&inputs, &[1, 1]);
    let stages = vec![vec![stage(&increment)]; 2];
    let jobs = chains(&inputs, &stages, &keys);
    let mut actual = gpu();
    let mut reference = gpu();
    assert!(actual.apply_independent_chains(&[]).is_none());
    assert!(actual.apply_independent_chains(&jobs[..1]).is_none());
    assert_eq!(actual.frame_index, 0);
    assert_eq!(
        sequential(&mut actual, &jobs[..1]),
        sequential(&mut reference, &jobs[..1])
    );

    let duplicate_inputs = vec![Arc::clone(&inputs[0]), Arc::clone(&inputs[0])];
    let duplicate_keys = instance_keys(&duplicate_inputs, &[1, 1]);
    let duplicate_jobs = chains(&duplicate_inputs, &stages, &duplicate_keys);
    let before = actual.frame_index;
    assert!(actual.apply_independent_chains(&duplicate_jobs).is_none());
    assert_eq!(actual.frame_index, before);
    assert_eq!(
        sequential(&mut actual, &duplicate_jobs),
        sequential(&mut reference, &duplicate_jobs)
    );
    drop(duplicate_jobs);
    drop(duplicate_inputs);

    let empty_stages = vec![Vec::new(), Vec::new()];
    let empty_keys = vec![Vec::new(), Vec::new()];
    let empty_jobs = chains(&inputs, &empty_stages, &empty_keys);
    let before = actual.frame_index;
    assert!(actual.apply_independent_chains(&empty_jobs).is_none());
    assert_eq!(actual.frame_index, before);
    assert_eq!(
        sequential(&mut actual, &empty_jobs),
        sequential(&mut reference, &empty_jobs)
    );
    assert_eq!(
        actual.frame_index, before,
        "empty stage-local chains consume no clock"
    );
    drop(empty_jobs);

    let persistent_stages = vec![vec![stage(&persistent)]; 2];
    let persistent_jobs = chains(&inputs, &persistent_stages, &keys);
    let first = sequential(&mut actual, &persistent_jobs);
    assert_eq!(first, sequential(&mut reference, &persistent_jobs));
    assert!(first
        .iter()
        .all(|(frame, faults)| faults.is_empty() && frame.data[0] == 1));
    let before = actual.frame_index;
    assert!(actual.apply_independent_chains(&persistent_jobs).is_none());
    assert_eq!(actual.frame_index, before);
    let second = sequential(&mut actual, &persistent_jobs);
    assert_eq!(second, sequential(&mut reference, &persistent_jobs));
    assert!(
        second
            .iter()
            .all(|(frame, faults)| faults.is_empty() && frame.data[0] == 2),
        "declined work must not consume persistent feedback"
    );
    drop(persistent_jobs);
    assert_eq!(actual.frame_index, reference.frame_index);
    assert_inputs(&inputs, &original, 2);
    drop(jobs);
    assert_inputs(&inputs, &original, 1);
}

#[derive(serde::Serialize)]
struct BatchBenchmarkSample {
    round: usize,
    position: usize,
    path: &'static str,
    warmup: bool,
    wall_ns: u64,
    frame_index_before: u32,
    frame_index_after: u32,
}

#[test]
#[ignore = "explicit matched independent-chain GPU experiment; raw samples, no native FPS or wall-time assertion"]
fn isf_independent_batch_timing_experiment() {
    const SOURCE: &str = r#"/*{"ISFVSN":"2","INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main(){ vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(c.rgb * 0.99, c.a); }"#;
    let shader = prepare_isf_shader(SOURCE).unwrap();
    let stages = vec![vec![stage(&shader); 3]; 3];
    let inputs = (1..=3)
        .map(|layer| {
            let seed = layer as u8 * 37;
            Arc::new(VideoFrame {
                layer_id: layer,
                width: 2560,
                height: 1600,
                pts_ms: 0,
                duration_ms: 16,
                format: VideoPixelFormat::Rgba8,
                data: (0..2560 * 1600)
                    .flat_map(|pixel| [seed.wrapping_add(pixel as u8), seed, 173, 255])
                    .collect(),
            })
        })
        .collect::<Vec<_>>();
    let keys = instance_keys(&inputs, &[3, 3, 3]);
    let jobs = chains(&inputs, &stages, &keys);
    let mut runtimes = [gpu(), gpu()];
    assert_eq!(runtimes[0].adapter_info.name, runtimes[1].adapter_info.name);
    assert_eq!(
        runtimes[0].adapter_info.vendor,
        runtimes[1].adapter_info.vendor
    );
    assert_eq!(
        runtimes[0].adapter_info.device,
        runtimes[1].adapter_info.device
    );
    assert_eq!(
        runtimes[0].adapter_info.backend,
        runtimes[1].adapter_info.backend
    );
    for runtime in &mut runtimes {
        runtime.set_stack_phase_timing_enabled(false);
    }
    let labels = ["sequential", "batch"];
    let mut expected: Option<Vec<VideoFrame>> = None;
    let mut samples = Vec::with_capacity(48);
    for round in 0..12 {
        assert_eq!(runtimes[0].frame_index, runtimes[1].frame_index);
        let order = if round % 2 == 0 {
            [0, 1, 1, 0]
        } else {
            [1, 0, 0, 1]
        };
        for (position, path) in order.into_iter().enumerate() {
            let runtime = &mut runtimes[path];
            let before = runtime.frame_index;
            let started = Instant::now();
            let output = if path == 0 {
                sequential(runtime, &jobs)
            } else {
                runtime
                    .apply_independent_chains(&jobs)
                    .expect("benchmark batch must actually be admitted")
                    .into_iter()
                    .collect()
            };
            let wall_ns = started.elapsed().as_nanos().try_into().unwrap();
            // Keep all three outputs alive through the timing boundary in
            // both paths. Equality, first reference copy and drops are outside.
            assert_eq!(output.len(), 3);
            assert!(output.iter().all(|(_, faults)| faults.is_empty()));
            assert_eq!(runtime.frame_index, before.wrapping_add(3));
            if let Some(expected) = &expected {
                for ((frame, _), expected) in output.iter().zip(expected) {
                    assert_eq!(frame, expected);
                }
            } else {
                expected = Some(output.iter().map(|(frame, _)| frame.clone()).collect());
            }
            samples.push(BatchBenchmarkSample {
                round,
                position,
                path: labels[path],
                warmup: round < 2,
                wall_ns,
                frame_index_before: before,
                frame_index_after: runtime.frame_index,
            });
        }
        assert_eq!(runtimes[0].frame_index, runtimes[1].frame_index);
    }
    assert_eq!(samples.len(), 48);
    assert_eq!(runtimes[0].frame_index, 72);
    assert_eq!(runtimes[1].frame_index, 72);
    assert_eq!(runtimes[0].independent_batch_count(), 0);
    assert_eq!(runtimes[1].independent_batch_count(), 24);
    for label in labels {
        assert_eq!(
            samples.iter().filter(|sample| sample.path == label).count(),
            24
        );
        assert_eq!(
            samples
                .iter()
                .filter(|sample| sample.path == label && !sample.warmup)
                .count(),
            20
        );
    }
    // Buffer all records until the measurements finish; stderr cannot stall
    // a measured sweep.
    eprintln!(
        "ISF_INDEPENDENT_BATCH_METADATA {}",
        serde_json::json!({
            "adapters": runtimes.iter().map(|runtime| format!("{:?}", runtime.adapter_info)).collect::<Vec<_>>(),
            "adapter_identity": "matching reported name/vendor/device/backend; not a unique physical adapter guarantee",
            "width": 2560, "height": 1600, "layers": 3, "stages_per_layer": 3,
            "rounds": 12, "warmup_rounds": 2, "order": "ABBA/BAAB alternation",
            "raw_sweeps": 48, "measured_sweeps_per_path": 20,
            "phase_clocks": "OFF for both runtimes",
            "submitted_batches": runtimes.iter().map(IsfGpuRuntime::independent_batch_count).collect::<Vec<_>>(),
            "boundary": "three synthetic GPU chains; all three outputs retained through wall boundary; excludes decode/composition/present and equality checks",
            "shader_source": SOURCE,
        })
    );
    for sample in &samples {
        eprintln!(
            "ISF_INDEPENDENT_BATCH_SAMPLE {}",
            serde_json::to_string(sample).unwrap()
        );
    }
    for label in labels {
        let mut measured = samples
            .iter()
            .filter(|sample| sample.path == label && !sample.warmup)
            .map(|sample| sample.wall_ns)
            .collect::<Vec<_>>();
        measured.sort_unstable();
        assert_eq!(measured.len(), 20);
        eprintln!(
            "ISF_INDEPENDENT_BATCH_SUM path={label} samples=20 p50_ns={} p95_ns={}",
            measured[10], measured[18]
        );
    }
}
