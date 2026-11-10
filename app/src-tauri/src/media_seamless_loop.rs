//! Derived "seamless loop" media: a non-looping clip is re-encoded so its last
//! `crossfade` seconds dissolve into its first ones. Looping the result has no
//! visible jump. The source file is never modified; the derived file is written
//! next to it (or reported as an error) and then imported like any other file.
//!
//! Layout for a clip of length `L` and crossfade `F` (output length `L - F`):
//! `source[F .. L-F]` followed by `xfade(source[L-F .. L] -> source[0 .. F])`.
//! The output ends on `source[F]`'s predecessor and starts on `source[F]`, so
//! the wrap continues the motion instead of cutting.
use protocol::hidden_console_command::hidden_command;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

pub(crate) const MIN_CROSSFADE_MS: u64 = 100;
pub(crate) const MAX_CROSSFADE_MS: u64 = 10_000;
/// The part of the clip that plays unblended must be at least this long.
const MIN_PLAIN_MS: u64 = 100;

fn seconds(ms: u64) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000)
}

/// The FFmpeg filter graph for a seamless loop, validated against the clip
/// length. Fails for crossfades the clip cannot hold.
pub(crate) fn seamless_loop_filter(duration_ms: u64, crossfade_ms: u64) -> Result<String, String> {
    if !(MIN_CROSSFADE_MS..=MAX_CROSSFADE_MS).contains(&crossfade_ms) {
        return Err(format!(
            "Seamless loop crossfade must be {}–{} ms",
            MIN_CROSSFADE_MS, MAX_CROSSFADE_MS
        ));
    }
    let needed = crossfade_ms
        .checked_mul(2)
        .and_then(|value| value.checked_add(MIN_PLAIN_MS))
        .ok_or_else(|| "Seamless loop crossfade is too long".to_string())?;
    if duration_ms < needed {
        return Err(format!(
            "The clip is {:.1} s; a {:.1} s crossfade needs at least {:.1} s",
            duration_ms as f64 / 1000.0,
            crossfade_ms as f64 / 1000.0,
            needed as f64 / 1000.0
        ));
    }
    let fade = seconds(crossfade_ms);
    let end = seconds(duration_ms);
    let offset = seconds(duration_ms - 2 * crossfade_ms);
    Ok(format!(
        "[0:v]split[body][head];\
         [body]trim=start={fade}:end={end},setpts=PTS-STARTPTS[tail];\
         [head]trim=start=0:end={fade},setpts=PTS-STARTPTS[lead];\
         [tail][lead]xfade=transition=fade:duration={fade}:offset={offset},format=yuv420p[out]"
    ))
}

