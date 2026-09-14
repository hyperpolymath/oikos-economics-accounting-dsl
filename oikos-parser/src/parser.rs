// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Chumsky parser for the Oikos DSL.
//!
//! Input type: `&[Token]` — a flat token slice (spans are `Span::SYNTHETIC`
//! throughout; diagnostic spans are a follow-up once the parser is stable).
//!
//! # Implemented
//! `model`, `period`, `account`, `sector`, `godley`, `transfer`, `close`, `instrument`,
//! `rate`, `convert`.

use chumsky::prelude::*;
use smol_str::SmolStr;

use oikos_syntax::{
    account::{AccountDecl, AccountKind},
    dimension::{CurrencyCode, FxRate, MoneyLiteral},
    expr::{Expr, FxConversionExpr, MoneyExpr, PeriodCloseExpr, TransferExpr},
    godley::{GodleyCell, GodleyMatrix, GodleySign},
    instrument::{InstrumentDecl, InstrumentState, StateTransition},
    model::Model,
    period::{PeriodDecl, PeriodKind},
    sector::{AccountRef, SectorDecl},
    span::Span,
};

use crate::{error::ParseError, lexer::Token};

// ── Input / error type aliases ────────────────────────────────────────────────

// `&[Token]` implements `Input` in chumsky 1.0-alpha; spans are index-based.
// We discard span information for now (all AST nodes carry `Span::SYNTHETIC`).
type Extra<'src> = extra::Err<Rich<'src, Token>>;

fn syn() -> Span {
    Span::SYNTHETIC
}

// ── Primitive parsers ─────────────────────────────────────────────────────────

fn ident<'src>() -> impl Parser<'src, &'src [Token], SmolStr, Extra<'src>> + Clone {
    select! { Token::Ident(s) => s }
}

fn date<'src>() -> impl Parser<'src, &'src [Token], SmolStr, Extra<'src>> + Clone {
    select! { Token::Date(s) => s }
}

fn number<'src>() -> impl Parser<'src, &'src [Token], SmolStr, Extra<'src>> + Clone {
    select! { Token::Number(s) => s }
}

fn string_lit<'src>() -> impl Parser<'src, &'src [Token], SmolStr, Extra<'src>> + Clone {
    select! { Token::StringLit(s) => s }
}

fn arrow<'src>() -> impl Parser<'src, &'src [Token], (), Extra<'src>> + Clone {
    just(Token::Arrow).or(just(Token::AsciiArrow)).ignored()
}

fn account_ref<'src>() -> impl Parser<'src, &'src [Token], AccountRef, Extra<'src>> + Clone {
    ident().map(|name| AccountRef { name, span: syn() })
}

fn currency<'src>() -> impl Parser<'src, &'src [Token], CurrencyCode, Extra<'src>> + Clone {
    ident().map(|code| CurrencyCode { code, span: syn() })
}

// ── Declarations ──────────────────────────────────────────────────────────────

/// `period FY2025 : FiscalYear from 2025-04-01 to 2026-03-31`
fn period_decl<'src>() -> impl Parser<'src, &'src [Token], PeriodDecl, Extra<'src>> + Clone {
    let kind = choice((
        just(Token::KwFiscalYear).to(PeriodKind::FiscalYear),
        just(Token::KwHalfYear).to(PeriodKind::HalfYear),
        just(Token::KwQuarter).to(PeriodKind::Quarter),
        just(Token::KwMonth).to(PeriodKind::Month),
        just(Token::KwWeek).to(PeriodKind::Week),
    ));

    just(Token::KwPeriod)
        .ignore_then(ident())
        .then_ignore(just(Token::Colon))
        .then(kind)
        .then_ignore(just(Token::KwFrom))
        .then(date())
        .then_ignore(just(Token::KwTo))
        .then(date())
        .map(|(((name, kind), from), to)| PeriodDecl {
            name,
            kind,
            from,
            to,
            parent: None,
            span: syn(),
        })
}

