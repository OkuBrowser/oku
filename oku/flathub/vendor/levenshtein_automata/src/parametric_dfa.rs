use super::alphabet::Alphabet;
use super::dfa::{Utf8DFABuilder, DFA};
use super::levenshtein_nfa::Distance;
use super::levenshtein_nfa::{LevenshteinNFA, MultiState};
use super::Index;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ParametricState {
    shape_id: u32,
    offset: u32,
}

impl ParametricState {
    fn empty() -> ParametricState {
        ParametricState {
            shape_id: 0u32,
            offset: 0u32,
        }
    }
    fn is_dead_end(&self) -> bool {
        self.shape_id == 0
    }
}

#[derive(Clone, Copy)]
pub struct Transition {
    dest_shape_id: u32,
    delta_offset: u32,
}

impl Transition {
    fn apply(&self, state: ParametricState) -> ParametricState {
        ParametricState {
            shape_id: self.dest_shape_id,
            offset: if self.dest_shape_id == 0 {
                0 //< We don't need any offset if we are in the dead state.
                  // This ensures we have only one dead state.
            } else {
                state.offset + self.delta_offset
            },
        }
    }
}

struct ParametricStateIndex {
    state_index: Vec<Option<u32>>,
    state_queue: Vec<ParametricState>,
    num_offsets: usize,
}

impl ParametricStateIndex {
    fn new(query_len: usize, num_param_states: usize) -> ParametricStateIndex {
        let num_offsets = query_len + 1;
        let max_num_states = num_param_states * num_offsets;
        ParametricStateIndex {
            state_index: vec![None; max_num_states],
            state_queue: Vec::with_capacity(100),
            num_offsets: num_offsets,
        }
    }

    fn num_states(&self) -> usize {
        self.state_queue.len()
    }

    fn max_num_states(&self) -> usize {
        self.state_index.len()
    }

    fn get_or_allocate(&mut self, parametric_state: ParametricState) -> u32 {
        let bucket = (parametric_state.shape_id as usize) * self.num_offsets
            + parametric_state.offset as usize;
        if let Some(state_id) = self.state_index[bucket] {
            return state_id;
        }
        let new_state = self.state_queue.len() as u32;
        self.state_queue.push(parametric_state);
        self.state_index[bucket] = Some(new_state);
        new_state
    }

    fn get(&self, state_id: u32) -> ParametricState {
        self.state_queue[state_id as usize]
    }
}

pub struct ParametricDFA {
    distance: Vec<u8>,
    transitions: Vec<Transition>,
    max_distance: u8,
    transition_stride: usize,
    diameter: usize,
}

impl ParametricDFA {
    pub fn initial_state() -> ParametricState {
        ParametricState {
            shape_id: 1,
            offset: 0,
        }
    }

