//! Allocation-free ASIO native sample conversion for the v3 callback.

use std::ffi::c_void;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub(crate) enum NativeFormat {
    I16Le = 1,
    I16Be = 2,
    I24Le = 3,
    I24Be = 4,
    I32Le = 5,
    I32Be = 6,
    F32Le = 7,
    F32Be = 8,
    F64Le = 9,
    F64Be = 10,
}

impl NativeFormat {
    pub(crate) fn from_code(code: u32) -> Option<Self> {
        match code {
            1 => Some(Self::I16Le),
            2 => Some(Self::I16Be),
            3 => Some(Self::I24Le),
            4 => Some(Self::I24Be),
            5 => Some(Self::I32Le),
            6 => Some(Self::I32Be),
            7 => Some(Self::F32Le),
            8 => Some(Self::F32Be),
            9 => Some(Self::F64Le),
            10 => Some(Self::F64Be),
            _ => None,
        }
    }

    #[cfg(all(target_os = "windows", feature = "asio"))]
    pub(crate) fn request_code(label: &str) -> Option<u32> {
        match label {
            "i16" => Some(Self::I16Le as u32),
            "i24" => Some(Self::I24Le as u32),
            "i32" => Some(Self::I32Le as u32),
            "f32" => Some(Self::F32Le as u32),
            "f64" => Some(Self::F64Le as u32),
            _ => None,
        }
    }

    #[cfg(all(target_os = "windows", feature = "asio"))]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::I16Le | Self::I16Be => "i16",
            Self::I24Le | Self::I24Be => "i24",
            Self::I32Le | Self::I32Be => "i32",
            Self::F32Le | Self::F32Be => "f32",
            Self::F64Le | Self::F64Be => "f64",
        }
    }

    pub(crate) fn bytes(self) -> usize {
        match self {
            Self::I16Le | Self::I16Be => 2,
            Self::I24Le | Self::I24Be => 3,
            Self::I32Le | Self::I32Be | Self::F32Le | Self::F32Be => 4,
            Self::F64Le | Self::F64Be => 8,
        }
    }
}

fn decode_i24(bytes: [u8; 3], little_endian: bool) -> i32 {
    let ordered = if little_endian {
        bytes
    } else {
        [bytes[2], bytes[1], bytes[0]]
    };
    let raw = i32::from_le_bytes([ordered[0], ordered[1], ordered[2], 0]);
    if raw & 0x0080_0000 != 0 {
        raw | !0x00ff_ffff
    } else {
        raw
    }
}

fn encode_i24(value: i32, little_endian: bool) -> [u8; 3] {
    let bytes = value.to_le_bytes();
    if little_endian {
        [bytes[0], bytes[1], bytes[2]]
    } else {
        [bytes[2], bytes[1], bytes[0]]
    }
}

fn float_to_signed(value: f32, positive_max: f64, negative_scale: f64) -> i64 {
    let value = value.clamp(-1.0, 1.0) as f64;
    if value >= 0.0 {
        (value * positive_max).round() as i64
    } else {
        (value * negative_scale).round() as i64
    }
}

unsafe fn read_sample(pointer: *const u8, format: NativeFormat) -> f32 {
    match format {
        NativeFormat::I16Le => {
            let bytes = unsafe { [pointer.read(), pointer.add(1).read()] };
            i16::from_le_bytes(bytes) as f32 / 32768.0
        }
        NativeFormat::I16Be => {
            let bytes = unsafe { [pointer.read(), pointer.add(1).read()] };
            i16::from_be_bytes(bytes) as f32 / 32768.0
        }
        NativeFormat::I24Le | NativeFormat::I24Be => {
            let bytes = unsafe { [pointer.read(), pointer.add(1).read(), pointer.add(2).read()] };
            decode_i24(bytes, format == NativeFormat::I24Le) as f32 / 8_388_608.0
        }
        NativeFormat::I32Le => {
            let bytes = unsafe {
                [
                    pointer.read(),
                    pointer.add(1).read(),
                    pointer.add(2).read(),
                    pointer.add(3).read(),
                ]
            };
            (i32::from_le_bytes(bytes) as f64 / 2_147_483_648.0) as f32
        }
        NativeFormat::I32Be => {
            let bytes = unsafe {
                [
                    pointer.read(),
                    pointer.add(1).read(),
                    pointer.add(2).read(),
                    pointer.add(3).read(),
                ]
            };
            (i32::from_be_bytes(bytes) as f64 / 2_147_483_648.0) as f32
        }
        NativeFormat::F32Le | NativeFormat::F32Be => {
            let bytes = unsafe {
                [
                    pointer.read(),
                    pointer.add(1).read(),
                    pointer.add(2).read(),
                    pointer.add(3).read(),
                ]
            };
            let bits = if format == NativeFormat::F32Le {
                u32::from_le_bytes(bytes)
            } else {
                u32::from_be_bytes(bytes)
            };
            f32::from_bits(bits)
        }
        NativeFormat::F64Le | NativeFormat::F64Be => {
            let mut bytes = [0_u8; 8];
            for (index, byte) in bytes.iter_mut().enumerate() {
                *byte = unsafe { pointer.add(index).read() };
            }
            let bits = if format == NativeFormat::F64Le {
                u64::from_le_bytes(bytes)
            } else {
                u64::from_be_bytes(bytes)
            };
            f64::from_bits(bits) as f32
        }
    }
}

