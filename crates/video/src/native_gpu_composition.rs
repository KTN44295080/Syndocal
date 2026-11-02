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

use std::cell::RefCell;
use std::sync::{Arc, Weak};

use wgpu::util::DeviceExt;

use crate::isf_chain_execution::PreparedStage;
use crate::isf_runtime::{IsfGpuRuntime, ResidentGroupFxTargets};
use crate::native_gpu_budget::{
    AllocationCharge, ChargeKind, FrameReservation, NativeAllocationPlan,
};
use crate::native_gpu_context::{
    EncodedGpuPhase, GpuPhaseEncoder, NativeGpuFrameTxn, NativeGpuOwner, PendingPhaseReceipt,
};
use crate::native_gpu_contract::{
    FrozenChainClocks, GpuAllocationId, GpuContextId, GpuPhase, GpuSubmissionId, HistoryEntryId,
    LayerResultIdentity, NativeFrameId, NativeGpuDecline, NativeGpuFault, NativeGpuFaultKind,
    RetirementId,
};
use crate::native_gpu_result::{EncodedLayerCandidate, GpuAllocationReadUse, GpuLayerLease};
use crate::native_gpu_snapshot::RecycledCpuSnapshot;
use crate::{axis_aligned_source_lookup, CompositionLayerPlan, VideoPixelFormat};

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

/// A prepared group FX chain. Only stateless stages are admitted
/// (`isf_runtime::resident_group_stages`); the rest stay canonical.
pub(crate) struct ResidentGroupEffect {
    pub stages: Vec<PreparedStage>,
    pub date: [f32; 4],
}

/// The caller resolves and prepares executing group FX before admission.
pub(crate) struct NativeCompositionGroup<'a> {
    pub group: &'a protocol::VideoLayerGroupSummary,
    pub fx: Option<Arc<ResidentGroupEffect>>,
}

/// Group FX runs on the straightened isolated composite, before the group
/// transform and outer blend, with the canonical stack's exact uniforms.
struct GroupFxEncoding {
    effect: Arc<ResidentGroupEffect>,
    clocks: FrozenChainClocks,
    /// Planned pass uniforms. Their sizes are fixed here; FRAMEINDEX is
    /// reserved from the runtime's canonical logical clock at encode.
    uniforms: Vec<Vec<u8>>,
}

impl std::fmt::Debug for GroupFxEncoding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GroupFxEncoding")
            .field("stages", &self.effect.stages.len())
            .field("passes", &self.uniforms.len())
            .finish()
    }
}

#[derive(Debug)]
struct GroupSampling {
    lookup: Vec<u32>,
    params: [u8; 32],
}
#[derive(Debug)]
struct GroupEncoding {
    members: Vec<usize>,
    transform: Option<GroupSampling>,
    outer: GroupSampling,
    fx: Option<GroupFxEncoding>,
}

impl GroupEncoding {
    fn needs_transform_target(&self) -> bool {
        self.transform.is_some() && self.fx.is_none()
    }
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
    groups: Vec<GroupEncoding>,
    /// A completed, unmapped snapshot readback of exactly this extent, reused
    /// instead of a new allocation (ADR-01 section 7.2 ring, section 11 C/E).
    reused_readback: Option<Arc<NativeChargedBuffer>>,
    reused_output: Option<Arc<NativeChargedBuffer>>,
    /// The displaced LastValid's packed buffer and its CPU charge, reused as
    /// this frame's CPU snapshot (an existing alias instead of a new charge).
    /// Taken once by the encoding port.
    reused_cpu_snapshot: RefCell<Option<RecycledCpuSnapshot>>,
    reused_cpu_snapshot_id: Option<GpuAllocationId>,
    pool_epoch: Option<u64>,
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
    #[cfg(test)]
    pub(crate) fn preflight(
        context: GpuContextId,
        frame: NativeFrameId,
        extent: (u32, u32),
        clear_color: [u8; 4],
        layers_bottom_first: &[NativeCompositionLayer<'_>],
        max_texture_dimension_2d: u32,
        max_storage_buffer_bytes: u64,
    ) -> Result<Self, CompositionPlanFailure> {
        Self::preflight_with_groups(
            context,
            frame,
            extent,
            clear_color,
            layers_bottom_first,
            &[],
            max_texture_dimension_2d,
            max_storage_buffer_bytes,
        )
    }

    /// [`Self::preflight`] with the composition's enabled, unbypassed groups.
    /// Each active group composes its members into an isolated target, then
    /// onto the output once, at its bottom-most member's position.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn preflight_with_groups(
        context: GpuContextId,
        frame: NativeFrameId,
        extent: (u32, u32),
        clear_color: [u8; 4],
        layers_bottom_first: &[NativeCompositionLayer<'_>],
        groups: &[NativeCompositionGroup<'_>],
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
            if identity.width == 0
                || identity.height == 0
                || identity.width > max_texture_dimension_2d
                || identity.height > max_texture_dimension_2d
            {
                return Err(decline(NativeGpuDecline::UnsupportedExtent));
            }
            if !resident_composition_layer(layer.plan) {
                return Err(decline(NativeGpuDecline::UnsupportedComposition));
            }
            let (lookup, rotated) = composition_sampling(
                &layer.plan.transform,
                extent,
                (identity.width, identity.height),
            );
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
                rotated,
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
        let groups = prepare_groups(
            groups,
            layers_bottom_first,
            &mut layers,
            extent,
            max_storage_buffer_bytes,
        )
        .map_err(decline)?;
        for group in &groups {
            for sampling in group.transform.iter().chain(std::iter::once(&group.outer)) {
                lookup_bytes += sampling.lookup.len() as u64 * 4;
                uniform_bytes += LAYER_PARAMS_BYTES;
            }
            for uniform in group.fx.iter().flat_map(|fx| &fx.uniforms) {
                uniform_bytes += uniform.len() as u64;
            }
        }
        let native_allocations = composition_allocations(
            frame,
            output_bytes,
            &layers,
            &groups,
            AllocationReuse::default(),
        )
        .map_err(decline)?;
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
            groups,
            reused_readback: None,
            reused_output: None,
            reused_cpu_snapshot: RefCell::new(None),
            reused_cpu_snapshot_id: None,
            pool_epoch: None,
        })
    }

