//! Pure input bounds and private-source snapshot projection for media previews.
//! Callers own content verification, project authority, and private-copy lifetime.
use protocol::{MediaAssetSummary, VideoBlendMode, VideoLayerState, VideoSourceKind};
use std::path::Path;

pub(super) const MEDIA_ASSET_THUMBNAIL_MAX_EDGE: u32 = 512;
const MEDIA_ASSET_PREVIEW_MAX_POSITION_MS: u64 = 24 * 60 * 60 * 1_000;

pub(super) fn validate_media_asset_thumbnail_dimensions(width: u32, height: u32) -> Result<(), String> {
    if width == 0
        || height == 0
        || width > MEDIA_ASSET_THUMBNAIL_MAX_EDGE
        || height > MEDIA_ASSET_THUMBNAIL_MAX_EDGE
    {
        return Err(format!(
            "Media asset thumbnail dimensions must be between 1 and {MEDIA_ASSET_THUMBNAIL_MAX_EDGE} pixels"
        ));
    }
    Ok(())
}

pub(super) fn validate_media_asset_preview_position(position_ms: u64) -> Result<(), String> {
    if position_ms > MEDIA_ASSET_PREVIEW_MAX_POSITION_MS {
        return Err(format!(
            "Media asset preview position must not exceed {MEDIA_ASSET_PREVIEW_MAX_POSITION_MS} ms"
        ));
    }
    Ok(())
}

/// This is deliberately stricter than the generic catalog schema. Rendering a
/// thumbnail consumes a local file, so legacy entries without a persisted
/// byte/hash identity are not authoritative enough to race safely.
pub(super) fn validate_media_asset_thumbnail_contract(asset: &MediaAssetSummary) -> Result<(), String> {
    if !matches!(
        asset.source.kind,
        VideoSourceKind::File | VideoSourceKind::StillImage
    ) {
        return Err(
            "Media asset thumbnails are available for local File and Still Image sources only"
                .to_string(),
        );
    }
    if asset
        .source
        .path
        .as_deref()
        .is_none_or(|path| path.trim().is_empty())
    {
        return Err("Media asset thumbnail requires a local source path".to_string());
    }
    if asset.content_hash.is_none() || asset.byte_size.is_none() {
        return Err(
            "Media asset thumbnail requires the catalog's exact local content identity".to_string(),
        );
    }
    Ok(())
}

pub(super) fn media_asset_thumbnail_snapshot(
    asset: &MediaAssetSummary,
    private_path: &Path,
    position_ms: u64,
) -> protocol::VideoSnapshot {
    let mut snapshot = protocol::VideoSnapshot::default();
    let mut private_source = asset.source.clone();
    private_source.path = Some(private_path.to_string_lossy().into_owned());
    snapshot.layers.push(protocol::VideoLayerSummary {
        // The layer and its source are strictly ephemeral. Do not attach the
        // catalog ID: the renderer must decode only the private verified copy,
        // never the user-controlled catalog path again.
        id: asset.id,
        label: asset.label.clone(),
        source: private_source,
        media_asset_id: None,
        blend_mode: VideoBlendMode::Normal,
        state: VideoLayerState {
            position_ms,
            ..VideoLayerState::default()
        },
        isf_effect: None,
        clip_slots: Vec::new(),
        default_clip_slot_id: None,
    });
    snapshot
}

#[cfg(test)]
mod tests {
    use super::{validate_media_asset_preview_position, MEDIA_ASSET_PREVIEW_MAX_POSITION_MS};

    #[test]
    fn media_asset_preview_position_accepts_zero_and_inclusive_limit() {
        assert!(validate_media_asset_preview_position(0).is_ok());
        assert!(validate_media_asset_preview_position(MEDIA_ASSET_PREVIEW_MAX_POSITION_MS).is_ok());
    }

    #[test]
    fn media_asset_preview_position_rejects_above_limit_and_overflow_extreme() {
        for position_ms in [MEDIA_ASSET_PREVIEW_MAX_POSITION_MS + 1, u64::MAX] {
            let error = validate_media_asset_preview_position(position_ms).unwrap_err();
            assert_eq!(
                error,
                format!(
                    "Media asset preview position must not exceed {MEDIA_ASSET_PREVIEW_MAX_POSITION_MS} ms"
                )
            );
        }
    }
}
