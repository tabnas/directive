# Reference

Complete API listing for the directive plugin. For an orientation on
how to use these pieces, see the [Tutorial](tutorial.md) or the
[How-to guides](how-to.md).

The TypeScript (canonical), Go, and Rust implementations each have
their own API section below.

The plugin's only dependency is the
[tabnas](https://github.com/tabnas/parser) parser engine; its types
(`Rule`, `Context`, `Tin`, …) come from there. The plugin modifies host
grammar rules (`val` / `list` / `map` / `pair`), so it is applied to an
engine instance that already has a grammar installed.


## TypeScript API

### `Directive` (plugin)

A `Plugin`. Register via `j.use(Directive, options)`.

```ts
import { Directive, DirectiveOptions } from '@tabnas/directive'
```

### `DirectiveOptions`

| Field     | Type                                  | Required | Description                                                             |
| --------- | ------------------------------------- | -------- | ----------------------------------------------------------------------- |
| `name`    | `string`                              | yes      | Directive name. Also used as the rule name and the token-name suffix.   |
| `open`    | `string`                              | yes      | Character sequence that starts the directive. Must be unique per instance. |
| `close`   | `string`                              | no       | Character sequence that ends the directive. If omitted the directive consumes a single value. |
| `action`  | `StateAction \| string`               | yes      | Function called when the directive closes, or a dotted path into the instance options. |
| `rules`   | `RulesOption \| null`                 | no       | Which existing grammar rules detect this directive. See [Rules](#rules-defaults). `null` → modify no rules. |
| `custom`  | `(tabnas, config) => void`            | no       | Callback invoked after setup. `config = { OPEN, CLOSE, name }`.         |

### `RulesOption`

```ts
type RulesOption = {
  open?:  string | string[] | Record<string, { c?: Function }>
  close?: string | string[] | Record<string, { c?: Function }>
}
```

String / string-array forms are split on commas and treated as a set
of rule names with no conditions. The record form lets you attach a
per-rule condition via `c`.

### `action(rule, ctx, next, tkn)`

| Arg    | Type      | Notes                                     |
| ------ | --------- | ----------------------------------------- |
| `rule` | `Rule`    | The directive rule. Set `rule.node` to assign the result. |
| `ctx`  | `Context` | Parse context.                            |
| `next` | `Rule`    | Rule to be processed next (useful in `bc` replacements). |
| `tkn`  | `Token?`  | Matching token.                           |

Assign `rule.node` to set the directive's result value. If `rule.parent`
is a pair you can mutate `rule.parent.node` instead and leave
`rule.node` alone. Returning a token overrides the next token (advanced).


## Go API

### `Directive` (plugin function)

```go
import (
    tabnas "github.com/tabnas/parser/go"
    directive "github.com/tabnas/directive/go"
)

// Apply returns (instance, error). The plugin never panics — every
// failure path is reported through the error.
j, err := directive.Apply(j, directive.DirectiveOptions{ ... })
// or, registering the raw plugin with named option keys:
err = j.Use(directive.Directive, map[string]any{
    "name": "upper", "open": "@", "action": action,
})
```

`j` is any `*tabnas.Tabnas` instance with a host grammar installed (one
that defines the `val` / `list` / `map` / `pair` rules).

### `Apply`

```go
func Apply(j *tabnas.Tabnas, opts DirectiveOptions) (*tabnas.Tabnas, error)
```

`Apply` registers the directive and returns any registration error (a
duplicate open token, or a grammar build failure). The plugin never
panics, so callers always get an `error` to handle.

### `DirectiveOptions`

| Field     | Type                            | Required | Description                                                             |
| --------- | ------------------------------- | -------- | ----------------------------------------------------------------------- |
| `Name`    | `string`                        | yes      | Directive name. Rule name and token-name suffix.                        |
| `Open`    | `string`                        | yes      | Open character sequence. Must be unique per instance.                   |
| `Close`   | `string`                        | no       | Close character sequence. Empty → directive consumes a single value.    |
| `Action`  | `Action \| TokenAction \| string` | yes      | Callback invoked when the directive closes, or a dotted path resolved in the plugin-options namespace. |
| `Rules`   | `*RulesOption`                  | no       | Rule modifications. `nil` → defaults. `&RulesOption{}` → no rules.      |
| `Custom`  | `CustomFunc`                    | no       | Callback after setup. Argument: `DirectiveConfig{OPEN, CLOSE, Name}`.   |

### `RulesOption`, `RuleMod`

```go
type RulesOption struct {
    Open  map[string]*RuleMod
    Close map[string]*RuleMod
}
type RuleMod struct {
    C tabnas.AltCond // optional per-rule condition
}
```

### `Action`

```go
type Action func(rule *tabnas.Rule, ctx *tabnas.Context)
```

### `CustomFunc`, `DirectiveConfig`

```go
type CustomFunc      func(j *tabnas.Tabnas, config DirectiveConfig)
type DirectiveConfig struct {
    OPEN  tabnas.Tin
    CLOSE tabnas.Tin // -1 if no close token
    Name  string
}
```


## Rust API

### `plugin`, `apply`

```rust
use tabnas::Tabnas;
use tabnas_directive::{apply, plugin, DirectiveOptions};

// apply registers the directive through Tabnas::use_plugin, so it is
// re-applied to derived instances. The plugin never panics — every
// failure path comes back as Err(DirectiveError).
apply(&mut parser, DirectiveOptions::new("upper", "@").with_action(action))?;

// Or build the Plugin value and install it yourself:
parser.use_plugin(plugin(DirectiveOptions::new("upper", "@")), None)?;
```

`parser` is any `Tabnas` instance with a host grammar installed (one
that defines the `val` / `list` / `map` / `pair` rules).

```rust
pub fn plugin(options: DirectiveOptions) -> Plugin;
pub fn apply(parser: &mut Tabnas, options: DirectiveOptions)
    -> Result<(), DirectiveError>;
```

### `DirectiveOptions`

Built with `DirectiveOptions::new(name, open)` plus the `with_*`
methods; the fields are public too.

| Field    | Type                    | Required | Description                                                             |
| -------- | ----------------------- | -------- | ----------------------------------------------------------------------- |
| `name`   | `String`                | yes      | Directive name. Rule name and token-name suffix.                        |
| `open`   | `String`                | yes      | Open character sequence. Must be unique per instance.                   |
| `close`  | `Option<String>`        | no       | Close character sequence. `None` (or empty) → the directive consumes a single value. |
| `action` | `DirectiveAction`       | no       | How the parsed body is transformed. Default `None`.                     |
| `rules`  | `Option<RulesOption>`   | no       | Rule modifications. `None` → defaults. `Some(RulesOption::new())` → no rules. |
| `custom` | `Option<CustomFn>`      | no       | Callback after setup. Argument: `&DirectiveConfig { open, close, name }`. |

| Builder                      | Sets                                                |
| ---------------------------- | --------------------------------------------------- |
| `with_close(close)`          | `close`                                             |
| `with_action(f)`             | `action = DirectiveAction::Call(f)`                 |
| `with_token_action(f)`       | `action = DirectiveAction::Token(f)`                |
| `with_action_path(path)`     | `action = DirectiveAction::Path(path)`              |
| `with_rules(rules)`          | `rules`                                             |
| `with_custom(f)`             | `custom`                                            |

### `DirectiveAction`

```rust
pub enum DirectiveAction {
    None,
    Call(ActionFn),   // Fn(&mut Rule, &mut Context) -> Result<(), ActionError>
    Token(TokenActionFn), // …-> Result<Option<Token>, ActionError>
    Path(String),     // dotted path into the plugin-options namespace
}
```

### `set_node`

```rust
pub fn set_node(rule: &mut Rule, value: Value);
```

A rule pushed by the engine SHARES its parent's node cell, so writing
through `rule.node.borrow_mut()` would overwrite the parent's node too.
`set_node` installs a fresh cell, which is what `rule.node = …` means in
the canonical TypeScript engine. Borrow the cell directly only to mutate
a container the rule genuinely shares (pushing onto an enclosing list,
or merging into the map behind a `pair`).

### `RulesOption`, `RuleMod`

```rust
pub struct RulesOption {
    pub open:  BTreeMap<String, RuleMod>,
    pub close: BTreeMap<String, RuleMod>,
}
pub struct RuleMod {
    pub c: Option<ConditionFn>, // optional per-rule condition
}
```

`RulesOption::new()` modifies nothing; `.open_rules("val,pair")` and
`.close_rules(…)` take the comma-separated form (whitespace and empty
names are dropped), and `.open_rule(name, RuleMod::when(cond))` /
`.close_rule(…)` add one rule with a condition. Rules are held in a
`BTreeMap`, so installation order is deterministic.

### `CustomFn`, `DirectiveConfig`

```rust
pub type CustomFn = Arc<dyn Fn(&mut Tabnas, &DirectiveConfig) + Send + Sync>;
pub struct DirectiveConfig {
    pub open:  Tin,
    pub close: Option<Tin>, // None if no close token
    pub name:  String,
}
```

### `DirectiveError`

A registration failure: an unusable name (empty, or containing
whitespace), a duplicate open token, or a grammar the engine refused.
Converts to and from `tabnas::PluginError`, so `apply` and `use_plugin`
report the same thing.


## Rules defaults

When `rules` / `Rules` is omitted:

| Direction | Default rule names         |
| --------- | -------------------------- |
| `open`    | `val`                      |
| `close`   | `list,elem,map,pair`       |

Meaning: the directive's open token is recognised in any value
position; the close token ends sibling parsing in list / element /
map / pair rules.


## Tokens

For a directive named `NAME` the plugin registers:

| Token name   | When                   | Fixed token value |
| ------------ | ---------------------- | ----------------- |
| `#OD_<NAME>` | always                 | `open`            |
| `#CD_<NAME>` | only if `close` set AND `close` isn't already a fixed token | `close` |

When `close` collides with an existing fixed token (for example, a
shared close across directives) the existing token is reused and no new
`#CD_<NAME>` token is created.


## Group tags

Every alt installed by the plugin carries `directive` as one of its
`g` tags. The per-alt tags (in addition) are:

| Context               | Tag(s)         |
| --------------------- | -------------- |
| Open-rule OPEN alt    | `start`        |
| Open-rule OPEN+CLOSE  | `start,end`    |
| Close-rule CLOSE      | `end`          |
| Close-rule `,CLOSE`   | `end,comma`    |


## Counters

While inside a directive, `rule.n['dr_<NAME>'] === 1`. This is how
the close-rule alts recognise they should close.

`dlist` and `dmap` counters control whether implicit lists / maps
are permitted inside the directive body:

| `close` present? | `dlist` / `dmap` set to |
| ---------------- | ----------------------- |
| yes              | 0 (implicits allowed)   |
| no               | 1 (implicits suppressed so trailing siblings aren't consumed) |


## Errors

| Situation                                       | TS behaviour               | Go behaviour              | Rust behaviour            |
| ----------------------------------------------- | -------------------------- | ------------------------- | ------------------------- |
| Registering a directive whose `open` is already fixed | `throw` Error             | `Apply` / `j.Use` return an `error` (no panic) | `apply` / `use_plugin` return `Err` (no panic) |
| Grammar build failure during registration       | `throw` (engine)           | `Apply` / `j.Use` return an `error` (no panic) | `apply` / `use_plugin` return `Err` (no panic) |
| Parsing a close token without its open          | engine `unexpected` error  | engine `unexpected` error | engine `unexpected` error |
| An action reporting failure                     | return an error `Token`    | return a `*tabnas.Token` with `Err` set | return `Err(ActionError)`, or a token carrying an error code |


## TypeScript / Go / Rust differences

TypeScript is canonical; the Go and Rust ports mirror its behaviour. All
three pass the identical shared `test/spec/*.tsv` conformance fixtures.
The following differences are intentional. They stem from static typing
and from engine-API differences, not from drift:

| Area | TypeScript | Go | Rust |
| --- | --- | --- | --- |
| **Rules shorthand** | `rules.open` / `rules.close` accept a comma string, a string array, or a record. | `Rules.Open` / `Rules.Close` are `map[string]*RuleMod` only, so build the map explicitly. | `RulesOption` is a `BTreeMap<String, RuleMod>` per direction; `.open_rules("val,pair")` takes the comma string, `.open_rule(name, RuleMod::when(…))` adds a condition. |
| **Partial `rules` + defaults** | Plugin defaults merge into a partial `rules` (omitted direction keeps its default). | A non-`nil` `*RulesOption` is a complete override; `nil` uses defaults, `&RulesOption{}` uses none. | Same as Go: `rules: None` uses the defaults, `Some(RulesOption::new())` modifies no rules, and any `Some` is a complete override. |
| **String-path action** | `action: 'a.b.c'` resolves a dotted path on the instance options at fire time. | Same, but the TS options object is open while the Go `Options` struct is closed, so the path resolves in the plugin-options namespace: `"custom.x"` reads `j.PluginOptions("custom")["x"]` at fire time. | Same as Go, for the same reason: `DirectiveAction::Path("custom.x")` reads `parser.plugin_options("custom")["x"]` from the parse's own resolved options at fire time. |
| **Action return value** | An action may return a `Token`; an error token halts the parse. | Same via the `TokenAction` form (`func(r, ctx) any`); a returned `*tabnas.Token` with `Err` set halts the parse, other tokens are ignored. | Same via `with_token_action` (`Fn(&mut Rule, &mut Context) -> Result<Option<Token>, ActionError>`); a returned token carrying an error code halts the parse, other tokens are forwarded and otherwise ignored. An action may also fail directly with `Err(ActionError)`. |
| **Registration failure** | The plugin `throw`s (propagated by `j.use`). | The plugin returns an `error` (propagated by `j.Use` / `Apply`) and never panics. | The plugin returns `Err(DirectiveError)` (propagated by `use_plugin` / `apply`) and never panics; a panic inside a user callback is contained by the engine and surfaces as a `PluginError`. |
| **`bc` child node** | The closing child node is read directly. | The `bc` hook walks the `Prev`-linked replacement chain to adopt the final child node, working around Go slice reallocation when a `val` is replaced by an implicit list. Exercised by `test/spec/implicit.tsv`. | Read directly, as in TypeScript. The Rust engine hands a replaced rule the same `Rc<RefCell<Value>>` node cell, so no chain walk is needed; `test/spec/implicit.tsv` exercises it. |
| **Assigning a node** | `rule.node = value`. | `rule.Node = value`. | A pushed or replaced rule SHARES its parent's node cell, so an assignment must install a fresh one: call `set_node(rule, value)` rather than writing through `rule.node.borrow_mut()`. Borrow the cell only to mutate a container the rule genuinely shares: pushing onto an enclosing list, say. |
| **Rule-map ordering** | Object key order. | Go map iteration order (unordered). | `BTreeMap`, so a directive installs its host-rule modifications in a deterministic order. |


## Spec file format (`test/spec/*.tsv`)

```
# comments start with '#'
# blank lines are ignored
<input><TAB><expected-json>
<input><TAB>ERROR:<code>
```

Parsed by the TypeScript, Go, and Rust test suites. The TypeScript and Go
loaders come from `@tabnas/support`; Rust has no support crate, so its
loader lives in `rs/tests/common/spec.rs` and must read the files the
same way: the same comment and blank-line skipping, the same `\n`,
`\r`, `\t` and `\\` escapes in the input column, and the same `ERROR:`
handling.
