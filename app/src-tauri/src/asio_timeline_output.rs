//! Typed, generation-guarded boundaries for Timeline Audio output.
//!
//! This module carries only the logical PROGRAM/CUE bus identity.  Physical
//! channels, devices, mixer ownership, and worker lifecycle remain outside the
//! boundary.  A source admitted here is permanently silent after its transport
//! generation changes, so a rotated transport cannot resume an old source.

use crate::asio_program_cue_render::TransportGeneration;
use protocol::TimelineAudioOutputBus;
use std::sync::Arc;
use std::time::Duration;

/// The complete logical identity of one Timeline Audio output admission.
///
/// The bus is part of the identity rather than a selector fallback.  Session
/// and transport generations are both non-zero admission facts and must remain
/// exact when a source is handed to a consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TimelineOutputIdentity {
    session_generation: u64,
    transport_generation: u64,
    bus: TimelineAudioOutputBus,
}

impl TimelineOutputIdentity {
    pub(crate) fn new(
        session_generation: u64,
        transport_generation: u64,
        bus: TimelineAudioOutputBus,
    ) -> Result<Self, TimelineOutputSelectorError> {
        let identity = Self {
            session_generation,
            transport_generation,
            bus,
        };
        identity.validate()?;
        Ok(identity)
    }

    #[cfg(test)]
    pub(crate) fn session_generation(self) -> u64 {
        self.session_generation
    }

    #[cfg(test)]
    pub(crate) fn transport_generation(self) -> u64 {
        self.transport_generation
    }

    pub(crate) fn bus(self) -> TimelineAudioOutputBus {
        self.bus
    }

    fn validate(self) -> Result<(), TimelineOutputSelectorError> {
        if self.session_generation == 0 {
            return Err(TimelineOutputSelectorError::InvalidSessionGeneration);
        }
        if self.transport_generation == 0 {
            return Err(TimelineOutputSelectorError::InvalidTransportGeneration);
        }
        Ok(())
    }

