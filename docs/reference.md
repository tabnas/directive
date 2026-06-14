# Reference

Complete API listing for the directive plugin. For an orientation on
how to use these pieces, see the [Tutorial](tutorial.md) or the
[How-to guides](how-to.md).

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
panics — callers always get an `error` to handle.

### `DirectiveOptions`

| Field     | Type                            | Required | Description                                                             |
| --------- | ------------------------------- | -------- | ----------------------------------------------------------------------- |
| `Name`    | `string`                        | yes      | Directive name. Rule name and token-name suffix.                        |
| `Open`    | `string`                        | yes      | Open character sequence. Must be unique per instance.                   |
| `Close`   | `string`                        | no       | Close character sequence. Empty → directive consumes a single value.    |
| `Action`  | `Action`                        | yes      | Callback invoked when the directive closes.                             |
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

When `close` collides with an existing fixed token (e.g. a shared
close across directives) the existing token is reused and no new
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

| Situation                                       | TS behaviour               | Go behaviour              |
| ----------------------------------------------- | -------------------------- | ------------------------- |
| Registering a directive whose `open` is already fixed | `throw` Error             | `Apply` / `j.Use` return an `error` (no panic) |
| Grammar build failure during registration       | `throw` (engine)           | `Apply` / `j.Use` return an `error` (no panic) |
| Parsing a close token without its open          | engine `unexpected` error  | engine `unexpected` error |


## TypeScript / Go differences

TypeScript is canonical; the Go port mirrors its behaviour. Both pass
the identical shared `test/spec/*.tsv` conformance fixtures. The
following differences are intentional — they stem from Go's static
typing and from engine-API differences, not from drift:

| Area | TypeScript | Go |
| --- | --- | --- |
| **Rules shorthand** | `rules.open` / `rules.close` accept a comma string, a string array, or a record. | `Rules.Open` / `Rules.Close` are `map[string]*RuleMod` only — build the map explicitly. |
| **Partial `rules` + defaults** | Plugin defaults merge into a partial `rules` (omitted direction keeps its default). | A non-`nil` `*RulesOption` is a complete override; `nil` uses defaults, `&RulesOption{}` uses none. |
| **String-path action** | `action: 'a.b.c'` resolves a dotted path on the instance options at fire time. | `Action` is a typed func; capture the value in a closure instead. |
| **Action return value** | An action may return a `Token` to override the next token. | `Action` returns nothing. |
| **Registration failure** | The plugin `throw`s (propagated by `j.use`). | The plugin returns an `error` (propagated by `j.Use` / `Apply`) and never panics. |
| **`bc` child node** | The closing child node is read directly. | The `bc` hook walks the `Prev`-linked replacement chain to adopt the final child node, working around Go slice reallocation when a `val` is replaced by an implicit list. Exercised by `test/spec/implicit.tsv`. |


## Spec file format (`test/spec/*.tsv`)

```
# comments start with '#'
# blank lines are ignored
<input><TAB><expected-json>
<input><TAB>!error <regex>
```

Parsed by both the TypeScript and the Go test suites.