    fn reuse(&self) -> AllocationReuse {
        AllocationReuse {
            readback: self
                .reused_readback
                .as_ref()
                .map(|buffer| buffer.allocation_id()),
            output: self
                .reused_output
                .as_ref()
                .map(|buffer| buffer.allocation_id()),
            cpu_snapshot: self.reused_cpu_snapshot_id,
        }
    }

    /// Replace this plan's new CPU snapshot charge and allocation by the
    /// displaced LastValid's packed buffer and its charge (ADR-01 2P swap,
    /// C4R12). Call before the plan is merged into the frame quote; the charge
    /// then becomes an existing alias of that quote. A mismatching context,
    /// kind or capacity is returned unchanged for the caller to drop.
    pub(crate) fn reuse_cpu_snapshot(
        &mut self,
        recycled: RecycledCpuSnapshot,
    ) -> Result<(), RecycledCpuSnapshot> {
        let charge = recycled.charge();
        let id = charge.id();
        if self.reused_cpu_snapshot_id.is_some()
            || id.context != self.context
            || charge.kind() != ChargeKind::CpuSnapshot
            || charge.capacity() != self.allocations.premapping_bytes
            || recycled.buffer_capacity() as u64 != self.allocations.premapping_bytes
        {
            return Err(recycled);
        }
        let reuse = AllocationReuse {
            cpu_snapshot: Some(id),
            ..self.reuse()
        };
        let Ok(native_allocations) = composition_allocations(
            self.frame,
            self.allocations.premapping_bytes,
            &self.layers_bottom_first,
            &self.groups,
            reuse,
        ) else {
            return Err(recycled);
        };
        self.native_allocations = native_allocations;
        self.reused_cpu_snapshot_id = Some(id);
        *self.reused_cpu_snapshot.get_mut() = Some(recycled);
        Ok(())
    }

    fn take_reused_cpu_snapshot(&self) -> Option<RecycledCpuSnapshot> {
        self.reused_cpu_snapshot.borrow_mut().take()
    }

    /// Before this tick leaves for the CPU route, release the recycled buffer
    /// (and its charge) so idle resident storage does not consume the CPU
    /// handover envelope.
    pub(crate) fn release_reused_cpu_snapshot(&mut self) {
        drop(self.reused_cpu_snapshot.get_mut().take());
    }

    /// Replace this plan's new snapshot readback by an existing, completed and
    /// unmapped readback of the same context and exact capacity. Call before
    /// the plan is merged into the frame quote; the readback then becomes an
    /// existing alias of that quote instead of a second charge. A mismatching
    /// or still-shared buffer is returned unchanged for the caller to drop.
    pub(crate) fn reuse_readback(
        &mut self,
        readback: Arc<NativeChargedBuffer>,
    ) -> Result<(), Arc<NativeChargedBuffer>> {
        let id = readback.allocation_id();
        if self.reused_readback.is_some()
            || Arc::strong_count(&readback) != 1
            || id.context != self.context
            || readback.charge.kind() != ChargeKind::SnapshotReadback
            || readback.capacity() != self.allocations.readback_bytes
            || readback.buffer.size() != self.allocations.readback_bytes
        {
            return Err(readback);
        }
        let reuse = AllocationReuse {
            readback: Some(id),
            ..self.reuse()
        };
        let Ok(native_allocations) = composition_allocations(
            self.frame,
            self.allocations.premapping_bytes,
            &self.layers_bottom_first,
            &self.groups,
            reuse,
        ) else {
            return Err(readback);
        };
        self.native_allocations = native_allocations;
        self.reused_readback = Some(readback);
        Ok(())
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
    group_layout: wgpu::BindGroupLayout,
    group_composite: wgpu::ComputePipeline,
    group_clear: wgpu::ComputePipeline,
    group_straighten: wgpu::ComputePipeline,
    fx_unpack_layout: wgpu::BindGroupLayout,
    fx_unpack: wgpu::ComputePipeline,
    fx_pack_layout: wgpu::BindGroupLayout,
    fx_pack: wgpu::ComputePipeline,
    fx_sampler: wgpu::Sampler,
}

/// Shared-context compositor. It owns no device; its pipelines are created in
/// G's scoped Initialize phase at bootstrap, never between ISF and P.
pub(crate) struct NativeGpuCompositor {
    context: GpuContextId,
    pipelines: Arc<NativeCompositionPipelines>,
    output_pool: RefCell<CompositionOutputPool<NativeChargedBuffer>>,
}

/// P's exact sub-reservation, split from the admitted frame root before ISF
/// binds anything, so neither lane can spend the other lane's planned slots.
pub(crate) struct CompositionReservation {
    frame: NativeFrameId,
    transaction: RetirementId,
    reservation: FrameReservation,
}

/// G owns the bank's strong references and releases them at the lifecycle
/// fence. P keeps only weak metadata, so it cannot prolong revoked resources.
/// Ready/LastValid and the transaction registry pin the same Arc; checkout is
/// possible only when G's bank is the sole remaining strong owner.
struct CompositionOutputPool<T> {
    key: Option<(u64, u64, (u32, u32))>,
    slots: Vec<Weak<T>>,
}

const COMPOSITION_OUTPUT_POOL_SLOTS: usize = 3;

impl<T> Default for CompositionOutputPool<T> {
    fn default() -> Self {
        Self {
            key: None,
            slots: Vec::new(),
        }
    }
}

impl<T> CompositionOutputPool<T> {
    fn prepare(&mut self, key: (u64, u64, (u32, u32))) -> Result<Option<Arc<T>>, ()> {
        if self.key != Some(key) {
            self.slots.clear();
            self.key = Some(key);
        }
        self.slots.retain(|slot| slot.strong_count() != 0);
        for slot in &self.slots {
            if let Some(output) = slot.upgrade() {
                if Arc::strong_count(&output) == 2 {
                    return Ok(Some(output));
                }
            }
        }
        if self.slots.len() == COMPOSITION_OUTPUT_POOL_SLOTS {
            Err(())
        } else {
            Ok(None)
        }
    }