/// `account deposits : Stock GBP`
fn account_decl<'src>() -> impl Parser<'src, &'src [Token], AccountDecl, Extra<'src>> + Clone {
    let kind = just(Token::KwStock)
        .to(AccountKind::Stock)
        .or(just(Token::KwFlow).to(AccountKind::Flow));

    just(Token::KwAccount)
        .ignore_then(ident())
        .then_ignore(just(Token::Colon))
        .then(kind)
        .then(currency())
        .map(|((name, kind), currency)| AccountDecl {
            name,
            kind,
            currency,
            description: None,
            span: syn(),
        })
}

/// `asset deposits`  /  `liability mortgage`
fn sector_item<'src>() -> impl Parser<'src, &'src [Token], (bool, AccountRef), Extra<'src>> + Clone
{
    just(Token::KwAsset)
        .to(true)
        .or(just(Token::KwLiability).to(false))
        .then(account_ref())
}

/// `sector Households { asset deposits; liability mortgage }`
fn sector_decl<'src>() -> impl Parser<'src, &'src [Token], SectorDecl, Extra<'src>> + Clone {
    let items = sector_item()
        .then_ignore(just(Token::Semicolon).or_not())
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::LBrace), just(Token::RBrace));

    just(Token::KwSector)
        .ignore_then(ident())
        .then(items)
        .map(|(name, raw)| {
            let mut assets: Vec<AccountRef> = Vec::new();
            let mut liabilities: Vec<AccountRef> = Vec::new();
            for (is_asset, aref) in raw {
                if is_asset {
                    assets.push(aref);
                } else {
                    liabilities.push(aref);
                }
            }
            SectorDecl {
                name,
                accounts: vec![],
                assets,
                liabilities,
                description: None,
                span: syn(),
            }
        })
}

// ── Godley matrix ─────────────────────────────────────────────────────────────

/// Two-pass Godley table interpreter.
///
/// The pipe-delimited table format is tricky for single-pass combinators
/// because sign cells may be empty (consecutive `|` characters), and
/// `or_not()` on a sign parser always succeeds — causing the combinator to
/// greedily consume the trailing `|` of a row and then error.
///
/// Solution: collect *all* non-brace tokens inside the `godley { }` block
/// as a flat `Vec<Token>`, then interpret them in Rust without chumsky.
fn interpret_godley_tokens(tokens: Vec<Token>) -> GodleyMatrix {
    // Split the flat stream on Pipe boundaries to get individual cell groups.
    // Each group is the tokens between two consecutive `|` separators.
    let mut groups: Vec<Vec<Token>> = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    for tok in tokens {
        if tok == Token::Pipe {
            groups.push(std::mem::take(&mut current));
        } else {
            current.push(tok);
        }
    }
    // Always push the final group: the last `|` in each row leaves `current` empty
    // but the loop ends without flushing it.  The trailing empty group is required
    // so that `(groups.len() - 1) / stride` counts rows correctly.
    groups.push(current);

    // Layout after splitting on `|`:
    //
    //   | Account | Households | Banks |
    //   | deposits| +          | -     |
    //
    // groups: [], [Account], [Households], [Banks], [], [deposits], [+], [-], []
    // idx:     0      1           2           3     4       5        6    7   8
    //
    // Adjacent rows share their boundary group (groups[4] above).
    // The stride is the index of the first empty group after position 0 — which
    // equals the number of groups consumed per row (the shared boundary is counted
    // as the *start* of each row, not the end).
    //
    //   stride = 4:  row 0 → groups[0..4], row 1 → groups[4..8]
    //   num_rows = (groups.len() - 1) / stride

    let stride = match groups
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, g)| g.is_empty())
    {
        Some((i, _)) => i,
        None => {
            return GodleyMatrix {
                cells: vec![],
                sectors: vec![],
                accounts: vec![],
                span: syn(),
            }
        }
    };

    // stride < 2 means no cell columns exist at all (pathological input).
    if stride < 2 || groups.len() < stride + 1 {
        return GodleyMatrix {
            cells: vec![],
            sectors: vec![],
            accounts: vec![],
            span: syn(),
        };
    }

    let num_rows = (groups.len() - 1) / stride;

    // Header row (row 0): groups[1] is the "Account" label (discarded).
    //                     groups[2..stride] are the sector names.
    let sectors: Vec<SmolStr> = groups[2..stride]
        .iter()
        .flat_map(|g| cell_as_ident(g))
        .collect();

    // Data rows (row 1..num_rows): row i starts at groups[i * stride].
    //   groups[i*stride]       — leading boundary (always empty)
    //   groups[i*stride + 1]   — account name
    //   groups[i*stride + 2 + col] — sign for column `col`
    let mut accounts: Vec<AccountRef> = Vec::new();
    let mut cells: Vec<GodleyCell> = Vec::new();

    for row_idx in 1..num_rows {
        let base = row_idx * stride;
        let account_name = groups
            .get(base + 1)
            .and_then(|g| cell_as_ident(g))
            .unwrap_or_default();
        accounts.push(AccountRef {
            name: account_name.clone(),
            span: syn(),
        });

        for col in 0..sectors.len() {
            let sign = groups
                .get(base + 2 + col)
                .map(|g| cell_as_sign(g))
                .unwrap_or(GodleySign::Zero);
            cells.push(GodleyCell {
                account: AccountRef {
                    name: account_name.clone(),
                    span: syn(),
                },
                sector: sectors.get(col).cloned().unwrap_or_default(),
                sign,
                span: syn(),
            });
        }
    }

    GodleyMatrix {
        cells,
        sectors,
        accounts,
        span: syn(),
    }
}

