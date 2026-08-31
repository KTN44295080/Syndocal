//! Deterministic contract for the two fixed same-machine show Spout outputs.
//!
//! This module intentionally has no engine or transport integration. Its
//! state machine distinguishes stopped content from physical output teardown:
//! `KeepAliveBlack` keeps an already-open sender alive, while `Fault` retires
//! it after authority is lost.

use std::fmt;

use protocol::{
    CompositionId, CompositionSummary, VideoOutputId, VideoOutputKind, VideoOutputMapping,
    VideoOutputSummary,
};

pub(crate) const SHOW_SPOUT_BACKGROUND_NAME: &str = "Syndocal Background";
pub(crate) const SHOW_SPOUT_FOREGROUND_NAME: &str = "Syndocal Foreground";
pub(crate) const SHOW_SPOUT_WIDTH: u32 = 1920;
pub(crate) const SHOW_SPOUT_HEIGHT: u32 = 1080;
pub(crate) const SHOW_SPOUT_OPACITY: f32 = 1.0;
/// `Main` is deliberately not a publication target.  V1 pointed both fixed
/// senders here; V2 is a clean break with one separately authored composition
/// per sender.
pub(crate) const SHOW_SPOUT_MAIN_COMPOSITION_ID: CompositionId = 1;
pub(crate) const SHOW_SPOUT_MAIN_COMPOSITION_LABEL: &str = "Main";
pub(crate) const SHOW_SPOUT_BACKGROUND_COMPOSITION_LABEL: &str = "Background Video2 Camera";
pub(crate) const SHOW_SPOUT_FOREGROUND_COMPOSITION_LABEL: &str = "Foreground Video 1";
pub(crate) const SHOW_SPOUT_FRAME_PIXEL_LEN: usize =
    (SHOW_SPOUT_WIDTH as usize) * (SHOW_SPOUT_HEIGHT as usize);

