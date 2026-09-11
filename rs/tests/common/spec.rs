/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! TSV spec runner for the shared conformance fixtures.
//!
//! The fixtures live at the repo root in `test/spec/*.tsv` and are run by
//! all three runtimes, so the implementations cannot drift without one
//! going red. TypeScript and Go read them through `@tabnas/support` and
//! its Go half; Rust has no support crate, so the loader lives here and
//! must keep to the same codec — see `test/AGENTS.md`.
//!
//! A row is `<input>\t<expected-json>`, or `<input>\tERROR:<code>` for
//! input that must be rejected. These files have no header line and no
//! opts column: what varies per case is the DIRECTIVE, and a directive is
//! a function, so each test builds its own parser and hands it here.

use std::path::PathBuf;

use tabnas::{Tabnas, Value};

/// Decode the shared codec's escapes: `\n`, `\r`, `\t` and `\\`. Every
/// other backslash sequence survives verbatim.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(current) = chars.next() {
        if current != '\\' {
            out.push(current);
            continue;
        }
        match chars.clone().next() {
            Some('n') => {
                out.push('\n');
                chars.next();
            }
            Some('r') => {
                out.push('\r');
                chars.next();
            }
            Some('t') => {
                out.push('\t');
                chars.next();
            }
            Some('\\') => {
                out.push('\\');
                chars.next();
            }
            // Not an escape the codec knows: keep the backslash as-is.
            _ => out.push('\\'),
        }
    }
    out
}

/// The repo-root `test/spec` directory, resolved from this crate's
/// manifest so the runner does not depend on the test's working
/// directory.
fn spec_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rs/ has a parent directory")
        .join("test")
        .join("spec")
}

/// One fixture row: source line number, input, expected column.
struct Row {
    line: usize,
    input: String,
    expected: String,
}

/// Read a fixture, skipping blank lines and `#` comments. There is no
/// header line, and the columns are positional.
fn load(file_name: &str) -> Vec<Row> {
    let path = spec_dir().join(file_name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    text.lines()
        .enumerate()
        .filter_map(|(index, raw)| {
            // A trailing \r is stripped, so the files work with either
            // line ending.
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            if line.trim().is_empty() || line.starts_with('#') {
                return None;
            }
            // Rows are split on every tab and the columns are positional;
            // no row has a tab inside `expected`.
            let mut columns = line.split('\t');
            let input = columns.next()?;
            let expected = columns.next()?;
            Some(Row {
                line: index + 1,
                input: unescape(input),
                expected: expected.to_string(),
            })
        })
        .collect()
}

/// Run every row of `file_name` against `parser`.
///
/// `expected` is either a JSON value (the parse result) or `ERROR:`
/// followed by the error's CODE, compared exactly. Every failing row is
/// reported, each naming its `<file>:<line>`, so one run shows the whole
/// picture rather than just the first divergence.
pub fn run_spec(parser: &Tabnas, file_name: &str) {
    let rows = load(file_name);
    assert!(!rows.is_empty(), "{file_name}: fixture has no rows");

    let mut failures = Vec::new();

    for row in rows {
        let at = format!("{file_name}:{}", row.line);
        match row.expected.strip_prefix("ERROR:") {
            Some(code) => match parser.parse(&row.input) {
                Ok(value) => failures.push(format!(
                    "{at} parse({:?}) should fail with {code}, but returned {value}",
                    row.input
                )),
                Err(error) => {
                    if error.code != code {
                        failures.push(format!(
                            "{at} parse({:?}) error code\n  got:      {}\n  expected: {code}",
                            row.input, error.code
                        ));
                    }
                }
            },
            None => {
                let expected: serde_json::Value = serde_json::from_str(&row.expected)
                    .unwrap_or_else(|error| {
                        panic!("{at} expected column is not valid JSON: {error}")
                    });
                let expected = Value::from_json(&expected);
                match parser.parse(&row.input) {
                    Ok(value) => {
                        if !value.deep_equal(&expected) {
                            failures.push(format!(
                                "{at} parse({:?})\n  got:      {value}\n  expected: {expected}",
                                row.input
                            ));
                        }
                    }
                    Err(error) => failures.push(format!(
                        "{at} parse({:?}) returned error: {error}",
                        row.input
                    )),
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} of the rows in {file_name} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