unsafe fn write_sample(pointer: *mut u8, format: NativeFormat, value: f32) {
    let bytes: [u8; 8];
    let slice: &[u8] = match format {
        NativeFormat::I16Le => {
            bytes = {
                let raw = float_to_signed(value, i16::MAX as f64, 32768.0) as i16;
                let value = raw.to_le_bytes();
                [value[0], value[1], 0, 0, 0, 0, 0, 0]
            };
            &bytes[..2]
        }
        NativeFormat::I16Be => {
            bytes = {
                let raw = float_to_signed(value, i16::MAX as f64, 32768.0) as i16;
                let value = raw.to_be_bytes();
                [value[0], value[1], 0, 0, 0, 0, 0, 0]
            };
            &bytes[..2]
        }
        NativeFormat::I24Le | NativeFormat::I24Be => {
            let raw = float_to_signed(value, 8_388_607.0, 8_388_608.0) as i32;
            let value = encode_i24(raw, format == NativeFormat::I24Le);
            bytes = [value[0], value[1], value[2], 0, 0, 0, 0, 0];
            &bytes[..3]
        }
        NativeFormat::I32Le => {
            let raw = float_to_signed(value, i32::MAX as f64, 2_147_483_648.0) as i32;
            let value = raw.to_le_bytes();
            unsafe {
                std::ptr::copy_nonoverlapping(value.as_ptr(), pointer, 4);
            }
            return;
        }
        NativeFormat::I32Be => {
            let raw = float_to_signed(value, i32::MAX as f64, 2_147_483_648.0) as i32;
            let value = raw.to_be_bytes();
            unsafe {
                std::ptr::copy_nonoverlapping(value.as_ptr(), pointer, 4);
            }
            return;
        }
        NativeFormat::F32Le | NativeFormat::F32Be => {
            let value = if format == NativeFormat::F32Le {
                value.to_bits().to_le_bytes()
            } else {
                value.to_bits().to_be_bytes()
            };
            unsafe {
                std::ptr::copy_nonoverlapping(value.as_ptr(), pointer, 4);
            }
            return;
        }
        NativeFormat::F64Le | NativeFormat::F64Be => {
            let value = if format == NativeFormat::F64Le {
                (value as f64).to_bits().to_le_bytes()
            } else {
                (value as f64).to_bits().to_be_bytes()
            };
            unsafe {
                std::ptr::copy_nonoverlapping(value.as_ptr(), pointer, 8);
            }
            return;
        }
    };
    unsafe {
        std::ptr::copy_nonoverlapping(slice.as_ptr(), pointer, slice.len());
    }
}

pub(crate) unsafe fn native_planar_to_interleaved(
    buffers: *const *const c_void,
    channels: usize,
    frames: usize,
    format: NativeFormat,
    output: &mut [f32],
) -> bool {
    if channels == 0 || buffers.is_null() || output.len() != channels.saturating_mul(frames) {
        return false;
    }
    let bytes = format.bytes();
    for channel in 0..channels {
        let pointer = unsafe { buffers.add(channel).read() }.cast::<u8>();
        if pointer.is_null() {
            return false;
        }
        for frame in 0..frames {
            let value = unsafe { read_sample(pointer.add(frame * bytes), format) };
            if !value.is_finite() {
                return false;
            }
            output[frame * channels + channel] = value.clamp(-1.0, 1.0);
        }
    }
    true
}

