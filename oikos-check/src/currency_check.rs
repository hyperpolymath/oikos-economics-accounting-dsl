// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Currency-dimension checker.
//!
//! Enforces two rules for every `transfer` expression:
//!
//! 1. The source and destination accounts must be denominated in the same
//!    currency.  Mixing GBP and USD without an explicit `convert` is an error.
//!
//! 2. The literal amount (if present) must be denominated in the same currency
//!    as the source account.
//!
//! Accounts that cannot be resolved (caught by `cross_ref`) are silently
//! skipped here to avoid duplicate errors.

use oikos_syntax::{
    expr::{Expr, MoneyExpr},
    Model,
};

use crate::{error::CheckError, resolve::SymbolTable};

/// Run currency checks on all transfer expressions.
pub fn check(model: &Model, symbols: &SymbolTable<'_>) -> Vec<CheckError> {
    let mut errors: Vec<CheckError> = Vec::new();

    for expr in &model.body {
        let Expr::Transfer(t) = expr else { continue };

        let from_decl = symbols.account(t.from.name.as_str());
        let to_decl = symbols.account(t.to.name.as_str());

        // Rule 1: source and destination currencies must match.
        if let (Some(from), Some(to)) = (from_decl, to_decl) {
            if from.currency.code != to.currency.code {
                errors.push(CheckError::TransferCurrencyMismatch {
                    from_account: t.from.name.to_string(),
                    to_account: t.to.name.to_string(),
                    from_currency: from.currency.code.to_string(),
                    to_currency: to.currency.code.to_string(),
                    span: t.span,
                });
            }

            // Rule 2: literal amount currency must match source account.
            if let MoneyExpr::Literal(lit) = &t.amount {
                if lit.currency.code != from.currency.code {
                    errors.push(CheckError::AmountCurrencyMismatch {
                        account: t.from.name.to_string(),
                        account_currency: from.currency.code.to_string(),
                        amount_currency: lit.currency.code.to_string(),
                        span: lit.span,
                    });
                }
            }
        }
    }

    errors
}
