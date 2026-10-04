// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Lookup tables for coupon-based cardinality estimation
//!
//! These pre-computed X and Y value pairs are used with cubic interpolation
//! to map from the number of observed coupons to estimated cardinality.

/// X values (coupon counts) for interpolation table
pub static X_ARR: [f64; 40] = [
    0.0, 1.0, 20.0, 400.0, 8000.0, 160000.0, 300000.0, 600000.0, 900000.0, 1200000.0, 1500000.0,
    1800000.0, 2100000.0, 2400000.0, 2700000.0, 3000000.0, 3300000.0, 3600000.0, 3900000.0,
    4200000.0, 4500000.0, 4800000.0, 5100000.0, 5400000.0, 5700000.0, 6000000.0, 6300000.0,
    6600000.0, 6900000.0, 7200000.0, 7500000.0, 7800000.0, 8100000.0, 8400000.0, 8700000.0,
    9000000.0, 9300000.0, 9600000.0, 9900000.0, 10200000.0,
];

/// Y values (estimated cardinalities) for interpolation table
pub static Y_ARR: [f64; 40] = [
    0.0000000000000000,
    1.0000000000000000,
    20.000_000_943_740_26,
    400.000_396_371_338_4,
    8_000.158_929_460_209,
    160_063.606_776_375_96,
    300_223.707_159_766_35,
    600_895.593_385_617,
    902_016.806_512_095_5,
    1_203_588.498_319_951,
    1_505_611.824_552_474_3,
    1_808_087.944_931_906_6,
    2_111_018.023_175_935_3,
    2_414_403.227_014_25,
    2_718_244.728_205_189,
    3_022_543.702_552_454,
    3_327_301.329_921_909,
    3_632_518.794_258_454,
    3_938_197.283_602_969,
    4_244_337.990_109_356,
    4_550_942.110_061_649,
    4_858_010.843_891_189,
    5_165_545.396_193_897,
    5_473_546.975_747_645,
    5_782_016.795_529_650_5,
    6_090_956.072_734_016,
    6_400_366.028_789_296,
    6_710_247.889_376_201,
    7_020_602.884_445_314,
    7_331_432.248_234_972,
    7_642_737.219_289_148,
    7_954_519.040_475_476_5,
    8_266_778.959_003_342,
    8_579_518.226_442_046,
    8_892_738.098_739_047,
    9_206_439.836_238_328,
    9_520_624.703_698_829,
    9_835_293.970_312_92,
    10_150_448.909_725_029,
    10_466_090.800_050_326,
];
