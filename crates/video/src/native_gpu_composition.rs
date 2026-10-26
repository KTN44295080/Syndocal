//! P lane's texture-source composition plan (INTERFACE-01 section 6.1).
//!
//! This policy owns no device, submission, scope or surface. A broker adapter
//! resolves selected identities and encodes through the port below. Layer
//! opacity already includes master opacity in the canonical composition plan;
//! applying master opacity here again would change its pixels.
//!
//! `NativeGpuCompositor` is the shared-context adapter: it encodes the plan in
//! its own `CompositionSnapshot` phase of G's frame transaction, reading only
//! G's `GpuAllocationReadUse::view()` leases, and submits exactly one command
//! buffer (clear, every layer, one premapping snapshot copy) after the ISF
//! submission. It never waits, maps, acquires a surface or publishes.

use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::native_gpu_budget::{
    AllocationCharge, ChargeKind, FrameReservation, NativeAllocationPlan,
};
use crate::native_gpu_context::{
    EncodedGpuPhase, GpuPhaseEncoder, NativeGpuFrameTxn, NativeGpuOwner, PendingPhaseReceipt,
};
use crate::native_gpu_contract::{
    GpuAllocationId, GpuContextId, GpuPhase, GpuSubmissionId, HistoryEntryId, LayerResultIdentity,
    NativeFrameId, NativeGpuDecline, NativeGpuFault, NativeGpuFaultKind, RetirementId,
};
use crate::native_gpu_result::{EncodedLayerCandidate, GpuAllocationReadUse, GpuLayerLease};
use crate::{
    axis_aligned_source_lookup, gpu_composition_layer_plan, CompositionLayerPlan, VideoPixelFormat,
};

/// Packed `Params` uniform of the texture composition shader.
const LAYER_PARAMS_BYTES: u64 = 32;

pub(crate) const TEXTURE_COMPOSITION_SHADER: &str =
    include_str!("gpu_display_composition_texture.wgsl");

/// Planning metadata, never evidence of GPU completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectedTextureKind {
    Validated,
    InThisTransaction,
}

pub(crate) struct NativeCompositionLayer<'a> {
    pub plan: &'a CompositionLayerPlan,
    pub selected_entry: HistoryEntryId,
    pub identity: LayerResultIdentity,
    pub kind: SelectedTextureKind,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CompositionPlanFailure {
    Declined(NativeGpuDecline),
    Fault(NativeGpuFault),
}

#[derive(Debug)]
pub(crate) struct CompositionLayerEncoding {
    pub selected_entry: HistoryEntryId,
    pub identity: LayerResultIdentity,
    pub kind: SelectedTextureKind,
    /// Canonical CPU sampling indices; no full-frame pixel upload.
    pub lookup: Vec<u32>,
    pub params: [u8; 32],
}

#[derive(Debug)]
pub(crate) struct NativeCompositionPlan {
    pub context: GpuContextId,
    pub frame: NativeFrameId,
    pub extent: (u32, u32),
    pub clear_color: [u8; 4],
    pub layers_bottom_first: Vec<CompositionLayerEncoding>,
    pub allocations: CompositionAllocationPlan,
    native_allocations: NativeAllocationPlan,
}

/// Checked minimum capacities. G must also account for actual grown capacities,
/// old/new resources and outstanding uses in its one shared allocation ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CompositionAllocationPlan {
    pub premapping_bytes: u64,
    pub readback_bytes: u64,
    pub lookup_bytes: u64,
    pub uniform_bytes: u64,
}

