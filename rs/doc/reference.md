# Reference (Rust)

Complete API listing for the Rust port of the directive plugin. For an
orientation on how to use these pieces, see the [tutorial](tutorial.md)
or the [how-to guides](guide.md). The cross-language reference lives in
[`../../docs/reference.md`](../../docs/reference.md).

The plugin's only dependency is the
[tabnas](https://github.com/tabnas/parser) parser engine; its types
(`Rule`, `Context`, `Tin`, `Token`, `Value`, `ActionError`, …) come from
there. The plugin modifies host grammar rules (`val` / `list` / `map` /
`pair`), so it is applied to an engine instance that already has a
grammar installed.

```rust
use tabnas_directive::{
    apply, plugin, set_node,
    DirectiveAction, DirectiveConfig, DirectiveError, DirectiveOptions,
    RuleMod, RulesOption, VERSION,
    ActionFn, ConditionFn, CustomFn, TokenActionFn,
};
```


## `apply`

```rust
pub fn apply(parser: &mut Tabnas, options: DirectiveOptions)
    -> Result<(), DirectiveError>;
```

Registers the directive through `Tabnas::use_plugin`, so it is
re-applied to derived instances. Returns any registration error: a
duplicate open token, or a grammar build failure. The plugin never
panics; a panic inside a user callback is contained by the engine and
surfaces as a `PluginError`.


## `plugin`

```rust
pub fn plugin(options: DirectiveOptions) -> tabnas::Plugin;
```

Builds the `Plugin` value named `Directive` that `apply` installs. Use
it when you want to install the plugin yourself:

```rust
parser.use_plugin(plugin(options), None)?;
```

The typed option data travels in the returned closure rather than in the
engine's serialized plugin-option bag, because a directive's action,
conditions and custom hook are Rust callbacks and no `Value` can carry
them.


## `DirectiveOptions`

| Field    | Type                  | Required | Description                                                             |
| -------- | --------------------- | -------- | ----------------------------------------------------------------------- |
| `name`   | `String`              | yes      | Directive name. Rule name and token-name suffix.                        |
| `open`   | `String`              | yes      | Open character sequence. Must be unique per instance.                   |
| `close`  | `Option<String>`      | no       | Close character sequence. `None` (or empty) → the directive consumes a single value. |
| `action` | `DirectiveAction`     | no       | How the parsed body is transformed. Default `DirectiveAction::None`.    |
| `rules`  | `Option<RulesOption>` | no       | Rule modifications. `None` → defaults. `Some(RulesOption::new())` → no rules. |
| `custom` | `Option<CustomFn>`    | no       | Callback after setup. Argument: `&DirectiveConfig`.                     |

Built with `DirectiveOptions::new(name, open)` plus the builders; the
fields are public too.

| Builder                  | Sets                                   |
| ------------------------ | -------------------------------------- |
| `with_close(close)`      | `close`                                |
| `with_action(f)`         | `action = DirectiveAction::Call(f)`    |
| `with_token_action(f)`   | `action = DirectiveAction::Token(f)`   |
| `with_action_path(path)` | `action = DirectiveAction::Path(path)` |
| `with_rules(rules)`      | `rules`                                |
| `with_custom(f)`         | `custom`                               |


## `DirectiveAction`

```rust
pub enum DirectiveAction {
    None,
    Call(ActionFn),
    Token(TokenActionFn),
    Path(String),
}
```

| Variant  | Signature / meaning |
| -------- | ------------------- |
| `None`   | No action. The directive's node stays the empty map seeded by the `bo` hook. |
| `Call`   | `Fn(&mut Rule, &mut Context) -> Result<(), ActionError>`. The classic form: call `set_node` to assign the result. `Err` aborts the parse under that code. |
| `Token`  | `Fn(&mut Rule, &mut Context) -> Result<Option<Token>, ActionError>`. May hand a token back to the engine, the TypeScript `Token \| void` contract. A token carrying an error code halts the parse; any other token is forwarded and otherwise ignored. |
| `Path`   | A dotted path resolved against the parse's own resolved options when the directive fires; the resolved value becomes the directive's node. TypeScript resolves from the open options object; Rust's `Options` struct is closed, like Go's, so the path resolves in the plugin-options namespace: `"custom.x"` reads `parser.plugin_options("custom")["x"]`. A missing segment resolves to `Value::Undefined`. |


## `set_node`

```rust
pub fn set_node(rule: &mut Rule, value: Value);
```

Sets a rule's node value. **A rule pushed by the engine SHARES its
parent's node cell**, so writing through `rule.node.borrow_mut()` would
overwrite the parent's node too. `set_node` installs a fresh cell; it
is what `rule.node = …` means in the canonical TypeScript engine, and
what the engine's own builtin `@val-bo` / `@map-bo` / `@list-bo` actions
do.

Borrow the cell directly only to mutate a container the rule genuinely
shares: pushing onto an enclosing list, or merging into the map behind a
`pair` via `rule.parent_node`.


## The action's rule

| Field                 | Notes                                                                 |
| --------------------- | --------------------------------------------------------------------- |
| `rule.child_node`     | The parsed body. `Value::Undefined` when the directive is empty.       |
| `rule.node`           | The result cell. Assign via `set_node`.                                |
| `rule.parent_node`    | `Option<Rc<RefCell<Value>>>`, the parent rule's node cell.            |
| `rule.parent_rule`    | `Option<Rc<RuleSnapshot>>`; `parent.name` says where the directive matched. |
| `rule.n["dr_<NAME>"]` | `1` while inside this directive. See [Counters](#counters).            |


## `RulesOption`, `RuleMod`

```rust
pub struct RulesOption {
    pub open:  BTreeMap<String, RuleMod>,
    pub close: BTreeMap<String, RuleMod>,
}
pub struct RuleMod {
    pub c: Option<ConditionFn>, // optional per-rule condition
}
```

| Method                                 | Meaning |
| -------------------------------------- | ------- |
| `RulesOption::new()`                   | Modifies nothing. |
| `.open_rules(csv)` / `.close_rules(csv)` | Set a direction from a comma-separated list; whitespace and empty names are dropped. |
| `.open_rule(name, m)` / `.close_rule(name, m)` | Add one rule, optionally with a condition. |
| `RuleMod::new()`                       | No extra condition. |
| `RuleMod::when(cond)`                  | `cond: Fn(&mut Rule, &mut Context) -> bool`. |

Rules are held in a `BTreeMap`, so a directive installs its host-rule
modifications in a deterministic order.


## `CustomFn`, `DirectiveConfig`

```rust
pub type CustomFn = Arc<dyn Fn(&mut Tabnas, &DirectiveConfig) + Send + Sync>;

pub struct DirectiveConfig {
    pub open:  Tin,
    pub close: Option<Tin>, // None if no close token
    pub name:  String,
}
```


## `DirectiveError`

```rust
pub struct DirectiveError(pub String);
```

A registration failure: a duplicate open token, or a grammar the engine
refused. Implements `Display` and `std::error::Error`, and converts to
and from `tabnas::PluginError`, so `apply` and `use_plugin` report the
same thing.


## Callback type aliases

```rust
pub type ActionFn      = Arc<dyn Fn(&mut Rule, &mut Context) -> Result<(), ActionError> + Send + Sync>;
pub type TokenActionFn = Arc<dyn Fn(&mut Rule, &mut Context) -> Result<Option<Token>, ActionError> + Send + Sync>;
pub type ConditionFn   = Arc<dyn Fn(&mut Rule, &mut Context) -> bool + Send + Sync>;
pub type CustomFn      = Arc<dyn Fn(&mut Tabnas, &DirectiveConfig) + Send + Sync>;
```


## `VERSION`

```rust
pub const VERSION: &str;
```

Must equal `rs/Cargo.toml`'s `version` and `ts/package.json`'s
`"version"`. `tests/version_test.rs` fails the build if they drift.


## Rules defaults

When `rules` is `None`:

| Direction | Default rule names   |
| --------- | -------------------- |
| `open`    | `val`                |
| `close`   | `list,elem,map,pair` |

Meaning: the directive's open token is recognised in any value position;
the close token ends sibling parsing in list / element / map / pair
rules.


## Tokens

For a directive named `NAME` the plugin registers:

| Token name   | When                   | Fixed token value |
| ------------ | ---------------------- | ----------------- |
| `#OD_<NAME>` | always                 | `open`            |
| `#CD_<NAME>` | only if `close` is set AND `close` isn't already a fixed token | `close` |

When `close` collides with an existing fixed token (a shared close
across directives) the existing token is reused and no new
`#CD_<NAME>` token is created.


## Group tags

Every alt installed by the plugin carries `directive` as one of its `g`
tags. The per-alt tags (in addition) are:

| Context              | Tag(s)      |
| -------------------- | ----------- |
| Open-rule OPEN alt   | `start`     |
| Open-rule OPEN+CLOSE | `start,end` |
| Close-rule CLOSE     | `end`       |
| Close-rule `,CLOSE`  | `end,comma` |


## Condition references

Per-rule conditions cannot travel in a JSON grammar document, so they
are registered on the instance under generated names and named from the
document:

| Reference                         | Gates |
| --------------------------------- | ----- |
| `@dr-open-c-<NAME>-<RULE>`        | The open-rule OPEN alt, when `RuleMod.c` is set. |
| `@dr-close-c-<NAME>-<RULE>`       | The close-rule CLOSE alt (`dr_<NAME> == 1`, then `RuleMod.c`). |
| `@dr-close-ca-c-<NAME>-<RULE>`    | The close-rule `,CLOSE` alt (`dr_<NAME> == 1`). |

The directive rule's lifecycle hooks use the engine's own convention:
`@<NAME>-bo` seeds an empty map node, `@<NAME>-bc` runs the action.


## Counters

While inside a directive, `rule.n["dr_<NAME>"] == 1`. This is how the
close-rule alts recognise they should close.

`dlist` and `dmap` control whether implicit lists / maps are permitted
inside the directive body:

| `close` present? | `dlist` / `dmap` set to |
| ---------------- | ----------------------- |
| yes              | 0 (implicits allowed)   |
| no               | 1 (implicits suppressed so trailing siblings aren't consumed) |


## Errors

| Situation                                             | Behaviour |
| ----------------------------------------------------- | --------- |
| Registering a directive whose `open` is already fixed | `apply` / `use_plugin` return `Err` (no panic) |
| Grammar build failure during registration             | `apply` / `use_plugin` return `Err` (no panic) |
| Parsing a close token without its open                | engine `unexpected` error |
| An action returning `Err(ActionError)`                | the parse aborts under that code |
| An action returning a token carrying an error code    | the parse aborts under that token's code |

The plugin declares no error codes of its own; the rejections it
produces surface under codes inherited from the engine.
