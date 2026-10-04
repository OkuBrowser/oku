use std::{
    cell::{Cell, RefCell},
    hash::{Hash, Hasher},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

use hashlink::linked_hash_map::Entry;
use hashlink::{LinkedHashMap, linked_hash_map};

#[allow(dead_code)]
fn assert_covariance() {
    fn set<'new>(v: LinkedHashMap<&'static str, ()>) -> LinkedHashMap<&'new str, ()> {
        v
    }

    fn iter<'a, 'new>(
        v: linked_hash_map::Iter<'a, &'static str, &'static str>,
    ) -> linked_hash_map::Iter<'a, &'new str, &'new str> {
        v
    }

    fn iter_mut<'a, 'new>(
        v: linked_hash_map::Iter<'a, &'static str, ()>,
    ) -> linked_hash_map::Iter<'a, &'new str, ()> {
        v
    }

    fn into_iter<'new>(
        v: linked_hash_map::IntoIter<&'static str, &'static str>,
    ) -> linked_hash_map::IntoIter<&'new str, &'new str> {
        v
    }

    fn drain<'new>(
        d: linked_hash_map::Drain<'static, &'static str, &'static str>,
    ) -> linked_hash_map::Drain<'new, &'new str, &'new str> {
        d
    }

    fn raw_entry_builder<'a, 'new>(
        v: linked_hash_map::RawEntryBuilder<'a, &'static str, &'static str, ()>,
    ) -> linked_hash_map::RawEntryBuilder<'a, &'new str, &'new str, ()> {
        v
    }
}

#[test]
fn test_index() {
    let mut map = LinkedHashMap::new();
    map.insert(1, 10);
    map.insert(2, 20);
    assert_eq!(10, map[&1]);
    map[&2] = 22;
    assert_eq!(22, map[&2]);
}

#[test]
fn test_insert_and_get() {
    let mut map = LinkedHashMap::new();
    map.insert(1, 10);
    map.insert(2, 20);
    assert_eq!(map.get(&1), Some(&10));
    assert_eq!(map.get(&2), Some(&20));
    assert_eq!(map.len(), 2);
}

#[test]
fn test_insert_update() {
    let mut map = LinkedHashMap::new();
    map.insert("1".to_string(), vec![10, 10]);
    map.insert("1".to_string(), vec![10, 19]);
    assert_eq!(map.get(&"1".to_string()), Some(&vec![10, 19]));
    assert_eq!(map.len(), 1);
}

#[test]
fn test_entry_insert_vacant() {
    let mut map = LinkedHashMap::new();
    match map.entry("1".to_string()) {
        linked_hash_map::Entry::Vacant(e) => {
            assert_eq!(*e.insert(vec![10, 10]), vec![10, 10]);
        }
        _ => panic!("fail"),
    }
    assert!(map.contains_key("1"));
    assert_eq!(map["1"], vec![10, 10]);

    match map.entry("1".to_string()) {
        linked_hash_map::Entry::Occupied(mut e) => {
            assert_eq!(*e.get(), vec![10, 10]);
            assert_eq!(e.insert(vec![10, 16]), vec![10, 10]);
        }
        _ => panic!("fail"),
    }

    assert!(map.contains_key("1"));
    assert_eq!(map["1"], vec![10, 16]);

    match map.entry("1".to_string()) {
        linked_hash_map::Entry::Occupied(e) => {
            assert_eq!(e.remove(), vec![10, 16]);
        }
        _ => panic!("fail"),
    }
}

#[test]
fn test_remove() {
    let mut map = LinkedHashMap::new();
    map.insert(1, 10);
    map.insert(2, 20);
    map.insert(3, 30);
    map.insert(4, 40);
    map.insert(5, 50);
    map.remove(&3);
    map.remove(&4);
    assert!(map.get(&3).is_none());
    assert!(map.get(&4).is_none());
    map.insert(6, 60);
    map.insert(7, 70);
    map.insert(8, 80);
    assert_eq!(map.get(&6), Some(&60));
    assert_eq!(map.get(&7), Some(&70));
    assert_eq!(map.get(&8), Some(&80));
}

