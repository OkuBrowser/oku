//! # Census
//!
//! Census' `Inventory`  makes it possible to track a set of living items of a specific type.
//! Items are automatically removed from the `Inventory<T>` as the living item's are dropped.
//!
//! ```rust
//! use census::{Inventory, TrackedObject};
//!
//! let inventory = Inventory::new();
//!
//! //  Each object tracked needs to be registered explicitely in the Inventory.
//! //  A `TrackedObject<T>` wrapper is then returned.
//! let one = inventory.track("one".to_string());
//! let two = inventory.track("two".to_string());
//!
//! // A snapshot  of the list of living instances can be obtained...
//! // (no guarantee on their order)
//! let living_instances: Vec<TrackedObject<String>> = inventory.list();
//! assert_eq!(living_instances.len(), 2);
//! ```

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};

use std::fmt::{Error, Formatter};

struct Items<T> {
    alive_count: usize,
    items: Vec<Weak<InnerTrackedObject<T>>>,
}

impl<T> Default for Items<T> {
    fn default() -> Self {
        Items {
            alive_count: 0,
            items: Vec::new(),
        }
    }
}

impl<T> Items<T> {
    fn record_birth(&mut self) {
        self.alive_count += 1;
    }

    fn record_death(&mut self) {
        self.alive_count -= 1;
    }

    fn len(&mut self) -> usize {
        self.alive_count()
    }

    fn list_arc(&mut self) -> Vec<TrackedObject<T>> {
        self.items
            .iter()
            .flat_map(|weak| weak.upgrade())
            .map(|v| TrackedObject { inner: v })
            .collect()
    }

