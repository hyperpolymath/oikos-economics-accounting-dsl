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

#[cfg(test)]
mod tests {
    use super::*;

    // The diagnostics below are the crate's public contract while the lowering
    // is unimplemented (#60): callers match on `Display` text as well as on the
    // variant, and every string asserted here is copied verbatim from the
    // `#[error(...)]` attribute above it. If a message must change, change it
    // deliberately and change the test in the same commit — a diagnostic that
    // drifts silently is a diagnostic nobody can grep for.

    #[test]
    fn ephapax_not_available_message_is_stable() {
        let msg = DesugarError::EphapaxNotAvailable.to_string();
        let want = "desugaring blocked: Ephapax IR dependency not yet available";
        assert_eq!(msg, want);
    }

    #[test]
    fn godley_imbalance_message_carries_sector_and_signed_net() {
        let err = DesugarError::GodleyColumnImbalance {
            sector: "Households".to_string(),
            net: -3,
        };
        let msg = err.to_string();
        assert!(msg.contains("`Households`"), "no sector in: {msg}");
        assert!(msg.contains("does not sum to zero"), "no premise in: {msg}");
        assert!(msg.contains("(net: -3)"), "no signed net in: {msg}");
    }

    #[test]
    fn unknown_account_message_names_the_account() {
        let err = DesugarError::UnknownAccount {
            name: "ghost".to_string(),
        };
        let msg = err.to_string();
        let want = "transfer references unknown account `ghost`";
        assert_eq!(msg, want);
    }

    #[test]
    fn unknown_fx_rate_message_names_the_rate() {
        let err = DesugarError::UnknownFxRate {
            rate: "gbp_usd".to_string(),
        };
        let msg = err.to_string();
        let want = "FX conversion references undeclared rate `gbp_usd`";
        assert_eq!(msg, want);
    }

    #[test]
    fn currency_mismatch_message_carries_both_currencies() {
        let err = DesugarError::CurrencyMismatch {
            lhs: "GBP".to_string(),
            rhs: "USD".to_string(),
        };
        let msg = err.to_string();
        let want = "currency mismatch: cannot combine GBP and USD without FX conversion";
        assert_eq!(msg, want);
    }

    #[test]
    fn invalid_state_transition_message_names_instrument_and_states() {
        let err = DesugarError::InvalidStateTransition {
            instrument: "bond".to_string(),
            from: "Issued".to_string(),
            to: "Matured".to_string(),
        };
        let msg = err.to_string();
        let want = "instrument `bond` has no transition from `Issued` to `Matured`";
        assert_eq!(msg, want);
    }
}
