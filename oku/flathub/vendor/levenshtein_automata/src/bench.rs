use super::{LevenshteinNFA, ParametricDFA};
use test::Bencher;

#[bench]
fn bench_build_dfa_distance1_no_transpose(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(1, false);
    let parametric_dfa = ParametricDFA::from_nfa(&nfa);
    b.iter(|| {
        let _dfa = parametric_dfa.build_dfa("Levenshtein", false);
    });
}

#[bench]
fn bench_build_dfa_distance2_no_transpose(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(2, false);
    let parametric_dfa = ParametricDFA::from_nfa(&nfa);
    b.iter(|| {
        let _dfa = parametric_dfa.build_dfa("Levenshtein", false);
    });
}

#[bench]
fn bench_build_dfa_distance3_no_transpose(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(3, false);
    let parametric_dfa = ParametricDFA::from_nfa(&nfa);
    b.iter(|| {
        let _dfa = parametric_dfa.build_dfa("Levenshtein", false);
    });
}

#[bench]
fn bench_build_dfa_distance4_no_transpose(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(4, false);
    let parametric_dfa = ParametricDFA::from_nfa(&nfa);
    b.iter(|| {
        let _dfa = parametric_dfa.build_dfa("Levenshtein", false);
    });
}

#[bench]
fn bench_build_dfa_distance1_with_transpose(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(1, true);
    let parametric_dfa = ParametricDFA::from_nfa(&nfa);
    b.iter(|| {
        let _dfa = parametric_dfa.build_dfa("Levenshtein", false);
    });
}

#[bench]
fn bench_build_dfa_distance2_with_transpose(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(2, true);
    let parametric_dfa = ParametricDFA::from_nfa(&nfa);
    b.iter(|| {
        let _dfa = parametric_dfa.build_dfa("Levenshtein", false);
    });
}

#[bench]
fn bench_build_dfa_distance3_with_transpose(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(3, true);
    let parametric_dfa = ParametricDFA::from_nfa(&nfa);
    b.iter(|| {
        let _dfa = parametric_dfa.build_dfa("Levenshtein", false);
    });
}

#[bench]
fn bench_build_parametricdfa_perf_1(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(1, false);
    b.iter(|| {
        let parametric_dfa = ParametricDFA::from_nfa(&nfa);
        assert_eq!(parametric_dfa.num_states(), 6);
    });
}

#[bench]
fn bench_build_parametricdfa_perf_2(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(2, false);
    b.iter(|| {
        let parametric_dfa = ParametricDFA::from_nfa(&nfa);
        assert_eq!(parametric_dfa.num_states(), 31);
    });
}

#[bench]
fn bench_build_parametricdfa_perf_3(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(3, false);
    b.iter(|| {
        let parametric_dfa = ParametricDFA::from_nfa(&nfa);
        assert_eq!(parametric_dfa.num_states(), 197);
    });
}

#[bench]
fn bench_build_parametricdfa_damerau_perf_1(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(1, true);
    b.iter(|| {
        let parametric_dfa = ParametricDFA::from_nfa(&nfa);
        // Actually 7 is sufficient.
        //
        // That's because the extra state is only reachable with chi = 11x.
        // This adds a state with the transpose flag, and an offset of 0.
        //
        // On the next extension our algorithm loses the information that
        // chi can only start by 11x.
        assert_eq!(parametric_dfa.num_states(), 8);
    });
}

#[bench]
fn bench_build_parametricdfa_damerau_perf_2_profile(b: &mut Bencher) {
    let nfa = LevenshteinNFA::levenshtein(2, true);
    let parametric_dfa = ParametricDFA::from_nfa(&nfa);
    b.iter(|| {
        let _dfa = parametric_dfa.build_dfa("Levenshtein", false);
    });
}
