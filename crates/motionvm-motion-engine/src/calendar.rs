//! The civil date, for the one kernel word that asks: the 16-bit `GIVEDATE`
//! pushes the day, the month and the year that DOS answers INT 21h function
//! 2Ah with.
//!
//! The clock is read here and nowhere else in the engine, and it is read as
//! **UTC**: `std` knows the seconds since the epoch and no time zone, and a
//! dependency for the zone would buy a day's difference in the hours either
//! side of midnight and nothing more — a departure the ledger carries. The
//! conversion from a day count to a calendar date is the proleptic Gregorian
//! one with no table in it: the era arithmetic is Howard Hinnant's, and the
//! tests below hold it to dates that are known.

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds in a day.
const DAY: u64 = 86_400;

/// Today's date in UTC, as `(day, month, year)` — the order `GIVEDATE`
/// pushes, day first and year on top.
pub(crate) fn today() -> (i32, i32, i32) {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() / DAY);
    civil_from_days(i64::try_from(days).unwrap_or(i64::MAX))
}

/// The calendar date `days` days after 1970-01-01, as `(day, month, year)`.
///
/// Hinnant's `civil_from_days`: the count is moved to an epoch of
/// 0000-03-01, where a leap day is the last day of its year and the
/// 146 097-day era is the calendar's whole 400-year cycle, and the year, the
/// month and the day are read off it in that order. Every intermediate
/// quantity is bounded by the comment beside it, so the narrowing at the end
/// cannot lose anything for any date a clock produces.
pub(crate) fn civil_from_days(days: i64) -> (i32, i32, i32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097); // day of era, 0..=146096
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // year of era, 0..=399
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year, 0..=365, from March
    let mp = (5 * doy + 2) / 153; // month from March, 0..=11
    let day = doy - (153 * mp + 2) / 5 + 1; // 1..=31
    let month = if mp < 10 { mp + 3 } else { mp - 9 }; // 1..=12
    let year = yoe + era * 400 + i64::from(month <= 2);
    (narrow(day), narrow(month), narrow(year))
}

/// A quantity the arithmetic above keeps within a few thousand, as the cell
/// it is pushed as.
fn narrow(n: i64) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    /// Known days: the epoch, the game's own release date, a leap day, a
    /// New Year's Eve and a New Year's Day across the 2000 boundary, and the
    /// day before the epoch.
    #[test]
    fn known_dates_come_back() {
        assert_eq!(civil_from_days(0), (1, 1, 1970));
        assert_eq!(civil_from_days(9051), (13, 10, 1994));
        assert_eq!(civil_from_days(11016), (29, 2, 2000));
        assert_eq!(civil_from_days(10591), (31, 12, 1998));
        assert_eq!(civil_from_days(19723), (1, 1, 2024));
        assert_eq!(civil_from_days(-1), (31, 12, 1969));
    }

    /// Walking a day at a time never skips or repeats a date: each step is
    /// the next day of the same month, or the first of the next month, or
    /// the first of January of the next year.
    #[test]
    fn consecutive_days_are_consecutive_dates() {
        let mut last = civil_from_days(0);
        for days in 1..=40_000 {
            let next = civil_from_days(days);
            let (d, m, y) = last;
            let ok = next == (d + 1, m, y)
                || (next.0 == 1 && next == (1, m + 1, y))
                || next == (1, 1, y + 1);
            assert!(ok, "{last:?} is followed by {next:?}");
            last = next;
        }
    }
}