pub(crate) unsafe fn interleaved_to_native_planar(
    input: &[f32],
    buffers: *const *mut c_void,
    channels: usize,
    frames: usize,
    format: NativeFormat,
) -> bool {
    if channels == 0 || buffers.is_null() || input.len() != channels.saturating_mul(frames) {
        return false;
    }
    let bytes = format.bytes();
    if input.iter().any(|sample| !sample.is_finite()) {
        return false;
    }
    for channel in 0..channels {
        let pointer = unsafe { buffers.add(channel).read() }.cast::<u8>();
        if pointer.is_null() {
            return false;
        }
        for frame in 0..frames {
            unsafe {
                write_sample(
                    pointer.add(frame * bytes),
                    format,
                    input[frame * channels + channel],
                );
            }
        }
    }
    true
}

pub(crate) unsafe fn silence_native_planar(
    buffers: *const *mut c_void,
    channels: usize,
    frames: usize,
    format: NativeFormat,
) -> bool {
    if channels == 0 || buffers.is_null() {
        return false;
    }
    let channel_bytes = frames.saturating_mul(format.bytes());
    for channel in 0..channels {
        let pointer = unsafe { buffers.add(channel).read() }.cast::<u8>();
        if pointer.is_null() {
            return false;
        }
        unsafe { std::ptr::write_bytes(pointer, 0, channel_bytes) };
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(format: NativeFormat, tolerance: f32) {
        let input = [-1.0_f32, -0.5, 0.0, 0.5, 1.0];
        let mut left = vec![0_u8; input.len() * format.bytes()];
        let mut pointers = [left.as_mut_ptr().cast::<c_void>()];
        assert!(unsafe {
            interleaved_to_native_planar(&input, pointers.as_mut_ptr(), 1, input.len(), format)
        });
        let read_pointers = [left.as_ptr().cast::<c_void>()];
        let mut output = [0.0_f32; 5];
        assert!(unsafe {
            native_planar_to_interleaved(
                read_pointers.as_ptr(),
                1,
                input.len(),
                format,
                &mut output,
            )
        });
        for (actual, expected) in output.into_iter().zip(input) {
            assert!(
                (actual - expected).abs() <= tolerance,
                "{format:?}: {actual} != {expected}"
            );
        }
    }

    #[test]
    fn every_supported_native_format_roundtrips_endpoints_and_midpoints() {
        for format in [
            NativeFormat::I16Le,
            NativeFormat::I16Be,
            NativeFormat::I24Le,
            NativeFormat::I24Be,
            NativeFormat::I32Le,
            NativeFormat::I32Be,
            NativeFormat::F32Le,
            NativeFormat::F32Be,
            NativeFormat::F64Le,
            NativeFormat::F64Be,
        ] {
            roundtrip(format, 1.0 / 32_767.0);
        }
    }

    #[test]
    fn reordered_planar_channels_interleave_exactly_and_silence_is_complete() {
        let mut first = [0_i16, i16::MAX];
        let mut second = [i16::MIN, 0_i16];
        let read = [
            first.as_ptr().cast::<c_void>(),
            second.as_ptr().cast::<c_void>(),
        ];
        let mut interleaved = [0.0_f32; 4];
        assert!(unsafe {
            native_planar_to_interleaved(read.as_ptr(), 2, 2, NativeFormat::I16Le, &mut interleaved)
        });
        assert_eq!(interleaved, [0.0, -1.0, i16::MAX as f32 / 32768.0, 0.0]);

        first.fill(17);
        second.fill(17);
        let write = [
            first.as_mut_ptr().cast::<c_void>(),
            second.as_mut_ptr().cast::<c_void>(),
        ];
        assert!(unsafe { silence_native_planar(write.as_ptr(), 2, 2, NativeFormat::I16Le) });
        assert_eq!(first, [0, 0]);
        assert_eq!(second, [0, 0]);
    }

    #[test]
    fn invalid_pointer_length_and_nonfinite_samples_fail_closed() {
        let mut storage = [0_u8; 8];
        let pointers = [storage.as_mut_ptr().cast::<c_void>()];
        assert!(!unsafe {
            interleaved_to_native_planar(&[f32::NAN], pointers.as_ptr(), 1, 1, NativeFormat::F32Le)
        });
        assert!(!unsafe {
            interleaved_to_native_planar(&[0.0, 1.0], pointers.as_ptr(), 1, 1, NativeFormat::F32Le)
        });
        assert!(!unsafe { silence_native_planar(std::ptr::null(), 1, 1, NativeFormat::F32Le) });
    }
}
