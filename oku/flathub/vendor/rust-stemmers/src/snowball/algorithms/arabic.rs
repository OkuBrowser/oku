//! This file was generated automatically by the Snowball to Rust compiler
//! http://snowballstem.org/

#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(unused_mut)]
use snowball::SnowballEnv;
use snowball::Among;

static A_0: &'static [Among<Context>; 161] = &[
    Among("!", -1, 3, None),
    Among("\"", -1, 3, None),
    Among("%", -1, 3, None),
    Among("*", -1, 3, None),
    Among(",", -1, 3, None),
    Among(".", -1, 3, None),
    Among("/", -1, 3, None),
    Among(":", -1, 3, None),
    Among(";", -1, 3, None),
    Among("?", -1, 3, None),
    Among("\\", -1, 3, None),
    Among("\u{060C}", -1, 4, None),
    Among("\u{061B}", -1, 4, None),
    Among("\u{061F}", -1, 4, None),
    Among("\u{0640}", -1, 2, None),
    Among("\u{064B}", -1, 1, None),
    Among("\u{064C}", -1, 1, None),
    Among("\u{064D}", -1, 1, None),
    Among("\u{064E}", -1, 1, None),
    Among("\u{064F}", -1, 1, None),
    Among("\u{0650}", -1, 1, None),
    Among("\u{0651}", -1, 1, None),
    Among("\u{0652}", -1, 1, None),
    Among("\u{0660}", -1, 5, None),
    Among("\u{0661}", -1, 6, None),
    Among("\u{0662}", -1, 7, None),
    Among("\u{0663}", -1, 8, None),
    Among("\u{0664}", -1, 9, None),
    Among("\u{0665}", -1, 10, None),
    Among("\u{0666}", -1, 11, None),
    Among("\u{0667}", -1, 12, None),
    Among("\u{0668}", -1, 13, None),
    Among("\u{0669}", -1, 14, None),
    Among("\u{066A}", -1, 15, None),
    Among("\u{066B}", -1, 15, None),
    Among("\u{066C}", -1, 15, None),
    Among("\u{FE80}", -1, 16, None),
    Among("\u{FE81}", -1, 20, None),
    Among("\u{FE82}", -1, 20, None),
    Among("\u{FE83}", -1, 17, None),
    Among("\u{FE84}", -1, 17, None),
    Among("\u{FE85}", -1, 21, None),
    Among("\u{FE86}", -1, 21, None),
    Among("\u{FE87}", -1, 18, None),
    Among("\u{FE88}", -1, 18, None),
    Among("\u{FE89}", -1, 19, None),
    Among("\u{FE8A}", -1, 19, None),
    Among("\u{FE8B}", -1, 19, None),
    Among("\u{FE8C}", -1, 19, None),
    Among("\u{FE8D}", -1, 22, None),
    Among("\u{FE8E}", -1, 22, None),
    Among("\u{FE8F}", -1, 23, None),
    Among("\u{FE90}", -1, 23, None),
    Among("\u{FE91}", -1, 23, None),
    Among("\u{FE92}", -1, 23, None),
    Among("\u{FE93}", -1, 24, None),
    Among("\u{FE94}", -1, 24, None),
    Among("\u{FE95}", -1, 25, None),
    Among("\u{FE96}", -1, 25, None),
    Among("\u{FE97}", -1, 25, None),
    Among("\u{FE98}", -1, 25, None),
    Among("\u{FE99}", -1, 26, None),
    Among("\u{FE9A}", -1, 26, None),
    Among("\u{FE9B}", -1, 26, None),
    Among("\u{FE9C}", -1, 26, None),
    Among("\u{FE9D}", -1, 27, None),
    Among("\u{FE9E}", -1, 27, None),
    Among("\u{FE9F}", -1, 27, None),
    Among("\u{FEA0}", -1, 27, None),
    Among("\u{FEA1}", -1, 28, None),
    Among("\u{FEA2}", -1, 28, None),
    Among("\u{FEA3}", -1, 28, None),
    Among("\u{FEA4}", -1, 28, None),
    Among("\u{FEA5}", -1, 29, None),
    Among("\u{FEA6}", -1, 29, None),
    Among("\u{FEA7}", -1, 29, None),
    Among("\u{FEA8}", -1, 29, None),
    Among("\u{FEA9}", -1, 30, None),
    Among("\u{FEAA}", -1, 30, None),
    Among("\u{FEAB}", -1, 31, None),
    Among("\u{FEAC}", -1, 31, None),
    Among("\u{FEAD}", -1, 32, None),
    Among("\u{FEAE}", -1, 32, None),
    Among("\u{FEAF}", -1, 33, None),
    Among("\u{FEB0}", -1, 33, None),
    Among("\u{FEB1}", -1, 34, None),
    Among("\u{FEB2}", -1, 34, None),
    Among("\u{FEB3}", -1, 34, None),
    Among("\u{FEB4}", -1, 34, None),
    Among("\u{FEB5}", -1, 35, None),
    Among("\u{FEB6}", -1, 35, None),
    Among("\u{FEB7}", -1, 35, None),
    Among("\u{FEB8}", -1, 35, None),
    Among("\u{FEB9}", -1, 36, None),
    Among("\u{FEBA}", -1, 36, None),
    Among("\u{FEBB}", -1, 36, None),
    Among("\u{FEBC}", -1, 36, None),
    Among("\u{FEBD}", -1, 37, None),
    Among("\u{FEBE}", -1, 37, None),
    Among("\u{FEBF}", -1, 37, None),
    Among("\u{FEC0}", -1, 37, None),
    Among("\u{FEC1}", -1, 38, None),
    Among("\u{FEC2}", -1, 38, None),
    Among("\u{FEC3}", -1, 38, None),
    Among("\u{FEC4}", -1, 38, None),
    Among("\u{FEC5}", -1, 39, None),
    Among("\u{FEC6}", -1, 39, None),
    Among("\u{FEC7}", -1, 39, None),
    Among("\u{FEC8}", -1, 39, None),
    Among("\u{FEC9}", -1, 40, None),
    Among("\u{FECA}", -1, 40, None),
    Among("\u{FECB}", -1, 40, None),
    Among("\u{FECC}", -1, 40, None),
    Among("\u{FECD}", -1, 41, None),
    Among("\u{FECE}", -1, 41, None),
    Among("\u{FECF}", -1, 41, None),
    Among("\u{FED0}", -1, 41, None),
    Among("\u{FED1}", -1, 42, None),
    Among("\u{FED2}", -1, 42, None),
    Among("\u{FED3}", -1, 42, None),
    Among("\u{FED4}", -1, 42, None),
    Among("\u{FED5}", -1, 43, None),
    Among("\u{FED6}", -1, 43, None),
    Among("\u{FED7}", -1, 43, None),
    Among("\u{FED8}", -1, 43, None),
    Among("\u{FED9}", -1, 44, None),
    Among("\u{FEDA}", -1, 44, None),
    Among("\u{FEDB}", -1, 44, None),
    Among("\u{FEDC}", -1, 44, None),
    Among("\u{FEDD}", -1, 45, None),
    Among("\u{FEDE}", -1, 45, None),
    Among("\u{FEDF}", -1, 45, None),
    Among("\u{FEE0}", -1, 45, None),
    Among("\u{FEE1}", -1, 46, None),
    Among("\u{FEE2}", -1, 46, None),
    Among("\u{FEE3}", -1, 46, None),
    Among("\u{FEE4}", -1, 46, None),
    Among("\u{FEE5}", -1, 47, None),
    Among("\u{FEE6}", -1, 47, None),
    Among("\u{FEE7}", -1, 47, None),
    Among("\u{FEE8}", -1, 47, None),
    Among("\u{FEE9}", -1, 48, None),
    Among("\u{FEEA}", -1, 48, None),
    Among("\u{FEEB}", -1, 48, None),
    Among("\u{FEEC}", -1, 48, None),
    Among("\u{FEED}", -1, 49, None),
    Among("\u{FEEE}", -1, 49, None),
    Among("\u{FEEF}", -1, 50, None),
    Among("\u{FEF0}", -1, 50, None),
    Among("\u{FEF1}", -1, 51, None),
    Among("\u{FEF2}", -1, 51, None),
    Among("\u{FEF3}", -1, 51, None),
    Among("\u{FEF4}", -1, 51, None),
    Among("\u{FEF5}", -1, 55, None),
    Among("\u{FEF6}", -1, 55, None),
    Among("\u{FEF7}", -1, 53, None),
    Among("\u{FEF8}", -1, 53, None),
    Among("\u{FEF9}", -1, 54, None),
    Among("\u{FEFA}", -1, 54, None),
    Among("\u{FEFB}", -1, 52, None),
    Among("\u{FEFC}", -1, 52, None),
];

