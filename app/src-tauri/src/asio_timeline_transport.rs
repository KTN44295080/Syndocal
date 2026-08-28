//! Control-plane observation of Timeline semantic changes for show-ASIO.
//!
//! A Timeline position is sampled frequently by the ordinary 44 Hz update
//! path, but that position is not a transport identity.  This module only
//! compares the semantic fields that identify the published Timeline audio
//! image.  A caller plans a possible generation rotation, performs the
//! fallible runtime rotation, and commits the plan only after the runtime
//! returns the exact generation requested by the plan.
//!
//! The types in this module are deliberately made only from copied primitive
//! values.  They are control-plane values: no callback path should construct,
//! lock, or retain this observer.

use std::fmt;

use crate::asio_output_runtime::AsioOutputRuntime;
#[cfg(test)]
use crate::{
    asio_program_cue::{PreflightSolo, PreflightTarget},
    asio_program_cue_render::preflight_now_ms,
};

/// The copied semantic identity of one published Timeline audio image.
///
/// `transport_revision` and `publication_generation` are intentionally not
/// required to be non-zero.  The engine's initial transport revision can be
/// zero, and the observer must compare the published value exactly rather
/// than silently normalizing it.  The ASIO session and runtime transport
/// generations supplied to [`AsioTimelineTransportObserver::plan`] are the
/// separate non-zero admission facts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TimelineSemanticStamp {
    pub(crate) source_projection_authority_epoch: u64,
    pub(crate) source_projection_authority_generation: u64,
    pub(crate) publication_generation: u64,
    pub(crate) transport_revision: u64,
    pub(crate) playing: bool,
}

impl TimelineSemanticStamp {
    /// Copy the semantic fields needed by the observer from a Timeline
    /// snapshot without retaining the snapshot itself.
    pub(crate) const fn new(
        source_projection_authority_epoch: u64,
        source_projection_authority_generation: u64,
        publication_generation: u64,
        transport_revision: u64,
        playing: bool,
    ) -> Self {
        Self {
            source_projection_authority_epoch,
            source_projection_authority_generation,
            publication_generation,
            transport_revision,
            playing,
        }
    }
}

/// One observer state that was successfully established or committed.
///
/// This is kept separate from the input stamp so that a plan can carry both
/// the state it was based on and the state it would establish after a caller
/// confirms the runtime operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AsioTimelineTransportObservation {
    pub(crate) session_generation: u64,
    pub(crate) transport_generation: u64,
    pub(crate) semantic: TimelineSemanticStamp,
}

/// The reason a plan was produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AsioTimelineTransportPlanKind {
    /// No observer baseline existed, or a new ASIO session was seen.
    EstablishBaseline,
    /// The exact session/runtime/semantic identity was already committed.
    Noop,
    /// The runtime generation changed outside this observer; adopt it without
    /// asking the runtime to rotate again.
    RebaseExternalGeneration,
    /// A semantic Timeline change occurred at the same session/runtime
    /// generation, so the caller must rotate exactly once.
    Rotate,
}

/// A two-phase observer plan.
///
/// Planning is read-only.  The `before` value is an optimistic state token;
/// commit rejects the plan if another observation has already changed the
/// observer.  For [`AsioTimelineTransportPlanKind::Rotate`], `after` contains
/// the checked next generation.  For the other kinds it contains the current
/// runtime generation and commit is only a baseline/no-op update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AsioTimelineTransportPlan {
    kind: AsioTimelineTransportPlanKind,
    before: Option<AsioTimelineTransportObservation>,
    after: AsioTimelineTransportObservation,
}

impl AsioTimelineTransportPlan {
    #[cfg(test)]
    pub(crate) const fn kind(self) -> AsioTimelineTransportPlanKind {
        self.kind
    }

    pub(crate) const fn requires_rotation(self) -> bool {
        matches!(self.kind, AsioTimelineTransportPlanKind::Rotate)
    }

    /// The generation that must be confirmed to commit this plan.  For a
    /// rotation this is the returned value from runtime rotation; for a
    /// baseline/rebase/no-op it is the current runtime generation. A rebase may
    /// also adopt a newly observed semantic stamp, but never asks the runtime to
    /// rotate again.
    #[cfg(test)]
    pub(crate) const fn confirmed_transport_generation(self) -> u64 {
        self.after.transport_generation
    }
}

