use std::cmp::Ordering;

#[cfg(test)]
pub fn compute_characteristic_vector(query: &[char], c: char) -> u64 {
    let mut chi = 0u64;
    for i in 0..query.len() {
        if query[i] == c {
            chi |= 1u64 << i;
        }
    }
    chi
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct MultiState {
    states: Vec<NFAState>,
}

impl MultiState {
    pub fn states(&self) -> &[NFAState] {
        &self.states[..]
    }

    fn clear(&mut self) {
        self.states.clear()
    }

    pub fn empty() -> MultiState {
        MultiState { states: Vec::new() }
    }

    pub fn normalize(&mut self) -> u32 {
        let min_offset: u32 = self
            .states
            .iter()
            .map(|state| state.offset)
            .min()
            .unwrap_or(0u32);
        for state in &mut self.states {
            state.offset -= min_offset;
        }
        self.states.sort();
        min_offset
    }

    fn add_state(&mut self, new_state: NFAState) {
        if self.states.iter().any(|state| state.imply(new_state)) {
            // this state is already included in the current set of states.
            return;
        }

        let mut i = 0;
        while i < self.states.len() {
            if new_state.imply(self.states[i]) {
                self.states.swap_remove(i);
            } else {
                i += 1;
            }
        }

        self.states.push(new_state);
    }
}

/// Levenshtein Distance computed by a Levenshtein Automaton.
///
/// Levenshtein automata can only compute the exact Levenshtein distance
/// up to a given `max_distance`.
///
/// Over this distance, the automaton will invariably
/// return `Distance::AtLeast(max_distance + 1)`.
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Distance {
    Exact(u8),
    AtLeast(u8),
}

impl Distance {
    /// Returns the highest lower bound for the distance.
    /// It is equivalent to
    ///
    /// ```ignored
    /// match distance {
    ///     Distance::Exact(d) |
    ///     Distance::AtLeast(d) => d,
    /// }
    /// ```
    pub fn to_u8(&self) -> u8 {
        match *self {
            Distance::Exact(d) | Distance::AtLeast(d) => d,
        }
    }
}

impl PartialOrd for Distance {
    fn partial_cmp(&self, other: &Distance) -> Option<Ordering> {
        use self::Distance::*;
        match (*self, *other) {
            (Exact(left), Exact(right)) => left.partial_cmp(&right),
            (Exact(left), AtLeast(right)) => {
                if right > left {
                    Some(Ordering::Greater)
                } else {
                    None
                }
            }
            (AtLeast(left), Exact(right)) => {
                if left > right {
                    Some(Ordering::Less)
                } else {
                    None
                }
            }
            (AtLeast(left), AtLeast(right)) => {
                if left == right {
                    Some(Ordering::Equal)
                } else {
                    None
                }
            }
        }
    }
}

pub struct LevenshteinNFA {
    max_distance: u8,
    damerau: bool,
}

fn extract_bit(bitset: u64, pos: u8) -> bool {
    let pos = pos as usize;
    let shift = bitset >> pos;
    let bit = shift & 1;
    bit == 1u64
}

fn dist(left: u32, right: u32) -> u32 {
    if left > right {
        left - right
    } else {
        right - left
    }
}

impl LevenshteinNFA {
    pub fn levenshtein(max_distance: u8, transposition: bool) -> LevenshteinNFA {
        LevenshteinNFA {
            max_distance: max_distance,
            damerau: transposition,
        }
    }

    pub fn multistate_distance(&self, multistate: &MultiState, query_len: u32) -> Distance {
        multistate
            .states()
            .iter()
            .map(|state| state.distance + dist(query_len, state.offset) as u8)
            .filter(|d| *d <= self.max_distance)
            .min()
            .map(Distance::Exact)
            .unwrap_or_else(|| Distance::AtLeast(self.max_distance + 1u8))
    }

    pub fn max_distance(&self) -> u8 {
        self.max_distance
    }

    pub fn multistate_diameter(&self) -> u8 {
        2u8 * self.max_distance + 1u8
    }

    pub fn initial_states(&self) -> MultiState {
        let mut multistate = MultiState::empty();
        multistate.add_state(NFAState::default());
        multistate
    }

    #[cfg(test)]
    pub fn compute_distance(&self, query: &str, other: &str) -> Distance {
        use std::mem;
        let query_chars: Vec<char> = query.chars().collect();
        let mut current_state = self.initial_states();
        let mut next_state = MultiState::empty();
        for chr in other.chars() {
            next_state.clear();
            let chi: u64 = compute_characteristic_vector(&query_chars[..], chr);
            self.transition(&current_state, &mut next_state, chi);
            mem::swap(&mut current_state, &mut next_state);
        }
        self.multistate_distance(&current_state, query_chars.len() as u32)
    }

    fn simple_transition(&self, state: NFAState, symbol: u64, multistate: &mut MultiState) {
        if state.distance < self.max_distance {
            // apparently we still have room to
            // make mistakes.

            // insertion
            multistate.add_state(NFAState {
                offset: state.offset,
                distance: state.distance + 1,
                in_transpose: false,
            });

            // substitution
            multistate.add_state(NFAState {
                offset: state.offset + 1,
                distance: state.distance + 1,
                in_transpose: false,
            });

            for d in 1u8..self.max_distance + 1u8 - state.distance {
                if extract_bit(symbol, d) {
                    // for d > 0, as many deletion and character match
                    multistate.add_state(NFAState {
                        offset: state.offset + 1 + u32::from(d),
                        distance: state.distance + d,
                        in_transpose: false,
                    });
                }
            }

            if self.damerau && extract_bit(symbol, 1) {
                multistate.add_state(NFAState {
                    offset: state.offset,
                    distance: state.distance + 1,
                    in_transpose: true,
                });
            }
        }
        if extract_bit(symbol, 0) {
            multistate.add_state(NFAState {
                offset: state.offset + 1,
                distance: state.distance,
                in_transpose: false,
            });
        }

        if state.in_transpose && extract_bit(symbol, 0u8) {
            multistate.add_state(NFAState {
                offset: state.offset + 2,
                distance: state.distance,
                in_transpose: false,
            });
        }
    }

    pub(crate) fn transition(
        &self,
        current_state: &MultiState,
        dest_state: &mut MultiState,
        shifted_chi_vector: u64,
    ) {
        dest_state.clear();
        let mask = (1u64 << self.multistate_diameter()) - 1u64;
        for &state in current_state.states() {
            let shifted_chi_vector = (shifted_chi_vector >> state.offset as usize) & mask;
            self.simple_transition(state, shifted_chi_vector, dest_state);
        }
        dest_state.states.sort();
    }
}

#[derive(Default, Hash, Eq, PartialOrd, Ord, PartialEq, Copy, Clone, Debug)]
pub struct NFAState {
    offset: u32,
    distance: u8,
    in_transpose: bool,
}

impl NFAState {
    fn imply(&self, other: NFAState) -> bool {
        let tranpose_imply = self.in_transpose | !other.in_transpose;
        let delta_offset: u32 = if self.offset >= other.offset {
            self.offset - other.offset
        } else {
            other.offset - self.offset
        };
        if tranpose_imply {
            u32::from(other.distance) >= u32::from(self.distance) + delta_offset
        } else {
            u32::from(other.distance) > u32::from(self.distance) + delta_offset
        }
    }
}
