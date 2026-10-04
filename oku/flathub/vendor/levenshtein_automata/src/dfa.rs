use super::Distance;

/// Sink state. See [DFA](./index.html)
pub const SINK_STATE: u32 = 0u32;

/// Implementation of a Deterministic Finite Automaton for
/// a Levenshtein Automaton targeting UTF-8 encoded strings.
///
/// The automaton does not validate `utf-8`.
/// It will not return errors when fed with invalid `utf-8`
///
/// The only `sink` state is guaranteed to be `SINK`.
///
/// This means that if you reach the sink state you are
/// guaranteed that regardless of the sequence of bytes
/// you might consume in the future, you will always
/// remain in the same state.
///
/// This property can be exploited to abort further
/// evaluation.
///
///
/// # Usage
///
/// ```rust
/// # extern crate levenshtein_automata;
/// # use levenshtein_automata::{LevenshteinAutomatonBuilder, Distance};
/// # use levenshtein_automata::SINK_STATE;
/// # fn main() {
/// #   // Building this factory is not free.
/// #   let lev_automaton_builder = LevenshteinAutomatonBuilder::new(2, true);
/// #  // We can now build an entire dfa.
/// #  let dfa = lev_automaton_builder.build_dfa("Levenshtein");
/// # let str = "Levenshtain";
/// let mut state = dfa.initial_state();
/// for &byte in str.as_bytes() {
///     state = dfa.transition(state, byte);
///     if state == SINK_STATE {
///         break;
///     }
/// }
/// let distance = dfa.distance(state);
/// # }
//```
pub struct DFA {
    transitions: Vec<[u32; 256]>,
    distances: Vec<Distance>,
    initial_state: u32,
}

impl DFA {
    /// Returns the initial state
    pub fn initial_state(&self) -> u32 {
        self.initial_state
    }

    /// Helper function that consumes all of the bytes
    /// a sequence of bytes and returns the resulting
    /// distance.
    pub fn eval<B: AsRef<[u8]>>(&self, text: B) -> Distance {
        let mut state = self.initial_state();
        for &b in text.as_ref() {
            state = self.transition(state, b);
        }
        self.distance(state)
    }

    /// Returns the Levenshtein distance associated to the
    /// current state.
    pub fn distance(&self, state_id: u32) -> Distance {
        self.distances[state_id as usize]
    }

    /// Returns the number of states in the `DFA`.
    pub fn num_states(&self) -> usize {
        self.transitions.len()
    }

    /// Returns the destination state reached after consuming a given byte.
    pub fn transition(&self, from_state_id: u32, b: u8) -> u32 {
        self.transitions[from_state_id as usize][b as usize]
    }
}

#[cfg(feature = "fst_automaton")]
use fst;
#[cfg(feature = "fst_automaton")]
impl fst::Automaton for DFA {
    type State = u32;

    fn start(&self) -> u32 {
        self.initial_state()
    }

    fn is_match(&self, state: &u32) -> bool {
        match self.distance(*state) {
            Distance::Exact(d) => true,
            Distance::AtLeast(_) => false,
        }
    }

    fn can_match(&self, state: &u32) -> bool {
        *state != SINK_STATE
    }

    fn accept(&self, state: &u32, byte: u8) -> u32 {
        self.transition(*state, byte)
    }
}

fn fill(dest: &mut [u32], val: u32) {
    for d in dest {
        *d = val;
    }
}

pub struct Utf8DFAStateBuilder<'a> {
    dfa_builder: &'a mut Utf8DFABuilder,
    state_id: u32,
    default_successor: [u32; 4],
}

impl<'a> Utf8DFAStateBuilder<'a> {
    fn add_transition_id(&mut self, from_state_id: u32, b: u8, to_state_id: u32) {
        self.dfa_builder.transitions[from_state_id as usize][b as usize] = to_state_id;
    }

    pub fn add_transition(&mut self, chr: char, to_state_id: u32) {
        let mut buffer = [0u8; 4];

        // The char may translate into more than one bytes.
        // We create a chain for this reason.
        let bytes: &[u8] = chr.encode_utf8(&mut buffer).as_bytes();
        let mut from_state_id_decoded = self.state_id;
        for (i, b) in bytes[..bytes.len() - 1].iter().cloned().enumerate() {
            let remaining_num_bytes = bytes.len() - i as usize - 1 as usize;
            let default_successor = self.default_successor[remaining_num_bytes];
            let mut intermediary_state_id: u32 =
                self.dfa_builder.transitions[from_state_id_decoded as usize][b as usize];
            if intermediary_state_id == default_successor {
                intermediary_state_id = self.dfa_builder.allocate();
                fill(
                    &mut self.dfa_builder.transitions[intermediary_state_id as usize],
                    self.default_successor[remaining_num_bytes - 1],
                );
            }
            self.add_transition_id(from_state_id_decoded, b, intermediary_state_id);
            from_state_id_decoded = intermediary_state_id;
        }

        let to_state_id_decoded = self
            .dfa_builder
            .get_or_allocate(Utf8StateId::original(to_state_id));
        self.add_transition_id(
            from_state_id_decoded,
            bytes[bytes.len() - 1],
            to_state_id_decoded,
        );
    }
}

