// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Integration tests for the Oikos parser.
//!
//! Each test checks that valid source text produces the expected AST structure,
//! and that invalid source produces errors rather than panicking.

use oikos_parser::parse;
use oikos_syntax::{
    account::AccountKind, godley::GodleySign, instrument::InstrumentState, period::PeriodKind,
};

// ── Helper ────────────────────────────────────────────────────────────────────

fn parse_ok(src: &str) -> oikos_syntax::Model {
    let (model, errors) = parse(src, "test.oikos").expect("parse should not Err");
    assert!(
        errors.is_empty(),
        "expected no parse errors, got: {errors:?}\nsource:\n{src}"
    );
    model.expect("expected Some(model), got None")
}

fn parse_err(src: &str) -> Vec<oikos_parser::ParseError> {
    let (_, errors) = parse(src, "test.oikos").expect("parse should not Err");
    assert!(!errors.is_empty(), "expected parse errors, got none");
    errors
}

// ── Model header ──────────────────────────────────────────────────────────────

#[test]
fn minimal_model_parses() {
    let src = r#"
        model Empty (period: FY2025) {
            godley {
                | Account |
            }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.name.as_str(), "Empty");
    assert_eq!(m.active_period.as_str(), "FY2025");
}

#[test]
fn model_name_and_period_captured() {
    let src = r#"
        model ModelSIM (period: Q1_2026) {
            godley { | Account | }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.name.as_str(), "ModelSIM");
    assert_eq!(m.active_period.as_str(), "Q1_2026");
}

// ── Period declarations ────────────────────────────────────────────────────────

#[test]
fn fiscal_year_period_parsed() {
    let src = r#"
        model M (period: FY2025) {
            period FY2025 : FiscalYear from 2025-04-01 to 2026-03-31
            godley { | Account | }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.periods.len(), 1);
    let p = &m.periods[0];
    assert_eq!(p.name.as_str(), "FY2025");
    assert_eq!(p.kind, PeriodKind::FiscalYear);
    assert_eq!(p.from.as_str(), "2025-04-01");
    assert_eq!(p.to.as_str(), "2026-03-31");
}

#[test]
fn quarter_period_parsed() {
    let src = r#"
        model M (period: Q1) {
            period Q1 : Quarter from 2026-01-01 to 2026-03-31
            godley { | Account | }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.periods[0].kind, PeriodKind::Quarter);
}

// ── Account declarations ───────────────────────────────────────────────────────

#[test]
fn stock_account_parsed() {
    let src = r#"
        model M (period: P) {
            account deposits : Stock GBP
            godley { | Account | }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.accounts.len(), 1);
    let a = &m.accounts[0];
    assert_eq!(a.name.as_str(), "deposits");
    assert_eq!(a.kind, AccountKind::Stock);
    assert_eq!(a.currency.code.as_str(), "GBP");
}

#[test]
fn flow_account_parsed() {
    let src = r#"
        model M (period: P) {
            account wages : Flow GBP
            godley { | Account | }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.accounts[0].kind, AccountKind::Flow);
}

#[test]
fn multiple_accounts_parsed() {
    let src = r#"
        model M (period: P) {
            account deposits : Stock GBP
            account wages    : Flow  GBP
            account taxes    : Flow  GBP
            godley { | Account | }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.accounts.len(), 3);
}

// ── Sector declarations ────────────────────────────────────────────────────────

#[test]
fn sector_with_assets_parsed() {
    let src = r#"
        model M (period: P) {
            sector Households {
                asset deposits
                asset equities
            }
            godley { | Account | }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.sectors.len(), 1);
    let s = &m.sectors[0];
    assert_eq!(s.name.as_str(), "Households");
    assert_eq!(s.assets.len(), 2);
    assert_eq!(s.liabilities.len(), 0);
}

#[test]
fn sector_with_liabilities_parsed() {
    let src = r#"
        model M (period: P) {
            sector Banks {
                asset reserves
                liability deposits
                liability advances
            }
            godley { | Account | }
        }
    "#;
    let m = parse_ok(src);
    let s = &m.sectors[0];
    assert_eq!(s.assets.len(), 1);
    assert_eq!(s.liabilities.len(), 2);
    assert_eq!(s.liabilities[0].name.as_str(), "deposits");
    assert_eq!(s.liabilities[1].name.as_str(), "advances");
}