static A_1: &'static [Among<Context>; 5] = &[
    Among("\u{0622}", -1, 1, None),
    Among("\u{0623}", -1, 1, None),
    Among("\u{0624}", -1, 2, None),
    Among("\u{0625}", -1, 1, None),
    Among("\u{0626}", -1, 3, None),
];

static A_2: &'static [Among<Context>; 5] = &[
    Among("\u{0622}", -1, 1, None),
    Among("\u{0623}", -1, 1, None),
    Among("\u{0624}", -1, 2, None),
    Among("\u{0625}", -1, 1, None),
    Among("\u{0626}", -1, 3, None),
];

static A_3: &'static [Among<Context>; 4] = &[
    Among("\u{0627}\u{0644}", -1, 2, None),
    Among("\u{0628}\u{0627}\u{0644}", -1, 1, None),
    Among("\u{0643}\u{0627}\u{0644}", -1, 1, None),
    Among("\u{0644}\u{0644}", -1, 2, None),
];

static A_4: &'static [Among<Context>; 5] = &[
    Among("\u{0623}\u{0622}", -1, 2, None),
    Among("\u{0623}\u{0623}", -1, 1, None),
    Among("\u{0623}\u{0624}", -1, 3, None),
    Among("\u{0623}\u{0625}", -1, 5, None),
    Among("\u{0623}\u{0627}", -1, 4, None),
];

static A_5: &'static [Among<Context>; 2] = &[
    Among("\u{0641}", -1, 1, None),
    Among("\u{0648}", -1, 2, None),
];

static A_6: &'static [Among<Context>; 4] = &[
    Among("\u{0627}\u{0644}", -1, 2, None),
    Among("\u{0628}\u{0627}\u{0644}", -1, 1, None),
    Among("\u{0643}\u{0627}\u{0644}", -1, 1, None),
    Among("\u{0644}\u{0644}", -1, 2, None),
];

static A_7: &'static [Among<Context>; 3] = &[
    Among("\u{0628}", -1, 1, None),
    Among("\u{0628}\u{0628}", 0, 2, None),
    Among("\u{0643}\u{0643}", -1, 3, None),
];

static A_8: &'static [Among<Context>; 4] = &[
    Among("\u{0633}\u{0623}", -1, 4, None),
    Among("\u{0633}\u{062A}", -1, 2, None),
    Among("\u{0633}\u{0646}", -1, 3, None),
    Among("\u{0633}\u{064A}", -1, 1, None),
];

static A_9: &'static [Among<Context>; 3] = &[
    Among("\u{062A}\u{0633}\u{062A}", -1, 1, None),
    Among("\u{0646}\u{0633}\u{062A}", -1, 1, None),
    Among("\u{064A}\u{0633}\u{062A}", -1, 1, None),
];

static A_10: &'static [Among<Context>; 10] = &[
    Among("\u{0643}", -1, 1, None),
    Among("\u{0643}\u{0645}", -1, 2, None),
    Among("\u{0647}\u{0645}", -1, 2, None),
    Among("\u{0647}\u{0646}", -1, 2, None),
    Among("\u{0647}", -1, 1, None),
    Among("\u{064A}", -1, 1, None),
    Among("\u{0643}\u{0645}\u{0627}", -1, 3, None),
    Among("\u{0647}\u{0645}\u{0627}", -1, 3, None),
    Among("\u{0646}\u{0627}", -1, 2, None),
    Among("\u{0647}\u{0627}", -1, 2, None),
];

static A_11: &'static [Among<Context>; 1] = &[
    Among("\u{0646}", -1, 1, None),
];

