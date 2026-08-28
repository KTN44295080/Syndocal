//! Concrete normal-output lease used by application media playback.
//!
//! A Rodio stream may only be opened in the router-admitted closure. The
//! lease, rather than an untracked `OutputStream`, owns the OS resource until
//! explicit retirement has received the router proof.

use crate::audio_output_router::{
    self, BridgeDiagnostic, NormalRouteStart, Route, RouterSlot, SlotNormalRetirementLease,
    SlotNormalRouteLease,
};
use rodio::{mixer::Mixer, OutputStream};
use std::sync::Arc;

pub(crate) struct PreparedNormalOutput {
    stream: OutputStream,
    mixer: Mixer,
    device_name: Option<String>,
    requested_device_name: Option<String>,
    sample_rate: u32,
    channels: u16,
}

/// Rodio has no separate stop result for an `OutputStream`: after all app
/// sinks are stopped, dropping this resource is its bounded close operation.
/// The router still owns the exact retirement receipt and therefore remains
/// the only path which can release the process-wide normal route.
fn retire_output(output: &mut PreparedNormalOutput) -> Result<(), BridgeDiagnostic> {
    output.stream.log_on_drop(false);
    Ok(())
}

pub(crate) type NormalOutputRetire = fn(&mut PreparedNormalOutput) -> Result<(), BridgeDiagnostic>;

pub(crate) struct NormalAudioOutput {
    lease: SlotNormalRouteLease<PreparedNormalOutput, NormalOutputRetire>,
}

/// A normal route which was not retired yet. It retains the exact concrete
/// resource and can only ask the Router for one fresh retirement permit.
/// Callers must keep this value for an explicit retry or surface its fault;
/// dropping a diagnostic alone never releases the route.
pub(crate) enum PendingNormalAudioOutputRetirement {
    /// Router retirement admission itself was unavailable; retain the
    /// original published lease so the same resource can be retried.
    Unadmitted(NormalAudioOutput),
    /// The resource's close callback failed or panicked; the Router issued an
    /// exact retry lease for the same handle.
    Retry(SlotNormalRetirementLease<PreparedNormalOutput, NormalOutputRetire>),
}

pub(crate) enum NormalAudioOutputOpenFailure {
    Fault {
        detail: String,
        pending_retirements: Vec<PendingNormalAudioOutputRetirement>,
    },
}

impl NormalAudioOutputOpenFailure {
    pub(crate) fn detail(&self) -> &str {
        match self {
            Self::Fault { detail, .. } => detail,
        }
    }

    pub(crate) fn into_pending_retirements(self) -> Vec<PendingNormalAudioOutputRetirement> {
        match self {
            Self::Fault {
                pending_retirements,
                ..
            } => pending_retirements,
        }
    }
}

pub(crate) enum NormalAudioOutputRetirement {
    Retired,
    Retry(PendingNormalAudioOutputRetirement),
}

impl NormalAudioOutput {
    /// Opens exactly one PROGRAM route after `RouterSlot` has removed its
    /// private Router from the mutex. This function must therefore be called
    /// only after the media-playback mutex was released.
    pub(crate) fn open(
        slot: &Arc<RouterSlot>,
        requested_device_name: Option<String>,
    ) -> Result<Self, NormalAudioOutputOpenFailure> {
        let start_name = requested_device_name.clone();
        let recover_name = requested_device_name;
        let factory = audio_output_router::normal_route_factory(
            move || open_output(start_name),
            move || open_output(recover_name),
            retire_output as NormalOutputRetire,
        );
        let task = slot
            .into_normal_publication(Route::Program, factory)
            .map_err(|error| NormalAudioOutputOpenFailure::Fault {
                detail: format!("normal PROGRAM admission rejected: {error:?}"),
                pending_retirements: Vec::new(),
            })?;
        let result = task.perform();
        let detail = format!(
            "normal PROGRAM open failed: {}: {}",
            result.snapshot().diagnostic.code,
            result.snapshot().diagnostic.message
        );
        let (lease, retry) = result.into_handles();
        match (lease, retry) {
            (Some(lease), None) => Ok(Self { lease }),
            (None, pending_retirement) => Err(NormalAudioOutputOpenFailure::Fault {
                detail,
                pending_retirements: pending_retirement
                    .map(PendingNormalAudioOutputRetirement::Retry)
                    .into_iter()
                    .collect(),
            }),
            // The sealed Router currently makes this unreachable, but retain
            // and explicitly retire *both* handles if a future implementation
            // violates that result invariant. Dropping either handle would
            // bypass the exact router retirement receipt.
            (Some(lease), Some(retry)) => {
                let mut pending_retirements =
                    vec![PendingNormalAudioOutputRetirement::Retry(retry)];
                let detail = match (Self { lease }).into_retirement() {
                    Ok(task) => match task.perform() {
                        NormalAudioOutputRetirement::Retired => {
                            "normal PROGRAM publication returned contradictory route handles"
                                .to_string()
                        }
                        NormalAudioOutputRetirement::Retry(pending) => {
                            pending_retirements.push(pending);
                            "normal PROGRAM publication returned contradictory route handles; retirement retry is required".to_string()
                        }
                    },
                    Err((output, error)) => {
                        pending_retirements
                            .push(PendingNormalAudioOutputRetirement::Unadmitted(output));
                        format!("normal PROGRAM publication returned contradictory route handles: {error:?}")
                    }
                };
                Err(NormalAudioOutputOpenFailure::Fault {
                    detail,
                    pending_retirements,
                })
            }
        }
    }

