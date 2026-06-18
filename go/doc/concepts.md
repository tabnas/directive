# Concepts: how the directive plugin works (Go)

This is the *why* and *how* for the Go port (`tabnasdirective`). You do
not need it to use the plugin — reach for it when debugging a grammar,
building something with `Custom`, or to understand the engine
relationship and the deliberate differences from the canonical
TypeScript implementation. The [reference](reference.md) lists *what*;
this explains the mechanism.


## Three layers

```
your input ──▶ [ tabnas engine ] ──▶ value
                     ▲   ▲
              host grammar   directive plugin
```

- **tabnas** — the parser engine (`github.com/tabnas/parser/go`), and
  the plugin's only dependency. It ships *no* grammar: just a
  matcher-based lexer and a rule-based parser driven by grammar specs.
  The `Rule`, `Context`, `Tin`, `RuleSpec`, `AltSpec`, `StateAction` and
  `AltCond` types are all tabnas types.
- **a host grammar** — any grammar installed onto a `*tabnas.Tabnas`
  that defines the usual `val` / `list` / `map` / `pair` / `elem` rules.
  The directive layers onto it. This repo's tests use a deliberately
  small one (`go/mini_grammar_test.go`).
- **directive** — this plugin. It extends the host grammar's rules to
  recognise directive tokens, which is why it operates on an instance
  with a grammar already installed, not a bare engine.


## What a directive is

A directive is a user-defined token sequence that the parser treats as a
call-out into custom logic. The plugin makes the parser recognise an
**open** token (and optionally a **close** token), push into a new parse
rule while the body is parsed, fire an **action** when the body
finishes, and let that action assign or transform the resulting node.

Two shapes:

- **Open-only** — consumes a single value after the open token (`@foo`).
- **Open + close** — consumes everything between open and close,
  including structured bodies (`sum<1, 2, 3>`).


## The rule model it plugs into

The parser is rule-based. Every parse step sits inside some rule — `val`,
`list`, `elem`, `map`, `pair`. Each rule has **alternates**: ordered
`*AltSpec` values that decide which branch to take when the rule opens or
closes.

The plugin weaves the directive into this model in three places:

1. **Open-rules** (default `val`) — get an open-alt that matches the open
   token and *pushes* (`P: name`) into a new rule named after the
   directive.
2. **Close-rules** (default `list`, `elem`, `map`, `pair`) — get a
   close-alt that matches the close token so they stop consuming siblings
   at the directive boundary.
3. **Directive rule** — a brand-new rule whose job is to parse one value
   (`P: "val"`), optionally look for the close token, and fire the
   action.

All of this is built as one `*tabnas.GrammarSpec` and applied with a
single `j.Grammar(spec, setting)` call. The directive rule's `bo`/`bc`
state hooks are wired through the spec's `Ref` map under the engine's
`@<name>-bo` / `@<name>-bc` naming convention: `bo` seeds
`r.Node = map[string]any{}`, and `bc` calls the action.


## The `dr_<NAME>` counter

Close tokens are ambiguous: a `>` could close `foo<>`, close `bar<>`, or
be a syntax error. The plugin disambiguates with a rule counter. On open
it sets `N["dr_"+name] = 1`. The close-alt's condition only fires when
`r.N["dr_"+name] == 1`, so a stray `>` with no matching open raises an
`unexpected` error. The counter unwinds when the directive rule resolves,
so outer rules again see the close token as untagged.


## Boundary closing

The close token closes more than the directive: it also terminates a
list or map opened **inside** the body. That is why `sum<[1, 2>` parses —
the `>` closes the open list and the directive together, no `]` needed.
Each close-rule gained a close-alt that fires on the close token while
`dr_<NAME>` is `1`, plus a `#CA <close>` variant for a trailing comma.


## Implicit lists and maps

Some host grammars support implicit containers (`1 2 3` as a list without
brackets). This interacts badly with an *open-only* directive: once it
pushes into `val` it would keep consuming siblings. The plugin guards
with counters set on the directive-rule push:

