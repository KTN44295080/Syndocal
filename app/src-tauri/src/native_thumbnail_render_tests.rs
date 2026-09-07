use super::*;

#[test]
fn thumbnail_render_cancellation_preserves_private_copy_and_normal_pixels() {
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("icons")
        .join("32x32.png");
    let prepared = prepare_one_local_media_asset(
        0,
        VideoSourceKind::StillImage,
        source_path.to_string_lossy().into_owned(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let asset = MediaAssetSummary {
        id: 74,
        label: "thumbnail-cancel-still".into(),
        source: prepared.source.clone(),
        content_hash: Some(prepared.content_hash),
        byte_size: Some(prepared.byte_size),
    };
    let private_copy = create_media_asset_preview_private_copy(&asset).unwrap();
    let private_path = private_copy.path.clone();
    let ordinary =
        render_media_asset_preview_from_private_copy(&asset, &private_copy, 0, 4, 4).unwrap();
    let cancellable = render_media_asset_preview_from_private_copy_with_cancellation(
        &asset,
        &private_copy,
        0,
        4,
        4,
        Some(&|| false),
    )
    .unwrap();
    assert_eq!(cancellable, ordinary);
    let error = render_media_asset_preview_from_private_copy_with_cancellation(
        &asset,
        &private_copy,
        0,
        4,
        4,
        Some(&|| true),
    )
    .unwrap_err();
    assert!(error.contains("Cancelled"));
    assert!(
        private_path.is_file(),
        "cancellation must not abandon the owner's private copy"
    );
    drop(private_copy);
    assert!(!private_path.exists());
}
