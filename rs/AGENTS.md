# Agent guide: rs/ (parity)

This is the Rust port of `@tabnas/directive`. It is **not** canonical:
it tracks the TypeScript implementation in `../ts`, which is the source
of truth. See [../AGENTS.md](../AGENTS.md) for the parity rules and
[../docs/reference.md](../docs/reference.md) for the full list of
intentional TS / Go / Rust differences.

- Source: `src/lib.rs`. Provides `plugin(options) -> Plugin` and
  `apply(parser, options)` (the convenience constructor over
  `Tabnas::use_plugin`), the option types `DirectiveOptions`,
  `DirectiveAction`, `RulesOption`, `RuleMod`, `DirectiveConfig`,
  `DirectiveError`, and the `set_node` helper.
- Tests: `tests/directive_test.rs`, driven by the shared
  `../test/spec/*.tsv` fixtures and mirroring
  `../ts/test/directive.test.ts` and `../go/directive_test.go`. The host
  grammar the tests run against is `tests/common/mini_grammar.rs`
  (`make_mini()`), the Rust twin of `../ts/test/mini-grammar.ts` — keep
  the three in step. `tests/version_test.rs` pins the version constants.
- Crate `tabnas-directive`, library `tabnas_directive`. The engine crate
  `tabnas` is a **path dependency on the sibling checkout**
  (`../../parser/rs`) — it is not published to a registry, so there is
  no version to fall back on. Clone `https://github.com/tabnas/parser`
  next to this repo.

```bash
cargo build --all-targets
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt
```

## Dependency wiring

- The only runtime dependency is the tabnas parser engine, plus
  `indexmap` and `serde_json` for the value and grammar types the engine
  itself exposes. The plugin imports its plugin-API types from the
  engine: `tabnas::{Tabnas, Rule, Context, Tin, Token, Value, Plugin,
  GrammarSpec, GrammarSetting, ActionError, PluginError}`.
- The engine ships no grammar, so the tests install the small
  `tests/common/mini_grammar.rs` grammar (`val` / `list` / `map` /
  `pair` / `elem`) on a bare `Tabnas::new()` instance, then apply the
  directive.

## Plugin registration

`plugin(options)` builds a `tabnas::Plugin` named `Directive`; `apply`
installs it via `use_plugin`, so it re-runs against a derived instance's
options like every native plugin. The typed option data lives in the
returned closure rather than in the engine's serialized plugin-option
bag, because a directive's action, conditions and custom hook are Rust
callbacks and no `Value` can carry them.

Grammar is installed the same way the Go port does it: a serialized
`GrammarSpec` document naming `@dr-*` condition references that the
plugin registers on the instance first, applied with
`GrammarSetting::groups("directive")` so every alt carries the
`directive` group tag.

The directive rule's `bo` / `bc` hooks are registered as
`@<name>-bo` / `@<name>-bc` through `state_action_with_next_ref`, which
the engine's `@<rule>-<phase>` convention wires automatically when the
`<name>` rule is installed. `state_action_with_next_ref` is the shape
that can return a `Token`, which is how a TypeScript action's
`Token | void` return survives the port.

## The shared node cell

The one Rust-specific hazard. A rule pushed (or replaced) by the engine
shares its parent's `Rc<RefCell<Value>>` node cell, so
`*rule.node.borrow_mut() = value` overwrites the PARENT's node too. An
assignment — `rule.node = …` in TypeScript, `rule.Node = …` in Go — must
install a fresh cell instead. That is what the exported `set_node` does,
and what the engine's own builtin `@val-bo` / `@map-bo` / `@list-bo`
actions do.

Borrow the cell directly only to mutate a container the rule genuinely
shares: `@elem-bc` pushing onto the enclosing list, `@pair-bc` writing
into the enclosing map, or the `inject` test merging a looked-up map
into `rule.parent_node`.

Get this wrong and the symptom is confusing rather than loud: the
directive's `bo` hook seeding `{}` would also blank the host `val`
rule's node, and `val` would then answer `{}` instead of adopting the
directive's result.

## Spec fixtures

`../test/spec/*.tsv` is the parity contract. TypeScript and Go read the
files through `@tabnas/support`; Rust has no support crate, so the
loader lives in `tests/common/spec.rs` and must keep to the same codec —
see [`../test/AGENTS.md`](../test/AGENTS.md). A new fixture has to be
wired into all three runtimes by hand, because what varies per case is
the DIRECTIVE and a directive is a function.
