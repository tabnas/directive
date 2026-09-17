/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

//! Doc-example harness for the Rust port.
//!
//! `ts/test/doc-examples.test.ts` extracts fenced ```js blocks from the
//! docs and evaluates the ones carrying `// =>` assertions. Rust cannot
//! evaluate markdown, so the equivalent protection is to transcribe the
//! documented snippets here as real, compiled tests: a doc example that
//! stops compiling — or stops producing what the doc claims — fails the
//! build.
//!
//! Each test names the doc and section it came from. Keep them in step:
//! if you change a snippet in `rs/README.md` or `rs/doc/*.md`, change the
//! test beside it.
//!
//! `register_host_grammar(&mut parser)` in the docs stands for whatever
//! host grammar the reader brings; here it is the repo's mini grammar.

mod common;

use std::collections::HashMap;
use std::sync::Arc;

use common::mini_grammar::make_mini;

use tabnas::{ActionError, AltSpec, Value};
use tabnas_directive::{apply, plugin, set_node, DirectiveOptions, RuleMod, RulesOption};

/// Assert a parse produces the documented value.
fn assert_parses(parser: &tabnas::Tabnas, src: &str, expected: Value) {
    let value = parser
        .parse(src)
        .unwrap_or_else(|error| panic!("{src:?} should parse: {error}"));
    assert!(
        value.deep_equal(&expected),
        "{src:?}\n  got:      {value}\n  expected: {expected}"
    );
}

fn string(text: &str) -> Value {
    Value::String(text.to_string())
}

fn object(entries: &[(&str, Value)]) -> Value {
    Value::object(
        entries
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect(),
    )
}

/// The `upper` directive used by `rs/README.md` § Use and
/// `rs/doc/tutorial.md` § 2.
fn upper_options() -> DirectiveOptions {
    DirectiveOptions::new("upper", "@").with_action(|rule, _ctx| {
        let body = match &rule.child_node {
            Value::String(text) => text.to_uppercase(),
            other => other.to_string().to_uppercase(),
        };
        set_node(rule, Value::String(body));
        Ok(())
    })
}

/// The `sum` directive used by `rs/doc/tutorial.md` §§ 4-5.
fn sum_options() -> DirectiveOptions {
    DirectiveOptions::new("sum", "sum<")
        .with_close(">")
        .with_action(|rule, _ctx| {
            let mut total = 0.0_f64;
            if let Value::Array(items) = &rule.child_node {
                for item in items.iter() {
                    if let Value::Number(number) = item {
                        total += number;
                    }
                }
            }
            set_node(rule, Value::Number(total));
            Ok(())
        })
}

#[test]
fn readme_use_and_tutorial_open_only_directive() {
    let mut parser = make_mini();
    apply(&mut parser, upper_options()).expect("upper registers");

    // rs/doc/tutorial.md § 2
    assert_parses(&parser, "@hello", string("HELLO"));

    // rs/README.md § Use, and rs/doc/tutorial.md § 3
    assert_parses(
        &parser,
        "[@a, @b, 1]",
        Value::array(vec![string("A"), string("B"), Value::Number(1.0)]),
    );
    assert_parses(
        &parser,
        "{x:@a, y:@b}",
        object(&[("x", string("A")), ("y", string("B"))]),
    );
}

#[test]
fn tutorial_close_token_and_boundary_closing() {
    let mut parser = make_mini();
    apply(&mut parser, sum_options()).expect("sum registers");

    // rs/doc/tutorial.md § 4
    assert_parses(&parser, "sum<[1, 2, 3]>", Value::Number(6.0));

    // rs/doc/tutorial.md § 5 — no ']' before '>'
    assert_parses(&parser, "sum<[1, 2>", Value::Number(3.0));
}

