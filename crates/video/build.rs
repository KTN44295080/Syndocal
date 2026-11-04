//! Detects whether the linked FFmpeg has `sws_scale_frame` (5.0 or later).
//! `ffmpeg-sys-next` publishes its detected versions to direct dependents.
fn main() {
    println!("cargo:rustc-check-cfg=cfg(syndocal_sws_frame)");
    println!("cargo:rerun-if-env-changed=DEP_FFMPEG_FFMPEG_5_0");
    if std::env::var("DEP_FFMPEG_FFMPEG_5_0").as_deref() == Ok("true") {
        println!("cargo:rustc-cfg=syndocal_sws_frame");
    }
}
