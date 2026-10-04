#![cfg(feature = "noq_endpoint_setup")]

use std::io::{self, ErrorKind};

use irpc::{
    channel::{
        SendError,
        oneshot::{self, RecvError},
    },
    util::AsyncWriteVarintExt,
};
use n0_error::e;
use noq::Endpoint;
use testresult::TestResult;

mod common;
use common::*;

async fn vec_receiver(server: Endpoint) -> Result<(), RecvError> {
    let conn = server
        .accept()
        .await
        .unwrap()
        .await
        .map_err(|err| e!(RecvError::Io, err.into()))?;
    let (_, recv) = conn
        .accept_bi()
        .await
        .map_err(|err| e!(RecvError::Io, err.into()))?;
    let recv = oneshot::Receiver::<Vec<u8>>::from(recv);
    recv.await?;
    Err(e!(RecvError::Io, io::ErrorKind::UnexpectedEof.into()))
}

/// Checks that the max message size is enforced on the sender side and that errors are propagated to the receiver side.
#[tokio::test]
async fn oneshot_max_message_size_send() -> TestResult<()> {
    let (server, client, server_addr) = create_connected_endpoints()?;
    let server = tokio::spawn(vec_receiver(server));
    let conn = client.connect(server_addr, "localhost")?.await?;
    let (send, _) = conn.open_bi().await?;
    let send = oneshot::Sender::<Vec<u8>>::from(send);
    // this one should fail!
    let Err(cause) = send.send(vec![0u8; 1024 * 1024 * 32]).await else {
        panic!("client should have failed due to max message size");
    };
    assert!(matches!(cause, SendError::MaxMessageSizeExceeded { .. }));
    let Err(cause) = server.await? else {
        panic!("server should have failed due to max message size");
    };
    assert!(
        matches!(cause, RecvError::Io { source, .. } if source.kind() == ErrorKind::ConnectionReset)
    );
    Ok(())
}

/// Checks that the max message size is enforced on receiver side.
#[tokio::test]
async fn oneshot_max_message_size_recv() -> TestResult<()> {
    let (server, client, server_addr) = create_connected_endpoints()?;
    let server = tokio::spawn(vec_receiver(server));
    let conn = client.connect(server_addr, "localhost")?.await?;
    let (mut send, _) = conn.open_bi().await?;
    // this one should fail on receive!
    send.write_length_prefixed(vec![0u8; 1024 * 1024 * 32])
        .await
        .ok();
    let Err(cause) = server.await? else {
        panic!("server should have failed due to max message size");
    };
    assert!(matches!(cause, RecvError::MaxMessageSizeExceeded { .. }));
    Ok(())
}

async fn noser_receiver(server: Endpoint) -> Result<(), RecvError> {
    let conn = server
        .accept()
        .await
        .unwrap()
        .await
        .map_err(|err| e!(RecvError::Io, err.into()))?;
    let (_, recv) = conn
        .accept_bi()
        .await
        .map_err(|err| e!(RecvError::Io, err.into()))?;
    let recv = oneshot::Receiver::<NoSer>::from(recv);
    recv.await?;
    Err(e!(RecvError::Io, io::ErrorKind::UnexpectedEof.into()))
}

/// Checks that trying to send a message that cannot be serialized results in an error on the sender side and a connection reset on the receiver side.
#[tokio::test]
async fn oneshot_serialize_error_send() -> TestResult<()> {
    let (server, client, server_addr) = create_connected_endpoints()?;
    let server = tokio::spawn(noser_receiver(server));
    let conn = client.connect(server_addr, "localhost")?.await?;
    let (send, _) = conn.open_bi().await?;
    let send = oneshot::Sender::<NoSer>::from(send);
    // this one should fail!
    let Err(cause) = send.send(NoSer(1)).await else {
        panic!("client should have failed due to serialization error");
    };
    assert!(
        matches!(cause, SendError::Io { source, .. } if source.kind() == ErrorKind::InvalidData)
    );
    let Err(cause) = server.await? else {
        panic!("server should have failed due to serialization error");
    };
    println!("Server error: {cause:?}");
    assert!(
        matches!(cause, RecvError::Io { source, .. } if source.kind() == ErrorKind::ConnectionReset)
    );
    Ok(())
}

#[tokio::test]
async fn oneshot_serialize_error_recv() -> TestResult<()> {
    let (server, client, server_addr) = create_connected_endpoints()?;
    let server = tokio::spawn(noser_receiver(server));
    let conn = client.connect(server_addr, "localhost")?.await?;
    let (mut send, _) = conn.open_bi().await?;
    // this one should fail on receive!
    send.write_length_prefixed(1u64).await?;
    send.finish()?;
    let Err(cause) = server.await? else {
        panic!("server should have failed due to serialization error");
    };
    println!("Server error: {cause:?}");
    assert!(
        matches!(cause, RecvError::Io { source, .. } if source.kind() == ErrorKind::InvalidData)
    );
    Ok(())
}
