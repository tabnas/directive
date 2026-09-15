/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! A deliberately small grammar — just enough structure to exercise the
//! directive plugin without depending on a full JSON / jsonic grammar. It
//! defines scalar values, explicit lists `[a, b]`, and explicit maps
//! `{k: v}` (unquoted keys), reusing the engine's default lexer (bare
//! words, numbers, quoted strings).
//!
//! The rule names (val, list, map, pair, elem) match the directive's
//! default open/close rule targets, so a directive registers against this
//! grammar exactly as it would against any host grammar. This mirrors
//! `ts/test/mini-grammar.ts` and `go/mini_grammar_test.go`; keep the three
//! in step.
//!
//! The engine also ships builtin `@val-bo` / `@pair-bc` / … references
//! with these very semantics, but serialized grammar wires a lifecycle
//! reference only when it is *registered* on the instance. Building the
//! rules imperatively — as the Go port does — keeps the host grammar's
//! hooks off the reference namespace the directive plugin writes into.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use tabnas::{
    AltSpec, Context, Rule, Tabnas, Value, TIN_CA, TIN_CB, TIN_CL, TIN_CS, TIN_OB, TIN_OS,
};

/// The engine's default `#VAL` / `#KEY` token sets: bare text, numbers,
/// quoted strings and the `true`/`false`/`null` literals.
fn token_set(parser: &Tabnas, name: &str) -> Vec<tabnas::Tin> {
    parser
        .token_set(name)
        .unwrap_or_else(|| panic!("engine default token set {name} is missing"))
}

/// Counters, as an alt's `n` map.
fn counters(entries: &[(&str, i32)]) -> HashMap<String, i32> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_string(), *value))
        .collect()
}

/// Assign a rule's node. A pushed or replaced rule shares its parent's
/// node cell, so an assignment installs a fresh cell — see
/// `tabnas_directive::set_node`, which this mirrors.
fn set_node(rule: &mut Rule, value: Value) {
    rule.node = Rc::new(RefCell::new(value));
}

/// Append `value` to a list node, ignoring a node that is not a list.
fn list_push(node: &mut Value, value: Value) {
    if let Value::Array(items) = node {
        items.push(value);
    }
}