static A_12: &'static [Among<Context>; 3] = &[
    Among("\u{0648}", -1, 1, None),
    Among("\u{064A}", -1, 1, None),
    Among("\u{0627}", -1, 1, None),
];

static A_13: &'static [Among<Context>; 1] = &[
    Among("\u{0627}\u{062A}", -1, 1, None),
];

static A_14: &'static [Among<Context>; 1] = &[
    Among("\u{062A}", -1, 1, None),
];

static A_15: &'static [Among<Context>; 1] = &[
    Among("\u{0629}", -1, 1, None),
];

static A_16: &'static [Among<Context>; 1] = &[
    Among("\u{064A}", -1, 1, None),
];

static A_17: &'static [Among<Context>; 12] = &[
    Among("\u{0643}", -1, 1, None),
    Among("\u{0643}\u{0645}", -1, 2, None),
    Among("\u{0647}\u{0645}", -1, 2, None),
    Among("\u{0643}\u{0646}", -1, 2, None),
    Among("\u{0647}\u{0646}", -1, 2, None),
    Among("\u{0647}", -1, 1, None),
    Among("\u{0643}\u{0645}\u{0648}", -1, 3, None),
    Among("\u{0646}\u{064A}", -1, 2, None),
    Among("\u{0643}\u{0645}\u{0627}", -1, 3, None),
    Among("\u{0647}\u{0645}\u{0627}", -1, 3, None),
    Among("\u{0646}\u{0627}", -1, 2, None),
    Among("\u{0647}\u{0627}", -1, 2, None),
];

static A_18: &'static [Among<Context>; 11] = &[
    Among("\u{0646}", -1, 2, None),
    Among("\u{0648}\u{0646}", 0, 4, None),
    Among("\u{064A}\u{0646}", 0, 4, None),
    Among("\u{0627}\u{0646}", 0, 4, None),
    Among("\u{062A}\u{0646}", 0, 3, None),
    Among("\u{064A}", -1, 2, None),
    Among("\u{0627}", -1, 2, None),
    Among("\u{062A}\u{0645}\u{0627}", 6, 5, None),
    Among("\u{0646}\u{0627}", 6, 3, None),
    Among("\u{062A}\u{0627}", 6, 3, None),
    Among("\u{062A}", -1, 1, None),
];

static A_19: &'static [Among<Context>; 2] = &[
    Among("\u{062A}\u{0645}", -1, 1, None),
    Among("\u{0648}\u{0627}", -1, 1, None),
];

static A_20: &'static [Among<Context>; 2] = &[
    Among("\u{0648}", -1, 1, None),
    Among("\u{062A}\u{0645}\u{0648}", 0, 2, None),
];

static A_21: &'static [Among<Context>; 1] = &[
    Among("\u{0649}", -1, 1, None),
];

#[derive(Clone)]
struct Context {
    b_is_defined: bool,
    b_is_verb: bool,
    b_is_noun: bool,
    i_word_len: usize,
}

