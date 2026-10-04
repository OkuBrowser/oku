use chrono::DateTime;
use chrono::FixedOffset;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use chrono::TimeZone;
use chrono::Utc;
use core::fmt;
use serde::Deserialize;
use serde::Serialize;
use sys_locale::get_locale;
use tracing::debug;

#[derive(Eq, PartialEq, PartialOrd, Clone, Default, Debug, Serialize, Deserialize)]
/// A page's date-time metadata
pub struct Date {
    /// Year with four digits
    pub year: String,
    /// Year without the century (00..99)
    pub short_year: String,
    /// Month (01..12)
    pub month: String,
    /// Month without leading zeros
    pub i_month: String,
    /// Three-letter month abbreviation, e.g. "Jan"
    pub short_month: String,
    /// Full month name, e.g. "January"
    pub long_month: String,
    /// Day of the month (01..31)
    pub day: String,
    /// Day of the month without leading zeros
    pub i_day: String,
    /// Ordinal day of the year, with leading zeros. (001..366)
    pub y_day: String,
    /// Week year which may differ from the month year for up to three days at the start of January and end of December
    pub w_year: String,
    /// Week number of the current year, starting with the first week having a majority of its days in January (01..53)
    pub week: String,
    /// Day of the week, starting with Monday (1..7)
    pub w_day: String,
    /// Three-letter weekday abbreviation, e.g. "Sun"
    pub short_day: String,
    /// Weekday name, e.g. "Sunday"
    pub long_day: String,
    /// Hour of the day, 24-hour clock, zero-padded (00..23)
    pub hour: String,
    /// Minute of the hour (00..59)
    pub minute: String,
    /// Second of the minute (00..59)
    pub second: String,
    /// A page's date-time metadata, formatted per the RFC 3339 standard
    pub rfc_3339: String,
    /// A page's date-time metadata, formatted per the RFC 2822 standard
    pub rfc_2822: String,
}

/// Handle conversion of a Date object into a string of characters
impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.rfc_3339)
    }
}

impl Date {
    /// Convert a `toml::Value` into a `Date`
    ///
    /// # Arguments
    ///
    /// * `value` - The `toml::Value` to convert
    ///
    /// * `locale` - The locale used to represent dates and times
    pub fn value_to_date(value: &toml::value::Datetime, locale: &chrono::Locale) -> Date {
        let date = match value.date {
            Some(date) => {
                NaiveDate::from_ymd_opt(date.year.into(), date.month.into(), date.day.into())
                    .unwrap()
            }
            None => NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
        };
        let time = match value.time {
            Some(time) => NaiveTime::from_hms_nano_opt(
                time.hour.into(),
                time.minute.into(),
                time.second.map(|x| x.into()).unwrap_or_default(),
                time.nanosecond.unwrap_or_default(),
            )
            .unwrap(),
            None => NaiveTime::from_hms_nano_opt(0, 0, 0, 0).unwrap(),
        };
        let naive_datetime = NaiveDateTime::new(date, time);
        let datetime: DateTime<FixedOffset> = match value.offset {
            Some(offset) => {
                let offset_seconds = match offset {
                    toml::value::Offset::Z => 0,
                    toml::value::Offset::Custom { minutes } => minutes * 60,
                };
                let timezone =
                    TimeZone::from_offset(&FixedOffset::east_opt(offset_seconds.into()).unwrap());
                naive_datetime.and_local_timezone(timezone).unwrap()
            }
            None => naive_datetime.and_utc().into(),
        };
        Date::chrono_to_date(&datetime.into(), locale)
    }

    /// Convert a `chrono::DateTime` into a `Date`
    ///
    /// # Arguments
    ///
    /// * `datetime` - A `chrono::DateTime<chrono::Utc>`
    ///
    /// * `locale` - The locale used to represent dates and times
    pub fn chrono_to_date(datetime: &chrono::DateTime<Utc>, locale: &chrono::Locale) -> Date {
        Date {
            year: format!("{}", datetime.format_localized("%Y", *locale)),
            short_year: format!("{}", datetime.format_localized("%y", *locale)),
            month: format!("{}", datetime.format_localized("%m", *locale)),
            i_month: format!("{}", datetime.format_localized("%-m", *locale)),
            short_month: format!("{}", datetime.format_localized("%b", *locale)),
            long_month: format!("{}", datetime.format_localized("%B", *locale)),
            day: format!("{}", datetime.format_localized("%d", *locale)),
            i_day: format!("{}", datetime.format_localized("%-d", *locale)),
            y_day: format!("{}", datetime.format_localized("%j", *locale)),
            w_year: format!("{}", datetime.format_localized("%G", *locale)),
            week: format!("{}", datetime.format_localized("%U", *locale)),
            w_day: format!("{}", datetime.format_localized("%u", *locale)),
            short_day: format!("{}", datetime.format_localized("%a", *locale)),
            long_day: format!("{}", datetime.format_localized("%A", *locale)),
            hour: format!("{}", datetime.format_localized("%H", *locale)),
            minute: format!("{}", datetime.format_localized("%M", *locale)),
            second: format!("{}", datetime.format_localized("%S", *locale)),
            rfc_3339: datetime.to_rfc3339(),
            rfc_2822: datetime.to_rfc2822(),
        }
    }
}

/// Gets a string representing the system locale, if available. Otherwise, defaults to 'en_US'
pub fn default_locale_string() -> String {
    get_locale().unwrap_or("en_US".to_owned())
}

/// Gets the system locale, if available. Otherwise, defaults to `en_US`
pub fn default_locale() -> chrono::Locale {
    chrono::Locale::try_from(default_locale_string().as_str()).unwrap_or(chrono::Locale::en_US)
}

/// Gets a `chrono::Locale` from a string
pub fn locale_string_to_locale(locale: &str) -> chrono::Locale {
    debug!("Locale: {}", locale);
    chrono::Locale::try_from(locale).unwrap_or(default_locale())
}
