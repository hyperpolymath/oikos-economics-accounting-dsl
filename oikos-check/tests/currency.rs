// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Integration tests for currency-dimension checking.

mod fixtures;

use oikos_check::{check_model, CheckError};
use oikos_syntax::{
    account::{AccountDecl, AccountKind},
    dimension::{CurrencyCode, MoneyLiteral},
    expr::{Expr, MoneyExpr, TransferExpr},
    sector::AccountRef,
    span::Span,
};
fn currency(code: &str) -> CurrencyCode {
    CurrencyCode {
        code: code.into(),
        span: Span::SYNTHETIC,
    }
}

fn account(name: &str, ccy: &str) -> AccountDecl {
    AccountDecl {
        name: name.into(),
        kind: AccountKind::Stock,
        currency: currency(ccy),
        description: None,
        span: Span::SYNTHETIC,
    }
}

fn aref(name: &str) -> AccountRef {
    AccountRef {
        name: name.into(),
        span: Span::SYNTHETIC,
    }
}

fn literal(amount: &str, ccy: &str) -> MoneyExpr {
    MoneyExpr::Literal(MoneyLiteral {
        amount: amount.into(),
        currency: currency(ccy),
        span: Span::SYNTHETIC,
    })
}

fn transfer(from: &str, to: &str, amount: MoneyExpr) -> Expr {
    Expr::Transfer(TransferExpr {
        from: aref(from),
        to: aref(to),
        amount,
        description: None,
        span: Span::SYNTHETIC,
    })
}

// ── Matching currencies pass ──────────────────────────────────────────────────

#[test]
fn same_currency_transfer_passes() {
    use fixtures::model_godley;
    let mut model = model_godley(vec![], vec![], vec![]);
    model.accounts = vec![account("wages", "GBP"), account("deposits", "GBP")];
    model.body = vec![transfer("wages", "deposits", literal("100.00", "GBP"))];
    assert!(check_model(&model).is_ok());
}

// ── Mismatched account currencies ────────────────────────────────────────────

#[test]
fn mismatched_account_currencies_reported() {
    use fixtures::model_godley;
    let mut model = model_godley(vec![], vec![], vec![]);
    model.accounts = vec![account("gbp_account", "GBP"), account("usd_account", "USD")];
    model.body = vec![transfer(
        "gbp_account",
        "usd_account",
        literal("100.00", "GBP"),
    )];

    let Err(errors) = check_model(&model) else {
        panic!("expected Err")
    };
    assert!(errors.iter().any(|e| matches!(e,
        CheckError::TransferCurrencyMismatch {
            from_account, to_account, from_currency, to_currency, ..
        }
        if from_account == "gbp_account"
            && to_account == "usd_account"
            && from_currency == "GBP"
            && to_currency == "USD"
    )));
}

// ── Mismatched amount currency ────────────────────────────────────────────────

#[test]
fn amount_currency_mismatch_reported() {
    use fixtures::model_godley;
    let mut model = model_godley(vec![], vec![], vec![]);
    // Both accounts are GBP but the amount is denominated in USD
    model.accounts = vec![account("wages", "GBP"), account("deposits", "GBP")];
    model.body = vec![transfer("wages", "deposits", literal("100.00", "USD"))];

    let Err(errors) = check_model(&model) else {
        panic!("expected Err")
    };
    assert!(errors.iter().any(|e| matches!(e,
        CheckError::AmountCurrencyMismatch {
            account, account_currency, amount_currency, ..
        }
        if account == "wages"
            && account_currency == "GBP"
            && amount_currency == "USD"
    )));
}

// ── Both errors reported independently ───────────────────────────────────────

#[test]
fn account_mismatch_and_amount_mismatch_both_reported() {
    use fixtures::model_godley;
    let mut model = model_godley(vec![], vec![], vec![]);
    // Different account currencies AND wrong amount currency
    model.accounts = vec![account("gbp_acc", "GBP"), account("usd_acc", "USD")];
    model.body = vec![transfer("gbp_acc", "usd_acc", literal("100.00", "EUR"))];

    let Err(errors) = check_model(&model) else {
        panic!("expected Err")
    };
    // TransferCurrencyMismatch (GBP vs USD)
    assert!(errors
        .iter()
        .any(|e| matches!(e, CheckError::TransferCurrencyMismatch { .. })));
    // AmountCurrencyMismatch (EUR vs GBP)
    assert!(errors
        .iter()
        .any(|e| matches!(e, CheckError::AmountCurrencyMismatch { .. })));
}
