// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! `oikos-desugar` — desugaring pass from Oikos AST to Ephapax IR.
//!
//! # Status: unblocked upstream, not yet wired
//!
//! **The originally recorded blocker has cleared.** Issue #60 and this
//! comment both stated that this crate was blocked on the Ephapax
//! surface-language parser. As verified on 2026-09-26 against
//! `hyperpolymath/ephapax@main`, that milestone shipped: the upstream
//! `ROADMAP.adoc` status snapshot records the "Lexer, parser, interpreter,
//! REPL, CLI, S-expression IR, two-phase pipeline, Zig FFI, and conformance
//! test suite" as complete, and the workspace now publishes `ephapax-parser`,
//! `ephapax-lexer`, `ephapax-surface`, `ephapax-syntax`, `ephapax-ir` and
//! `ephapax-typing` crates. The same snapshot records the **type checker** and
//! WASM code generation as still in progress.
//!
//! So the dependency is commented out in `Cargo.toml` no longer because the
//! parser is missing, but because the `ephapax-*` path dependencies have not
//! been wired into this workspace and the lowering has not been implemented
//! or tested. `desugar` therefore still returns
//! [`DesugarError::EphapaxNotAvailable`], and **no claim of a working
//! lowering is made here** — this crate has zero tests. See issue #60 for the
//! acceptance criteria.
//!
//! # Desugaring map
//!
//! | Oikos construct          | Ephapax IR target                                  |
//! |--------------------------|-----------------------------------------------------|
//! | `Stock GBP`              | Linear value with phantom type `Gbp`                |
//! | `Flow GBP`               | Linear value with phantom type `Flow<Gbp>`          |
//! | Fiscal period            | Tofte–Talpin region                                 |
//! | `transfer A → B`         | `let v = consume(A); produce(B, v)`                 |
//! | `convert … via rate`     | `let v = consume(A); produce(B, fx(v, rate))`       |
//! | `close A from P into Q`  | Region-end finaliser + new-region initialiser        |
//! | Instrument typestate     | Ephapax typestate machine                           |
//! | Godley matrix invariant  | Module-level linear type constraint                 |

pub mod error;

pub use error::DesugarError;
use oikos_syntax::Model;

/// Placeholder for the Ephapax IR module root.
///
/// Replace with `use ephapax_ir::Module;` once the dependency is available.
pub struct EphapaxIr; // placeholder

/// Desugar an Oikos [`Model`] into an Ephapax IR module.
///
/// # Errors
///
/// Returns [`DesugarError::EphapaxNotAvailable`] until the Ephapax IR
/// dependency is wired in.  Other error variants cover semantic problems
/// detected during desugaring (e.g. Godley column imbalance).
pub fn desugar(_model: &Model) -> Result<EphapaxIr, DesugarError> {
    Err(DesugarError::EphapaxNotAvailable)
}
