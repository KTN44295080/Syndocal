//! Test-only host boundaries around the canonical synchronous GPU stack.
//! Clock reads, state and experiment code are absent from product builds.

use std::time::Instant;

use super::*;

const PHASES: [&str; 9] = [
    "push_error_scopes",
    "validate_convert_workspace",
    "upload",
    "encode",
    "finish_submit",
    "map_poll_wait_receive",
    "mapped_copy_unmap",
    "feedback_commit",
    "pop_error_scopes",
];

pub(super) struct StackPhaseTiming {
    pub(super) sequence: u64,
    started: Instant,
    previous: Instant,
    ns: [Option<u64>; 9],
}

impl StackPhaseTiming {
    pub(super) fn new(sequence: u64) -> Self {
        let started = Instant::now();
        Self {
            sequence,
            started,
            previous: started,
            ns: [None; 9],
        }
    }

    pub(super) fn mark(&mut self, phase: usize) {
        let now = Instant::now();
        assert!(self.ns[phase].is_none(), "phase recorded twice");
        self.ns[phase] = Some(
            now.duration_since(self.previous)
                .as_nanos()
                .try_into()
                .unwrap(),
        );
        self.previous = now;
    }
}

#[derive(serde::Serialize)]
struct Sample {
    round: usize,
    layer: u64,
    warmup: bool,
    sequence: u64,
    call_wall_ns: u64,
    phases_ns: [u64; 9],
}

