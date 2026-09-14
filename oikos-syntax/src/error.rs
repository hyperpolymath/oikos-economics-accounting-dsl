// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Syntax-level error type for `oikos-syntax`.
//!
//! These errors arise during AST construction (e.g. invalid literal formats)
//! rather than during parsing; parse errors are defined in `oikos-parser`.

use thiserror::Error;

/// An error that can be produced when constructing or validating AST nodes.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SyntaxError {
    /// A currency code is not a valid ISO 4217 three-letter alphabetic code.
    #[error("invalid currency code `{code}`: expected three uppercase ASCII letters")]
    InvalidCurrencyCode { code: String },

    /// A monetary amount could not be parsed as a valid decimal.
    #[error("invalid monetary amount `{raw}`: {reason}")]
    InvalidAmount { raw: String, reason: String },

    /// A date string could not be parsed as ISO 8601 (YYYY-MM-DD).
    #[error("invalid date `{raw}`: expected YYYY-MM-DD format")]
    InvalidDate { raw: String },

    /// A period's end date precedes its start date.
    #[error("period `{name}` ends ({to}) before it starts ({from})")]
    PeriodEndBeforeStart {
        name: String,
        from: String,
        to: String,
    },
}
