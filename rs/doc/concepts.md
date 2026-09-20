# Concepts: how the directive plugin works (Rust)

This is the *why* and *how* for the Rust port (`tabnas_directive`). You
do not need it to use the plugin; reach for it when debugging a
grammar, building something with the custom hook, or to understand the
engine relationship and the deliberate differences from the canonical
TypeScript implementation. The [reference](reference.md) lists *what*;
this explains the mechanism.


## Three layers

```
your input ──▶ [ tabnas engine ] ──▶ value
                     ▲   ▲
              host grammar   directive plugin
```

- **tabnas**. The parser engine (the `tabnas` crate), and the plugin's
  only dependency. It ships *no* grammar: just a matcher-based lexer and
  a rule-based parser driven by grammar specs. The `Rule`, `Context`,
  `Tin`, `Token`, `Value`, `RuleSpec`, `AltSpec`, `GrammarSpec` and
  `ActionError` types are all tabnas types.
- **a host grammar**. Any grammar installed onto a `Tabnas` that
  defines the usual `val` / `list` / `map` / `pair` / `elem` rules. The
  directive layers onto it. This repo's tests use a deliberately small
  one (`tests/common/mini_grammar.rs`).
- **directive**. This plugin. It extends the host grammar's rules to
  recognise directive tokens, which is why it operates on an instance
  with a grammar already installed, not a bare engine.


## What a directive is

A directive is a user-defined token sequence that the parser treats as a
call-out into custom logic. The plugin makes the parser recognise an
**open** token (and optionally a **close** token), push into a new parse
rule while the body is parsed, fire an **action** when the body
finishes, and let that action assign or transform the resulting node.

Two shapes:

- **Open-only**. Consumes a single value after the open token (`@foo`).
- **Open + close**. Consumes everything between open and close,
  including structured bodies (`sum<1, 2, 3>`).


## The rule model it plugs into

The parser is rule-based. Every parse step sits inside some rule: `val`,
`list`, `elem`, `map`, `pair`. Each rule has **alternates**: ordered
`AltSpec` values that decide which branch to take when the rule opens or
closes.

The plugin weaves the directive into this model in three places:

1. **Open-rules** (default `val`). Get an open-alt that matches the open
   token and *pushes* (`"p": name`) into a new rule named after the
   directive.
2. **Close-rules** (default `list`, `elem`, `map`, `pair`). Get a
   close-alt that matches the close token so they stop consuming siblings
   at the directive boundary.
3. **Directive rule**. A brand-new rule whose job is to parse one value
   (`"p": "val"`), optionally look for the close token, and fire the
   action.

All of this is built as one serialized `GrammarSpec` document and
applied with a single `grammar_with_setting` call. The directive rule's
`bo` / `bc` state hooks are registered as `@<name>-bo` / `@<name>-bc`
through `state_action_with_next_ref`, which the engine's
`@<rule>-<phase>` convention wires automatically when the `<name>` rule
is installed: `bo` seeds an empty map node, and `bc` calls the action.

`state_action_with_next_ref` is deliberately the *chained* lifecycle
shape, the one that can return a `Token`. That is how the TypeScript
action's `Token | void` return survives the port: the engine halts the
parse when the returned token carries an error code and otherwise
forwards it.


## The shared node cell

This is the one Rust-specific hazard, and it has no counterpart in the
TypeScript or Go ports.

A rule pushed by the engine is created with
`Rule::with_shared_node(name, parent.node.clone())`, so it shares the
**same** `Rc<RefCell<Value>>` cell as its parent. So:

```rust
*rule.node.borrow_mut() = value;  // ALSO overwrites the parent's node
set_node(rule, value);            // installs a fresh cell (correct)
```

`set_node` is what `rule.node = …` means in the canonical engine, and
what the engine's own builtin `@val-bo` / `@map-bo` / `@list-bo` actions
do. Every directive action producing a value should go through it.

Borrow the cell directly only to mutate a container the rule genuinely
shares: the host grammar's `elem` close hook pushing onto the enclosing
list, its `pair` close hook writing into the enclosing map, or a
directive merging a looked-up map into `rule.parent_node`.