#[test]
#[ignore = "explicit synchronous GPU stack phase experiment; no native FPS or wall-time assertion"]
fn synchronous_stack_phase_timing_experiment() {
    // Same targetless, control-free color operation as the native 3l/3fx fixture.
    let source = r#"/*{"ISFVSN":"2","INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main(){ vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(c.rgb * 0.99, c.a); }"#;
    let shader = crate::prepare_isf_shader(source).unwrap();
    let stages = [IsfStageInputs {
        shader: &shader,
        controls: &[],
        images: &[],
    }; 3];
    let inputs = (1..=3)
        .map(|layer| {
            let seed = layer as u8 * 37;
            VideoFrame {
                layer_id: layer,
                width: 2560,
                height: 1600,
                pts_ms: 0,
                duration_ms: 16,
                format: VideoPixelFormat::Rgba8,
                data: (0..2560 * 1600)
                    .flat_map(|pixel| [seed.wrapping_add(pixel as u8), seed, 173, 255])
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    let mut runtime = IsfGpuRuntime::new().expect("phase experiment requires a GPU");
    let mut expected = [None, None, None];
    let mut samples = Vec::new();
    let mut last_sequence = 0;
    for round in 0..24 {
        for (index, input) in inputs.iter().enumerate() {
            let keys = (0..3)
                .map(|stage| IsfInstanceKey::legacy(input.layer_id, stage))
                .collect::<Vec<_>>();
            let started = Instant::now();
            let (output, failures) = runtime.apply_stack_stage_local_with_inputs_and_keys(
                input,
                &stages,
                1.0,
                0.016,
                [2026.0, 10.0, 25.0, 0.0],
                &keys,
            );
            let call_wall_ns: u64 = started.elapsed().as_nanos().try_into().unwrap();
            assert!(failures.is_empty(), "{failures:?}");
            let timing = runtime.resources.phase_timing.as_ref().unwrap();
            assert_eq!(
                timing.sequence,
                last_sequence + 1,
                "one batched stack per layer"
            );
            last_sequence = timing.sequence;
            let phases_ns = timing.ns.map(Option::unwrap);
            let measured_ns: u64 = phases_ns.iter().sum();
            assert_eq!(
                measured_ns as u128,
                timing.previous.duration_since(timing.started).as_nanos()
            );
            assert!(measured_ns <= call_wall_ns);
            assert_eq!(
                (output.width, output.height, output.layer_id),
                (2560, 1600, input.layer_id)
            );
            assert_eq!(output.format, VideoPixelFormat::Rgba8);
            assert_eq!(output.data.len(), input.data.len());
            if let Some(expected) = &expected[index] {
                assert_eq!(&output.data, expected);
            } else {
                assert!(output.data.chunks_exact(4).all(|pixel| pixel[3] == 255));
                expected[index] = Some(output.data.clone());
            }
            samples.push(Sample {
                round,
                layer: input.layer_id,
                warmup: round < 4,
                sequence: timing.sequence,
                call_wall_ns,
                phases_ns,
            });
        }
    }
    assert_eq!(samples.len(), 72);
    assert_eq!(
        runtime.resources.entries.len(),
        0,
        "targetless fixture has no feedback cache"
    );
    // Emit only after all measurements, so logging cannot stall sampled calls.
    eprintln!(
        "ISF_PHASE_METADATA {}",
        serde_json::json!({
            "adapter": format!("{:?}", runtime.adapter_info), "width": 2560, "height": 1600,
            "layers": 3, "stages_per_layer": 3, "rounds": 24, "warmup_rounds": 4,
            "phase_names": PHASES, "shader_source": source,
            "boundary": "host synchronous stack calls; synthetic pixels; no decode/composition/present",
        })
    );
    for sample in &samples {
        eprintln!(
            "ISF_PHASE_SAMPLE {}",
            serde_json::to_string(sample).unwrap()
        );
    }
    for (phase, name) in PHASES.iter().enumerate() {
        let mut sums = (4..24)
            .map(|round| {
                samples
                    .iter()
                    .filter(|sample| sample.round == round)
                    .map(|sample| sample.phases_ns[phase])
                    .sum::<u64>()
            })
            .collect::<Vec<_>>();
        sums.sort_unstable();
        eprintln!(
            "ISF_PHASE_SUM phase={name} parents={} p50_ns={} p95_ns={}",
            sums.len(),
            sums[10],
            sums[18]
        );
    }
}

// Initialization mirrors IsfGpuRuntime::new, varying only the backend for this
// opt-in experiment. Product adapter selection remains unchanged.
fn dx12_runtime() -> IsfGpuRuntime {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    }))
    .expect("DX12 comparison requires a supported adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("Syndocal ISF runtime"),
        required_features: adapter.features() & wgpu::Features::FLOAT32_FILTERABLE,
        ..Default::default()
    }))
    .expect("DX12 comparison requires a supported device");
    let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Syndocal ISF full-screen vertex shader"),
        source: wgpu::ShaderSource::Wgsl(super::super::ISF_VERTEX_SHADER.into()),
    });
    IsfGpuRuntime {
        device,
        queue,
        adapter_info: adapter.get_info(),
        vertex_shader,
        pipelines: Default::default(),
        resources: Default::default(),
        audio_frame: None,
        frame_index: 0,
        last_stack_stage_count: 0,
        last_stack_render_us: 0,
    }
}

#[derive(serde::Serialize)]
struct BackendSample {
    backend: &'static str,
    sweep: usize,
    #[serde(flatten)]
    sample: Sample,
}

