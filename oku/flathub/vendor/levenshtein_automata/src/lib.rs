/*!

Build a deterministic finite automaton (DFA) that computes
the levenshtein distance from a given string.


# Example

```rust
# extern crate levenshtein_automata;
use levenshtein_automata::{LevenshteinAutomatonBuilder, Distance};
# fn main() {

// Building this factory is not free.
let lev_automaton_builder = LevenshteinAutomatonBuilder::new(2, true);

// We can now build an entire dfa.
let dfa = lev_automaton_builder.build_dfa("Levenshtein");

let mut state = dfa.initial_state();
    for &b in "Levenshtain".as_bytes() {
    state = dfa.transition(state, b);
}

assert_eq!(dfa.distance(state), Distance::Exact(1));
# }
```

The implementation is based on the following paper
**Fast String Correction with Levenshtein-Automata (2002)** by by Klaus Schulz and Stoyan Mihov.
I also tried to explain it in the following [blog post](https://fulmicoton.com/posts/levenshtein/).

!*/

#![cfg_attr(test, feature(test))]

#[cfg(test)]
extern crate test;

#[cfg(test)]
mod bench;
#[cfg(test)]
mod tests;

mod alphabet;
mod dfa;
mod index;
mod levenshtein_nfa;
mod parametric_dfa;

pub use self::dfa::{DFA, SINK_STATE};
use self::index::Index;
pub use self::levenshtein_nfa::Distance;
use self::levenshtein_nfa::LevenshteinNFA;
use self::parametric_dfa::ParametricDFA;

/// Builder for Levenshtein Automata.
///
/// It wraps a precomputed datastructure that allows to
/// produce small (but not minimal) DFA.
pub struct LevenshteinAutomatonBuilder {
    parametric_dfa: ParametricDFA,
}

impl LevenshteinAutomatonBuilder {
    /// Creates a Levenshtein automaton builder.
    /// The builder
    ///
    /// * `max_distance` - maximum distance considered by the automaton.
    /// * `transposition_cost_one` - assign a distance of 1 for transposition
    ///
    /// Building this automaton builder is computationally intensive.
    /// While it takes only a few milliseconds for `d=2`, it grows exponentially with
    /// `d`. It is only reasonable to `d <= 5`.
    pub fn new(max_distance: u8, transposition_cost_one: bool) -> LevenshteinAutomatonBuilder {
        let levenshtein_nfa = LevenshteinNFA::levenshtein(max_distance, transposition_cost_one);
        let parametric_dfa = ParametricDFA::from_nfa(&levenshtein_nfa);
        LevenshteinAutomatonBuilder {
            parametric_dfa: parametric_dfa,
        }
    }

    /// Builds a Finite Determinstic Automaton to compute
    /// the levenshtein distance to a fixed given `query`.
    ///
    /// There is no guarantee that the resulting DFA is minimal
    /// but its number of states is guaranteed to be smaller
    /// than `C * (query.len() + 1)` in which C is a constant that depends
    /// on the distance as well as whether transposition are supported
    /// or not.
    ///
    /// For instance for `d=2` and with transposition, `C=68`.
    pub fn build_dfa(&self, query: &str) -> DFA {
        self.parametric_dfa.build_dfa(query, false)
    }

    /// Builds a Finite Determinstic Automaton that computes
    /// the prefix levenshtein distance to a given `query`.
    ///
    /// Given a test string, the resulting distance is defined as
    ///
    /// ```formula
    ///     min( levenshtein(&test_string[..i], query } for i in 0..test_string.len() )
    /// ```
    ///
    /// Which translates as *the minimum distance of the prefixes of `test_strings`*.
    ///
    /// See also [.build_dfa(...)](./struct.LevenshteinAutomatonBuilder.html#method.build_dfa).
    pub fn build_prefix_dfa(&self, query: &str) -> DFA {
        self.parametric_dfa.build_dfa(query, true)
    }
}