/// Both senders use RGB black. The receiver derives Foreground transparency
/// from RGB brightness, so the alpha byte is deliberately not relied upon.
pub(crate) const SHOW_SPOUT_BLACK_PIXEL_RGBA: [u8; 4] = [0, 0, 0, 255];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShowSpoutOutputs {
    pub(crate) background: VideoOutputSummary,
    pub(crate) foreground: VideoOutputSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShowSpoutCompositionTargets {
    pub(crate) background: CompositionId,
    pub(crate) foreground: CompositionId,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ShowSpoutResetCandidate {
    Absent,
    LegacyV1(ShowSpoutOutputs),
    CurrentV2(ShowSpoutOutputs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShowSpoutValidationError {
    MissingPair,
    PartialPair,
    ExtraSpoutOutputs,
    DuplicateOutputId(VideoOutputId),
    InvalidOutputId,
    InvalidCompositionId,
    CompositionMismatch,
    InvalidCompositionTarget,
    LegacyPairRejected,
    InvalidOutputBackreference,
    DuplicateBackground,
    DuplicateForeground,
    UnexpectedSpoutOutput,
    ReservedNameWrongKind,
    NameMismatch,
    InvalidDimensions,
    Disabled,
    BlackoutEnabled,
    InvalidOpacity,
    InvalidPresentationTarget,
    InvalidMapping,
    ConflictingPair,
}

impl fmt::Display for ShowSpoutValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingPair => "show Spout outputs are missing",
            Self::PartialPair => "show Spout outputs require both fixed senders",
            Self::ExtraSpoutOutputs => "show Spout outputs must contain exactly two senders",
            Self::DuplicateOutputId(id) => {
                return write!(formatter, "duplicate video output id {id}")
            }
            Self::InvalidOutputId => "show Spout output id must be non-zero",
            Self::InvalidCompositionId => "show Spout composition id must be non-zero",
            Self::CompositionMismatch => "show Spout outputs must use distinct target compositions",
            Self::InvalidCompositionTarget => {
                "show Spout targets must be unique exact-name non-Main compositions"
            }
            Self::LegacyPairRejected => {
                "legacy same-Main show Spout pair is retired; reset it before enabling V2"
            }
            Self::InvalidOutputBackreference => {
                "show Spout composition output backreferences are inconsistent"
            }
            Self::DuplicateBackground => "duplicate Syndocal Background Spout output",
            Self::DuplicateForeground => "duplicate Syndocal Foreground Spout output",
            Self::UnexpectedSpoutOutput => "unexpected Spout sender in show output set",
            Self::ReservedNameWrongKind => {
                "reserved show Spout name is used by another output kind"
            }
            Self::NameMismatch => "show Spout label and endpoint must match exactly",
            Self::InvalidDimensions => "show Spout sender must be exactly 1920x1080",
            Self::Disabled => "show Spout sender must be enabled",
            Self::BlackoutEnabled => "show Spout sender blackout must be disabled",
            Self::InvalidOpacity => "show Spout sender opacity must be exactly 1.0",
            Self::InvalidPresentationTarget => {
                "show Spout sender cannot be fullscreen or target a monitor"
            }
            Self::InvalidMapping => "show Spout sender mapping must be the exact default",
            Self::ConflictingPair => "a different valid show Spout pair is already present",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ShowSpoutValidationError {}

/// Build the two fixed physical outputs for an atomic engine publication.
pub(crate) fn build_show_spout_outputs(
    background_id: VideoOutputId,
    foreground_id: VideoOutputId,
    background_composition_id: CompositionId,
    foreground_composition_id: CompositionId,
) -> Result<ShowSpoutOutputs, ShowSpoutValidationError> {
    if background_id == 0 || foreground_id == 0 {
        return Err(ShowSpoutValidationError::InvalidOutputId);
    }
    if background_id == foreground_id {
        return Err(ShowSpoutValidationError::DuplicateOutputId(background_id));
    }
    if background_composition_id == 0 || foreground_composition_id == 0 {
        return Err(ShowSpoutValidationError::InvalidCompositionId);
    }
    if background_composition_id == foreground_composition_id
        || background_composition_id == SHOW_SPOUT_MAIN_COMPOSITION_ID
        || foreground_composition_id == SHOW_SPOUT_MAIN_COMPOSITION_ID
    {
        return Err(ShowSpoutValidationError::InvalidCompositionTarget);
    }
    Ok(ShowSpoutOutputs {
        background: fixed_output(
            background_id,
            SHOW_SPOUT_BACKGROUND_NAME,
            background_composition_id,
        ),
        foreground: fixed_output(
            foreground_id,
            SHOW_SPOUT_FOREGROUND_NAME,
            foreground_composition_id,
        ),
    })
}

pub(crate) fn derive_show_spout_composition_targets(
    compositions: &[CompositionSummary],
) -> Result<ShowSpoutCompositionTargets, ShowSpoutValidationError> {
    let main = unique_composition_id(compositions, SHOW_SPOUT_MAIN_COMPOSITION_LABEL)?;
    if main != SHOW_SPOUT_MAIN_COMPOSITION_ID {
        return Err(ShowSpoutValidationError::InvalidCompositionTarget);
    }
    let background = unique_composition_id(compositions, SHOW_SPOUT_BACKGROUND_COMPOSITION_LABEL)?;
    let foreground = unique_composition_id(compositions, SHOW_SPOUT_FOREGROUND_COMPOSITION_LABEL)?;
    if background == 0
        || foreground == 0
        || background == foreground
        || background == main
        || foreground == main
    {
        return Err(ShowSpoutValidationError::InvalidCompositionTarget);
    }
    Ok(ShowSpoutCompositionTargets {
        background,
        foreground,
    })
}

fn unique_composition_id(
    compositions: &[CompositionSummary],
    label: &str,
) -> Result<CompositionId, ShowSpoutValidationError> {
    let mut matches = compositions
        .iter()
        .filter(|composition| composition.label == label);
    let Some(composition) = matches.next() else {
        return Err(ShowSpoutValidationError::InvalidCompositionTarget);
    };
    if matches.next().is_some() || composition.id == 0 {
        return Err(ShowSpoutValidationError::InvalidCompositionTarget);
    }
    Ok(composition.id)
}

fn fixed_output(
    id: VideoOutputId,
    name: &str,
    composition_id: CompositionId,
) -> VideoOutputSummary {
    VideoOutputSummary {
        id,
        label: name.to_string(),
        kind: VideoOutputKind::SpoutSender,
        enabled: true,
        composition_id,
        fullscreen: false,
        monitor_id: None,
        monitor_identity: None,
        width: SHOW_SPOUT_WIDTH,
        height: SHOW_SPOUT_HEIGHT,
        endpoint_name: Some(name.to_string()),
        opacity: SHOW_SPOUT_OPACITY,
        blackout: false,
        mapping: VideoOutputMapping::default(),
    }
}

/// Validate a complete project output list and return the pair in canonical
/// Background/Foreground order. Other output kinds are allowed, but every
/// Spout sender is part of this exact two-sender contract.
pub(crate) fn validate_show_spout_outputs(
    outputs: &[VideoOutputSummary],
) -> Result<ShowSpoutOutputs, ShowSpoutValidationError> {
    validate_unique_output_ids(outputs)?;
    reject_reserved_non_spout_outputs(outputs)?;
    let spout: Vec<&VideoOutputSummary> = outputs
        .iter()
        .filter(|output| output.kind == VideoOutputKind::SpoutSender)
        .collect();
    match spout.len() {
        0 => Err(ShowSpoutValidationError::MissingPair),
        1 => Err(ShowSpoutValidationError::PartialPair),
        2 => {
            let pair = collect_pair(spout)?;
            validate_pair_contract(&pair)?;
            Ok(pair)
        }
        _ => Err(ShowSpoutValidationError::ExtraSpoutOutputs),
    }
}

fn validate_unique_output_ids(
    outputs: &[VideoOutputSummary],
) -> Result<(), ShowSpoutValidationError> {
    let mut seen = std::collections::BTreeSet::new();
    for output in outputs {
        if output.id == 0 {
            return Err(ShowSpoutValidationError::InvalidOutputId);
        }
        if !seen.insert(output.id) {
            return Err(ShowSpoutValidationError::DuplicateOutputId(output.id));
        }
    }
    Ok(())
}

fn reject_reserved_non_spout_outputs(
    outputs: &[VideoOutputSummary],
) -> Result<(), ShowSpoutValidationError> {
    for output in outputs {
        if output.kind == VideoOutputKind::SpoutSender {
            continue;
        }
        if output.label == SHOW_SPOUT_BACKGROUND_NAME
            || output.label == SHOW_SPOUT_FOREGROUND_NAME
            || output.endpoint_name.as_deref() == Some(SHOW_SPOUT_BACKGROUND_NAME)
            || output.endpoint_name.as_deref() == Some(SHOW_SPOUT_FOREGROUND_NAME)
        {
            return Err(ShowSpoutValidationError::ReservedNameWrongKind);
        }
    }
    Ok(())
}

fn collect_pair<'a>(
    outputs: Vec<&'a VideoOutputSummary>,
) -> Result<ShowSpoutOutputs, ShowSpoutValidationError> {
    let mut background: Option<&VideoOutputSummary> = None;
    let mut foreground: Option<&VideoOutputSummary> = None;
    for output in outputs {
        match exact_slot(output)? {
            ShowSpoutSlot::Background => {
                if background.replace(output).is_some() {
                    return Err(ShowSpoutValidationError::DuplicateBackground);
                }
            }
            ShowSpoutSlot::Foreground => {
                if foreground.replace(output).is_some() {
                    return Err(ShowSpoutValidationError::DuplicateForeground);
                }
            }
        }
    }
    let Some(background) = background else {
        return Err(ShowSpoutValidationError::PartialPair);
    };
    let Some(foreground) = foreground else {
        return Err(ShowSpoutValidationError::PartialPair);
    };
    Ok(ShowSpoutOutputs {
        background: background.clone(),
        foreground: foreground.clone(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShowSpoutSlot {
    Background,
    Foreground,
}

fn exact_slot(output: &VideoOutputSummary) -> Result<ShowSpoutSlot, ShowSpoutValidationError> {
    match (output.label.as_str(), output.endpoint_name.as_deref()) {
        (SHOW_SPOUT_BACKGROUND_NAME, Some(SHOW_SPOUT_BACKGROUND_NAME)) => {
            Ok(ShowSpoutSlot::Background)
        }
        (SHOW_SPOUT_FOREGROUND_NAME, Some(SHOW_SPOUT_FOREGROUND_NAME)) => {
            Ok(ShowSpoutSlot::Foreground)
        }
        _ => Err(ShowSpoutValidationError::NameMismatch),
    }
}

fn validate_output_contract(output: &VideoOutputSummary) -> Result<(), ShowSpoutValidationError> {
    if output.kind != VideoOutputKind::SpoutSender {
        return Err(ShowSpoutValidationError::UnexpectedSpoutOutput);
    }
    if output.id == 0 {
        return Err(ShowSpoutValidationError::InvalidOutputId);
    }
    if output.width != SHOW_SPOUT_WIDTH || output.height != SHOW_SPOUT_HEIGHT {
        return Err(ShowSpoutValidationError::InvalidDimensions);
    }
    if !output.enabled {
        return Err(ShowSpoutValidationError::Disabled);
    }
    if output.blackout {
        return Err(ShowSpoutValidationError::BlackoutEnabled);
    }
    if output.opacity != SHOW_SPOUT_OPACITY {
        return Err(ShowSpoutValidationError::InvalidOpacity);
    }
    if output.fullscreen || output.monitor_id.is_some() || output.monitor_identity.is_some() {
        return Err(ShowSpoutValidationError::InvalidPresentationTarget);
    }
    if output.mapping != VideoOutputMapping::default() {
        return Err(ShowSpoutValidationError::InvalidMapping);
    }
    Ok(())
}

fn validate_pair_contract(pair: &ShowSpoutOutputs) -> Result<(), ShowSpoutValidationError> {
    validate_output_contract(&pair.background)?;
    validate_output_contract(&pair.foreground)?;
    if pair.background.composition_id == 0 || pair.foreground.composition_id == 0 {
        return Err(ShowSpoutValidationError::InvalidCompositionId);
    }
    if pair.background.composition_id == pair.foreground.composition_id
        || pair.background.composition_id == SHOW_SPOUT_MAIN_COMPOSITION_ID
        || pair.foreground.composition_id == SHOW_SPOUT_MAIN_COMPOSITION_ID
    {
        return Err(ShowSpoutValidationError::CompositionMismatch);
    }
    Ok(())
}

fn validate_legacy_pair_contract(pair: &ShowSpoutOutputs) -> Result<(), ShowSpoutValidationError> {
    validate_output_contract(&pair.background)?;
    validate_output_contract(&pair.foreground)?;
    if pair.background.composition_id != SHOW_SPOUT_MAIN_COMPOSITION_ID
        || pair.foreground.composition_id != SHOW_SPOUT_MAIN_COMPOSITION_ID
    {
        return Err(ShowSpoutValidationError::InvalidCompositionId);
    }
    Ok(())
}

/// Validate the V2 pair against the authoritative composed snapshot.  Output
/// IDs are derived fields, so both forward assignment and every composition
/// backreference are checked before a physical sender is admitted.
pub(crate) fn validate_show_spout_outputs_with_compositions(
    outputs: &[VideoOutputSummary],
    compositions: &[CompositionSummary],
) -> Result<ShowSpoutOutputs, ShowSpoutValidationError> {
    let pair = match validate_show_spout_outputs(outputs) {
        Err(ShowSpoutValidationError::CompositionMismatch)
            if outputs
                .iter()
                .filter(|output| output.kind == VideoOutputKind::SpoutSender)
                .count()
                == 2
                && outputs
                    .iter()
                    .filter(|output| output.kind == VideoOutputKind::SpoutSender)
                    .all(|output| output.composition_id == SHOW_SPOUT_MAIN_COMPOSITION_ID) =>
        {
            return Err(ShowSpoutValidationError::LegacyPairRejected)
        }
        result => result?,
    };
    let targets = derive_show_spout_composition_targets(compositions)?;
    if pair.background.composition_id != targets.background
        || pair.foreground.composition_id != targets.foreground
    {
        return Err(ShowSpoutValidationError::InvalidCompositionTarget);
    }
    validate_output_backreferences(&pair, compositions, &targets)?;
    Ok(pair)
}

fn validate_output_backreferences(
    pair: &ShowSpoutOutputs,
    compositions: &[CompositionSummary],
    targets: &ShowSpoutCompositionTargets,
) -> Result<(), ShowSpoutValidationError> {
    for composition in compositions {
        let wants_background = composition.id == targets.background;
        let wants_foreground = composition.id == targets.foreground;
        let has_background = composition
            .output_ids
            .iter()
            .filter(|id| **id == pair.background.id)
            .count();
        let has_foreground = composition
            .output_ids
            .iter()
            .filter(|id| **id == pair.foreground.id)
            .count();
        if has_background > 1
            || has_foreground > 1
            || (wants_background && has_background != 1)
            || (!wants_background && has_background != 0)
            || (wants_foreground && has_foreground != 1)
            || (!wants_foreground && has_foreground != 0)
        {
            return Err(ShowSpoutValidationError::InvalidOutputBackreference);
        }
    }
    Ok(())
}

/// Reset accepts only absence, the exact retired V1 same-Main pair, or the
/// exact current V2 pair.  It deliberately does not classify a partial or a
/// generic Spout sender as safe to delete.
pub(crate) fn classify_show_spout_reset_candidate(
    outputs: &[VideoOutputSummary],
    compositions: &[CompositionSummary],
) -> Result<ShowSpoutResetCandidate, ShowSpoutValidationError> {
    validate_unique_output_ids(outputs)?;
    reject_reserved_non_spout_outputs(outputs)?;
    let spout: Vec<&VideoOutputSummary> = outputs
        .iter()
        .filter(|output| output.kind == VideoOutputKind::SpoutSender)
        .collect();
    if spout.is_empty() {
        return Ok(ShowSpoutResetCandidate::Absent);
    }
    if spout.len() != 2 {
        return Err(if spout.len() == 1 {
            ShowSpoutValidationError::PartialPair
        } else {
            ShowSpoutValidationError::ExtraSpoutOutputs
        });
    }
    let pair = collect_pair(spout)?;
    if pair.background.composition_id == SHOW_SPOUT_MAIN_COMPOSITION_ID
        && pair.foreground.composition_id == SHOW_SPOUT_MAIN_COMPOSITION_ID
    {
        validate_legacy_pair_contract(&pair)?;
        validate_legacy_output_backreferences(&pair, compositions)?;
        return Ok(ShowSpoutResetCandidate::LegacyV1(pair));
    }
    let pair = validate_show_spout_outputs_with_compositions(outputs, compositions)?;
    Ok(ShowSpoutResetCandidate::CurrentV2(pair))
}

fn validate_legacy_output_backreferences(
    pair: &ShowSpoutOutputs,
    compositions: &[CompositionSummary],
) -> Result<(), ShowSpoutValidationError> {
    let main = compositions
        .iter()
        .filter(|composition| {
            composition.id == SHOW_SPOUT_MAIN_COMPOSITION_ID
                && composition.label == SHOW_SPOUT_MAIN_COMPOSITION_LABEL
        })
        .collect::<Vec<_>>();
    if main.len() != 1 {
        return Err(ShowSpoutValidationError::InvalidCompositionTarget);
    }
    for composition in compositions {
        let is_main = composition.id == SHOW_SPOUT_MAIN_COMPOSITION_ID;
        let has_background = composition
            .output_ids
            .iter()
            .filter(|id| **id == pair.background.id)
            .count();
        let has_foreground = composition
            .output_ids
            .iter()
            .filter(|id| **id == pair.foreground.id)
            .count();
        if has_background > 1
            || has_foreground > 1
            || (is_main && (has_background != 1 || has_foreground != 1))
            || (!is_main && (has_background != 0 || has_foreground != 0))
        {
            return Err(ShowSpoutValidationError::InvalidOutputBackreference);
        }
    }
    Ok(())
}

fn same_physical_spec(left: &VideoOutputSummary, right: &VideoOutputSummary) -> bool {
    left == right
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShowSpoutEnsureDecision {
    CreatePair,
    NoOp,
}

/// Decide whether an atomic ensure action must create the pair or is already
/// satisfied. A valid but different pair is a conflict, never replacement.
pub(crate) fn decide_show_spout_ensure(
    existing_outputs: &[VideoOutputSummary],
    desired: &ShowSpoutOutputs,
) -> Result<ShowSpoutEnsureDecision, ShowSpoutValidationError> {
    validate_pair_contract(desired)?;
    if desired.background.id == desired.foreground.id {
        return Err(ShowSpoutValidationError::DuplicateOutputId(
            desired.background.id,
        ));
    }
    validate_unique_output_ids(existing_outputs)?;
    reject_reserved_non_spout_outputs(existing_outputs)?;
    let spout: Vec<&VideoOutputSummary> = existing_outputs
        .iter()
        .filter(|output| output.kind == VideoOutputKind::SpoutSender)
        .collect();
    match spout.len() {
        0 => Ok(ShowSpoutEnsureDecision::CreatePair),
        1 => Err(ShowSpoutValidationError::PartialPair),
        2 => {
            let current = collect_pair(spout)?;
            validate_pair_contract(&current)?;
            if same_physical_spec(&current.background, &desired.background)
                && same_physical_spec(&current.foreground, &desired.foreground)
            {
                Ok(ShowSpoutEnsureDecision::NoOp)
            } else {
                Err(ShowSpoutValidationError::ConflictingPair)
            }
        }
        _ => Err(ShowSpoutValidationError::ExtraSpoutOutputs),
    }
}

/// An owned frame intended to be allocated once and reused by every
/// keep-alive send. Do not construct it in a 44Hz render tick.
#[derive(Debug)]
pub(crate) struct ShowSpoutBlackFrame {
    rgba: Box<[u8]>,
}

impl ShowSpoutBlackFrame {
    pub(crate) fn new() -> Self {
        Self {
            rgba: SHOW_SPOUT_BLACK_PIXEL_RGBA
                .repeat(SHOW_SPOUT_FRAME_PIXEL_LEN)
                .into_boxed_slice(),
        }
    }

    pub(crate) const fn width(&self) -> u32 {
        SHOW_SPOUT_WIDTH
    }

    pub(crate) const fn height(&self) -> u32 {
        SHOW_SPOUT_HEIGHT
    }

    pub(crate) fn as_rgba(&self) -> &[u8] {
        &self.rgba
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShowSpoutPresentationState {
    Live,
    KeepAliveBlack,
    Transitioning { sender_open: bool },
    Fault,
}

impl ShowSpoutPresentationState {
    pub(crate) const fn initial() -> Self {
        Self::transitioning_without_sender()
    }

    pub(crate) const fn transitioning_without_sender() -> Self {
        Self::Transitioning { sender_open: false }
    }

    pub(crate) const fn transitioning_with_sender() -> Self {
        Self::Transitioning { sender_open: true }
    }

    pub(crate) const fn sender_remains_open(self) -> bool {
        match self {
            Self::Live | Self::KeepAliveBlack => true,
            Self::Transitioning { sender_open } => sender_open,
            Self::Fault => false,
        }
    }

    pub(crate) const fn sends_black_frame(self) -> bool {
        matches!(self, Self::KeepAliveBlack)
    }

    pub(crate) fn apply(
        self,
        event: ShowSpoutPresentationEvent,
    ) -> Result<Self, ShowSpoutStateTransitionError> {
        use ShowSpoutPresentationEvent::*;
        use ShowSpoutPresentationState::*;

        match (self, event) {
            (_, AuthorityLost) => Ok(Fault),
            (Fault, AuthorityRestored) => Ok(Self::transitioning_without_sender()),
            (Live, BeginTransition) | (KeepAliveBlack, BeginTransition) => {
                Ok(Self::transitioning_with_sender())
            }
            (Transitioning { sender_open }, BeginTransition) => {
                Ok(Self::Transitioning { sender_open })
            }
            (Transitioning { sender_open: false }, SenderEstablished) => {
                Ok(Self::transitioning_with_sender())
            }
            (Transitioning { sender_open: true }, ContentLive)
            | (Live | KeepAliveBlack, ContentLive) => Ok(Live),
            (Transitioning { sender_open: true }, ContentStopped)
            | (Live | KeepAliveBlack, ContentStopped) => Ok(KeepAliveBlack),
            (Transitioning { sender_open: false }, ContentLive | ContentStopped) => {
                Err(ShowSpoutStateTransitionError { state: self, event })
            }
            (Fault, ContentLive | ContentStopped | BeginTransition) => {
                Err(ShowSpoutStateTransitionError { state: self, event })
            }
            (_, SenderEstablished) => Err(ShowSpoutStateTransitionError { state: self, event }),
            (_, AuthorityRestored) => Err(ShowSpoutStateTransitionError { state: self, event }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShowSpoutPresentationEvent {
    BeginTransition,
    ContentLive,
    ContentStopped,
    AuthorityLost,
    AuthorityRestored,
    SenderEstablished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShowSpoutStateTransitionError {
    pub(crate) state: ShowSpoutPresentationState,
    pub(crate) event: ShowSpoutPresentationEvent,
}

impl fmt::Display for ShowSpoutStateTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid show Spout state transition: {:?} + {:?}",
            self.state, self.event
        )
    }
}

impl std::error::Error for ShowSpoutStateTransitionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair() -> ShowSpoutOutputs {
        build_show_spout_outputs(11, 12, 3, 2).expect("valid V2 pair")
    }

    fn unrelated_display(id: VideoOutputId) -> VideoOutputSummary {
        let mut output = pair().background;
        output.id = id;
        output.label = "Preview Display".to_string();
        output.kind = VideoOutputKind::Display;
        output.endpoint_name = None;
        output
    }

    fn composition(
        id: CompositionId,
        label: &str,
        output_ids: Vec<VideoOutputId>,
    ) -> CompositionSummary {
        CompositionSummary {
            id,
            label: label.to_string(),
            layer_ids: Vec::new(),
            timeline_layer_ids: Vec::new(),
            output_ids,
        }
    }

    fn v2_compositions(expected: &ShowSpoutOutputs) -> Vec<CompositionSummary> {
        vec![
            composition(1, SHOW_SPOUT_MAIN_COMPOSITION_LABEL, Vec::new()),
            composition(
                2,
                SHOW_SPOUT_FOREGROUND_COMPOSITION_LABEL,
                vec![expected.foreground.id],
            ),
            composition(
                3,
                SHOW_SPOUT_BACKGROUND_COMPOSITION_LABEL,
                vec![expected.background.id],
            ),
        ]
    }

    fn legacy_pair() -> ShowSpoutOutputs {
        let mut legacy = pair();
        legacy.background.composition_id = SHOW_SPOUT_MAIN_COMPOSITION_ID;
        legacy.foreground.composition_id = SHOW_SPOUT_MAIN_COMPOSITION_ID;
        legacy
    }

    fn legacy_compositions(expected: &ShowSpoutOutputs) -> Vec<CompositionSummary> {
        vec![composition(
            SHOW_SPOUT_MAIN_COMPOSITION_ID,
            SHOW_SPOUT_MAIN_COMPOSITION_LABEL,
            vec![expected.background.id, expected.foreground.id],
        )]
    }

    #[test]
    fn exact_pair_is_order_independent_and_allows_other_kinds() {
        let expected = pair();
        let actual = validate_show_spout_outputs(&[
            expected.foreground.clone(),
            unrelated_display(90),
            expected.background.clone(),
        ])
        .expect("fixed pair should validate");
        assert_eq!(actual.background.id, expected.background.id);
        assert_eq!(actual.foreground.id, expected.foreground.id);
    }

    #[test]
    fn missing_partial_duplicate_and_extra_pairs_fail_closed() {
        let expected = pair();
        assert_eq!(
            validate_show_spout_outputs(&[]),
            Err(ShowSpoutValidationError::MissingPair)
        );
        assert_eq!(
            validate_show_spout_outputs(&[expected.background.clone()]),
            Err(ShowSpoutValidationError::PartialPair)
        );
        let mut duplicate_background = expected.background.clone();
        duplicate_background.id = 13;
        assert_eq!(
            validate_show_spout_outputs(&[expected.background.clone(), duplicate_background]),
            Err(ShowSpoutValidationError::DuplicateBackground)
        );
        let mut extra = unrelated_display(14);
        extra.kind = VideoOutputKind::SpoutSender;
        extra.label = "Extra sender".to_string();
        extra.endpoint_name = Some("Extra sender".to_string());
        assert_eq!(
            validate_show_spout_outputs(&[expected.background, expected.foreground, extra]),
            Err(ShowSpoutValidationError::ExtraSpoutOutputs)
        );
    }

    #[test]
    fn malformed_fields_and_composition_identity_fail_closed() {
        let expected = pair();
        let mut malformed = expected.background.clone();
        malformed.width = 1280;
        assert_eq!(
            validate_show_spout_outputs(&[malformed, expected.foreground.clone()]),
            Err(ShowSpoutValidationError::InvalidDimensions)
        );
        let mut disabled = expected.background.clone();
        disabled.enabled = false;
        assert_eq!(
            validate_show_spout_outputs(&[disabled, expected.foreground.clone()]),
            Err(ShowSpoutValidationError::Disabled)
        );
        let mut blackout = expected.background.clone();
        blackout.blackout = true;
        assert_eq!(
            validate_show_spout_outputs(&[blackout, expected.foreground.clone()]),
            Err(ShowSpoutValidationError::BlackoutEnabled)
        );
        let mut opacity = expected.background.clone();
        opacity.opacity = 0.5;
        assert_eq!(
            validate_show_spout_outputs(&[opacity, expected.foreground.clone()]),
            Err(ShowSpoutValidationError::InvalidOpacity)
        );
        let mut name = expected.background.clone();
        name.endpoint_name = Some("other".to_string());
        assert_eq!(
            validate_show_spout_outputs(&[name, expected.foreground.clone()]),
            Err(ShowSpoutValidationError::NameMismatch)
        );
        let mut fullscreen = expected.background.clone();
        fullscreen.fullscreen = true;
        assert_eq!(
            validate_show_spout_outputs(&[fullscreen, expected.foreground.clone()]),
            Err(ShowSpoutValidationError::InvalidPresentationTarget)
        );
        let mut monitor = expected.background.clone();
        monitor.monitor_id = Some(0);
        assert_eq!(
            validate_show_spout_outputs(&[monitor, expected.foreground.clone()]),
            Err(ShowSpoutValidationError::InvalidPresentationTarget)
        );
        let mut monitor_identity = expected.background.clone();
        monitor_identity.monitor_identity = Some("stable-display".to_string());
        assert_eq!(
            validate_show_spout_outputs(&[monitor_identity, expected.foreground.clone()]),
            Err(ShowSpoutValidationError::InvalidPresentationTarget)
        );
        let mut mapping = expected.background.clone();
        mapping.mapping.scale_x = 1.25;
        assert_eq!(
            validate_show_spout_outputs(&[mapping, expected.foreground.clone()]),
            Err(ShowSpoutValidationError::InvalidMapping)
        );
        let mut composition = expected.foreground.clone();
        composition.composition_id = expected.background.composition_id;
        assert_eq!(
            validate_show_spout_outputs(&[expected.background.clone(), composition]),
            Err(ShowSpoutValidationError::CompositionMismatch)
        );
        let mut zero_background = expected.background.clone();
        let mut zero_foreground = expected.foreground.clone();
        zero_background.composition_id = 0;
        zero_foreground.composition_id = 0;
        assert_eq!(
            validate_show_spout_outputs(&[zero_background, zero_foreground]),
            Err(ShowSpoutValidationError::InvalidCompositionId)
        );
        assert_eq!(
            build_show_spout_outputs(11, 12, 0, 2),
            Err(ShowSpoutValidationError::InvalidCompositionId)
        );
        assert_eq!(
            build_show_spout_outputs(11, 12, 2, 2),
            Err(ShowSpoutValidationError::InvalidCompositionTarget)
        );
    }

    #[test]
    fn reserved_name_wrong_kind_and_duplicate_ids_fail_closed() {
        let expected = pair();
        let mut collision = expected.background.clone();
        collision.kind = VideoOutputKind::Display;
        assert_eq!(
            validate_show_spout_outputs(&[collision, expected.foreground.clone()]),
            Err(ShowSpoutValidationError::ReservedNameWrongKind)
        );
        let mut duplicate_id = expected.foreground;
        duplicate_id.id = expected.background.id;
        assert_eq!(
            validate_show_spout_outputs(&[expected.background, duplicate_id]),
            Err(ShowSpoutValidationError::DuplicateOutputId(11))
        );
    }

    #[test]
    fn v2_targets_are_derived_exactly_and_legacy_enable_is_rejected() {
        let expected = pair();
        let compositions = v2_compositions(&expected);
        assert_eq!(
            derive_show_spout_composition_targets(&compositions),
            Ok(ShowSpoutCompositionTargets {
                background: 3,
                foreground: 2,
            })
        );
        assert_eq!(
            validate_show_spout_outputs_with_compositions(
                &[expected.background.clone(), expected.foreground.clone()],
                &compositions,
            ),
            Ok(expected.clone())
        );

        let legacy = legacy_pair();
        assert_eq!(
            validate_show_spout_outputs_with_compositions(
                &[legacy.background.clone(), legacy.foreground.clone()],
                &legacy_compositions(&legacy),
            ),
            Err(ShowSpoutValidationError::LegacyPairRejected)
        );
    }

    #[test]
    fn reset_recognizes_only_absent_or_exact_v1_v2_pairs() {
        let expected = pair();
        assert_eq!(
            classify_show_spout_reset_candidate(&[], &v2_compositions(&expected)),
            Ok(ShowSpoutResetCandidate::Absent)
        );
        assert_eq!(
            classify_show_spout_reset_candidate(
                &[expected.background.clone(), expected.foreground.clone()],
                &v2_compositions(&expected),
            ),
            Ok(ShowSpoutResetCandidate::CurrentV2(expected.clone()))
        );
        let legacy = legacy_pair();
        assert_eq!(
            classify_show_spout_reset_candidate(
                &[legacy.background.clone(), legacy.foreground.clone()],
                &legacy_compositions(&legacy),
            ),
            Ok(ShowSpoutResetCandidate::LegacyV1(legacy))
        );
    }

    #[test]
    fn ambiguous_targets_and_inconsistent_backreferences_fail_closed() {
        let expected = pair();
        let mut ambiguous = v2_compositions(&expected);
        ambiguous.push(composition(
            4,
            SHOW_SPOUT_BACKGROUND_COMPOSITION_LABEL,
            Vec::new(),
        ));
        assert_eq!(
            validate_show_spout_outputs_with_compositions(
                &[expected.background.clone(), expected.foreground.clone()],
                &ambiguous,
            ),
            Err(ShowSpoutValidationError::InvalidCompositionTarget)
        );

        let mut inconsistent = v2_compositions(&expected);
        inconsistent[2].output_ids.clear();
        assert_eq!(
            classify_show_spout_reset_candidate(
                &[expected.background.clone(), expected.foreground.clone()],
                &inconsistent,
            ),
            Err(ShowSpoutValidationError::InvalidOutputBackreference)
        );
    }

    #[test]
    fn ensure_distinguishes_create_noop_and_conflict() {
        let expected = pair();
        assert_eq!(
            decide_show_spout_ensure(&[unrelated_display(90)], &expected),
            Ok(ShowSpoutEnsureDecision::CreatePair)
        );
        assert_eq!(
            decide_show_spout_ensure(
                &[
                    expected.background.clone(),
                    unrelated_display(90),
                    expected.foreground.clone()
                ],
                &expected
            ),
            Ok(ShowSpoutEnsureDecision::NoOp)
        );
        let mut mapped_background = expected.background.clone();
        mapped_background.mapping.scale_x = 1.25;
        assert_eq!(
            decide_show_spout_ensure(&[mapped_background, expected.foreground.clone()], &expected),
            Err(ShowSpoutValidationError::InvalidMapping)
        );
        let mut different_composition = expected.background.clone();
        different_composition.composition_id = 2;
        let mut other_composition = expected.foreground.clone();
        other_composition.composition_id = 2;
        assert_eq!(
            decide_show_spout_ensure(&[different_composition, other_composition], &expected),
            Err(ShowSpoutValidationError::CompositionMismatch)
        );
        assert_eq!(
            decide_show_spout_ensure(&[expected.background.clone()], &expected),
            Err(ShowSpoutValidationError::PartialPair)
        );
    }

    #[test]
    fn black_frame_is_owned_fixed_size_and_reusable() {
        let frame = ShowSpoutBlackFrame::new();
        assert_eq!(frame.width(), SHOW_SPOUT_WIDTH);
        assert_eq!(frame.height(), SHOW_SPOUT_HEIGHT);
        assert_eq!(
            frame.as_rgba().len(),
            SHOW_SPOUT_FRAME_PIXEL_LEN * SHOW_SPOUT_BLACK_PIXEL_RGBA.len()
        );
        assert!(frame
            .as_rgba()
            .chunks_exact(4)
            .all(|pixel| pixel == SHOW_SPOUT_BLACK_PIXEL_RGBA.as_slice()));
        let first_ptr = frame.as_rgba().as_ptr();
        let second_ptr = frame.as_rgba().as_ptr();
        assert_eq!(first_ptr, second_ptr);
    }

    #[test]
    fn stopped_content_keeps_existing_sender_open_and_resume_is_live() {
        let creating = ShowSpoutPresentationState::initial();
        assert!(!creating.sender_remains_open());
        assert!(creating
            .apply(ShowSpoutPresentationEvent::ContentLive)
            .is_err());
        assert!(creating
            .apply(ShowSpoutPresentationEvent::ContentStopped)
            .is_err());
        let established = creating
            .apply(ShowSpoutPresentationEvent::SenderEstablished)
            .expect("sender establishment is required before content");
        assert!(established.sender_remains_open());
        let existing = ShowSpoutPresentationState::transitioning_with_sender();
        assert!(existing.sender_remains_open());
        let stopped = existing
            .apply(ShowSpoutPresentationEvent::ContentStopped)
            .expect("stopped content is valid");
        assert_eq!(stopped, ShowSpoutPresentationState::KeepAliveBlack);
        assert!(stopped.sender_remains_open());
        assert!(stopped.sends_black_frame());
        let live = stopped
            .apply(ShowSpoutPresentationEvent::ContentLive)
            .expect("resumed content is valid");
        assert_eq!(live, ShowSpoutPresentationState::Live);
        assert!(live.sender_remains_open());
        assert!(!live.sends_black_frame());
    }

    #[test]
    fn fault_requires_explicit_authority_restored_before_new_content() {
        let established = ShowSpoutPresentationState::initial()
            .apply(ShowSpoutPresentationEvent::SenderEstablished)
            .expect("sender establishment is valid");
        let live = established
            .apply(ShowSpoutPresentationEvent::ContentLive)
            .expect("live content is valid");
        let fault = live
            .apply(ShowSpoutPresentationEvent::AuthorityLost)
            .expect("authority loss is visible");
        assert_eq!(fault, ShowSpoutPresentationState::Fault);
        assert!(!fault.sender_remains_open());
        assert!(fault
            .apply(ShowSpoutPresentationEvent::BeginTransition)
            .is_err());
        assert!(fault
            .apply(ShowSpoutPresentationEvent::ContentLive)
            .is_err());
        let recovering = fault
            .apply(ShowSpoutPresentationEvent::AuthorityRestored)
            .expect("authority restoration is explicit");
        assert_eq!(
            recovering,
            ShowSpoutPresentationState::Transitioning { sender_open: false }
        );
        assert!(recovering
            .apply(ShowSpoutPresentationEvent::ContentLive)
            .is_err());
        let reestablished = recovering
            .apply(ShowSpoutPresentationEvent::SenderEstablished)
            .expect("recovered authority still requires sender establishment");
        assert!(reestablished
            .apply(ShowSpoutPresentationEvent::ContentLive)
            .is_ok());
    }
}