Get this wrong and the symptom is confusing rather than loud. The
directive's `bo` hook seeding `{}` would also blank the host `val`
rule's node; `val` would then see a non-undefined node and answer `{}`
instead of adopting the directive's result.


## The `dr_<NAME>` counter

Close tokens are ambiguous: a `>` could close `foo<>`, close `bar<>`, or
be a syntax error. The plugin disambiguates with a rule counter. On open
it sets `n["dr_<name>"] = 1`. The close-alt's condition only fires when
that counter is `1`, so a stray `>` with no matching open raises an
`unexpected` error. The counter unwinds when the directive rule
resolves, so outer rules again see the close token as untagged.


## Boundary closing

The close token closes more than the directive: it also terminates a
list or map opened **inside** the body. That is why `sum<[1, 2>` parses:
the `>` closes the open list and the directive together, no `]` needed.
Each close-rule gained a close-alt that fires on the close token while
`dr_<NAME>` is `1`, plus a `#CA <close>` variant for a trailing comma.


## Implicit lists and maps

Some host grammars support implicit containers (`1, 2, 3` as a list
without brackets). This interacts badly with an *open-only* directive:
once it pushes into `val` it would keep consuming siblings. The plugin
guards with counters set on the directive-rule push:

| `close` present? | `dlist`, `dmap` inside the body |
| ---------------- | ------------------------------- |
| yes              | reset to 0, implicits allowed  |
| no               | raised to 1, implicits suppressed |

The host grammar reads these (the mini grammar only starts an implicit
list when `n["dlist"] != 1`).

Where the Go port needs a `Prev`-chain walk in its `bc` hook to find the
final child node after a `val` is replaced by an implicit list (a Go
slice-reallocation workaround), Rust needs nothing: a replaced rule
keeps the same node cell, so `rule.child_node` is already the final
value. `../../test/spec/implicit.tsv` exercises it.


## Why one grammar spec tagged `directive`

Every alt the plugin installs is tagged with the group `directive` in
addition to its per-alt tag, via `GrammarSetting::groups("directive")`
passed to `grammar_with_setting`. The engine appends `"directive"` to
each alt's group list, making plugin-added alternates easy to identify
and trace.


## Why conditions are named references

A serialized grammar document is JSON: it cannot carry a Rust closure.
So each per-rule condition is registered on the instance first, under a
generated name (`@dr-open-c-<name>-<rule>`,
`@dr-close-c-<name>-<rule>`, `@dr-close-ca-c-<name>-<rule>`), and the
document names it. The engine resolves the reference during grammar
installation and fails the install (transactionally) if it is missing.

The same reasoning explains why the typed options travel in the plugin
closure rather than in the engine's serialized plugin-option bag: an
action, a condition and a custom hook are Rust callbacks, and no `Value`
can carry them.


## Why shared close tokens reuse the existing fixed token

Two directives with the same close character must resolve to the same
engine `Tin`, so the lexer produces one token type. The plugin checks
`parser.fixed(close)` first: if the close character is already
registered it reuses that token (and grabs its name via
`parser.token_name` so the grammar document resolves to the same `Tin`);
otherwise it registers a fresh `#CD_<NAME>`.


## Why the custom hook receives resolved tokens

The custom hook runs last and is handed the resolved `open` / `close`
tins (`close` is `None` when there is none), so you can install extra
alternates that match the directive's tokens without re-resolving them.
Those tins are only stable after the plugin's own wiring; hence the
callback fires at the end.


## Relationship to the engine

The plugin is purely additive: it uses public engine APIs
(`token_with_source`, `fixed`, `token_name`, `define_rule`,
`alt_condition`, `state_action_with_next_ref`, `grammar_with_setting`)
to register tokens and extend rule specs. Mix it freely with other
plugins. The one shared resource to watch is the fixed-token table: two
plugins wanting the same open sequence collide, and the second
registration returns an error.

### `derive` and the fixed-token table