impl NativeCompositionPlan {
    pub(crate) fn preflight(
        context: GpuContextId,
        frame: NativeFrameId,
        extent: (u32, u32),
        clear_color: [u8; 4],
        layers_bottom_first: &[NativeCompositionLayer<'_>],
        max_texture_dimension_2d: u32,
        max_storage_buffer_bytes: u64,
    ) -> Result<Self, CompositionPlanFailure> {
        let decline = |reason| CompositionPlanFailure::Declined(reason);
        let mismatch = || {
            CompositionPlanFailure::Fault(NativeGpuFault {
                context,
                frame: Some(frame),
                phase: GpuPhase::CompositionSnapshot,
                kind: NativeGpuFaultKind::IdentityMismatch,
                message: "composition selected entry/context/frame does not match its texture"
                    .into(),
            })
        };
        let (width, height) = extent;
        let output_bytes = u64::from(width)
            .checked_mul(u64::from(height))
            // WGSL's packed-buffer indices are u32, independent of host size.
            .filter(|pixels| *pixels <= u64::from(u32::MAX))
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes <= max_storage_buffer_bytes)
            .ok_or_else(|| decline(NativeGpuDecline::UnsupportedExtent))?;
        if width == 0
            || height == 0
            || width > max_texture_dimension_2d
            || height > max_texture_dimension_2d
        {
            return Err(decline(NativeGpuDecline::UnsupportedExtent));
        }
        if layers_bottom_first.is_empty() {
            return Err(decline(NativeGpuDecline::UnsupportedComposition));
        }
        let mut layers = Vec::with_capacity(layers_bottom_first.len());
        let mut lookup_bytes = 0u64;
        let mut uniform_bytes = 0u64;
        for layer in layers_bottom_first {
            let identity = layer.identity;
            if identity.context != context
                || identity.entry != layer.selected_entry
                || identity.layer_id != layer.plan.layer_id
                || identity.frame.output_id != frame.output_id
                || identity.frame.project_epoch != frame.project_epoch
                || identity.frame.geometry_generation != frame.geometry_generation
                || identity.entry.input.project_render_epoch != frame.project_epoch
                || (layer.kind == SelectedTextureKind::InThisTransaction && identity.frame != frame)
            {
                return Err(mismatch());
            }
            if identity.format != VideoPixelFormat::Rgba8 {
                return Err(decline(NativeGpuDecline::UnsupportedFormat));
            }
            if (identity.width, identity.height) != extent {
                return Err(decline(NativeGpuDecline::UnsupportedExtent));
            }
            if !gpu_composition_layer_plan(layer.plan) {
                return Err(decline(NativeGpuDecline::UnsupportedComposition));
            }
            let lookup = axis_aligned_source_lookup(&layer.plan.transform, extent, extent);
            let bytes = u64::try_from(lookup.len())
                .ok()
                .and_then(|len| len.checked_mul(4))
                .filter(|bytes| *bytes <= max_storage_buffer_bytes)
                .ok_or_else(|| decline(NativeGpuDecline::UnsupportedExtent))?;
            lookup_bytes = lookup_bytes
                .checked_add(bytes)
                .ok_or_else(|| decline(NativeGpuDecline::UnsupportedExtent))?;
            uniform_bytes = uniform_bytes
                .checked_add(LAYER_PARAMS_BYTES)
                .ok_or_else(|| decline(NativeGpuDecline::UnsupportedExtent))?;
            let mut params = [0u8; 32];
            for (index, word) in [
                width,
                height,
                identity.width,
                identity.height,
                layer.plan.opacity.clamp(0.0, 1.0).to_bits(),
                crate::gpu_compositor::blend_mode_index(&layer.plan.blend_mode),
                u32::from_le_bytes(clear_color),
                0,
            ]
            .into_iter()
            .enumerate()
            {
                params[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
            }
            layers.push(CompositionLayerEncoding {
                selected_entry: layer.selected_entry,
                identity,
                kind: layer.kind,
                lookup,
                params,
            });
        }
        // One exact charge per physical allocation, in the order the shared
        // adapter binds them: premapping target, its single readback, the owned
        // packed CPU snapshot, then each layer's lookup and parameter buffer.
        let mut native_allocations = NativeAllocationPlan::new(frame);
        let per_layer = layers.iter().flat_map(|layer| {
            [
                (ChargeKind::CompositionBuffer, layer.lookup.len() as u64 * 4),
                (ChargeKind::CompositionBuffer, LAYER_PARAMS_BYTES),
            ]
        });
        for (kind, bytes) in [
            (ChargeKind::CompositionBuffer, output_bytes),
            (ChargeKind::SnapshotReadback, output_bytes),
            (ChargeKind::CpuSnapshot, output_bytes),
        ]
        .into_iter()
        .chain(per_layer)
        {
            native_allocations.add_new(kind, bytes).map_err(decline)?;
        }
        Ok(Self {
            context,
            frame,
            extent,
            clear_color,
            layers_bottom_first: layers,
            allocations: CompositionAllocationPlan {
                premapping_bytes: output_bytes,
                readback_bytes: output_bytes,
                lookup_bytes,
                uniform_bytes,
            },
            native_allocations,
        })
    }

    /// Merge this exact private sub-plan into the G/P frame quote before the
    /// frame is reserved. Validated source allocations are aliases which the
    /// caller adds to the complete frame plan; candidates are allocated by G.
    pub(crate) fn allocation_plan(&self) -> &NativeAllocationPlan {
        &self.native_allocations
    }

    /// The adapter reserves the whole plan, resolves identities and retains
    /// selected read uses before encoding. No wait/map/cache/acquire here.
    pub(crate) fn encode<P: CompositionEncodingPort>(
        &self,
        port: &mut P,
    ) -> Result<P::Encoded, P::Error> {
        port.begin(self)?;
        for layer in &self.layers_bottom_first {
            port.composite_layer(layer)?;
        }
        port.copy_premapping_to_snapshot(self.allocations.premapping_bytes)?;
        port.finish()
    }
}

pub(crate) trait CompositionEncodingPort {
    type Encoded;
    type Error;
    fn begin(&mut self, plan: &NativeCompositionPlan) -> Result<(), Self::Error>;
    fn composite_layer(&mut self, layer: &CompositionLayerEncoding) -> Result<(), Self::Error>;
    fn copy_premapping_to_snapshot(&mut self, bytes: u64) -> Result<(), Self::Error>;
    fn finish(&mut self) -> Result<Self::Encoded, Self::Error>;
}

/// INTERFACE-01 §6.1 selected source: validated history or a candidate that is
/// private to this transaction. A caller texture/view cannot bypass G's device
/// identity check or its read-use lifetime pin.
pub(crate) enum SelectedLayerInput<'a> {
    Validated(&'a GpuLayerLease),
    InThisTransaction(&'a EncodedLayerCandidate),
}

impl SelectedLayerInput<'_> {
    fn identity(&self) -> &LayerResultIdentity {
        match self {
            Self::Validated(lease) => lease.identity(),
            Self::InThisTransaction(candidate) => candidate.identity(),
        }
    }

    fn kind(&self) -> SelectedTextureKind {
        match self {
            Self::Validated(_) => SelectedTextureKind::Validated,
            Self::InThisTransaction(_) => SelectedTextureKind::InThisTransaction,
        }
    }

    fn pin(&self, txn: &mut NativeGpuFrameTxn<'_>) -> Result<GpuAllocationReadUse, NativeGpuFault> {
        match self {
            Self::Validated(lease) => lease.pin_for_read(txn),
            Self::InThisTransaction(candidate) => candidate.pin_for_read(txn),
        }
    }
}

struct NativeCompositionPipelines {
    layout: wgpu::BindGroupLayout,
    clear: wgpu::ComputePipeline,
    composite: wgpu::ComputePipeline,
}

/// Shared-context compositor. It owns no device; its pipelines are created in
/// G's scoped Initialize phase at bootstrap, never between ISF and P.
pub(crate) struct NativeGpuCompositor {
    context: GpuContextId,
    pipelines: Arc<NativeCompositionPipelines>,
}

/// P's exact sub-reservation, split from the admitted frame root before ISF
/// binds anything, so neither lane can spend the other lane's planned slots.
pub(crate) struct CompositionReservation {
    frame: NativeFrameId,
    transaction: RetirementId,
    reservation: FrameReservation,
}

impl NativeGpuCompositor {
    pub(crate) fn from_shared(owner: &mut NativeGpuOwner) -> Result<Self, NativeGpuFault> {
        let context = owner.context().id();
        let pipelines = owner.initialize(|device, _| {
            let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            };
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Syndocal shared texture composition layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    storage(1, false),
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    storage(3, true),
                ],
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Syndocal shared texture composition pipelines"),
                bind_group_layouts: &[&layout],
                push_constant_ranges: &[],
            });
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Syndocal shared texture composition shader"),
                source: wgpu::ShaderSource::Wgsl(TEXTURE_COMPOSITION_SHADER.into()),
            });
            let pipeline = |entry| {
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Syndocal shared texture composition pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    cache: None,
                })
            };
            NativeCompositionPipelines {
                clear: pipeline("clear_output"),
                composite: pipeline("composite_layer"),
                layout,
            }
        })?;
        Ok(Self {
            context,
            pipelines: Arc::new(pipelines),
        })
    }

    fn check_transaction(
        &self,
        txn: &NativeGpuFrameTxn<'_>,
        plan: &NativeCompositionPlan,
    ) -> Result<(), NativeGpuFault> {
        if self.context != txn.context().id()
            || plan.context != self.context
            || txn.frame() != plan.frame
        {
            return Err(composition_fault(
                plan.context,
                plan.frame,
                NativeGpuFaultKind::IdentityMismatch,
                "composition context/frame does not match the shared transaction",
            ));
        }
        Ok(())
    }

    /// Call immediately after `begin_frame`, before G encodes ISF. The frame
    /// quote must include `plan.allocation_plan()`; the split is exact and once.
    pub(crate) fn reserve(
        &self,
        txn: &mut NativeGpuFrameTxn<'_>,
        plan: &NativeCompositionPlan,
    ) -> Result<CompositionReservation, NativeGpuFault> {
        self.check_transaction(txn, plan)?;
        txn.validate_plan(plan.allocation_plan())?;
        let reservation = txn.split_reservation(plan.allocation_plan())?;
        Ok(CompositionReservation {
            frame: plan.frame,
            transaction: txn.id(),
            reservation,
        })
    }

    /// Opens P's own `CompositionSnapshot` scope after G sealed the ISF scope
    /// at its submit, then submits one command buffer: GPU clear, every layer
    /// from G's read views, and the single premapping snapshot copy. There is
    /// no poll, wait, map, cache update or surface operation here.
    ///
    /// Failure after the phase opened is recorded on this phase and sealed by
    /// an empty submit, so the transaction still resolves the earlier ISF
    /// phase independently (ISF-valid/composition-failed keeps ISF history).
    pub(crate) fn encode_composition_and_snapshot(
        &self,
        txn: &mut NativeGpuFrameTxn<'_>,
        plan: &NativeCompositionPlan,
        inputs: &[SelectedLayerInput<'_>],
        reservation: CompositionReservation,
    ) -> Result<EncodedPremappingSnapshot, NativeGpuFault> {
        self.check_transaction(txn, plan)?;
        let mismatch = |message| {
            composition_fault(
                plan.context,
                plan.frame,
                NativeGpuFaultKind::IdentityMismatch,
                message,
            )
        };
        if reservation.transaction != txn.id() || reservation.frame != plan.frame {
            return Err(mismatch(
                "composition reservation belongs to another transaction",
            ));
        }
        if inputs.len() != plan.layers_bottom_first.len() {
            return Err(mismatch("composition input count differs from its plan"));
        }
        for (layer, input) in plan.layers_bottom_first.iter().zip(inputs) {
            if input.kind() != layer.kind
                || input.identity() != &layer.identity
                || input.identity().entry != layer.selected_entry
            {
                return Err(mismatch(
                    "selected input does not match its planned G lease",
                ));
            }
        }
        let reads = inputs
            .iter()
            .map(|input| input.pin(txn))
            .collect::<Result<Vec<_>, _>>()?;
        let transaction = txn.id();
        let phase = txn.begin_phase(GpuPhase::CompositionSnapshot)?;
        let mut port = SharedCompositionPort {
            context: plan.context,
            frame: plan.frame,
            phase: Some(phase),
            reservation: reservation.reservation,
            reads,
            pipelines: Arc::clone(&self.pipelines),
            extent: plan.extent,
            encoding: None,
            next_layer: 0,
            copied: false,
        };
        let encoded = match plan.encode(&mut port) {
            Ok(encoded) => encoded,
            Err(fault) => {
                let sealed = port.phase.take().map(|mut phase| {
                    phase.record_failure(fault.kind, fault.message.clone());
                    phase.finish(Vec::new())
                });
                drop(port);
                if let Some(sealed) = sealed {
                    // If even the empty seal is refused, the phase stays open
                    // and G's transaction finish/drop abandons and retires it.
                    let _ = txn.submit_phase(sealed);
                }
                return Err(fault);
            }
        };
        drop(port);
        let receipt = txn.submit_phase(encoded.phase)?;
        Ok(EncodedPremappingSnapshot {
            output: encoded.output,
            readback: encoded.readback,
            cpu_charge: encoded.cpu_charge,
            receipt,
            transaction,
            frame: plan.frame,
            extent: plan.extent,
        })
    }
}