#[test]
fn tutorial_handle_failure() {
    let mut parser = make_mini();
    apply(&mut parser, sum_options()).expect("sum registers");

    // rs/doc/tutorial.md § 6 — a duplicate open token
    let error = apply(&mut parser, DirectiveOptions::new("dup", "sum<"))
        .expect_err("a duplicate open token is a registration error");
    assert_eq!(
        error.to_string(),
        "Directive open token already in use: sum<"
    );

    // rs/doc/tutorial.md § 6, rs/doc/guide.md § Report a failure
    apply(
        &mut parser,
        DirectiveOptions::new("strict", "strict<")
            .with_close(">")
            .with_action(|rule, _ctx| match &rule.child_node {
                Value::Number(_) => Ok(()),
                _ => Err(ActionError::new("strict_body", "expected a number")),
            }),
    )
    .expect("strict registers");

    assert!(parser.parse("strict<1>").is_ok());
    let error = parser
        .parse("strict<a>")
        .expect_err("a non-number body is rejected");
    assert_eq!(error.code, "strict_body");
}

#[test]
fn guide_wrap_an_arbitrary_body() {
    let mut parser = make_mini();
    apply(
        &mut parser,
        DirectiveOptions::new("group", "(")
            .with_close(")")
            .with_action(|rule, _ctx| {
                let body = rule.child_node.clone();
                set_node(rule, body);
                Ok(())
            }),
    )
    .expect("group registers");

    assert_parses(
        &parser,
        "([1, 2])",
        Value::array(vec![Value::Number(1.0), Value::Number(2.0)]),
    );
}

#[test]
fn guide_share_a_close_token() {
    let mut parser = make_mini();
    apply(
        &mut parser,
        DirectiveOptions::new("foo", "foo<")
            .with_close(">")
            .with_action(|rule, _ctx| {
                set_node(rule, string("FOO"));
                Ok(())
            }),
    )
    .expect("foo registers");
    apply(
        &mut parser,
        DirectiveOptions::new("bar", "bar<")
            .with_close(">")
            .with_action(|rule, _ctx| {
                set_node(rule, string("BAR"));
                Ok(())
            }),
    )
    .expect("bar registers");

    assert_parses(
        &parser,
        "[foo<a>, bar<b>]",
        Value::array(vec![string("FOO"), string("BAR")]),
    );

    let error = apply(&mut parser, DirectiveOptions::new("baz", "foo<"))
        .expect_err("an open token cannot be reused");
    assert_eq!(
        error.to_string(),
        "Directive open token already in use: foo<"
    );
}

#[test]
fn guide_restrict_where_recognised_and_merge_at_a_pair() {
    // rs/doc/guide.md §§ Restrict where a directive is recognised /
    // Merge into the surrounding map at a `pair` position.
    let lookup = object(&[("b", object(&[("b", Value::Number(1.0))]))]);

    let mut parser = make_mini();
    apply(
        &mut parser,
        DirectiveOptions::new("inject", "@")
            .with_rules(
                RulesOption::new()
                    .open_rules("val,pair")
                    .close_rules("list,elem,map,pair"),
            )
            .with_action(move |rule, _ctx| {
                let key = match &rule.child_node {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                };
                let Value::Object(entries) = &lookup else {
                    unreachable!("the lookup table is an object")
                };
                let value = entries.get(&key).cloned().unwrap_or(Value::Null);

                let at_pair = rule
                    .parent_rule
                    .as_ref()
                    .is_some_and(|parent| parent.name == "pair");
                if at_pair {
                    if let (Some(parent_node), Value::Object(fields)) =
                        (rule.parent_node.clone(), &value)
                    {
                        if let Some(target) = parent_node.borrow_mut().as_object_mut() {
                            for (field, field_value) in fields.iter() {
                                target.insert(field.clone(), field_value.clone());
                            }
                            return Ok(());
                        }
                    }
                }

                set_node(rule, value);
                Ok(())
            }),
    )
    .expect("inject registers");

    // Value position: a missing key is null.
    assert_parses(&parser, "{x:@z}", object(&[("x", Value::Null)]));
    // Pair position: the looked-up map merges into the parent.
    assert_parses(&parser, "{@b}", object(&[("b", Value::Number(1.0))]));
}

#[test]
fn guide_gate_a_rule_modification_with_a_condition() {
    let mut parser = make_mini();
    apply(
        &mut parser,
        DirectiveOptions::new("gated", "gated<")
            .with_close(">")
            .with_action(|rule, _ctx| {
                set_node(rule, string("GATED"));
                Ok(())
            })
            .with_rules(
                RulesOption::new()
                    .open_rule("val", RuleMod::new())
                    .open_rule("pair", RuleMod::when(|rule, _ctx| rule.lte("pk", 0)))
                    .close_rules("list,elem,map,pair"),
            ),
    )
    .expect("gated registers");

    assert_parses(&parser, "gated<a>", string("GATED"));
    assert_parses(&parser, "[gated<a>]", Value::array(vec![string("GATED")]));
}

