use thiserror::Error;

/// Main error type.
#[derive(Error, Debug)]
pub enum Error {
    /// When links are not sorted according to the spec.
    #[error("the links are not sorted correctly")]
    LinksWrongOrder,
    /// When the conversion from [`ipld_core::ipld::Ipld`] to DAG-PB fails.
    #[error("cannot convert from IPLD: {0}")]
    FromIpld(String),
    /// When there is an error during the buffers buffers encoding.
    #[error("cannot encode protocol buffers")]
    ToPb(#[from] quick_protobuf::Error),
    /// When there is an error with the reader or writer.
    #[error("IO error")]
    Io(#[from] std::io::Error),
}
