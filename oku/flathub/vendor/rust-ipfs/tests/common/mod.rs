use rust_ipfs::Node;

/// The way in which nodes are connected to each other; to be used with spawn_nodes.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topology {
    /// no connections
    None,
    /// a > b > c
    Line,
    /// a > b > c > a
    Ring,
    /// a <> b <> c <> a
    Mesh,
    /// a > b, a > c
    Star,
}

#[allow(dead_code)]
pub async fn spawn_nodes<const N: usize>(topology: Topology) -> Vec<Node> {
    let mut nodes = Vec::with_capacity(N);

    for i in 0..N {
        let node = Node::new(i.to_string()).await;
        nodes.push(node);
    }

    match topology {
        Topology::Line | Topology::Ring => {
            for i in 0..(N - 1) {
                nodes[i]
                    .connect(nodes[i + 1].addrs[0].clone())
                    .await
                    .unwrap();
            }
            if topology == Topology::Ring {
                nodes[N - 1]
                    .connect(nodes[0].addrs[0].clone())
                    .await
                    .unwrap();
            }
        }
        Topology::Mesh => {
            for i in 0..N {
                for (j, peer) in nodes.iter().enumerate() {
                    if i != j {
                        //TODO: Determine the cause of `Failed to negotiate transport protocol(s).... Cannot assign requested address (os error 99): Cannot assign requested address (os error 99)`
                        if let Err(_e) = nodes[i].connect(peer.addrs[0].clone()).await {}
                    }
                }
            }
        }
        Topology::Star => {
            for node in nodes.iter().skip(1) {
                nodes[0].connect(node.addrs[0].clone()).await.unwrap();
            }
        }
        Topology::None => {}
    }

    nodes
}
