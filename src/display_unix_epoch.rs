// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt;
use std::time::Duration;

pub struct DisplayUnixTimeStamp {
    /// The duration since the UNIX epoch.
    duration: Option<Duration>,

    in_millis: bool,

    with_timezone: bool,
}

impl fmt::Display for DisplayUnixTimeStamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let duration = match self.duration {
            Some(d) => d,
            None => return write!(f, "None"),
        };

        let secs = duration.as_secs();
        let (year, month, day) = civil_from_days(secs / 86_400);
        let secs_of_day = secs % 86_400;
        let (hour, minute, second) = (
            secs_of_day / 3600,
            secs_of_day % 3600 / 60,
            secs_of_day % 60,
        );

        write!(
            f,
            "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}"
        )?;

        // The fraction is truncated, not rounded.
        if self.in_millis {
            write!(f, ".{:03}", duration.subsec_millis())?;
        } else {
            write!(f, ".{:06}", duration.subsec_micros())?;
        }

        // The time is always in UTC.
        if self.with_timezone {
            write!(f, "Z+0000")?;
        }

        Ok(())
    }
}

/// Convert days since 1970-01-01 to a proleptic Gregorian `(year, month, day)`.
///
/// Howard Hinnant's `civil_from_days`, restricted to days on or after the epoch:
/// <https://howardhinnant.github.io/date_algorithms.html>
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    // Shift the epoch to 0000-03-01, so that the leap day is the last day of a year.
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    // Month counted from March: 0 is March, 11 is February.
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

impl DisplayUnixTimeStamp {
    pub fn new(duration: Option<Duration>) -> Self {
        Self {
            duration,
            in_millis: false,
            with_timezone: true,
        }
    }

    pub fn in_millis(self, in_millis: bool) -> Self {
        Self { in_millis, ..self }
    }

    pub fn with_timezone(self, with_timezone: bool) -> Self {
        Self {
            with_timezone,
            ..self
        }
    }
}

/// Implement `Display` for `Duration` to display the duration since the UNIX epoch.
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use display_more::DisplayUnixTimeStampExt;
///
/// let duration = Duration::from_millis(1723102819023);
/// assert_eq!(
///     duration.display_unix_timestamp().to_string(),
///     "2024-08-08T07:40:19.023000Z+0000"
/// );
/// ```
pub trait DisplayUnixTimeStampExt {
    fn display_unix_timestamp(&self) -> DisplayUnixTimeStamp;

    /// Display the duration since the UNIX epoch in milliseconds without timezone.
    fn display_unix_timestamp_short(&self) -> DisplayUnixTimeStamp {
        self.display_unix_timestamp()
            .in_millis(true)
            .with_timezone(false)
    }
}

impl DisplayUnixTimeStampExt for Duration {
    fn display_unix_timestamp(&self) -> DisplayUnixTimeStamp {
        DisplayUnixTimeStamp::new(Some(*self))
    }
}

impl DisplayUnixTimeStampExt for Option<Duration> {
    fn display_unix_timestamp(&self) -> DisplayUnixTimeStamp {
        DisplayUnixTimeStamp::new(*self)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_display_unix_epoch() {
        let epoch = Duration::from_millis(0);
        let display = epoch.display_unix_timestamp();
        assert_eq!(format!("{}", display), "1970-01-01T00:00:00.000000Z+0000");

        let epoch = Duration::from_millis(1723102819023);
        let display = epoch.display_unix_timestamp();
        assert_eq!(format!("{}", display), "2024-08-08T07:40:19.023000Z+0000");

        let display = epoch.display_unix_timestamp_short();
        assert_eq!(format!("{}", display), "2024-08-08T07:40:19.023");

        let display = epoch.display_unix_timestamp().in_millis(true);
        assert_eq!(format!("{}", display), "2024-08-08T07:40:19.023Z+0000");

        let display = epoch.display_unix_timestamp().with_timezone(false);
        assert_eq!(format!("{}", display), "2024-08-08T07:40:19.023000");

        // Option<Duration>: Some
        let some = Some(Duration::from_millis(1723102819023));
        assert_eq!(
            some.display_unix_timestamp().to_string(),
            "2024-08-08T07:40:19.023000Z+0000"
        );
        assert_eq!(
            some.display_unix_timestamp_short().to_string(),
            "2024-08-08T07:40:19.023"
        );

        // Leap days and year boundaries.
        let leap_day = Duration::from_secs(951_782_400); // 2000-02-29T00:00:00Z
        assert_eq!(
            leap_day.display_unix_timestamp_short().to_string(),
            "2000-02-29T00:00:00.000"
        );
        let new_year_eve = Duration::new(1_704_067_199, 999_999_999); // 2023-12-31T23:59:59Z
        assert_eq!(
            new_year_eve.display_unix_timestamp().to_string(),
            "2023-12-31T23:59:59.999999Z+0000"
        );

        // Option<Duration>: None
        let none: Option<Duration> = None;
        assert_eq!(none.display_unix_timestamp().to_string(), "None");
        assert_eq!(none.display_unix_timestamp_short().to_string(), "None");
    }
}