// ── Godley matrix ─────────────────────────────────────────────────────────────

#[test]
fn godley_sectors_extracted() {
    let src = r#"
        model M (period: P) {
            godley {
                | Account  | Households | Banks |
                | deposits | +          | -     |
            }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.godley.sectors, vec!["Households", "Banks"]);
    assert_eq!(m.godley.accounts.len(), 1);
    assert_eq!(m.godley.accounts[0].name.as_str(), "deposits");
}

#[test]
fn godley_signs_extracted_correctly() {
    let src = r#"
        model M (period: P) {
            godley {
                | Account  | Households | Banks |
                | deposits | +          | -     |
                | wages    | -          | +     |
            }
        }
    "#;
    let m = parse_ok(src);
    let cells = &m.godley.cells;
    // Row 0 (deposits): H=+, B=-
    assert_eq!(cells[0].sign, GodleySign::Plus);
    assert_eq!(cells[0].sector.as_str(), "Households");
    assert_eq!(cells[1].sign, GodleySign::Minus);
    assert_eq!(cells[1].sector.as_str(), "Banks");
    // Row 1 (wages): H=-, B=+
    assert_eq!(cells[2].sign, GodleySign::Minus);
    assert_eq!(cells[3].sign, GodleySign::Plus);
}

#[test]
fn godley_empty_cells_are_zero() {
    let src = r#"
        model M (period: P) {
            godley {
                | Account | A | B | C |
                | loans   |   | + | - |
            }
        }
    "#;
    let m = parse_ok(src);
    let cells = &m.godley.cells;
    assert_eq!(cells[0].sign, GodleySign::Zero); // A column — empty
    assert_eq!(cells[1].sign, GodleySign::Plus); // B column
    assert_eq!(cells[2].sign, GodleySign::Minus); // C column
}

// ── Transfer expressions ──────────────────────────────────────────────────────

#[test]
fn transfer_with_unicode_arrow_parsed() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            transfer wages → deposits {
                amount: 2500.00 GBP
            }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.body.len(), 1);
    let oikos_syntax::expr::Expr::Transfer(t) = &m.body[0] else {
        panic!("expected Transfer");
    };
    assert_eq!(t.from.name.as_str(), "wages");
    assert_eq!(t.to.name.as_str(), "deposits");
}

#[test]
fn transfer_with_ascii_arrow_parsed() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            transfer wages -> deposits { amount: 100.00 GBP }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.body.len(), 1);
}

#[test]
fn transfer_with_description_parsed() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            transfer wages → deposits {
                amount:      1_000.00 GBP
                description: "Monthly salary"
            }
        }
    "#;
    let m = parse_ok(src);
    let oikos_syntax::expr::Expr::Transfer(t) = &m.body[0] else {
        panic!("expected Transfer");
    };
    assert_eq!(t.description.as_deref(), Some("Monthly salary"));
}

#[test]
fn transfer_amount_preserved() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            transfer wages → deposits { amount: 2_500.50 GBP }
        }
    "#;
    let m = parse_ok(src);
    let oikos_syntax::expr::Expr::Transfer(t) = &m.body[0] else {
        panic!("expected Transfer");
    };
    let oikos_syntax::expr::MoneyExpr::Literal(lit) = &t.amount else {
        panic!("expected Literal");
    };
    assert_eq!(lit.amount.as_str(), "2_500.50");
    assert_eq!(lit.currency.code.as_str(), "GBP");
}

// ── Close expressions ─────────────────────────────────────────────────────────

#[test]
fn close_expr_parsed() {
    let src = r#"
        model M (period: FY2025) {
            godley { | Account | }
            close deposits from FY2024 into FY2025
        }
    "#;
    let m = parse_ok(src);
    let oikos_syntax::expr::Expr::PeriodClose(c) = &m.body[0] else {
        panic!("expected PeriodClose");
    };
    assert_eq!(c.account.name.as_str(), "deposits");
    assert_eq!(c.from_period.as_str(), "FY2024");
    assert_eq!(c.to_period.as_str(), "FY2025");
}

// ── Full model ────────────────────────────────────────────────────────────────