| `Close` present? | `dlist`, `dmap` inside the body |
| ---------------- | ------------------------------- |
| yes              | reset to 0 — implicits allowed  |
| no               | raised to 1 — implicits suppressed |

The host grammar reads these (the mini grammar only starts an implicit
list when `r.N["dlist"] != 1`).


## Why one grammar spec tagged `G: "directive"`

Every alt the plugin installs is tagged with the group `directive` in
addition to its per-alt tag, via the setting
`&tabnas.GrammarSetting{Rule: {Alt: {G: "directive"}}}` passed to
`j.Grammar`. The engine appends `"directive"` to each alt's group list,
making plugin-added alternates easy to identify and trace.


## Why shared close tokens reuse the existing fixed token

Two directives with the same close character must resolve to the same
engine `Tin`, so the lexer produces one token type. The plugin checks
`j.Config().FixedTokens` first: if the close character is already
registered it reuses that token (and grabs its name via `j.TinName` so
the grammar spec resolves to the same `Tin`); otherwise it registers a
fresh `#CD_<NAME>`.


## Why `Custom` receives resolved tokens

`Custom` runs last and is handed the resolved `OPEN` / `CLOSE` Tins
(`CLOSE == -1` when none), so you can install extra alternates that match
the directive's tokens without re-resolving them. Those Tins are only
stable after the plugin's own wiring — hence the callback fires at the
end.


## Relationship to the engine

The plugin is purely additive: it uses public engine APIs (`j.Token`,
`j.Rule`, `j.Grammar`, `j.Config`) to register tokens and extend rule
specs. Mix it freely with other plugins. The one shared resource to
watch is the fixed-token table — two plugins wanting the same open
sequence collide, and the second registration returns an error.


## Differences from the TS version

TypeScript is canonical; this Go port mirrors its option names, defaults
and the shared `../test/spec/*.tsv` conformance fixtures (both runtimes
pass identical fixtures). The following differences are **intentional** —
they stem from Go's static typing and engine-API shape, not from drift:

| Area | TypeScript | Go |
| ---- | ---------- | --- |
| **Constructor** | `j.use(Directive, options)` (chainable, throws on error). | `tabnasdirective.Apply(j, opts)` returns `(*Tabnas, error)`; or `j.Use(Directive, map[string]any{...})` with named keys. |
| **Rules shorthand** | `rules.open` / `rules.close` accept a comma string, a string slice, or a record. | `Rules.Open` / `Rules.Close` are `map[string]*RuleMod` only — build the map explicitly. |
| **Partial `rules` + defaults** | Plugin defaults merge into a partial `rules` (an omitted direction keeps its default). | A non-`nil` `*RulesOption` is a complete override; `nil` uses defaults, `&RulesOption{}` uses none. |
| **String-path action** | `action: 'a.b.c'` resolves a dotted path on the instance options at fire time. | `Action` is a typed func; capture the value in a closure instead. |
| **Action return value** | A `StateAction` may return a `Token` to override the next token. | `Action` returns nothing. |
| **Registration failure** | The plugin `throw`s (propagated by `j.use`). | The plugin returns an `error` (propagated by `j.Use` / `Apply`) and never panics. |
| **`bc` child node** | The closing child node is read directly. | The `bc` hook walks the `Prev`-linked replacement chain to adopt the final child node, working around Go slice reallocation when a `val` is replaced by an implicit list. Exercised by `../test/spec/implicit.tsv`. |


## Design principles

- **Declarative first.** Rule modifications are one `GrammarSpec`;
  imperative `rs.Clear()` is reserved for resetting the directive rule
  before the spec installs a clean set of alternates and state actions.
- **Language parity.** TypeScript is canonical; this port tracks its
  option names, defaults, and shared fixtures.
- **Fail loudly, never panic.** Re-registering an open token returns an
  error rather than silently overwriting; a close token with no open is a
  parse error, not a wrong parse.