fn cell_as_ident(group: &[Token]) -> Option<SmolStr> {
    group.iter().find_map(|t| {
        if let Token::Ident(s) = t {
            Some(s.clone())
        } else {
            None
        }
    })
}

fn cell_as_sign(group: &[Token]) -> GodleySign {
    for t in group {
        match t {
            Token::Plus => return GodleySign::Plus,
            Token::Minus => return GodleySign::Minus,
            _ => {}
        }
    }
    GodleySign::Zero
}

/// ```text
/// godley {
///     | Account   | Households | Banks |
///     | deposits  | +          | -     |
/// }
/// ```
fn godley_block<'src>() -> impl Parser<'src, &'src [Token], GodleyMatrix, Extra<'src>> + Clone {
    // Collect all tokens inside { } (excluding the braces themselves), then
    // hand off to the pure two-pass interpreter.  This sidesteps the partial-
    // consumption problem that `or_not()` causes in the single-pass approach.
    let inner_tokens = any()
        .filter(|t: &Token| !matches!(t, Token::LBrace | Token::RBrace))
        .repeated()
        .collect::<Vec<Token>>()
        .delimited_by(just(Token::LBrace), just(Token::RBrace))
        .map(interpret_godley_tokens);

    just(Token::KwGodley).ignore_then(inner_tokens)
}

// ── Expressions ───────────────────────────────────────────────────────────────

fn money_expr<'src>() -> impl Parser<'src, &'src [Token], MoneyExpr, Extra<'src>> + Clone {
    number().then(currency()).map(|(amount, currency)| {
        MoneyExpr::Literal(MoneyLiteral {
            amount,
            currency,
            span: syn(),
        })
    })
}

/// `transfer wages → deposits { amount: 2_500.00 GBP  description: "..." }`
fn transfer_expr<'src>() -> impl Parser<'src, &'src [Token], Expr, Extra<'src>> + Clone {
    let body = just(Token::KwAmount)
        .ignore_then(just(Token::Colon))
        .ignore_then(money_expr())
        .then(
            just(Token::KwDescription)
                .ignore_then(just(Token::Colon))
                .ignore_then(string_lit())
                .or_not(),
        )
        .delimited_by(just(Token::LBrace), just(Token::RBrace));

    just(Token::KwTransfer)
        .ignore_then(account_ref())
        .then_ignore(arrow())
        .then(account_ref())
        .then(body)
        .map(|((from, to), (amount, description))| {
            Expr::Transfer(TransferExpr {
                from,
                to,
                amount,
                description,
                span: syn(),
            })
        })
}

