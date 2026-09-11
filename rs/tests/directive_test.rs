/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

//! Mirrors `ts/test/directive.test.ts` and `go/directive_test.go`. The
//! shared `test/spec/*.tsv` fixtures are the parity contract; the cases
//! that cannot be written as input → output live here.

mod common;

use common::mini_grammar::make_mini;
use common::spec::run_spec;

use tabnas::{Tabnas, Value};
use tabnas_directive::{
    apply, set_node, DirectiveError, DirectiveOptions, RuleMod, RulesOption, VERSION,
};

/// The TypeScript `'' + value` coercion the fixtures' actions rely on.
fn text_of(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Text(text) => text.string.clone(),
        Value::Undefined => "undefined".to_string(),
        other => other.to_string(),
    }
}

/// The numbers in a list body, ignoring anything that is not a number.
fn numbers(value: &Value) -> Vec<f64> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::Number(number) => Some(*number),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Register a directive, failing the test on error. Setup helper for the
/// many cases whose configuration is known to be valid.
fn must_apply(parser: &mut Tabnas, options: DirectiveOptions) {
    let name = options.name.clone();
    if let Err(error) = apply(parser, options) {
        panic!("apply({name:?}): {error}");
    }
}

/// A parser with the mini grammar and the "upper" directive: `@a` → `"A"`.
fn upper_parser() -> Tabnas {
    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("upper", "@").with_action(|rule, _context| {
            let body = text_of(&rule.child_node).to_uppercase();
            set_node(rule, Value::String(body));
            Ok(())
        }),
    );
    parser
}

#[test]
fn happy() {
    let parser = upper_parser();

    // The plugin registered its open token and its own rule.
    assert!(
        parser.fixed("@").is_some(),
        "the #OD_upper fixed token was not registered"
    );
    assert!(
        parser.rule_names().iter().any(|name| name == "upper"),
        "the upper rule was not installed"
    );

    run_spec(&parser, "happy.tsv");
}

#[test]
fn close() {
    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("foo", "foo<")
            .with_close(">")
            .with_action(|rule, _context| {
                set_node(rule, Value::String("FOO".into()));
                Ok(())
            }),
    );

    run_spec(&parser, "close-foo.tsv");

    // The close token also terminates an enclosing list/map opened inside
    // the directive (boundary closing).
    run_spec(&parser, "close-boundary.tsv");

    // A second directive sharing the same close token ">".
    must_apply(
        &mut parser,
        DirectiveOptions::new("bar", "bar<")
            .with_close(">")
            .with_action(|rule, _context| {
                set_node(rule, Value::String("BAR".into()));
                Ok(())
            }),
    );

    run_spec(&parser, "close-foo-bar.tsv");

    // Re-registering the same open token is an error, not a panic.
    let error = apply(&mut parser, DirectiveOptions::new("baz", "bar<"))
        .expect_err("re-registering an open token must fail");
    assert!(
        error.to_string().contains("bar<"),
        "error should name the duplicate token, got: {error}"
    );
}

#[test]
fn adder() {
    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("adder", "add<")
            .with_close(">")
            .with_action(|rule, _context| {
                // Fold from a literal +0.0: `Iterator::sum` folds from
                // -0.0, and the engine compares numbers bit-for-bit, so an
                // empty body would answer -0 where the fixture says 0.
                let total = numbers(&rule.child_node)
                    .iter()
                    .fold(0.0_f64, |total, value| total + value);
                set_node(rule, Value::Number(total));
                Ok(())
            }),
    );

    run_spec(&parser, "adder.tsv");

    // Implicit (bracketless) list bodies, e.g. add<1, 2, 3>.
    run_spec(&parser, "implicit.tsv");

    must_apply(
        &mut parser,
        DirectiveOptions::new("multiplier", "mul<")
            .with_close(">")
            .with_action(|rule, _context| {
                let values = numbers(&rule.child_node);
                let product = if values.is_empty() {
                    0.0
                } else {
                    values.iter().product()
                };
                set_node(rule, Value::Number(product));
                Ok(())
            }),
    );

    run_spec(&parser, "multiplier.tsv");

    // The adder still works after the second registration.
    run_spec(&parser, "adder.tsv");
}