/// `clip.mov` -> `clip_seamless_1000ms.mp4` beside the source, never an
/// existing file (a numeric suffix is added instead).
pub(crate) fn seamless_loop_output_path(
    source: &Path,
    crossfade_ms: u64,
) -> Result<PathBuf, String> {
    let directory = source
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            format!(
                "'{}' has no folder to write the loop into",
                source.display()
            )
        })?;
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| format!("'{}' has no usable file name", source.display()))?;
    for index in 0..1000u32 {
        let name = if index == 0 {
            format!("{stem}_seamless_{crossfade_ms}ms.mp4")
        } else {
            format!("{stem}_seamless_{crossfade_ms}ms_{index}.mp4")
        };
        let candidate = directory.join(name);
        if !candidate.exists() && !partial_path(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err("Too many seamless loop files already exist next to this clip".to_string())
}

fn partial_path(output: &Path) -> PathBuf {
    let mut name = output.file_name().map(OsString::from).unwrap_or_default();
    name.push(".partial");
    output.with_file_name(name)
}

pub(crate) fn seamless_loop_ffmpeg_args(
    input: &Path,
    partial_output: &Path,
    filter: &str,
    encoder: &str,
) -> Vec<OsString> {
    let mut args: Vec<OsString> = ["-hide_banner", "-nostdin", "-v", "error", "-n", "-i"]
        .iter()
        .map(OsString::from)
        .collect();
    args.push(input.as_os_str().to_owned());
    for arg in [
        "-filter_complex",
        filter,
        "-map",
        "[out]",
        "-an",
        "-c:v",
        encoder,
        "-b:v",
        "24M",
        "-maxrate",
        "32M",
        "-bufsize",
        "48M",
        "-g",
        "30",
        "-movflags",
        "+faststart",
        "-f",
        "mp4",
    ] {
        args.push(OsString::from(arg));
    }
    args.push(partial_output.as_os_str().to_owned());
    args
}

/// Render the loop and return the final path. Blocking; call off the event loop.
pub(crate) fn render_seamless_loop(
    source: &Path,
    duration_ms: u64,
    crossfade_ms: u64,
) -> Result<PathBuf, String> {
    let filter = seamless_loop_filter(duration_ms, crossfade_ms)?;
    if !source.is_file() {
        return Err(format!("'{}' is not available", source.display()));
    }
    let output = seamless_loop_output_path(source, crossfade_ms)?;
    let partial = partial_path(&output);
    let stop = AtomicBool::new(false);
    let (ffmpeg, encoder) = crate::video_recording::select_lgpl_h264_encoder(&stop)
        .map_err(|error| error.replace("Recording unavailable", "Seamless loop unavailable"))?;
    let mut child = hidden_command(&ffmpeg)
        .args(seamless_loop_ffmpeg_args(
            source, &partial, &filter, encoder,
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Unable to start FFmpeg for the seamless loop: {error}"))?;
    // Generous bound: hardware encoders run far faster than real time.
    let limit = Duration::from_millis(duration_ms.saturating_mul(4)).max(Duration::from_secs(60));
    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&partial);
                return Err("FFmpeg took too long to render the seamless loop".to_string());
            }
            Err(error) => {
                let _ = child.kill();
                let _ = std::fs::remove_file(&partial);
                return Err(format!("FFmpeg seamless loop wait failed: {error}"));
            }
        }
    };
    if !status.success() {
        let mut detail = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            use std::io::Read;
            let _ = stderr.read_to_string(&mut detail);
        }
        let _ = std::fs::remove_file(&partial);
        let detail = detail.lines().last().unwrap_or("").trim().to_string();
        return Err(format!(
            "FFmpeg could not render the seamless loop ({status}){}",
            if detail.is_empty() {
                String::new()
            } else {
                format!(": {detail}")
            }
        ));
    }
    std::fs::rename(&partial, &output).map_err(|error| {
        let _ = std::fs::remove_file(&partial);
        format!("Unable to finish '{}': {error}", output.display())
    })?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_places_the_crossfade_at_the_wrap() {
        let filter = seamless_loop_filter(8_000, 1_000).unwrap();
        assert!(filter.contains("trim=start=1.000:end=8.000"), "{filter}");
        assert!(filter.contains("trim=start=0:end=1.000"), "{filter}");
        assert!(
            filter.contains("xfade=transition=fade:duration=1.000:offset=6.000"),
            "{filter}"
        );
    }

    #[test]
    fn filter_rejects_crossfades_the_clip_cannot_hold() {
        assert!(seamless_loop_filter(8_000, 50).is_err());
        assert!(seamless_loop_filter(8_000, 20_000).is_err());
        assert!(seamless_loop_filter(2_000, 1_000).is_err());
        assert!(seamless_loop_filter(2_100, 1_000).is_ok());
    }

    #[test]
    fn output_path_never_replaces_an_existing_file() {
        let directory =
            std::env::temp_dir().join(format!("syndocal-seamless-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let source = directory.join("clip.mov");
        let first = seamless_loop_output_path(&source, 500).unwrap();
        assert_eq!(first.file_name().unwrap(), "clip_seamless_500ms.mp4");
        std::fs::write(&first, b"x").unwrap();
        let second = seamless_loop_output_path(&source, 500).unwrap();
        assert_eq!(second.file_name().unwrap(), "clip_seamless_500ms_1.mp4");
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn args_never_overwrite_and_drop_audio() {
        let args = seamless_loop_ffmpeg_args(
            Path::new("in.mp4"),
            Path::new("out.mp4.partial"),
            "f",
            "h264_mf",
        );
        let args: Vec<String> = args
            .into_iter()
            .map(|arg| arg.into_string().unwrap())
            .collect();
        assert!(args.contains(&"-n".to_string()));
        assert!(args.contains(&"-an".to_string()));
        assert_eq!(args.last().unwrap(), "out.mp4.partial");
    }
}