/// `close deposits from FY2024 into FY2025`
fn close_expr<'src>() -> impl Parser<'src, &'src [Token], Expr, Extra<'src>> + Clone {
    just(Token::KwClose)
        .ignore_then(account_ref())
        .then_ignore(just(Token::KwFrom))
        .then(ident())
        .then_ignore(just(Token::KwInto))
        .then(ident())
        .map(|((account, from_period), to_period)| {
            Expr::PeriodClose(PeriodCloseExpr {
                account,
                from_period,
                to_period,
                span: syn(),
            })
        })
}

// ── FX rate declarations and convert expressions ──────────────────────────────

/// `rate GBP_USD_Q4 : GBP → USD 1.2650 on 2025-12-31`
///
/// The `FxRate(...)` wrapper from the spec is simplified: `FxRate` is implied
/// by the `rate` keyword; only the `from → to` currency pair is needed.
fn rate_decl<'src>() -> impl Parser<'src, &'src [Token], FxRate, Extra<'src>> + Clone {
    just(Token::KwRate)
        .ignore_then(ident()) // rate name
        .then_ignore(just(Token::Colon))
        .then(currency()) // from currency
        .then_ignore(arrow())
        .then(currency()) // to currency
        .then(number()) // rate value
        .then_ignore(just(Token::KwOn))
        .then(date()) // effective date
        .map(|((((name, from), to), rate), on)| FxRate {
            name,
            from,
            to,
            rate,
            on,
            span: syn(),
        })
}

/// `convert 1_000.00 GBP via GBP_USD_Q4 into usd_account`
fn convert_expr<'src>() -> impl Parser<'src, &'src [Token], Expr, Extra<'src>> + Clone {
    just(Token::KwConvert)
        .ignore_then(money_expr()) // amount and source currency
        .then_ignore(just(Token::KwVia))
        .then(ident()) // named FX rate
        .then_ignore(just(Token::KwInto))
        .then(account_ref()) // destination account
        .map(|((amount, rate_name), destination)| {
            Expr::FxConversion(FxConversionExpr {
                amount,
                rate_name,
                destination,
                span: syn(),
            })
        })
}

// ── Instrument declarations ───────────────────────────────────────────────────

/// Map a bare identifier to an `InstrumentState`.
/// Built-in states are recognised by name; anything else becomes `Custom`.
fn instrument_state<'src>() -> impl Parser<'src, &'src [Token], InstrumentState, Extra<'src>> + Clone
{
    ident().map(|s| match s.as_str() {
        "Draft" => InstrumentState::Draft,
        "Issued" => InstrumentState::Issued,
        "PartiallyPaid" => InstrumentState::PartiallyPaid,
        "Settled" => InstrumentState::Settled,
        "Void" => InstrumentState::Void,
        _ => InstrumentState::Custom(s),
    })
}

/// ```text
/// instrument Invoice : GBP {
///     states  Draft, Issued, PartiallyPaid, Settled, Void
///     initial Draft
///     transitions {
///         Draft         → Issued
///         Draft         → Void
///         Issued        → PartiallyPaid
///         PartiallyPaid → Settled
///         PartiallyPaid → Void
///     }
///     description: "Standard invoice lifecycle"   -- optional
/// }
/// ```
fn instrument_decl<'src>() -> impl Parser<'src, &'src [Token], InstrumentDecl, Extra<'src>> + Clone
{
    // `states Draft, Issued, PartiallyPaid, Settled, Void`
    let states_clause = just(Token::KwStates).ignore_then(
        instrument_state()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>(),
    );

    // `initial Draft`
    let initial_clause = just(Token::KwInitial).ignore_then(instrument_state());

    // A single transition line: `Draft → Issued`
    let transition = instrument_state()
        .then_ignore(arrow())
        .then(instrument_state())
        .map(|(from, to)| StateTransition {
            from,
            to,
            span: syn(),
        });

    // `transitions { Draft → Issued  ... }`
    let transitions_clause = just(Token::KwTransitions).ignore_then(
        transition
            .repeated()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LBrace), just(Token::RBrace)),
    );

    // Optional `description: "..."` — same style as transfer body
    let description_clause = just(Token::KwDescription)
        .ignore_then(just(Token::Colon))
        .ignore_then(string_lit())
        .or_not();

    just(Token::KwInstrument)
        .ignore_then(ident())
        .then_ignore(just(Token::Colon))
        .then(currency())
        .then(
            states_clause
                .then(initial_clause)
                .then(transitions_clause)
                .then(description_clause)
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
        .map(
            |((name, currency), (((states, initial), transitions), description))| InstrumentDecl {
                name,
                currency,
                states,
                initial,
                transitions,
                description,
                span: syn(),
            },
        )
}

