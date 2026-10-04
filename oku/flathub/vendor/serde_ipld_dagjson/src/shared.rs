use ipld_core::cid::{multibase::Base, Cid};
use serde::{de, Deserialize, Serialize};

/// Result of deserializing a DAG-JSON map consisting of the reserved key `/`.
///
/// The values are the already parsed/decoded data.
#[derive(Debug)]
pub(crate) enum ReservedKeyValueParsed {
    Cid(Cid),
    Bytes(Vec<u8>),
}

/// Used for deserializing a DAG-JSON map, consisting of the reserved key `/`.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ReservedKeyMap {
    #[serde(rename = "/")]
    pub(crate) _slash: ReservedKeyValue,
}

/// Used for deserializing a DAG-JSON map, consisting of the reserved key `/`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum ReservedKeyValue {
    Cid(String),
    Bytes { bytes: String },
}

impl ReservedKeyValue {
    pub(crate) fn parse<E>(&self) -> Result<ReservedKeyValueParsed, E>
    where
        E: de::Error,
    {
        match self {
            ReservedKeyValue::Cid(base_encoded_cid) => {
                let cid = Cid::try_from(&base_encoded_cid[..]).map_err(|_| {
                    de::Error::custom(format!("Invalid CID `{}`", base_encoded_cid))
                })?;
                Ok(ReservedKeyValueParsed::Cid(cid))
            }
            ReservedKeyValue::Bytes {
                bytes: base_encoded_bytes,
            } => {
                let bytes = Base::Base64.decode(&base_encoded_bytes[..]).map_err(|_| {
                    de::Error::custom(format!("Cannot base decode bytes `{}`", base_encoded_bytes))
                })?;
                Ok(ReservedKeyValueParsed::Bytes(bytes))
            }
        }
    }
}
