# `oku-fs`

A distributed file system for use with the Oku browser.

## Build Instructions

### Prerequisites

To build, please install:
* A copy of the [Rust toolchain](https://www.rust-lang.org/tools/install)
    * It is recommended that you install Rust using [`rustup.rs`](https://rustup.rs/), though many Linux distributions also package the Rust toolchain as well.
* [`libfuse`](https://github.com/libfuse/libfuse/)
    * This is required if you intend on building with the `fuse` feature, but is otherwise optional.
    * It is recommended that you obtain this development package from your distribution.

### Commands

After pre-requisites are installed, you may run:
* `cargo build` for debug builds.
* `cargo build --release` for release builds.
* `cargo install --path .` to install.
* Note: If intending on building or installing an executable rather than a library, please specify the intended features by appending `--features="<features separated by commas>"` to the build command.

### Features
* `cli` - A command-line interface for performing file system operations.
* `fuse` - Enables mounting the file system via [FUSE](https://en.wikipedia.org/wiki/Filesystem_in_Userspace).
* Note: If the `cli` feature is not enabled, this software will be installed as a development library.

## Technical Design

Files and directories are stored in replicas implemented as [Iroh](https://www.iroh.computer) documents, allowing them to be shared publicly over [the Mainline DHT](https://en.wikipedia.org/wiki/Mainline_DHT) or directly between Oku file system nodes.

Content discovery occurs over the Mainline DHT. Content discovery happens as follows:
1. A node queries using a replica's ID, being its public key.
2. The node receives an Iroh document ticket.
3. The node uses the ticket to connect to the document swarm and download the document.

#### Hole punching

It may seem unclear how a ticket allows one node to connect to another node behind NAT.
N0, Inc maintains a [network of Iroh nodes and relay servers](https://iroh.network/), with the [Iroh relay servers](https://docs.rs/iroh-net/latest/iroh_net/relay/server/struct.Server.html) facilitating hole punching during document syncing.

The Iroh network's relay servers are currently free. When this is no longer the case, the Oku project will need to host its own relay servers.