// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>

//! Integration tests for cross-reference and duplicate-name checking.

mod fixtures;

use oikos_check::{check_model, CheckError};

// ── Duplicate names ───────────────────────────────────────────────────────────

#[test]
fn duplicate_account_name_reported() {
    use fixtures::*;
    let mut model = model_godley(vec![], vec![], vec![]);
    model.accounts = vec![stock("wages"), stock("wages")]; // duplicate
    let Err(errors) = check_model(&model) else {
        panic!("expected Err")
    };
    assert!(errors.iter().any(|e| matches!(e,
        CheckError::DuplicateName { kind, name, .. }
        if kind == "account" && name == "wages"
    )));
}

// ── Unknown accounts ──────────────────────────────────────────────────────────

#[test]
fn transfer_with_undeclared_from_account_reported() {
    use fixtures::*;
    use oikos_syntax::{
        dimension::MoneyLiteral,
        expr::{Expr, MoneyExpr, TransferExpr},
        sector::AccountRef,
        span::Span,
    };
    use smol_str::SmolStr;

    let from = AccountRef {
        name: SmolStr::from("ghost"),
        span: Span::SYNTHETIC,
    };
    let to = AccountRef {
        name: SmolStr::from("deposits"),
        span: Span::SYNTHETIC,
    };
    let lit = MoneyLiteral {
        amount: "100.00".into(),
        currency: gbp(),
        span: Span::SYNTHETIC,
    };

    let mut model = model_godley(vec![], vec![], vec![]);
    model.accounts = vec![stock("deposits")];
    model.body = vec![Expr::Transfer(TransferExpr {
        from,
        to,
        amount: MoneyExpr::Literal(lit),
        description: None,
        span: Span::SYNTHETIC,
    })];

    let Err(errors) = check_model(&model) else {
        panic!("expected Err")
    };
    assert!(errors.iter().any(|e| matches!(e,
        CheckError::UnknownAccount { name, .. } if name == "ghost"
    )));
}

#[test]
fn transfer_with_all_declared_accounts_passes() {
    use fixtures::*;
    use oikos_syntax::{
        dimension::MoneyLiteral,
        expr::{Expr, MoneyExpr, TransferExpr},
        sector::AccountRef,
        span::Span,
    };
    use smol_str::SmolStr;

    let from = AccountRef {
        name: SmolStr::from("wages"),
        span: Span::SYNTHETIC,
    };
    let to = AccountRef {
        name: SmolStr::from("deposits"),
        span: Span::SYNTHETIC,
    };
    let lit = MoneyLiteral {
        amount: "100.00".into(),
        currency: gbp(),
        span: Span::SYNTHETIC,
    };

    let mut model = model_godley(vec![], vec![], vec![]);
    model.accounts = vec![stock("wages"), stock("deposits")];
    model.body = vec![Expr::Transfer(TransferExpr {
        from,
        to,
        amount: MoneyExpr::Literal(lit),
        description: None,
        span: Span::SYNTHETIC,
    })];

    assert!(check_model(&model).is_ok());
}

#[test]
fn close_with_undeclared_period_reported() {
    use fixtures::*;
    use oikos_syntax::{
        expr::{Expr, PeriodCloseExpr},
        sector::AccountRef,
        span::Span,
    };
    use smol_str::SmolStr;

    let account = AccountRef {
        name: SmolStr::from("deposits"),
        span: Span::SYNTHETIC,
    };
    let mut model = model_godley(vec![], vec![], vec![]);
    model.accounts = vec![stock("deposits")];
    // Declare one period so the presence check kicks in
    model.periods = vec![oikos_syntax::period::PeriodDecl {
        name: "FY2025".into(),
        kind: oikos_syntax::period::PeriodKind::FiscalYear,
        from: "2025-01-01".into(),
        to: "2025-12-31".into(),
        parent: None,
        span: Span::SYNTHETIC,
    }];
    model.body = vec![Expr::PeriodClose(PeriodCloseExpr {
        account,
        from_period: SmolStr::from("FY2024"), // not declared
        to_period: SmolStr::from("FY2025"),
        span: Span::SYNTHETIC,
    })];

    let Err(errors) = check_model(&model) else {
        panic!("expected Err")
    };
    assert!(errors.iter().any(|e| matches!(e,
        CheckError::UnknownPeriod { name, .. } if name == "FY2024"
    )));
}
