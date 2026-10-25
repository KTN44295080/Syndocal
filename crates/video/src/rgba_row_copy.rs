//! Row copies admitted by the compositor's existing sampling lookup.

use super::{blend_pixel, OpaqueSourceBlendTable, VideoBlendMode};

pub(super) struct ContiguousRgbaSpan {
    pub destination_start: usize,
    pub source_start: usize,
    pub byte_len: usize,
}

/// Accept only one covered run with consecutive source pixels. Inspecting the
/// actual lookup preserves the sampler's floating-point rounding and cropping.
pub(super) fn contiguous_rgba_span(columns: &[Option<usize>]) -> Option<ContiguousRgbaSpan> {
    let first = columns.iter().position(Option::is_some)?;
    let end = columns.iter().rposition(Option::is_some)? + 1;
    let source_start = columns[first]?;
    let contiguous = columns[first..end]
        .iter()
        .enumerate()
        .all(|(index, column)| {
            index
                .checked_mul(4)
                .and_then(|offset| source_start.checked_add(offset))
                .is_some_and(|offset| *column == Some(offset))
        });
    contiguous.then_some(ContiguousRgbaSpan {
        destination_start: first.checked_mul(4)?,
        source_start,
        byte_len: (end - first).checked_mul(4)?,
    })
}

/// Preserve the existing byte blend while iterating consecutive pixels without
/// a per-pixel optional lookup. Opaque Normal/full-opacity rows can use memcpy.
pub(super) fn blend_contiguous_rgba_row(
    destination: &mut [u8],
    source: &[u8],
    opacity: f32,
    blend_mode: &VideoBlendMode,
    copy_opaque: bool,
    opaque_table: Option<&OpaqueSourceBlendTable>,
) {
    if copy_opaque && source.chunks_exact(4).all(|pixel| pixel[3] == 255) {
        destination.copy_from_slice(source);
        return;
    }
    for (target, source) in destination.chunks_exact_mut(4).zip(source.chunks_exact(4)) {
        if source[3] != 255 {
            blend_pixel(target, source, opacity, blend_mode);
        } else if copy_opaque {
            target.copy_from_slice(source);
        } else if let Some(table) = opaque_table {
            table.apply(target, source);
        } else {
            blend_pixel(target, source, opacity, blend_mode);
        }
    }
}
