//! This file was generated automatically by the Snowball to Rust compiler
//! http://snowballstem.org/

#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(unused_mut)]
use snowball::SnowballEnv;
use snowball::Among;

static A_0: &'static [Among<Context>; 37] = &[
    Among("a", -1, 1, None),
    Among("arna", 0, 1, None),
    Among("erna", 0, 1, None),
    Among("heterna", 2, 1, None),
    Among("orna", 0, 1, None),
    Among("ad", -1, 1, None),
    Among("e", -1, 1, None),
    Among("ade", 6, 1, None),
    Among("ande", 6, 1, None),
    Among("arne", 6, 1, None),
    Among("are", 6, 1, None),
    Among("aste", 6, 1, None),
    Among("en", -1, 1, None),
    Among("anden", 12, 1, None),
    Among("aren", 12, 1, None),
    Among("heten", 12, 1, None),
    Among("ern", -1, 1, None),
    Among("ar", -1, 1, None),
    Among("er", -1, 1, None),
    Among("heter", 18, 1, None),
    Among("or", -1, 1, None),
    Among("s", -1, 2, None),
    Among("as", 21, 1, None),
    Among("arnas", 22, 1, None),
    Among("ernas", 22, 1, None),
    Among("ornas", 22, 1, None),
    Among("es", 21, 1, None),
    Among("ades", 26, 1, None),
    Among("andes", 26, 1, None),
    Among("ens", 21, 1, None),
    Among("arens", 29, 1, None),
    Among("hetens", 29, 1, None),
    Among("erns", 21, 1, None),
    Among("at", -1, 1, None),
    Among("andet", -1, 1, None),
    Among("het", -1, 1, None),
    Among("ast", -1, 1, None),
];

static A_1: &'static [Among<Context>; 7] = &[
    Among("dd", -1, -1, None),
    Among("gd", -1, -1, None),
    Among("nn", -1, -1, None),
    Among("dt", -1, -1, None),
    Among("gt", -1, -1, None),
    Among("kt", -1, -1, None),
    Among("tt", -1, -1, None),
];

static A_2: &'static [Among<Context>; 5] = &[
    Among("ig", -1, 1, None),
    Among("lig", 0, 1, None),
    Among("els", -1, 1, None),
    Among("fullt", -1, 3, None),
    Among("l\u{00F6}st", -1, 2, None),
];

static G_v: &'static [u8; 19] = &[17, 65, 16, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 24, 0, 32];

static G_s_ending: &'static [u8; 3] = &[119, 127, 149];

#[derive(Clone)]
struct Context {
    i_x: usize,
    i_p1: usize,
}

