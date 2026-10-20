//! UTC Gregorian DATE, months 1..=12. Capture once per frame or inject a frozen value.
use std::time::{SystemTime, UNIX_EPOCH};

pub fn isf_date_now() -> [f32; 4] {
    isf_date_from_system_time(SystemTime::now())
}

pub fn isf_date_from_system_time(time: SystemTime) -> [f32; 4] {
    let nanos = match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos() as i128,
        Err(error) => -(error.duration().as_nanos() as i128),
    };
    let day_nanos = 86_400_000_000_000_i128;
    let days = nanos.div_euclid(day_nanos);
    let within_day = nanos.rem_euclid(day_nanos);
    // Gregorian eras contain 400 years / 146097 days.
    let shifted = days + 719468;
    let era = shifted.div_euclid(146097);
    let day_of_era = shifted - era * 146097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = month_from_march + if month_from_march < 10 { 3 } else { -9 };
    year += i128::from(month <= 2);
    [
        year as f32,
        month as f32,
        day as f32,
        // The final f32 rounding step must not cross into tomorrow.
        ((within_day as f64 / 1_000_000_000.0) as f32).min(86399.99),
    ]
}

pub(super) fn valid_isf_date(date: [f32; 4]) -> bool {
    if !date.iter().all(|value| value.is_finite())
        || date[0].fract() != 0.0
        || !(1.0..=9999.0).contains(&date[0])
        || date[1].fract() != 0.0
        || !(1.0..=12.0).contains(&date[1])
        || date[2].fract() != 0.0
        || !(0.0..86_400.0).contains(&date[3])
    {
        return false;
    }
    let year = date[0] as i32;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match date[1] as u32 {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1.0..=max_day as f32).contains(&date[2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn isf_date_epoch_leap_day_and_fractional_seconds_are_deterministic() {
        assert_eq!(
            isf_date_from_system_time(UNIX_EPOCH),
            [1970.0, 1.0, 1.0, 0.0]
        );
        assert_eq!(
            isf_date_from_system_time(UNIX_EPOCH - Duration::from_millis(250)),
            [1969.0, 12.0, 31.0, 86399.75]
        );
        assert_eq!(
            isf_date_from_system_time(UNIX_EPOCH + Duration::from_millis(951827696500)),
            [2000.0, 2.0, 29.0, 45296.5]
        );
        assert!(valid_isf_date(isf_date_from_system_time(
            UNIX_EPOCH + Duration::from_nanos(86399999999999)
        )));
    }

    #[test]
    fn isf_date_rejects_nonfinite_and_invalid_calendar_values() {
        assert!(valid_isf_date([2026.0, 10.0, 20.0, 45296.5]));
        assert!(!valid_isf_date([2026.0, 2.0, 29.0, 0.0]));
        assert!(!valid_isf_date([2026.0, 0.0, 20.0, 0.0]));
        assert!(!valid_isf_date([2026.0, 10.0, 20.0, f32::NAN]));
        assert!(!valid_isf_date([2026.0, 10.0, 20.0, 86400.0]));
    }
}