    // Returns true iff whatever characters come afterward, we will never reach
    // a shorter distance
    fn is_prefix_sink(&self, state: ParametricState, query_len: usize) -> bool {
        if state.is_dead_end() {
            return true;
        }
        let remaining_offset: usize = query_len - state.offset as usize;
        if remaining_offset < self.diameter {
            let state_distances = &self.distance[(self.diameter * state.shape_id as usize)..];
            let prefix_distance = state_distances[remaining_offset];
            if prefix_distance > self.max_distance {
                return false;
            }
            for potential_distance in state_distances[..remaining_offset].iter().cloned() {
                if potential_distance < prefix_distance {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Builds a [DFA] for the given query. If `prefix` is set to `true`, the resulting
    /// DFA will match whenever the `query` is a prefix of the input being processed.
    pub fn build_dfa(&self, query: &str, prefix: bool) -> DFA {
        self.build_custom_dfa(query, prefix, false)
    }

    /// Builds a [DFA] for the given query. If `prefix` is set to `true`, the resulting
    /// DFA will match whenever the `query` is a prefix of the input being processed.
    /// If `use_applied_distance` is set to `true`, the distance being reported isn't the
    /// "absolute distance" for the query but rather the distance (number of edits) which
    /// have been applied so far.
    pub fn build_custom_dfa(&self, query: &str, prefix: bool, use_applied_distance: bool) -> DFA {
        let query_chars: Vec<char> = query.chars().collect();
        let query_len = query_chars.len();
        let alphabet = Alphabet::for_query_chars(&query_chars);

        let mut parametric_state_index = ParametricStateIndex::new(query_len, self.num_states());
        let max_num_states = parametric_state_index.max_num_states();

        let dead_end_state_id = parametric_state_index.get_or_allocate(ParametricState::empty());
        assert_eq!(dead_end_state_id, 0);
        let initial_state_id =
            parametric_state_index.get_or_allocate(ParametricDFA::initial_state());

        let mut dfa_builder = Utf8DFABuilder::with_max_num_states(max_num_states);
        let mask = (1 << self.diameter) - 1;

        for state_id in 0u32.. {
            if state_id == parametric_state_index.num_states() as u32 {
                break;
            }
            let state = parametric_state_index.get(state_id);
            let distance = if use_applied_distance {
                self.applied_distance(state)
            } else {
                self.distance(state, query_len)
            };

            if prefix && self.is_prefix_sink(state, query_len) {
                dfa_builder.add_state(state_id, distance, state_id);
            } else {
                let default_successor = self.transition(state, 0u32).apply(state);
                let default_successor_id =
                    parametric_state_index.get_or_allocate(default_successor);
                let mut state_builder =
                    dfa_builder.add_state(state_id, distance, default_successor_id);
                for &(ref chr, ref characteristic_vec) in alphabet.iter() {
                    let chi = characteristic_vec.shift_and_mask(state.offset as usize, mask);
                    let dest_state: ParametricState = self.transition(state, chi).apply(state);
                    let dest_state_id = parametric_state_index.get_or_allocate(dest_state);
                    state_builder.add_transition(*chr, dest_state_id);
                }
            }
        }

        dfa_builder.set_initial_state(initial_state_id);
        dfa_builder.build()
    }

    pub fn num_states(&self) -> usize {
        self.transitions.len() / self.transition_stride
    }

    // only for debug
    #[cfg(test)]
    pub fn compute_distance(&self, left: &str, right: &str) -> Distance {
        use super::levenshtein_nfa::compute_characteristic_vector;
        use std::cmp;
        let mut state = Self::initial_state();
        let left_chars: Vec<char> = left.chars().collect();
        for chr in right.chars() {
            let start = state.offset as usize;
            let stop = cmp::min(start + self.diameter, left_chars.len());
            let chi = compute_characteristic_vector(&left_chars[start..stop], chr) as u32;
            state = self.transition(state, chi).apply(state);
            if state.is_dead_end() {
                return Distance::AtLeast(self.max_distance + 1u8);
            }
        }
        self.distance(state, left.len())
    }

    pub fn distance(&self, state: ParametricState, query_len: usize) -> Distance {
        let remaining_offset: usize = query_len - state.offset as usize;
        if state.is_dead_end() || remaining_offset >= self.diameter {
            Distance::AtLeast(self.max_distance + 1u8)
        } else {
            let d = self.distance[(self.diameter * state.shape_id as usize) + remaining_offset];
            if d > self.max_distance {
                Distance::AtLeast(d)
            } else {
                Distance::Exact(d)
            }
        }
    }

    pub fn applied_distance(&self, state: ParametricState) -> Distance {
        let d = self.distance[self.diameter * state.shape_id as usize];
        if d > self.max_distance {
            Distance::AtLeast(d)
        } else {
            Distance::Exact(d)
        }
    }

    pub fn transition(&self, state: ParametricState, chi: u32) -> Transition {
        assert!((chi as usize) < self.transition_stride);
        self.transitions[self.transition_stride * state.shape_id as usize + chi as usize]
    }

    pub fn from_nfa(nfa: &LevenshteinNFA) -> ParametricDFA {
        let mut index: Index<MultiState> = Index::new();
        index.get_or_allocate(&MultiState::empty());
        let initial_state = nfa.initial_states();
        index.get_or_allocate(&initial_state);

        let max_distance = nfa.max_distance();
        let multistate_diameter = nfa.multistate_diameter();
        let mut transitions: Vec<Transition> = vec![];

        let num_chi = 1 << multistate_diameter;
        let chi_values: Vec<u64> = (0..num_chi).collect();

        let mut dest_multistate = MultiState::empty();

        for state_id in 0.. {
            if state_id == index.len() {
                break;
            }
            for &chi in &chi_values {
                {
                    let multistate: &MultiState = index.get_from_id(state_id);
                    nfa.transition(multistate, &mut dest_multistate, chi);
                }
                let translation = dest_multistate.normalize();
                let dest_id = index.get_or_allocate(&dest_multistate);
                transitions.push(Transition {
                    dest_shape_id: dest_id,
                    delta_offset: translation,
                });
            }
        }

        let num_states = index.len();
        let multistate_diameter = multistate_diameter as usize;
        let mut distance: Vec<u8> = Vec::with_capacity(multistate_diameter * num_states as usize);

        for state_id in 0..num_states {
            let multistate = index.get_from_id(state_id);
            for offset in 0..multistate_diameter {
                let dist = nfa.multistate_distance(multistate, offset as u32).to_u8();
                distance.push(dist);
            }
        }

        ParametricDFA {
            transition_stride: num_chi as usize,
            distance,
            max_distance,
            transitions,
            diameter: multistate_diameter as usize,
        }
    }
}
