# Reference

Complete API listing for `@jsonic/directive`. For an orientation on
how to use these pieces, see the [Tutorial](tutorial.md) or the
[How-to guides](how-to.md).


## TypeScript API

### `Directive` (Jsonic plugin)

A Jsonic `Plugin`. Register via `jsonic.use(Directive, options)`.

```ts
import { Directive, DirectiveOptions } from '@jsonic/directive'
```

### `DirectiveOptions`

| Field     | Type                                  | Required | Description                                                             |
| --------- | ------------------------------------- | -------- | ----------------------------------------------------------------------- |
| `name`    | `string`                              | yes      | Directive name. Also used as the rule name and the token-name suffix.   |
| `open`    | `string`                              | yes      | Character sequence that starts the directive. Must be unique per jsonic instance. |
| `close`   | `string`                              | no       | Character sequence that ends the directive. If omitted the directive consumes a single value. |
| `action`  | `StateAction \| string`               | yes      | Function called when the directive closes, or a dotted path into `jsonic.options`. |
| `rules`   | `RulesOption \| null`                 | no       | Which existing grammar rules detect this directive. See [Rules](#rules). |
| `custom`  | `(jsonic, config) => void`            | no       | Callback invoked after setup. `config = { OPEN, CLOSE, name }`.         |

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

Assign `rule.node` to set the directive's result value. If `rule.parent` is a
pair you can mutate `rule.parent.node` instead and leave `rule.node`
alone.


## Go API

### `Directive` (Jsonic plugin function)

```go
import directive "github.com/jsonicjs/directive/go"

directive.Apply(j, directive.DirectiveOptions{ ... })
// or, manually:
j.Use(directive.Directive, map[string]any{"_opts": &opts})
```

### `DirectiveOptions`

| Field     | Type                            | Required | Description                                                             |
| --------- | ------------------------------- | -------- | ----------------------------------------------------------------------- |
| `Name`    | `string`                        | yes      | Directive name. Rule name and token-name suffix.                        |
| `Open`    | `string`                        | yes      | Open character sequence. Must be unique per jsonic instance.            |
| `Close`   | `string`                        | no       | Close character sequence. Empty → directive consumes a single value.    |
| `Action`  | `Action`                        | yes      | Callback invoked when the directive closes.                             |
| `Rules`   | `*RulesOption`                  | no       | Rule modifications. `nil` → defaults. `&RulesOption{}` → no rules.      |
| `Custom`  | `CustomFunc`                    | no       | Callback after setup. Argument: `DirectiveConfig{OPEN, CLOSE, Name}`.   |

### `RulesOption`

```go
type RulesOption struct {
    Open  map[string]*RuleMod
    Close map[string]*RuleMod
}
type RuleMod struct {
    C jsonic.AltCond
}
```

### `Action`

```go
type Action func(rule *jsonic.Rule, ctx *jsonic.Context)
```

### `CustomFunc`, `DirectiveConfig`

```go
type CustomFunc      func(j *jsonic.Jsonic, config DirectiveConfig)
type DirectiveConfig struct {
    OPEN  jsonic.Tin
    CLOSE jsonic.Tin // -1 if no close token
    Name  string
}
```


## Defaults

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
| Registering a directive whose `open` is already fixed | `throw` Error             | `panic`                   |
| Parsing a close token without its open          | Jsonic `unexpected` error  | Jsonic `unexpected` error |


## Spec file format (`test/spec/*.tsv`)

```
# comments start with '#'
# blank lines are ignored
<input><TAB><expected-json>
<input><TAB>!error <regex>
```

Parsed by both the TypeScript and the Go test suites.
