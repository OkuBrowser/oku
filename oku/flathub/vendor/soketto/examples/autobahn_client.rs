// Copyright (c) 2019 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

// Example to be used with the autobahn test suite, a fully automated test
// suite to verify client and server implementations of websocket
// implementation.
//
// Once started, the tests can be executed with: wstest -m fuzzingserver
//
// See https://github.com/crossbario/autobahn-testsuite for details.

use futures::io::{BufReader, BufWriter};
use soketto::{connection, handshake, BoxedError};
use std::str::FromStr;
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};

const SOKETTO_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), BoxedError> {
	let n = num_of_cases().await?;
	for i in 1..=n {
		if let Err(e) = run_case(i).await {
			log::error!("case {}: {:?}", i, e)
		}
	}
	update_report().await?;
	Ok(())
}

async fn num_of_cases() -> Result<usize, BoxedError> {
	let socket = TcpStream::connect("127.0.0.1:9001").await?;
	let mut client = new_client(socket, "/getCaseCount");
	assert!(matches!(client.handshake().await?, handshake::ServerResponse::Accepted { .. }));
	let (_, mut receiver) = client.into_builder().finish();
	let mut data = Vec::new();
	let kind = receiver.receive_data(&mut data).await?;
	assert!(kind.is_text());
	let num = usize::from_str(std::str::from_utf8(&data)?)?;
	log::info!("{} cases to run", num);
	Ok(num)
}

async fn run_case(n: usize) -> Result<(), BoxedError> {
	log::info!("running case {}", n);
	let resource = format!("/runCase?case={}&agent=soketto-{}", n, SOKETTO_VERSION);
	let socket = TcpStream::connect("127.0.0.1:9001").await?;
	let mut client = new_client(socket, &resource);
	assert!(matches!(client.handshake().await?, handshake::ServerResponse::Accepted { .. }));
	let (mut sender, mut receiver) = client.into_builder().finish();
	let mut message = Vec::new();
	loop {
		message.clear();
		match receiver.receive_data(&mut message).await {
			Ok(soketto::Data::Binary(n)) => {
				assert_eq!(n, message.len());
				sender.send_binary_mut(&mut message).await?;
				sender.flush().await?
			}
			Ok(soketto::Data::Text(n)) => {
				assert_eq!(n, message.len());
				sender.send_text(std::str::from_utf8(&message)?).await?;
				sender.flush().await?
			}
			Err(connection::Error::Closed) => return Ok(()),
			Err(e) => return Err(e.into()),
		}
	}
}

async fn update_report() -> Result<(), BoxedError> {
	log::info!("requesting report generation");
	let resource = format!("/updateReports?agent=soketto-{}", SOKETTO_VERSION);
	let socket = TcpStream::connect("127.0.0.1:9001").await?;
	let mut client = new_client(socket, &resource);
	assert!(matches!(client.handshake().await?, handshake::ServerResponse::Accepted { .. }));
	client.into_builder().finish().0.close().await?;
	Ok(())
}

#[cfg(not(feature = "deflate"))]
fn new_client(socket: TcpStream, path: &str) -> handshake::Client<'_, BufReader<BufWriter<Compat<TcpStream>>>> {
	handshake::Client::new(BufReader::new(BufWriter::new(socket.compat())), "127.0.0.1:9001", path)
}

#[cfg(feature = "deflate")]
fn new_client(socket: TcpStream, path: &str) -> handshake::Client<'_, BufReader<BufWriter<Compat<TcpStream>>>> {
	let socket = BufReader::with_capacity(8 * 1024, BufWriter::with_capacity(64 * 1024, socket.compat()));
	let mut client = handshake::Client::new(socket, "127.0.0.1:9001", path);
	let deflate = soketto::extension::deflate::Deflate::new(soketto::Mode::Client);
	client.add_extension(Box::new(deflate));
	client
}
