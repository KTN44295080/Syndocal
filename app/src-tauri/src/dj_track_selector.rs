//! Deterministic authored DJ track selector resolution.
//!
//! The resolver intentionally reports an overlap instead of selecting a first
//! mapping.  Admission is a show-control authority boundary, so matching two
//! authored selectors is an error rather than an ordering policy.

use protocol::{DjLinkTrackPayload, DjTrackSelector, DjTrackTriggerMapping};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DjTrackMappingResolution {
    NoMapping,
    Unique(DjTrackTriggerMapping),
    Ambiguous,
}

pub(super) fn resolve_track_mapping(
    mappings: &[DjTrackTriggerMapping],
    payload: &DjLinkTrackPayload,
) -> DjTrackMappingResolution {
    let payload_key = payload_selector_key(payload);
    let normalized_title = payload_title_contains_key(payload);
    let mut primary_matches = mappings.iter().filter(|mapping| {
        let selector_key = mapping.selector.canonical_key().ok();
        let exact_match = payload_key
            .as_deref()
            .zip(selector_key.as_deref())
            .is_some_and(|(payload_key, selector_key)| payload_key == selector_key);
        if exact_match {
            return true;
        }

        if let Some(title_key) = normalized_title.as_deref() {
            return mapping
                .selector
                .title_contains
                .as_ref()
                .and_then(|_| selector_key.as_deref())
                .and_then(|key| key.strip_prefix("title_contains:"))
                .is_some_and(|needle| title_key.contains(needle));
        }

        false
    });
    if let Some(first) = primary_matches.next() {
        return if primary_matches.next().is_some() {
            DjTrackMappingResolution::Ambiguous
        } else {
            DjTrackMappingResolution::Unique(first.clone())
        };
    }

    let mut fallback_matches = mappings.iter().filter(|mapping| {
        mapping.selector.title_contains.is_some()
            && mapping.selector.fallback_deck == Some(payload.deck)
    });
    let Some(first) = fallback_matches.next() else {
        return DjTrackMappingResolution::NoMapping;
    };
    if fallback_matches.next().is_some() {
        DjTrackMappingResolution::Ambiguous
    } else {
        DjTrackMappingResolution::Unique(first.clone())
    }
}

fn payload_selector_key(payload: &DjLinkTrackPayload) -> Option<String> {
    let selector = DjTrackSelector {
        content_id: payload.content_id.clone(),
        title: payload.title.clone(),
        artist: payload.artist.clone(),
        title_contains: None,
        fallback_deck: None,
    };
    selector.canonical_key().ok()
}

/// Uses the protocol selector's same trim-and-NFC canonicalization without a
/// second Unicode policy in the app crate.
fn payload_title_contains_key(payload: &DjLinkTrackPayload) -> Option<String> {
    let selector = DjTrackSelector {
        content_id: None,
        title: None,
        artist: None,
        title_contains: payload.title.clone(),
        fallback_deck: None,
    };
    selector
        .canonical_key()
        .ok()?
        .strip_prefix("title_contains:")
        .map(ToOwned::to_owned)
}
