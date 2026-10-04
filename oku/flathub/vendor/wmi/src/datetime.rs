use crate::WMIError;
use chrono::prelude::*;
use serde::{de, ser};
use std::{fmt, str::FromStr};

/// A wrapper type around `chrono`'s `DateTime` (if the `chrono` feature is active. ), which supports parsing from WMI-format strings.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct WMIDateTime(pub DateTime<FixedOffset>);

impl FromStr for WMIDateTime {
    type Err = WMIError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 21 {
            return Err(WMIError::ConvertDatetimeError(s.into()));
        }

        let (datetime_part, tz_part) = s.split_at(21);
        let tz_min: i32 = tz_part.parse()?;
        let tz = FixedOffset::east_opt(tz_min * 60).unwrap();
        let dt = NaiveDateTime::parse_from_str(datetime_part, "%Y%m%d%H%M%S.%f")?
            .and_local_timezone(tz)
            .single()
            .ok_or(WMIError::ParseDatetimeLocalError)?;

        Ok(Self(dt))
    }
}

#[derive(Debug, Clone)]
struct DateTimeVisitor;

impl<'de> de::Visitor<'de> for DateTimeVisitor {
    type Value = WMIDateTime;

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

impl<'de> de::Deserialize<'de> for WMIDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(DateTimeVisitor)
    }
}

impl ser::Serialize for WMIDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let formatted = self.0.to_rfc3339();

        serializer.serialize_str(&formatted)
    }
}

#[cfg(test)]
mod tests {
    use super::WMIDateTime;
    use serde_json;

    #[test]
    fn it_works_with_negative_offset() {
        let dt: WMIDateTime = "20190113200517.500000-180".parse().unwrap();

        let formatted = dt.0.to_rfc3339();

        assert_eq!(formatted, "2019-01-13T20:05:17.000500-03:00");
    }

    #[test]
    fn it_works_with_positive_offset() {
        let dt: WMIDateTime = "20190113200517.500000+060".parse().unwrap();

        let formatted = dt.0.to_rfc3339();

        assert_eq!(formatted, "2019-01-13T20:05:17.000500+01:00");
    }

    #[test]
    fn it_fails_with_malformed_str() {
        let dt_res: Result<WMIDateTime, _> = "20190113200517".parse();

        assert!(dt_res.is_err());
    }

    #[test]
    fn it_fails_with_malformed_str_with_no_tz() {
        let dt_res: Result<WMIDateTime, _> = "20190113200517.000500".parse();

        assert!(dt_res.is_err());
    }

    #[test]
    fn it_serializes_to_rfc() {
        let dt: WMIDateTime = "20190113200517.500000+060".parse().unwrap();

        let v = serde_json::to_string(&dt).unwrap();
        assert_eq!(v, "\"2019-01-13T20:05:17.000500+01:00\"");
    }
}
