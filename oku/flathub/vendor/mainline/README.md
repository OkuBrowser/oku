# Mainline

Simple, robust, BitTorrent's [Mainline](https://en.wikipedia.org/wiki/Mainline_DHT) DHT implementation.

This library is focused on being the best and simplest Rust client for Mainline, especially focused on reliable and fast time-to-first-response.

It should work as a routing / storing node (server mode) as well, and has been running in production for many months without an issue. 
However if you are concerned about spam or DoS, you should consider implementing [rate limiting](#rate-limiting).

**[API Docs](https://docs.rs/mainline/latest/mainline/)**

## Getting started

Check the [Examples](https://github.com/Pubky/mainline/tree/main/examples).

## Features

### Client

Running as a client, means you can store and query for values on the DHT, but not accept any incoming requests.

```rust
use mainline::Dht;

let dht = Dht::client().unwrap();
```

Supported BEPs:
- [x] [BEP_0005 DHT Protocol](https://www.bittorrent.org/beps/bep_0005.html)
- [x] [BEP_0042 DHT Security extension](https://www.bittorrent.org/beps/bep_0042.html)
- [x] [BEP_0043 Read-only DHT Nodes](https://www.bittorrent.org/beps/bep_0043.html)
- [x] [BEP_0044 Storing arbitrary data in the DHT](https://www.bittorrent.org/beps/bep_0044.html)

This implementation also includes [measures against Vertical Sybil Attacks](./docs/sybil-resistance.md).

### Server

Running as a server is the same as a client, but you also respond to incoming requests and serve as a routing and storing node, supporting the general routing of the DHT, and contributing to the storage capacity of the DHT.

```rust
use mainline::Dht;

let dht = Dht::server().unwrap(); // or `Dht::builder::server_mode().build();` 
```

Supported BEPs:
- [x] [BEP_0005 DHT Protocol](https://www.bittorrent.org/beps/bep_0005.html)
- [x] [BEP_0042 DHT Security extension](https://www.bittorrent.org/beps/bep_0042.html)
- [x] [BEP_0043 Read-only DHT Nodes](https://www.bittorrent.org/beps/bep_0043.html)
- [x] [BEP_0044 Storing arbitrary data in the DHT](https://www.bittorrent.org/beps/bep_0044.html)

#### Rate limiting

The server implementation has no rate-limiting, you can run your own [request filter](./examples/request_filter.rs) and apply your custom rate-limiting. 
However, that limit/block will only apply _after_ parsing incoming messages, and it won't affect handling incoming responses.

### Adaptive mode

The default Adaptive mode will start the node in client mode, and after 15 minutes of running with a publicly accessible address,
it will switch to server mode. This way nodes that can serve as routing nodes (accessible and less likely to churn), serve as such.

If you want to explicitly start in Server mode, because you know you are not running behind firewall,
you can call `Dht::builder().server_mode().build()`, and you can optionally add your known public ip so the node doesn't have to depend on,
votes from responding nodes: `Dht::builder().server_mode().public_ip().build()`.

## Acknowledgment

This implementation was possible thanks to [Webtorrent's Bittorrent-dht](https://github.com/webtorrent/bittorrent-dht) as a reference, 
and [Rustydht-lib](https://github.com/raptorswing/rustydht-lib) that saved me a lot of time, especially at the serialization and deserialization of Bencode messages.
