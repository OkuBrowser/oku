//! Manage announced peer ids for info_hashes

use std::num::NonZeroUsize;

use crate::common::{Id, SignedAnnounce};

use lru::LruCache;
use rand::Rng;

const CHANCE_SCALE: f32 = 2.0 * (1u32 << 31) as f32;

#[derive(Debug, Clone)]
/// An LRU cache of signed peers announced per info hashes.
///
/// Read [BEP_????](https://github.com/Nuhvi/mainline/blob/main/beps/bep_signed_peers.rst) for more information.
pub struct SignedPeersStore {
    info_hashes: LruCache<Id, LruCache<[u8; 32], SignedAnnounce>>,
    max_peers: NonZeroUsize,
}

impl SignedPeersStore {
    /// Create a new store of peers announced on info hashes.
    pub fn new(max_info_hashes: NonZeroUsize, max_peers: NonZeroUsize) -> Self {
        Self {
            info_hashes: LruCache::new(max_info_hashes),
            max_peers,
        }
    }

    /// Add a peer for an info hash.
    pub fn add_peer(&mut self, info_hash: Id, peer: SignedAnnounce) {
        if let Some(info_hash_lru) = self.info_hashes.get_mut(&info_hash) {
            info_hash_lru.put(*peer.key(), peer);
        } else {
            let mut info_hash_lru = LruCache::new(self.max_peers);
            info_hash_lru.put(*peer.key(), peer);
            self.info_hashes.put(info_hash, info_hash_lru);
        };
    }

    /// Returns a random set of peers per an info hash.
    pub fn get_random_peers(&mut self, info_hash: &Id) -> Option<Vec<SignedAnnounce>> {
        if let Some(info_hash_lru) = self.info_hashes.get(info_hash) {
            let size = info_hash_lru.len();
            let target_size = 10;

            if size == 0 {
                return None;
            }
            if size < target_size {
                return Some(
                    info_hash_lru
                        .iter()
                        .map(|n| n.1.to_owned())
                        .collect::<Vec<_>>(),
                );
            }

            let mut results = Vec::with_capacity(20);

            let mut chunk = vec![0_u8; info_hash_lru.iter().len() * 4];
            rand::rng().fill_bytes(chunk.as_mut_slice());

            for (index, (_, signed_announce)) in info_hash_lru.iter().enumerate() {
                // Calculate the chance of adding the current item based on remaining items and slots
                let remaining_slots = target_size - results.len();
                let remaining_items = info_hash_lru.len() - index;
                let current_chance =
                    ((remaining_slots as f32 / remaining_items as f32) * CHANCE_SCALE) as u32;

                // Get random integer from the chunk
                let rand_int =
                    u32::from_le_bytes(chunk[index..index + 4].try_into().expect("infallible"));

                // Randomly decide to add the item based on the current chance
                if rand_int < current_chance {
                    results.push(signed_announce.clone());
                    if results.len() == target_size {
                        break;
                    }
                }
            }

            return Some(results);
        }

        None
    }
}

#[cfg(test)]
mod test {
    use ed25519_dalek::SigningKey;

    use super::*;

    fn make_signer() -> SigningKey {
        let mut secret_key = [0; 32];
        rand::rng().fill_bytes(&mut secret_key);
        SigningKey::from_bytes(&secret_key)
    }

    fn make_peer(signer: &SigningKey, target: &Id) -> SignedAnnounce {
        SignedAnnounce::new(signer, target)
    }

    #[test]
    fn max_info_hashes() {
        let mut store = SignedPeersStore::new(
            NonZeroUsize::new(1).unwrap(),
            NonZeroUsize::new(100).unwrap(),
        );

        let info_hash_a = Id::random();
        let info_hash_b = Id::random();

        let signer = make_signer();

        store.add_peer(info_hash_a, make_peer(&signer, &info_hash_a));
        store.add_peer(info_hash_b, make_peer(&signer, &info_hash_b));

        assert_eq!(store.info_hashes.len(), 1);
        assert!(store.get_random_peers(&info_hash_b).is_some());
        assert!(store.get_random_peers(&info_hash_a).is_none());
    }

    #[test]
    fn all_peers() {
        let mut store =
            SignedPeersStore::new(NonZeroUsize::new(1).unwrap(), NonZeroUsize::new(2).unwrap());

        let info_hash = Id::random();

        let signer1 = make_signer();
        let signer2 = make_signer();
        let signer3 = make_signer();

        store.add_peer(info_hash, make_peer(&signer1, &info_hash));
        store.add_peer(info_hash, make_peer(&signer2, &info_hash));
        store.add_peer(info_hash, make_peer(&signer3, &info_hash));

        assert_eq!(
            store
                .get_random_peers(&info_hash)
                .unwrap()
                .iter()
                .map(|p| p.key())
                .collect::<Vec<_>>(),
            vec![
                signer3.verifying_key().as_bytes(),
                signer2.verifying_key().as_bytes()
            ]
        );
    }

    #[test]
    fn random_peers_subset() {
        let mut store = SignedPeersStore::new(
            NonZeroUsize::new(1).unwrap(),
            NonZeroUsize::new(200).unwrap(),
        );

        let info_hash = Id::random();

        for _ in 0..200 {
            store.add_peer(info_hash, make_peer(&make_signer(), &info_hash))
        }

        assert_eq!(store.info_hashes.get(&info_hash).unwrap().len(), 200);

        let sample = store.get_random_peers(&info_hash).unwrap();

        assert_eq!(sample.len(), 10);
    }
}