/// A charged physical buffer. The charge lives exactly as long as the buffer
/// owner, including retained GPU uses and the later mapping input.
pub(crate) struct NativeChargedBuffer {
    buffer: wgpu::Buffer,
    charge: AllocationCharge,
}

impl NativeChargedBuffer {
    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub(crate) fn allocation_id(&self) -> GpuAllocationId {
        self.charge.id()
    }
}

/// Submitted encoding owners only; never completion or pixel evidence.
pub(crate) struct EncodedPremappingSnapshot {
    output: Arc<NativeChargedBuffer>,
    readback: Arc<NativeChargedBuffer>,
    cpu_charge: AllocationCharge,
    receipt: PendingPhaseReceipt,
    transaction: RetirementId,
    frame: NativeFrameId,
    extent: (u32, u32),
}

impl EncodedPremappingSnapshot {
    pub(crate) fn submission(&self) -> GpuSubmissionId {
        self.receipt.submission()
    }

    pub(crate) fn frame(&self) -> NativeFrameId {
        self.frame
    }

    pub(crate) fn extent(&self) -> (u32, u32) {
        self.extent
    }

    /// Snapshot-module hand-off. The receipt alone cannot mint any proof; it
    /// only selects P's phase from G's resolved transaction.
    pub(crate) fn into_parts(self) -> EncodedPremappingParts {
        EncodedPremappingParts {
            output: self.output,
            readback: self.readback,
            cpu_charge: self.cpu_charge,
            receipt: self.receipt,
            transaction: self.transaction,
            frame: self.frame,
            extent: self.extent,
        }
    }
}

