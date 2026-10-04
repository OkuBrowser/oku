use crate::WMIError;
use serde::{de, ser};
use std::{fmt, str::FromStr};
use time::{
    PrimitiveDateTime, UtcOffset, format_description::FormatItem, macros::format_description,
    parsing::Parsed,
};

/// A wrapper type around `time`'s `OffsetDateTime` (if the
// `time` feature is active), which supports parsing from WMI-format strings.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct WMIOffsetDateTime(pub time::OffsetDateTime);

impl FromStr for WMIOffsetDateTime {
    type Err = WMIError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 21 {
            return Err(WMIError::ConvertDatetimeError(s.into()));
        }

        // We have to ignore the year here, see bottom of https://time-rs.github.io/book/api/format-description.html
        // about the large-dates feature (permanent link:
        // https://github.com/time-rs/book/blob/0476c5bb35b512ac0cbda5c6cd5f0d0628b0269e/src/api/format-description.md?plain=1#L205)
        const TIME_FORMAT: &[FormatItem<'static>] =
            format_description!("[month][day][hour][minute][second].[subsecond digits:6]");

        let minutes_offset = s[21..].parse::<i32>()?;
        let offset =
            UtcOffset::from_whole_seconds(minutes_offset * 60).map_err(time::Error::from)?;

        let mut parser = Parsed::new();

        let naive_date_time = &s[4..21];
        parser
            .parse_items(naive_date_time.as_bytes(), TIME_FORMAT)
            .map_err(time::Error::from)?;
        // Microsoft thinks it is okay to return a subsecond value in microseconds but not put the zeros before it
        // so 1.1 is 1 second and 100 microsecond, ergo 1.000100 ...
        parser
            .set_subsecond(parser.subsecond().unwrap_or(0) / 1000)
            .ok_or_else(|| time::error::Format::InvalidComponent("subsecond"))
            .map_err(time::Error::from)?;

        let naive_year = s[..4].parse::<i32>()?;
        parser
            .set_year(naive_year)
            .ok_or_else(|| time::error::Format::InvalidComponent("year"))
            .map_err(time::Error::from)?;

        let naive_date_time: PrimitiveDateTime =
            std::convert::TryInto::try_into(parser).map_err(time::Error::from)?;
        let dt = naive_date_time.assume_offset(offset);
        Ok(Self(dt))
    }
}

#[derive(Debug, Clone)]
struct DateTimeVisitor;

impl<'de> de::Visitor<'de> for DateTimeVisitor {
    type Value = WMIOffsetDateTime;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a timestamp in WMI format")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value.parse().map_err(|err| E::custom(format!("{}", err)))
    }
}

impl<'de> de::Deserialize<'de> for WMIOffsetDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(DateTimeVisitor)
    }
}

const RFC3339_WITH_6_DIGITS: &[FormatItem<'_>] = format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6][offset_hour sign:mandatory]:[offset_minute]"
);

impl ser::Serialize for WMIOffsetDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        // Unwrap: we passed a well known format, if it fails something has gone very wrong
        let formatted = self.0.format(RFC3339_WITH_6_DIGITS).unwrap();

        serializer.serialize_str(&formatted)
    }
}

#[cfg(test)]
mod tests {
    use super::WMIOffsetDateTime;
    use serde_json;

    #[test]
    fn it_works_with_negative_offset() {
        let dt: WMIOffsetDateTime = "20190113200517.500000-180".parse().unwrap();

        let formatted = dt.0.format(super::RFC3339_WITH_6_DIGITS).unwrap();

        assert_eq!(formatted, "2019-01-13T20:05:17.000500-03:00");
    }

    #[test]
    fn it_works_with_positive_offset() {
        let dt: WMIOffsetDateTime = "20190113200517.500000+060".parse().unwrap();

        let formatted = dt.0.format(super::RFC3339_WITH_6_DIGITS).unwrap();

        assert_eq!(formatted, "2019-01-13T20:05:17.000500+01:00");
    }

    #[test]
    fn it_fails_with_malformed_str() {
        let dt_res: Result<WMIOffsetDateTime, _> = "20190113200517".parse();

        assert!(dt_res.is_err());
    }

    #[test]
    fn it_fails_with_malformed_str_with_no_tz() {
        let dt_res: Result<WMIOffsetDateTime, _> = "20190113200517.000500".parse();

        assert!(dt_res.is_err());
    }

    #[test]
    fn it_serializes_to_rfc() {
        let dt: WMIOffsetDateTime = "20190113200517.500000+060".parse().unwrap();

        let v = serde_json::to_string(&dt).unwrap();
        assert_eq!(v, "\"2019-01-13T20:05:17.000500+01:00\"");
    }
}
