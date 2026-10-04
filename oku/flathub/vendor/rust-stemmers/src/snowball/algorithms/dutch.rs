//! This file was generated automatically by the Snowball to Rust compiler
//! http://snowballstem.org/

#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(unused_mut)]
use snowball::SnowballEnv;
use snowball::Among;

static A_0: &'static [Among<Context>; 11] = &[
    Among("", -1, 6, None),
    Among("\u{00E1}", 0, 1, None),
    Among("\u{00E4}", 0, 1, None),
    Among("\u{00E9}", 0, 2, None),
    Among("\u{00EB}", 0, 2, None),
    Among("\u{00ED}", 0, 3, None),
    Among("\u{00EF}", 0, 3, None),
    Among("\u{00F3}", 0, 4, None),
    Among("\u{00F6}", 0, 4, None),
    Among("\u{00FA}", 0, 5, None),
    Among("\u{00FC}", 0, 5, None),
];

static A_1: &'static [Among<Context>; 3] = &[
    Among("", -1, 3, None),
    Among("I", 0, 2, None),
    Among("Y", 0, 1, None),
];

static A_2: &'static [Among<Context>; 3] = &[
    Among("dd", -1, -1, None),
    Among("kk", -1, -1, None),
    Among("tt", -1, -1, None),
];

static A_3: &'static [Among<Context>; 5] = &[
    Among("ene", -1, 2, None),
    Among("se", -1, 3, None),
    Among("en", -1, 2, None),
    Among("heden", 2, 1, None),
    Among("s", -1, 3, None),
];

static A_4: &'static [Among<Context>; 6] = &[
    Among("end", -1, 1, None),
    Among("ig", -1, 2, None),
    Among("ing", -1, 1, None),
    Among("lijk", -1, 3, None),
    Among("baar", -1, 4, None),
    Among("bar", -1, 5, None),
];

static A_5: &'static [Among<Context>; 4] = &[
    Among("aa", -1, -1, None),
    Among("ee", -1, -1, None),
    Among("oo", -1, -1, None),
    Among("uu", -1, -1, None),
];

static G_v: &'static [u8; 17] = &[17, 65, 16, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128];

static G_v_I: &'static [u8; 20] = &[1, 0, 0, 17, 65, 16, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128];

static G_v_j: &'static [u8; 17] = &[17, 67, 16, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128];

#[derive(Clone)]
struct Context {
    i_p2: usize,
    i_p1: usize,
    b_e_found: bool,
}

fn r_prelude(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 41
    // test, line 42
    let v_1 = env.cursor;
    // repeat, line 42
    'replab0: loop{
        let v_2 = env.cursor;
        'lab1: for _ in 0..1 {
            // (, line 42
            // [, line 43
            env.bra = env.cursor;
            // substring, line 43
            among_var = env.find_among(A_0, context);
            if among_var == 0 {
                break 'lab1;
            }
            // ], line 43
            env.ket = env.cursor;
            if among_var == 0 {
                break 'lab1;
            } else if among_var == 1 {
                // (, line 45
                // <-, line 45
                if !env.slice_from("a") {
                    return false;
                }
            } else if among_var == 2 {
                // (, line 47
                // <-, line 47
                if !env.slice_from("e") {
                    return false;
                }
            } else if among_var == 3 {
                // (, line 49
                // <-, line 49
                if !env.slice_from("i") {
                    return false;
                }
            } else if among_var == 4 {
                // (, line 51
                // <-, line 51
                if !env.slice_from("o") {
                    return false;
                }
            } else if among_var == 5 {
                // (, line 53
                // <-, line 53
                if !env.slice_from("u") {
                    return false;
                }
            } else if among_var == 6 {
                // (, line 54
                // next, line 54
                if env.cursor >= env.limit {
                    break 'lab1;
                }
                env.next_char();
            }
            continue 'replab0;
        }
        env.cursor = v_2;
        break 'replab0;
    }
    env.cursor = v_1;
    // try, line 57
    let v_3 = env.cursor;
    'lab2: loop {
        // (, line 57
        // [, line 57
        env.bra = env.cursor;
        // literal, line 57
        if !env.eq_s(&"y") {
            env.cursor = v_3;
            break 'lab2;
        }
        // ], line 57
        env.ket = env.cursor;
        // <-, line 57
        if !env.slice_from("Y") {
            return false;
        }
        break 'lab2;
    }
    // repeat, line 58
    'replab3: loop{
        let v_4 = env.cursor;
        'lab4: for _ in 0..1 {
            // goto, line 58
            'golab5: loop {
                let v_5 = env.cursor;
                'lab6: loop {
                    // (, line 58
                    if !env.in_grouping(G_v, 97, 232) {
                        break 'lab6;
                    }
                    // [, line 59
                    env.bra = env.cursor;
                    // or, line 59
                    'lab7: loop {
                        let v_6 = env.cursor;
                        'lab8: loop {
                            // (, line 59
                            // literal, line 59
                            if !env.eq_s(&"i") {
                                break 'lab8;
                            }
                            // ], line 59
                            env.ket = env.cursor;
                            if !env.in_grouping(G_v, 97, 232) {
                                break 'lab8;
                            }
                            // <-, line 59
                            if !env.slice_from("I") {
                                return false;
                            }
                            break 'lab7;
                        }
                        env.cursor = v_6;
                        // (, line 60
                        // literal, line 60
                        if !env.eq_s(&"y") {
                            break 'lab6;
                        }
                        // ], line 60
                        env.ket = env.cursor;
                        // <-, line 60
                        if !env.slice_from("Y") {
                            return false;
                        }
                        break 'lab7;
                    }
                    env.cursor = v_5;
                    break 'golab5;
                }
                env.cursor = v_5;
                if env.cursor >= env.limit {
                    break 'lab4;
                }
                env.next_char();
            }
            continue 'replab3;
        }
        env.cursor = v_4;
        break 'replab3;
    }
    return true;
}

