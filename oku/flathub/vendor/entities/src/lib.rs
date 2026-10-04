//! Provides the raw data needed to convert to and from HTML entities.
//!
//! ```
//! extern crate entities;
//! use entities::ENTITIES;
//!
//! fn main() {
//!     let entity = ENTITIES
//!         .iter()
//!         .find(|e| e.entity == "&gt;")
//!         .unwrap();
//!
//!     assert_eq!(entity.characters, ">");
//!     assert_eq!(entity.entity, "&gt;");
//! }
//! ```
//!
//! There isn't a 1-to-1 mapping of entities to "characters" which is why this
//! crate provides a flat array rather than a map&mdash;the best way to map the
//! entities depends on the problem _you're_ trying to solve.
//!
//! If you want to create a mapping structure you can make one using static `str`
//! slices to reuse the statically allocated strings from this crate e.g.
//!
//! ```
//! # use self::std::collections::HashMap;
//! # use entities::ENTITIES;
//! fn make_mapping() -> HashMap<&'static str, &'static str> {
//!     let mut mapping = HashMap::new();
//!     mapping.insert(ENTITIES[0].entity, ENTITIES[0].characters);
//!     mapping
//! }
//! ```
//!
//! Data was generated from
//! [https://www.w3.org/TR/html5/entities.json](https://www.w3.org/TR/html5/entities.json)


#![no_std]


mod entities;


/// The unicode codepoint(s) for the "character" an entity is encoding.
#[derive(Debug)]
pub enum Codepoints {
    Single(u32),
    Double(u32, u32),
}


/// Represents an entry in the array of entity definitions.
#[derive(Debug)]
pub struct Entity {
    pub entity: &'static str,
    pub codepoints: Codepoints,
    pub characters: &'static str,
}


pub use entities::ENTITIES;


#[cfg(test)]
mod tests {
    extern crate std;
    use self::std::collections::HashMap;
    use super::ENTITIES;

    #[test]
    fn sanity() {
        let found = ENTITIES.iter().find(|e| e.entity == "&gt;");
        assert!(found.is_some());
        assert_eq!(found.unwrap().characters, ">");
    }

    #[test]
    fn make_map() {
        let mut mapping = HashMap::<&'static str, &'static str>::new();
        mapping.insert(ENTITIES[0].entity, ENTITIES[0].characters);
        assert_eq!(mapping.get(ENTITIES[0].entity),
                   Some(&ENTITIES[0].characters));
    }
}