    fn retain(&mut self, key: (u64, u64, (u32, u32)), output: &Arc<T>) {
        let weak = Arc::downgrade(output);
        if self.key == Some(key)
            && self.slots.len() < COMPOSITION_OUTPUT_POOL_SLOTS
            && !self.slots.iter().any(|slot| Weak::ptr_eq(slot, &weak))
        {
            self.slots.push(weak);
        }
    }

    fn clear(&mut self) {
        self.slots.clear();
        self.key = None;
    }
}

impl NativeGpuCompositor {
    /// Call before merging the composition allocation plan into the quote.
    /// Existing slots contribute aliases, never duplicate physical charges.
    pub(crate) fn prepare_pooled_output(
        &self,
        plan: &mut NativeCompositionPlan,
        pool_epoch: u64,
    ) -> Result<(), NativeGpuDecline> {
        if plan.context != self.context || plan.reused_output.is_some() {
            return Err(NativeGpuDecline::QuoteExpired);
        }
        let mut pool = self
            .output_pool
            .try_borrow_mut()
            .map_err(|_| NativeGpuDecline::QuoteExpired)?;
        let output = pool
            .prepare((pool_epoch, plan.frame.geometry_generation, plan.extent))
            .map_err(|_| NativeGpuDecline::BudgetExceeded {
                live: plan.allocations.premapping_bytes * COMPOSITION_OUTPUT_POOL_SLOTS as u64,
                requested: plan.allocations.premapping_bytes,
                limit: plan.allocations.premapping_bytes * COMPOSITION_OUTPUT_POOL_SLOTS as u64,
            })?;
        let reuse = AllocationReuse {
            output: output.as_ref().map(|buffer| buffer.allocation_id()),
            ..plan.reuse()
        };
        plan.native_allocations = composition_allocations(
            plan.frame,
            plan.allocations.premapping_bytes,
            &plan.layers_bottom_first,
            &plan.groups,
            reuse,
        )?;
        plan.reused_output = output;
        plan.pool_epoch = Some(pool_epoch);
        Ok(())
    }