fn r_mark_regions(env: &mut SnowballEnv, context: &mut Context) -> bool {
    // (, line 26
    context.i_p1 = env.limit;
    // test, line 29
    let v_1 = env.cursor;
    // (, line 29
    // hop, line 29
    let c = env.byte_index_for_hop(3);
    if 0 as i32 > c || c > env.limit as i32 {
        return false;
    }
    env.cursor = c as usize;
    // setmark x, line 29
    context.i_x = env.cursor;
    env.cursor = v_1;
    // goto, line 30
    'golab0: loop {
        let v_2 = env.cursor;
        'lab1: loop {
            if !env.in_grouping(G_v, 97, 246) {
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
    // gopast, line 30
    'golab2: loop {
        'lab3: loop {
            if !env.out_grouping(G_v, 97, 246) {
                break 'lab3;
            }
            break 'golab2;
        }
        if env.cursor >= env.limit {
            return false;
        }
        env.next_char();
    }
    // setmark p1, line 30
    context.i_p1 = env.cursor;
    // try, line 31
    'lab4: loop {
        // (, line 31
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
    // (, line 36
    // setlimit, line 37
    let v_1 = env.limit - env.cursor;
    // tomark, line 37
    if env.cursor < context.i_p1 {
        return false;
    }
    env.cursor = context.i_p1;
    let v_2 = env.limit_backward;
    env.limit_backward = env.cursor;
    env.cursor = env.limit - v_1;
    // (, line 37
    // [, line 37
    env.ket = env.cursor;
    // substring, line 37
    among_var = env.find_among_b(A_0, context);
    if among_var == 0 {
        env.limit_backward = v_2;
        return false;
    }
    // ], line 37
    env.bra = env.cursor;
    env.limit_backward = v_2;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 44
        // delete, line 44
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 2 {
        // (, line 46
        if !env.in_grouping_b(G_s_ending, 98, 121) {
            return false;
        }
        // delete, line 46
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_consonant_pair(env: &mut SnowballEnv, context: &mut Context) -> bool {
    // setlimit, line 50
    let v_1 = env.limit - env.cursor;
    // tomark, line 50
    if env.cursor < context.i_p1 {
        return false;
    }
    env.cursor = context.i_p1;
    let v_2 = env.limit_backward;
    env.limit_backward = env.cursor;
    env.cursor = env.limit - v_1;
    // (, line 50
    // and, line 52
    let v_3 = env.limit - env.cursor;
    // among, line 51
    if env.find_among_b(A_1, context) == 0 {
        env.limit_backward = v_2;
        return false;
    }
    env.cursor = env.limit - v_3;
    // (, line 52
    // [, line 52
    env.ket = env.cursor;
    // next, line 52
    if env.cursor <= env.limit_backward {
        env.limit_backward = v_2;
        return false;
    }
    env.previous_char();
    // ], line 52
    env.bra = env.cursor;
    // delete, line 52
    if !env.slice_del() {
        return false;
    }
    env.limit_backward = v_2;
    return true;
}

fn r_other_suffix(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // setlimit, line 55
    let v_1 = env.limit - env.cursor;
    // tomark, line 55
    if env.cursor < context.i_p1 {
        return false;
    }
    env.cursor = context.i_p1;
    let v_2 = env.limit_backward;
    env.limit_backward = env.cursor;
    env.cursor = env.limit - v_1;
    // (, line 55
    // [, line 56
    env.ket = env.cursor;
    // substring, line 56
    among_var = env.find_among_b(A_2, context);
    if among_var == 0 {
        env.limit_backward = v_2;
        return false;
    }
    // ], line 56
    env.bra = env.cursor;
    if among_var == 0 {
        env.limit_backward = v_2;
        return false;
    } else if among_var == 1 {
        // (, line 57
        // delete, line 57
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 2 {
        // (, line 58
        // <-, line 58
        if !env.slice_from("l\u{00F6}s") {
            return false;
        }
    } else if among_var == 3 {
        // (, line 59
        // <-, line 59
        if !env.slice_from("full") {
            return false;
        }
    }
    env.limit_backward = v_2;
    return true;
}

pub fn stem(env: &mut SnowballEnv) -> bool {
    let mut context = &mut Context {
        i_x: 0,
        i_p1: 0,
    };
    // (, line 64
    // do, line 66
    let v_1 = env.cursor;
    'lab0: loop {
        // call mark_regions, line 66
        if !r_mark_regions(env, context) {
            break 'lab0;
        }
        break 'lab0;
    }
    env.cursor = v_1;
    // backwards, line 67
    env.limit_backward = env.cursor;
    env.cursor = env.limit;
    // (, line 67
    // do, line 68
    let v_2 = env.limit - env.cursor;
    'lab1: loop {
        // call main_suffix, line 68
        if !r_main_suffix(env, context) {
            break 'lab1;
        }
        break 'lab1;
    }
    env.cursor = env.limit - v_2;
    // do, line 69
    let v_3 = env.limit - env.cursor;
    'lab2: loop {
        // call consonant_pair, line 69
        if !r_consonant_pair(env, context) {
            break 'lab2;
        }
        break 'lab2;
    }
    env.cursor = env.limit - v_3;
    // do, line 70
    let v_4 = env.limit - env.cursor;
    'lab3: loop {
        // call other_suffix, line 70
        if !r_other_suffix(env, context) {
            break 'lab3;
        }
        break 'lab3;
    }
    env.cursor = env.limit - v_4;
    env.cursor = env.limit_backward;
    return true;
}
