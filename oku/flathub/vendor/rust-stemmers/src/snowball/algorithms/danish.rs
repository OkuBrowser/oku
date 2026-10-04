//! This file was generated automatically by the Snowball to Rust compiler
//! http://snowballstem.org/

#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(unused_mut)]
use snowball::SnowballEnv;
use snowball::Among;

static A_0: &'static [Among<Context>; 32] = &[
    Among("hed", -1, 1, None),
    Among("ethed", 0, 1, None),
    Among("ered", -1, 1, None),
    Among("e", -1, 1, None),
    Among("erede", 3, 1, None),
    Among("ende", 3, 1, None),
    Among("erende", 5, 1, None),
    Among("ene", 3, 1, None),
    Among("erne", 3, 1, None),
    Among("ere", 3, 1, None),
    Among("en", -1, 1, None),
    Among("heden", 10, 1, None),
    Among("eren", 10, 1, None),
    Among("er", -1, 1, None),
    Among("heder", 13, 1, None),
    Among("erer", 13, 1, None),
    Among("s", -1, 2, None),
    Among("heds", 16, 1, None),
    Among("es", 16, 1, None),
    Among("endes", 18, 1, None),
    Among("erendes", 19, 1, None),
    Among("enes", 18, 1, None),
    Among("ernes", 18, 1, None),
    Among("eres", 18, 1, None),
    Among("ens", 16, 1, None),
    Among("hedens", 24, 1, None),
    Among("erens", 24, 1, None),
    Among("ers", 16, 1, None),
    Among("ets", 16, 1, None),
    Among("erets", 28, 1, None),
    Among("et", -1, 1, None),
    Among("eret", 30, 1, None),
];

static A_1: &'static [Among<Context>; 4] = &[
    Among("gd", -1, -1, None),
    Among("dt", -1, -1, None),
    Among("gt", -1, -1, None),
    Among("kt", -1, -1, None),
];

static A_2: &'static [Among<Context>; 5] = &[
    Among("ig", -1, 1, None),
    Among("lig", 0, 1, None),
    Among("elig", 1, 1, None),
    Among("els", -1, 1, None),
    Among("l\u{00F8}st", -1, 2, None),
];

static G_v: &'static [u8; 19] = &[17, 65, 16, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 48, 0, 128];

static G_s_ending: &'static [u8; 17] = &[239, 254, 42, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16];

#[derive(Clone)]
struct Context {
    i_x: usize,
    i_p1: usize,
    S_ch: String,
}

fn r_mark_regions(env: &mut SnowballEnv, context: &mut Context) -> bool {
    // (, line 29
    context.i_p1 = env.limit;
    // test, line 33
    let v_1 = env.cursor;
    // (, line 33
    // hop, line 33
    let c = env.byte_index_for_hop(3);
    if 0 as i32 > c || c > env.limit as i32 {
        return false;
    }
    env.cursor = c as usize;
    // setmark x, line 33
    context.i_x = env.cursor;
    env.cursor = v_1;
    // goto, line 34
    'golab0: loop {
        let v_2 = env.cursor;
        'lab1: loop {
            if !env.in_grouping(G_v, 97, 248) {
                break 'lab1;
            }
            env.cursor = v_2;
            break 'golab0;
        }
        env.cursor = v_2;
        if env.cursor >= env.limit {
            return false;
        }
        env.next_char();
    }
    // gopast, line 34
    'golab2: loop {
        'lab3: loop {
            if !env.out_grouping(G_v, 97, 248) {
                break 'lab3;
            }
            break 'golab2;
        }
        if env.cursor >= env.limit {
            return false;
        }
        env.next_char();
    }
    // setmark p1, line 34
    context.i_p1 = env.cursor;
    // try, line 35
    'lab4: loop {
        // (, line 35
        if !(context.i_p1 < context.i_x){
            break 'lab4;
        }
        context.i_p1 = context.i_x;
        break 'lab4;
    }
    return true;
}

