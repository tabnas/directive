# Tutorial: your first directive (Rust)

This tutorial takes you from nothing to a working parser that
understands a custom *directive*. It is the Rust port of the
[TypeScript tutorial](../../ts/doc/tutorial.md); the TypeScript
implementation is canonical and this crate (`tabnas_directive`) tracks
it.

A directive is a plugin for the
[tabnas](https://github.com/tabnas/parser) parser engine. The engine
ships **no grammar of its own**, so a directive cannot stand alone: you
bring a **host grammar** (one that defines the usual `val` / `list` /
`map` / `pair` rules) and the directive layers onto it. You apply it to
a `Tabnas` instance that already has that grammar installed.

Below, `register_host_grammar` stands for your host grammar. The repo's
own test host (scalars, `[a, b]` lists, `{k: v}` maps) lives in
[`tests/common/mini_grammar.rs`](../tests/common/mini_grammar.rs).


## 1. Install

The `tabnas` crate is not published to a registry, so the engine is
consumed as a **sibling checkout**. Clone
`https://github.com/tabnas/parser` next to this repository and point at
it:

```toml
[dependencies]
tabnas = { path = "../parser/rs" }
tabnas-directive = { path = "../directive/rs" }
```

You use two crates: the engine `tabnas` (for the `Rule` / `Context` /
`Value` types your action uses) and the directive plugin
`tabnas_directive`.


## 2. Register an open-only directive

The simplest directive has just an **open** token; it consumes the one
value that follows. Here `@` uppercases the next value.

```rust
use tabnas::{Tabnas, Value};
use tabnas_directive::{apply, set_node, DirectiveOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Tabnas::new();
    register_host_grammar(&mut parser); // provides val / list / map / pair

    // apply returns Result. The plugin never panics — a duplicate open
    // token or a grammar build failure comes back as Err.
    apply(
        &mut parser,
        DirectiveOptions::new("upper", "@").with_action(|rule, _ctx| {
            let body = match &rule.child_node {
                Value::String(text) => text.to_uppercase(),
                other => other.to_string().to_uppercase(),
            };
            set_node(rule, Value::String(body));
            Ok(())
        }),
    )?;

    println!("{}", parser.parse("@hello")?); // "HELLO"
    Ok(())
}
```

What each piece does:

- `DirectiveOptions::new(name, open)` — the directive's name (the plugin
  creates a parse rule with this name and uses it as a token-name
  suffix) and the character sequence that triggers it.
- `with_action(f)` — a callback run once the body has parsed. The body's
  value is `rule.child_node`; call `set_node` to set the result.

**Use `set_node`, not `rule.node.borrow_mut()`.** A rule pushed by the
engine SHARES its parent's node cell, so writing through the cell would
overwrite the parent's node too. `set_node` installs a fresh cell, which
is what `rule.node = …` means in the canonical TypeScript engine. Borrow
the cell directly only to mutate a container the rule genuinely shares —
see [How-to guides](guide.md#merge-into-the-surrounding-map-at-a-pair-position).


## 3. Use it inside structures

The open token is wired (by default) into the host's `val` rule, so the
directive works anywhere a value is allowed:

```rust
for src in ["@hello", "[@a, @b, 1]", "{x:@a, y:@b}"] {
    println!("{src} -> {}", parser.parse(src)?);
}
// @hello       -> "HELLO"
// [@a, @b, 1]  -> ["A","B",1]
// {x:@a, y:@b} -> {"x":"A","y":"B"}
```

(The bare-word and unquoted-key syntax above is what the repo's mini
host grammar accepts; a strict-JSON host would require quotes.)


## 4. Add a close token

An open-only directive grabs one value. To wrap an arbitrary body, add a
**close** token; the directive then consumes everything between open and
close. This `sum<...>` directive sums the numbers in its list body:

```rust
let mut parser = Tabnas::new();
register_host_grammar(&mut parser);

apply(
    &mut parser,
    DirectiveOptions::new("sum", "sum<")
        .with_close(">")
        .with_action(|rule, _ctx| {
            let mut total = 0.0_f64;
            if let Value::Array(items) = &rule.child_node {
                for item in items {
                    if let Value::Number(number) = item {
                        total += number;
                    }
                }
            }
            set_node(rule, Value::Number(total));
            Ok(())
        }),
)?;

println!("{}", parser.parse("sum<[1, 2, 3]>")?); // 6
```


## 5. Boundary closing

A close token also terminates a list or map opened **inside** the
directive — you do not have to close the inner bracket first:

```rust
println!("{}", parser.parse("sum<[1, 2>")?); // 3 — note: no ']' before '>'
```

The `>` closes both the open list and the directive at once. See the
[concepts](concepts.md) doc for why.


## 6. Handle failure

Registration and actions both report failure through `Result` rather
than panicking:

```rust
// A duplicate open token is a registration error.
let err = apply(&mut parser, DirectiveOptions::new("dup", "sum<")).unwrap_err();
// "Directive open token already in use: sum<"

// An action can reject a document it cannot transform.
DirectiveOptions::new("strict", "strict<")
    .with_close(">")
    .with_action(|rule, _ctx| match &rule.child_node {
        Value::Number(_) => Ok(()),
        _ => Err(tabnas::ActionError::new("strict_body", "expected a number")),
    });
// parser.parse("strict<a>") -> Err, error.code == "strict_body"
```


## Where to go next

- [How-to guides](guide.md) — focused recipes.
- [Reference](reference.md) — every option, type and counter.
- [Concepts](concepts.md) — the engine relationship, the design
  trade-offs, and the differences from the TypeScript version.