    fn gc_if_needed(&mut self) {
        if !self.should_gc() {
            return;
        }
        let mut i = 0;
        while i < self.items.len() {
            let should_remove = self.items[i].strong_count() == 0;
            if should_remove {
                self.items.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn alive_count(&self) -> usize {
        self.alive_count
    }

    fn should_gc(&self) -> bool {
        self.alive_count * 2 <= self.items.len()
    }
}

struct InnerInventory<T> {
    items: Mutex<Items<T>>,
    condvar: Condvar,
}

/// The `Inventory` register and keeps track of all of the objects alive.
pub struct Inventory<T> {
    inner: Arc<InnerInventory<T>>,
}

impl<T> Default for Inventory<T> {
    fn default() -> Self {
        Inventory {
            inner: Arc::new(InnerInventory {
                items: Mutex::new(Items::default()),
                condvar: Condvar::new(),
            }),
        }
    }
}

impl<T> Clone for Inventory<T> {
    fn clone(&self) -> Self {
        Inventory {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Inventory<T> {
    /// Creates a new inventory.
    pub fn new() -> Inventory<T> {
        Inventory::default()
    }

    fn lock_items(&self) -> MutexGuard<Items<T>> {
        let mut guard = self.inner.items.lock().unwrap();
        guard.gc_if_needed();
        guard
    }

    /// Returns the number of tracked object.
    pub fn len(&self) -> usize {
        self.lock_items().len()
    }

    /// Takes a snapshot of the list of tracked object.
    ///
    /// Note that the list is a simple `Vec` of tracked object.
    /// As a result, it is a consistent snapshot of the
    /// list of living instance at the time of the call,
    ///
    /// Obviously, instances may have been created after the call.
    /// They will obviously not appear in the snapshot.
    ///
    /// ```rust
    /// use census::{Inventory, TrackedObject};
    ///
    /// let inventory = Inventory::new();
    ///
    /// let one = inventory.track("one".to_string());
    /// let living_instances: Vec<TrackedObject<String>> = inventory.list();
    /// let two = inventory.track("two".to_string());
    ///
    /// // our snapshot is a bit old.
    /// assert_eq!(living_instances.len(), 1);
    ///
    /// // a fresher snapshot would contain our new element.
    /// assert_eq!(inventory.list().len(), 2);
    /// ```
    ///
    /// Also, the instance in the snapshot itself
    /// are considered "living".
    ///
    /// As a result, as long as a snapshot is not dropped,
    /// all of its instances will be part of the inventory.
    ///
    /// ```rust
    /// # use census::{Inventory, TrackedObject};
    ///
    /// let inventory = Inventory::new();
    ///
    /// let one = inventory.track("one".to_string());
    /// let living_instances: Vec<TrackedObject<String>> = inventory.list();
    ///
    /// // let's drop one here
    /// drop(one);
    ///
    /// // The instance is technically still in the inventory
    /// // as our previous snapshot is extending its life...
    /// assert_eq!(inventory.list().len(), 1);
    ///
    /// // If we drop our previous snapshot however...
    /// drop(living_instances);
    ///
    /// // `one` is really untracked.
    /// assert!(inventory.list().is_empty());
    /// ```
    ///
    pub fn list(&self) -> Vec<TrackedObject<T>> {
        self.lock_items().list_arc()
    }

    /// This function blocks until there are no more items in the inventory.
    ///
    /// It is a helper calling
    /// ```ignore
    /// self.wait_until_predicate(|count| count == 0)
    /// ```
    ///
    /// Note it is very easy to misuse this function and create a deadlock.
    /// For instance, if any living TrackedObject is on the stack at the moment of the call,
    /// it will not get dropped, and the inventory cannot become empty.
    pub fn wait_until_empty(&self) {
        self.wait_until_predicate(|count| count == 0)
    }

    /// This function blocks until the number of items in the repository matches a specific
    /// predicate.
    ///
    /// See also `wait_until_empty`.
    ///
    /// Note it is very easy to misuse this function and create a deadlock.
    /// For instance, if any living TrackedObject is on the stack at the moment of the call,
    /// it will not get dropped, and the inventory cannot become empty.
    pub fn wait_until_predicate<F: Fn(usize) -> bool>(&self, predicate_on_count: F) {
        let mut count = self.lock_items();
        while !predicate_on_count(count.alive_count()) {
            count = self.inner.condvar.wait(count).unwrap();
        }
    }

    /// Starts tracking a given `T` object.
    pub fn track(&self, item: T) -> TrackedObject<T> {
        let item_arc = Arc::new(InnerTrackedObject {
            census: self.clone(),
            item,
        });
        let item_weak = Arc::downgrade(&item_arc);
        let mut items_lock = self.lock_items();
        items_lock.items.push(item_weak);
        items_lock.record_birth();
        self.inner.condvar.notify_all();
        TrackedObject { inner: item_arc }
    }
}

/// Your tracked object.
///
/// A tracked object contains reference counting logic and an
/// `Arc` to your object. It is cloneable but calling clone will
/// not clone your internal object.
///
/// Your object cannot be mutated. You can borrow it using
/// the `Deref` interface.
#[derive(Clone)]
pub struct TrackedObject<T> {
    inner: Arc<InnerTrackedObject<T>>,
}

struct InnerTrackedObject<T> {
    census: Inventory<T>,
    item: T,
}

impl<T: fmt::Debug> fmt::Debug for TrackedObject<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Tracked({:?})", self.inner.item)
    }
}

impl<T> TrackedObject<T> {
    /// Creates a new object from an existing one.
    ///
    /// The new object will be registered
    /// in your original object's inventory.
    ///
    /// ```rust
    /// use census::{Inventory, TrackedObject};
    ///
    /// let inventory = Inventory::new();
    ///
    /// let seven = inventory.track(7);
    /// let fourteen = seven.map(|i| i * 2);
    /// assert_eq!(*fourteen, 14);
    ///
    /// let living_instances = inventory.list();
    /// assert_eq!(living_instances.len(), 2);
    /// ```
    pub fn map<F>(&self, f: F) -> TrackedObject<T>
    where
        F: FnOnce(&T) -> T,
    {
        let t = f(self);
        self.inner.census.track(t)
    }
}

impl<T> Drop for InnerTrackedObject<T> {
    fn drop(&mut self) {
        let mut lock = self.census.lock_items();
        lock.record_death();
        self.census.inner.condvar.notify_all();
    }
}

impl<T> Deref for TrackedObject<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner.item
    }
}

impl<T> AsRef<T> for TrackedObject<T> {
    fn as_ref(&self) -> &T {
        &self.inner.item
    }
}

impl<T> Borrow<T> for TrackedObject<T> {
    fn borrow(&self) -> &T {
        &self.inner.item
    }
}

#[cfg(test)]
mod tests {

    use super::Inventory;
    use std::sync::mpsc::channel;
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn test_census_map() {
        let census = Inventory::new();
        let a = census.track(1);
        let _b = a.map(|v| v * 7);
        assert_eq!(census.len(), 2);
        assert_eq!(
            census.list().into_iter().map(|m| *m).collect::<Vec<_>>(),
            vec![1, 7]
        );
    }

    #[test]
    fn test_census() {
        let census = Inventory::new();
        let _a = census.track(1);
        let _b = census.track(3);
        assert_eq!(census.len(), 2,);
        assert_eq!(
            census.list().into_iter().map(|m| *m).collect::<Vec<_>>(),
            vec![1, 3]
        );
    }

    #[test]
    fn test_census_2() {
        let census = Inventory::new();
        {
            let _a = census.track(1);
            let _b = census.track(3);
            // dropping both here
        }
        assert_eq!(census.len(), 0);
        assert!(census.list().is_empty());
    }

    #[test]
    fn test_census_3() {
        let census = Inventory::new();
        let a = census.track(1);
        let _a2 = a.clone();
        drop(a);
        assert_eq!(census.len(), 1);
        assert_eq!(
            census.list().into_iter().map(|m| *m).collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn test_census_list_extends_life() {
        let census = Inventory::new();
        let a = census.track(1);
        let living = census.list();
        assert_eq!(living.len(), 1);
        drop(a);
        let living_2 = census.list();
        assert_eq!(living_2.len(), 1);
        drop(living_2);
        drop(living);
        assert_eq!(census.len(), 0);
        assert!(census.list().is_empty());
    }

    #[test]
    fn test_census_race_condition() {
        let census = Inventory::new();
        let census_clone = census.clone();
        thread::spawn(move || {
            for _ in 0..1_000 {
                let _a = census_clone.track(1);
            }
        });
        for i in 0..10_000 {
            println!("i {}", i);
            census.list();
        }
    }

    #[test]
    fn test_census_concurrent_drop() {
        let census = Inventory::new();
        let mut senders = Vec::new();
        let mut handles = Vec::new();
        let barrier = Arc::new(Barrier::new(2));
        for _ in 0..2 {
            let (send, recv) = channel();
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                for obj in recv {
                    barrier.wait();
                    drop(obj);
                }
            }));
            senders.push(send);
        }
        for i in 0..50_000 {
            let tracked = census.track(i);
            for send in &senders {
                send.send(tracked.clone()).unwrap();
            }
        }
        drop(senders);
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(census.len(), 0);
    }

    fn test_census_changes_iter_util(el: usize) {
        let census = Inventory::new();
        for i in 0..el {
            let tracked = census.track(i);
            thread::spawn(move || {
                let _tracked = tracked;
            });
        }
        census.wait_until_empty();
        assert_eq!(census.len(), 0);
        assert!(census.list().is_empty());
    }

    #[test]
    fn test_census_changes_iter_many() {
        for i in 1..200 {
            test_census_changes_iter_util(i);
        }
    }
}
