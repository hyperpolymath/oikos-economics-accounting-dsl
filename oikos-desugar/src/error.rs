// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Error types for the desugaring pass.

use thiserror::Error;

/// An error produced during desugaring from Oikos AST to Ephapax IR.
#[derive(Debug, Error, PartialEq)]
pub enum DesugarError {
    /// The Ephapax IR dependency is not yet available (compile-time blocker).
    ///
    /// Remove this variant once `ephapax-ir` is wired into `Cargo.toml`.
    #[error("desugaring blocked: Ephapax IR dependency not yet available")]
    EphapaxNotAvailable,

    /// The Godley matrix column for `sector` does not sum to zero.
    ///
    /// This is a hard compile error; models that violate double-entry
    /// bookkeeping are rejected before any IR is generated.
    #[error(
        "Godley matrix column for sector `{sector}` does not sum to zero \
         (net: {net:+}); every sector's liabilities must equal another \
         sector's assets"
    )]
    GodleyColumnImbalance { sector: String, net: i64 },

    /// A `transfer` expression references an account that does not exist.
    #[error("transfer references unknown account `{name}`")]
    UnknownAccount { name: String },

    /// A `convert` expression references an undeclared FX rate.
    #[error("FX conversion references undeclared rate `{rate}`")]
    UnknownFxRate { rate: String },

    /// A currency dimension mismatch was detected during desugaring.
    ///
    /// In the fully wired Ephapax backend this would be a type error; this
    /// variant exists for early detection before the IR is available.
    #[error("currency mismatch: cannot combine {lhs} and {rhs} without FX conversion")]
    CurrencyMismatch { lhs: String, rhs: String },

    /// An instrument state transition is not declared in the typestate machine.
    #[error("instrument `{instrument}` has no transition from `{from}` to `{to}`")]
    InvalidStateTransition {
        instrument: String,
        from: String,
        to: String,
    },
}
