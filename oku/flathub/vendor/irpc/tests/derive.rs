#![cfg(feature = "derive")]

use irpc::{
    channel::{none::NoSender, oneshot},
    rpc_requests,
};
use serde::{Deserialize, Serialize};

#[test]
fn derive_simple() {
    #![allow(dead_code)]

    #[derive(Debug, Serialize, Deserialize)]
    struct RpcRequest;

    #[derive(Debug, Serialize, Deserialize)]
    struct ServerStreamingRequest;

    #[derive(Debug, Serialize, Deserialize)]
    struct ClientStreamingRequest;

    #[derive(Debug, Serialize, Deserialize)]
    struct BidiStreamingRequest;

    #[derive(Debug, Serialize, Deserialize)]
    struct Update1;

    #[derive(Debug, Serialize, Deserialize)]
    struct Update2;

    #[derive(Debug, Serialize, Deserialize)]
    struct Response1;

    #[derive(Debug, Serialize, Deserialize)]
    struct Response2;

    #[derive(Debug, Serialize, Deserialize)]
    struct Response3;

    #[derive(Debug, Serialize, Deserialize)]
    struct Response4;

    #[rpc_requests(message = RequestWithChannels, no_rpc, no_spans)]
    #[derive(Debug, Serialize, Deserialize)]
    enum Request {
        #[rpc(tx=oneshot::Sender<()>)]
        Rpc(RpcRequest),
        #[rpc(tx=NoSender)]
        ServerStreaming(ServerStreamingRequest),
        #[rpc(tx=NoSender)]
        BidiStreaming(BidiStreamingRequest),
        #[rpc(tx=NoSender)]
        ClientStreaming(ClientStreamingRequest),
    }
}

/// Use
///
/// TRYBUILD=overwrite cargo test --test smoke
///
/// to update the snapshots
#[test]
#[ignore = "stupid diffs depending on rustc version"]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
