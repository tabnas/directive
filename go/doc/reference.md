# Reference (Go)

The exact public surface of `tabnasdirective`
(`github.com/tabnas/directive/go`). For orientation see the
[tutorial](tutorial.md); for tasks see the [how-to guides](guide.md).
This is the Go port of the [TypeScript reference](../../ts/doc/reference.md);
TypeScript is canonical.

The plugin's only dependency is the
[tabnas](https://github.com/tabnas/parser) engine
(`github.com/tabnas/parser/go`); its types (`Rule`, `Context`, `Tin`,
`AltCond`, `Plugin`, …) come from there. The plugin modifies host-grammar
rules (`val` / `list` / `map` / `pair`), so it is always applied to a
`*tabnas.Tabnas` that already has a grammar installed.


## Package and version

```go
import (
	tabnas "github.com/tabnas/parser/go"
	tabnasdirective "github.com/tabnas/directive/go"
)
```

`tabnasdirective.Version` is the current version string.


## `Apply`

```go
func Apply(j *tabnas.Tabnas, opts DirectiveOptions) (*tabnas.Tabnas, error)
```

The typed convenience constructor — the Go equivalent of TypeScript's
`j.use(Directive, options)`. It forwards `opts` to `j.Use` as the plugin
option map and returns the instance (for chaining) plus any registration
error: a duplicate open token, or a grammar build failure. **The plugin
never panics** — every failure path is reported through this `error`.

`j` must already have a host grammar installed (one defining the `val` /
`list` / `map` / `pair` rules).


## `Directive` (plugin)

```go
var Directive tabnas.Plugin
```

The raw plugin value, for when you register from a JSON-driven config
rather than the typed `Apply`:

```go
err := j.Use(tabnasdirective.Directive, map[string]any{
	"name":   "upper",
	"open":   "@",
	"close":  ">",                 // optional
	"action": myAction,            // tabnasdirective.Action
	"rules":  myRules,             // *tabnasdirective.RulesOption (optional)
	"custom": myCustom,            // tabnasdirective.CustomFunc (optional)
})
```

Recognised option keys: `name`, `open`, `close`, `action`, `rules`,
`custom`. Omitting the `rules` key uses the defaults; passing an empty
`*RulesOption` modifies no rules.


## `DirectiveOptions`

```go
type DirectiveOptions struct {
	Name   string
	Open   string
	Close  string
	Action Action
	Rules  *RulesOption
	Custom CustomFunc
}
```

| Field    | Type           | Required | Description |
| -------- | -------------- | -------- | ----------- |
| `Name`   | `string`       | yes      | Directive name. Also the parse-rule name and token-name suffix (`#OD_<Name>`, `#CD_<Name>`). |
| `Open`   | `string`       | yes      | Character sequence that starts the directive. Must be unique per instance. |
| `Close`  | `string`       | no       | Character sequence that ends the directive. Empty → open-only (consumes one value). |
| `Action` | `Action`       | yes      | Fired when the directive body finishes. |
| `Rules`  | `*RulesOption` | no       | Which host rules detect the directive. `nil` → [defaults](#defaults); `&RulesOption{}` → no rules. |
| `Custom` | `CustomFunc`   | no       | Callback run after setup, given the resolved tokens. |


## `Action`

```go
type Action func(rule *tabnas.Rule, ctx *tabnas.Context)
```

Called once the body has parsed. `rule.Child.Node` holds the parsed body
value (subject to how the host grammar propagates nodes). Assign
`rule.Node` to set the directive's result. If `rule.Parent` is a `pair`,
mutate `rule.Parent.Node` instead and leave `rule.Node` unset.

Unlike TypeScript's `StateAction`, the Go `Action` returns nothing —
there is no token-override return value.


## `RulesOption`, `RuleMod`

```go
type RulesOption struct {
	Open  map[string]*RuleMod
	Close map[string]*RuleMod
}

type RuleMod struct {
	C tabnas.AltCond // optional per-rule condition; nil = always match
}
```

`Open` maps host rule names to modifications that detect the open token
(push into the directive rule). `Close` maps host rule names to
modifications that detect the close token (so they stop consuming
siblings) — only meaningful when `Close` is set. A `nil` `RuleMod` entry
is normalised to `&RuleMod{}`.

There is no comma-string / string-slice shorthand (a TypeScript-only
convenience); build the map explicitly.


## `CustomFunc`, `DirectiveConfig`

```go
type CustomFunc func(j *tabnas.Tabnas, config DirectiveConfig)

type DirectiveConfig struct {
	OPEN  tabnas.Tin
	CLOSE tabnas.Tin // -1 if no close token
	Name  string
}
```

`Custom` runs last, after the plugin has resolved its tokens. `CLOSE` is
`-1` when no close token was configured.


## Defaults

When `Rules` is `nil`:

| Direction | Default rule names    |
| --------- | --------------------- |
| `Open`    | `val`                 |
| `Close`   | `list`, `elem`, `map`, `pair` |

So the open token is recognised in any value position, and the close
token ends sibling parsing in list / element / map / pair rules. (Pass
`&RulesOption{}` to override with no rules at all.)


## Tokens

For a directive named `NAME`:

| Token name   | When                                                       | Value   |
| ------------ | ---------------------------------------------------------- | ------- |
| `#OD_<NAME>` | always                                                     | `Open`  |
| `#CD_<NAME>` | only if `Close` is set **and** not already a fixed token   | `Close` |

When `Close` collides with an existing fixed token (a close character
shared across directives) the existing token is reused — the plugin
looks up its registered name via `j.TinName` so the grammar spec
resolves to the same `Tin` — and no new `#CD_<NAME>` is created.


## Group tags

Every alternate the plugin installs carries `directive` as one of its
`G` tags. The additional per-alt tags:

| Context                | Tag(s)        |
| ---------------------- | ------------- |
| Open-rule OPEN alt     | `start`       |
| Open-rule OPEN+CLOSE   | `start,end`   |
| Close-rule CLOSE       | `end`         |
| Close-rule `,CLOSE`    | `end,comma`   |


## Counters

While inside a directive, `rule.N["dr_<NAME>"] == 1`. The close-rule
alternates only fire when this counter is `1`, so a stray close token
with no matching open becomes an `unexpected` error rather than a wrong
parse.

`dlist` and `dmap` control implicit (bracketless) containers inside the
body:

| `Close` present? | `dlist` / `dmap` inside the body |
| ---------------- | -------------------------------- |
| yes              | 0 — implicits allowed (the close bounds the body) |
| no               | 1 — implicits suppressed (so the directive consumes exactly one value) |


## Errors

| Situation                                   | Behaviour |
| ------------------------------------------- | --------- |
| `Open` is already a fixed token             | `Apply` / `j.Use` return `error: "Directive open token already in use: <open>"` (no panic) |
| Grammar build failure during registration   | `Apply` / `j.Use` return the engine's `error` (no panic) |
| Parsing a close token with no matching open  | `j.Parse` returns an `unexpected` error |
| Open token with no following value           | `j.Parse` returns an `unexpected` error |


## Spec file format (`../test/spec/*.tsv`)

```
# comments start with '#'
# blank lines are ignored
<input><TAB><expected-json>
<input><TAB>!error <regex>
```

Loaded by both the Go and TypeScript test suites.
