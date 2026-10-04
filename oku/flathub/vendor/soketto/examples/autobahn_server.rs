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
// Once started, the tests can be executed with: wstest -m fuzzingclient
//
// See https://github.com/crossbario/autobahn-testsuite for details.

use futures::io::{BufReader, BufWriter};
use soketto::{connection, handshake, BoxedError};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::{wrappers::TcpListenerStream, StreamExt};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};
#[tokio::main]
async fn main() -> Result<(), BoxedError> {
	let listener = TcpListener::bind("127.0.0.1:9001").await?;
	let mut incoming = TcpListenerStream::new(listener);
	while let Some(socket) = incoming.next().await {
		let mut server = new_server(socket?);
		let key = {
			let req = server.receive_request().await?;
			req.key()
		};
		let accept = handshake::server::Response::Accept { key, protocol: None };
		server.send_response(&accept).await?;
		let (mut sender, mut receiver) = server.into_builder().finish();
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
					if let Ok(txt) = std::str::from_utf8(&message) {
						sender.send_text(txt).await?;
						sender.flush().await?
					} else {
						break;
					}
				}
				Err(connection::Error::Closed) => break,
				Err(e) => {
					log::error!("connection error: {}", e);
					break;
				}
			}
		}
	}
	Ok(())
}

#[cfg(not(feature = "deflate"))]
fn new_server<'a>(socket: TcpStream) -> handshake::Server<'a, BufReader<BufWriter<Compat<TcpStream>>>> {
	handshake::Server::new(BufReader::new(BufWriter::new(socket.compat())))
}

#[cfg(feature = "deflate")]
fn new_server<'a>(socket: TcpStream) -> handshake::Server<'a, BufReader<BufWriter<Compat<TcpStream>>>> {
	let socket = BufReader::with_capacity(8 * 1024, BufWriter::with_capacity(16 * 1024, socket.compat()));
	let mut server = handshake::Server::new(socket);
	let deflate = soketto::extension::deflate::Deflate::new(soketto::Mode::Server);
	server.add_extension(Box::new(deflate));
	server
}