    /// Stop, authority revocation/HardBlackout and device loss must release
    /// the bank through G. This clears weak metadata only; outstanding owners
    /// remain charged until safely retired.
    pub(crate) fn release_pooled_outputs(&self) {
        self.output_pool.borrow_mut().clear();
    }

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
            let group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Syndocal resident packed group layout"),
                entries: &[
                    storage(0, true),
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
            let group_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Syndocal resident packed group pipelines"),
                    bind_group_layouts: &[&group_layout],
                    push_constant_ranges: &[],
                });
            let group_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Syndocal resident packed group shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("gpu_display_composition.wgsl").into(),
                ),
            });
            let group_pipeline = |entry| {
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Syndocal resident packed group pipeline"),
                    layout: Some(&group_pipeline_layout),
                    module: &group_shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    cache: None,
                })
            };
            let uniform = |binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            };
            let fx_unpack_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Syndocal resident group FX unpack layout"),
                    entries: &[
                        storage(0, true),
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        uniform(2),
                    ],
                });
            let fx_pack_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Syndocal resident group FX pack layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        storage(4, false),
                        uniform(5),
                    ],
                });
            let fx_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Syndocal resident group FX bridge shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("gpu_display_group_fx.wgsl").into()),
            });
            let fx_pipeline = |layout: &wgpu::BindGroupLayout, entry| {
                let pipeline_layout =
                    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Syndocal resident group FX bridge pipelines"),
                        bind_group_layouts: &[layout],
                        push_constant_ranges: &[],
                    });
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Syndocal resident group FX bridge pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &fx_shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    cache: None,
                })
            };
            NativeCompositionPipelines {
                clear: pipeline("clear_output"),
                composite: pipeline("composite_layer"),
                layout,
                group_composite: group_pipeline("composite_resident_group"),
                group_clear: group_pipeline("clear_resident_output"),
                group_straighten: group_pipeline("straighten_resident_output"),
                group_layout,
                fx_unpack: fx_pipeline(&fx_unpack_layout, "unpack_group"),
                fx_pack: fx_pipeline(&fx_pack_layout, "pack_group"),
                fx_unpack_layout,
                fx_pack_layout,
                fx_sampler: crate::isf_runtime::resident_group_sampler(device),
            }
        })?;
        Ok(Self {
            context,
            pipelines: Arc::new(pipelines),
            output_pool: RefCell::new(CompositionOutputPool::default()),
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
            || plan
                .pool_epoch
                .is_some_and(|epoch| epoch != txn.context().resource_pool_epoch())
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
    /// Group FX passes join the same command buffer through `runtime`.
    pub(crate) fn encode_composition_and_snapshot(
        &self,
        txn: &mut NativeGpuFrameTxn<'_>,
        plan: &NativeCompositionPlan,
        inputs: &[SelectedLayerInput<'_>],
        reservation: CompositionReservation,
        runtime: Option<&mut IsfGpuRuntime>,
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
        if runtime.is_none() && plan.groups.iter().any(|group| group.fx.is_some()) {
            return Err(mismatch("group FX composition has no ISF runtime"));
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
            reused_cpu_snapshot: plan.take_reused_cpu_snapshot(),
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
            runtime,
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
        // Retain only after submission was admitted. The transaction's
        // ComposedResources Arc pins this output until verified retirement.
        if let Some(epoch) = plan.pool_epoch {
            self.output_pool
                .try_borrow_mut()
                .map_err(|_| mismatch("composition output pool already borrowed"))?
                .retain(
                    (epoch, plan.frame.geometry_generation, plan.extent),
                    &encoded.output,
                );
        }
        Ok(EncodedPremappingSnapshot {
            output: encoded.output,
            readback: encoded.readback,
            cpu_charge: encoded.cpu_charge,
            cpu_buffer: encoded.cpu_buffer,
            receipt,
            transaction,
            frame: plan.frame,
            extent: plan.extent,
        })
    }
}

/// A charged physical buffer. The charge lives exactly as long as the buffer
/// owner, including retained GPU uses and the later mapping input.
#[derive(Debug)]
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

    pub(crate) fn capacity(&self) -> u64 {
        self.charge.capacity()
    }
}

/// Submitted encoding owners only; never completion or pixel evidence.
pub(crate) struct EncodedPremappingSnapshot {
    output: Arc<NativeChargedBuffer>,
    readback: Arc<NativeChargedBuffer>,
    cpu_charge: AllocationCharge,
    /// The recycled packed buffer `cpu_charge` covers, if one was reused.
    cpu_buffer: Option<Vec<u8>>,
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
            cpu_buffer: self.cpu_buffer,
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
    pub cpu_buffer: Option<Vec<u8>>,
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

struct ComposedGroup {
    members: Vec<usize>,
    _buffers: Vec<Arc<NativeChargedBuffer>>,
    transform: Option<ComposedLayer>,
    outer: ComposedLayer,
    straighten: wgpu::BindGroup,
    fx: Option<ComposedGroupFx>,
}

struct ComposedGroupFx {
    effect: Arc<ResidentGroupEffect>,
    uniforms: Vec<NativeChargedBuffer>,
    unpack: wgpu::BindGroup,
    /// Untransformed groups pack the FX result back into the isolated target.
    pack: Option<wgpu::BindGroup>,
}

/// Strong owners retained by G's registry until every dependent submission
/// completed: the target, readback, layer buffers, bind groups, and the
/// selected read uses whose views the bind groups sample.
struct ComposedResources {
    output: Arc<NativeChargedBuffer>,
    readback: Arc<NativeChargedBuffer>,
    layers: Vec<ComposedLayer>,
    groups: Vec<ComposedGroup>,
    /// Groups encode one after another, so FX groups share one ping/pong.
    fx_targets: Option<ResidentGroupFxTargets>,
    _reads: Vec<GpuAllocationReadUse>,
    _pipelines: Arc<NativeCompositionPipelines>,
}

struct PortEncoding {
    encoder: wgpu::CommandEncoder,
    resources: Arc<ComposedResources>,
    cpu_charge: AllocationCharge,
    cpu_buffer: Option<Vec<u8>>,
}

struct SharedEncodedPhase {
    phase: EncodedGpuPhase,
    output: Arc<NativeChargedBuffer>,
    readback: Arc<NativeChargedBuffer>,
    cpu_charge: AllocationCharge,
    cpu_buffer: Option<Vec<u8>>,
}

struct SharedCompositionPort<'a> {
    reused_cpu_snapshot: Option<RecycledCpuSnapshot>,
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
    runtime: Option<&'a mut IsfGpuRuntime>,
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
        let output_charge = match plan.reused_output.as_ref() {
            Some(_) => None,
            None => Some(phase.bind_reserved_allocation(
                &mut self.reservation,
                ChargeKind::CompositionBuffer,
                plan.allocations.premapping_bytes,
            )?),
        };
        let readback_charge = match plan.reused_readback.as_ref() {
            Some(_) => None,
            None => Some(phase.bind_reserved_allocation(
                &mut self.reservation,
                ChargeKind::SnapshotReadback,
                plan.allocations.readback_bytes,
            )?),
        };
        // A recycled CPU snapshot is this quote's existing alias; otherwise
        // the new charge is bound like every other planned allocation.
        let (cpu_charge, cpu_buffer) = match self.reused_cpu_snapshot.take() {
            Some(recycled) => {
                let (charge, buffer) = recycled.into_parts();
                (charge, Some(buffer))
            }
            None => (
                phase.bind_reserved_allocation(
                    &mut self.reservation,
                    ChargeKind::CpuSnapshot,
                    plan.allocations.premapping_bytes,
                )?,
                None,
            ),
        };
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
        // Groups encode one after another, so every group shares one isolated
        // target and, when any group without FX is transformed, one transform
        // target. An FX group transforms its FX result back into the isolated
        // target instead.
        let mut scratch_charge = None;
        let mut transformed_charge = None;
        let transformed = plan
            .groups
            .iter()
            .any(GroupEncoding::needs_transform_target);
        for (slot, needed) in [
            (&mut scratch_charge, !plan.groups.is_empty()),
            (&mut transformed_charge, transformed),
        ] {
            if needed {
                *slot = Some(phase.bind_reserved_allocation(
                    &mut self.reservation,
                    ChargeKind::CompositionBuffer,
                    plan.allocations.premapping_bytes,
                )?);
            }
        }
        let fx_charges = if plan.groups.iter().any(|group| group.fx.is_some()) {
            let mut charge = || {
                phase.bind_reserved_allocation(
                    &mut self.reservation,
                    ChargeKind::IsfScratch,
                    plan.allocations.premapping_bytes,
                )
            };
            Some([charge()?, charge()?])
        } else {
            None
        };
        let mut group_charges = Vec::with_capacity(plan.groups.len());
        for group in &plan.groups {
            let mut samplings = Vec::new();
            for sampling in group.transform.iter().chain(std::iter::once(&group.outer)) {
                let lookup = phase.bind_reserved_allocation(
                    &mut self.reservation,
                    ChargeKind::CompositionBuffer,
                    sampling.lookup.len() as u64 * 4,
                )?;
                let params = phase.bind_reserved_allocation(
                    &mut self.reservation,
                    ChargeKind::CompositionBuffer,
                    LAYER_PARAMS_BYTES,
                )?;
                samplings.push((lookup, params));
            }
            let mut uniforms = Vec::new();
            for uniform in group.fx.iter().flat_map(|fx| &fx.uniforms) {
                uniforms.push(phase.bind_reserved_allocation(
                    &mut self.reservation,
                    ChargeKind::CompositionBuffer,
                    uniform.len() as u64,
                )?);
            }
            group_charges.push((samplings, uniforms));
        }
        let device = phase.device();
        let output = match (plan.reused_output.as_ref(), output_charge) {
            (Some(reused), None) => Arc::clone(reused),
            (None, Some(charge)) => Arc::new(NativeChargedBuffer {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Syndocal shared premapping composition"),
                    size: plan.allocations.premapping_bytes,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                charge,
            }),
            _ => unreachable!("output is either reused or bound above"),
        };
        // A reused ring readback is an existing alias of this frame's quote; the
        // retained resource below records this submission's use of it.
        let readback = match (plan.reused_readback.as_ref(), readback_charge) {
            (Some(reused), None) => Arc::clone(reused),
            (None, Some(charge)) => Arc::new(NativeChargedBuffer {
                buffer: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Syndocal shared premapping snapshot readback"),
                    size: plan.allocations.readback_bytes,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                charge,
            }),
            _ => unreachable!("readback is either reused or bound above"),
        };
        let mut ids = vec![output.allocation_id(), readback.allocation_id()];
        let charged = |charge| {
            Arc::new(charged_group_buffer(
                device,
                plan.allocations.premapping_bytes,
                charge,
            ))
        };
        let scratch = scratch_charge.map(charged);
        let transformed = transformed_charge.map(charged);
        ids.extend(
            scratch
                .iter()
                .chain(&transformed)
                .map(|buffer| buffer.allocation_id()),
        );
        let fx_targets = fx_charges.map(|charges| {
            ids.extend(charges.iter().map(AllocationCharge::id));
            ResidentGroupFxTargets::new(device, self.extent, charges)
        });
        let mut layers = Vec::with_capacity(layer_charges.len());
        for (layer_index, ((layer, read), (lookup_charge, params_charge))) in plan
            .layers_bottom_first
            .iter()
            .zip(&self.reads)
            .zip(layer_charges)
            .enumerate()
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
            let target = plan
                .groups
                .iter()
                .position(|group| group.members.contains(&layer_index))
                .map_or(&*output, |_| {
                    scratch.as_deref().expect("groups share one target")
                });
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
                        resource: target.buffer.as_entire_binding(),
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
        // Canonical group FX consumes the runtime's logical clock once per
        // executing chain, after the layer chains and in composition order.
        let mut fx_uniforms = Vec::new();
        for fx in plan.groups.iter().filter_map(|group| group.fx.as_ref()) {
            let Some(runtime) = self.runtime.as_deref_mut() else {
                return Err(composition_fault(
                    self.context,
                    self.frame,
                    NativeGpuFaultKind::StateMismatch,
                    "group FX composition has no ISF runtime",
                ));
            };
            let clocks = FrozenChainClocks {
                frame_index: runtime.reserve_resident_group_frame_index(),
                ..fx.clocks
            };
            let uniforms =
                crate::isf_runtime::resident_group_uniforms(&fx.effect.stages, self.extent, clocks);
            if uniforms.len() != fx.uniforms.len()
                || uniforms
                    .iter()
                    .zip(&fx.uniforms)
                    .any(|(a, b)| a.len() != b.len())
            {
                return Err(composition_fault(
                    self.context,
                    self.frame,
                    NativeGpuFaultKind::StateMismatch,
                    "group FX uniforms differ from their plan",
                ));
            }
            fx_uniforms.push(uniforms);
        }
        let mut fx_uniforms = fx_uniforms.into_iter();
        let mut groups = Vec::with_capacity(plan.groups.len());
        for (group, (charges, uniform_charges)) in plan.groups.iter().zip(group_charges) {
            let mut charges = charges.into_iter();
            let scratch = scratch.as_ref().expect("groups share one target");
            let mut buffers = vec![Arc::clone(scratch)];
            if group.needs_transform_target() {
                buffers.push(Arc::clone(
                    transformed
                        .as_ref()
                        .expect("a transformed group has its target"),
                ));
            }
            let transform = group.transform.as_ref().map(|sampling| match &group.fx {
                Some(fx) => make_texture_group_layer(
                    device,
                    &self.pipelines.layout,
                    fx_targets
                        .as_ref()
                        .expect("FX groups share one ping/pong")
                        .result_view(fx.effect.stages.len()),
                    &buffers[0],
                    sampling,
                    charges.next().unwrap(),
                    &mut ids,
                ),
                None => make_group_layer(
                    device,
                    &self.pipelines.group_layout,
                    &buffers[0],
                    &buffers[1],
                    sampling,
                    charges.next().unwrap(),
                    &mut ids,
                ),
            });
            let outer_charges = charges.next().expect("outer sampling charge");
            let mut outer_sampling = GroupSampling {
                lookup: group.outer.lookup.clone(),
                params: group.outer.params,
            };
            outer_sampling.params[24..28]
                .copy_from_slice(&u32::from_le_bytes(plan.clear_color).to_le_bytes());
            let outer = make_group_layer(
                device,
                &self.pipelines.group_layout,
                buffers.last().unwrap(),
                &output,
                &outer_sampling,
                outer_charges,
                &mut ids,
            );
            let straighten = packed_group_bind_group(
                device,
                &self.pipelines.group_layout,
                &output,
                &buffers[0],
                &outer._params,
                &outer._lookup,
            );
            let fx = group.fx.as_ref().map(|fx| {
                let targets = fx_targets.as_ref().expect("FX groups share one target set");
                let uniforms = fx_uniforms
                    .next()
                    .expect("every FX group reserved its clock")
                    .iter()
                    .zip(uniform_charges)
                    .map(|(contents, charge)| {
                        let buffer = NativeChargedBuffer {
                            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Syndocal resident group FX pass uniforms"),
                                contents,
                                usage: wgpu::BufferUsages::UNIFORM,
                            }),
                            charge,
                        };
                        ids.push(buffer.allocation_id());
                        buffer
                    })
                    .collect();
                // The outer sampling's params begin with the full extent.
                let unpack = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Syndocal resident group FX unpack"),
                    layout: &self.pipelines.fx_unpack_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: buffers[0].buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(targets.input_view()),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: outer._params.buffer.as_entire_binding(),
                        },
                    ],
                });
                let pack = group.transform.is_none().then(|| {
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Syndocal resident group FX pack"),
                        layout: &self.pipelines.fx_pack_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::TextureView(
                                    targets.result_view(fx.effect.stages.len()),
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 4,
                                resource: buffers[0].buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 5,
                                resource: outer._params.buffer.as_entire_binding(),
                            },
                        ],
                    })
                });
                ComposedGroupFx {
                    effect: Arc::clone(&fx.effect),
                    uniforms,
                    unpack,
                    pack,
                }
            });
            groups.push(ComposedGroup {
                members: group.members.clone(),
                _buffers: buffers,
                transform,
                outer,
                straighten,
                fx,
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
            if let Some(group) = groups.first() {
                pass.set_pipeline(&self.pipelines.group_clear);
                pass.set_bind_group(0, &group.outer.bind_group, &[]);
            } else {
                pass.set_pipeline(&self.pipelines.clear);
                pass.set_bind_group(0, &layers[0].bind_group, &[]);
            }
            pass.dispatch_workgroups(self.extent.0.div_ceil(8), self.extent.1.div_ceil(8), 1);
        }
        if plan.pool_epoch.is_some() && plan.reused_output.is_none() {
            phase.retain_pooled_resource(Arc::clone(&output))?;
        }
        let resources = Arc::new(ComposedResources {
            output,
            readback,
            layers,
            groups,
            fx_targets,
            _reads: std::mem::take(&mut self.reads),
            _pipelines: Arc::clone(&self.pipelines),
        });
        phase.retain_resource(Arc::clone(&resources), &ids)?;
        self.encoding = Some(PortEncoding {
            encoder,
            resources,
            cpu_charge,
            cpu_buffer,
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
        let dispatch = |encoder: &mut wgpu::CommandEncoder,
                        pipeline: &wgpu::ComputePipeline,
                        bind_group: &wgpu::BindGroup| {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Syndocal resident layer or group"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(self.extent.0.div_ceil(8), self.extent.1.div_ceil(8), 1);
        };
        if let Some(group) = encoding
            .resources
            .groups
            .iter()
            .find(|group| group.members.contains(&index))
        {
            if group.members[0] == index {
                dispatch(
                    &mut encoding.encoder,
                    &self.pipelines.clear,
                    &encoding.resources.layers[index].bind_group,
                );
                for &member in &group.members {
                    dispatch(
                        &mut encoding.encoder,
                        &self.pipelines.composite,
                        &encoding.resources.layers[member].bind_group,
                    );
                }
                dispatch(
                    &mut encoding.encoder,
                    &self.pipelines.group_straighten,
                    &group.straighten,
                );
                if let Some(fx) = &group.fx {
                    let (Some(runtime), Some(phase), Some(targets)) = (
                        self.runtime.as_deref_mut(),
                        self.phase.as_mut(),
                        encoding.resources.fx_targets.as_ref(),
                    ) else {
                        return Err(composition_fault(
                            self.context,
                            self.frame,
                            NativeGpuFaultKind::StateMismatch,
                            "group FX lost its runtime, phase or targets",
                        ));
                    };
                    dispatch(&mut encoding.encoder, &self.pipelines.fx_unpack, &fx.unpack);
                    let uniforms = fx
                        .uniforms
                        .iter()
                        .map(|uniform| &uniform.buffer)
                        .collect::<Vec<_>>();
                    let mut pins = Vec::new();
                    let encoded = runtime.encode_resident_group_fx(
                        phase.context(),
                        &mut encoding.encoder,
                        targets,
                        &self.pipelines.fx_sampler,
                        &fx.effect.stages,
                        &uniforms,
                        &mut pins,
                    );
                    // Pipelines created before a failure are pinned as well.
                    phase.retain_resource(Arc::new(pins), &[])?;
                    encoded.map_err(|error| {
                        composition_fault(
                            self.context,
                            self.frame,
                            NativeGpuFaultKind::StateMismatch,
                            &format!("resident group FX encoding failed: {error}"),
                        )
                    })?;
                    match (&group.transform, &fx.pack) {
                        // The same transparent transform step as below, read
                        // from the FX result texture into the isolated target.
                        (Some(transform), _) => {
                            for pipeline in [&self.pipelines.clear, &self.pipelines.composite] {
                                dispatch(&mut encoding.encoder, pipeline, &transform.bind_group);
                            }
                            dispatch(
                                &mut encoding.encoder,
                                &self.pipelines.group_straighten,
                                &group.straighten,
                            );
                        }
                        (None, Some(pack)) => {
                            dispatch(&mut encoding.encoder, &self.pipelines.fx_pack, pack)
                        }
                        (None, None) => unreachable!("an untransformed FX group packs"),
                    }
                }
                if let (Some(transform), None) = (&group.transform, &group.fx) {
                    dispatch(
                        &mut encoding.encoder,
                        &self.pipelines.group_clear,
                        &transform.bind_group,
                    );
                    dispatch(
                        &mut encoding.encoder,
                        &self.pipelines.group_composite,
                        &transform.bind_group,
                    );
                    dispatch(
                        &mut encoding.encoder,
                        &self.pipelines.group_straighten,
                        &transform.bind_group,
                    );
                }
                dispatch(
                    &mut encoding.encoder,
                    &self.pipelines.group_composite,
                    &group.outer.bind_group,
                );
            }
        } else {
            dispatch(
                &mut encoding.encoder,
                &self.pipelines.composite,
                &layer.bind_group,
            );
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
            cpu_buffer: encoding.cpu_buffer,
        })
    }
}