fn r_mark_regions(env: &mut SnowballEnv, context: &mut Context) -> bool {
    // (, line 64
    context.i_p1 = env.limit;
    context.i_p2 = env.limit;
    // gopast, line 69
    'golab0: loop {
        'lab1: loop {
            if !env.in_grouping(G_v, 97, 232) {
                break 'lab1;
            }
            break 'golab0;
        }
        if env.cursor >= env.limit {
            return false;
        }
        env.next_char();
    }
    // gopast, line 69
    'golab2: loop {
        'lab3: loop {
            if !env.out_grouping(G_v, 97, 232) {
                break 'lab3;
            }
            break 'golab2;
        }
        if env.cursor >= env.limit {
            return false;
        }
        env.next_char();
    }
    // setmark p1, line 69
    context.i_p1 = env.cursor;
    // try, line 70
    'lab4: loop {
        // (, line 70
        if !(context.i_p1 < 3){
            break 'lab4;
        }
        context.i_p1 = 3;
        break 'lab4;
    }
    // gopast, line 71
    'golab5: loop {
        'lab6: loop {
            if !env.in_grouping(G_v, 97, 232) {
                break 'lab6;
            }
            break 'golab5;
        }
        if env.cursor >= env.limit {
            return false;
        }
        env.next_char();
    }
    // gopast, line 71
    'golab7: loop {
        'lab8: loop {
            if !env.out_grouping(G_v, 97, 232) {
                break 'lab8;
            }
            break 'golab7;
        }
        if env.cursor >= env.limit {
            return false;
        }
        env.next_char();
    }
    // setmark p2, line 71
    context.i_p2 = env.cursor;
    return true;
}

fn r_postlude(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // repeat, line 75
    'replab0: loop{
        let v_1 = env.cursor;
        'lab1: for _ in 0..1 {
            // (, line 75
            // [, line 77
            env.bra = env.cursor;
            // substring, line 77
            among_var = env.find_among(A_1, context);
            if among_var == 0 {
                break 'lab1;
            }
            // ], line 77
            env.ket = env.cursor;
            if among_var == 0 {
                break 'lab1;
            } else if among_var == 1 {
                // (, line 78
                // <-, line 78
                if !env.slice_from("y") {
                    return false;
                }
            } else if among_var == 2 {
                // (, line 79
                // <-, line 79
                if !env.slice_from("i") {
                    return false;
                }
            } else if among_var == 3 {
                // (, line 80
                // next, line 80
                if env.cursor >= env.limit {
                    break 'lab1;
                }
                env.next_char();
            }
            continue 'replab0;
        }
        env.cursor = v_1;
        break 'replab0;
    }
    return true;
}

fn r_R1(env: &mut SnowballEnv, context: &mut Context) -> bool {
    if !(context.i_p1 <= env.cursor){
        return false;
    }
    return true;
}

fn r_R2(env: &mut SnowballEnv, context: &mut Context) -> bool {
    if !(context.i_p2 <= env.cursor){
        return false;
    }
    return true;
}

fn r_undouble(env: &mut SnowballEnv, context: &mut Context) -> bool {
    // (, line 90
    // test, line 91
    let v_1 = env.limit - env.cursor;
    // among, line 91
    if env.find_among_b(A_2, context) == 0 {
        return false;
    }
    env.cursor = env.limit - v_1;
    // [, line 91
    env.ket = env.cursor;
    // next, line 91
    if env.cursor <= env.limit_backward {
        return false;
    }
    env.previous_char();
    // ], line 91
    env.bra = env.cursor;
    // delete, line 91
    if !env.slice_del() {
        return false;
    }
    return true;
}

