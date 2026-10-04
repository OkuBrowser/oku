use std::collections::BTreeMap;

use proptest::prelude::*;

use super::*;

/// Randomly permutes an iterator such that each element is displaced by at most `k` positions.
pub fn lag_permute<I: Iterator>(iter: I, k: usize) -> impl Iterator<Item = I::Item> {
    let mut source = iter;
    let mut buffer = Vec::with_capacity(k + 1);
    let mut rng = rand::rng();

    std::iter::from_fn(move || {
        buffer.extend((&mut source).take(k + 1 - buffer.len()));

        if buffer.is_empty() {
            return None;
        }

        Some(buffer.swap_remove(rng.random_range(0..buffer.len())))
    })
}

#[test]
fn test_usage() {
    let elements = lag_permute(0..10000, 100).collect::<Vec<_>>();
    let mut reference = BTreeMap::<u64, u64>::new();
    let mut pb = SortedIndexBuffer::<u64>::default();
    let d = 100;
    let add = elements
        .iter()
        .map(|x| Some(*x))
        .chain(std::iter::repeat_n(None, d));
    let remove = std::iter::repeat_n(None, d).chain(elements.iter().map(|x| Some(*x)));
    for (a, r) in add.zip(remove) {
        if let Some(i) = a {
            pb.insert(i, i * 10);
            reference.insert(i, i * 10);
        }
        if let Some(i) = r {
            let v1 = pb.remove(i);
            let v2 = reference.remove(&i);
            assert_eq!(v1, v2);
        }
        assert_same(pb.iter(), reference.iter().map(|(k, v)| (*k, v)));
        pb.check_invariants_expensive();
    }
    assert!(reference.is_empty());
    assert!(pb.is_empty());
}

#[test]
fn test_range_iterators() {
    let mut pb = SortedIndexBuffer::default();
    for i in 0..100 {
        pb.insert(i, i * 10);
    }

    // Test keys_range
    let keys: Vec<_> = pb.keys_range(10..20).collect();
    assert_eq!(keys, (10..20).collect::<Vec<_>>());

    // Test reverse
    let keys_rev: Vec<_> = pb.keys_range(10..20).rev().collect();
    assert_eq!(keys_rev, (10..20).rev().collect::<Vec<_>>());

    // Test values_range
    let values: Vec<_> = pb.values_range(10..20).cloned().collect();
    assert_eq!(values, (10..20).map(|i| i * 10).collect::<Vec<_>>());

    // Test iter_range
    let pairs: Vec<_> = pb.iter_range(10..20).map(|(k, v)| (k, *v)).collect();
    assert_eq!(pairs, (10..20).map(|i| (i, i * 10)).collect::<Vec<_>>());
}

#[test]
fn test_retain() {
    let mut pb = SortedIndexBuffer::default();
    for i in 0..100 {
        pb.insert(i, i * 10);
    }

    pb.retain(|i, _v| i % 2 == 0);

    assert_eq!(pb.keys().next(), Some(0));
    assert_eq!(pb.keys().next_back(), Some(98));
    assert_eq!(pb.keys().count(), 50);
    for i in 0..100 {
        if i % 2 == 0 {
            assert_eq!(pb.get(i), Some(&(i * 10)));
        } else {
            assert_eq!(pb.get(i), None);
        }
    }
    pb.check_invariants_expensive();

    pb.retain(|_, _| false);
    assert!(pb.is_empty());
    pb.check_invariants_expensive();
}

#[test]
fn test_retain_range() {
    let mut pb = SortedIndexBuffer::default();
    for i in 0..100 {
        pb.insert(i, i * 10);
    }

    pb.retain_range(20..80);

    assert_eq!(pb.keys().next(), Some(20));
    assert_eq!(pb.keys().next_back(), Some(79));
    assert_eq!(pb.keys().count(), 60);
    for i in 20..80 {
        assert_eq!(pb.get(i), Some(&(i * 10)));
    }
    pb.check_invariants_expensive();

    // Retain with range outside current bounds -> empty
    pb.retain_range(200..300);
    assert!(pb.is_empty());
    pb.check_invariants_expensive();

    // Rebuild and retain with superset range -> no-op
    for i in 10..20 {
        pb.insert(i, i * 10);
    }
    pb.retain_range(0..100);
    assert_eq!(pb.keys().count(), 10);
    pb.check_invariants_expensive();
}

fn assert_same<I1, I2, T>(iter1: I1, iter2: I2)
where
    I1: Iterator<Item = T>,
    I2: Iterator<Item = T>,
    T: PartialEq + std::fmt::Debug,
{
    let vec1: Vec<T> = iter1.collect();
    let vec2: Vec<T> = iter2.collect();
    assert_eq!(vec1, vec2);
}

#[derive(Debug, Clone)]
enum InsertRemoveGetOp {
    Insert(u64, u64),
    Remove(u64),
    Get(u64),
}

fn op_strategy() -> impl Strategy<Value = InsertRemoveGetOp> {
    prop_oneof![
        (0..1000u64, any::<u64>()).prop_map(|(k, v)| InsertRemoveGetOp::Insert(k, v)),
        (0..1000u64).prop_map(InsertRemoveGetOp::Remove),
        (0..1000u64).prop_map(InsertRemoveGetOp::Get),
    ]
}

#[test_strategy::proptest]
fn test_insert_remove_get(
    #[strategy(prop::collection::vec(op_strategy(), 0..1000))] ops: Vec<InsertRemoveGetOp>,
) {
    let mut pb = SortedIndexBuffer::default();
    let mut reference = BTreeMap::new();

    for op in ops {
        match op {
            InsertRemoveGetOp::Insert(k, v) => {
                pb.insert(k, v);
                reference.insert(k, v);
            }
            InsertRemoveGetOp::Remove(k) => {
                let v1 = pb.remove(k);
                let v2 = reference.remove(&k);
                assert_eq!(v1, v2);
            }
            InsertRemoveGetOp::Get(k) => {
                let v1 = pb.get(k);
                let v2 = reference.get(&k);
                assert_eq!(v1, v2);
            }
        }
        pb.check_invariants_expensive();
    }

    // Final state should match
    assert_same(pb.iter(), reference.iter().map(|(k, v)| (*k, v)));
}