fn r_main_suffix(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 40
    // setlimit, line 41
    let v_1 = env.limit - env.cursor;
    // tomark, line 41
    if env.cursor < context.i_p1 {
        return false;
    }
    env.cursor = context.i_p1;
    let v_2 = env.limit_backward;
    env.limit_backward = env.cursor;
    env.cursor = env.limit - v_1;
    // (, line 41
    // [, line 41
    env.ket = env.cursor;
    // substring, line 41
    among_var = env.find_among_b(A_0, context);
    if among_var == 0 {
        env.limit_backward = v_2;
        return false;
    }
    // ], line 41
    env.bra = env.cursor;
    env.limit_backward = v_2;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 48
        // delete, line 48
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 2 {
        // (, line 50
        if !env.in_grouping_b(G_s_ending, 97, 229) {
            return false;
        }
        // delete, line 50
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_consonant_pair(env: &mut SnowballEnv, context: &mut Context) -> bool {
    // (, line 54
    // test, line 55
    let v_1 = env.limit - env.cursor;
    // (, line 55
    // setlimit, line 56
    let v_2 = env.limit - env.cursor;
    // tomark, line 56
    if env.cursor < context.i_p1 {
        return false;
    }
    env.cursor = context.i_p1;
    let v_3 = env.limit_backward;
    env.limit_backward = env.cursor;
    env.cursor = env.limit - v_2;
    // (, line 56
    // [, line 56
    env.ket = env.cursor;
    // substring, line 56
    if env.find_among_b(A_1, context) == 0 {
        env.limit_backward = v_3;
        return false;
    }
    // ], line 56
    env.bra = env.cursor;
    env.limit_backward = v_3;
    env.cursor = env.limit - v_1;
    // next, line 62
    if env.cursor <= env.limit_backward {
        return false;
    }
    env.previous_char();
    // ], line 62
    env.bra = env.cursor;
    // delete, line 62
    if !env.slice_del() {
        return false;
    }
    return true;
}

fn r_other_suffix(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 65
    // do, line 66
    let v_1 = env.limit - env.cursor;
    'lab0: loop {
        // (, line 66
        // [, line 66
        env.ket = env.cursor;
        // literal, line 66
        if !env.eq_s_b(&"st") {
            break 'lab0;
        }
        // ], line 66
        env.bra = env.cursor;
        // literal, line 66
        if !env.eq_s_b(&"ig") {
            break 'lab0;
        }
        // delete, line 66
        if !env.slice_del() {
            return false;
        }
        break 'lab0;
    }
    env.cursor = env.limit - v_1;
    // setlimit, line 67
    let v_2 = env.limit - env.cursor;
    // tomark, line 67
    if env.cursor < context.i_p1 {
        return false;
    }
    env.cursor = context.i_p1;
    let v_3 = env.limit_backward;
    env.limit_backward = env.cursor;
    env.cursor = env.limit - v_2;
    // (, line 67
    // [, line 67
    env.ket = env.cursor;
    // substring, line 67
    among_var = env.find_among_b(A_2, context);
    if among_var == 0 {
        env.limit_backward = v_3;
        return false;
    }
    // ], line 67
    env.bra = env.cursor;
    env.limit_backward = v_3;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 70
        // delete, line 70
        if !env.slice_del() {
            return false;
        }
        // do, line 70
        let v_4 = env.limit - env.cursor;
        'lab1: loop {
            // call consonant_pair, line 70
            if !r_consonant_pair(env, context) {
                break 'lab1;
            }
            break 'lab1;
        }
        env.cursor = env.limit - v_4;
    } else if among_var == 2 {
        // (, line 72
        // <-, line 72
        if !env.slice_from("l\u{00F8}s") {
            return false;
        }
    }
    return true;
}

fn r_undouble(env: &mut SnowballEnv, context: &mut Context) -> bool {
    // (, line 75
    // setlimit, line 76
    let v_1 = env.limit - env.cursor;
    // tomark, line 76
    if env.cursor < context.i_p1 {
        return false;
    }
    env.cursor = context.i_p1;
    let v_2 = env.limit_backward;
    env.limit_backward = env.cursor;
    env.cursor = env.limit - v_1;
    // (, line 76
    // [, line 76
    env.ket = env.cursor;
    if !env.out_grouping_b(G_v, 97, 248) {
        env.limit_backward = v_2;
        return false;
    }
    // ], line 76
    env.bra = env.cursor;
    // -> ch, line 76
    context.S_ch = env.slice_to();
    if context.S_ch.is_empty() {
        return false;
    }
    env.limit_backward = v_2;
    // name ch, line 77
    if !env.eq_s_b(&context.S_ch) {
        return false;
    }
    // delete, line 78
    if !env.slice_del() {
        return false;
    }
    return true;
}

pub fn stem(env: &mut SnowballEnv) -> bool {
    let mut context = &mut Context {
        i_x: 0,
        i_p1: 0,
        S_ch: String::new(),
    };
    // (, line 82
    // do, line 84
    let v_1 = env.cursor;
    'lab0: loop {
        // call mark_regions, line 84
        if !r_mark_regions(env, context) {
            break 'lab0;
        }
        break 'lab0;
    }
    env.cursor = v_1;
    // backwards, line 85
    env.limit_backward = env.cursor;
    env.cursor = env.limit;
    // (, line 85
    // do, line 86
    let v_2 = env.limit - env.cursor;
    'lab1: loop {
        // call main_suffix, line 86
        if !r_main_suffix(env, context) {
            break 'lab1;
        }
        break 'lab1;
    }
    env.cursor = env.limit - v_2;
    // do, line 87
    let v_3 = env.limit - env.cursor;
    'lab2: loop {
        // call consonant_pair, line 87
        if !r_consonant_pair(env, context) {
            break 'lab2;
        }
        break 'lab2;
    }
    env.cursor = env.limit - v_3;
    // do, line 88
    let v_4 = env.limit - env.cursor;
    'lab3: loop {
        // call other_suffix, line 88
        if !r_other_suffix(env, context) {
            break 'lab3;
        }
        break 'lab3;
    }
    env.cursor = env.limit - v_4;
    // do, line 89
    let v_5 = env.limit - env.cursor;
    'lab4: loop {
        // call undouble, line 89
        if !r_undouble(env, context) {
            break 'lab4;
        }
        break 'lab4;
    }
    env.cursor = env.limit - v_5;
    env.cursor = env.limit_backward;
    return true;
}
