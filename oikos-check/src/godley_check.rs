// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Godley matrix column-sum invariant checker.
//!
//! For each sector column in the Godley matrix, the signed sum of all entries
//! must equal zero.  A `+` counts as +1, a `-` as -1, an empty cell as 0.
//! (The actual amounts are checked during desugaring; this pass checks the
//! sign-pattern structure, which is sufficient to catch most accounting errors
//! at parse time.)

use std::collections::HashMap;

use oikos_syntax::{godley::GodleySign, Model};

use crate::error::CheckError;

/// Check the Godley matrix column-sum invariant.
///
/// Returns `Ok(())` when all columns sum to zero.
/// Returns `Err(errors)` with one error per imbalanced column.
pub fn check(model: &Model) -> Result<(), Vec<CheckError>> {
    let mut column_sums: HashMap<&str, i64> = HashMap::new();

    for sector in &model.godley.sectors {
        column_sums.insert(sector.as_str(), 0);
    }

    for cell in &model.godley.cells {
        let delta = match cell.sign {
            GodleySign::Plus => 1,
            GodleySign::Minus => -1,
            GodleySign::Zero => 0,
        };
        *column_sums.entry(cell.sector.as_str()).or_insert(0) += delta;
    }

    let errors: Vec<CheckError> = column_sums
        .into_iter()
        .filter(|(_, net)| *net != 0)
        .map(|(sector, net)| CheckError::GodleyImbalance {
            sector: sector.to_owned(),
            net,
            span: model.godley.span,
        })
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