fn r_Normalize_pre(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 258
    // loop, line 259
    for _ in 0..env.current.chars().count() {
        // (, line 259
        // or, line 328
        'lab0: loop {
            let v_2 = env.cursor;
            'lab1: loop {
                // (, line 260
                // [, line 261
                env.bra = env.cursor;
                // substring, line 261
                among_var = env.find_among(A_0, context);
                if among_var == 0 {
                    break 'lab1;
                }
                // ], line 261
                env.ket = env.cursor;
                if among_var == 0 {
                    break 'lab1;
                } else if among_var == 1 {
                    // (, line 262
                    // delete, line 262
                    if !env.slice_del() {
                        return false;
                    }
                } else if among_var == 2 {
                    // (, line 263
                    // delete, line 263
                    if !env.slice_del() {
                        return false;
                    }
                } else if among_var == 3 {
                    // (, line 266
                    // delete, line 266
                    if !env.slice_del() {
                        return false;
                    }
                } else if among_var == 4 {
                    // (, line 267
                    // delete, line 267
                    if !env.slice_del() {
                        return false;
                    }
                } else if among_var == 5 {
                    // (, line 270
                    // <-, line 270
                    if !env.slice_from("0") {
                        return false;
                    }
                } else if among_var == 6 {
                    // (, line 271
                    // <-, line 271
                    if !env.slice_from("1") {
                        return false;
                    }
                } else if among_var == 7 {
                    // (, line 272
                    // <-, line 272
                    if !env.slice_from("2") {
                        return false;
                    }
                } else if among_var == 8 {
                    // (, line 273
                    // <-, line 273
                    if !env.slice_from("3") {
                        return false;
                    }
                } else if among_var == 9 {
                    // (, line 274
                    // <-, line 274
                    if !env.slice_from("4") {
                        return false;
                    }
                } else if among_var == 10 {
                    // (, line 275
                    // <-, line 275
                    if !env.slice_from("5") {
                        return false;
                    }
                } else if among_var == 11 {
                    // (, line 276
                    // <-, line 276
                    if !env.slice_from("6") {
                        return false;
                    }
                } else if among_var == 12 {
                    // (, line 277
                    // <-, line 277
                    if !env.slice_from("7") {
                        return false;
                    }
                } else if among_var == 13 {
                    // (, line 278
                    // <-, line 278
                    if !env.slice_from("8") {
                        return false;
                    }
                } else if among_var == 14 {
                    // (, line 279
                    // <-, line 279
                    if !env.slice_from("9") {
                        return false;
                    }
                } else if among_var == 15 {
                    // (, line 280
                    // delete, line 280
                    if !env.slice_del() {
                        return false;
                    }
                } else if among_var == 16 {
                    // (, line 283
                    // <-, line 283
                    if !env.slice_from("\u{0621}") {
                        return false;
                    }
                } else if among_var == 17 {
                    // (, line 284
                    // <-, line 284
                    if !env.slice_from("\u{0623}") {
                        return false;
                    }
                } else if among_var == 18 {
                    // (, line 285
                    // <-, line 285
                    if !env.slice_from("\u{0625}") {
                        return false;
                    }
                } else if among_var == 19 {
                    // (, line 286
                    // <-, line 286
                    if !env.slice_from("\u{0626}") {
                        return false;
                    }
                } else if among_var == 20 {
                    // (, line 287
                    // <-, line 287
                    if !env.slice_from("\u{0622}") {
                        return false;
                    }
                } else if among_var == 21 {
                    // (, line 288
                    // <-, line 288
                    if !env.slice_from("\u{0624}") {
                        return false;
                    }
                } else if among_var == 22 {
                    // (, line 289
                    // <-, line 289
                    if !env.slice_from("\u{0627}") {
                        return false;
                    }
                } else if among_var == 23 {
                    // (, line 290
                    // <-, line 290
                    if !env.slice_from("\u{0628}") {
                        return false;
                    }
                } else if among_var == 24 {
                    // (, line 291
                    // <-, line 291
                    if !env.slice_from("\u{0629}") {
                        return false;
                    }
                } else if among_var == 25 {
                    // (, line 292
                    // <-, line 292
                    if !env.slice_from("\u{062A}") {
                        return false;
                    }
                } else if among_var == 26 {
                    // (, line 293
                    // <-, line 293
                    if !env.slice_from("\u{062B}") {
                        return false;
                    }
                } else if among_var == 27 {
                    // (, line 294
                    // <-, line 294
                    if !env.slice_from("\u{062C}") {
                        return false;
                    }
                } else if among_var == 28 {
                    // (, line 295
                    // <-, line 295
                    if !env.slice_from("\u{062D}") {
                        return false;
                    }
                } else if among_var == 29 {
                    // (, line 296
                    // <-, line 296
                    if !env.slice_from("\u{062E}") {
                        return false;
                    }
                } else if among_var == 30 {
                    // (, line 297
                    // <-, line 297
                    if !env.slice_from("\u{062F}") {
                        return false;
                    }
                } else if among_var == 31 {
                    // (, line 298
                    // <-, line 298
                    if !env.slice_from("\u{0630}") {
                        return false;
                    }
                } else if among_var == 32 {
                    // (, line 299
                    // <-, line 299
                    if !env.slice_from("\u{0631}") {
                        return false;
                    }
                } else if among_var == 33 {
                    // (, line 300
                    // <-, line 300
                    if !env.slice_from("\u{0632}") {
                        return false;
                    }
                } else if among_var == 34 {
                    // (, line 301
                    // <-, line 301
                    if !env.slice_from("\u{0633}") {
                        return false;
                    }
                } else if among_var == 35 {
                    // (, line 302
                    // <-, line 302
                    if !env.slice_from("\u{0634}") {
                        return false;
                    }
                } else if among_var == 36 {
                    // (, line 303
                    // <-, line 303
                    if !env.slice_from("\u{0635}") {
                        return false;
                    }
                } else if among_var == 37 {
                    // (, line 304
                    // <-, line 304
                    if !env.slice_from("\u{0636}") {
                        return false;
                    }
                } else if among_var == 38 {
                    // (, line 305
                    // <-, line 305
                    if !env.slice_from("\u{0637}") {
                        return false;
                    }
                } else if among_var == 39 {
                    // (, line 306
                    // <-, line 306
                    if !env.slice_from("\u{0638}") {
                        return false;
                    }
                } else if among_var == 40 {
                    // (, line 307
                    // <-, line 307
                    if !env.slice_from("\u{0639}") {
                        return false;
                    }
                } else if among_var == 41 {
                    // (, line 308
                    // <-, line 308
                    if !env.slice_from("\u{063A}") {
                        return false;
                    }
                } else if among_var == 42 {
                    // (, line 309
                    // <-, line 309
                    if !env.slice_from("\u{0641}") {
                        return false;
                    }
                } else if among_var == 43 {
                    // (, line 310
                    // <-, line 310
                    if !env.slice_from("\u{0642}") {
                        return false;
                    }
                } else if among_var == 44 {
                    // (, line 311
                    // <-, line 311
                    if !env.slice_from("\u{0643}") {
                        return false;
                    }
                } else if among_var == 45 {
                    // (, line 312
                    // <-, line 312
                    if !env.slice_from("\u{0644}") {
                        return false;
                    }
                } else if among_var == 46 {
                    // (, line 313
                    // <-, line 313
                    if !env.slice_from("\u{0645}") {
                        return false;
                    }
                } else if among_var == 47 {
                    // (, line 314
                    // <-, line 314
                    if !env.slice_from("\u{0646}") {
                        return false;
                    }
                } else if among_var == 48 {
                    // (, line 315
                    // <-, line 315
                    if !env.slice_from("\u{0647}") {
                        return false;
                    }
                } else if among_var == 49 {
                    // (, line 316
                    // <-, line 316
                    if !env.slice_from("\u{0648}") {
                        return false;
                    }
                } else if among_var == 50 {
                    // (, line 317
                    // <-, line 317
                    if !env.slice_from("\u{0649}") {
                        return false;
                    }
                } else if among_var == 51 {
                    // (, line 318
                    // <-, line 318
                    if !env.slice_from("\u{064A}") {
                        return false;
                    }
                } else if among_var == 52 {
                    // (, line 321
                    // <-, line 321
                    if !env.slice_from("\u{0644}\u{0627}") {
                        return false;
                    }
                } else if among_var == 53 {
                    // (, line 322
                    // <-, line 322
                    if !env.slice_from("\u{0644}\u{0623}") {
                        return false;
                    }
                } else if among_var == 54 {
                    // (, line 323
                    // <-, line 323
                    if !env.slice_from("\u{0644}\u{0625}") {
                        return false;
                    }
                } else if among_var == 55 {
                    // (, line 324
                    // <-, line 324
                    if !env.slice_from("\u{0644}\u{0622}") {
                        return false;
                    }
                }
                break 'lab0;
            }
            env.cursor = v_2;
            // next, line 329
            if env.cursor >= env.limit {
                return false;
            }
            env.next_char();
            break 'lab0;
        }
    }
    return true;
}