pub(crate) struct EncodedPremappingParts {
    pub output: Arc<NativeChargedBuffer>,
    pub readback: Arc<NativeChargedBuffer>,
    pub cpu_charge: AllocationCharge,
    pub receipt: PendingPhaseReceipt,
    pub transaction: RetirementId,
    pub frame: NativeFrameId,
    pub extent: (u32, u32),
}

struct ComposedLayer {
    bind_group: wgpu::BindGroup,
    _lookup: NativeChargedBuffer,
    _params: NativeChargedBuffer,
}

/// Strong owners retained by G's registry until every dependent submission
/// completed: the target, readback, layer buffers, bind groups, and the
/// selected read uses whose views the bind groups sample.
struct ComposedResources {
    output: Arc<NativeChargedBuffer>,
    readback: Arc<NativeChargedBuffer>,
    layers: Vec<ComposedLayer>,
    _reads: Vec<GpuAllocationReadUse>,
    _pipelines: Arc<NativeCompositionPipelines>,
}

struct PortEncoding {
    encoder: wgpu::CommandEncoder,
    resources: Arc<ComposedResources>,
    cpu_charge: AllocationCharge,
}

struct SharedEncodedPhase {
    phase: EncodedGpuPhase,
    output: Arc<NativeChargedBuffer>,
    readback: Arc<NativeChargedBuffer>,
    cpu_charge: AllocationCharge,
}