#[test]
fn model_sim_parses_end_to_end() {
    let src = r#"
        -- Minimal Model SIM (Godley & Lavoie, 2007)
        model ModelSIM (period: FY2025) {
            period FY2025 : FiscalYear from 2025-01-01 to 2025-12-31

            account wages       : Flow  GBP
            account taxes       : Flow  GBP
            account consumption : Flow  GBP
            account deposits    : Stock GBP

            sector Households { asset deposits }
            sector Government  { liability deposits }

            godley {
                | Account     | Households | Government |
                | wages       | +          | -          |
                | taxes       | -          | +          |
                | consumption | +          | -          |
                | deposits    | +          | -          |
            }

            transfer wages    → deposits { amount: 100.00 GBP }
            transfer deposits → taxes    { amount: 20.00  GBP }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.name.as_str(), "ModelSIM");
    assert_eq!(m.periods.len(), 1);
    assert_eq!(m.accounts.len(), 4);
    assert_eq!(m.sectors.len(), 2);
    assert_eq!(m.godley.sectors.len(), 2);
    assert_eq!(m.godley.accounts.len(), 4);
    assert_eq!(m.body.len(), 2);
}

// ── Instrument declarations ───────────────────────────────────────────────────

#[test]
fn instrument_minimal_parsed() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            instrument Bond : GBP {
                states  Draft, Settled, Void
                initial Draft
                transitions {
                    Draft → Settled
                    Draft → Void
                }
            }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.instruments.len(), 1);
    let i = &m.instruments[0];
    assert_eq!(i.name.as_str(), "Bond");
    assert_eq!(i.currency.code.as_str(), "GBP");
    assert_eq!(i.states.len(), 3);
    assert_eq!(i.initial, InstrumentState::Draft);
    assert_eq!(i.transitions.len(), 2);
    assert!(i.description.is_none());
}

#[test]
fn instrument_builtin_states_recognised() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            instrument Invoice : GBP {
                states  Draft, Issued, PartiallyPaid, Settled, Void
                initial Draft
                transitions {
                    Draft         → Issued
                    Draft         → Void
                    Issued        → PartiallyPaid
                    Issued        → Void
                    PartiallyPaid → Settled
                    PartiallyPaid → Void
                }
            }
        }
    "#;
    let m = parse_ok(src);
    let i = &m.instruments[0];
    assert_eq!(
        i.states,
        vec![
            InstrumentState::Draft,
            InstrumentState::Issued,
            InstrumentState::PartiallyPaid,
            InstrumentState::Settled,
            InstrumentState::Void,
        ]
    );
}

#[test]
fn instrument_custom_state_parsed() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            instrument TBill : GBP {
                states  Draft, InMarket, Redeemed, Void
                initial Draft
                transitions {
                    Draft    → InMarket
                    InMarket → Redeemed
                    Draft    → Void
                }
            }
        }
    "#;
    let m = parse_ok(src);
    let i = &m.instruments[0];
    assert_eq!(i.states[1], InstrumentState::Custom("InMarket".into()));
    assert_eq!(i.states[2], InstrumentState::Custom("Redeemed".into()));
}

#[test]
fn instrument_with_description_parsed() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            instrument Bond : GBP {
                states  Draft, Settled, Void
                initial Draft
                transitions {
                    Draft → Settled
                    Draft → Void
                }
                description: "Simple two-step bond"
            }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(
        m.instruments[0].description.as_deref(),
        Some("Simple two-step bond")
    );
}

#[test]
fn instrument_ascii_arrow_in_transitions() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            instrument Loan : GBP {
                states  Draft, Active, Settled, Void
                initial Draft
                transitions {
                    Draft  -> Active
                    Active -> Settled
                    Active -> Void
                }
            }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.instruments[0].transitions.len(), 3);
}

#[test]
fn multiple_instruments_parsed() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            instrument Bond : GBP {
                states  Draft, Settled, Void
                initial Draft
                transitions { Draft → Settled  Draft → Void }
            }
            instrument Invoice : GBP {
                states  Draft, Issued, Settled, Void
                initial Draft
                transitions { Draft → Issued  Issued → Settled  Issued → Void }
            }
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.instruments.len(), 2);
    assert_eq!(m.instruments[0].name.as_str(), "Bond");
    assert_eq!(m.instruments[1].name.as_str(), "Invoice");
}

