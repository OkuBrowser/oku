use std::{collections::HashSet, convert::TryInto};

use crate::{common::MAX_BUCKET_SIZE_K, Id, Node};

#[derive(Debug, Clone)]
/// Manage closest nodes found in a query.
///
/// Useful to estimate the Dht size.
pub struct ClosestNodes {
    target: Id,
    nodes: Vec<Node>,
}

impl ClosestNodes {
    /// Create a new instance of [ClosestNodes].
    pub fn new(target: Id) -> Self {
        Self {
            target,
            nodes: Vec::with_capacity(200),
        }
    }

    // === Getters ===

    /// Returns the target of the query for these closest nodes.
    pub fn target(&self) -> Id {
        self.target
    }

    /// Returns a slice of the nodes array.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Returns the number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if there are no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    // === Public Methods ===

    /// Add a node.
    pub fn add(&mut self, node: Node) {
        let seek = node.id().xor(&self.target);

        if node.already_exists(&self.nodes) {
            return;
        }

        if let Err(pos) = self.nodes.binary_search_by(|prope| {
            if prope.is_secure() && !node.is_secure() {
                std::cmp::Ordering::Less
            } else if !prope.is_secure() && node.is_secure() {
                std::cmp::Ordering::Greater
            } else if prope.id() == node.id() {
                std::cmp::Ordering::Equal
            } else {
                prope.id().xor(&self.target).cmp(&seek)
            }
        }) {
            self.nodes.insert(pos, node)
        }
    }

    /// Take enough nodes closest to the target, until the following are satisfied:
    /// 1. At least the closest `k` nodes (20).
    /// 2. The last node should be at a distance `edk` which is the expected distance of the 20th
    ///    node given previous estimations of the DHT size.
    /// 3. The number of subnets with unique 6 bits prefix in nodes ipv4 addresses match or exceeds
    ///    the average from previous queries.
    ///
    /// If one or more of these conditions are not met, then we just take all responding nodes
    /// and store data at them.
    pub fn take_until_secure(
        &self,
        previous_dht_size_estimate: usize,
        average_subnets: usize,
    ) -> &[Node] {
        let mut until_secure = 0;

        // 20 / dht_size_estimate == expected_dk / ID space
        // so expected_dk = 20 * ID space / dht_size_estimate
        let expected_dk =
            (20.0 * u128::MAX as f64 / (previous_dht_size_estimate as f64 + 1.0)) as u128;

        let mut subnets = HashSet::new();

        for node in &self.nodes {
            let distance = distance(&self.target, node);

            subnets.insert(subnet(node));

            if distance >= expected_dk && subnets.len() >= average_subnets {
                break;
            }

            until_secure += 1;
        }

        &self.nodes[0..until_secure.max(MAX_BUCKET_SIZE_K).min(self.nodes().len())]
    }

    /// Count the number of subnets with unique 6 bits prefix in ipv4
    pub fn subnets_count(&self) -> u8 {
        if self.nodes.is_empty() {
            return 20;
        }

        let mut subnets = HashSet::new();

        for node in self.nodes.iter().take(MAX_BUCKET_SIZE_K) {
            subnets.insert(subnet(node));
        }

        subnets.len() as u8
    }

    /// An estimation of the Dht from the distribution of closest nodes
    /// responding to a query.
    ///
    /// [Read more](https://github.com/pubky/mainline/blob/main/docs/dht_size_estimate.md)
    pub fn dht_size_estimate(&self) -> f64 {
        dht_size_estimate(
            self.nodes
                .iter()
                .take(MAX_BUCKET_SIZE_K)
                .map(|node| distance(&self.target, node)),
        )
    }
}

fn subnet(node: &Node) -> u8 {
    ((node.address().ip().to_bits() >> 26) & 0b0011_1111) as u8
}

fn distance(target: &Id, node: &Node) -> u128 {
    let xor = node.id().xor(target);

    // Round up the lower 4 bytes to get a u128 from u160.
    u128::from_be_bytes(xor.as_bytes()[0..16].try_into().expect("infallible"))
}

