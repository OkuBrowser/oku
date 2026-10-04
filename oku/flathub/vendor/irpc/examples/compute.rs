use std::{
    io::{self, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};

use anyhow::bail;
use futures_buffered::BufferedStreamExt;
use irpc::{
    Client, Request, WithChannels,
    channel::{mpsc, oneshot},
    rpc::{RemoteService, listen},
    rpc_requests,
    util::{make_client_endpoint, make_server_endpoint},
};
use n0_future::{
    stream::StreamExt,
    task::{self, AbortOnDropHandle},
};
use serde::{Deserialize, Serialize};
use thousands::Separable;
use tracing::trace;

// Define the protocol and message enums using the macro
#[rpc_requests(message = ComputeMessage)]
#[derive(Serialize, Deserialize, Debug)]
enum ComputeProtocol {
    #[rpc(tx=oneshot::Sender<u128>)]
    Sqr(Sqr),
    #[rpc(rx=mpsc::Receiver<i64>, tx=oneshot::Sender<i64>)]
    Sum(Sum),
    #[rpc(tx=mpsc::Sender<u64>)]
    Fibonacci(Fibonacci),
    #[rpc(rx=mpsc::Receiver<u64>, tx=mpsc::Sender<u64>)]
    Multiply(Multiply),
}

// Define ComputeProtocol sub-messages
#[derive(Debug, Serialize, Deserialize)]
struct Sqr {
    num: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Sum;

#[derive(Debug, Serialize, Deserialize)]
struct Fibonacci {
    max: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Multiply {
    initial: u64,
}

// The actor that processes requests
struct ComputeActor {
    recv: irpc::channel::mpsc::Receiver<ComputeMessage>,
}

impl ComputeActor {
    pub fn local() -> ComputeApi {
        let (tx, rx) = irpc::channel::mpsc::channel(128);
        let actor = Self { recv: rx };
        n0_future::task::spawn(actor.run());
        ComputeApi {
            inner: Client::local(tx),
        }
    }

    async fn run(mut self) {
        while let Ok(Some(msg)) = self.recv.recv().await {
            n0_future::task::spawn(async move {
                if let Err(cause) = Self::handle(msg).await {
                    eprintln!("Error: {cause}");
                }
            });
        }
    }

    async fn handle(msg: ComputeMessage) -> io::Result<()> {
        match msg {
            ComputeMessage::Sqr(sqr) => {
                trace!("sqr {:?}", sqr);
                let WithChannels {
                    tx, inner, span, ..
                } = sqr;
                let _entered = span.enter();
                let result = (inner.num as u128) * (inner.num as u128);
                tx.send(result).await?;
            }
            ComputeMessage::Sum(sum) => {
                trace!("sum {:?}", sum);
                let WithChannels { rx, tx, span, .. } = sum;
                let _entered = span.enter();
                let mut receiver = rx;
                let mut total = 0;
                while let Some(num) = receiver.recv().await? {
                    total += num;
                }
                tx.send(total).await?;
            }
            ComputeMessage::Fibonacci(fib) => {
                trace!("fibonacci {:?}", fib);
                let WithChannels {
                    tx, inner, span, ..
                } = fib;
                let _entered = span.enter();
                let sender = tx;
                let mut a = 0u64;
                let mut b = 1u64;
                while a <= inner.max {
                    sender.send(a).await?;
                    let next = a + b;
                    a = b;
                    b = next;
                }
            }
            ComputeMessage::Multiply(mult) => {
                trace!("multiply {:?}", mult);
                let WithChannels {
                    rx,
                    tx,
                    inner,
                    span,
                    ..
                } = mult;
                let _entered = span.enter();
                let mut receiver = rx;
                let sender = tx;
                let multiplier = inner.initial;
                while let Some(num) = receiver.recv().await? {
                    sender.send(multiplier * num).await?;
                }
            }
        }
        Ok(())
    }
}
// The API for interacting with the ComputeService
#[derive(Clone)]
struct ComputeApi {
    inner: Client<ComputeProtocol>,
}

impl ComputeApi {
    pub fn connect(endpoint: noq::Endpoint, addr: SocketAddr) -> anyhow::Result<ComputeApi> {
        Ok(ComputeApi {
            inner: Client::noq(endpoint, addr),
        })
    }

    pub fn listen(&self, endpoint: noq::Endpoint) -> anyhow::Result<AbortOnDropHandle<()>> {
        let Some(local) = self.inner.as_local() else {
            bail!("cannot listen on a remote service");
        };
        let handler = ComputeProtocol::remote_handler(local);
        Ok(AbortOnDropHandle::new(task::spawn(listen(
            endpoint, handler,
        ))))
    }

    pub async fn sqr(&self, num: u64) -> anyhow::Result<oneshot::Receiver<u128>> {
        let msg = Sqr { num };
        match self.inner.request().await? {
            Request::Local(request) => {
                let (tx, rx) = oneshot::channel();
                request.send((msg, tx)).await?;
                Ok(rx)
            }
            Request::Remote(request) => {
                let (_tx, rx) = request.write(msg).await?;
                Ok(rx.into())
            }
        }
    }

    pub async fn sum(&self) -> anyhow::Result<(mpsc::Sender<i64>, oneshot::Receiver<i64>)> {
        let msg = Sum;
        match self.inner.request().await? {
            Request::Local(request) => {
                let (num_tx, num_rx) = mpsc::channel(10);
                let (sum_tx, sum_rx) = oneshot::channel();
                request.send((msg, sum_tx, num_rx)).await?;
                Ok((num_tx, sum_rx))
            }
            Request::Remote(request) => {
                let (tx, rx) = request.write(msg).await?;
                Ok((tx.into(), rx.into()))
            }
        }
    }

    pub async fn fibonacci(&self, max: u64) -> anyhow::Result<mpsc::Receiver<u64>> {
        let msg = Fibonacci { max };
        match self.inner.request().await? {
            Request::Local(request) => {
                let (tx, rx) = mpsc::channel(128);
                request.send((msg, tx)).await?;
                Ok(rx)
            }
            Request::Remote(request) => {
                let (_tx, rx) = request.write(msg).await?;
                Ok(rx.into())
            }
        }
    }

    pub async fn multiply(
        &self,
        initial: u64,
    ) -> anyhow::Result<(mpsc::Sender<u64>, mpsc::Receiver<u64>)> {
        let msg = Multiply { initial };
        match self.inner.request().await? {
            Request::Local(request) => {
                let (in_tx, in_rx) = mpsc::channel(128);
                let (out_tx, out_rx) = mpsc::channel(128);
                request.send((msg, out_tx, in_rx)).await?;
                Ok((in_tx, out_rx))
            }
            Request::Remote(request) => {
                let (tx, rx) = request.write(msg).await?;
                Ok((tx.into(), rx.into()))
            }
        }
    }
}

// Local usage example
async fn local() -> anyhow::Result<()> {
    let api = ComputeActor::local();

    // Test Sqr
    let rx = api.sqr(5).await?;
    println!("Local: 5^2 = {}", rx.await?);

    // Test Sum
    let (tx, rx) = api.sum().await?;
    tx.send(1).await?;
    tx.send(2).await?;
    tx.send(3).await?;
    drop(tx);
    println!("Local: sum of [1, 2, 3] = {}", rx.await?);

    // Test Fibonacci
    let mut rx = api.fibonacci(10).await?;
    print!("Local: Fibonacci up to 10 = ");
    while let Some(num) = rx.recv().await? {
        print!("{num} ");
    }
    println!();

    // Test Multiply
    let (in_tx, mut out_rx) = api.multiply(3).await?;
    in_tx.send(2).await?;
    in_tx.send(4).await?;
    in_tx.send(6).await?;
    drop(in_tx);
    print!("Local: 3 * [2, 4, 6] = ");
    while let Some(num) = out_rx.recv().await? {
        print!("{num} ");
    }
    println!();

    Ok(())
}

fn remote_api() -> anyhow::Result<(ComputeApi, AbortOnDropHandle<()>)> {
    let port = 10114;
    let (server, cert) =
        make_server_endpoint(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port).into())?;
    let client =
        make_client_endpoint(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0).into(), &[&cert])?;
    let compute = ComputeActor::local();
    let handle = compute.listen(server)?;
    let api = ComputeApi::connect(client, SocketAddrV4::new(Ipv4Addr::LOCALHOST, port).into())?;
    Ok((api, handle))
}

// Remote usage example
async fn remote() -> anyhow::Result<()> {
    let (api, handle) = remote_api()?;

    // Test Sqr
    let rx = api.sqr(4).await?;
    println!("Remote: 4^2 = {}", rx.await?);

    // Test Sum
    let (tx, rx) = api.sum().await?;
    tx.send(4).await?;
    tx.send(5).await?;
    tx.send(6).await?;
    drop(tx);
    println!("Remote: sum of [4, 5, 6] = {}", rx.await?);

    // Test Fibonacci
    let mut rx = api.fibonacci(20).await?;
    print!("Remote: Fibonacci up to 20 = ");
    while let Some(num) = rx.recv().await? {
        print!("{num} ");
    }
    println!();

    // Test Multiply
    let (in_tx, mut out_rx) = api.multiply(5).await?;
    in_tx.send(1).await?;
    in_tx.send(2).await?;
    in_tx.send(3).await?;
    drop(in_tx);
    print!("Remote: 5 * [1, 2, 3] = ");
    while let Some(num) = out_rx.recv().await? {
        print!("{num} ");
    }
    println!();

    drop(handle);
    Ok(())
}

// Benchmark function using the new ComputeApi
async fn bench(api: ComputeApi, n: u64) -> anyhow::Result<()> {
    // Individual RPCs (sequential)
    {
        let mut sum = 0;
        let t0 = std::time::Instant::now();
        for i in 0..n {
            sum += api.sqr(i).await?.await?;
            if i % 10000 == 0 {
                print!(".");
                io::stdout().flush()?;
            }
        }
        let rps = ((n as f64) / t0.elapsed().as_secs_f64()).round() as u64;
        assert_eq!(sum, sum_of_squares(n));
        clear_line()?;
        println!("RPC seq {} rps", rps.separate_with_underscores());
    }

    // Parallel RPCs
    {
        let t0 = std::time::Instant::now();
        let api = api.clone();
        let reqs = n0_future::stream::iter((0..n).map(move |i| {
            let api = api.clone();
            async move { anyhow::Ok(api.sqr(i).await?.await?) }
        }));
        let resp: Vec<_> = reqs.buffered_unordered(32).try_collect().await?;
        let sum = resp.into_iter().sum::<u128>();
        let rps = ((n as f64) / t0.elapsed().as_secs_f64()).round() as u64;
        assert_eq!(sum, sum_of_squares(n));
        clear_line()?;
        println!("RPC par {} rps", rps.separate_with_underscores());
    }

    // Sequential streaming (using Multiply instead of MultiplyUpdate)
    {
        let t0 = std::time::Instant::now();
        let (send, mut recv) = api.multiply(2).await?;
        let handle = tokio::task::spawn(async move {
            for i in 0..n {
                send.send(i).await?;
            }
            Ok::<(), io::Error>(())
        });
        let mut sum = 0;
        let mut i = 0;
        while let Some(res) = recv.recv().await? {
            sum += res;
            if i % 10000 == 0 {
                print!(".");
                io::stdout().flush()?;
            }
            i += 1;
        }
        let rps = ((n as f64) / t0.elapsed().as_secs_f64()).round() as u64;
        assert_eq!(sum, (0..n).map(|x| x * 2).sum::<u64>());
        clear_line()?;
        println!("bidi seq {} rps", rps.separate_with_underscores());
        handle.await??;
    }

    Ok(())
}

// Helper function to compute the sum of squares
fn sum_of_squares(n: u64) -> u128 {
    (0..n).map(|x| (x * x) as u128).sum()
}

// Helper function to clear the current line
fn clear_line() -> io::Result<()> {
    io::stdout().write_all(b"\r\x1b[K")?;
    io::stdout().flush()?;
    Ok(())
}

// Simple benchmark sending oneshot senders via an mpsc channel
pub async fn reference_bench(n: u64) -> anyhow::Result<()> {
    // Create an mpsc channel to send oneshot senders
    let (tx, mut rx) = tokio::sync::mpsc::channel::<tokio::sync::oneshot::Sender<u64>>(32);

    // Spawn a task to respond to all oneshot senders
    tokio::spawn(async move {
        while let Some(sender) = rx.recv().await {
            // Immediately send a fixed response (42) back through the oneshot sender
            sender.send(42).ok();
        }
        Ok::<(), io::Error>(())
    });

    // Sequential oneshot sends
    {
        let mut sum = 0;
        let t0 = std::time::Instant::now();
        for i in 0..n {
            let (send, recv) = tokio::sync::oneshot::channel();
            tx.send(send).await?;
            sum += recv.await?;
            if i % 10000 == 0 {
                print!(".");
                io::stdout().flush()?;
            }
        }
        let rps = ((n as f64) / t0.elapsed().as_secs_f64()).round() as u64;
        assert_eq!(sum, 42 * n); // Each response is 42
        clear_line()?;
        println!("Reference seq {} rps", rps.separate_with_underscores());
    }

    // Parallel oneshot sends
    {
        let t0 = std::time::Instant::now();
        let reqs = n0_future::stream::iter((0..n).map(|_| async {
            let (send, recv) = tokio::sync::oneshot::channel();
            tx.send(send).await?;
            anyhow::Ok(recv.await?)
        }));
        let resp: Vec<_> = reqs.buffered_unordered(32).try_collect().await?;
        let sum = resp.into_iter().sum::<u64>();
        let rps = ((n as f64) / t0.elapsed().as_secs_f64()).round() as u64;
        assert_eq!(sum, 42 * n); // Each response is 42
        clear_line()?;
        println!("Reference par {} rps", rps.separate_with_underscores());
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    println!("Local use");
    local().await?;
    println!("Remote use");
    remote().await?;

    println!("Local bench");
    let api = ComputeActor::local();
    bench(api, 100000).await?;

    let (api, handle) = remote_api()?;
    println!("Remote bench");
    bench(api, 100000).await?;
    drop(handle);

    println!("Reference bench");
    reference_bench(100000).await?;
    Ok(())
}
