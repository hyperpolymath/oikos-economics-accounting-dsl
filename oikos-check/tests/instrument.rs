// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Integration tests for the instrument typestate reachability checker.
//!
//! The checker verifies two properties for each instrument:
//! 1. Every declared state is reachable from the initial state.
//! 2. Every non-terminal state has at least one path to a terminal state
//!    (Settled or Void).

mod fixtures;
use fixtures::*;

use oikos_check::{check_model, CheckError};
use oikos_syntax::instrument::InstrumentState;

// ── Passing cases ─────────────────────────────────────────────────────────────

#[test]
fn no_instruments_passes() {
    // Nothing to check — should pass trivially.
    let m = model_instruments(vec![]);
    assert!(check_model(&m).is_ok());
}

#[test]
fn invoice_full_lifecycle_passes() {
    // Standard invoice: all states reachable; all non-terminal have a path to
    // Settled or Void.
    let m = model_instruments(vec![invoice()]);
    assert!(
        check_model(&m).is_ok(),
        "{:?}",
        check_model(&m).unwrap_err()
    );
}

#[test]
fn simple_bond_passes() {
    // Draft → Settled | Void — minimal valid instrument.
    let m = model_instruments(vec![simple_bond()]);
    assert!(check_model(&m).is_ok());
}

#[test]
fn initial_state_is_terminal_passes() {
    // An instrument that starts already settled (unusual but valid —
    // e.g. a pre-executed contract record).
    let inst = instrument(
        "PreSettled",
        vec![InstrumentState::Settled],
        InstrumentState::Settled,
        vec![],
    );
    let m = model_instruments(vec![inst]);
    assert!(check_model(&m).is_ok());
}

#[test]
fn multiple_valid_instruments_all_pass() {
    let m = model_instruments(vec![invoice(), simple_bond()]);
    assert!(check_model(&m).is_ok());
}

#[test]
fn linear_chain_draft_to_settled() {
    // Draft → Issued → Settled — simplest non-trivial lifecycle.
    use InstrumentState::*;
    let inst = instrument(
        "SimpleInvoice",
        vec![Draft, Issued, Settled],
        Draft,
        vec![transition(Draft, Issued), transition(Issued, Settled)],
    );
    let m = model_instruments(vec![inst]);
    assert!(check_model(&m).is_ok());
}

// ── Unreachable state ─────────────────────────────────────────────────────────

#[test]
fn declared_state_not_reachable_from_initial() {
    // PartiallyPaid is declared but no transition leads to it.
    // Draft → Issued → Settled, but PartiallyPaid is stranded.
    use InstrumentState::*;
    let inst = instrument(
        "BrokenInvoice",
        vec![Draft, Issued, PartiallyPaid, Settled],
        Draft,
        vec![
            transition(Draft, Issued),
            transition(Issued, Settled),
            // PartiallyPaid has no incoming transition — unreachable.
        ],
    );
    let m = model_instruments(vec![inst]);
    let Err(errors) = check_model(&m) else {
        panic!("expected Err, got Ok");
    };
    assert!(
        errors.iter().any(|e| matches!(
            e,
            CheckError::UnreachableState { instrument, state, .. }
            if instrument == "BrokenInvoice" && state.contains("PartiallyPaid")
        )),
        "expected UnreachableState for PartiallyPaid, got: {errors:?}"
    );
}

#[test]
fn multiple_unreachable_states_all_reported() {
    // Both PartiallyPaid and Void are unreachable from Draft.
    use InstrumentState::*;
    let inst = instrument(
        "DoubleBroken",
        vec![Draft, PartiallyPaid, Settled, Void],
        Draft,
        vec![transition(Draft, Settled)], // only Draft → Settled wired up
    );
    let m = model_instruments(vec![inst]);
    let Err(errors) = check_model(&m) else {
        panic!("expected Err, got Ok");
    };
    let unreachable: Vec<_> = errors
        .iter()
        .filter(|e| matches!(e, CheckError::UnreachableState { .. }))
        .collect();
    assert_eq!(
        unreachable.len(),
        2,
        "both PartiallyPaid and Void should be flagged: {errors:?}"
    );
}

// ── Non-terminating state ─────────────────────────────────────────────────────

#[test]
fn cycle_with_no_exit_is_non_terminating() {
    // Issued ↔ PartiallyPaid cycle with no path to Settled or Void.
    // Draft → Issued, Issued ↔ PartiallyPaid — neither can settle.
    use InstrumentState::*;
    let inst = instrument(
        "CyclicInvoice",
        vec![Draft, Issued, PartiallyPaid, Settled],
        Draft,
        vec![
            transition(Draft, Issued),
            transition(Issued, PartiallyPaid),
            transition(PartiallyPaid, Issued), // cycle back — no exit
                                               // Settled is declared but unreachable; Issued + PartiallyPaid
                                               // are both non-terminating.
        ],
    );
    let m = model_instruments(vec![inst]);
    let Err(errors) = check_model(&m) else {
        panic!("expected Err, got Ok");
    };
    // Settled is also unreachable; Issued and PartiallyPaid are non-terminating.
    // We don't prescribe the exact error count, but at least one
    // NonTerminatingState must be present.
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, CheckError::NonTerminatingState { .. })),
        "expected at least one NonTerminatingState error, got: {errors:?}"
    );
}

#[test]
fn non_terminating_draft_with_no_transitions() {
    // Draft is the initial state, not terminal, and has no transitions.
    // It cannot reach Settled or Void → non-terminating.
    use InstrumentState::*;
    let inst = instrument(
        "StuckDraft",
        vec![Draft],
        Draft,
        vec![], // no transitions at all
    );
    let m = model_instruments(vec![inst]);
    let Err(errors) = check_model(&m) else {
        panic!("expected Err, got Ok");
    };
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, CheckError::NonTerminatingState { .. })),
        "Draft with no transitions should be non-terminating: {errors:?}"
    );
}

// ── Combined: multiple instruments, mixed validity ────────────────────────────

#[test]
fn valid_and_invalid_instruments_both_reported() {
    // invoice() is valid; BrokenInvoice has an unreachable state.
    // Errors from BrokenInvoice should appear even though invoice() passes.
    use InstrumentState::*;
    let broken = instrument(
        "BrokenBond",
        vec![Draft, PartiallyPaid, Settled],
        Draft,
        vec![transition(Draft, Settled)], // PartiallyPaid unreachable
    );
    let m = model_instruments(vec![invoice(), broken]);
    let Err(errors) = check_model(&m) else {
        panic!("expected Err, got Ok");
    };
    assert!(
        errors.iter().any(|e| matches!(
            e,
            CheckError::UnreachableState { instrument, .. }
            if instrument == "BrokenBond"
        )),
        "expected UnreachableState for BrokenBond: {errors:?}"
    );
    // No errors should mention the valid invoice.
    assert!(
        !errors.iter().any(|e| match e {
            CheckError::UnreachableState { instrument, .. }
            | CheckError::NonTerminatingState { instrument, .. } => instrument == "Invoice",
            _ => false,
        }),
        "Invoice should have no errors: {errors:?}"
    );
}