/// Fail-closed errors for invalid input, exhausted generations, or stale
/// two-phase commits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AsioTimelineTransportError {
    InvalidSessionGeneration,
    InvalidTransportGeneration,
    GenerationExhausted,
    InvalidConfirmedGeneration,
    ConfirmedGenerationMismatch {
        expected: u64,
        actual: u64,
    },
    StalePlan {
        expected: Option<AsioTimelineTransportObservation>,
        actual: Option<AsioTimelineTransportObservation>,
    },
}

impl fmt::Display for AsioTimelineTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSessionGeneration => {
                formatter.write_str("ASIO Timeline session generation must be non-zero")
            }
            Self::InvalidTransportGeneration => {
                formatter.write_str("ASIO Timeline runtime transport generation must be non-zero")
            }
            Self::GenerationExhausted => {
                formatter.write_str("ASIO Timeline transport generation is exhausted")
            }
            Self::InvalidConfirmedGeneration => {
                formatter.write_str("ASIO Timeline confirmed transport generation must be non-zero")
            }
            Self::ConfirmedGenerationMismatch { expected, actual } => write!(
                formatter,
                "ASIO Timeline transport plan expected generation {expected}, got {actual}"
            ),
            Self::StalePlan { expected, actual } => write!(
                formatter,
                "ASIO Timeline transport plan is stale: expected observer state {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl std::error::Error for AsioTimelineTransportError {}

/// Deterministic control-plane observer for Timeline-driven ASIO transport
/// generation barriers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AsioTimelineTransportObserver {
    committed: Option<AsioTimelineTransportObservation>,
}

impl AsioTimelineTransportObserver {
    pub(crate) const fn new() -> Self {
        Self { committed: None }
    }

    /// Compare one semantic Timeline stamp with the committed state.
    ///
    /// This method never mutates the observer.  In particular, a semantic
    /// change produces a rotation plan but leaves the old state in place until
    /// [`Self::commit`] confirms the exact next generation returned by the
    /// runtime.  A failed runtime rotation can therefore retry the same plan
    /// without losing the semantic change.
    pub(crate) fn plan(
        &self,
        session_generation: u64,
        current_transport_generation: u64,
        semantic: TimelineSemanticStamp,
    ) -> Result<AsioTimelineTransportPlan, AsioTimelineTransportError> {
        if session_generation == 0 {
            return Err(AsioTimelineTransportError::InvalidSessionGeneration);
        }
        if current_transport_generation == 0 {
            return Err(AsioTimelineTransportError::InvalidTransportGeneration);
        }

        let current = AsioTimelineTransportObservation {
            session_generation,
            transport_generation: current_transport_generation,
            semantic,
        };

        let (kind, before, after) = match self.committed {
            None => (
                AsioTimelineTransportPlanKind::EstablishBaseline,
                None,
                current,
            ),
            Some(previous) if previous.session_generation != session_generation => (
                AsioTimelineTransportPlanKind::EstablishBaseline,
                Some(previous),
                current,
            ),
            Some(previous) if previous.transport_generation != current_transport_generation => (
                AsioTimelineTransportPlanKind::RebaseExternalGeneration,
                Some(previous),
                current,
            ),
            Some(previous) if previous.semantic == semantic => {
                (AsioTimelineTransportPlanKind::Noop, Some(previous), current)
            }
            Some(previous) => {
                let next_transport_generation = current_transport_generation
                    .checked_add(1)
                    .ok_or(AsioTimelineTransportError::GenerationExhausted)?;
                (
                    AsioTimelineTransportPlanKind::Rotate,
                    Some(previous),
                    AsioTimelineTransportObservation {
                        transport_generation: next_transport_generation,
                        ..current
                    },
                )
            }
        };

        Ok(AsioTimelineTransportPlan {
            kind,
            before,
            after,
        })
    }

    /// Commit a plan only after the caller has confirmed the exact target
    /// generation returned by the runtime.  Any rejected commit leaves the
    /// observer byte-for-byte unchanged.
    pub(crate) fn commit(
        &mut self,
        plan: AsioTimelineTransportPlan,
        confirmed_transport_generation: u64,
    ) -> Result<(), AsioTimelineTransportError> {
        if confirmed_transport_generation == 0 {
            return Err(AsioTimelineTransportError::InvalidConfirmedGeneration);
        }
        if confirmed_transport_generation != plan.after.transport_generation {
            return Err(AsioTimelineTransportError::ConfirmedGenerationMismatch {
                expected: plan.after.transport_generation,
                actual: confirmed_transport_generation,
            });
        }
        if self.committed != plan.before {
            return Err(AsioTimelineTransportError::StalePlan {
                expected: plan.before,
                actual: self.committed,
            });
        }

        self.committed = Some(plan.after);
        Ok(())
    }
}