#[test]
fn inject() {
    let source = Value::Object(
        [
            ("a".to_string(), Value::String("A".into())),
            (
                "b".to_string(),
                Value::Object(
                    [("b".to_string(), Value::Number(1.0))]
                        .into_iter()
                        .collect(),
                ),
            ),
            (
                "c".to_string(),
                Value::Array(vec![Value::Number(2.0), Value::Number(3.0)]),
            ),
        ]
        .into_iter()
        .collect(),
    );

    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("inject", "@")
            .with_rules(RulesOption::new().open_rules("val,pair"))
            .with_action(move |rule, _context| {
                let key = text_of(&rule.child_node);
                let Value::Object(entries) = &source else {
                    unreachable!("the lookup table is an object")
                };
                // A missing key is null, as in the TypeScript action.
                let value = entries.get(&key).cloned().unwrap_or(Value::Null);

                let at_pair = rule
                    .parent_rule
                    .as_ref()
                    .is_some_and(|parent| parent.name == "pair");
                if at_pair {
                    // The looked-up map merges into the surrounding map —
                    // the pair rule's node, which it shares with the map.
                    if let (Some(parent_node), Value::Object(fields)) =
                        (rule.parent_node.clone(), &value)
                    {
                        if let Value::Object(target) = &mut *parent_node.borrow_mut() {
                            for (field, field_value) in fields {
                                target.insert(field.clone(), field_value.clone());
                            }
                            return Ok(());
                        }
                    }
                }

                set_node(rule, value);
                Ok(())
            }),
    );

    run_spec(&parser, "inject.tsv");
}

#[test]
fn edges() {
    // An explicit empty RulesOption modifies no host rules, so the open
    // token is unrecognised. (TypeScript spells this `rules: null`; an
    // empty object there deep-merges with the defaults instead — an
    // intentional divergence, see docs/reference.md.)
    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("none", "@").with_rules(RulesOption::new()),
    );

    let error = parser
        .parse("[@a]")
        .expect_err("[@a] must fail when no host rule is modified");
    assert_eq!(error.code, "unexpected");
}

#[test]
fn action_option_prop() {
    // A string action resolves a dotted path against the parser options at
    // directive-execution time. The Rust Options struct is closed, like
    // Go's, so arbitrary option data lives in the plugin-options namespace
    // (TypeScript: `j.options({ custom: { x: 11 } })`).
    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("constant", "@").with_action_path("custom.x"),
    );

    // Set after registration: the path must resolve when the directive
    // fires, not when the plugin is applied.
    parser.set_plugin_options(
        "custom",
        Value::Object(
            [("x".to_string(), Value::Number(11.0))]
                .into_iter()
                .collect(),
        ),
    );

    let value = parser.parse("@y").expect("@y parses");
    assert!(
        value.deep_equal(&Value::Number(11.0)),
        "@y => {value}, want 11"
    );
}

#[test]
fn action_option_prop_walks_nested_segments() {
    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("nested", "@").with_action_path("custom.deep.x"),
    );

    parser.set_plugin_options(
        "custom",
        Value::Object(
            [(
                "deep".to_string(),
                Value::Object(
                    [("x".to_string(), Value::Number(7.0))]
                        .into_iter()
                        .collect(),
                ),
            )]
            .into_iter()
            .collect(),
        ),
    );

    let value = parser.parse("@y").expect("@y parses");
    assert!(
        value.deep_equal(&Value::Number(7.0)),
        "@y => {value}, want 7"
    );
}

#[test]
fn action_option_prop_missing_segments_resolve_to_undefined() {
    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("missing", "@").with_action_path("nope.deeper"),
    );

    // A missing path leaves the directive's node undefined, so the mini
    // grammar's `val` rule falls back to its own opening token — which,
    // for a directive, is the open token's source. TypeScript does exactly
    // the same: `tabnas.util.prop` answers undefined and `@val-bc` then
    // resolves `r.o0`. The assertion pins that fallback rather than an
    // undefined result.
    let value = parser.parse("@y").expect("@y parses");
    assert!(
        value.deep_equal(&Value::String("@".into())),
        "@y => {value}, want the val fallback \"@\""
    );
}

#[test]
fn rules_object_form() {
    // rules.open / rules.close carry a per-rule `c` condition for each
    // modified host rule. "elem" appears in both directions, so the second
    // lookup reuses the grammar-rule entry built by the first.
    let open_runs = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let close_runs = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let custom_name = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let custom_tokens = std::sync::Arc::new(std::sync::Mutex::new(None));

    let open_seen = open_runs.clone();
    let close_seen = close_runs.clone();
    let name_seen = custom_name.clone();
    let tokens_seen = custom_tokens.clone();

    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("cov", "cov<")
            .with_close(">")
            .with_action(|rule, _context| {
                set_node(rule, Value::String("COV".into()));
                Ok(())
            })
            .with_rules(
                RulesOption::new()
                    .open_rule(
                        "val",
                        RuleMod::when(move |_rule, _context| {
                            open_seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            true
                        }),
                    )
                    .open_rule("elem", RuleMod::new())
                    .close_rules("list,map,pair")
                    .close_rule(
                        "elem",
                        RuleMod::when(move |_rule, _context| {
                            close_seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            true
                        }),
                    ),
            )
            .with_custom(move |_parser, config| {
                *name_seen.lock().unwrap() = config.name.clone();
                *tokens_seen.lock().unwrap() = Some((config.open, config.close));
            }),
    );

    assert_eq!(*custom_name.lock().unwrap(), "cov");
    let (open_tin, close_tin) = custom_tokens.lock().unwrap().expect("custom callback ran");
    assert!(open_tin > 0, "OPEN tin resolved");
    assert!(close_tin.is_some(), "CLOSE tin resolved");

    let value = parser.parse("cov<[1, 2>").expect("cov<[1, 2> parses");
    assert!(
        value.deep_equal(&Value::String("COV".into())),
        "cov<[1, 2> => {value}"
    );
    assert!(
        open_runs.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "the open condition was never evaluated"
    );
    assert!(
        close_runs.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "the close condition was never evaluated"
    );
}

