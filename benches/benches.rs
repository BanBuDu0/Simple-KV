// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

#![allow(dead_code)] // Due to criterion we need this to avoid warnings.
#![cfg_attr(feature = "cargo-clippy", allow(clippy::let_and_return))] // Benches often artificially return values. Allow it.

use criterion::Criterion;
use std::time::Duration;

mod suites;

pub const DEFAULT_SCAN_SETS: [(i64, i64); 5] = [(0, 0), (-1, -1), (1, 100), (1, 20), (1, 1000)];

pub const DEFAULT_GET_SETS: [i64; 5] = [0,1,2,3,4];
pub const DEFAULT_PUT_SETS: [(i64, &str); 5] = [
    (0, "aaa"),
    (1, "bbb"),
    (2, "ccc"),
    (3, "ddd"),
    (4, "eee")
];

pub const DEFAULT_DELETE_SETS: [i64; 5] = [0,1,2,3,4];

fn main() {
    let mut c = Criterion::default()
        // Configure defaults before overriding with args.
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(3))
        .configure_from_args();

    suites::bench_client(&mut c);

    c.final_summary();
}
