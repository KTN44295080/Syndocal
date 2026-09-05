//! D3D11VA codec-context ownership and hardware-frame transfer.
//!
//! Follows FFmpeg's hw_decode example: configure before avcodec_open2 and
//! transfer hardware frames before CPU scaling. AVCodecContext owns the device
//! reference; no raw pointer is stored in a Send Rust wrapper or global cache.
use super::{ffmpeg, VideoDecodeError, VideoFrameRequest};

pub(super) fn classify(
    request: &VideoFrameRequest,
    hardware: bool,
    error: VideoDecodeError,
) -> VideoDecodeError {
    if hardware && !matches!(error, VideoDecodeError::HardwareDecode { .. }) {
        failure(request, format!("{error:?}"))
    } else {
        error
    }
}

fn failure(request: &VideoFrameRequest, message: impl Into<String>) -> VideoDecodeError {
    VideoDecodeError::HardwareDecode {
        layer_id: request.layer_id,
        label: request.label.clone(),
        message: format!("D3D11VA: {}", message.into()),
    }
}

pub(super) fn configure(
    context: &mut ffmpeg::codec::context::Context,
    request: &VideoFrameRequest,
    enabled: bool,
) -> Result<bool, VideoDecodeError> {
    #[cfg(target_os = "windows")]
    {
        if enabled
            && matches!(
                context.id(),
                ffmpeg::codec::Id::H264 | ffmpeg::codec::Id::HEVC
            )
        {
            windows::configure(context, request)?;
            return Ok(true);
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = (context, request, enabled);
    Ok(false)
}

pub(super) fn transfer(
    request: &VideoFrameRequest,
    hardware: bool,
    decoded: &ffmpeg::util::frame::video::Video,
) -> Result<Option<ffmpeg::util::frame::video::Video>, VideoDecodeError> {
    if !hardware {
        return Ok(None);
    }
    #[cfg(target_os = "windows")]
    {
        use ffmpeg::ffi;
        // A configured device alone does not prove hardware decode. Reject a
        // software frame if negotiation ever deviates from the selected backend.
        if unsafe { (*decoded.as_ptr()).format } != ffi::AVPixelFormat::AV_PIX_FMT_D3D11 as i32 {
            return Err(failure(request, "decoder returned a non-D3D11 frame"));
        }
        let mut software = ffmpeg::util::frame::video::Video::empty();
        // SAFETY: both frames are owned, live AVFrames. FFmpeg allocates the
        // destination's software buffers; Video releases them on every exit.
        let result =
            unsafe { ffi::av_hwframe_transfer_data(software.as_mut_ptr(), decoded.as_ptr(), 0) };
        if result < 0 {
            return Err(failure(
                request,
                format!("frame transfer failed: {}", ffmpeg::Error::from(result)),
            ));
        }
        Ok(Some(software))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = decoded;
        Err(failure(
            request,
            "hardware backend unavailable on this platform",
        ))
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use ffmpeg::ffi;
    use std::{
        ffi::{CString, OsString},
        ptr,
    };

    const ADAPTER_ENV: &str = "SYNDOCAL_D3D11VA_ADAPTER";

    fn adapter(value: Option<OsString>) -> Result<Option<CString>, String> {
        let Some(value) = value else {
            return Ok(None);
        };
        let value = value
            .into_string()
            .map_err(|_| format!("{ADAPTER_ENV} must be an ASCII adapter index"))?;
        if value.is_empty()
            || !value.bytes().all(|b| b.is_ascii_digit())
            || value.parse::<i32>().is_err()
        {
            return Err(format!(
                "{ADAPTER_ENV} must be a non-negative decimal adapter index (0..2147483647)"
            ));
        }
        CString::new(value)
            .map(Some)
            .map_err(|error| error.to_string())
    }

    fn validate_adapter(
        device_name: &CString,
        request: &VideoFrameRequest,
    ) -> Result<(), VideoDecodeError> {
        use ::windows::Win32::Graphics::Dxgi::{CreateDXGIFactory, IDXGIFactory};
        let index = device_name
            .to_str()
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| failure(request, "adapter index invariant violated"))?;
        // FFmpeg's D3D11VA path can continue with a null adapter when its
        // EnumAdapters fails, silently selecting the system default. Validate
        // explicit indices first, using the same DXGI enumeration API.
        // SAFETY: Windows returns reference-counted interfaces owned by these
        // local wrappers. They are released before returning, on every path.
        let factory: IDXGIFactory = unsafe { CreateDXGIFactory() }.map_err(|error| {
            failure(request, format!("cannot enumerate DXGI adapters: {error}"))
        })?;
        let adapter = unsafe { factory.EnumAdapters(index) }.map_err(|error| {
            failure(
                request,
                format!("requested DXGI adapter index {index} is unavailable: {error}"),
            )
        })?;
        let description = unsafe { adapter.GetDesc() }.map_err(|error| {
            failure(
                request,
                format!("cannot identify DXGI adapter index {index}: {error}"),
            )
        })?;
        let length = description
            .Description
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(description.Description.len());
        let name = String::from_utf16_lossy(&description.Description[..length]);
        // This identifies the validated selection, not an atomic identity
        // guarantee across hot-plug and FFmpeg's subsequent device creation.
        eprintln!("D3D11VA validated adapter index={index} name={name} vendor=0x{:04x} luid={:08x}:{:08x}",
            description.VendorId, description.AdapterLuid.HighPart as u32, description.AdapterLuid.LowPart);
        Ok(())
    }

    // FFmpeg calls this with a sentinel-terminated format list. Stateless and
    // panic-free: never negotiate software as a fallback after selecting HW.
    unsafe extern "C" fn get_format(
        _context: *mut ffi::AVCodecContext,
        mut formats: *const ffi::AVPixelFormat,
    ) -> ffi::AVPixelFormat {
        if formats.is_null() {
            return ffi::AVPixelFormat::AV_PIX_FMT_NONE;
        }
        while *formats != ffi::AVPixelFormat::AV_PIX_FMT_NONE {
            if *formats == ffi::AVPixelFormat::AV_PIX_FMT_D3D11 {
                return *formats;
            }
            formats = formats.add(1);
        }
        ffi::AVPixelFormat::AV_PIX_FMT_NONE
    }

    pub(super) fn configure(
        context: &mut ffmpeg::codec::context::Context,
        request: &VideoFrameRequest,
    ) -> Result<(), VideoDecodeError> {
        let device_name =
            adapter(std::env::var_os(ADAPTER_ENV)).map_err(|error| failure(request, error))?;
        if let Some(device_name) = &device_name {
            validate_adapter(device_name, request)?;
        }
        let codec = ffmpeg::decoder::find(context.id())
            .ok_or_else(|| failure(request, "codec not found"))?;
        // SAFETY: codec is FFmpeg's static descriptor; configurations remain
        // valid for its lifetime. A null descriptor terminates the enumeration.
        let supported = unsafe {
            let mut index = 0;
            loop {
                let config = ffi::avcodec_get_hw_config(codec.as_ptr(), index);
                if config.is_null() {
                    break false;
                }
                if (*config).device_type == ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_D3D11VA
                    && (*config).pix_fmt == ffi::AVPixelFormat::AV_PIX_FMT_D3D11
                    && ((*config).methods & ffi::AV_CODEC_HW_CONFIG_METHOD_HW_DEVICE_CTX as i32)
                        != 0
                {
                    break true;
                }
                index += 1;
            }
        };
        if !supported {
            return Err(failure(
                request,
                "codec has no D3D11VA hardware configuration",
            ));
        }
        let mut device = ptr::null_mut();
        // FFmpeg initializes the D3D11 device/context's internal locking. Its
        // numeric device string selects a DXGI adapter; null uses system default.
        let result = unsafe {
            ffi::av_hwdevice_ctx_create(
                &mut device,
                ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_D3D11VA,
                device_name
                    .as_ref()
                    .map_or(ptr::null(), |name| name.as_ptr()),
                ptr::null_mut(),
                0,
            )
        };
        if result < 0 {
            unsafe {
                ffi::av_buffer_unref(&mut device);
            }
            return Err(failure(
                request,
                format!(
                    "device creation failed (adapter {}): {}",
                    device_name
                        .as_ref()
                        .map_or("system default", |name| name.to_str().unwrap_or("invalid")),
                    ffmpeg::Error::from(result)
                ),
            ));
        }
        if device.is_null() {
            return Err(failure(request, "device creation returned no device"));
        }
        // SAFETY: this owned, unopened context has no device yet. Ownership of
        // the one reference is transferred, not duplicated. avcodec_free_context
        // releases it even if opening the decoder fails. No borrowed callback
        // state survives this call, and session decode remains exclusively held.
        unsafe {
            (*context.as_mut_ptr()).hw_device_ctx = device;
            (*context.as_mut_ptr()).get_format = Some(get_format);
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn adapter_selection_is_strict_and_default_is_explicit() {
            assert!(adapter(None).unwrap().is_none());
            for valid in ["0", "1", "0002", "2147483647"] {
                assert_eq!(
                    adapter(Some(valid.into()))
                        .unwrap()
                        .unwrap()
                        .to_str()
                        .unwrap(),
                    valid
                );
            }
            for invalid in ["", "-1", "+1", " 0", "0 ", "1.0", "2147483648", "NVIDIA"] {
                assert!(adapter(Some(invalid.into())).is_err(), "{invalid}");
            }
        }

        #[test]
        fn format_negotiation_never_accepts_software() {
            use ffi::AVPixelFormat::*;
            let available = [AV_PIX_FMT_YUV420P, AV_PIX_FMT_D3D11, AV_PIX_FMT_NONE];
            let software = [AV_PIX_FMT_YUV420P, AV_PIX_FMT_NONE];
            unsafe {
                assert_eq!(
                    get_format(ptr::null_mut(), available.as_ptr()),
                    AV_PIX_FMT_D3D11
                );
                assert_eq!(
                    get_format(ptr::null_mut(), software.as_ptr()),
                    AV_PIX_FMT_NONE
                );
                assert_eq!(get_format(ptr::null_mut(), ptr::null()), AV_PIX_FMT_NONE);
            }
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod integration_tests {
    use super::*;
    use crate::{LibavFrameDecoder, VideoFrameDecoder};
    use protocol::{VideoSourceKind, VideoSourceSummary};

    fn request(path: String) -> VideoFrameRequest {
        VideoFrameRequest {
            layer_id: 9001,
            label: "D3D11VA acceptance fixture".into(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(path),
                name: None,
                codec: Some("h264".into()),
                metadata: None,
            },
            position_ms: 0,
            width: 320,
            height: 128,
        }
    }

    #[test]
    fn selected_hardware_errors_remain_fail_closed() {
        let request = request("unused.mp4".into());
        let error = super::super::decode_message(&request, "receive failed");
        assert_eq!(classify(&request, false, error.clone()), error);
        let hardware = classify(&request, true, error);
        assert!(matches!(hardware, VideoDecodeError::HardwareDecode { .. }));
        assert_eq!(classify(&request, true, hardware.clone()), hardware);
        let software_frame = ffmpeg::util::frame::video::Video::empty();
        assert!(matches!(
            transfer(&request, true, &software_frame),
            Err(VideoDecodeError::HardwareDecode { .. })
        ));
        assert!(transfer(&request, false, &software_frame)
            .unwrap()
            .is_none());
    }

    /// Run in a separate process with an impossible, valid numeric adapter.
    /// Never mutates global environment while other tests may be decoding.
    #[test]
    #[ignore = "requires SYNDOCAL_GPU_TEST_VIDEO and SYNDOCAL_D3D11VA_ADAPTER=2147483647"]
    fn d3d11va_invalid_adapter_rejects_cli_fallback() {
        use crate::{FfmpegCliFrameDecoder, PreferredVideoFrameDecoder};
        assert_eq!(
            std::env::var("SYNDOCAL_D3D11VA_ADAPTER").as_deref(),
            Ok("2147483647")
        );
        let request =
            request(std::env::var("SYNDOCAL_GPU_TEST_VIDEO").expect("set an H264/HEVC video path"));
        let missing_cli =
            std::env::temp_dir().join("syndocal-hardware-fail-closed-missing-ffmpeg.exe");
        let mut preferred =
            PreferredVideoFrameDecoder::new(FfmpegCliFrameDecoder::new(missing_cli));
        let error = match preferred.decode_frame(&request) {
            Err(error) => error,
            Ok(_) => panic!("invalid GPU adapter unexpectedly decoded a frame"),
        };
        assert!(
            matches!(error, VideoDecodeError::HardwareDecode { .. }),
            "{error:?}"
        );
        let diagnostics = preferred.diagnostics();
        assert_eq!(diagnostics.cli_fallback_requests, 0);
        assert_eq!(diagnostics.libav_hardware_session_count, 0);
        assert_eq!(diagnostics.libav_hardware_frame_count, 0);
        assert_eq!(diagnostics.libav_hardware_error_count, 1);
    }

    /// Explicit opt-in real-device acceptance. The supplied H264/HEVC fixture
    /// must be longer than three seconds. No UI/output devices are opened.
    #[test]
    #[ignore = "requires Windows D3D11VA GPU and SYNDOCAL_GPU_TEST_VIDEO"]
    fn d3d11va_real_video_frames_seek_reuse_and_teardown() {
        let path = std::env::var("SYNDOCAL_GPU_TEST_VIDEO").expect("set an H264/HEVC video path");
        let mut request = request(path);
        let mut gpu = LibavFrameDecoder::new();
        let mut cpu = LibavFrameDecoder::software_for_tests();
        let started = std::time::Instant::now();
        for position in [0, 34, 67, 100, 134, 167, 200, 2000, 2034, 2067, 0, 34] {
            request.position_ms = position;
            let actual = gpu
                .decode_frame(&request)
                .expect("D3D11VA frame")
                .expect("video frame");
            let expected = cpu
                .decode_frame(&request)
                .expect("software oracle")
                .expect("video frame");
            assert_eq!(
                (
                    actual.width,
                    actual.height,
                    actual.pts_ms,
                    actual.duration_ms,
                    actual.format
                ),
                (
                    expected.width,
                    expected.height,
                    expected.pts_ms,
                    expected.duration_ms,
                    expected.format
                )
            );
            assert_eq!(actual.data.len(), expected.data.len());
            // GPU NV12 and software planar YUV use different swscale kernels.
            // Require bounded pixel parity, not merely a successful GPU init.
            let mut total = 0u64;
            let mut maximum = 0u8;
            for (index, (actual, expected)) in actual.data.iter().zip(&expected.data).enumerate() {
                let delta = actual.abs_diff(*expected);
                if index % 4 == 3 {
                    assert_eq!(delta, 0);
                } else {
                    total += u64::from(delta);
                    maximum = maximum.max(delta);
                }
            }
            let mean = total as f64 / (actual.data.len() / 4 * 3) as f64;
            assert!(
                maximum <= 3 && mean <= 0.5,
                "position {position}: max={maximum}, mean={mean}"
            );
            let before_repeat = gpu.session_diagnostics();
            let repeat = gpu.decode_frame(&request).unwrap().unwrap();
            assert_eq!(repeat.data, actual.data);
            assert_eq!(
                gpu.session_diagnostics().hardware_frames,
                before_repeat.hardware_frames
            );
            assert_eq!(gpu.session_diagnostics().hardware_sessions, 1);
        }
        let diagnostics = gpu.session_diagnostics();
        assert!(diagnostics.hardware_frames >= 10, "{diagnostics:?}");
        assert!(
            diagnostics.sequential_continues > 0 && diagnostics.resets >= 2,
            "{diagnostics:?}"
        );
        assert_eq!(diagnostics.hardware_errors, 0);
        eprintln!(
            "D3D11VA real frames (GPU + software parity, not decode-only benchmark): {:?}, {:?}",
            started.elapsed(),
            diagnostics
        );
        gpu.release_layer(request.layer_id);
        assert_eq!(gpu.session_diagnostics().hardware_sessions, 0);
        assert_eq!(gpu.cache_len(), 0);
        let reopened = gpu.decode_frame(&request).unwrap().unwrap();
        assert!(!reopened.data.is_empty());
        assert_eq!(gpu.session_diagnostics().hardware_sessions, 1);
        drop(gpu);
    }
}
