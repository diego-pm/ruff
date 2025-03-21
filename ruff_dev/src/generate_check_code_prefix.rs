//! Generate the `CheckCodePrefix` enum.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use anyhow::{ensure, Result};
use clap::Parser;
use codegen::{Scope, Type, Variant};
use itertools::Itertools;
use ruff::registry::{CheckCode, PREFIX_REDIRECTS};
use strum::IntoEnumIterator;

const ALL: &str = "ALL";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Write the generated source code to stdout (rather than to
    /// `src/registry_gen.rs`).
    #[arg(long)]
    pub(crate) dry_run: bool,
}

pub fn main(cli: &Cli) -> Result<()> {
    // Build up a map from prefix to matching CheckCodes.
    let mut prefix_to_codes: BTreeMap<String, BTreeSet<CheckCode>> = BTreeMap::default();
    for check_code in CheckCode::iter() {
        let code_str: String = check_code.as_ref().to_string();
        let code_prefix_len = code_str
            .chars()
            .take_while(|char| char.is_alphabetic())
            .count();
        let code_suffix_len = code_str.len() - code_prefix_len;
        for i in 0..=code_suffix_len {
            let prefix = code_str[..code_prefix_len + i].to_string();
            prefix_to_codes
                .entry(prefix)
                .or_default()
                .insert(check_code.clone());
        }
        prefix_to_codes
            .entry(ALL.to_string())
            .or_default()
            .insert(check_code.clone());
    }

    // Add any prefix aliases (e.g., "U" to "UP").
    for (alias, check_code) in PREFIX_REDIRECTS.iter() {
        prefix_to_codes.insert(
            (*alias).to_string(),
            prefix_to_codes
                .get(&check_code.as_ref().to_string())
                .unwrap_or_else(|| panic!("Unknown CheckCode: {alias:?}"))
                .clone(),
        );
    }

    let mut scope = Scope::new();

    // Create the `CheckCodePrefix` definition.
    let mut gen = scope
        .new_enum("CheckCodePrefix")
        .vis("pub")
        .derive("EnumString")
        .derive("AsRefStr")
        .derive("Debug")
        .derive("PartialEq")
        .derive("Eq")
        .derive("PartialOrd")
        .derive("Ord")
        .derive("Clone")
        .derive("Serialize")
        .derive("Deserialize")
        .derive("JsonSchema");
    for prefix in prefix_to_codes.keys() {
        gen = gen.push_variant(Variant::new(prefix.to_string()));
    }

    // Create the `SuffixLength` definition.
    scope
        .new_enum("SuffixLength")
        .vis("pub")
        .derive("PartialEq")
        .derive("Eq")
        .derive("PartialOrd")
        .derive("Ord")
        .push_variant(Variant::new("None"))
        .push_variant(Variant::new("Zero"))
        .push_variant(Variant::new("One"))
        .push_variant(Variant::new("Two"))
        .push_variant(Variant::new("Three"))
        .push_variant(Variant::new("Four"));

    // Create the `match` statement, to map from definition to relevant codes.
    let mut gen = scope
        .new_impl("CheckCodePrefix")
        .new_fn("codes")
        .arg_ref_self()
        .ret(Type::new("Vec<CheckCode>"))
        .vis("pub")
        .line("#[allow(clippy::match_same_arms)]")
        .line("match self {");
    for (prefix, codes) in &prefix_to_codes {
        if let Some(target) = PREFIX_REDIRECTS.get(&prefix.as_str()) {
            gen = gen.line(format!(
                "CheckCodePrefix::{prefix} => {{ one_time_warning!(\"{{}}{{}} {{}}\", \
                 \"warning\".yellow().bold(), \":\".bold(), \"`{}` has been remapped to \
                 `{}`\".bold()); \n vec![{}] }}",
                prefix,
                target.as_ref(),
                codes
                    .iter()
                    .map(|code| format!("CheckCode::{}", code.as_ref()))
                    .join(", ")
            ));
        } else {
            gen = gen.line(format!(
                "CheckCodePrefix::{prefix} => vec![{}],",
                codes
                    .iter()
                    .map(|code| format!("CheckCode::{}", code.as_ref()))
                    .join(", ")
            ));
        }
    }
    gen.line("}");

    // Create the `match` statement, to map from definition to specificity.
    let mut gen = scope
        .new_impl("CheckCodePrefix")
        .new_fn("specificity")
        .arg_ref_self()
        .ret(Type::new("SuffixLength"))
        .vis("pub")
        .line("#[allow(clippy::match_same_arms)]")
        .line("match self {");
    for prefix in prefix_to_codes.keys() {
        let specificity = if prefix == "ALL" {
            "None"
        } else {
            let num_numeric = prefix.chars().filter(|char| char.is_numeric()).count();
            match num_numeric {
                0 => "Zero",
                1 => "One",
                2 => "Two",
                3 => "Three",
                4 => "Four",
                _ => panic!("Invalid prefix: {prefix}"),
            }
        };
        gen = gen.line(format!(
            "CheckCodePrefix::{prefix} => SuffixLength::{specificity},"
        ));
    }
    gen.line("}");

    // Construct the output contents.
    let mut output = String::new();
    output
        .push_str("//! File automatically generated by `examples/generate_check_code_prefix.rs`.");
    output.push('\n');
    output.push('\n');
    output.push_str("use colored::Colorize;");
    output.push('\n');
    output.push_str("use schemars::JsonSchema;");
    output.push('\n');
    output.push_str("use serde::{Deserialize, Serialize};");
    output.push('\n');
    output.push_str("use strum_macros::{AsRefStr, EnumString};");
    output.push('\n');
    output.push('\n');
    output.push_str("use crate::registry::CheckCode;");
    output.push('\n');
    output.push_str("use crate::one_time_warning;");
    output.push('\n');
    output.push('\n');
    output.push_str(&scope.to_string());
    output.push('\n');
    output.push('\n');

    // Add the list of output categories (not generated).
    output.push_str("pub const CATEGORIES: &[CheckCodePrefix] = &[");
    output.push('\n');
    for prefix in prefix_to_codes.keys() {
        if prefix.chars().all(char::is_alphabetic)
            && !PREFIX_REDIRECTS.contains_key(&prefix.as_str())
        {
            output.push_str(&format!("CheckCodePrefix::{prefix},"));
            output.push('\n');
        }
    }
    output.push_str("];");
    output.push('\n');
    output.push('\n');

    let rustfmt = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    write!(rustfmt.stdin.as_ref().unwrap(), "{output}")?;
    let Output { status, stdout, .. } = rustfmt.wait_with_output()?;
    ensure!(status.success(), "rustfmt failed with {status}");

    // Write the output to `src/registry_gen.rs` (or stdout).
    if cli.dry_run {
        println!("{}", String::from_utf8(stdout)?);
    } else {
        let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Failed to find root directory")
            .join("src/registry_gen.rs");
        if fs::read(&file).map_or(true, |old| old != stdout) {
            fs::write(&file, stdout)?;
        }
    }

    Ok(())
}
