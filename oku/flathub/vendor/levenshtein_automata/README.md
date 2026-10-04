# Levenshtein-automaton

This crate makes it fast and simple to build a finite determinic automaton that computes
the levenshtein distance from a given string.

# Example

```rust
use levenshtein_automata::{LevenshteinAutomatonBuilder, Distance};

fn main() {

    // Building this factory is not free.
    let lev_automaton_builder = LevenshteinAutomatonBuilder::new(2, true);

    // We can now build an entire dfa.
    let dfa = lev_automaton_builder.build_dfa("Levenshtein");

    let mut state = dfa.initial_state();
    for &b in "Levenshtain".as_bytes() {
        state = dfa.transition(state, b);
    }

    assert_eq!(dfa.distance(state), Distance::Exact(1));
}
```

The implementation is based on the following paper
**Fast String Correction with Levenshtein-Automata (2002)** by by Klaus Schulz and Stoyan Mihov.
I also tried to explain it in the following [blog post](https://fulmicoton.com/posts/levenshtein/).


# Bench


The time taken by the construction a Levenshtein DFA
strongly depends on the max distance it can measure and the length of the input string.

Here is the time spent to build a Levenshtein DFA for the string "Levenshtein"


```ignore
dfa dist1 no transposition        35,627 ns/iter (+/- 3,237)
dfa dist1 with transposition      36,493 ns/iter (+/- 12,680)
dfa dist2 no transposition        97,137 ns/iter (+/- 14,556)
dfa dist2 with transposition     100,958 ns/iter (+/- 4,231)
dfa dist3 no transposition       834,412 ns/iter (+/- 158,329)
dfa dist3 with transposition   1,414,523 ns/iter (+/- 396,278)
dfa dist4 no transposition     4,716,365 ns/iter (+/- 869,024)
dfa dist4 with transposition   8,044,162 ns/iter (+/- 594,523)
```
