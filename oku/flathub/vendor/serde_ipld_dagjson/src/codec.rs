use std::io::{BufRead, Write};

use ipld_core::{
    cid::Cid,
    codec::{Codec, Links},
    serde::ExtractLinks,
};

use serde::{de::Deserialize, ser::Serialize};

use crate::{de::Deserializer, error::CodecError, DAG_JSON_CODE};

/// DAG-JSON implementation of ipld-core's `Codec` trait.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DagJsonCodec;

impl<T> Codec<T> for DagJsonCodec
where
    T: for<'a> Deserialize<'a> + Serialize,
{
    const CODE: u64 = DAG_JSON_CODE;
    type Error = CodecError;

    fn decode<R: BufRead>(reader: R) -> Result<T, Self::Error> {
        Ok(crate::from_reader(reader)?)
    }

    fn encode<W: Write>(writer: W, data: &T) -> Result<(), Self::Error> {
        Ok(crate::to_writer(writer, data)?)
    }
}

impl Links for DagJsonCodec {
    type LinksError = CodecError;

    fn links(data: &[u8]) -> Result<impl Iterator<Item = Cid>, Self::LinksError> {
        let mut json_deserializer = serde_json::Deserializer::from_slice(data);
        let deserializer = Deserializer::new(&mut json_deserializer);
        Ok(ExtractLinks::deserialize(deserializer)?
            .into_vec()
            .into_iter())
    }
}