fn r_Normalize_post(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 333
    // do, line 335
    let v_1 = env.cursor;
    'lab0: loop {
        // (, line 335
        // backwards, line 337
        env.limit_backward = env.cursor;
        env.cursor = env.limit;
        // (, line 337
        // [, line 338
        env.ket = env.cursor;
        // substring, line 338
        among_var = env.find_among_b(A_1, context);
        if among_var == 0 {
            break 'lab0;
        }
        // ], line 338
        env.bra = env.cursor;
        if among_var == 0 {
            break 'lab0;
        } else if among_var == 1 {
            // (, line 339
            // <-, line 339
            if !env.slice_from("\u{0621}") {
                return false;
            }
        } else if among_var == 2 {
            // (, line 340
            // <-, line 340
            if !env.slice_from("\u{0621}") {
                return false;
            }
        } else if among_var == 3 {
            // (, line 341
            // <-, line 341
            if !env.slice_from("\u{0621}") {
                return false;
            }
        }
        env.cursor = env.limit_backward;
        break 'lab0;
    }
    env.cursor = v_1;
    // do, line 346
    let v_2 = env.cursor;
    'lab1: loop {
        // loop, line 346
        for _ in 0..context.i_word_len {
            // (, line 346
            // or, line 355
            'lab2: loop {
                let v_4 = env.cursor;
                'lab3: loop {
                    // (, line 347
                    // [, line 349
                    env.bra = env.cursor;
                    // substring, line 349
                    among_var = env.find_among(A_2, context);
                    if among_var == 0 {
                        break 'lab3;
                    }
                    // ], line 349
                    env.ket = env.cursor;
                    if among_var == 0 {
                        break 'lab3;
                    } else if among_var == 1 {
                        // (, line 350
                        // <-, line 350
                        if !env.slice_from("\u{0627}") {
                            return false;
                        }
                    } else if among_var == 2 {
                        // (, line 351
                        // <-, line 351
                        if !env.slice_from("\u{0648}") {
                            return false;
                        }
                    } else if among_var == 3 {
                        // (, line 352
                        // <-, line 352
                        if !env.slice_from("\u{064A}") {
                            return false;
                        }
                    }
                    break 'lab2;
                }
                env.cursor = v_4;
                // next, line 356
                if env.cursor >= env.limit {
                    break 'lab1;
                }
                env.next_char();
                break 'lab2;
            }
        }
        break 'lab1;
    }
    env.cursor = v_2;
    return true;
}

fn r_Checks1(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 361
    context.i_word_len = env.current.chars().count();
    // [, line 363
    env.bra = env.cursor;
    // substring, line 363
    among_var = env.find_among(A_3, context);
    if among_var == 0 {
        return false;
    }
    // ], line 363
    env.ket = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 364
        if !(context.i_word_len > 4){
            return false;
        }
        // set is_noun, line 364
        context.b_is_noun = true;
        // unset is_verb, line 364
        context.b_is_verb = false;
        // set is_defined, line 364
        context.b_is_defined = true;
    } else if among_var == 2 {
        // (, line 365
        if !(context.i_word_len > 3){
            return false;
        }
        // set is_noun, line 365
        context.b_is_noun = true;
        // unset is_verb, line 365
        context.b_is_verb = false;
        // set is_defined, line 365
        context.b_is_defined = true;
    }
    return true;
}

fn r_Prefix_Step1(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 371
    context.i_word_len = env.current.chars().count();
    // [, line 373
    env.bra = env.cursor;
    // substring, line 373
    among_var = env.find_among(A_4, context);
    if among_var == 0 {
        return false;
    }
    // ], line 373
    env.ket = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 374
        if !(context.i_word_len > 3){
            return false;
        }
        // <-, line 374
        if !env.slice_from("\u{0623}") {
            return false;
        }
    } else if among_var == 2 {
        // (, line 375
        if !(context.i_word_len > 3){
            return false;
        }
        // <-, line 375
        if !env.slice_from("\u{0622}") {
            return false;
        }
    } else if among_var == 3 {
        // (, line 376
        if !(context.i_word_len > 3){
            return false;
        }
        // <-, line 376
        if !env.slice_from("\u{0623}") {
            return false;
        }
    } else if among_var == 4 {
        // (, line 377
        if !(context.i_word_len > 3){
            return false;
        }
        // <-, line 377
        if !env.slice_from("\u{0627}") {
            return false;
        }
    } else if among_var == 5 {
        // (, line 378
        if !(context.i_word_len > 3){
            return false;
        }
        // <-, line 378
        if !env.slice_from("\u{0625}") {
            return false;
        }
    }
    return true;
}

