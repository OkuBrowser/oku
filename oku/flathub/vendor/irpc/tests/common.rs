#![cfg(feature = "noq_endpoint_setup")]

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use irpc::util::{make_client_endpoint, make_server_endpoint};
use n0_error::stack_error;
use noq::Endpoint;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use testresult::TestResult;

pub fn create_connected_endpoints() -> TestResult<(Endpoint, Endpoint, SocketAddr)> {
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0).into();
    let (server, cert) = make_server_endpoint(addr)?;
    let client = make_client_endpoint(addr, &[cert.as_slice()])?;
    let port = server.local_addr()?.port();
    let server_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port).into();
    Ok((server, client, server_addr))
}

#[derive(Debug)]
pub struct NoSer(pub u64);

#[stack_error(derive)]
#[error("Cannot serialize odd number")]
pub struct OddNumberError(u64);

impl Serialize for NoSer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0 % 2 == 1 {
            Err(serde::ser::Error::custom(OddNumberError(self.0)))
        } else {
            serializer.serialize_u64(self.0)
        }
    }
}

impl<'de> Deserialize<'de> for NoSer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        if value % 2 != 0 {
            Err(serde::de::Error::custom(OddNumberError(value)))
        } else {
            Ok(NoSer(value))
        }
    }
}
