use log::error;
use rustpython_ast::{Constant, Expr, ExprContext, ExprKind, Location};

use crate::ast::types::Range;
use crate::autofix::Fix;
use crate::checkers::ast::Checker;
use crate::python::identifiers::IDENTIFIER_REGEX;
use crate::python::keyword::KWLIST;
use crate::registry::{Check, CheckKind};
use crate::source_code_generator::SourceCodeGenerator;

fn attribute(value: &Expr, attr: &str) -> Expr {
    Expr::new(
        Location::default(),
        Location::default(),
        ExprKind::Attribute {
            value: Box::new(value.clone()),
            attr: attr.to_string(),
            ctx: ExprContext::Load,
        },
    )
}

/// B009
pub fn getattr_with_constant(checker: &mut Checker, expr: &Expr, func: &Expr, args: &[Expr]) {
    let ExprKind::Name { id, .. } = &func.node else {
        return;
    };
    if id != "getattr" {
        return;
    }
    let [obj, arg] = args else {
        return;
    };
    let ExprKind::Constant {
        value: Constant::Str(value),
        ..
    } = &arg.node else {
        return;
    };
    if !IDENTIFIER_REGEX.is_match(value) {
        return;
    }
    if KWLIST.contains(&value.as_str()) {
        return;
    }

    let mut check = Check::new(CheckKind::GetAttrWithConstant, Range::from_located(expr));
    if checker.patch(check.kind.code()) {
        let mut generator = SourceCodeGenerator::new(
            checker.style.indentation(),
            checker.style.quote(),
            checker.style.line_ending(),
        );
        generator.unparse_expr(&attribute(obj, value), 0);
        match generator.generate() {
            Ok(content) => {
                check.amend(Fix::replacement(
                    content,
                    expr.location,
                    expr.end_location.unwrap(),
                ));
            }
            Err(e) => error!("Failed to rewrite `getattr`: {e}"),
        }
    }
    checker.add_check(check);
}