fn charged_group_buffer(
    device: &wgpu::Device,
    bytes: u64,
    charge: AllocationCharge,
) -> NativeChargedBuffer {
    NativeChargedBuffer {
        buffer: device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Syndocal charged isolated group"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        }),
        charge,
    }
}
fn packed_group_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source: &NativeChargedBuffer,
    target: &NativeChargedBuffer,
    params: &NativeChargedBuffer,
    lookup: &NativeChargedBuffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Syndocal resident packed group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: source.buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: target.buffer.as_entire_binding(),
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
    })
}
fn make_group_layer(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source: &NativeChargedBuffer,
    target: &NativeChargedBuffer,
    sampling: &GroupSampling,
    charges: (AllocationCharge, AllocationCharge),
    ids: &mut Vec<GpuAllocationId>,
) -> ComposedLayer {
    let (lookup, params) = group_sampling_buffers(device, sampling, charges, ids);
    let bind_group = packed_group_bind_group(device, layout, source, target, &params, &lookup);
    ComposedLayer {
        bind_group,
        _lookup: lookup,
        _params: params,
    }
}

fn group_sampling_buffers(
    device: &wgpu::Device,
    sampling: &GroupSampling,
    charges: (AllocationCharge, AllocationCharge),
    ids: &mut Vec<GpuAllocationId>,
) -> (NativeChargedBuffer, NativeChargedBuffer) {
    let lookup_bytes = sampling
        .lookup
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<_>>();
    let lookup = NativeChargedBuffer {
        buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Syndocal group sampling lookup"),
            contents: &lookup_bytes,
            usage: wgpu::BufferUsages::STORAGE,
        }),
        charge: charges.0,
    };
    let params = NativeChargedBuffer {
        buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Syndocal group sampling params"),
            contents: &sampling.params,
            usage: wgpu::BufferUsages::UNIFORM,
        }),
        charge: charges.1,
    };
    ids.extend([lookup.allocation_id(), params.allocation_id()]);
    (lookup, params)
}