fn r_e_ending(env: &mut SnowballEnv, context: &mut Context) -> bool {
    // (, line 94
    // unset e_found, line 95
    context.b_e_found = false;
    // [, line 96
    env.ket = env.cursor;
    // literal, line 96
    if !env.eq_s_b(&"e") {
        return false;
    }
    // ], line 96
    env.bra = env.cursor;
    // call R1, line 96
    if !r_R1(env, context) {
        return false;
    }
    // test, line 96
    let v_1 = env.limit - env.cursor;
    if !env.out_grouping_b(G_v, 97, 232) {
        return false;
    }
    env.cursor = env.limit - v_1;
    // delete, line 96
    if !env.slice_del() {
        return false;
    }
    // set e_found, line 97
    context.b_e_found = true;
    // call undouble, line 98
    if !r_undouble(env, context) {
        return false;
    }
    return true;
}

fn r_en_ending(env: &mut SnowballEnv, context: &mut Context) -> bool {
    // (, line 101
    // call R1, line 102
    if !r_R1(env, context) {
        return false;
    }
    // and, line 102
    let v_1 = env.limit - env.cursor;
    if !env.out_grouping_b(G_v, 97, 232) {
        return false;
    }
    env.cursor = env.limit - v_1;
    // not, line 102
    let v_2 = env.limit - env.cursor;
    'lab0: loop {
        // literal, line 102
        if !env.eq_s_b(&"gem") {
            break 'lab0;
        }
        return false;
    }
    env.cursor = env.limit - v_2;
    // delete, line 102
    if !env.slice_del() {
        return false;
    }
    // call undouble, line 103
    if !r_undouble(env, context) {
        return false;
    }
    return true;
}