#[test]
fn test_pop() {
    let mut map = LinkedHashMap::new();
    map.insert(1, 10);
    map.insert(2, 20);
    map.insert(3, 30);
    map.insert(4, 40);
    map.insert(5, 50);
    assert_eq!(map.pop_front(), Some((1, 10)));
    assert!(map.get(&1).is_none());
    assert_eq!(map.pop_back(), Some((5, 50)));
    assert!(map.get(&5).is_none());
    map.insert(6, 60);
    map.insert(7, 70);
    map.insert(8, 80);
    assert_eq!(map.pop_front(), Some((2, 20)));
    assert!(map.get(&2).is_none());
    assert_eq!(map.pop_back(), Some((8, 80)));
    assert!(map.get(&8).is_none());
    map.insert(3, 30);
    assert_eq!(map.pop_front(), Some((4, 40)));
    assert!(map.get(&4).is_none());
    assert_eq!(map.pop_back(), Some((3, 30)));
    assert!(map.get(&3).is_none());
}

#[test]
fn test_move() {
    let to_back = |map: &mut LinkedHashMap<_, _>, key| match map.entry(key) {
        linked_hash_map::Entry::Occupied(mut occupied) => occupied.to_back(),
        linked_hash_map::Entry::Vacant(_) => panic!(),
    };

    let to_front = |map: &mut LinkedHashMap<_, _>, key| match map.entry(key) {
        linked_hash_map::Entry::Occupied(mut occupied) => occupied.to_front(),
        linked_hash_map::Entry::Vacant(_) => panic!(),
    };

    let mut map = LinkedHashMap::new();
    map.insert(1, 10);
    map.insert(2, 20);
    map.insert(3, 30);
    map.insert(4, 40);
    map.insert(5, 50);

    to_back(&mut map, 1);
    assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![2, 3, 4, 5, 1]);

    to_front(&mut map, 4);
    assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![4, 2, 3, 5, 1]);

    to_back(&mut map, 3);
    assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![4, 2, 5, 1, 3]);

    to_front(&mut map, 2);
    assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![2, 4, 5, 1, 3]);

    to_back(&mut map, 3);
    assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![2, 4, 5, 1, 3]);

    to_front(&mut map, 2);
    assert_eq!(map.keys().copied().collect::<Vec<_>>(), vec![2, 4, 5, 1, 3]);
}

#[test]
fn test_clear() {
    let mut map = LinkedHashMap::new();
    map.insert(1, 10);
    map.insert(2, 20);
    map.clear();
    assert!(map.get(&1).is_none());
    assert!(map.get(&2).is_none());
    assert!(map.is_empty());
}

#[test]
fn test_iter() {
    let mut map = LinkedHashMap::new();

    // empty iter
    assert_eq!(None, map.iter().next());

    map.insert("a", 10);
    map.insert("b", 20);
    map.insert("c", 30);

    // regular iter
    let mut iter = map.iter();
    assert_eq!((&"a", &10), iter.next().unwrap());
    assert_eq!((&"b", &20), iter.next().unwrap());
    assert_eq!((&"c", &30), iter.next().unwrap());
    assert_eq!(None, iter.next());
    assert_eq!(None, iter.next());

    let mut iter = map.iter();
    assert_eq!((&"a", &10), iter.next().unwrap());
    let mut iclone = iter.clone();
    assert_eq!((&"b", &20), iter.next().unwrap());
    assert_eq!((&"b", &20), iclone.next().unwrap());
    assert_eq!((&"c", &30), iter.next().unwrap());
    assert_eq!((&"c", &30), iclone.next().unwrap());

    // reversed iter
    let mut rev_iter = map.iter().rev();
    assert_eq!((&"c", &30), rev_iter.next().unwrap());
    assert_eq!((&"b", &20), rev_iter.next().unwrap());
    assert_eq!((&"a", &10), rev_iter.next().unwrap());
    assert_eq!(None, rev_iter.next());
    assert_eq!(None, rev_iter.next());

    // mixed
    let mut mixed_iter = map.iter();
    assert_eq!((&"a", &10), mixed_iter.next().unwrap());
    assert_eq!((&"c", &30), mixed_iter.next_back().unwrap());
    assert_eq!((&"b", &20), mixed_iter.next().unwrap());
    assert_eq!(None, mixed_iter.next());
    assert_eq!(None, mixed_iter.next_back());
}