/// A group sampling step whose source is a texture (an FX result): the layer
/// texture pipelines with the group step's exact lookup and params.
fn make_texture_group_layer(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source: &wgpu::TextureView,
    target: &NativeChargedBuffer,
    sampling: &GroupSampling,
    charges: (AllocationCharge, AllocationCharge),
    ids: &mut Vec<GpuAllocationId>,
) -> ComposedLayer {
    let (lookup, params) = group_sampling_buffers(device, sampling, charges, ids);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Syndocal resident group FX transform"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: target.buffer.as_entire_binding(),
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
    ComposedLayer {
        bind_group,
        _lookup: lookup,
        _params: params,
    }
}

/// Preserve the canonical nearest-neighbour sampler, including its original
/// arithmetic order. Rotation uploads O(width + height) factorized coordinates,
/// not a full-frame CPU lookup or frame copy. The old axis lookup stays exact.
fn composition_sampling(
    transform: &protocol::Transform2D,
    extent: (u32, u32),
    source: (u32, u32),
) -> (Vec<u32>, u32) {
    if transform.rotation_deg == 0.0 {
        return (axis_aligned_source_lookup(transform, extent, source), 0);
    }
    let mut lookup = Vec::with_capacity(extent.0 as usize + extent.1 as usize + 8);
    lookup.extend(
        (0..extent.0).map(|x| ((x as f32 + 0.5) / extent.0 as f32 - 0.5 - transform.x).to_bits()),
    );
    lookup.extend(
        (0..extent.1).map(|y| ((y as f32 + 0.5) / extent.1 as f32 - 0.5 - transform.y).to_bits()),
    );
    let angle = -transform.rotation_deg.to_radians();
    let left = transform.crop_left.clamp(0.0, 1.0);
    let top = transform.crop_top.clamp(0.0, 1.0);
    let right = (1.0 - transform.crop_right.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let bottom = (1.0 - transform.crop_bottom.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    lookup.extend(
        [
            angle.sin(),
            angle.cos(),
            transform.scale_x.max(0.001),
            transform.scale_y.max(0.001),
            left,
            top,
            right - left,
            bottom - top,
        ]
        .map(f32::to_bits),
    );
    (lookup, 1)
}
pub(crate) fn resident_composition_layer(plan: &CompositionLayerPlan) -> bool {
    plan.opacity.is_finite()
        && plan.color == protocol::VideoColorAdjust::default()
        && plan.fx == protocol::VideoFxAdjust::default()
        && [
            plan.transform.x,
            plan.transform.y,
            plan.transform.scale_x,
            plan.transform.scale_y,
            plan.transform.rotation_deg,
            plan.transform.crop_left,
            plan.transform.crop_top,
            plan.transform.crop_right,
            plan.transform.crop_bottom,
        ]
        .iter()
        .all(|v| v.is_finite())
}
fn group_sampling(
    transform: &protocol::Transform2D,
    extent: (u32, u32),
    opacity: f32,
    blend: &protocol::VideoBlendMode,
) -> GroupSampling {
    let (lookup, rotated) = composition_sampling(transform, extent, extent);
    let mut params = [0; 32];
    for (i, word) in [
        extent.0,
        extent.1,
        extent.0,
        extent.1,
        opacity.to_bits(),
        crate::gpu_compositor::blend_mode_index(blend),
        0,
        rotated,
    ]
    .into_iter()
    .enumerate()
    {
        params[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    GroupSampling { lookup, params }
}
fn prepare_groups(
    groups: &[NativeCompositionGroup<'_>],
    inputs: &[NativeCompositionLayer<'_>],
    layers: &mut [CompositionLayerEncoding],
    extent: (u32, u32),
    max_storage: u64,
) -> Result<Vec<GroupEncoding>, NativeGpuDecline> {
    let decline = NativeGpuDecline::UnsupportedComposition;
    let mut claimed = vec![false; inputs.len()];
    let mut result = Vec::new();
    for (group_index, entry) in groups.iter().enumerate() {
        let group = entry.group;
        if groups[..group_index]
            .iter()
            .any(|entry| entry.group.id == group.id)
            || group
                .layer_ids
                .iter()
                .enumerate()
                .any(|(i, id)| group.layer_ids[..i].contains(id))
        {
            return Err(decline);
        }
        if !group.enabled || group.bypassed {
            continue;
        }
        if entry
            .fx
            .as_ref()
            .is_some_and(|fx| !crate::isf_runtime::resident_group_stages(&fx.stages))
            || !group.opacity.is_finite()
            || !(0.0..=1.0).contains(&group.opacity)
        {
            return Err(decline);
        }
        let transform = crate::layer_group_composition::group_transform(group);
        let members = inputs
            .iter()
            .enumerate()
            .filter_map(|(i, input)| group.layer_ids.contains(&input.plan.layer_id).then_some(i))
            .collect::<Vec<_>>();
        if members.iter().any(|i| claimed[*i]) {
            return Err(decline);
        }
        for &i in &members {
            claimed[i] = true;
        }
        if members.is_empty() {
            continue;
        }
        let probe = CompositionLayerPlan {
            transform,
            ..inputs[members[0]].plan.clone()
        };
        if !resident_composition_layer(&probe) {
            return Err(decline);
        }
        // Match the canonical organizational group and exact singleton
        // shortcut; group FX always needs the isolated composite.
        if entry.fx.is_none()
            && group.opacity == 1.0
            && group.blend_mode == protocol::VideoBlendMode::Normal
            && transform == protocol::Transform2D::default()
        {
            continue;
        }
        if entry.fx.is_none()
            && members.len() == 1
            && transform == protocol::Transform2D::default()
            && inputs[members[0]].plan.blend_mode == protocol::VideoBlendMode::Normal
        {
            let opacity = (inputs[members[0]].plan.opacity * group.opacity).clamp(0.0, 1.0);
            layers[members[0]].params[16..20].copy_from_slice(&opacity.to_bits().to_le_bytes());
            layers[members[0]].params[20..24].copy_from_slice(
                &crate::gpu_compositor::blend_mode_index(&group.blend_mode).to_le_bytes(),
            );
            continue;
        }
        for &i in &members {
            layers[i].params[24..28].fill(0);
        }
        let transformed = (transform != protocol::Transform2D::default())
            .then(|| group_sampling(&transform, extent, 1.0, &protocol::VideoBlendMode::Normal));
        let outer = group_sampling(
            &protocol::Transform2D::default(),
            extent,
            group.opacity,
            &group.blend_mode,
        );
        if transformed
            .iter()
            .chain(std::iter::once(&outer))
            .any(|sampling| sampling.lookup.len() as u64 * 4 > max_storage)
        {
            return Err(NativeGpuDecline::UnsupportedExtent);
        }
        // Canonical group FX clocks: the latest member PTS and no duration.
        let fx = entry.fx.as_ref().map(|effect| {
            let pts_ms = members
                .iter()
                .map(|&i| layers[i].identity.pts_ms)
                .max()
                .unwrap_or(0);
            let clocks = FrozenChainClocks {
                frame_index: 0,
                time_seconds_bits: (pts_ms as f32 / 1000.0).to_bits(),
                time_delta_seconds_bits: 0.0f32.to_bits(),
                date_bits: effect.date.map(f32::to_bits),
            };
            GroupFxEncoding {
                effect: Arc::clone(effect),
                clocks,
                uniforms: crate::isf_runtime::resident_group_uniforms(
                    &effect.stages,
                    extent,
                    clocks,
                ),
            }
        });
        result.push(GroupEncoding {
            members,
            transform: transformed,
            outer,
            fx,
        });
    }
    Ok(result)
}

/// Existing allocations a plan reuses instead of charging new ones.
#[derive(Clone, Copy, Debug, Default)]
struct AllocationReuse {
    readback: Option<GpuAllocationId>,
    output: Option<GpuAllocationId>,
    cpu_snapshot: Option<GpuAllocationId>,
}

/// One exact charge per physical allocation, in the order the shared adapter
/// binds them: premapping target, its single readback (or an existing ring
/// readback alias), the owned packed CPU snapshot (or the recycled displaced
/// LastValid's charge), then each layer's lookup and parameter buffer.
fn composition_allocations(
    frame: NativeFrameId,
    output_bytes: u64,
    layers: &[CompositionLayerEncoding],
    groups: &[GroupEncoding],
    reuse: AllocationReuse,
) -> Result<NativeAllocationPlan, NativeGpuDecline> {
    let mut native_allocations = NativeAllocationPlan::new(frame);
    match reuse.output {
        Some(id) => native_allocations.add_existing(id),
        None => native_allocations.add_new(ChargeKind::CompositionBuffer, output_bytes)?,
    }
    match reuse.readback {
        Some(id) => native_allocations.add_existing(id),
        None => native_allocations.add_new(ChargeKind::SnapshotReadback, output_bytes)?,
    }
    match reuse.cpu_snapshot {
        Some(id) => native_allocations.add_existing(id),
        None => native_allocations.add_new(ChargeKind::CpuSnapshot, output_bytes)?,
    }
    for layer in layers {
        native_allocations.add_new(ChargeKind::CompositionBuffer, layer.lookup.len() as u64 * 4)?;
        native_allocations.add_new(ChargeKind::CompositionBuffer, LAYER_PARAMS_BYTES)?;
    }
    // Groups share one isolated target and one transform target (see encode).
    if !groups.is_empty() {
        native_allocations.add_new(ChargeKind::CompositionBuffer, output_bytes)?;
    }
    if groups.iter().any(GroupEncoding::needs_transform_target) {
        native_allocations.add_new(ChargeKind::CompositionBuffer, output_bytes)?;
    }
    // FX groups share one ping/pong pair; the input is unpacked into pong.
    if groups.iter().any(|group| group.fx.is_some()) {
        for _ in 0..2 {
            native_allocations.add_new(ChargeKind::IsfScratch, output_bytes)?;
        }
    }
    for group in groups {
        for sampling in group.transform.iter().chain(std::iter::once(&group.outer)) {
            native_allocations.add_new(
                ChargeKind::CompositionBuffer,
                sampling.lookup.len() as u64 * 4,
            )?;
            native_allocations.add_new(ChargeKind::CompositionBuffer, LAYER_PARAMS_BYTES)?;
        }
        for uniform in group.fx.iter().flat_map(|fx| &fx.uniforms) {
            native_allocations.add_new(ChargeKind::CompositionBuffer, uniform.len() as u64)?;
        }
    }
    Ok(native_allocations)
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
