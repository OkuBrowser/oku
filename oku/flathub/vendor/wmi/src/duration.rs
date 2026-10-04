use crate::WMIError;
use serde::{de, ser};
use std::{fmt, str::FromStr, time::Duration};

/// A wrapper type around Duration, which supports parsing from WMI-format strings.
///
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct WMIDuration(pub Duration);

impl FromStr for WMIDuration {
    type Err = WMIError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 25 {
            return Err(WMIError::ConvertDurationError(s.into()));
        }

        let (seconds_part, reminder) = s.split_at(14);
        let (micros_part, _) = reminder[1..].split_at(6);

        let seconds: u64 = seconds_part.parse()?;
        let micros: u64 = micros_part.parse()?;

        let duration = Duration::from_secs(seconds) + Duration::from_micros(micros);

        Ok(Self(duration))
    }
}

#[derive(Debug, Clone)]
struct DurationVisitor;

impl<'de> de::Visitor<'de> for DurationVisitor {
    type Value = WMIDuration;

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

impl<'de> de::Deserialize<'de> for WMIDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_u64(DurationVisitor)
    }
}

impl ser::Serialize for WMIDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_u64(self.0.as_micros() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::WMIDuration;
    use serde_json;

    #[test]
    fn it_works() {
        let duration: WMIDuration = "00000005141436.100001:000".parse().unwrap();

        assert_eq!(duration.0.as_micros(), 5141436100001);
        assert_eq!(duration.0.as_millis(), 5141436100);
        assert_eq!(duration.0.as_secs(), 5141436);
    }

    #[test]
    fn it_serializes_to_rfc() {
        let duration: WMIDuration = "00000005141436.100001:000".parse().unwrap();

        let v = serde_json::to_string(&duration).unwrap();
        assert_eq!(v, "5141436100001");
    }
}
