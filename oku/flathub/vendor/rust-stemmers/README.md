# Rust Stemmers

This crate implements some stemmer algorithms found in the [snowball project](http://snowballstem.org/) which are compiled to rust using the rust-backend of the [snowball compiler](https://github.com/snowballstem/snowball).

# Supported Algorithms

-   Arabic
-   Danish
-   Dutch
-   English
-   French
-   German
-   Greek
-   Hungarian
-   Italian
-   Norwegian
-   Portuguese
-   Romanian
-   Russian
-   Spanish
-   Swedish
-   Tamil
-   Turkish


# Usage

```rust
extern crate rust_stemmers;
use rust_stemmers::{Algorithm, Stemmer};

// Create a stemmer for the english language
let en_stemmer = Stemmer::create(Algorithm::English);

// Stemm the word "fruitlessly"
// Please be aware that all algorithms expect their input to only contain lowercase characters.
assert_eq!(en_stemmer.stem("fruitlessly"), "fruitless");
```

# Related Projects

-   The [stemmer](https://github.com/lise-henry/stemmer-rs) crate provides bindings to the C Snowball implementation.

