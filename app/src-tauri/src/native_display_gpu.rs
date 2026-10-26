//! Unwired I1 bootstrap policy for native Display.
//!
//! The adapter owns the window/surface, shared GPU owner/context, runtime,
//! history, cache fence and the one Stop token. This module creates no window,
//! device, thread or physical present, and is not registered in main.rs.
//! A prepared bundle is NOT presentation authority: the C1 adapter must still
//! use the existing native permit, full authority/Stop fence and present path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LogicalExtent {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PhysicalExtent {
    pub width: u32,
    pub height: u32,
}

impl PhysicalExtent {
    fn is_zero(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BootstrapGeometry {
    Hidden {
        declared_physical: PhysicalExtent,
        // Diagnostic metadata only; never used as the render extent.
        observed_logical: LogicalExtent,
    },
    Visible {
        observed_physical: PhysicalExtent,
    },
}

impl BootstrapGeometry {
    fn render_extent(self) -> PhysicalExtent {
        match self {
            Self::Hidden {
                declared_physical, ..
            } => declared_physical,
            Self::Visible { observed_physical } => observed_physical,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BootstrapCheckpoint {
    Initial,
    BeforePrepare,
    BeforeShow,
    BeforeConfigure,
    BeforeHandoff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BootstrapOperation {
    InitializeUnconfigured,
    Prepare,
    ShowActivate,
    ObservePhysical,
    Configure,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BootstrapRejection<E> {
    Cancelled {
        at: BootstrapCheckpoint,
    },
    Authority {
        at: BootstrapCheckpoint,
        error: E,
    },
    Backend {
        operation: BootstrapOperation,
        error: E,
    },
    Suspended {
        observed: PhysicalExtent,
    },
    GeometryMismatch {
        expected: PhysicalExtent,
        observed: PhysicalExtent,
    },
    ReadyGeometryMismatch {
        expected: PhysicalExtent,
        actual: PhysicalExtent,
    },
}

/// Owned, movable adapter bundle; no owner may borrow the creation thread.
/// Each fallible method MUST retain partially-created/in-flight resources in
/// self on error. initialize_unconfigured must not configure/acquire/present.
/// prepare_once must return a completed, cache-committed Ready or an error;
/// GPU completion/cache policy itself belongs to the video-side I/G/P modules.
pub(crate) trait BootstrapBackend: Send + 'static {
    type Ready: Send + 'static;
    type Error: Send + 'static;

    fn is_stopped(&self) -> bool;
    fn check_authority(&mut self, at: BootstrapCheckpoint) -> Result<(), Self::Error>;
    fn initialize_unconfigured(&mut self) -> Result<(), Self::Error>;
    fn prepare_once(&mut self, extent: PhysicalExtent) -> Result<Self::Ready, Self::Error>;
    fn ready_extent(&self, ready: &Self::Ready) -> PhysicalExtent;
    /// Production binding must perform existing show/activation and its
    /// bounded exact-size convergence check, never an unbounded retry.
    /// The real adapter must recheck live Stop and authority immediately before
    /// each side effect under the existing native gate. An outer policy guard
    /// does not provide atomic exclusion against concurrent Stop/revocation.
    fn show_and_activate(
        &mut self,
        expected: PhysicalExtent,
    ) -> Result<PhysicalExtent, Self::Error>;
    fn observe_physical_extent(&mut self) -> Result<PhysicalExtent, Self::Error>;
    /// The real adapter must recheck live Stop and authority at the configure
    /// side-effect boundary under the existing native gate; the outer guard
    /// alone is not atomic exclusion against concurrent Stop/revocation.
    fn configure(&mut self, verified: PhysicalExtent) -> Result<(), Self::Error>;
}

/// Affine preparation result, deliberately neither Clone nor Copy.
/// Move this whole value to the next owner. Splitting it transfers BOTH the
/// backend bundle and Ready; it does not publish a worker or authorize present.
#[must_use]
pub(crate) struct PreparedBootstrap<B: BootstrapBackend> {
    ready: B::Ready,
    backend: B,
    extent: PhysicalExtent,
    geometry: BootstrapGeometry,
}

impl<B: BootstrapBackend> PreparedBootstrap<B> {
    pub(crate) fn extent(&self) -> PhysicalExtent {
        self.extent
    }

    pub(crate) fn geometry(&self) -> BootstrapGeometry {
        self.geometry
    }

    pub(crate) fn into_parts(self) -> (B::Ready, B) {
        (self.ready, self.backend)
    }
}

/// Failed/suspended preparation still owns all created resources and any Ready.
/// The caller must transfer these to its existing cleanup/retirement owner.
/// Dropping this metadata is not a cleanup ACK, GPU drain, or successful Stop.
#[must_use]
pub(crate) struct RetainedBootstrapFailure<B: BootstrapBackend> {
    reason: BootstrapRejection<B::Error>,
    ready: Option<B::Ready>,
    backend: B,
}

impl<B: BootstrapBackend> RetainedBootstrapFailure<B> {
    pub(crate) fn reason(&self) -> &BootstrapRejection<B::Error> {
        &self.reason
    }

    pub(crate) fn has_prepared_frame(&self) -> bool {
        self.ready.is_some()
    }

    pub(crate) fn into_parts(self) -> (BootstrapRejection<B::Error>, Option<B::Ready>, B) {
        (self.reason, self.ready, self.backend)
    }
}

fn guard<B: BootstrapBackend>(
    backend: &mut B,
    at: BootstrapCheckpoint,
) -> Result<(), BootstrapRejection<B::Error>> {
    if backend.is_stopped() {
        return Err(BootstrapRejection::Cancelled { at });
    }
    let authority = backend.check_authority(at);
    // Authority validation can take short locks. Do not let Stop cross it.
    if backend.is_stopped() {
        return Err(BootstrapRejection::Cancelled { at });
    }
    authority.map_err(|error| BootstrapRejection::Authority { at, error })
}

fn classify_backend_error<B: BootstrapBackend>(
    backend: &B,
    operation: BootstrapOperation,
    at: BootstrapCheckpoint,
    error: B::Error,
) -> BootstrapRejection<B::Error> {
    // Use the next barrier that would observe Stop after a successful call.
    if backend.is_stopped() {
        BootstrapRejection::Cancelled { at }
    } else {
        BootstrapRejection::Backend { operation, error }
    }
}

fn verify_extent<E>(
    expected: PhysicalExtent,
    observed: PhysicalExtent,
) -> Result<(), BootstrapRejection<E>> {
    if observed.is_zero() {
        Err(BootstrapRejection::Suspended { observed })
    } else if observed != expected {
        Err(BootstrapRejection::GeometryMismatch { expected, observed })
    } else {
        Ok(())
    }
}

/// One bounded bootstrap attempt. No loop, re-decode or second FX execution.
/// Hidden logical geometry cannot become the render size; declared physical
/// geometry is required. Every error returns the same owned backend bundle.
pub(crate) fn prepare_native_display<B: BootstrapBackend>(
    mut backend: B,
    geometry: BootstrapGeometry,
) -> Result<PreparedBootstrap<B>, RetainedBootstrapFailure<B>> {
    let extent = geometry.render_extent();
    let mut ready = None;
    let attempt = (|| {
        guard(&mut backend, BootstrapCheckpoint::Initial)?;
        if extent.is_zero() {
            return Err(BootstrapRejection::Suspended { observed: extent });
        }
        backend.initialize_unconfigured().map_err(|error| {
            classify_backend_error(
                &backend,
                BootstrapOperation::InitializeUnconfigured,
                BootstrapCheckpoint::BeforePrepare,
                error,
            )
        })?;
        guard(&mut backend, BootstrapCheckpoint::BeforePrepare)?;
        ready = Some(backend.prepare_once(extent).map_err(|error| {
            classify_backend_error(
                &backend,
                BootstrapOperation::Prepare,
                BootstrapCheckpoint::BeforeShow,
                error,
            )
        })?);
        guard(&mut backend, BootstrapCheckpoint::BeforeShow)?;
        let actual = backend.ready_extent(ready.as_ref().expect("Ready was just stored"));
        if actual != extent {
            return Err(BootstrapRejection::ReadyGeometryMismatch {
                expected: extent,
                actual,
            });
        }
        let observed = match geometry {
            BootstrapGeometry::Hidden { .. } => {
                backend.show_and_activate(extent).map_err(|error| {
                    classify_backend_error(
                        &backend,
                        BootstrapOperation::ShowActivate,
                        BootstrapCheckpoint::BeforeConfigure,
                        error,
                    )
                })?
            }
            BootstrapGeometry::Visible { .. } => {
                backend.observe_physical_extent().map_err(|error| {
                    classify_backend_error(
                        &backend,
                        BootstrapOperation::ObservePhysical,
                        BootstrapCheckpoint::BeforeConfigure,
                        error,
                    )
                })?
            }
        };
        guard(&mut backend, BootstrapCheckpoint::BeforeConfigure)?;
        verify_extent(extent, observed)?;
        backend.configure(observed).map_err(|error| {
            classify_backend_error(
                &backend,
                BootstrapOperation::Configure,
                BootstrapCheckpoint::BeforeHandoff,
                error,
            )
        })?;
        guard(&mut backend, BootstrapCheckpoint::BeforeHandoff)?;
        // Detect minimization/resize during configuration before offering the
        // bundle to a new owner. The later physical present still revalidates.
        let observed = backend.observe_physical_extent().map_err(|error| {
            classify_backend_error(
                &backend,
                BootstrapOperation::ObservePhysical,
                BootstrapCheckpoint::BeforeHandoff,
                error,
            )
        })?;
        guard(&mut backend, BootstrapCheckpoint::BeforeHandoff)?;
        verify_extent(extent, observed)
    })();
    match attempt {
        Ok(()) => Ok(PreparedBootstrap {
            backend,
            ready: ready.expect("successful bootstrap prepared exactly once"),
            extent,
            geometry,
        }),
        Err(reason) => Err(RetainedBootstrapFailure {
            reason,
            backend,
            ready,
        }),
    }
}

#[cfg(test)]
#[path = "native_display_gpu_tests.rs"]
mod tests;
