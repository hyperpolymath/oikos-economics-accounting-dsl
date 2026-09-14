// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! `oikos-check` — pre-desugar type checker for the Oikos DSL.
//!
//! # Checking pipeline
//!
//! ```text
//! oikos_syntax::Model
//!   └─▶ oikos_check::check_model()
//!         ├─▶ Phase 1: symbol table + duplicate-name detection  (resolve)
//!         ├─▶ Phase 2: cross-reference resolution               (cross_ref)
//!         ├─▶ Phase 3: currency-dimension compatibility         (currency_check)
//!         ├─▶ Phase 4: Godley column-sum invariant              (godley_check)
//!         ├─▶ Phase 5: instrument typestate reachability        (instrument_check)
//!         └─▶ Phase 6: desugar → Ephapax type checking  ← blocked on Ephapax parser
//! ```
//!
//! Phases 1–5 run entirely on the `oikos_syntax::Model` AST without
//! desugaring.  Phase 6 is stubbed until `ephapax-typing` ships.

pub mod cross_ref;
pub mod currency_check;
pub mod error;
pub mod godley_check;
pub mod instrument_check;
pub mod resolve;

pub use error::CheckError;
use oikos_syntax::Model;

/// Run all pre-desugar Oikos checks on a parsed model.
///
/// All phases run and accumulate errors rather than short-circuiting on the
/// first failure, so the caller receives a complete diagnostic list.
/// Returns `Ok(())` only when every phase passes.
pub fn check_model(model: &Model) -> Result<(), Vec<CheckError>> {
    let mut errors: Vec<CheckError> = Vec::new();

    // Phase 1 — build the symbol table; catch duplicate declarations.
    let (symbols, resolve_errors) = resolve::build(model);
    errors.extend(resolve_errors);

    // Phase 2 — cross-reference resolution.
    errors.extend(cross_ref::check(model, &symbols));

    // Phase 3 — currency-dimension compatibility.
    errors.extend(currency_check::check(model, &symbols));

    // Phase 4 — Godley column-sum invariant.
    if let Err(godley_errors) = godley_check::check(model) {
        errors.extend(godley_errors);
    }

    // Phase 5 — instrument typestate reachability.
    if let Err(inst_errors) = instrument_check::check(model) {
        errors.extend(inst_errors);
    }

    // Phase 6 — desugar to Ephapax IR + Ephapax type checking (blocked).
    // Uncomment once ephapax-typing is available as a crate dependency:
    //
    // match oikos_desugar::desugar(model) {
    //     Ok(ir) => {
    //         if let Err(e) = ephapax_typing::check(&ir) {
    //             errors.push(CheckError::EphapaxTypeError(e));
    //         }
    //     }
    //     Err(e) => errors.push(CheckError::Desugar(e)),
    // }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