// ── Model ─────────────────────────────────────────────────────────────────────

enum Item {
    Period(PeriodDecl),
    Account(AccountDecl),
    Sector(SectorDecl),
    Instrument(InstrumentDecl),
    FxRate(FxRate),
    Godley(GodleyMatrix),
    Expr(Expr),
}

fn model_parser<'src>() -> impl Parser<'src, &'src [Token], Model, Extra<'src>> {
    let item = choice((
        period_decl().map(Item::Period),
        account_decl().map(Item::Account),
        sector_decl().map(Item::Sector),
        instrument_decl().map(Item::Instrument),
        rate_decl().map(Item::FxRate),
        godley_block().map(Item::Godley),
        transfer_expr().map(Item::Expr),
        close_expr().map(Item::Expr),
        convert_expr().map(Item::Expr),
    ));

    just(Token::KwModel)
        .ignore_then(ident())
        .then_ignore(just(Token::LParen))
        .then_ignore(just(Token::KwPeriod))
        .then_ignore(just(Token::Colon))
        .then(ident())
        .then_ignore(just(Token::RParen))
        .then(
            item.repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
        .map(|((name, active_period), items)| {
            let mut periods = Vec::new();
            let mut accounts = Vec::new();
            let mut sectors = Vec::new();
            let mut instruments = Vec::new();
            let mut fx_rates = Vec::new();
            let mut godley_opt: Option<GodleyMatrix> = None;
            let mut body = Vec::new();
            for item in items {
                match item {
                    Item::Period(p) => periods.push(p),
                    Item::Account(a) => accounts.push(a),
                    Item::Sector(s) => sectors.push(s),
                    Item::Instrument(i) => instruments.push(i),
                    Item::FxRate(r) => fx_rates.push(r),
                    Item::Godley(g) => godley_opt = Some(g),
                    Item::Expr(e) => body.push(e),
                }
            }
            let godley = godley_opt.unwrap_or(GodleyMatrix {
                cells: vec![],
                sectors: vec![],
                accounts: vec![],
                span: syn(),
            });
            Model {
                name,
                active_period,
                periods,
                fx_rates,
                accounts,
                instruments,
                sectors,
                godley,
                body,
                span: syn(),
            }
        })
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn parse_model(source: &str) -> Result<(Option<Model>, Vec<ParseError>), ParseError> {
    use logos::Logos;

    // Collect tokens alongside their source byte-ranges from Logos.
    // We keep a parallel `token_spans` vec so that chumsky's token-index-based
    // error spans can be translated back to character offsets for diagnostics.
    let mut tokens: Vec<Token> = Vec::new();
    let mut token_spans: Vec<std::ops::Range<usize>> = Vec::new();

    for (result, range) in Token::lexer(source).spanned() {
        if let Ok(tok) = result {
            tokens.push(tok);
            token_spans.push(range);
        }
        // Unrecognised characters are silently skipped for now.
        // TODO: collect lexer errors and surface them as ParseError::UnknownChar.
    }

    let (model, raw_errors) = model_parser().parse(tokens.as_slice()).into_output_errors();

    let errors = raw_errors
        .into_iter()
        .map(|e| {
            // e.span() is a SimpleSpan<usize> over token indices.
            // Map to the corresponding source byte offsets.
            let tok_start = e.span().start;
            let tok_end = e.span().end;
            let char_start = token_spans.get(tok_start).map(|r| r.start).unwrap_or(0);
            let char_end = token_spans
                .get(tok_end.saturating_sub(1))
                .map(|r| r.end)
                .unwrap_or(source.len());
            let span = Span::new(char_start, char_end);
            ParseError::UnexpectedToken {
                expected: e
                    .expected()
                    .map(|t| format!("{t:?}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                found: format!("{:?}", e.found()),
                span,
            }
        })
        .collect();

    Ok((model, errors))
}