fn dht_size_estimate<I>(distances: I) -> f64
where
    I: IntoIterator<Item = u128>,
{
    let mut sum = 0.0;
    let mut count = 0;

    // Ignoring the first node, as that gives the best result in simulations.
    for distance in distances {
        count += 1;

        sum += count as f64 * distance as f64;
    }

    if count == 0 {
        return 0.0;
    }

    let lsq_constant = (count * (count + 1) * (2 * count + 1) / 6) as f64;

    lsq_constant * u128::MAX as f64 / sum
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, net::SocketAddrV4, str::FromStr, sync::Arc, time::Instant};

    use crate::common::NodeInner;

    use super::*;

    #[test]
    fn add_sorted_by_id() {
        let target = Id::random();

        let mut closest_nodes = ClosestNodes::new(target);

        for i in 0..100 {
            let node = Node::unique(i);
            closest_nodes.add(node.clone());
            closest_nodes.add(node);
        }

        assert_eq!(closest_nodes.nodes().len(), 100);

        let distances = closest_nodes
            .nodes()
            .iter()
            .map(|n| n.id().distance(&target))
            .collect::<Vec<_>>();

        let mut sorted = distances.clone();
        sorted.sort();

        assert_eq!(sorted, distances);
    }

    #[test]
    fn order_by_secure_id() {
        let unsecure = Node::random();
        let secure = Node(Arc::new(NodeInner {
            id: Id::from_str("5a3ce9c14e7a08645677bbd1cfe7d8f956d53256").unwrap(),
            address: SocketAddrV4::new([21, 75, 31, 124].into(), 0),
            token: None,
            last_seen: Instant::now(),
        }));

        let mut closest_nodes = ClosestNodes::new(*unsecure.id());

        closest_nodes.add(unsecure.clone());
        closest_nodes.add(secure.clone());

        assert_eq!(closest_nodes.nodes(), vec![secure, unsecure])
    }

    #[test]
    fn take_until_expected_distance_to_20th_node() {
        let target = Id::random();
        let dht_size_estimate = 200;

        let mut closest_nodes = ClosestNodes::new(target);

        let target_bytes = target.as_bytes();

        for i in 0..dht_size_estimate {
            let node = Node::unique(i);
            closest_nodes.add(node);
        }

        let mut sybil = ClosestNodes::new(target);

        for _ in 0..20 {
            let mut bytes = target_bytes.to_vec();
            bytes[18..].copy_from_slice(&Id::random().as_bytes()[18..]);
            let node = Node::new(Id::random(), SocketAddrV4::new(0.into(), 0));

            sybil.add(node.clone());
            closest_nodes.add(node);
        }

        let closest = closest_nodes.take_until_secure(dht_size_estimate, 0);

        assert!((closest.len() - sybil.nodes().len()) > 10);
    }

    #[test]
    fn simulation() {
        let lookups = 4;
        let acceptable_margin = 0.2;
        let sims = 10;
        let dht_size = 2500_f64;

        let mean = (0..sims)
            .map(|_| simulate(dht_size as usize, lookups) as f64)
            .sum::<f64>()
            / (sims as f64);

        let margin = (mean - dht_size).abs() / dht_size;

        assert!(margin <= acceptable_margin);
    }

    fn simulate(dht_size: usize, lookups: usize) -> usize {
        let mut nodes = BTreeMap::new();
        for i in 0..dht_size {
            let node = Node::unique(i);
            nodes.insert(*node.id(), node);
        }

        (0..lookups)
            .map(|_| {
                let target = Id::random();

                let mut closest_nodes = ClosestNodes::new(target);

                for (_, node) in nodes.range(target..).take(100) {
                    closest_nodes.add(node.clone())
                }
                for (_, node) in nodes.range(..target).rev().take(100) {
                    closest_nodes.add(node.clone())
                }

                let estimate = closest_nodes.dht_size_estimate();

                estimate as usize
            })
            .sum::<usize>()
            / lookups
    }
}
