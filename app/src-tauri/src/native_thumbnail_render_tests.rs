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

// Successful real-file decoding; no AppState, operator storage or device is opened.
const COLORS_A: [[u8; 3]; 4] = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]];
const COLORS_B: [[u8; 3]; 4] = [[255, 255, 0], [0, 255, 255], [255, 0, 255], [80, 80, 80]];

struct OwnedThumbnailFixture(PathBuf);
impl OwnedThumbnailFixture {
    fn create() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = env::temp_dir().join(format!(
            "syndocal-thumbnail-proof-{}-{}-{}",
            std::process::id(),
            current_unix_ms(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn write_png(&self, name: &str, colors: [[u8; 3]; 4]) -> PathBuf {
        let path = self.0.join(name);
        image::RgbImage::from_fn(96, 64, |x, y| {
            image::Rgb(colors[usize::from(y >= 32) * 2 + usize::from(x >= 48)])
        })
        .save(&path)
        .unwrap();
        path
    }
}
impl Drop for OwnedThumbnailFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("owned fixture cleanup failed");
    }
}

fn fixture_asset(path: &Path, kind: VideoSourceKind, id: u64) -> MediaAssetSummary {
    let prepared = prepare_one_local_media_asset(
        0,
        kind,
        path.to_string_lossy().into_owned(),
        &AtomicBool::new(false),
    )
    .unwrap();
    MediaAssetSummary {
        id,
        label: "real-file-proof".into(),
        source: prepared.source,
        content_hash: Some(prepared.content_hash),
        byte_size: Some(prepared.byte_size),
    }
}

fn assert_quadrants(frame: &video::VideoFrame, colors: [[u8; 3]; 4], tolerance: u8) {
    assert_eq!(frame.format, video::VideoPixelFormat::Rgba8);
    assert_eq!(frame.data.len(), (frame.width * frame.height * 4) as usize);
    for (index, color) in colors.iter().enumerate() {
        let x = frame.width * (if index % 2 == 0 { 1 } else { 3 }) / 4;
        let y = frame.height * (if index < 2 { 1 } else { 3 }) / 4;
        let at = ((y * frame.width + x) * 4) as usize;
        for channel in 0..3 {
            assert!(
                frame.data[at + channel].abs_diff(color[channel]) <= tolerance,
                "quadrant {index} channel {channel}: {:?} expected {color:?}",
                &frame.data[at..at + 4]
            );
        }
        assert_eq!(frame.data[at + 3], 255, "opaque source must stay opaque");
    }
}

#[test]
fn real_file_thumbnail_png_pixels_dimensions_and_repeat() {
    let fixture = OwnedThumbnailFixture::create();
    let path = fixture.write_png("four colors.png", COLORS_A);
    let original = fs::read(&path).unwrap();
    let asset = fixture_asset(&path, VideoSourceKind::StillImage, 75);
    for (width, height) in [(96, 64), (48, 32)] {
        let frame =
            render_media_asset_thumbnail(asset.clone(), 0, width, height, &AtomicBool::new(false))
                .unwrap();
        assert_eq!((frame.width, frame.height), (width, height));
        assert_quadrants(&frame, COLORS_A, 0);
        let repeated =
            render_media_asset_thumbnail(asset.clone(), 0, width, height, &AtomicBool::new(false))
                .unwrap();
        assert_eq!(
            frame, repeated,
            "repeat must decode the same source, not a placeholder"
        );
    }
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn real_file_thumbnail_video_observes_actual_frame_position() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-assets/thumbnail-two-patterns.mp4");
    let asset = fixture_asset(&path, VideoSourceKind::File, 76);
    for (position, expected) in [(0, COLORS_A), (500, COLORS_B)] {
        let frame =
            render_media_asset_thumbnail(asset.clone(), position, 48, 32, &AtomicBool::new(false))
                .unwrap();
        assert_eq!((frame.width, frame.height), (48, 32));
        assert_quadrants(&frame, expected, 5);
    }
}

#[test]
fn real_file_thumbnail_copy_isolation_and_catalog_replacement() {
    let fixture = OwnedThumbnailFixture::create();
    let path = fixture.write_png("replace.png", COLORS_A);
    let old_asset = fixture_asset(&path, VideoSourceKind::StillImage, 77);
    let copy = create_media_asset_preview_private_copy(&old_asset).unwrap();
    let copy_path = copy.path.clone();
    fixture.write_png("replace.png", COLORS_B);
    let preserved =
        render_media_asset_preview_from_private_copy(&old_asset, &copy, 0, 48, 32).unwrap();
    assert_quadrants(&preserved, COLORS_A, 0);
    let stale = render_media_asset_thumbnail(old_asset.clone(), 0, 48, 32, &AtomicBool::new(false))
        .unwrap_err();
    assert!(stale.contains("catalog identity"), "wrong failure: {stale}");
    let replacement = fixture_asset(&path, VideoSourceKind::StillImage, old_asset.id);
    assert_ne!(replacement.content_hash, old_asset.content_hash);
    let current =
        render_media_asset_thumbnail(replacement, 0, 48, 32, &AtomicBool::new(false)).unwrap();
    assert_quadrants(&current, COLORS_B, 0);
    drop(copy);
    assert!(
        !copy_path.exists(),
        "owned private copy must be removed on release"
    );
}

#[test]
fn real_file_thumbnail_successful_native_worker_releases_admission() {
    let fixture = OwnedThumbnailFixture::create();
    let path = fixture.write_png("worker.png", COLORS_A);
    let asset = fixture_asset(&path, VideoSourceKind::StillImage, 78);
    let work = native_thumbnail_work::NativeThumbnailWork::default();
    for _ in 0..2 {
        let job = work.assets.try_acquire().unwrap();
        let source = asset.clone();
        let frame =
            tauri::async_runtime::block_on(native_thumbnail_dispatch::run(job, move |cancel| {
                render_media_asset_thumbnail(source, 0, 48, 32, cancel)
            }))
            .unwrap();
        assert_quadrants(&frame, COLORS_A, 0);
        drop(
            work.assets
                .try_acquire()
                .expect("successful worker retained admission"),
        );
    }
}
