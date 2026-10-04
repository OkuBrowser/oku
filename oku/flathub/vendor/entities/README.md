# entities [![Build Status](https://travis-ci.org/p-jackson/entities.svg?branch=master)](https://travis-ci.org/p-jackson/entities) [![Crates.io](https://img.shields.io/crates/v/entities.svg?maxAge=3600)](https://crates.io/crates/entities)

Provides the raw data needed to convert to and from HTML entities.

## Basic Usage

```rust
extern crate entities;
use entities::ENTITIES;

fn main() {
    let entity = ENTITIES
        .iter()
        .find(|e| e.entity == "&gt;")
        .unwrap();

    assert_eq!(entity.characters, ">");
    assert_eq!(entity.entity, "&gt;");
}
```

There isn't a 1-to-1 mapping of entities to "characters" which is why this
crate provides a flat array rather than a map&mdash;the best way to map the
entities depends on the problem _you're_ trying to solve.

If you want to create a mapping structure you can make one using static `str`
slices to reuse the statically allocated strings from this crate e.g.

```rust
fn make_mapping() -> HashMap<&'static str, &'static str> {
    let mut mapping = HashMap::new();
    mapping.insert(ENTITIES[0].entity, ENTITIES[0].characters);
    mapping
}
```

## Documentation

[https://docs.rs/entities](https://docs.rs/entities)