#[test]
fn transition_from_and_to_captured() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            instrument Bond : GBP {
                states  Draft, Settled, Void
                initial Draft
                transitions {
                    Draft → Settled
                    Draft → Void
                }
            }
        }
    "#;
    let m = parse_ok(src);
    let t = &m.instruments[0].transitions;
    assert_eq!(t[0].from, InstrumentState::Draft);
    assert_eq!(t[0].to, InstrumentState::Settled);
    assert_eq!(t[1].from, InstrumentState::Draft);
    assert_eq!(t[1].to, InstrumentState::Void);
}

// ── FX rate declarations ──────────────────────────────────────────────────────

#[test]
fn rate_declaration_parsed() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            rate GBP_USD_Q4 : GBP → USD 1.2650 on 2025-12-31
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.fx_rates.len(), 1);
    let r = &m.fx_rates[0];
    assert_eq!(r.name.as_str(), "GBP_USD_Q4");
    assert_eq!(r.from.code.as_str(), "GBP");
    assert_eq!(r.to.code.as_str(), "USD");
    assert_eq!(r.rate.as_str(), "1.2650");
    assert_eq!(r.on.as_str(), "2025-12-31");
}

#[test]
fn rate_ascii_arrow_parsed() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            rate EUR_GBP : EUR -> GBP 0.8550 on 2025-06-30
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.fx_rates[0].from.code.as_str(), "EUR");
    assert_eq!(m.fx_rates[0].to.code.as_str(), "GBP");
}

#[test]
fn multiple_rates_parsed() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            rate GBP_USD : GBP → USD 1.2650 on 2025-12-31
            rate EUR_GBP : EUR → GBP 0.8550 on 2025-12-31
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.fx_rates.len(), 2);
}

// ── Convert expressions ───────────────────────────────────────────────────────

#[test]
fn convert_expr_parsed() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            rate GBP_USD : GBP → USD 1.2650 on 2025-12-31
            convert 1_000.00 GBP via GBP_USD into usd_account
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.body.len(), 1);
    let oikos_syntax::expr::Expr::FxConversion(fx) = &m.body[0] else {
        panic!("expected FxConversion");
    };
    assert_eq!(fx.rate_name.as_str(), "GBP_USD");
    assert_eq!(fx.destination.name.as_str(), "usd_account");
}

#[test]
fn convert_amount_preserved() {
    let src = r#"
        model M (period: P) {
            godley { | Account | }
            rate GBP_USD : GBP → USD 1.2650 on 2025-12-31
            convert 500.00 GBP via GBP_USD into usd_reserve
        }
    "#;
    let m = parse_ok(src);
    let oikos_syntax::expr::Expr::FxConversion(fx) = &m.body[0] else {
        panic!("expected FxConversion");
    };
    let oikos_syntax::expr::MoneyExpr::Literal(lit) = &fx.amount else {
        panic!("expected Literal");
    };
    assert_eq!(lit.amount.as_str(), "500.00");
    assert_eq!(lit.currency.code.as_str(), "GBP");
}

#[test]
fn rate_and_convert_in_full_model() {
    let src = r#"
        model UK (period: FY2025) {
            period FY2025 : FiscalYear from 2025-01-01 to 2025-12-31
            account gbp_reserve : Stock GBP
            account usd_reserve : Stock USD
            rate GBP_USD : GBP → USD 1.2650 on 2025-12-31
            godley {
                | Account     | BoE  |
                | gbp_reserve | +    |
                | usd_reserve | -    |
            }
            convert 10_000.00 GBP via GBP_USD into usd_reserve
        }
    "#;
    let m = parse_ok(src);
    assert_eq!(m.fx_rates.len(), 1);
    assert_eq!(m.body.len(), 1);
    assert!(matches!(
        m.body[0],
        oikos_syntax::expr::Expr::FxConversion(_)
    ));
}

// ── Error cases ───────────────────────────────────────────────────────────────

#[test]
fn empty_source_produces_error() {
    parse_err("");
}

#[test]
fn missing_godley_keyword_before_model_produces_error() {
    parse_err("sector Households {}"); // no wrapping model
}