    fn with_transport_generation(self, transport_generation: u64) -> Self {
        Self {
            transport_generation,
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimelineOutputSelectorError {
    InvalidSessionGeneration,
    InvalidTransportGeneration,
    IdentityMismatch {
        expected: TimelineOutputIdentity,
        actual: TimelineOutputIdentity,
    },
}

impl std::fmt::Display for TimelineOutputSelectorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSessionGeneration => {
                formatter.write_str("ASIO Timeline output session generation must be non-zero")
            }
            Self::InvalidTransportGeneration => {
                formatter.write_str("ASIO Timeline output transport generation must be non-zero")
            }
            Self::IdentityMismatch { expected, actual } => write!(
                formatter,
                "ASIO Timeline output identity mismatch: expected {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl std::error::Error for TimelineOutputSelectorError {}

/// Selects one logical PROGRAM/CUE bus for one admitted session/transport.
///
/// The selector does not own or expose a physical mixer.  It only revalidates
/// the complete logical identity before wrapping a source with the transport
/// generation fence.
#[derive(Clone)]
pub(crate) struct AsioTimelineOutputSelector {
    identity: TimelineOutputIdentity,
    transport: Arc<TransportGeneration>,
}

impl AsioTimelineOutputSelector {
    /// Admit an identity against the current transport generation.
    pub(crate) fn from_identity(
        identity: TimelineOutputIdentity,
        transport: Arc<TransportGeneration>,
    ) -> Result<Self, TimelineOutputSelectorError> {
        identity.validate()?;
        let current_transport_generation = transport.current();
        if current_transport_generation == 0 {
            return Err(TimelineOutputSelectorError::InvalidTransportGeneration);
        }
        if current_transport_generation != identity.transport_generation {
            return Err(TimelineOutputSelectorError::IdentityMismatch {
                expected: identity,
                actual: identity.with_transport_generation(current_transport_generation),
            });
        }
        Ok(Self {
            identity,
            transport,
        })
    }

    #[cfg(test)]
    pub(crate) fn identity_for(&self, bus: TimelineAudioOutputBus) -> TimelineOutputIdentity {
        TimelineOutputIdentity {
            bus,
            ..self.identity
        }
    }

    /// Wrap a source only when its complete session/transport/bus identity is
    /// exactly the identity admitted by this selector.
    pub(crate) fn source_for<S>(
        &self,
        identity: TimelineOutputIdentity,
        source: S,
    ) -> Result<GenerationGuardedSource<S>, TimelineOutputSelectorError>
    where
        S: rodio::Source,
    {
        self.validate_identity(identity)?;
        GenerationGuardedSource::new(source, Arc::clone(&self.transport), identity)
    }

    fn validate_identity(
        &self,
        identity: TimelineOutputIdentity,
    ) -> Result<(), TimelineOutputSelectorError> {
        identity.validate()?;
        if identity != self.identity {
            return Err(TimelineOutputSelectorError::IdentityMismatch {
                expected: self.identity,
                actual: identity,
            });
        }

        let current_transport_generation = self.transport.current();
        if current_transport_generation == 0 {
            return Err(TimelineOutputSelectorError::InvalidTransportGeneration);
        }
        if current_transport_generation != self.identity.transport_generation {
            return Err(TimelineOutputSelectorError::IdentityMismatch {
                expected: self.identity,
                actual: self
                    .identity
                    .with_transport_generation(current_transport_generation),
            });
        }
        Ok(())
    }
}

/// A `rodio::Source` that permanently ends when its transport generation is
/// no longer current.
///
/// The test build stores the full logical identity for observability, while the
/// transport generation is the realtime admission fence.  A stale source is
/// rejected before its first sample and cannot resume after a rotation.
pub(crate) struct GenerationGuardedSource<S> {
    inner: S,
    transport: Arc<TransportGeneration>,
    #[cfg(test)]
    identity: TimelineOutputIdentity,
    expected_transport_generation: u64,
    ended: bool,
}

impl<S> GenerationGuardedSource<S>
where
    S: rodio::Source,
{
    pub(crate) fn new(
        inner: S,
        transport: Arc<TransportGeneration>,
        identity: TimelineOutputIdentity,
    ) -> Result<Self, TimelineOutputSelectorError> {
        identity.validate()?;
        Ok(Self {
            inner,
            transport,
            expected_transport_generation: identity.transport_generation,
            #[cfg(test)]
            identity,
            ended: false,
        })
    }

    #[cfg(test)]
    pub(crate) fn identity(&self) -> TimelineOutputIdentity {
        self.identity
    }

    #[cfg(test)]
    pub(crate) fn expected_transport_generation(&self) -> u64 {
        self.expected_transport_generation
    }

    fn generation_matches(&self) -> bool {
        self.transport.current() == self.expected_transport_generation
    }

    fn end_if_stale(&mut self) -> bool {
        if !self.generation_matches() {
            self.ended = true;
        }
        self.ended
    }

    fn stale_seek_error() -> rodio::source::SeekError {
        rodio::source::SeekError::NotSupported {
            underlying_source: "GenerationGuardedSource (stale transport generation)",
        }
    }
}

impl<S> Iterator for GenerationGuardedSource<S>
where
    S: rodio::Source,
{
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.end_if_stale() {
            return None;
        }

        let sample = self.inner.next();
        if self.end_if_stale() {
            return None;
        }

        if sample.is_none() {
            self.ended = true;
        }
        sample
    }
}

impl<S> rodio::Source for GenerationGuardedSource<S>
where
    S: rodio::Source,
{
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, position: Duration) -> Result<(), rodio::source::SeekError> {
        if self.end_if_stale() {
            return Err(Self::stale_seek_error());
        }

        let result = self.inner.try_seek(position);
        if self.end_if_stale() {
            return Err(Self::stale_seek_error());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::{buffer::SamplesBuffer, Source};

    fn transport(initial: u64) -> Arc<TransportGeneration> {
        Arc::new(TransportGeneration::new(initial).unwrap())
    }

    fn identity(
        session_generation: u64,
        transport_generation: u64,
        bus: TimelineAudioOutputBus,
    ) -> TimelineOutputIdentity {
        TimelineOutputIdentity::new(session_generation, transport_generation, bus).unwrap()
    }

    fn samples(values: &[f32]) -> SamplesBuffer {
        SamplesBuffer::new(1, 48_000, values.to_vec())
    }

    #[test]
    fn stale_before_first_sample_ends_permanently() {
        let transport = transport(1);
        let source_identity = identity(11, 1, TimelineAudioOutputBus::Program);
        let mut source = GenerationGuardedSource::new(
            samples(&[0.25, 0.5]),
            Arc::clone(&transport),
            source_identity,
        )
        .unwrap();

        transport.rotate().unwrap();
        assert_eq!(source.next(), None);
        assert_eq!(source.next(), None);
        assert_eq!(source.identity(), source_identity);
        assert_eq!(source.expected_transport_generation(), 1);
    }

    #[test]
    fn rotation_after_samples_stops_without_resuming() {
        let transport = transport(1);
        let mut source = GenerationGuardedSource::new(
            samples(&[0.25, 0.5, 0.75]),
            Arc::clone(&transport),
            identity(11, 1, TimelineAudioOutputBus::Cue),
        )
        .unwrap();

        assert_eq!(source.next(), Some(0.25));
        transport.rotate().unwrap();
        assert_eq!(source.next(), None);
        assert_eq!(source.next(), None);
    }

    #[test]
    fn new_generation_source_can_start_after_rotation() {
        let transport = transport(1);
        let first_identity = identity(11, 1, TimelineAudioOutputBus::Program);
        let mut old_source =
            GenerationGuardedSource::new(samples(&[0.25]), Arc::clone(&transport), first_identity)
                .unwrap();
        transport.rotate().unwrap();
        assert_eq!(old_source.next(), None);

        let second_identity = identity(11, 2, TimelineAudioOutputBus::Program);
        let mut new_source =
            GenerationGuardedSource::new(samples(&[0.9]), Arc::clone(&transport), second_identity)
                .unwrap();
        assert_eq!(new_source.next(), Some(0.9));
    }

    #[test]
    fn source_metadata_is_delegated_exactly() {
        let transport = transport(1);
        let source = SamplesBuffer::new(2, 44_100, vec![0.25, 0.5, 0.75, 1.0]);
        let expected_span = source.current_span_len();
        let expected_channels = source.channels();
        let expected_rate = source.sample_rate();
        let expected_duration = source.total_duration();
        let guarded = GenerationGuardedSource::new(
            source,
            transport,
            identity(11, 1, TimelineAudioOutputBus::Program),
        )
        .unwrap();

        assert_eq!(guarded.current_span_len(), expected_span);
        assert_eq!(guarded.channels(), expected_channels);
        assert_eq!(guarded.sample_rate(), expected_rate);
        assert_eq!(guarded.total_duration(), expected_duration);
    }

    #[test]
    fn selector_requires_exact_session_transport_and_bus() {
        let transport = transport(1);
        let program_identity = identity(41, 1, TimelineAudioOutputBus::Program);
        let selector =
            AsioTimelineOutputSelector::from_identity(program_identity, Arc::clone(&transport))
                .unwrap();
        let cue_identity = selector.identity_for(TimelineAudioOutputBus::Cue);
        assert_eq!(program_identity.bus(), TimelineAudioOutputBus::Program);
        assert_eq!(cue_identity.bus(), TimelineAudioOutputBus::Cue);
        assert_eq!(cue_identity.session_generation(), 41);
        assert_eq!(cue_identity.transport_generation(), 1);

        let mut program_source = selector
            .source_for(program_identity, samples(&[0.1]))
            .unwrap();
        assert_eq!(program_source.next(), Some(0.1));

        let wrong_bus = identity(41, 1, TimelineAudioOutputBus::Cue);
        assert!(matches!(
            selector.source_for(wrong_bus, samples(&[0.2])),
            Err(TimelineOutputSelectorError::IdentityMismatch {
                expected,
                actual
            }) if expected == program_identity && actual == wrong_bus
        ));

        let wrong_session = identity(42, 1, TimelineAudioOutputBus::Program);
        assert!(matches!(
            selector.source_for(wrong_session, samples(&[0.3])),
            Err(TimelineOutputSelectorError::IdentityMismatch {
                expected,
                actual
            }) if expected == program_identity && actual == wrong_session
        ));

        let wrong_transport = identity(41, 2, TimelineAudioOutputBus::Program);
        assert!(matches!(
            selector.source_for(wrong_transport, samples(&[0.4])),
            Err(TimelineOutputSelectorError::IdentityMismatch {
                expected,
                actual
            }) if expected == program_identity && actual == wrong_transport
        ));
    }

    #[test]
    fn stale_selector_and_session_rollover_collision_fail_closed() {
        let old_transport = transport(1);
        let old_identity = identity(41, 1, TimelineAudioOutputBus::Program);
        let old_selector =
            AsioTimelineOutputSelector::from_identity(old_identity, Arc::clone(&old_transport))
                .unwrap();

        old_transport.rotate().unwrap();
        assert!(matches!(
            old_selector.source_for(old_identity, samples(&[0.1])),
            Err(TimelineOutputSelectorError::IdentityMismatch { .. })
        ));

        // A new session may legitimately restart its transport counter at 1,
        // but the old selector's complete identity must not admit it.
        let new_transport = transport(1);
        let new_identity = identity(42, 1, TimelineAudioOutputBus::Program);
        let new_selector =
            AsioTimelineOutputSelector::from_identity(new_identity, new_transport).unwrap();
        assert!(matches!(
            old_selector.source_for(new_identity, samples(&[0.2])),
            Err(TimelineOutputSelectorError::IdentityMismatch { .. })
        ));
        assert!(new_selector
            .source_for(new_identity, samples(&[0.3]))
            .is_ok());
    }

    #[test]
    fn zero_identity_and_expected_generation_are_rejected() {
        assert!(matches!(
            TimelineOutputIdentity::new(0, 1, TimelineAudioOutputBus::Program),
            Err(TimelineOutputSelectorError::InvalidSessionGeneration)
        ));
        assert!(matches!(
            TimelineOutputIdentity::new(1, 0, TimelineAudioOutputBus::Cue),
            Err(TimelineOutputSelectorError::InvalidTransportGeneration)
        ));

        let transport = transport(1);
        let identity = identity(1, 1, TimelineAudioOutputBus::Program);
        transport.rotate().unwrap();
        assert!(matches!(
            AsioTimelineOutputSelector::from_identity(identity, Arc::clone(&transport)),
            Err(TimelineOutputSelectorError::IdentityMismatch { .. })
        ));
    }
}
