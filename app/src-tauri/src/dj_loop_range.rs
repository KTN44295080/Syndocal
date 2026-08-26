//! Exact Rekordbox Beat Loop lengths accepted by the DJ Link timeline bridge.
//!
//! Stage-1 maps an eight-beat Rekordbox selection to authored division zero,
//! and every subsequent division halves that range. This mapping accepts only
//! an explicit profile length carried either by fresh measured authority or
//! the distinct bounded no-response fallback event. It never selects or
//! predicts a value locally from an absent report.

/// Maps one exact Rekordbox Beat Loop profile length to the engine's absolute
/// loop division.
///
/// Every accepted value is a dyadic fraction and exactly representable as an
/// `f64`, so use exact comparison instead of the protocol's span-consistency
/// tolerance. Accepting a near value would turn an ambiguous representation
/// into a different loop selection.
pub(crate) fn division_for_profile_length(length_beats: f64) -> Result<u8, &'static str> {
    if !length_beats.is_finite() {
        return Err("loop_length_not_finite");
    }
    if length_beats <= 0.0 {
        return Err("loop_length_not_positive");
    }
    let max_length_beats = protocol::DJ_LINK_LOOP_PROFILE_LENGTH_BEATS[0];
    let min_length_beats = *protocol::DJ_LINK_LOOP_PROFILE_LENGTH_BEATS
        .last()
        .expect("DJ Link loop profile must have a minimum");
    if length_beats > max_length_beats {
        return Err("loop_length_above_supported_range");
    }
    if length_beats < min_length_beats {
        return Err("loop_length_below_supported_range");
    }

    protocol::DJ_LINK_LOOP_PROFILE_LENGTH_BEATS
        .iter()
        .enumerate()
        .find_map(|(division, &supported_length)| {
            (length_beats == supported_length).then_some(division as u8)
        })
        .ok_or("loop_length_not_exact_supported_dyadic")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_the_full_supported_rekordbox_beat_loop_range() {
        for (length_beats, division) in [
            (8.0, 0),
            (4.0, 1),
            (2.0, 2),
            (1.0, 3),
            (1.0 / 2.0, 4),
            (1.0 / 4.0, 5),
            (1.0 / 8.0, 6),
            (1.0 / 16.0, 7),
            (1.0 / 32.0, 8),
            (1.0 / 64.0, 9),
        ] {
            assert_eq!(division_for_profile_length(length_beats), Ok(division));
        }
    }

    #[test]
    fn rejects_non_finite_or_non_positive_lengths() {
        for length_beats in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                division_for_profile_length(length_beats),
                Err("loop_length_not_finite")
            );
        }
        for length_beats in [0.0, -1.0] {
            assert_eq!(
                division_for_profile_length(length_beats),
                Err("loop_length_not_positive")
            );
        }
    }

    #[test]
    fn rejects_out_of_range_and_non_dyadic_lengths_without_rounding() {
        assert_eq!(
            division_for_profile_length(16.0),
            Err("loop_length_above_supported_range")
        );
        assert_eq!(
            division_for_profile_length(1.0 / 128.0),
            Err("loop_length_below_supported_range")
        );
        for length_beats in [
            3.0,
            3.0 / 4.0,
            4.0 + (protocol::DJ_LINK_LOOP_LENGTH_TOLERANCE_BEATS / 2.0),
        ] {
            assert_eq!(
                division_for_profile_length(length_beats),
                Err("loop_length_not_exact_supported_dyadic")
            );
        }
    }
}