#[test]
#[ignore = "explicit matched same-adapter GPU backend experiment; no product backend or native FPS assertion"]
fn synchronous_stack_backend_timing_experiment() {
    let source = r#"/*{"ISFVSN":"2","INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main(){ vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(c.rgb * 0.99, c.a); }"#;
    let shader = crate::prepare_isf_shader(source).unwrap();
    let stages = [IsfStageInputs {
        shader: &shader,
        controls: &[],
        images: &[],
    }; 3];
    let inputs = (1..=3)
        .map(|layer| {
            let seed = layer as u8 * 37;
            VideoFrame {
                layer_id: layer,
                width: 2560,
                height: 1600,
                pts_ms: 0,
                duration_ms: 16,
                format: VideoPixelFormat::Rgba8,
                data: (0..2560 * 1600)
                    .flat_map(|pixel| [seed.wrapping_add(pixel as u8), seed, 173, 255])
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    let mut runtimes = [
        IsfGpuRuntime::new().expect("default GPU required"),
        dx12_runtime(),
    ];
    assert_eq!(
        runtimes[0].adapter_info.vendor,
        runtimes[1].adapter_info.vendor
    );
    assert_eq!(
        runtimes[0].adapter_info.device,
        runtimes[1].adapter_info.device
    );
    assert_eq!(runtimes[1].adapter_info.backend, wgpu::Backend::Dx12);
    let labels = ["default", "dx12"];
    let mut expected = [None, None, None];
    let mut sequences = [0, 0];
    let mut samples = Vec::new();
    for round in 0..12 {
        let order = if round % 2 == 0 {
            [0, 1, 1, 0]
        } else {
            [1, 0, 0, 1]
        };
        for (position, backend) in order.into_iter().enumerate() {
            let runtime = &mut runtimes[backend];
            for (index, input) in inputs.iter().enumerate() {
                let keys = (0..3)
                    .map(|stage| IsfInstanceKey::legacy(input.layer_id, stage))
                    .collect::<Vec<_>>();
                let started = Instant::now();
                let (output, failures) = runtime.apply_stack_stage_local_with_inputs_and_keys(
                    input,
                    &stages,
                    1.0,
                    0.016,
                    [2026.0, 10.0, 25.0, 0.0],
                    &keys,
                );
                let call_wall_ns = started.elapsed().as_nanos().try_into().unwrap();
                assert!(failures.is_empty(), "{failures:?}");
                let timing = runtime.resources.phase_timing.as_ref().unwrap();
                assert_eq!(timing.sequence, sequences[backend] + 1);
                sequences[backend] = timing.sequence;
                let phases_ns = timing.ns.map(Option::unwrap);
                assert!(phases_ns.iter().sum::<u64>() <= call_wall_ns);
                if let Some(expected) = &expected[index] {
                    assert_eq!(&output.data, expected);
                } else {
                    expected[index] = Some(output.data.clone());
                }
                samples.push(BackendSample {
                    backend: labels[backend],
                    sweep: round * 4 + position,
                    sample: Sample {
                        round,
                        layer: input.layer_id,
                        warmup: round < 2,
                        sequence: timing.sequence,
                        call_wall_ns,
                        phases_ns,
                    },
                });
            }
        }
    }
    assert_eq!(samples.len(), 144);
    eprintln!(
        "ISF_BACKEND_METADATA {}",
        serde_json::json!({
            "adapters": runtimes.iter().map(|runtime| format!("{:?}",runtime.adapter_info)).collect::<Vec<_>>(),
            "width": 2560, "height": 1600, "layers": 3, "stages_per_layer": 3,
            "rounds": 12, "warmup_rounds": 2, "order": "ABBA/BAAB alternation",
            "phase_names": PHASES, "shader_source": source,
            "boundary": "same-adapter synthetic synchronous stacks; no decode/composition/present",
        })
    );
    for sample in &samples {
        eprintln!(
            "ISF_BACKEND_SAMPLE {}",
            serde_json::to_string(sample).unwrap()
        );
    }
    for backend in labels {
        for (phase, name) in PHASES.iter().enumerate() {
            let mut sums = (8..48)
                .filter_map(|sweep| {
                    let layers = samples
                        .iter()
                        .filter(|row| row.backend == backend && row.sweep == sweep)
                        .collect::<Vec<_>>();
                    if layers.is_empty() {
                        return None;
                    }
                    assert_eq!(layers.len(), 3);
                    Some(
                        layers
                            .iter()
                            .map(|row| row.sample.phases_ns[phase])
                            .sum::<u64>(),
                    )
                })
                .collect::<Vec<_>>();
            sums.sort_unstable();
            assert_eq!(sums.len(), 20);
            eprintln!(
                "ISF_BACKEND_SUM backend={backend} phase={name} parents={} p50_ns={} p95_ns={}",
                sums.len(),
                sums[10],
                sums[18]
            );
        }
    }
}