struct SharedCompositionPort<'a> {
    context: GpuContextId,
    frame: NativeFrameId,
    phase: Option<GpuPhaseEncoder<'a>>,
    reservation: FrameReservation,
    reads: Vec<GpuAllocationReadUse>,
    pipelines: Arc<NativeCompositionPipelines>,
    extent: (u32, u32),
    encoding: Option<PortEncoding>,
    next_layer: usize,
    copied: bool,
}

impl SharedCompositionPort<'_> {
    fn state_fault(&self, message: &str) -> NativeGpuFault {
        composition_fault(
            self.context,
            self.frame,
            NativeGpuFaultKind::StateMismatch,
            message,
        )
    }
}

impl CompositionEncodingPort for SharedCompositionPort<'_> {
    type Encoded = SharedEncodedPhase;
    type Error = NativeGpuFault;

    fn begin(&mut self, plan: &NativeCompositionPlan) -> Result<(), Self::Error> {
        if self.encoding.is_some() || self.reads.len() != plan.layers_bottom_first.len() {
            return Err(self.state_fault("composition port began twice or lost its reads"));
        }
        let Some(phase) = self.phase.as_mut() else {
            return Err(self.state_fault("composition phase is already sealed"));
        };
        // Bind every charge before creating its physical allocation.
        let output_charge = phase.bind_reserved_allocation(
            &mut self.reservation,
            ChargeKind::CompositionBuffer,
            plan.allocations.premapping_bytes,
        )?;
        let readback_charge = phase.bind_reserved_allocation(
            &mut self.reservation,
            ChargeKind::SnapshotReadback,
            plan.allocations.readback_bytes,
        )?;
        let cpu_charge = phase.bind_reserved_allocation(
            &mut self.reservation,
            ChargeKind::CpuSnapshot,
            plan.allocations.premapping_bytes,
        )?;
        let mut layer_charges = Vec::with_capacity(plan.layers_bottom_first.len());
        for layer in &plan.layers_bottom_first {
            let lookup = phase.bind_reserved_allocation(
                &mut self.reservation,
                ChargeKind::CompositionBuffer,
                layer.lookup.len() as u64 * 4,
            )?;
            let params = phase.bind_reserved_allocation(
                &mut self.reservation,
                ChargeKind::CompositionBuffer,
                LAYER_PARAMS_BYTES,
            )?;
            layer_charges.push((lookup, params));
        }
        let device = phase.device();
        let output = Arc::new(NativeChargedBuffer {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Syndocal shared premapping composition"),
                size: plan.allocations.premapping_bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            charge: output_charge,
        });
        let readback = Arc::new(NativeChargedBuffer {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Syndocal shared premapping snapshot readback"),
                size: plan.allocations.readback_bytes,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            charge: readback_charge,
        });
        let mut ids = vec![output.allocation_id(), readback.allocation_id()];
        let mut layers = Vec::with_capacity(layer_charges.len());
        for ((layer, read), (lookup_charge, params_charge)) in plan
            .layers_bottom_first
            .iter()
            .zip(&self.reads)
            .zip(layer_charges)
        {
            let lookup_bytes: Vec<u8> = layer
                .lookup
                .iter()
                .flat_map(|word| word.to_le_bytes())
                .collect();
            let lookup = NativeChargedBuffer {
                buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Syndocal shared composition integer lookup"),
                    contents: &lookup_bytes,
                    usage: wgpu::BufferUsages::STORAGE,
                }),
                charge: lookup_charge,
            };
            let params = NativeChargedBuffer {
                buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Syndocal shared composition layer params"),
                    contents: &layer.params,
                    usage: wgpu::BufferUsages::UNIFORM,
                }),
                charge: params_charge,
            };
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Syndocal shared composition G read view"),
                layout: &self.pipelines.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(read.view()),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: lookup.buffer.as_entire_binding(),
                    },
                ],
            });
            ids.push(lookup.allocation_id());
            ids.push(params.allocation_id());
            layers.push(ComposedLayer {
                bind_group,
                _lookup: lookup,
                _params: params,
            });
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Syndocal shared composition and one premapping snapshot"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Syndocal shared premapping clear"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.clear);
            pass.set_bind_group(0, &layers[0].bind_group, &[]);
            pass.dispatch_workgroups(self.extent.0.div_ceil(8), self.extent.1.div_ceil(8), 1);
        }
        let resources = Arc::new(ComposedResources {
            output,
            readback,
            layers,
            _reads: std::mem::take(&mut self.reads),
            _pipelines: Arc::clone(&self.pipelines),
        });
        phase.retain_resource(Arc::clone(&resources), &ids)?;
        self.encoding = Some(PortEncoding {
            encoder,
            resources,
            cpu_charge,
        });
        Ok(())
    }

    fn composite_layer(&mut self, _layer: &CompositionLayerEncoding) -> Result<(), Self::Error> {
        let index = self.next_layer;
        let Some(encoding) = self.encoding.as_mut() else {
            return Err(self.state_fault("composition layer before begin"));
        };
        let Some(layer) = encoding.resources.layers.get(index) else {
            return Err(self.state_fault("more composited layers than planned"));
        };
        {
            let mut pass = encoding
                .encoder
                .begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Syndocal shared post-FX layer"),
                    timestamp_writes: None,
                });
            pass.set_pipeline(&self.pipelines.composite);
            pass.set_bind_group(0, &layer.bind_group, &[]);
            pass.dispatch_workgroups(self.extent.0.div_ceil(8), self.extent.1.div_ceil(8), 1);
        }
        self.next_layer += 1;
        Ok(())
    }

    fn copy_premapping_to_snapshot(&mut self, bytes: u64) -> Result<(), Self::Error> {
        let Some(encoding) = self.encoding.as_mut() else {
            return Err(self.state_fault("snapshot copy before begin"));
        };
        let resources = &encoding.resources;
        if self.copied
            || bytes != resources.output.charge.capacity()
            || bytes != resources.readback.charge.capacity()
        {
            return Err(self.state_fault("exactly one full premapping snapshot copy is allowed"));
        }
        encoding.encoder.copy_buffer_to_buffer(
            &resources.output.buffer,
            0,
            &resources.readback.buffer,
            0,
            bytes,
        );
        self.copied = true;
        Ok(())
    }

    fn finish(&mut self) -> Result<Self::Encoded, Self::Error> {
        let complete = self.copied
            && self
                .encoding
                .as_ref()
                .is_some_and(|encoding| encoding.resources.layers.len() == self.next_layer);
        if !complete || self.phase.is_none() {
            return Err(self.state_fault("composition encoding is incomplete"));
        }
        let (Some(encoding), Some(phase)) = (self.encoding.take(), self.phase.take()) else {
            return Err(self.state_fault("composition encoding is incomplete"));
        };
        let command = encoding.encoder.finish();
        Ok(SharedEncodedPhase {
            phase: phase.finish(vec![command]),
            output: Arc::clone(&encoding.resources.output),
            readback: Arc::clone(&encoding.resources.readback),
            cpu_charge: encoding.cpu_charge,
        })
    }
}

fn composition_fault(
    context: GpuContextId,
    frame: NativeFrameId,
    kind: NativeGpuFaultKind,
    message: &str,
) -> NativeGpuFault {
    NativeGpuFault {
        context,
        frame: Some(frame),
        phase: GpuPhase::CompositionSnapshot,
        kind,
        message: message.into(),
    }
}

#[cfg(test)]
#[path = "native_gpu_composition_tests.rs"]
pub(crate) mod tests;
