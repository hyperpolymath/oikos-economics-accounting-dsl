// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Symbol-table construction and duplicate-name detection.
//!
//! Builds four lookup maps (accounts, periods, sectors, instruments, fx_rates)
//! from the model's declarations.  Duplicate names within the same namespace
//! are reported as `CheckError::DuplicateName`.

use std::collections::HashMap;

use smol_str::SmolStr;

use oikos_syntax::{
    account::AccountDecl, dimension::FxRate, instrument::InstrumentDecl, period::PeriodDecl,
    sector::SectorDecl, span::Span, Model,
};

use crate::error::CheckError;

/// All declared names from a single model, keyed for O(1) lookup.
pub struct SymbolTable<'m> {
    pub accounts: HashMap<&'m str, &'m AccountDecl>,
    pub periods: HashMap<&'m str, &'m PeriodDecl>,
    pub sectors: HashMap<&'m str, &'m SectorDecl>,
    pub instruments: HashMap<&'m str, &'m InstrumentDecl>,
    pub fx_rates: HashMap<&'m str, &'m FxRate>,
}

impl<'m> SymbolTable<'m> {
    pub fn account(&self, name: &str) -> Option<&'m AccountDecl> {
        self.accounts.get(name).copied()
    }

    pub fn period(&self, name: &str) -> Option<&'m PeriodDecl> {
        self.periods.get(name).copied()
    }

    pub fn sector(&self, name: &str) -> Option<&'m SectorDecl> {
        self.sectors.get(name).copied()
    }

    pub fn fx_rate(&self, name: &str) -> Option<&'m FxRate> {
        self.fx_rates.get(name).copied()
    }
}

/// Build a `SymbolTable` from a model, collecting `DuplicateName` errors.
///
/// The table is always returned (even when errors are present) so that
/// subsequent passes can continue with partial information.
pub fn build(model: &Model) -> (SymbolTable<'_>, Vec<CheckError>) {
    let mut errors: Vec<CheckError> = Vec::new();

    let accounts = collect(&model.accounts, |d| d.name.as_str(), "account", &mut errors);
    let periods = collect(&model.periods, |d| d.name.as_str(), "period", &mut errors);
    let sectors = collect(&model.sectors, |d| d.name.as_str(), "sector", &mut errors);
    let instruments = collect(
        &model.instruments,
        |d| d.name.as_str(),
        "instrument",
        &mut errors,
    );
    let fx_rates = collect(&model.fx_rates, |d| d.name.as_str(), "rate", &mut errors);

    (
        SymbolTable {
            accounts,
            periods,
            sectors,
            instruments,
            fx_rates,
        },
        errors,
    )
}

/// Generic helper: insert declarations into a map, reporting duplicates.
fn collect<'m, T>(
    decls: &'m [T],
    key_fn: impl Fn(&'m T) -> &'m str,
    kind: &'static str,
    errors: &mut Vec<CheckError>,
) -> HashMap<&'m str, &'m T> {
    let mut map: HashMap<&'m str, &'m T> = HashMap::new();
    for decl in decls {
        let key = key_fn(decl);
        if let Some(_prev) = map.insert(key, decl) {
            errors.push(CheckError::DuplicateName {
                kind: kind.to_owned(),
                name: key.to_owned(),
                span: Span::SYNTHETIC,
            });
        }
    }
    map
}

// Satisfy the borrow checker for SmolStr-keyed collections when the key function
// returns a &str that is derived from a SmolStr field.
impl SymbolTable<'_> {
    /// Check whether a given account name is known.
    pub fn has_account(&self, name: &SmolStr) -> bool {
        self.accounts.contains_key(name.as_str())
    }
}
