// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Error types for `oikos-check`.

use thiserror::Error;

use oikos_desugar::DesugarError;
use oikos_syntax::span::Span;

/// A type-check error produced by `oikos-check`.
#[derive(Debug, Error)]
pub enum CheckError {
    // ── Structural invariants ─────────────────────────────────────────────────
    /// The Godley matrix column for a sector does not sum to zero.
    #[error("accounting identity violated: sector `{sector}` column sums to {net:+}, not zero")]
    GodleyImbalance {
        sector: String,
        net: i64,
        span: Span,
    },

    /// An instrument typestate machine contains an unreachable state.
    #[error("instrument `{instrument}`: state `{state}` is unreachable from initial state")]
    UnreachableState {
        instrument: String,
        state: String,
        span: Span,
    },

    /// An instrument typestate machine has no path from a non-terminal state
    /// to any terminal state (Settled or Void).
    #[error("instrument `{instrument}`: state `{state}` has no path to a terminal state")]
    NonTerminatingState {
        instrument: String,
        state: String,
        span: Span,
    },

    // ── Name resolution ───────────────────────────────────────────────────────
    /// A name is declared more than once in the same namespace.
    #[error("duplicate {kind} name `{name}`")]
    DuplicateName {
        kind: String,
        name: String,
        span: Span,
    },

    /// A reference to an account that was not declared.
    #[error("unknown account `{name}`")]
    UnknownAccount { name: String, span: Span },

    /// A reference to a sector that was not declared.
    #[error("unknown sector `{name}`")]
    UnknownSector { name: String, span: Span },

    /// A reference to a period that was not declared.
    #[error("unknown period `{name}`")]
    UnknownPeriod { name: String, span: Span },

    /// A reference to an FX rate that was not declared.
    #[error("unknown FX rate `{name}`")]
    UnknownFxRate { name: String, span: Span },

    /// The `active_period` in the model header names no declared period.
    #[error("model active period `{name}` is not declared in this model")]
    UnknownActivePeriod { name: String, span: Span },

    // ── Currency / dimension checks ───────────────────────────────────────────
    /// A transfer moves money between accounts denominated in different currencies.
    #[error(
        "currency mismatch in transfer `{from_account}` → `{to_account}`: \
         source is {from_currency}, destination is {to_currency}"
    )]
    TransferCurrencyMismatch {
        from_account: String,
        to_account: String,
        from_currency: String,
        to_currency: String,
        span: Span,
    },

    /// The amount currency in a transfer does not match the source account currency.
    #[error(
        "amount currency `{amount_currency}` does not match source account \
         `{account}` currency `{account_currency}`"
    )]
    AmountCurrencyMismatch {
        account: String,
        account_currency: String,
        amount_currency: String,
        span: Span,
    },

    /// Generic dimension mismatch (used by the Ephapax desugar pass).
    #[error("dimension mismatch: cannot combine `{lhs}` and `{rhs}`")]
    DimensionMismatch {
        lhs: String,
        rhs: String,
        span: Span,
    },

    // ── Desugaring ────────────────────────────────────────────────────────────
    /// An error propagated from the desugaring pass.
    #[error("desugaring error: {0}")]
    Desugar(#[from] DesugarError),
}
