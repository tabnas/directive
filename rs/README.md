# tabnas-directive (Rust)

Directive-syntax plugin for the
[`tabnas`](https://github.com/tabnas/parser) parser engine.

A *directive* is a token sequence, `@name` (open-only) or `add<1,2>`
(open + close), that pushes into a dedicated rule and fires an action
to transform the parsed body. This is the Rust port of the canonical
TypeScript implementation in [`../ts`](../ts); the TypeScript version is
authoritative and this crate tracks it. A few intentional differences
(static typing, engine-API shape) are listed in
[the concepts doc](doc/concepts.md#differences-from-the-ts-version) and
tabulated in [`../docs/reference.md`](../docs/reference.md).

## Documentation

The four-quadrant Rust docs live in [`doc/`](doc):
[tutorial](doc/tutorial.md) · [how-to guide](doc/guide.md) ·
[reference](doc/reference.md) · [concepts](doc/concepts.md). The
canonical TypeScript docs are in [`../ts/doc/`](../ts/doc).

The plugin's only dependency is the tabnas engine (the `tabnas` crate).
It modifies host-grammar rules (`val`, `list`, `elem`, `map`, `pair`
by default), so you apply it to a `Tabnas` instance that already has a
grammar installed, not a bare engine. A minimal host grammar is in
[`tests/common/mini_grammar.rs`](tests/common/mini_grammar.rs).

## Install

The `tabnas` crate is not published to a registry, so the engine is
consumed as a **sibling checkout**, the standard tabnas development
model. Clone `https://github.com/tabnas/parser` next to this repository
and point at it:

```toml
[dependencies]
tabnas = { path = "../parser/rs" }
tabnas-directive = { path = "../directive/rs" }
```

## Use

```rust
use tabnas::{Tabnas, Value};
use tabnas_directive::{apply, set_node, DirectiveOptions};

let mut parser = Tabnas::new();
register_host_grammar(&mut parser); // provides val / list / map / pair

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

let value = parser.parse("[@a, @b, 1]")?; // ["A", "B", 1]
```

`set_node` is not decoration. A rule pushed by the engine SHARES its
parent's node cell, so writing through `rule.node.borrow_mut()` would
overwrite the parent's node as well; `set_node` installs a fresh cell,
which is what `rule.node = …` means in the canonical TypeScript engine.
Borrow the cell directly only to mutate a container the rule genuinely
shares: pushing onto an enclosing list, or merging into the map behind
a `pair`.

## Build and test

This crate takes the engine as a path dependency on the sibling
checkout, so there is nothing to fetch by hand:

```bash
cargo build --all-targets
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

Or, from the repository root, `make test-rs` runs the tests and Clippy.

The suite runs the shared `../test/spec/*.tsv` conformance fixtures
(the same files the TypeScript and Go suites run) against the mini host
grammar. A row green in one runtime and red in another is a failure, not
a discrepancy.

## License

MIT.