    pub(crate) fn mixer(&self) -> Mixer {
        self.lease.with_resource(|output| output.mixer.clone())
    }

    pub(crate) fn device_name(&self) -> Option<String> {
        self.lease
            .with_resource(|output| output.device_name.clone())
    }

    pub(crate) fn requested_device_name(&self) -> Option<String> {
        self.lease
            .with_resource(|output| output.requested_device_name.clone())
    }

    pub(crate) fn sample_rate(&self) -> u32 {
        self.lease.with_resource(|output| output.sample_rate)
    }

    pub(crate) fn channels(&self) -> u16 {
        self.lease.with_resource(|output| output.channels)
    }

    pub(crate) fn into_retirement(self) -> Result<NormalAudioOutputRetirementTask, (Self, String)> {
        self.lease
            .into_retirement()
            .map(|task| NormalAudioOutputRetirementTask { task })
            .map_err(|(lease, error)| {
                (
                    Self { lease },
                    format!("normal PROGRAM retirement admission rejected: {error:?}"),
                )
            })
    }
}

pub(crate) struct NormalAudioOutputRetirementTask {
    task: audio_output_router::SlotNormalRetirementTask<PreparedNormalOutput, NormalOutputRetire>,
}

impl NormalAudioOutputRetirementTask {
    /// Runs outside the media-playback mutex. A failure retains an opaque
    /// retry lease for the same stream; it never becomes a silent drop.
    pub(crate) fn perform(self) -> NormalAudioOutputRetirement {
        let result = self.task.perform();
        match result.into_retry() {
            Some(lease) => {
                NormalAudioOutputRetirement::Retry(PendingNormalAudioOutputRetirement::Retry(lease))
            }
            None => NormalAudioOutputRetirement::Retired,
        }
    }
}

impl PendingNormalAudioOutputRetirement {
    pub(crate) fn into_retirement(self) -> Result<NormalAudioOutputRetirementTask, (Self, String)> {
        match self {
            Self::Unadmitted(output) => output
                .into_retirement()
                .map_err(|(output, error)| (Self::Unadmitted(output), error)),
            Self::Retry(lease) => lease
                .into_retry()
                .map(|task| NormalAudioOutputRetirementTask { task })
                .map_err(|(lease, error)| {
                    (
                        Self::Retry(lease),
                        format!("normal PROGRAM retirement retry rejected: {error:?}"),
                    )
                }),
        }
    }
}

fn open_output(requested: Option<String>) -> NormalRouteStart<PreparedNormalOutput> {
    use rodio::cpal::traits::{DeviceTrait, HostTrait};

    let result: Result<PreparedNormalOutput, String> = (|| {
        let host = rodio::cpal::default_host();
        let device = match requested.as_deref() {
            Some(name) => host
                .output_devices()
                .map_err(|error| error.to_string())?
                .find(|device| device.name().ok().as_deref() == Some(name))
                .ok_or_else(|| format!("Audio output device '{name}' was not found"))?,
            None => host
                .default_output_device()
                .ok_or_else(|| "No default audio output device is available".to_owned())?,
        };
        let resolved_name = device.name().ok();
        let stream = rodio::OutputStreamBuilder::from_device(device)
            .and_then(|builder| builder.open_stream_or_fallback())
            .map_err(|error| error.to_string())?;
        let config = *stream.config();
        Ok(PreparedNormalOutput {
            mixer: stream.mixer().clone(),
            stream,
            device_name: resolved_name,
            requested_device_name: requested,
            sample_rate: config.sample_rate(),
            channels: config.channel_count(),
        })
    })();
    match result {
        Ok(output) => NormalRouteStart::Ready(output),
        Err(error) => NormalRouteStart::FailedBeforeResource(
            BridgeDiagnostic::checked("normal_output_open", error).unwrap_or_else(|_| {
                BridgeDiagnostic::checked(
                    "normal_output_open",
                    "Normal audio output could not be opened.",
                )
                .expect("constant diagnostic is valid")
            }),
        ),
    }
}
