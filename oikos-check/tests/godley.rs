// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Integration tests for the Godley matrix column-sum invariant checker.
//!
//! Each test builds a `Model` programmatically and calls `check_model`.
//! The checker is structural: it counts sign polarities per sector column
//! and requires every column sum to be zero.

mod fixtures;
use fixtures::*;

use oikos_check::{check_model, CheckError};
use oikos_syntax::godley::GodleySign;

// ── Passing cases ─────────────────────────────────────────────────────────────

#[test]
fn empty_godley_passes() {
    // No sectors → nothing to violate. Vacuously balanced.
    let m = model_godley(vec![], vec![], vec![]);
    assert!(check_model(&m).is_ok());
}

#[test]
fn two_sector_one_row_balanced() {
    // deposits: Households(+), Banks(-) → H=+1, B=-1 … columns NOT zero yet.
    // Add a second row to balance each column.
    //
    // | Account     | Households | Banks |
    // | consumption | -          | +     |   H:-1, B:+1
    // | wages       | +          | -     |   H:+1, B:-1
    // Column sums: H=0, B=0 ✓
    let m = model_godley(
        vec![
            minus("consumption", "Households"),
            plus("consumption", "Banks"),
            plus("wages", "Households"),
            minus("wages", "Banks"),
        ],
        vec!["Households", "Banks"],
        vec!["consumption", "wages"],
    );
    assert!(check_model(&m).is_ok());
}

#[test]
fn three_sector_balanced() {
    // | Account | Households | Firms | Banks |
    // | wages   | +          | -     |       |   H:+1, F:-1
    // | loans   |            | +     | -     |   F:+1, B:-1
    // | int     |            | -     | +     |   F:-1, B:+1
    // | taxes   | -          | +     |       |   H:-1, F:+1
    // Column sums: H=0, F=0, B=0 ✓
    let m = model_godley(
        vec![
            plus("wages", "Households"),
            minus("wages", "Firms"),
            plus("loans", "Firms"),
            minus("loans", "Banks"),
            minus("interest", "Firms"),
            plus("interest", "Banks"),
            minus("taxes", "Households"),
            plus("taxes", "Firms"),
        ],
        vec!["Households", "Firms", "Banks"],
        vec!["wages", "loans", "interest", "taxes"],
    );
    assert!(check_model(&m).is_ok());
}

#[test]
fn all_zero_cells_pass() {
    // Every cell is Zero → all column sums are 0.
    let m = model_godley(
        vec![
            cell("deposits", "Households", GodleySign::Zero),
            cell("deposits", "Banks", GodleySign::Zero),
        ],
        vec!["Households", "Banks"],
        vec!["deposits"],
    );
    assert!(check_model(&m).is_ok());
}

#[test]
fn single_sector_with_zero_sum() {
    // One sector, one + and one -.
    // | Account | Sector |
    // | a       | +      |   sum: +1
    // | b       | -      |   sum: -1
    // Column Sector: +1 + (-1) = 0 ✓
    let m = model_godley(
        vec![plus("a", "Sector"), minus("b", "Sector")],
        vec!["Sector"],
        vec!["a", "b"],
    );
    assert!(check_model(&m).is_ok());
}

// ── Failing cases ─────────────────────────────────────────────────────────────

#[test]
fn single_plus_imbalanced() {
    // One sector, one + entry, nothing to cancel it.
    let m = model_godley(
        vec![plus("deposits", "Households")],
        vec!["Households"],
        vec!["deposits"],
    );
    let Err(errors) = check_model(&m) else {
        panic!("expected Err, got Ok");
    };
    assert!(
        errors.iter().any(|e| matches!(
            e,
            CheckError::GodleyImbalance { sector, net, .. }
            if sector == "Households" && *net == 1
        )),
        "expected GodleyImbalance for Households with net=+1, got: {errors:?}"
    );
}

#[test]
fn single_minus_imbalanced() {
    let m = model_godley(
        vec![minus("deposits", "Banks")],
        vec!["Banks"],
        vec!["deposits"],
    );
    let Err(errors) = check_model(&m) else {
        panic!("expected Err, got Ok");
    };
    assert!(errors.iter().any(|e| matches!(
        e,
        CheckError::GodleyImbalance { sector, net, .. }
        if sector == "Banks" && *net == -1
    )));
}

#[test]
fn two_sectors_one_imbalanced() {
    // Households is balanced; Banks has an extra + with no matching -.
    // | Account     | Households | Banks |
    // | consumption | -          | +     |   H:-1, B:+1
    // | wages       | +          | -     |   H:+1, B:-1
    // | interest    |            | +     |   B:+1  ← extra, breaks Banks
    // Column sums: H=0 ✓, B=+1 ✗
    let m = model_godley(
        vec![
            minus("consumption", "Households"),
            plus("consumption", "Banks"),
            plus("wages", "Households"),
            minus("wages", "Banks"),
            plus("interest", "Banks"), // deliberate imbalance
        ],
        vec!["Households", "Banks"],
        vec!["consumption", "wages", "interest"],
    );
    let Err(errors) = check_model(&m) else {
        panic!("expected Err, got Ok");
    };
    // Exactly one sector imbalanced.
    let godley_errors: Vec<_> = errors
        .iter()
        .filter(|e| matches!(e, CheckError::GodleyImbalance { .. }))
        .collect();
    assert_eq!(godley_errors.len(), 1);
    assert!(godley_errors.iter().any(|e| matches!(
        e,
        CheckError::GodleyImbalance { sector, net, .. }
        if sector == "Banks" && *net == 1
    )));
}

#[test]
fn both_sectors_imbalanced_both_reported() {
    // Two sectors, each with only a single cell → neither can sum to zero.
    // | Account   | Alice | Bob |
    // | thing     | +     | -   |   Alice:+1, Bob:-1
    // Neither is 0; both should be reported.
    let m = model_godley(
        vec![plus("thing", "Alice"), minus("thing", "Bob")],
        vec!["Alice", "Bob"],
        vec!["thing"],
    );
    let Err(errors) = check_model(&m) else {
        panic!("expected Err, got Ok");
    };
    let godley_errors: Vec<_> = errors
        .iter()
        .filter(|e| matches!(e, CheckError::GodleyImbalance { .. }))
        .collect();
    assert_eq!(
        godley_errors.len(),
        2,
        "both sectors should be flagged: {errors:?}"
    );
}

#[test]
fn net_two_excess_reported_correctly() {
    // Three + and one - in the same column → net = +2.
    let m = model_godley(
        vec![
            plus("a", "S"),
            plus("b", "S"),
            plus("c", "S"),
            minus("d", "S"),
        ],
        vec!["S"],
        vec!["a", "b", "c", "d"],
    );
    let Err(errors) = check_model(&m) else {
        panic!("expected Err, got Ok");
    };
    assert!(
        errors.iter().any(|e| matches!(
            e,
            CheckError::GodleyImbalance { sector, net, .. }
            if sector == "S" && *net == 2
        )),
        "expected net=+2 for sector S, got: {errors:?}"
    );
}