fn r_Prefix_Step2(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 383
    context.i_word_len = env.current.chars().count();
    // not, line 385
    let v_1 = env.cursor;
    'lab0: loop {
        // literal, line 385
        if !env.eq_s(&"\u{0641}\u{0627}") {
            break 'lab0;
        }
        return false;
    }
    env.cursor = v_1;
    // not, line 386
    let v_2 = env.cursor;
    'lab1: loop {
        // literal, line 386
        if !env.eq_s(&"\u{0648}\u{0627}") {
            break 'lab1;
        }
        return false;
    }
    env.cursor = v_2;
    // [, line 387
    env.bra = env.cursor;
    // substring, line 387
    among_var = env.find_among(A_5, context);
    if among_var == 0 {
        return false;
    }
    // ], line 387
    env.ket = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 388
        if !(context.i_word_len > 3){
            return false;
        }
        // delete, line 388
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 2 {
        // (, line 389
        if !(context.i_word_len > 3){
            return false;
        }
        // delete, line 389
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Prefix_Step3a_Noun(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 393
    context.i_word_len = env.current.chars().count();
    // [, line 395
    env.bra = env.cursor;
    // substring, line 395
    among_var = env.find_among(A_6, context);
    if among_var == 0 {
        return false;
    }
    // ], line 395
    env.ket = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 396
        if !(context.i_word_len > 5){
            return false;
        }
        // delete, line 396
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 2 {
        // (, line 397
        if !(context.i_word_len > 4){
            return false;
        }
        // delete, line 397
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Prefix_Step3b_Noun(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 401
    context.i_word_len = env.current.chars().count();
    // not, line 403
    let v_1 = env.cursor;
    'lab0: loop {
        // literal, line 403
        if !env.eq_s(&"\u{0628}\u{0627}") {
            break 'lab0;
        }
        return false;
    }
    env.cursor = v_1;
    // [, line 404
    env.bra = env.cursor;
    // substring, line 404
    among_var = env.find_among(A_7, context);
    if among_var == 0 {
        return false;
    }
    // ], line 404
    env.ket = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 405
        if !(context.i_word_len > 3){
            return false;
        }
        // delete, line 405
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 2 {
        // (, line 407
        if !(context.i_word_len > 3){
            return false;
        }
        // <-, line 407
        if !env.slice_from("\u{0628}") {
            return false;
        }
    } else if among_var == 3 {
        // (, line 408
        if !(context.i_word_len > 3){
            return false;
        }
        // <-, line 408
        if !env.slice_from("\u{0643}") {
            return false;
        }
    }
    return true;
}

fn r_Prefix_Step3_Verb(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 413
    context.i_word_len = env.current.chars().count();
    // [, line 415
    env.bra = env.cursor;
    // substring, line 415
    among_var = env.find_among(A_8, context);
    if among_var == 0 {
        return false;
    }
    // ], line 415
    env.ket = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 417
        if !(context.i_word_len > 4){
            return false;
        }
        // <-, line 417
        if !env.slice_from("\u{064A}") {
            return false;
        }
    } else if among_var == 2 {
        // (, line 418
        if !(context.i_word_len > 4){
            return false;
        }
        // <-, line 418
        if !env.slice_from("\u{062A}") {
            return false;
        }
    } else if among_var == 3 {
        // (, line 419
        if !(context.i_word_len > 4){
            return false;
        }
        // <-, line 419
        if !env.slice_from("\u{0646}") {
            return false;
        }
    } else if among_var == 4 {
        // (, line 420
        if !(context.i_word_len > 4){
            return false;
        }
        // <-, line 420
        if !env.slice_from("\u{0623}") {
            return false;
        }
    }
    return true;
}

fn r_Prefix_Step4_Verb(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 424
    context.i_word_len = env.current.chars().count();
    // [, line 426
    env.bra = env.cursor;
    // substring, line 426
    among_var = env.find_among(A_9, context);
    if among_var == 0 {
        return false;
    }
    // ], line 426
    env.ket = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 427
        if !(context.i_word_len > 4){
            return false;
        }
        // set is_verb, line 427
        context.b_is_verb = true;
        // unset is_noun, line 427
        context.b_is_noun = false;
        // <-, line 427
        if !env.slice_from("\u{0627}\u{0633}\u{062A}") {
            return false;
        }
    }
    return true;
}

