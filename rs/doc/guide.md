# How-to guides (Rust)

Focused recipes for real tasks. Each assumes you have read the
[tutorial](tutorial.md) and can register a basic directive. This is the
Rust port of the [TypeScript how-to guides](../../ts/doc/guide.md).

The examples use two crates: `tabnas` (the engine, and the `Rule` /
`Context` / `Value` / `AltSpec` types) and `tabnas_directive`. They
register against a `parser` that already has a host grammar installed:

```rust
use tabnas::{Tabnas, Value};
use tabnas_directive::{apply, set_node, DirectiveOptions, RuleMod, RulesOption};

let mut parser = Tabnas::new();
register_host_grammar(&mut parser); // provides val / list / map / pair
```

`apply` returns `Result<(), DirectiveError>`; the examples elide the
error for brevity — check it in real code. The plugin never panics.


## Wrap an arbitrary body with a close token

Use `with_close` when a directive should consume everything up to a
closing token rather than a single value.

```rust
apply(
    &mut parser,
    DirectiveOptions::new("group", "(")
        .with_close(")")
        .with_action(|rule, _ctx| {
            let body = rule.child_node.clone();
            set_node(rule, body);
            Ok(())
        }),
)?;
```


## Share a close token between two directives

Register a second directive with the same close string. The plugin
detects that the close character is already a registered fixed token and
**reuses** it (the lexer cannot hold two mappings for one sequence):

```rust
apply(&mut parser, DirectiveOptions::new("foo", "foo<").with_close(">").with_action(foo))?;
apply(&mut parser, DirectiveOptions::new("bar", "bar<").with_close(">").with_action(bar))?;
// parser.parse("[foo<a>, bar<b>]") -> ["FOO", "BAR"]
```

The **open** tokens (`foo<`, `bar<`) must still be unique. Reusing an
open token returns an error (the plugin never panics):

```rust
let error = apply(&mut parser, DirectiveOptions::new("baz", "foo<")).unwrap_err();
// "Directive open token already in use: foo<"
```


## Restrict where a directive is recognised

By default the open token is wired into `val`, so it matches in any
value position. Pass `rules` to narrow it. The comma-string shorthand is
spelled as a builder method:

```rust
apply(
    &mut parser,
    DirectiveOptions::new("inject", "@")
        .with_rules(RulesOption::new().open_rules("val,pair"))
        .with_action(inject),
)?;
```

Note that `with_rules` is a **complete override**, as in Go: supplying
only the open direction drops the close-rule defaults too. TypeScript
deep-merges its defaults into a partial `rules` instead — an intentional
divergence, tabulated in [`../../docs/reference.md`](../../docs/reference.md).
Set both directions when you need both.


## Merge into the surrounding map at a `pair` position

A directive that can appear as a whole map entry (a `pair`) as well as a
value can, in the `pair` branch, mutate the parent's node instead of
setting its own — for example to merge a looked-up map into the
surrounding map.

This is the one place to borrow the node cell rather than call
`set_node`: `rule.parent_node` is the cell the `pair` rule shares with
its `map`, and mutating it in place is exactly the intent.

```rust
.with_action(move |rule, _ctx| {
    let key = match &rule.child_node {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    let value = lookup(&key).unwrap_or(Value::Null); // missing key -> null

    let at_pair = rule
        .parent_rule
        .as_ref()
        .is_some_and(|parent| parent.name == "pair");
    if at_pair {
        if let (Some(parent_node), Value::Object(fields)) = (rule.parent_node.clone(), &value) {
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
})
```


## Gate a rule modification with a condition

Use `RuleMod::when`. The directive only matches inside that host rule
when the condition returns `true`:

```rust
.with_rules(
    RulesOption::new()
        .open_rule("val", RuleMod::new())
        .open_rule("pair", RuleMod::when(|rule, _ctx| rule.lte("pk", 0)))
        .close_rules("list,elem,map,pair"),
)
```

The condition receives `(&mut Rule, &mut Context)` and returns `bool` —
the same shape the engine's `alt_condition` uses everywhere.


## Read a value from options (path-action form)

Like TypeScript's `action: 'a.b.c'`, a path action resolves a dotted
path on the parser options at fire time. The TS options object is open
(arbitrary top-level keys), while Rust's `Options` struct is closed —
like Go's — so the Rust path resolves in the plugin-options namespace:

```rust
apply(
    &mut parser,
    DirectiveOptions::new("constant", "@").with_action_path("custom.x"),
)?;
parser.set_plugin_options(
    "custom",
    Value::Object([("x".to_string(), Value::Number(42.0))].into_iter().collect()),
);
// parser.parse("@y") -> 42
```

Resolution happens each time the directive fires, so later
`set_plugin_options` calls are picked up. A closure capturing a value
works just as well when the value is not option-driven.


## Report a failure from an action

Return `Err(ActionError)` to abort the parse under your own code — the
Rust spelling of a TypeScript action returning an error token:

```rust
.with_action(|rule, _ctx| match &rule.child_node {
    Value::Number(_) => Ok(()),
    _ => Err(tabnas::ActionError::new("strict_body", "expected a number")),
})
// parser.parse("strict<a>") -> Err, error.code == "strict_body"
```

To hand a token back to the engine instead — the full TypeScript
`Token | void` contract — use `with_token_action`. A token carrying an
error code halts the parse; any other token is forwarded and otherwise
ignored:

```rust
.with_token_action(|_rule, ctx| Ok(ctx.t.first().cloned()))
```


## Run extra wiring after setup with `with_custom`

The custom hook fires last and is handed the resolved `open` / `close`
tins and the directive name, so you can install your own alternates that
reference the directive's tokens without re-resolving them:

```rust
use tabnas::AltSpec;
use std::collections::HashMap;

apply(
    &mut parser,
    DirectiveOptions::new("subobj", "@")
        .with_action(action)
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
                alt.c_fn = Some(std::sync::Arc::new(|rule, _ctx| rule.d == 0));
                spec.prepend_open(alt);
            });
        }),
)?;
```


## Turn off all default rule wiring

A `rules` of `None` means "use the defaults". To modify **no** host
rules, pass an explicit empty `RulesOption`. Only the directive's own
rule is then created, so its open token is unrecognised unless you
install alternates in the custom hook:

```rust
apply(
    &mut parser,
    DirectiveOptions::new("none", "@").with_rules(RulesOption::new()),
)?;

let error = parser.parse("[@a]").unwrap_err(); // error.code == "unexpected"
```


## Test a directive against a shared spec file

Conformance rows live in `../../test/spec/*.tsv`. Each row is one of:

```
<input><TAB><expected-json>
<input><TAB>ERROR:<code>
```

Blank lines and `#`-prefixed lines are ignored. The TypeScript, Go and
Rust suites load the same files, so a new row is exercised by all three
runtimes — but a new FILE has to be wired into each suite by hand,
because what varies per case is the directive and a directive is a
function.

```rust
mod common;
use common::mini_grammar::make_mini;
use common::spec::run_spec;

#[test]
fn my_directive() {
    let mut parser = make_mini();
    apply(&mut parser, DirectiveOptions::new("upper", "@").with_action(upper)).unwrap();
    run_spec(&parser, "happy.tsv");
}
```

Run with `cargo test --all-targets` (Rust), `go test ./...` (Go) and
`npm test` (TS), or `make test` from the repo root.