Because `apply` goes through `Tabnas::use_plugin`, a directive is
re-applied when you `derive` a child instance, like every native plugin.
A derived instance also inherits the parent's fixed tokens, so the
re-run finds the open token already claimed and `derive` returns

```
Directive open token already in use: @
```

This is not a Rust quirk. The canonical engine does the same: TypeScript's
`make()` re-runs inherited plugins over merged options, and the plugin's
`tabnas.fixed(open)` check then throws. Deriving is how a fixed-token
registration collides with itself.

**Work with it, not around it:** derive from an instance that does *not*
yet have the directive, and apply the directive to the child. The parent
is left intact by the failed derive either way.

That only works when the host grammar is itself a plugin. `derive`
rebuilds the child by re-running the parent's *plugins*, so a grammar
installed imperatively through `define_rule` (the repo's own
`make_mini()` scaffold, say) does not reach the child at all: the child
has no rules, and applying the directive to it succeeds but parses
nothing. Install the host grammar through `use_plugin` when you intend
to derive.

`rs/tests/directive_test.rs` pins both behaviours
(`deriving_an_instance_that_already_has_the_directive_reports_the_duplicate`
and `deriving_from_a_plugin_host_grammar_then_applying_the_directive_works`)
so they stay recorded properties rather than accidents.


## Differences from the TS version

TypeScript is canonical; this Rust port mirrors its option names,
defaults and the shared `../../test/spec/*.tsv` conformance fixtures (all
three runtimes pass identical fixtures). The following differences are
**intentional**: they stem from Rust's type system and the engine's
API shape, not from drift:

| Area | TypeScript | Rust |
| ---- | ---------- | ---- |
| **Constructor** | `j.use(Directive, options)` (chainable, throws on error). | `apply(&mut parser, options)` returns `Result<(), DirectiveError>`; or `parser.use_plugin(plugin(options), None)`. |
| **Rules shorthand** | `rules.open` / `rules.close` accept a comma string, a string array, or a record. | `RulesOption` is a `BTreeMap<String, RuleMod>` per direction; `.open_rules("val,pair")` takes the comma string, `.open_rule(name, RuleMod::when(…))` adds a condition. |
| **Partial `rules` + defaults** | Plugin defaults merge into a partial `rules` (an omitted direction keeps its default). | `Some(_)` is a complete override; `None` uses defaults, `Some(RulesOption::new())` uses none. Same as Go. |
| **String-path action** | `action: 'a.b.c'` resolves a dotted path on the instance options at fire time. | Same, but the TS options object is open while Rust's `Options` struct is closed, so the path resolves in the plugin-options namespace: `DirectiveAction::Path("custom.x")` reads `parser.plugin_options("custom")["x"]` at fire time. |
| **Action return value** | A `StateAction` may return a `Token`; an error token halts the parse. | Same via `with_token_action` (`-> Result<Option<Token>, ActionError>`); a token carrying an error code halts the parse, other tokens are forwarded and otherwise ignored. An action may also fail directly with `Err(ActionError)`. |
| **Registration failure** | The plugin `throw`s (propagated by `j.use`). | The plugin returns `Err(DirectiveError)` (propagated by `use_plugin` / `apply`) and never panics; a panic inside a user callback is contained by the engine and surfaces as a `PluginError`. |
| **Assigning a node** | `rule.node = value`. | `set_node(rule, value)`, because a pushed rule shares its parent's node cell, so an assignment must install a fresh one. |
| **Rule-map ordering** | Object key order. | `BTreeMap`, so host-rule modifications install in a deterministic order. |


## Design principles

- **Declarative first.** Rule modifications are one serialized
  `GrammarSpec`; imperative `spec.clear()` is reserved for resetting the
  directive rule before the document installs a clean set of alternates
  and state actions.
- **Language parity.** TypeScript is canonical; this port tracks its
  option names, defaults, and shared fixtures.
- **Fail loudly, never panic.** Re-registering an open token returns an
  error rather than silently overwriting; a close token with no open is a
  parse error, not a wrong parse.
