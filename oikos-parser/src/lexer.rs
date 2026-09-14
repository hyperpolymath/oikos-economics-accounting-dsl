// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Logos lexer for the Oikos DSL.
//!
//! Longest-match rules (Logos guarantees):
//! - `2025-04-01` → `Date` (10 chars) beats `Number` (4 chars) + `Minus`.
//! - `model` → `KwModel` (`#[token]` beats `#[regex]` at equal length).
//! - `model_x` → `Ident` (7 chars) beats `KwModel` (5 chars).
//!
//! Value-carrying variants (`Ident`, `Number`, `Date`, `StringLit`) store
//! the matched text as a `SmolStr` so the parser need not reach back into
//! the source string.

use logos::Logos;
use smol_str::SmolStr;

/// The token stream produced by the Logos lexer.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]
// skip whitespace
// `[^\n]*` is intentionally greedy here — a line comment runs to the
// next newline by definition. logos 0.16 added a heuristic warning for
// unbounded greedy repetitions; `allow_greedy = true` opts in.
#[logos(skip("--[^\n]*", allow_greedy = true))]
pub enum Token {
    // ── Keywords (unit variants; #[token] beats #[regex] at equal length) ────
    #[token("model")]
    KwModel,
    #[token("period")]
    KwPeriod,
    #[token("rate")]
    KwRate,
    #[token("account")]
    KwAccount,
    #[token("sector")]
    KwSector,
    #[token("instrument")]
    KwInstrument,
    #[token("godley")]
    KwGodley,
    #[token("transfer")]
    KwTransfer,
    #[token("convert")]
    KwConvert,
    #[token("close")]
    KwClose,
    #[token("from")]
    KwFrom,
    #[token("to")]
    KwTo,
    #[token("into")]
    KwInto,
    #[token("via")]
    KwVia,
    #[token("on")]
    KwOn,
    #[token("states")]
    KwStates,
    #[token("initial")]
    KwInitial,
    #[token("transitions")]
    KwTransitions,
    #[token("asset")]
    KwAsset,
    #[token("liability")]
    KwLiability,
    #[token("amount")]
    KwAmount,
    #[token("description")]
    KwDescription,
    #[token("balance")]
    KwBalance,
    #[token("fraction")]
    KwFraction,

    // ── Account kinds ─────────────────────────────────────────────────────────
    #[token("Stock")]
    KwStock,
    #[token("Flow")]
    KwFlow,

    // ── Period kinds ──────────────────────────────────────────────────────────
    #[token("FiscalYear")]
    KwFiscalYear,
    #[token("HalfYear")]
    KwHalfYear,
    #[token("Quarter")]
    KwQuarter,
    #[token("Month")]
    KwMonth,
    #[token("Week")]
    KwWeek,

    // ── Arithmetic signs (for Godley matrix cells) ────────────────────────────
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,

    // ── Punctuation ───────────────────────────────────────────────────────────
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("|")]
    Pipe,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token("→")]
    Arrow, // U+2192
    #[token("->")]
    AsciiArrow, // ASCII fallback

    // ── Literals (value-carrying) ─────────────────────────────────────────────
    /// ISO 8601 date.  Must be tried before `Number` to get longest match.
    /// The Logos longest-match rule handles this: `\d{4}-\d{2}-\d{2}` wins
    /// over `[0-9][0-9_]*` for input `2025-04-01`.
    #[regex(r"\d{4}-\d{2}-\d{2}", |lex| SmolStr::from(lex.slice()))]
    Date(SmolStr),

    /// Decimal numeral, optionally with underscore digit separators.
    #[regex(r"[0-9][0-9_]*(\.[0-9][0-9_]*)?", |lex| SmolStr::from(lex.slice()))]
    Number(SmolStr),

    /// Identifier: `[A-Za-z_][A-Za-z0-9_]*`.
    /// Keywords are separate variants and take priority at equal length.
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| SmolStr::from(lex.slice()))]
    Ident(SmolStr),

    /// Double-quoted string literal; the stored value has the quotes stripped.
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        SmolStr::from(&s[1..s.len() - 1])
    })]
    StringLit(SmolStr),
}
