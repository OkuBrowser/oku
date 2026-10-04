use std::{fmt, time::Duration};

/// A wrapper type that allows you to Display a Duration
#[derive(Debug, Clone)]
pub struct FormattedDuration(Duration);

pub fn format_duration(val: Duration) -> FormattedDuration {
    FormattedDuration(val)
}

fn item_plural(f: &mut fmt::Formatter, started: &mut bool, name: &str, value: u64) -> fmt::Result {
    if value > 0 {
        if *started {
            f.write_str(" ")?;
        }
        write!(f, "{}{}", value, name)?;
        if value > 1 {
            f.write_str("s")?;
        }
        *started = true;
    }
    Ok(())
}
fn item_ms(
    f: &mut fmt::Formatter,
    started: &mut bool,
    name: &str,
    duration: Duration,
) -> fmt::Result {
    // Three cases
    // 1. Above 30seconds, we dont' show ms anymore
    // 2. Between 30sec and 1 sec we don't show rounded ms
    // 3. Below we round to two digits
    if Duration::new(30, 0) < duration {
        return Ok(());
    }
    if Duration::new(1, 0) < duration {
        if *started {
            f.write_str(" ")?;
        }
        write!(f, "{}{}", duration.subsec_millis(), name)?;
        *started = true;
        return Ok(());
    }
    //show rounded
    if *started {
        f.write_str(" ")?;
    }
    write!(
        f,
        "{}{}",
        (duration.subsec_nanos() / 10_000) as f32 / 100.,
        name
    )?;
    *started = true;
    Ok(())
}
fn item(
    f: &mut fmt::Formatter,
    started: &mut bool,
    name: &str,
    value: u32,
    skip: bool,
) -> fmt::Result {
    if skip {
        return Ok(());
    }
    if value > 0 {
        if *started {
            f.write_str(" ")?;
        }
        write!(f, "{}{}", value, name)?;
        *started = true;
    }
    Ok(())
}

const ONE_MINUTE_IN_SECONDS: u64 = 60;
const ONE_HOURS_IN_SECONDS: u64 = ONE_MINUTE_IN_SECONDS * 60;
const ONE_DAY_IN_SECONDS: u64 = ONE_HOURS_IN_SECONDS * 24;

impl fmt::Display for FormattedDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let secs = self.0.as_secs();
        let nanos = self.0.subsec_nanos();

        if secs == 0 && nanos == 0 {
            f.write_str("0s")?;
            return Ok(());
        }

        let years = secs / 31_557_600; // 365.25d
        let ydays = secs % 31_557_600;
        let months = ydays / 2_630_016; // 30.44d
        let mdays = ydays % 2_630_016;
        let days = mdays / 86400;
        let day_secs = mdays % 86400;
        let hours = day_secs / 3600;
        let minutes = day_secs % 3600 / 60;
        let seconds = day_secs % 60;

        let started = &mut false;
        item_plural(f, started, "year", years)?;
        item_plural(f, started, "month", months)?;
        item_plural(f, started, "day", days)?;

        item(
            f,
            started,
            "h",
            hours as u32,
            self.0 > Duration::from_secs(ONE_DAY_IN_SECONDS * 30), // skip hours after 30 days
        )?;
        item(
            f,
            started,
            "m",
            minutes as u32,
            self.0 > Duration::from_secs(ONE_DAY_IN_SECONDS), // skip minutes after one day
        )?;
        item(
            f,
            started,
            "s",
            seconds as u32,
            self.0 > Duration::from_secs(ONE_HOURS_IN_SECONDS * 3), // skip seconds after one hour
        )?;
        item_ms(f, started, "ms", self.0)?;
        Ok(())
    }
}

pub fn human_readable_time(duration: Duration) -> String {
    format_duration(duration).to_string()
}

#[test]
fn human_readable_time_test() {
    assert_eq!(
        human_readable_time(Duration::new(5, 900_000_000)),
        "5s 900ms"
    );
    assert_eq!(
        human_readable_time(Duration::new(ONE_HOURS_IN_SECONDS + 1, 900_123_000)),
        "1h 1s"
    );
    assert_eq!(
        human_readable_time(Duration::new(ONE_HOURS_IN_SECONDS - 1, 900_123_000)),
        "59m 59s"
    );
    assert_eq!(
        human_readable_time(Duration::new(3 * ONE_HOURS_IN_SECONDS + 1, 900_123_000)),
        "3h"
    );
    assert_eq!(
        human_readable_time(Duration::new(5, 900_123_000)),
        "5s 900ms"
    );
    assert_eq!(
        human_readable_time(Duration::new(
            ONE_HOURS_IN_SECONDS * 10 + ONE_MINUTE_IN_SECONDS * 10 + 10, // don't print seconds
            123
        )),
        "10h 10m"
    );
    assert_eq!(
        human_readable_time(Duration::new(
            ONE_HOURS_IN_SECONDS * 10 + ONE_MINUTE_IN_SECONDS,
            123
        )),
        "10h 1m"
    );
    assert_eq!(
        human_readable_time(Duration::new(ONE_HOURS_IN_SECONDS * 3 + 10, 123)),
        "3h"
    );
    assert_eq!(human_readable_time(Duration::new(0, 900_000)), "0.9ms");
    assert_eq!(human_readable_time(Duration::new(0, 950_000)), "0.95ms");
    assert_eq!(human_readable_time(Duration::new(0, 1_950_000)), "1.95ms");
    assert_eq!(human_readable_time(Duration::new(0, 1_950_000)), "1.95ms");
    assert_eq!(human_readable_time(Duration::new(0, 1_957_123)), "1.95ms");
}