/// Install the mini grammar on `parser`.
pub fn register_mini_grammar(parser: &mut Tabnas) {
    let val_set = token_set(parser, "VAL");
    let key_set = token_set(parser, "KEY");

    // val: a value is a map, a list, or a plain scalar token.
    parser.define_rule("val", move |spec| {
        spec.add_bo(|rule, _context| {
            set_node(rule, Value::Undefined);
        });
        spec.add_bc(|rule, context| {
            if !rule.node.borrow().is_undefined() {
                return;
            }
            if !rule.child_node.is_undefined() {
                let child = rule.child_node.clone();
                set_node(rule, child);
            } else if rule.os() != 0 {
                let value = rule.resolve_open_value(0, context);
                set_node(rule, value);
            }
        });

        spec.add_open(AltSpec {
            s: vec![vec![TIN_OB]],
            p: Some("map".into()),
            b: 1,
            ..Default::default()
        })
        .add_open(AltSpec {
            s: vec![vec![TIN_OS]],
            p: Some("list".into()),
            b: 1,
            ..Default::default()
        })
        .add_open(AltSpec {
            s: vec![val_set],
            ..Default::default()
        });

        // Implicit list: a standalone value followed by a comma starts a
        // bracketless list. Only fires outside an elem/pair/ilist position
        // (so explicit `[a, b]` and `{k: v}` are unaffected) and where
        // implicit lists are permitted (`n.dlist != 1`, the counter the
        // directive plugin sets to 1 to suppress them).
        let mut implicit = AltSpec {
            s: vec![vec![TIN_CA]],
            b: 1,
            r: Some("ilist".into()),
            ..Default::default()
        };
        implicit.c_fn = Some(Arc::new(|rule: &mut Rule, _context: &mut Context| {
            let parent = rule.parent_rule.as_ref().map(|parent| parent.name.as_str());
            rule.n.get("dlist").copied().unwrap_or(0) != 1
                && !matches!(parent, Some("elem") | Some("pair") | Some("ilist"))
        }));
        // Seed the list with the already-parsed scalar value.
        implicit.add_action(|rule, _context| {
            let seed = rule.node.borrow().clone();
            set_node(rule, Value::Array(vec![seed]));
        });

        spec.add_close(AltSpec {
            s: vec![vec![tabnas::TIN_ZZ]],
            ..Default::default()
        })
        .add_close(implicit)
        .add_close(AltSpec {
            b: 1,
            ..Default::default()
        });
    });

    // ilist: bracketless list continuation. Created by replacing a val (so
    // it inherits the seeded `[first]` node) and then absorbs `, value`
    // pairs until something else closes it.
    parser.define_rule("ilist", |spec| {
        let push = |rule: &mut Rule, _context: &mut Context| {
            if rule.child_node.is_undefined() {
                return;
            }
            let child = rule.child_node.clone();
            list_push(&mut rule.node.borrow_mut(), child);
        };

        // consume comma, parse next value
        spec.add_open(AltSpec {
            s: vec![vec![TIN_CA]],
            p: Some("val".into()),
            ..Default::default()
        });

        let mut more = AltSpec {
            s: vec![vec![TIN_CA]],
            b: 1,
            r: Some("ilist".into()),
            ..Default::default()
        };
        more.add_action(push);
        let mut last = AltSpec {
            b: 1,
            ..Default::default()
        };
        last.add_action(push);

        spec.add_close(more).add_close(last);
    });

    // map: an object `{ k: v, ... }`.
    parser.define_rule("map", |spec| {
        spec.add_bo(|rule, _context| {
            set_node(rule, Value::Object(Default::default()));
        });
        spec.add_open(AltSpec {
            s: vec![vec![TIN_OB], vec![TIN_CB]],
            b: 1,
            n: counters(&[("pk", 0)]),
            ..Default::default()
        })
        .add_open(AltSpec {
            s: vec![vec![TIN_OB]],
            p: Some("pair".into()),
            n: counters(&[("pk", 0)]),
            ..Default::default()
        })
        .add_close(AltSpec {
            s: vec![vec![TIN_CB]],
            ..Default::default()
        });
    });

    // list: an array `[ a, b, ... ]`.
    parser.define_rule("list", |spec| {
        spec.add_bo(|rule, _context| {
            set_node(rule, Value::Array(Vec::new()));
        });
        spec.add_open(AltSpec {
            s: vec![vec![TIN_OS], vec![TIN_CS]],
            b: 1,
            ..Default::default()
        })
        .add_open(AltSpec {
            s: vec![vec![TIN_OS]],
            p: Some("elem".into()),
            ..Default::default()
        })
        .add_close(AltSpec {
            s: vec![vec![TIN_CS]],
            ..Default::default()
        });
    });

    // pair: a key:value entry inside a map.
    parser.define_rule("pair", move |spec| {
        spec.add_bc(|rule, _context| {
            if !rule.u.contains_key("pair") {
                return;
            }
            let Some(Value::String(key)) = rule.u.get("key").cloned() else {
                return;
            };
            let child = rule.child_node.clone();
            if let Value::Object(map) = &mut *rule.node.borrow_mut() {
                map.insert(key, child.unwrap_undefined());
            }
        });

        let mut open = AltSpec {
            s: vec![key_set, vec![TIN_CL]],
            p: Some("val".into()),
            u: HashMap::from([("pair".to_string(), Value::Bool(true))]),
            ..Default::default()
        };
        open.add_action(|rule, _context| {
            let Some(token) = rule.o0() else { return };
            let key = match &token.val {
                Value::String(text) => text.clone(),
                _ => token.src.to_string(),
            };
            rule.u_mut().insert("key".to_string(), Value::String(key));
        });

        spec.add_open(open)
            .add_close(AltSpec {
                s: vec![vec![TIN_CA]],
                r: Some("pair".into()),
                ..Default::default()
            })
            .add_close(AltSpec {
                s: vec![vec![TIN_CB]],
                b: 1,
                ..Default::default()
            });
    });

    // elem: a value inside a list.
    parser.define_rule("elem", |spec| {
        spec.add_bc(|rule, _context| {
            if rule.child_node.is_undefined() {
                return;
            }
            let child = rule.child_node.clone();
            list_push(&mut rule.node.borrow_mut(), child);
        });
        spec.add_open(AltSpec {
            p: Some("val".into()),
            ..Default::default()
        })
        .add_close(AltSpec {
            s: vec![vec![TIN_CA]],
            r: Some("elem".into()),
            ..Default::default()
        })
        .add_close(AltSpec {
            s: vec![vec![TIN_CS]],
            b: 1,
            ..Default::default()
        });
    });
}

/// Build a parser with just the mini grammar installed.
pub fn make_mini() -> Tabnas {
    let mut parser = Tabnas::new();
    register_mini_grammar(&mut parser);
    parser
}