#[test]
fn test_borrow() {
    #[derive(PartialEq, Eq, Hash)]
    struct Foo(Bar);
    #[derive(PartialEq, Eq, Hash)]
    struct Bar(i32);

    impl ::std::borrow::Borrow<Bar> for Foo {
        fn borrow(&self) -> &Bar {
            &self.0
        }
    }

    let mut map = LinkedHashMap::new();
    map.insert(Foo(Bar(1)), "a");
    map.insert(Foo(Bar(2)), "b");

    assert!(map.contains_key(&Bar(1)));
    assert!(map.contains_key(&Bar(2)));
    assert!(map.contains_key(&Foo(Bar(1))));
    assert!(map.contains_key(&Foo(Bar(2))));

    assert_eq!(map.get(&Bar(1)), Some(&"a"));
    assert_eq!(map.get(&Bar(2)), Some(&"b"));
    assert_eq!(map.get(&Foo(Bar(1))), Some(&"a"));
    assert_eq!(map.get(&Foo(Bar(2))), Some(&"b"));

    assert_eq!(map.get_mut(&Bar(1)), Some(&mut "a"));
    assert_eq!(map.get_mut(&Bar(2)), Some(&mut "b"));
    assert_eq!(map.get_mut(&Foo(Bar(1))), Some(&mut "a"));
    assert_eq!(map.get_mut(&Foo(Bar(2))), Some(&mut "b"));

    assert_eq!(map[&Bar(1)], "a");
    assert_eq!(map[&Bar(2)], "b");
    assert_eq!(map[&Foo(Bar(1))], "a");
    assert_eq!(map[&Foo(Bar(2))], "b");

    assert_eq!(map.remove(&Bar(1)), Some("a"));
    assert_eq!(map.remove(&Bar(2)), Some("b"));
    assert_eq!(map.remove(&Foo(Bar(1))), None);
    assert_eq!(map.remove(&Foo(Bar(2))), None);
}

#[test]
fn test_iter_mut() {
    let mut map = LinkedHashMap::new();
    map.insert("a", 10);
    map.insert("c", 30);
    map.insert("b", 20);

    {
        let mut iter = map.iter_mut();
        let entry = iter.next().unwrap();
        assert_eq!("a", *entry.0);
        *entry.1 = 17;

        assert_eq!(format!("{:?}", iter), "[(\"c\", 30), (\"b\", 20)]");

        // reverse iterator
        let mut iter = iter.rev();
        let entry = iter.next().unwrap();
        assert_eq!("b", *entry.0);
        *entry.1 = 23;

        let entry = iter.next().unwrap();
        assert_eq!("c", *entry.0);
        assert_eq!(None, iter.next());
        assert_eq!(None, iter.next());
    }

    assert_eq!(17, map[&"a"]);
    assert_eq!(23, map[&"b"]);
}

#[test]
fn test_consuming_iter() {
    let map = {
        let mut map = LinkedHashMap::new();
        map.insert("a", 10);
        map.insert("c", 30);
        map.insert("b", 20);
        map
    };

    let mut iter = map.into_iter();
    assert_eq!(Some(("a", 10)), iter.next());
    assert_eq!(Some(("b", 20)), iter.next_back());
    assert_eq!(iter.len(), 1);
    assert_eq!(format!("{:?}", iter), "[(\"c\", 30)]");
    assert_eq!(Some(("c", 30)), iter.next());
    assert_eq!(None, iter.next());
}

#[test]
fn test_consuming_iter_empty() {
    let map = LinkedHashMap::<&str, i32>::new();
    let mut iter = map.into_iter();
    assert_eq!(None, iter.next());
}

#[test]
fn test_consuming_iter_with_free_list() {
    let mut map = LinkedHashMap::new();
    map.insert("a", 10);
    map.insert("c", 30);
    map.insert("b", 20);
    map.remove("a");
    map.remove("b");

    let mut iter = map.into_iter();
    assert_eq!(Some(("c", 30)), iter.next());
    assert_eq!(None, iter.next());
}

#[test]
fn test_into_iter_drop() {
    struct Counter<'a>(&'a mut usize);

    impl Drop for Counter<'_> {
        fn drop(&mut self) {
            *self.0 += 1;
        }
    }

    let mut a = 0;
    let mut b = 0;
    let mut c = 0;

    {
        let mut map = LinkedHashMap::new();
        map.insert("a", Counter(&mut a));
        map.insert("b", Counter(&mut b));
        map.insert("c", Counter(&mut c));

        let mut iter = map.into_iter();
        assert_eq!(iter.next().map(|p| p.0), Some("a"));
        assert_eq!(iter.next_back().map(|p| p.0), Some("c"));
    }

    assert_eq!(a, 1);
    assert_eq!(b, 1);
    assert_eq!(c, 1);
}