fn r_standard_suffix(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 106
    // do, line 107
    let v_1 = env.limit - env.cursor;
    'lab0: loop {
        // (, line 107
        // [, line 108
        env.ket = env.cursor;
        // substring, line 108
        among_var = env.find_among_b(A_3, context);
        if among_var == 0 {
            break 'lab0;
        }
        // ], line 108
        env.bra = env.cursor;
        if among_var == 0 {
            break 'lab0;
        } else if among_var == 1 {
            // (, line 110
            // call R1, line 110
            if !r_R1(env, context) {
                break 'lab0;
            }
            // <-, line 110
            if !env.slice_from("heid") {
                return false;
            }
        } else if among_var == 2 {
            // (, line 113
            // call en_ending, line 113
            if !r_en_ending(env, context) {
                break 'lab0;
            }
        } else if among_var == 3 {
            // (, line 116
            // call R1, line 116
            if !r_R1(env, context) {
                break 'lab0;
            }
            if !env.out_grouping_b(G_v_j, 97, 232) {
                break 'lab0;
            }
            // delete, line 116
            if !env.slice_del() {
                return false;
            }
        }
        break 'lab0;
    }
    env.cursor = env.limit - v_1;
    // do, line 120
    let v_2 = env.limit - env.cursor;
    'lab1: loop {
        // call e_ending, line 120
        if !r_e_ending(env, context) {
            break 'lab1;
        }
        break 'lab1;
    }
    env.cursor = env.limit - v_2;
    // do, line 122
    let v_3 = env.limit - env.cursor;
    'lab2: loop {
        // (, line 122
        // [, line 122
        env.ket = env.cursor;
        // literal, line 122
        if !env.eq_s_b(&"heid") {
            break 'lab2;
        }
        // ], line 122
        env.bra = env.cursor;
        // call R2, line 122
        if !r_R2(env, context) {
            break 'lab2;
        }
        // not, line 122
        let v_4 = env.limit - env.cursor;
        'lab3: loop {
            // literal, line 122
            if !env.eq_s_b(&"c") {
                break 'lab3;
            }
            break 'lab2;
        }
        env.cursor = env.limit - v_4;
        // delete, line 122
        if !env.slice_del() {
            return false;
        }
        // [, line 123
        env.ket = env.cursor;
        // literal, line 123
        if !env.eq_s_b(&"en") {
            break 'lab2;
        }
        // ], line 123
        env.bra = env.cursor;
        // call en_ending, line 123
        if !r_en_ending(env, context) {
            break 'lab2;
        }
        break 'lab2;
    }
    env.cursor = env.limit - v_3;
    // do, line 126
    let v_5 = env.limit - env.cursor;
    'lab4: loop {
        // (, line 126
        // [, line 127
        env.ket = env.cursor;
        // substring, line 127
        among_var = env.find_among_b(A_4, context);
        if among_var == 0 {
            break 'lab4;
        }
        // ], line 127
        env.bra = env.cursor;
        if among_var == 0 {
            break 'lab4;
        } else if among_var == 1 {
            // (, line 129
            // call R2, line 129
            if !r_R2(env, context) {
                break 'lab4;
            }
            // delete, line 129
            if !env.slice_del() {
                return false;
            }
            // or, line 130
            'lab5: loop {
                let v_6 = env.limit - env.cursor;
                'lab6: loop {
                    // (, line 130
                    // [, line 130
                    env.ket = env.cursor;
                    // literal, line 130
                    if !env.eq_s_b(&"ig") {
                        break 'lab6;
                    }
                    // ], line 130
                    env.bra = env.cursor;
                    // call R2, line 130
                    if !r_R2(env, context) {
                        break 'lab6;
                    }
                    // not, line 130
                    let v_7 = env.limit - env.cursor;
                    'lab7: loop {
                        // literal, line 130
                        if !env.eq_s_b(&"e") {
                            break 'lab7;
                        }
                        break 'lab6;
                    }
                    env.cursor = env.limit - v_7;
                    // delete, line 130
                    if !env.slice_del() {
                        return false;
                    }
                    break 'lab5;
                }
                env.cursor = env.limit - v_6;
                // call undouble, line 130
                if !r_undouble(env, context) {
                    break 'lab4;
                }
                break 'lab5;
            }
        } else if among_var == 2 {
            // (, line 133
            // call R2, line 133
            if !r_R2(env, context) {
                break 'lab4;
            }
            // not, line 133
            let v_8 = env.limit - env.cursor;
            'lab8: loop {
                // literal, line 133
                if !env.eq_s_b(&"e") {
                    break 'lab8;
                }
                break 'lab4;
            }
            env.cursor = env.limit - v_8;
            // delete, line 133
            if !env.slice_del() {
                return false;
            }
        } else if among_var == 3 {
            // (, line 136
            // call R2, line 136
            if !r_R2(env, context) {
                break 'lab4;
            }
            // delete, line 136
            if !env.slice_del() {
                return false;
            }
            // call e_ending, line 136
            if !r_e_ending(env, context) {
                break 'lab4;
            }
        } else if among_var == 4 {
            // (, line 139
            // call R2, line 139
            if !r_R2(env, context) {
                break 'lab4;
            }
            // delete, line 139
            if !env.slice_del() {
                return false;
            }
        } else if among_var == 5 {
            // (, line 142
            // call R2, line 142
            if !r_R2(env, context) {
                break 'lab4;
            }
            // Boolean test e_found, line 142
            if !context.b_e_found {
                break 'lab4;
            }
            // delete, line 142
            if !env.slice_del() {
                return false;
            }
        }
        break 'lab4;
    }
    env.cursor = env.limit - v_5;
    // do, line 146
    let v_9 = env.limit - env.cursor;
    'lab9: loop {
        // (, line 146
        if !env.out_grouping_b(G_v_I, 73, 232) {
            break 'lab9;
        }
        // test, line 148
        let v_10 = env.limit - env.cursor;
        // (, line 148
        // among, line 149
        if env.find_among_b(A_5, context) == 0 {
            break 'lab9;
        }
        if !env.out_grouping_b(G_v, 97, 232) {
            break 'lab9;
        }
        env.cursor = env.limit - v_10;
        // [, line 152
        env.ket = env.cursor;
        // next, line 152
        if env.cursor <= env.limit_backward {
            break 'lab9;
        }
        env.previous_char();
        // ], line 152
        env.bra = env.cursor;
        // delete, line 152
        if !env.slice_del() {
            return false;
        }
        break 'lab9;
    }
    env.cursor = env.limit - v_9;
    return true;
}

pub fn stem(env: &mut SnowballEnv) -> bool {
    let mut context = &mut Context {
        i_p2: 0,
        i_p1: 0,
        b_e_found: false,
    };
    // (, line 157
    // do, line 159
    let v_1 = env.cursor;
    'lab0: loop {
        // call prelude, line 159
        if !r_prelude(env, context) {
            break 'lab0;
        }
        break 'lab0;
    }
    env.cursor = v_1;
    // do, line 160
    let v_2 = env.cursor;
    'lab1: loop {
        // call mark_regions, line 160
        if !r_mark_regions(env, context) {
            break 'lab1;
        }
        break 'lab1;
    }
    env.cursor = v_2;
    // backwards, line 161
    env.limit_backward = env.cursor;
    env.cursor = env.limit;
    // do, line 162
    let v_3 = env.limit - env.cursor;
    'lab2: loop {
        // call standard_suffix, line 162
        if !r_standard_suffix(env, context) {
            break 'lab2;
        }
        break 'lab2;
    }
    env.cursor = env.limit - v_3;
    env.cursor = env.limit_backward;
    // do, line 163
    let v_4 = env.cursor;
    'lab3: loop {
        // call postlude, line 163
        if !r_postlude(env, context) {
            break 'lab3;
        }
        break 'lab3;
    }
    env.cursor = v_4;
    return true;
}