/// `Utf8DFABuilder` makes it possible to define a DFA
/// that takes unicode character, and build a `DFA`
/// that operates on utf-8 encoded `&[u8]`.
pub struct Utf8DFABuilder {
    index: Vec<Option<u32>>,
    distances: Vec<Distance>,
    transitions: Vec<[u32; 256]>,
    initial_state: u32,
    num_states: u32,
    max_num_states: u32,
}

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
struct Utf8StateId(u32);
impl Utf8StateId {
    pub fn original(state_id: u32) -> Utf8StateId {
        Utf8StateId::predecessor(state_id, 0u8)
    }

    pub fn predecessor(state_id: u32, num_steps: u8) -> Utf8StateId {
        Utf8StateId(state_id * 4u32 + u32::from(num_steps))
    }
}

impl Utf8DFABuilder {
    /// Creates a new dictionary.
    ///
    /// The `builder` will only accept `state_id` that are
    /// lower than `max_num_states`.
    pub fn with_max_num_states(max_num_states: usize) -> Utf8DFABuilder {
        Utf8DFABuilder {
            index: vec![None; max_num_states * 4 + 3],
            distances: Vec::with_capacity(100),
            transitions: Vec::with_capacity(100),
            initial_state: 0u32,
            num_states: 0u32,
            max_num_states: max_num_states as u32,
        }
    }

    fn allocate(&mut self) -> u32 {
        let new_state = self.num_states;
        self.num_states += 1;
        self.distances
            .resize(new_state as usize + 1, Distance::AtLeast(255));
        self.transitions.resize(new_state as usize + 1, [0u32; 256]);
        new_state
    }

    fn get_or_allocate(&mut self, state: Utf8StateId) -> u32 {
        let state_bucket = state.0 as usize;
        if let Some(state) = self.index[state_bucket] {
            return state;
        }
        let new_state = self.allocate();
        self.index[state_bucket] = Some(new_state);
        new_state
    }

    pub fn set_initial_state(&mut self, initial_state: u32) {
        let state_id_decoded = self.get_or_allocate(Utf8StateId::original(initial_state));
        self.initial_state = state_id_decoded
    }

    /// Define a new state.
    pub fn add_state(
        &mut self,
        state: u32,
        distance: Distance,
        default_successor_orig: u32,
    ) -> Utf8DFAStateBuilder {
        assert!(
            state < self.max_num_states,
            "State id is larger than max_num_states"
        );
        let state_id = self.get_or_allocate(Utf8StateId::original(state));
        self.distances[state_id as usize] = distance;

        let default_successor_id =
            self.get_or_allocate(Utf8StateId::original(default_successor_orig));

        // creates a chain of states of predecessors of `default_successor_orig`.
        // Accepting k-bytes (whatever the bytes are) from `predecessor_states[k-1]`
        // leads to the `default_successor_orig` state.
        let mut predecessor_states = [default_successor_id; 4];

        {
            for num_bytes in 1..4 {
                let predecessor_state =
                    Utf8StateId::predecessor(default_successor_orig, num_bytes as u8);
                let predecessor_state_id = self.get_or_allocate(predecessor_state);
                predecessor_states[num_bytes] = predecessor_state_id;
                let succ = predecessor_states[num_bytes - 1];
                fill(&mut self.transitions[predecessor_state_id as usize], succ);
            }
        }

        {
            let transitions = &mut self.transitions[state_id as usize];
            // 1-byte encoded chars.
            fill(&mut transitions[0..192], predecessor_states[0]);
            // 2-bytes encoded chars.
            fill(&mut transitions[192..224], predecessor_states[1]);
            // 3-bytes encoded chars.
            fill(&mut transitions[224..240], predecessor_states[2]);
            // 4-bytes encoded chars.
            fill(&mut transitions[240..256], predecessor_states[3]);
        }

        Utf8DFAStateBuilder {
            dfa_builder: self,
            state_id,
            default_successor: predecessor_states,
        }
    }

    pub fn build(self) -> DFA {
        DFA {
            transitions: self.transitions,
            distances: self.distances,
            initial_state: self.initial_state,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::Distance;
    use super::Utf8DFABuilder;

    #[test]
    fn test_utf8_dfa_builder() {
        let mut dfa_builder = Utf8DFABuilder::with_max_num_states(2);
        dfa_builder.add_state(0, Distance::Exact(1u8), 1);
        dfa_builder.add_state(1, Distance::Exact(0u8), 0);
        dfa_builder.set_initial_state(1u32);
        let dfa = dfa_builder.build();
        let parity_num_letters = |s: &str| dfa.eval(s).to_u8();
        assert_eq!(parity_num_letters("abcdef"), 0u8);
        assert_eq!(parity_num_letters("a"), 1u8);
        assert_eq!(parity_num_letters("aあ"), 0u8);
        assert_eq!(parity_num_letters("❤"), 1u8);
        assert_eq!(parity_num_letters("❤❤"), 0u8);
        assert_eq!(parity_num_letters("❤a"), 0u8);
        assert_eq!(parity_num_letters("あ"), 1u8);
        assert_eq!(parity_num_letters("ああ"), 0u8);
    }
}