#[test]
fn test_drain() {
    use std::{cell::Cell, rc::Rc};

    struct Counter(Rc<Cell<u32>>);

    impl Drop for Counter {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    let mut map = LinkedHashMap::new();

    let a = Rc::new(Cell::new(0));
    let b = Rc::new(Cell::new(0));
    let c = Rc::new(Cell::new(0));

    map.insert("a", Counter(a.clone()));
    map.insert("b", Counter(b.clone()));
    map.insert("c", Counter(c.clone()));

    let mut iter = map.drain();
    assert_eq!(iter.next().map(|p| p.0), Some("a"));
    assert_eq!(iter.next_back().map(|p| p.0), Some("c"));
    assert_eq!(iter.next_back().map(|p| p.0), Some("b"));
    assert!(iter.next().is_none());
    assert!(iter.next_back().is_none());

    drop(iter);
    assert_eq!(map.len(), 0);

    assert_eq!(a.get(), 1);
    assert_eq!(b.get(), 1);
    assert_eq!(c.get(), 1);

    map.insert("a", Counter(a.clone()));
    map.insert("b", Counter(b.clone()));
    map.insert("c", Counter(c.clone()));

    let mut iter = map.drain();
    assert_eq!(iter.next().map(|p| p.0), Some("a"));
    assert_eq!(iter.next().map(|p| p.0), Some("b"));
    assert_eq!(iter.next_back().map(|p| p.0), Some("c"));
    assert!(iter.next().is_none());
    assert!(iter.next_back().is_none());

    drop(iter);
    assert_eq!(map.len(), 0);

    assert_eq!(a.get(), 2);
    assert_eq!(b.get(), 2);
    assert_eq!(c.get(), 2);

    map.insert("a", Counter(a.clone()));
    map.insert("b", Counter(b.clone()));
    map.insert("c", Counter(c.clone()));

    map.drain();
    assert_eq!(map.len(), 0);

    assert_eq!(a.get(), 3);
    assert_eq!(b.get(), 3);
    assert_eq!(c.get(), 3);
}

#[test]
fn test_send_sync() {
    fn is_send_sync<T: Send + Sync>() {}

    is_send_sync::<LinkedHashMap<u32, i32>>();
    is_send_sync::<linked_hash_map::Entry<u32, i32, ()>>();
    is_send_sync::<linked_hash_map::RawEntryBuilder<u32, i32, ()>>();
    is_send_sync::<linked_hash_map::RawEntryBuilderMut<u32, i32, ()>>();
    is_send_sync::<linked_hash_map::RawEntryMut<u32, i32, ()>>();
    is_send_sync::<linked_hash_map::Iter<u32, i32>>();
    is_send_sync::<linked_hash_map::IterMut<u32, i32>>();
    is_send_sync::<linked_hash_map::Drain<u32, i32>>();
    is_send_sync::<linked_hash_map::Keys<u32, i32>>();
    is_send_sync::<linked_hash_map::Values<u32, i32>>();
}

#[test]
fn test_retain() {
    use std::{cell::Cell, rc::Rc};

    let xs = [1, 2, 3, 4, 5, 6];
    let mut map: LinkedHashMap<String, i32> = xs.iter().map(|i| (i.to_string(), *i)).collect();
    map.retain(|_, v| *v % 2 == 0);
    assert_eq!(map.len(), 3);
    assert!(map.contains_key("2"));
    assert!(map.contains_key("4"));
    assert!(map.contains_key("6"));

    struct Counter(Rc<Cell<u32>>);

    impl Drop for Counter {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    let c = Rc::new(Cell::new(0));

    let mut map = LinkedHashMap::new();
    map.insert(1, Counter(Rc::clone(&c)));
    map.insert(2, Counter(Rc::clone(&c)));
    map.insert(3, Counter(Rc::clone(&c)));
    map.insert(4, Counter(Rc::clone(&c)));

    map.retain(|k, _| *k % 2 == 0);

    assert!(c.get() == 2);
    drop(map);
    assert!(c.get() == 4);
}

#[test]
fn test_order_equality() {
    let xs = [1, 2, 3, 4, 5, 6];
    let mut map1: LinkedHashMap<String, i32> = xs.iter().map(|i| (i.to_string(), *i)).collect();
    let mut map2: LinkedHashMap<String, i32> = xs.iter().map(|i| (i.to_string(), *i)).collect();

    assert_eq!(map1, map2);

    map1.to_front("4");
    assert_ne!(map1, map2);

    map2.to_front("4");
    assert_eq!(map1, map2);
}

#[test]
fn test_replace() {
    let mut map = LinkedHashMap::new();

    map.insert(1, 1);
    map.insert(2, 2);
    map.insert(3, 3);
    map.insert(4, 4);

    assert!(
        map.iter()
            .map(|(k, v)| (*k, *v))
            .eq([(1, 1), (2, 2), (3, 3), (4, 4)].iter().copied())
    );

    map.insert(3, 5);

    assert!(
        map.iter()
            .map(|(k, v)| (*k, *v))
            .eq([(1, 1), (2, 2), (4, 4), (3, 5)].iter().copied())
    );

    map.replace(2, 6);

    assert!(
        map.iter()
            .map(|(k, v)| (*k, *v))
            .eq([(1, 1), (2, 6), (4, 4), (3, 5)].iter().copied())
    );
}

#[test]
fn test_reserve() {
    let mut map = LinkedHashMap::new();

    map.insert(1, 1);
    map.insert(2, 2);
    map.insert(3, 3);
    map.insert(4, 4);

    assert!(map.capacity() - map.len() < 100);
    map.reserve(100);
    assert!(map.capacity() - map.len() >= 100);
}

#[test]
fn test_shrink_to_fit_resize() {
    let mut map = LinkedHashMap::new();
    map.shrink_to_fit();

    for i in 0..100 {
        map.insert(i, i);
    }
    map.shrink_to_fit();

    for _ in 0..50 {
        map.pop_front();
        map.shrink_to_fit();
    }

    assert_eq!(map.len(), 50);
    for i in 50..100 {
        assert_eq!(map.get(&i).unwrap(), &i);
    }
}

#[test]
fn test_cursor_mut_current() {
    let mut map = LinkedHashMap::new();

    map.insert(3, 3);

    if let linked_hash_map::Entry::Occupied(entry) = map.entry(3) {
        let mut cursor = entry.cursor_mut();
        let value = cursor.current().unwrap();
        assert_eq!(value, (&3, &mut 3));
        *value.1 = 5;
        let value = cursor.current().unwrap();
        assert_eq!(value, (&3, &mut 5));
    }
}

#[test]
fn test_cursor_mut_move_next() {
    let mut map = LinkedHashMap::new();

    map.insert(3, 3);
    map.insert(4, 4);
    map.insert(5, 5);
    map.insert(6, 6);

    if let linked_hash_map::Entry::Occupied(entry) = map.entry(3) {
        let mut cursor = entry.cursor_mut();
        let value = cursor.current();
        assert!(&value.is_some());
        assert_eq!(value.unwrap().1, &mut 3);
        cursor.move_next();
        let value = cursor.current();
        assert!(&value.is_some());
        assert_eq!(value.unwrap().1, &mut 4);
        cursor.move_next();
        let value = cursor.current();
        assert!(&value.is_some());
        assert_eq!(value.unwrap().1, &mut 5);
        cursor.move_next();
        let value = cursor.current();
        assert!(&value.is_some());
        assert_eq!(value.unwrap().1, &mut 6);
        cursor.move_next();
        let value = cursor.current();
        assert!(value.is_none());
        cursor.move_next();
        let value = cursor.current();
        assert!(value.is_some());
        assert_eq!(value.unwrap().1, &mut 3);
        cursor.move_next();
        let value = cursor.current();
        assert!(&value.is_some());
        assert_eq!(value.unwrap().1, &mut 4);
    }
}

#[test]
fn test_cursor_mut_move_prev() {
    let mut map = LinkedHashMap::new();

    map.insert(3, 3);

    if let linked_hash_map::Entry::Occupied(entry) = map.entry(3) {
        let mut cursor = entry.cursor_mut();
        cursor.move_prev();
        let value = cursor.current();
        assert!(value.is_none());
        cursor.move_prev();
        let value = cursor.current();
        assert!(&value.is_some());
        assert_eq!(value.unwrap().1, &mut 3);
    }
}

#[test]
fn test_cursor_mut_pick_next() {
    let mut map = LinkedHashMap::new();

    map.insert(3, 3);
    map.insert(4, 4);

    if let linked_hash_map::Entry::Occupied(entry) = map.entry(3) {
        let mut cursor = entry.cursor_mut();
        let next = cursor.peek_next();
        assert!(&next.is_some());
        assert_eq!(next.unwrap().1, &mut 4);
        cursor.move_next();
        let next = cursor.peek_next();
        assert!(&next.is_none());
        cursor.move_next();
        let next = cursor.peek_next();
        assert!(&next.is_some());
        let value = next.as_ref().unwrap().to_owned();
        assert_eq!(*value.1, 3);
        *next.unwrap().1 = 5;
        let next = cursor.peek_next();
        assert!(&next.is_some());
        assert_eq!(next.unwrap().1, &mut 5);
    }
}

#[test]
fn test_cursor_mut_pick_prev() {
    let mut map = LinkedHashMap::new();

    map.insert(3, 3);
    map.insert(4, 4);

    if let linked_hash_map::Entry::Occupied(entry) = map.entry(3) {
        let mut cursor = entry.cursor_mut();
        let next = cursor.peek_prev();
        assert!(&next.is_none());
        cursor.move_prev();
        let next = cursor.peek_prev();
        assert!(&next.is_some());
        assert_eq!(next.unwrap(), (&4, &mut 4));
    }
}

#[test]
fn test_cursor_mut_insert_before() {
    let mut map = LinkedHashMap::new();

    map.insert(3, 3);
    map.insert(4, 4);

    // Insert new element in the middle
    if let linked_hash_map::Entry::Occupied(entry) = map.entry(4) {
        entry.cursor_mut().insert_before(5, 5);
        assert!(
            map.iter()
                .map(|(k, v)| (*k, *v))
                .eq([(3, 3), (5, 5), (4, 4)].iter().copied())
        );
    }

    // Insert new element at the very end of the list
    if let linked_hash_map::Entry::Occupied(entry) = map.entry(3) {
        let mut cursor = entry.cursor_mut();
        cursor.move_prev();
        cursor.insert_before(6, 6);
        assert!(
            map.iter()
                .map(|(k, v)| (*k, *v))
                .eq([(3, 3), (5, 5), (4, 4), (6, 6)].iter().copied())
        );
    }

    // Relocate element and override value
    if let linked_hash_map::Entry::Occupied(entry) = map.entry(5) {
        entry.cursor_mut().insert_before(4, 42);
        assert!(
            map.iter()
                .map(|(k, v)| (*k, *v))
                .eq([(3, 3), (4, 42), (5, 5), (6, 6)].iter().copied())
        );
    }
}

#[test]
fn test_cursor_mut_insert_after() {
    let mut map = LinkedHashMap::new();

    map.insert(3, 3);
    map.insert(4, 4);

    // Insert new element in the middle.
    if let linked_hash_map::Entry::Occupied(entry) = map.entry(3) {
        entry.cursor_mut().insert_after(5, 5);
        assert!(
            map.iter()
                .map(|(k, v)| (*k, *v))
                .eq([(3, 3), (5, 5), (4, 4)].iter().copied())
        );
    }

    // Insert new element as the first one.
    if let linked_hash_map::Entry::Occupied(entry) = map.entry(4) {
        let mut cursor = entry.cursor_mut();
        cursor.move_next();
        cursor.insert_after(6, 6);
        assert!(
            map.iter()
                .map(|(k, v)| (*k, *v))
                .eq([(6, 6), (3, 3), (5, 5), (4, 4)].iter().copied())
        );
    }
}

#[test]
fn test_cursor_mut_insert_before_itself() {
    let mut map = LinkedHashMap::new();

    map.insert(2, 2);
    map.insert(3, 3);
    map.insert(4, 4);

    // Insert a new value before its key. This is a corner case that needs to be
    // handled explicitly.
    if let linked_hash_map::Entry::Occupied(entry) = map.entry(3) {
        entry.cursor_mut().insert_before(3, 5);
        assert!(
            map.iter()
                .map(|(k, v)| (*k, *v))
                .eq([(2, 2), (3, 5), (4, 4)].iter().copied())
        );
    }
}

#[test]
fn test_cursor_front_mut() {
    let mut map: LinkedHashMap<i32, i32> = LinkedHashMap::new();
    // The `CursorMut`` in an empty LinkedHashMap will always return `None` as its
    // current element, regardless of any move in any direction.
    let mut cursor = map.cursor_front_mut();
    assert!(cursor.current().is_none());
    cursor.move_next();
    assert!(cursor.current().is_none());
    cursor.insert_after(1, 1);
    cursor.move_next();
    assert!(cursor.current().is_some());
    assert_eq!(cursor.current().unwrap().1, &mut 1);
    cursor.move_next();
    assert!(cursor.current().is_none());

    assert!(
        map.iter()
            .map(|(k, v)| (*k, *v))
            .eq([(1, 1)].iter().copied())
    );

    map.insert(2, 2);
    map.insert(3, 3);

    let mut cursor = map.cursor_front_mut();
    assert!(cursor.current().is_some());
    assert_eq!(cursor.current().unwrap().1, &mut 1);
}

#[test]
fn test_cursor_back_mut() {
    let mut map: LinkedHashMap<i32, i32> = LinkedHashMap::new();

    map.insert(1, 1);
    map.insert(2, 2);
    map.insert(3, 3);

    let mut cursor = map.cursor_back_mut();
    assert!(cursor.current().is_some());
    assert_eq!(cursor.current().unwrap().1, &mut 3);
}

#[test]
fn test_vacant_entry_insert_entry() {
    let mut map: LinkedHashMap<i32, i32> = LinkedHashMap::new();

    map.insert(1, 1);
    map.insert(2, 2);
    map.insert(3, 3);

    let Entry::Vacant(vacant) = map.entry(4) else {
        panic!("no entry with 4 in map");
    };

    let occu = vacant.insert_entry(55);
    let (k, v) = occu.remove_entry();
    assert_eq!(k, 4);
    assert_eq!(v, 55);

    assert!(map.get(&4).is_none());

    let Entry::Vacant(vacant) = map.entry(4) else {
        panic!("no entry with 4 in map");
    };

    let occu = vacant.insert_entry(56);
    assert_eq!(occu.get(), &56);
    assert_eq!(occu.key(), &4);

    assert_eq!(map.get(&4), Some(&56));
    assert_eq!(map.back(), Some((&4, &56)));
}

#[test]
fn test_front_back_entry() {
    let mut map: LinkedHashMap<i32, i32> = LinkedHashMap::new();
    assert!(map.front_entry().is_none());
    assert!(map.back_entry().is_none());

    map.insert(1, 5);
    map.insert(2, 6);
    map.insert(3, 7);

    let Some(fe) = map.front_entry() else {
        panic!("no front entry");
    };

    assert_eq!(fe.key(), &1);
    assert_eq!(fe.get(), &5);

    let Some(fe) = map.back_entry() else {
        panic!("no back entry");
    };

    assert_eq!(fe.key(), &3);
    assert_eq!(fe.get(), &7);

    let Some(fe) = map.front_entry() else {
        panic!("no front entry");
    };

    assert_eq!(fe.remove_entry(), (1, 5));
    assert_eq!(map.front(), Some((&2, &6)));
    assert_eq!(map.back(), Some((&3, &7)));

    let Some(fe) = map.back_entry() else {
        panic!("no back entry");
    };

    assert_eq!(fe.remove_entry(), (3, 7));
    assert_eq!(map.front(), Some((&2, &6)));
    assert_eq!(map.back(), Some((&2, &6)));
    assert_eq!(map.len(), 1);

    let Some(fe) = map.back_entry() else {
        panic!("no back entry");
    };
    assert_eq!(fe.remove_entry(), (2, 6));
    assert!(map.is_empty());
}

#[test]
fn test_cursor_to_entry() {
    let mut map: LinkedHashMap<i32, i32> = LinkedHashMap::new();
    assert!(map.cursor_back_mut().current_entry().is_err());
    map.insert(1, 5);
    map.insert(2, 6);
    map.insert(3, 7);

    let e = map
        .cursor_back_mut()
        .current_entry()
        .map_err(|_| ())
        .expect("map empty");
    assert_eq!(e.key(), &3);
    assert_eq!(e.get(), &7);

    let mut cursor = map.cursor_back_mut();
    cursor.move_prev();
    let e = cursor.current_entry().map_err(|_| ()).expect("map empty");
    assert_eq!(e.key(), &2);
    assert_eq!(e.get(), &6);

    let e = map
        .cursor_front_mut()
        .current_entry()
        .map_err(|_| ())
        .expect("map empty");
    assert_eq!(e.key(), &1);
    assert_eq!(e.get(), &5);

    let mut cursor = map.cursor_back_mut();
    cursor.move_prev();
    let e = cursor.current_entry().map_err(|_| ()).expect("map empty");
    assert_eq!(e.remove_entry(), (2, 6));

    assert_eq!(map.len(), 2);

    let mut cursor = map.cursor_back_mut();
    cursor.move_prev();
    let e = cursor.current_entry().map_err(|_| ()).expect("map empty");
    assert_eq!(e.key(), &1);
    assert_eq!(e.get(), &5);

    let mut cursor = map.cursor_back_mut();
    cursor.move_prev();
    cursor.move_prev();
    assert!(cursor.current_entry().is_err());

    let mut cursor = map.cursor_front_mut();
    cursor.move_next();
    cursor.move_next();
    assert!(cursor.current_entry().is_err());

    let collected = map.into_iter().collect::<Vec<_>>();
    assert_eq!(collected, vec![(1, 5), (3, 7)]);
}

// Regression test for https://github.com/djc/hashlink/issues/43
//
// A panic while dropping an entry during `clear` must not leave a moved-out node reachable
// from the value list, which would cause its entry to be dropped again.
#[test]
fn test_clear_panic_safe() {
    struct PanicOnDrop {
        should_panic: Rc<Cell<bool>>,
    }

    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            if self.should_panic.replace(false) {
                panic!("intentional panic while clearing LinkedHashMap");
            }
        }
    }

    let should_panic = Rc::new(Cell::new(true));
    let mut map = LinkedHashMap::new();
    map.insert(
        1,
        PanicOnDrop {
            should_panic: Rc::clone(&should_panic),
        },
    );
    map.insert(
        2,
        PanicOnDrop {
            should_panic: Rc::clone(&should_panic),
        },
    );

    let result = catch_unwind(AssertUnwindSafe(|| map.clear()));
    assert!(result.is_err());
    should_panic.set(false);

    // Dropping (or reusing) the map after the caught panic must not traverse a
    // stale, moved-out node.
    drop(map);
}

