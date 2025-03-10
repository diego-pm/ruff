use itertools::Itertools;
use log::error;
use rustpython_ast::{Alias, AliasData, Located};
use rustpython_parser::ast::Stmt;

use crate::ast::types::Range;
use crate::autofix;
use crate::checkers::ast::Checker;
use crate::registry::{Check, CheckKind};

const BUILTINS: &[&str] = &[
    "*",
    "ascii",
    "bytes",
    "chr",
    "dict",
    "filter",
    "hex",
    "input",
    "int",
    "isinstance",
    "list",
    "map",
    "max",
    "min",
    "next",
    "object",
    "oct",
    "open",
    "pow",
    "range",
    "round",
    "str",
    "super",
    "zip",
];
const IO: &[&str] = &["open"];
const SIX_MOVES_BUILTINS: &[&str] = BUILTINS;
const SIX: &[&str] = &["callable", "next"];
const SIX_MOVES: &[&str] = &["filter", "input", "map", "range", "zip"];

/// UP029
pub fn unnecessary_builtin_import(
    checker: &mut Checker,
    stmt: &Stmt,
    module: &str,
    names: &[Located<AliasData>],
) {
    let deprecated_names = match module {
        "builtins" => BUILTINS,
        "io" => IO,
        "six" => SIX,
        "six.moves" => SIX_MOVES,
        "six.moves.builtins" => SIX_MOVES_BUILTINS,
        _ => return,
    };

    let mut unused_imports: Vec<&Alias> = vec![];
    for alias in names {
        if alias.node.asname.is_some() {
            continue;
        }
        if deprecated_names.contains(&alias.node.name.as_str()) {
            unused_imports.push(alias);
        }
    }

    if unused_imports.is_empty() {
        return;
    }
    let mut check = Check::new(
        CheckKind::UnnecessaryBuiltinImport(
            unused_imports
                .iter()
                .map(|alias| alias.node.name.to_string())
                .sorted()
                .collect(),
        ),
        Range::from_located(stmt),
    );

    if checker.patch(check.kind.code()) {
        let deleted: Vec<&Stmt> = checker.deletions.iter().map(|node| node.0).collect();
        let defined_by = checker.current_stmt();
        let defined_in = checker.current_stmt_parent();
        let unused_imports: Vec<String> = unused_imports
            .iter()
            .map(|alias| format!("{module}.{}", alias.node.name))
            .collect();
        match autofix::helpers::remove_unused_imports(
            unused_imports.iter().map(String::as_str),
            defined_by.0,
            defined_in.map(|node| node.0),
            &deleted,
            checker.locator,
        ) {
            Ok(fix) => {
                if fix.content.is_empty() || fix.content == "pass" {
                    checker.deletions.insert(defined_by.clone());
                }
                check.amend(fix);
            }
            Err(e) => error!("Failed to remove builtin import: {e}"),
        }
    }
    checker.add_check(check);
}