fn r_Suffix_Noun_Step1a(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 434
    context.i_word_len = env.current.chars().count();
    // [, line 436
    env.ket = env.cursor;
    // substring, line 436
    among_var = env.find_among_b(A_10, context);
    if among_var == 0 {
        return false;
    }
    // ], line 436
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 437
        if !(context.i_word_len >= 4){
            return false;
        }
        // delete, line 437
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 2 {
        // (, line 438
        if !(context.i_word_len >= 5){
            return false;
        }
        // delete, line 438
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 3 {
        // (, line 439
        if !(context.i_word_len >= 6){
            return false;
        }
        // delete, line 439
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Suffix_Noun_Step1b(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 442
    context.i_word_len = env.current.chars().count();
    // [, line 444
    env.ket = env.cursor;
    // substring, line 444
    among_var = env.find_among_b(A_11, context);
    if among_var == 0 {
        return false;
    }
    // ], line 444
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 445
        if !(context.i_word_len > 5){
            return false;
        }
        // delete, line 445
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Suffix_Noun_Step2a(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 449
    context.i_word_len = env.current.chars().count();
    // [, line 451
    env.ket = env.cursor;
    // substring, line 451
    among_var = env.find_among_b(A_12, context);
    if among_var == 0 {
        return false;
    }
    // ], line 451
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 452
        if !(context.i_word_len > 4){
            return false;
        }
        // delete, line 452
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Suffix_Noun_Step2b(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 456
    context.i_word_len = env.current.chars().count();
    // [, line 458
    env.ket = env.cursor;
    // substring, line 458
    among_var = env.find_among_b(A_13, context);
    if among_var == 0 {
        return false;
    }
    // ], line 458
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 459
        if !(context.i_word_len >= 5){
            return false;
        }
        // delete, line 459
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Suffix_Noun_Step2c1(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 463
    context.i_word_len = env.current.chars().count();
    // [, line 465
    env.ket = env.cursor;
    // substring, line 465
    among_var = env.find_among_b(A_14, context);
    if among_var == 0 {
        return false;
    }
    // ], line 465
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 466
        if !(context.i_word_len >= 4){
            return false;
        }
        // delete, line 466
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Suffix_Noun_Step2c2(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 469
    context.i_word_len = env.current.chars().count();
    // [, line 471
    env.ket = env.cursor;
    // substring, line 471
    among_var = env.find_among_b(A_15, context);
    if among_var == 0 {
        return false;
    }
    // ], line 471
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 472
        if !(context.i_word_len >= 4){
            return false;
        }
        // delete, line 472
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Suffix_Noun_Step3(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 475
    context.i_word_len = env.current.chars().count();
    // [, line 477
    env.ket = env.cursor;
    // substring, line 477
    among_var = env.find_among_b(A_16, context);
    if among_var == 0 {
        return false;
    }
    // ], line 477
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 478
        if !(context.i_word_len >= 3){
            return false;
        }
        // delete, line 478
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Suffix_Verb_Step1(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 482
    context.i_word_len = env.current.chars().count();
    // [, line 484
    env.ket = env.cursor;
    // substring, line 484
    among_var = env.find_among_b(A_17, context);
    if among_var == 0 {
        return false;
    }
    // ], line 484
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 485
        if !(context.i_word_len >= 4){
            return false;
        }
        // delete, line 485
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 2 {
        // (, line 486
        if !(context.i_word_len >= 5){
            return false;
        }
        // delete, line 486
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 3 {
        // (, line 487
        if !(context.i_word_len >= 6){
            return false;
        }
        // delete, line 487
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Suffix_Verb_Step2a(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 490
    context.i_word_len = env.current.chars().count();
    // [, line 492
    env.ket = env.cursor;
    // substring, line 492
    among_var = env.find_among_b(A_18, context);
    if among_var == 0 {
        return false;
    }
    // ], line 492
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 493
        if !(context.i_word_len >= 4){
            return false;
        }
        // delete, line 493
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 2 {
        // (, line 494
        if !(context.i_word_len >= 4){
            return false;
        }
        // delete, line 494
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 3 {
        // (, line 495
        if !(context.i_word_len >= 5){
            return false;
        }
        // delete, line 495
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 4 {
        // (, line 496
        if !(context.i_word_len > 5){
            return false;
        }
        // delete, line 496
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 5 {
        // (, line 497
        if !(context.i_word_len >= 6){
            return false;
        }
        // delete, line 497
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Suffix_Verb_Step2b(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 501
    context.i_word_len = env.current.chars().count();
    // [, line 503
    env.ket = env.cursor;
    // substring, line 503
    among_var = env.find_among_b(A_19, context);
    if among_var == 0 {
        return false;
    }
    // ], line 503
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 504
        if !(context.i_word_len >= 5){
            return false;
        }
        // delete, line 504
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Suffix_Verb_Step2c(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 509
    context.i_word_len = env.current.chars().count();
    // [, line 511
    env.ket = env.cursor;
    // substring, line 511
    among_var = env.find_among_b(A_20, context);
    if among_var == 0 {
        return false;
    }
    // ], line 511
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 512
        if !(context.i_word_len >= 4){
            return false;
        }
        // delete, line 512
        if !env.slice_del() {
            return false;
        }
    } else if among_var == 2 {
        // (, line 513
        if !(context.i_word_len >= 6){
            return false;
        }
        // delete, line 513
        if !env.slice_del() {
            return false;
        }
    }
    return true;
}

fn r_Suffix_All_alef_maqsura(env: &mut SnowballEnv, context: &mut Context) -> bool {
    let mut among_var;
    // (, line 517
    context.i_word_len = env.current.chars().count();
    // [, line 519
    env.ket = env.cursor;
    // substring, line 519
    among_var = env.find_among_b(A_21, context);
    if among_var == 0 {
        return false;
    }
    // ], line 519
    env.bra = env.cursor;
    if among_var == 0 {
        return false;
    } else if among_var == 1 {
        // (, line 520
        // <-, line 520
        if !env.slice_from("\u{064A}") {
            return false;
        }
    }
    return true;
}

pub fn stem(env: &mut SnowballEnv) -> bool {
    let mut context = &mut Context {
        b_is_defined: false,
        b_is_verb: false,
        b_is_noun: false,
        i_word_len: 0,
    };
    // (, line 527
    // set is_noun, line 529
    context.b_is_noun = true;
    // set is_verb, line 530
    context.b_is_verb = true;
    // unset is_defined, line 531
    context.b_is_defined = false;
    // do, line 534
    let v_1 = env.cursor;
    'lab0: loop {
        // call Checks1, line 534
        if !r_Checks1(env, context) {
            break 'lab0;
        }
        break 'lab0;
    }
    env.cursor = v_1;
    // do, line 537
    let v_2 = env.cursor;
    'lab1: loop {
        // call Normalize_pre, line 537
        if !r_Normalize_pre(env, context) {
            break 'lab1;
        }
        break 'lab1;
    }
    env.cursor = v_2;
    // backwards, line 540
    env.limit_backward = env.cursor;
    env.cursor = env.limit;
    // (, line 540
    // do, line 542
    let v_3 = env.limit - env.cursor;
    'lab2: loop {
        // (, line 542
        // or, line 556
        'lab3: loop {
            let v_4 = env.limit - env.cursor;
            'lab4: loop {
                // (, line 544
                // Boolean test is_verb, line 545
                if !context.b_is_verb {
                    break 'lab4;
                }
                // (, line 546
                // or, line 551
                'lab5: loop {
                    let v_5 = env.limit - env.cursor;
                    'lab6: loop {
                        // (, line 547
                        // (, line 548
                        // atleast, line 548
                        let mut v_6 = 1;
                        // atleast, line 548
                        'replab7: loop{
                            let v_7 = env.limit - env.cursor;
                            'lab8: for _ in 0..1 {
                                // call Suffix_Verb_Step1, line 548
                                if !r_Suffix_Verb_Step1(env, context) {
                                    break 'lab8;
                                }
                                v_6 -= 1;
                                continue 'replab7;
                            }
                            env.cursor = env.limit - v_7;
                            break 'replab7;
                        }
                        if v_6 > 0 {
                            break 'lab6;
                        }
                        // (, line 549
                        // or, line 549
                        'lab9: loop {
                            let v_8 = env.limit - env.cursor;
                            'lab10: loop {
                                // call Suffix_Verb_Step2a, line 549
                                if !r_Suffix_Verb_Step2a(env, context) {
                                    break 'lab10;
                                }
                                break 'lab9;
                            }
                            env.cursor = env.limit - v_8;
                            'lab11: loop {
                                // call Suffix_Verb_Step2c, line 549
                                if !r_Suffix_Verb_Step2c(env, context) {
                                    break 'lab11;
                                }
                                break 'lab9;
                            }
                            env.cursor = env.limit - v_8;
                            // next, line 549
                            if env.cursor <= env.limit_backward {
                                break 'lab6;
                            }
                            env.previous_char();
                            break 'lab9;
                        }
                        break 'lab5;
                    }
                    env.cursor = env.limit - v_5;
                    'lab12: loop {
                        // call Suffix_Verb_Step2b, line 551
                        if !r_Suffix_Verb_Step2b(env, context) {
                            break 'lab12;
                        }
                        break 'lab5;
                    }
                    env.cursor = env.limit - v_5;
                    // call Suffix_Verb_Step2a, line 552
                    if !r_Suffix_Verb_Step2a(env, context) {
                        break 'lab4;
                    }
                    break 'lab5;
                }
                break 'lab3;
            }
            env.cursor = env.limit - v_4;
            'lab13: loop {
                // (, line 556
                // Boolean test is_noun, line 557
                if !context.b_is_noun {
                    break 'lab13;
                }
                // (, line 558
                // try, line 560
                let v_9 = env.limit - env.cursor;
                'lab14: loop {
                    // (, line 560
                    // or, line 562
                    'lab15: loop {
                        let v_10 = env.limit - env.cursor;
                        'lab16: loop {
                            // call Suffix_Noun_Step2c2, line 561
                            if !r_Suffix_Noun_Step2c2(env, context) {
                                break 'lab16;
                            }
                            break 'lab15;
                        }
                        env.cursor = env.limit - v_10;
                        'lab17: loop {
                            // (, line 562
                            // not, line 562
                            'lab18: loop {
                                // Boolean test is_defined, line 562
                                if !context.b_is_defined {
                                    break 'lab18;
                                }
                                break 'lab17;
                            }
                            // call Suffix_Noun_Step1a, line 562
                            if !r_Suffix_Noun_Step1a(env, context) {
                                break 'lab17;
                            }
                            // (, line 562
                            // or, line 564
                            'lab19: loop {
                                let v_12 = env.limit - env.cursor;
                                'lab20: loop {
                                    // call Suffix_Noun_Step2a, line 563
                                    if !r_Suffix_Noun_Step2a(env, context) {
                                        break 'lab20;
                                    }
                                    break 'lab19;
                                }
                                env.cursor = env.limit - v_12;
                                'lab21: loop {
                                    // call Suffix_Noun_Step2b, line 564
                                    if !r_Suffix_Noun_Step2b(env, context) {
                                        break 'lab21;
                                    }
                                    break 'lab19;
                                }
                                env.cursor = env.limit - v_12;
                                'lab22: loop {
                                    // call Suffix_Noun_Step2c1, line 565
                                    if !r_Suffix_Noun_Step2c1(env, context) {
                                        break 'lab22;
                                    }
                                    break 'lab19;
                                }
                                env.cursor = env.limit - v_12;
                                // next, line 566
                                if env.cursor <= env.limit_backward {
                                    break 'lab17;
                                }
                                env.previous_char();
                                break 'lab19;
                            }
                            break 'lab15;
                        }
                        env.cursor = env.limit - v_10;
                        'lab23: loop {
                            // (, line 567
                            // call Suffix_Noun_Step1b, line 567
                            if !r_Suffix_Noun_Step1b(env, context) {
                                break 'lab23;
                            }
                            // (, line 567
                            // or, line 569
                            'lab24: loop {
                                let v_13 = env.limit - env.cursor;
                                'lab25: loop {
                                    // call Suffix_Noun_Step2a, line 568
                                    if !r_Suffix_Noun_Step2a(env, context) {
                                        break 'lab25;
                                    }
                                    break 'lab24;
                                }
                                env.cursor = env.limit - v_13;
                                'lab26: loop {
                                    // call Suffix_Noun_Step2b, line 569
                                    if !r_Suffix_Noun_Step2b(env, context) {
                                        break 'lab26;
                                    }
                                    break 'lab24;
                                }
                                env.cursor = env.limit - v_13;
                                // call Suffix_Noun_Step2c1, line 570
                                if !r_Suffix_Noun_Step2c1(env, context) {
                                    break 'lab23;
                                }
                                break 'lab24;
                            }
                            break 'lab15;
                        }
                        env.cursor = env.limit - v_10;
                        'lab27: loop {
                            // (, line 571
                            // not, line 571
                            'lab28: loop {
                                // Boolean test is_defined, line 571
                                if !context.b_is_defined {
                                    break 'lab28;
                                }
                                break 'lab27;
                            }
                            // call Suffix_Noun_Step2a, line 571
                            if !r_Suffix_Noun_Step2a(env, context) {
                                break 'lab27;
                            }
                            break 'lab15;
                        }
                        env.cursor = env.limit - v_10;
                        // (, line 572
                        // call Suffix_Noun_Step2b, line 572
                        if !r_Suffix_Noun_Step2b(env, context) {
                            env.cursor = env.limit - v_9;
                            break 'lab14;
                        }
                        break 'lab15;
                    }
                    break 'lab14;
                }
                // call Suffix_Noun_Step3, line 574
                if !r_Suffix_Noun_Step3(env, context) {
                    break 'lab13;
                }
                break 'lab3;
            }
            env.cursor = env.limit - v_4;
            // call Suffix_All_alef_maqsura, line 580
            if !r_Suffix_All_alef_maqsura(env, context) {
                break 'lab2;
            }
            break 'lab3;
        }
        break 'lab2;
    }
    env.cursor = env.limit - v_3;
    env.cursor = env.limit_backward;
    // do, line 585
    let v_15 = env.cursor;
    'lab29: loop {
        // (, line 585
        // try, line 586
        let v_16 = env.cursor;
        'lab30: loop {
            // call Prefix_Step1, line 586
            if !r_Prefix_Step1(env, context) {
                env.cursor = v_16;
                break 'lab30;
            }
            break 'lab30;
        }
        // try, line 587
        let v_17 = env.cursor;
        'lab31: loop {
            // call Prefix_Step2, line 587
            if !r_Prefix_Step2(env, context) {
                env.cursor = v_17;
                break 'lab31;
            }
            break 'lab31;
        }
        // (, line 588
        // or, line 589
        'lab32: loop {
            let v_18 = env.cursor;
            'lab33: loop {
                // call Prefix_Step3a_Noun, line 588
                if !r_Prefix_Step3a_Noun(env, context) {
                    break 'lab33;
                }
                break 'lab32;
            }
            env.cursor = v_18;
            'lab34: loop {
                // (, line 589
                // Boolean test is_noun, line 589
                if !context.b_is_noun {
                    break 'lab34;
                }
                // call Prefix_Step3b_Noun, line 589
                if !r_Prefix_Step3b_Noun(env, context) {
                    break 'lab34;
                }
                break 'lab32;
            }
            env.cursor = v_18;
            // (, line 590
            // Boolean test is_verb, line 590
            if !context.b_is_verb {
                break 'lab29;
            }
            // try, line 590
            let v_19 = env.cursor;
            'lab35: loop {
                // call Prefix_Step3_Verb, line 590
                if !r_Prefix_Step3_Verb(env, context) {
                    env.cursor = v_19;
                    break 'lab35;
                }
                break 'lab35;
            }
            // call Prefix_Step4_Verb, line 590
            if !r_Prefix_Step4_Verb(env, context) {
                break 'lab29;
            }
            break 'lab32;
        }
        break 'lab29;
    }
    env.cursor = v_15;
    // do, line 595
    let v_20 = env.cursor;
    'lab36: loop {
        // call Normalize_post, line 595
        if !r_Normalize_post(env, context) {
            break 'lab36;
        }
        break 'lab36;
    }
    env.cursor = v_20;
    return true;
}