// Regression test for https://github.com/djc/hashlink/issues/42
//
// A key that changes its hash/equality through interior mutability inside the
// `retain_with_order` callback must not make the method remove a different table entry than the
// node it frees, which would leave the table referencing a freed node.
#[test]
fn test_retain_with_order_key_mutation_sound() {
    #[derive(Debug)]
    struct Key(RefCell<String>);

    impl Key {
        fn new(value: &str) -> Self {
            Self(RefCell::new(value.to_owned()))
        }

        fn set(&self, value: &str) {
            *self.0.borrow_mut() = value.to_owned();
        }
    }

    impl PartialEq for Key {
        fn eq(&self, other: &Self) -> bool {
            self.0.borrow().as_str() == other.0.borrow().as_str()
        }
    }

    impl Eq for Key {}

    impl Hash for Key {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.0.borrow().hash(state);
        }
    }

    let mut map = LinkedHashMap::new();
    map.insert(Key::new("a"), 1);
    map.insert(Key::new("b"), 2);

    // The callback mutates the first key to compare equal to the second, then
    // asks to drop it.
    let mut first = true;
    map.retain_with_order(|key, _| {
        if first {
            first = false;
            key.set("b");
            false
        } else {
            true
        }
    });

    // Looking up a stale key must not dereference a freed node.
    let _ = map.contains_key(&Key::new("a"));
}

#[test]
pub fn test_insert_front() {
    let mut under_test = LinkedHashMap::new();
    under_test.insert_front(1, 1);
    assert!(under_test.contains_key(&1));
    assert_eq!(under_test.front(), Some((&1, &1)));
    assert_eq!(under_test.back(), Some((&1, &1)));

    under_test.insert(2, 2);
    assert!(under_test.contains_key(&2));
    assert_eq!(under_test.front(), Some((&1, &1)));
    assert_eq!(under_test.back(), Some((&2, &2)));
    under_test.insert_front(3, 3);
    assert!(under_test.contains_key(&3));
    assert_eq!(under_test.front(), Some((&3, &3)));
    assert_eq!(under_test.back(), Some((&2, &2)));

    let elems = under_test.keys().collect::<Vec<_>>();
    assert_eq!(elems, vec![&3, &1, &2]);

    assert_eq!(under_test.insert_front(2, 4), Some(2));
    assert_eq!(under_test.front(), Some((&2, &4)));
    assert_eq!(under_test.back(), Some((&1, &1)));

    let elems = under_test.keys().collect::<Vec<_>>();
    assert_eq!(elems, vec![&2, &3, &1]);
}