#[test]
fn guide_read_a_value_from_options() {
    let mut parser = make_mini();
    apply(
        &mut parser,
        DirectiveOptions::new("constant", "@").with_action_path("custom.x"),
    )
    .expect("constant registers");

    parser.set_plugin_options("custom", object(&[("x", Value::Number(42.0))]));
    assert_parses(&parser, "@y", Value::Number(42.0));

    // Resolution happens each time the directive fires, so a later
    // set_plugin_options call is picked up.
    parser.set_plugin_options("custom", object(&[("x", Value::Number(43.0))]));
    assert_parses(&parser, "@y", Value::Number(43.0));
}

#[test]
fn guide_token_action_forwards_a_token() {
    let mut parser = make_mini();
    apply(
        &mut parser,
        DirectiveOptions::new("tok", "tok<")
            .with_close(">")
            .with_token_action(|_rule, ctx| Ok(ctx.t.first().cloned())),
    )
    .expect("tok registers");

    // The action returns a token rather than assigning a node, so the
    // result is the empty map the `bo` hook seeded.
    assert_parses(&parser, "tok<a>", Value::Object(Default::default()));
}

#[test]
fn guide_run_extra_wiring_with_custom() {
    let mut parser = make_mini();
    apply(
        &mut parser,
        DirectiveOptions::new("subobj", "@")
            .with_action(|rule, _ctx| {
                set_node(rule, string("SUB"));
                Ok(())
            })
            .with_custom(|parser, config| {
                let open = config.open;
                let counter = format!("{}_top", config.name);
                let group = format!("{}-top", config.name);
                parser.define_rule("val", move |spec| {
                    let mut alt = AltSpec {
                        s: vec![vec![open]],
                        p: Some("map".into()),
                        b: 1,
                        n: HashMap::from([(counter, 1)]),
                        g: group,
                        ..Default::default()
                    };
                    alt.c_fn = Some(Arc::new(|rule, _ctx| rule.d == 0));
                    spec.prepend_open(alt);
                });
            }),
    )
    .expect("subobj registers");

    // The extra alt only fires at depth 0, so a nested directive still
    // takes the plugin's own path.
    assert_parses(&parser, "[@a]", Value::array(vec![string("SUB")]));
}

#[test]
fn guide_turn_off_all_default_rule_wiring() {
    let mut parser = make_mini();
    apply(
        &mut parser,
        DirectiveOptions::new("none", "@").with_rules(RulesOption::new()),
    )
    .expect("none registers");

    let error = parser
        .parse("[@a]")
        .expect_err("the open token is unrecognised");
    assert_eq!(error.code, "unexpected");
}

#[test]
fn reference_plugin_can_be_installed_directly() {
    // docs/reference.md § Rust API — `parser.use_plugin(plugin(options), None)`
    let mut parser = make_mini();
    parser
        .use_plugin(plugin(upper_options()), None)
        .expect("upper installs through use_plugin");

    assert_parses(&parser, "@a", string("A"));
}

#[test]
fn reference_empty_directive_body_is_undefined() {
    // rs/doc/reference.md § The action's rule — "Value::Undefined when
    // the directive is empty".
    let seen: Arc<std::sync::Mutex<Option<Value>>> = Arc::new(std::sync::Mutex::new(None));
    let recorder = seen.clone();

    let mut parser = make_mini();
    apply(
        &mut parser,
        DirectiveOptions::new("peek", "peek<")
            .with_close(">")
            .with_action(move |rule, _ctx| {
                *recorder.lock().unwrap() = Some(rule.child_node.clone());
                set_node(rule, string("PEEK"));
                Ok(())
            }),
    )
    .expect("peek registers");

    assert_parses(&parser, "peek<>", string("PEEK"));
    let body = seen.lock().unwrap().clone().expect("the action ran");
    assert!(
        body.is_undefined(),
        "an empty body is undefined, got {body}"
    );
}
