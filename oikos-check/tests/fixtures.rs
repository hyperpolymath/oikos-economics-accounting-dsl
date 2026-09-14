// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Test fixture builders shared across integration test files.
//!
//! All builders use `Span::SYNTHETIC` throughout; spans are irrelevant
//! for the structural checks under test.

#![allow(dead_code)] // individual test files use subsets of these helpers

use oikos_syntax::{
    account::{AccountDecl, AccountKind},
    dimension::CurrencyCode,
    godley::{GodleyCell, GodleyMatrix, GodleySign},
    instrument::{InstrumentDecl, InstrumentState, StateTransition},
    model::Model,
    sector::AccountRef,
    span::Span,
};
use smol_str::SmolStr;

// ── Span shorthand ────────────────────────────────────────────────────────────

pub fn s() -> Span {
    Span::SYNTHETIC
}

// ── Account / sector helpers ──────────────────────────────────────────────────

pub fn account_ref(name: &str) -> AccountRef {
    AccountRef {
        name: name.into(),
        span: s(),
    }
}

pub fn gbp() -> CurrencyCode {
    CurrencyCode {
        code: "GBP".into(),
        span: s(),
    }
}

pub fn stock(name: &str) -> AccountDecl {
    AccountDecl {
        name: name.into(),
        kind: AccountKind::Stock,
        currency: gbp(),
        description: None,
        span: s(),
    }
}

// ── Godley helpers ────────────────────────────────────────────────────────────

pub fn cell(account: &str, sector: &str, sign: GodleySign) -> GodleyCell {
    GodleyCell {
        account: account_ref(account),
        sector: SmolStr::from(sector),
        sign,
        span: s(),
    }
}

pub fn plus(account: &str, sector: &str) -> GodleyCell {
    cell(account, sector, GodleySign::Plus)
}

pub fn minus(account: &str, sector: &str) -> GodleyCell {
    cell(account, sector, GodleySign::Minus)
}

/// Build a model containing only the given Godley matrix.
/// All other fields are empty/synthetic.
pub fn model_godley(cells: Vec<GodleyCell>, sectors: Vec<&str>, accounts: Vec<&str>) -> Model {
    Model {
        name: "TestModel".into(),
        active_period: "P".into(),
        periods: vec![],
        fx_rates: vec![],
        accounts: vec![],
        instruments: vec![],
        sectors: vec![],
        godley: GodleyMatrix {
            cells,
            sectors: sectors.iter().map(|s| SmolStr::from(*s)).collect(),
            accounts: accounts.iter().map(|a| account_ref(a)).collect(),
            span: s(),
        },
        body: vec![],
        span: s(),
    }
}

/// Minimal empty Godley matrix (no rows, no columns).
pub fn empty_godley() -> GodleyMatrix {
    GodleyMatrix {
        cells: vec![],
        sectors: vec![],
        accounts: vec![],
        span: s(),
    }
}

// ── Instrument helpers ────────────────────────────────────────────────────────

pub fn transition(from: InstrumentState, to: InstrumentState) -> StateTransition {
    StateTransition {
        from,
        to,
        span: s(),
    }
}

pub fn instrument(
    name: &str,
    states: Vec<InstrumentState>,
    initial: InstrumentState,
    transitions: Vec<StateTransition>,
) -> InstrumentDecl {
    InstrumentDecl {
        name: name.into(),
        currency: gbp(),
        states,
        initial,
        transitions,
        description: None,
        span: s(),
    }
}

/// Build a model containing only the given instrument declarations.
pub fn model_instruments(instruments: Vec<InstrumentDecl>) -> Model {
    Model {
        name: "TestModel".into(),
        active_period: "P".into(),
        periods: vec![],
        fx_rates: vec![],
        accounts: vec![],
        instruments,
        sectors: vec![],
        godley: empty_godley(),
        body: vec![],
        span: s(),
    }
}

// ── Canned instrument lifecycles ──────────────────────────────────────────────

/// Standard invoice lifecycle:
/// Draft → Issued → PartiallyPaid → Settled
///       ↘ Void     ↘ Void           ↘ Void
pub fn invoice() -> InstrumentDecl {
    use InstrumentState::*;
    instrument(
        "Invoice",
        vec![Draft, Issued, PartiallyPaid, Settled, Void],
        Draft,
        vec![
            transition(Draft, Issued),
            transition(Draft, Void),
            transition(Issued, PartiallyPaid),
            transition(Issued, Void),
            transition(PartiallyPaid, Settled),
            transition(PartiallyPaid, Void),
        ],
    )
}

/// A simple two-step instrument: Draft → Settled | Void
pub fn simple_bond() -> InstrumentDecl {
    use InstrumentState::*;
    instrument(
        "Bond",
        vec![Draft, Settled, Void],
        Draft,
        vec![transition(Draft, Settled), transition(Draft, Void)],
    )
}