#[test]
fn action_returns_token() {
    // When the action returns a token, the close hook forwards it. This
    // token carries no error code, so the parse succeeds and the directive
    // node is the empty map seeded by the open hook.
    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("tok", "tok<")
            .with_close(">")
            .with_token_action(|_rule, context| Ok(context.t.first().cloned())),
    );

    let value = parser.parse("tok<a>").expect("tok<a> parses");
    assert!(
        value.deep_equal(&Value::Object(Default::default())),
        "tok<a> => {value}, want an empty map"
    );
}

#[test]
fn action_error_aborts_the_parse() {
    // The Rust spelling of a TypeScript action returning an ERROR token:
    // an Err from the action halts the parse under its own code.
    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("bad", "bad<")
            .with_close(">")
            .with_action(|_rule, _context| {
                Err(tabnas::ActionError::new("directive_failed", "no good"))
            }),
    );

    let error = parser
        .parse("bad<a>")
        .expect_err("a failing action must abort the parse");
    assert_eq!(error.code, "directive_failed");
    assert_eq!(error.detail, "no good");
}

#[test]
fn open_only_directive_does_not_consume_following_siblings() {
    // Without a close token the directive rule sets dlist/dmap to 1, so an
    // implicit list cannot swallow the directive's following siblings.
    let parser = upper_parser();
    let value = parser.parse("[@a, @b, 2]").expect("parses");
    assert!(
        value.deep_equal(&Value::Array(vec![
            Value::String("A".into()),
            Value::String("B".into()),
            Value::Number(2.0),
        ])),
        "[@a, @b, 2] => {value}"
    );
}

#[test]
fn rules_option_parses_comma_separated_names() {
    // Whitespace and empty entries are dropped, matching the TypeScript
    // `resolveRules` string form.
    let rules = RulesOption::new().open_rules(" val , pair ,, ");
    let names: Vec<&str> = rules.open.keys().map(String::as_str).collect();
    assert_eq!(names, ["pair", "val"]);
    assert!(rules.close.is_empty());
}

#[test]
fn default_rules_are_used_when_rules_is_absent() {
    // An absent `rules` selects open:"val", close:"list,elem,map,pair" —
    // and the close-direction default is what lets ">" terminate a list
    // opened inside the directive.
    let mut parser = make_mini();
    must_apply(
        &mut parser,
        DirectiveOptions::new("mrg", "mrg<")
            .with_close(">")
            .with_action(|rule, _context| {
                set_node(rule, Value::String("MRG".into()));
                Ok(())
            }),
    );

    let value = parser.parse("[mrg<1>]").expect("[mrg<1>] parses");
    assert!(
        value.deep_equal(&Value::Array(vec![Value::String("MRG".into())])),
        "[mrg<1>] => {value}"
    );

    let value = parser.parse("mrg<[1, 2>").expect("mrg<[1, 2> parses");
    assert!(
        value.deep_equal(&Value::String("MRG".into())),
        "mrg<[1, 2> => {value}"
    );
}

#[test]
fn deriving_an_instance_that_already_has_the_directive_reports_the_duplicate() {
    // `apply` goes through use_plugin, so `derive` re-runs the plugin
    // against the child's options — and a derived instance inherits the
    // parent's fixed tokens, so the open token is already claimed. The
    // canonical engine behaves the same way: TypeScript's `make()`
    // re-runs inherited plugins over merged options, and the plugin's
    // `tabnas.fixed(open)` check then throws.
    //
    // Derive from a parser WITHOUT the directive and apply it to the
    // child instead.
    let parser = upper_parser();
    // Tabnas is not Debug, so unwrap the Result by hand rather than
    // through expect_err.
    let Err(error) = parser.derive(|options| options.tag = "child".into()) else {
        panic!("re-running the plugin must re-register the open token");
    };
    assert!(
        error.0.contains("already in use"),
        "expected a duplicate-open-token error, got: {}",
        error.0
    );

    // The parent is untouched by the failed derive.
    let value = parser.parse("@a").expect("@a still parses");
    assert!(
        value.deep_equal(&Value::String("A".into())),
        "@a => {value}"
    );
}

#[test]
fn directive_error_converts_to_and_from_plugin_error() {
    let error = DirectiveError("boom".into());
    let plugin_error: tabnas::PluginError = error.clone().into();
    assert_eq!(plugin_error.0, "boom");
    assert_eq!(DirectiveError::from(plugin_error), error);
    assert_eq!(error.to_string(), "boom");
}

#[test]
fn version_is_exported() {
    assert!(!VERSION.is_empty());
}