/// Synchronize one already-active ASIO runtime with a copied Timeline semantic
/// stamp. The caller owns the runtime mutex for this complete transaction, so
/// an operator preflight command cannot interleave between the generation
/// observation, optional rotation, observer commit, and live-playback gate.
///
/// A runtime rotation is committed to the observer only after the runtime
/// returns the exact planned generation. Any failure leaves an uncommitted
/// semantic change retryable on the next worker tick. If rotation succeeded but
/// the later live-playback gate fails, the observer remains committed to that
/// real runtime generation and the gate itself is retried on the next tick.
pub(crate) fn synchronize_active_runtime(
    observer: &mut AsioTimelineTransportObserver,
    runtime: &mut AsioOutputRuntime,
    semantic: TimelineSemanticStamp,
) -> Result<(), String> {
    let observed = runtime.current_preflight_identity()?;
    let plan = observer
        .plan(
            observed.session_generation(),
            observed.transport_generation(),
            semantic,
        )
        .map_err(|error| error.to_string())?;
    let confirmed_generation = if plan.requires_rotation() {
        runtime.rotate_transport_generation()?
    } else {
        observed.transport_generation()
    };
    observer
        .commit(plan, confirmed_generation)
        .map_err(|error| error.to_string())?;

    let current = runtime.current_preflight_identity()?;
    if semantic.playing {
        runtime.admit_timeline_live_playback_for_identity(current)?;
    } else {
        runtime.set_preflight_live_playback_for_identity(current, false)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp() -> TimelineSemanticStamp {
        TimelineSemanticStamp::new(3, 5, 7, 11, true)
    }

    fn commit_target(
        observer: &mut AsioTimelineTransportObserver,
        plan: AsioTimelineTransportPlan,
    ) {
        observer
            .commit(plan, plan.confirmed_transport_generation())
            .unwrap();
    }

    fn established_observer() -> AsioTimelineTransportObserver {
        let mut observer = AsioTimelineTransportObserver::new();
        let plan = observer.plan(41, 1, stamp()).unwrap();
        assert_eq!(
            plan.kind(),
            AsioTimelineTransportPlanKind::EstablishBaseline
        );
        assert!(!plan.requires_rotation());
        commit_target(&mut observer, plan);
        observer
    }

    #[test]
    fn ordinary_44hz_position_ticks_are_not_an_input() {
        let mut observer = established_observer();

        // Position is intentionally absent from TimelineSemanticStamp and
        // from plan(). Repeating the same semantic publication while an
        // ordinary position tick advances must remain a no-op.
        for _position_ms in [0_u64, 23, 46, 69, 92] {
            let plan = observer.plan(41, 1, stamp()).unwrap();
            assert_eq!(plan.kind(), AsioTimelineTransportPlanKind::Noop);
            assert!(!plan.requires_rotation());
            commit_target(&mut observer, plan);
        }
    }

    #[test]
    fn identical_stamp_is_a_noop() {
        let mut observer = established_observer();
        let before = observer.committed;

        let plan = observer.plan(41, 1, stamp()).unwrap();
        assert_eq!(plan.kind(), AsioTimelineTransportPlanKind::Noop);
        assert_eq!(plan.confirmed_transport_generation(), 1);
        commit_target(&mut observer, plan);

        assert_eq!(observer.committed, before);
    }

    #[test]
    fn each_semantic_field_change_requests_one_rotation() {
        let variants = [
            TimelineSemanticStamp::new(4, 5, 7, 11, true),
            TimelineSemanticStamp::new(3, 6, 7, 11, true),
            TimelineSemanticStamp::new(3, 5, 8, 11, true),
            TimelineSemanticStamp::new(3, 5, 7, 12, true),
            TimelineSemanticStamp::new(3, 5, 7, 11, false),
        ];

        for changed in variants {
            let mut observer = established_observer();
            let plan = observer.plan(41, 1, changed).unwrap();
            assert_eq!(plan.kind(), AsioTimelineTransportPlanKind::Rotate);
            assert_eq!(plan.confirmed_transport_generation(), 2);
            commit_target(&mut observer, plan);

            let no_second_rotation = observer.plan(41, 2, changed).unwrap();
            assert_eq!(
                no_second_rotation.kind(),
                AsioTimelineTransportPlanKind::Noop
            );
            assert_eq!(no_second_rotation.confirmed_transport_generation(), 2);
        }
    }

    #[test]
    fn failed_rotation_can_retry_without_losing_pending_change() {
        let mut observer = established_observer();
        let changed = TimelineSemanticStamp::new(3, 5, 8, 11, true);

        let first_attempt = observer.plan(41, 1, changed).unwrap();
        assert_eq!(first_attempt.kind(), AsioTimelineTransportPlanKind::Rotate);
        assert_eq!(observer.committed.unwrap().transport_generation, 1);

        // A failed runtime.rotate_transport_generation is represented by not
        // committing the plan. The retry is deterministic and identical.
        let retry = observer.plan(41, 1, changed).unwrap();
        assert_eq!(retry, first_attempt);
        commit_target(&mut observer, retry);
        assert_eq!(observer.committed.unwrap().transport_generation, 2);
    }

    #[test]
    fn external_runtime_rotation_rebases_without_a_second_rotation() {
        let mut observer = established_observer();
        let changed = TimelineSemanticStamp::new(3, 6, 8, 12, false);

        let rebase = observer.plan(41, 2, changed).unwrap();
        assert_eq!(
            rebase.kind(),
            AsioTimelineTransportPlanKind::RebaseExternalGeneration
        );
        assert!(!rebase.requires_rotation());
        assert_eq!(rebase.confirmed_transport_generation(), 2);
        commit_target(&mut observer, rebase);

        let no_double_rotation = observer.plan(41, 2, changed).unwrap();
        assert_eq!(
            no_double_rotation.kind(),
            AsioTimelineTransportPlanKind::Noop
        );
        assert_eq!(no_double_rotation.confirmed_transport_generation(), 2);
    }

    #[test]
    fn external_runtime_rotation_with_the_same_stamp_rebases_without_rotation() {
        let mut observer = established_observer();

        let rebase = observer.plan(41, 2, stamp()).unwrap();
        assert_eq!(
            rebase.kind(),
            AsioTimelineTransportPlanKind::RebaseExternalGeneration
        );
        assert!(!rebase.requires_rotation());
        assert_eq!(rebase.confirmed_transport_generation(), 2);
        commit_target(&mut observer, rebase);

        let no_double_rotation = observer.plan(41, 2, stamp()).unwrap();
        assert_eq!(
            no_double_rotation.kind(),
            AsioTimelineTransportPlanKind::Noop
        );
    }

    #[test]
    fn new_session_establishes_a_baseline_without_rotation() {
        let mut observer = established_observer();
        let new_stamp = TimelineSemanticStamp::new(99, 100, 101, 102, false);

        let rollover = observer.plan(42, 1, new_stamp).unwrap();
        assert_eq!(
            rollover.kind(),
            AsioTimelineTransportPlanKind::EstablishBaseline
        );
        assert!(!rollover.requires_rotation());
        assert_eq!(rollover.confirmed_transport_generation(), 1);
        commit_target(&mut observer, rollover);

        let no_op = observer.plan(42, 1, new_stamp).unwrap();
        assert_eq!(no_op.kind(), AsioTimelineTransportPlanKind::Noop);
    }

    #[test]
    fn zero_inputs_exhaustion_and_stale_commits_leave_state_unchanged() {
        let mut observer = established_observer();
        let baseline = observer.committed;
        let changed = TimelineSemanticStamp::new(3, 5, 8, 11, true);

        assert_eq!(
            observer.plan(0, 1, changed),
            Err(AsioTimelineTransportError::InvalidSessionGeneration)
        );
        assert_eq!(observer.committed, baseline);
        assert_eq!(
            observer.plan(41, 0, changed),
            Err(AsioTimelineTransportError::InvalidTransportGeneration)
        );
        assert_eq!(observer.committed, baseline);

        let mut exhausted = AsioTimelineTransportObserver::new();
        let establish_max = exhausted.plan(41, u64::MAX, stamp()).unwrap();
        commit_target(&mut exhausted, establish_max);
        let exhausted_before = exhausted.committed;
        assert_eq!(
            exhausted.plan(41, u64::MAX, changed),
            Err(AsioTimelineTransportError::GenerationExhausted)
        );
        assert_eq!(exhausted.committed, exhausted_before);

        let stale = observer.plan(41, 1, changed).unwrap();
        commit_target(&mut observer, stale);
        let after_first_commit = observer.committed;
        let later = observer
            .plan(41, 2, TimelineSemanticStamp::new(3, 5, 9, 11, true))
            .unwrap();
        commit_target(&mut observer, later);
        let before_stale_commit = observer.committed;
        assert_eq!(
            observer.commit(stale, 2),
            Err(AsioTimelineTransportError::StalePlan {
                expected: after_first_commit.map(|_| AsioTimelineTransportObservation {
                    session_generation: 41,
                    transport_generation: 1,
                    semantic: stamp(),
                }),
                actual: before_stale_commit,
            })
        );
        assert_eq!(observer.committed, before_stale_commit);
    }

    #[test]
    fn invalid_confirmed_or_stale_generation_is_rejected_without_mutation() {
        let mut observer = established_observer();
        let changed = TimelineSemanticStamp::new(3, 5, 8, 11, true);
        let plan = observer.plan(41, 1, changed).unwrap();
        let before = observer.committed;

        assert_eq!(
            observer.commit(plan, 0),
            Err(AsioTimelineTransportError::InvalidConfirmedGeneration)
        );
        assert_eq!(observer.committed, before);
        assert_eq!(
            observer.commit(plan, 99),
            Err(AsioTimelineTransportError::ConfirmedGenerationMismatch {
                expected: 2,
                actual: 99,
            })
        );
        assert_eq!(observer.committed, before);

        commit_target(&mut observer, plan);
        let after = observer.committed;
        assert_eq!(
            observer.commit(plan, 2),
            Err(AsioTimelineTransportError::StalePlan {
                expected: before,
                actual: after,
            })
        );
        assert_eq!(observer.committed, after);
    }

    #[test]
    fn rotate_plan_commits_only_the_generation_returned_by_the_active_runtime() {
        let (mut runtime, _context, _program_source, _cue_source) =
            AsioOutputRuntime::active_in_memory(73, 9).unwrap();
        let mut observer = AsioTimelineTransportObserver::new();
        synchronize_active_runtime(&mut observer, &mut runtime, stamp()).unwrap();

        let changed = TimelineSemanticStamp::new(3, 5, 8, 12, false);
        synchronize_active_runtime(&mut observer, &mut runtime, changed).unwrap();

        let current = runtime.current_preflight_identity().unwrap();
        assert_eq!(current.transport_generation(), 10);
        let no_double_rotation = observer
            .plan(
                current.session_generation(),
                current.transport_generation(),
                changed,
            )
            .unwrap();
        assert_eq!(
            no_double_rotation.kind(),
            AsioTimelineTransportPlanKind::Noop
        );
    }

    #[test]
    fn playing_baseline_clears_preflight_before_live_audio_is_admitted() {
        let (mut runtime, _context, _program_source, _cue_source) =
            AsioOutputRuntime::active_in_memory(74, 1).unwrap();
        let now_ms = preflight_now_ms();
        runtime
            .set_preflight_test(PreflightTarget::Cue, now_ms, 30_000)
            .unwrap();
        let armed = runtime.preflight_status(now_ms).unwrap();
        assert_eq!(armed.test(), Some(PreflightTarget::Cue));
        assert_eq!(armed.solo(), PreflightSolo::None);
        assert!(!armed.live_playback_active());
        let mut observer = AsioTimelineTransportObserver::new();

        synchronize_active_runtime(&mut observer, &mut runtime, stamp()).unwrap();

        let status = runtime.preflight_status(preflight_now_ms()).unwrap();
        assert_eq!(status.test(), None);
        assert_eq!(status.solo(), PreflightSolo::None);
        assert!(status.live_playback_active());
        assert_eq!(
            runtime
                .current_preflight_identity()
                .unwrap()
                .transport_generation(),
            1,
            "establishing a playing baseline clears preflight without a fabricated rotation"
        );
    }

    #[test]
    fn playing_baseline_clears_solo_before_live_audio_is_admitted() {
        let (mut runtime, _context, _program_source, _cue_source) =
            AsioOutputRuntime::active_in_memory(75, 1).unwrap();
        let now_ms = preflight_now_ms();
        runtime
            .set_preflight_solo(PreflightSolo::CueOnly, now_ms)
            .unwrap();
        let armed = runtime.preflight_status(now_ms).unwrap();
        assert_eq!(armed.test(), None);
        assert_eq!(armed.solo(), PreflightSolo::CueOnly);
        assert!(!armed.live_playback_active());
        let mut observer = AsioTimelineTransportObserver::new();

        synchronize_active_runtime(&mut observer, &mut runtime, stamp()).unwrap();

        let status = runtime.preflight_status(preflight_now_ms()).unwrap();
        assert_eq!(status.test(), None);
        assert_eq!(status.solo(), PreflightSolo::None);
        assert!(status.live_playback_active());
        assert_eq!(
            runtime
                .current_preflight_identity()
                .unwrap()
                .transport_generation(),
            1,
            "establishing a playing baseline clears preflight without a fabricated rotation"
        );
    }
}
