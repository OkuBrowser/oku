# Census
[![Build status](https://ci.appveyor.com/api/projects/status/5eb6wkt9jpr7qqaj/branch/master?svg=true)](https://ci.appveyor.com/project/fulmicoton/census/branch/master)


This crate makes it possible to create an inventory object that keeps track of
instances of a given type.

It is used in tantivy to get an accurate list of all of the files that are still in use
by the index, and avoid garbage collecting them.


This `TrackedObject<T>` instance include some reference counting logic to ensure that
the object is removed from the inventory once the last instance is dropped.


# Example

```rust

extern crate census;

use census::{Inventory, TrackedObject};

fn main() {
    let inventory = Inventory::new();

    //  Each object tracked needs to be registered explicitely in the Inventory.
    //  A `TrackedObject<T>` wrapper is then returned.
    let one = inventory.track("one".to_string());
    let two = inventory.track("two".to_string());

    // A snapshot  of the list of living instances can be obtained...
    // (no guarantee on their order)
    let living_instances: Vec<TrackedObject<String>> = inventory.list();
    assert_eq!(living_instances.len(), 2);
}
```