# How-to guides (Go)

Focused recipes for real tasks. Each assumes you have read the
[tutorial](tutorial.md) and can register a basic directive. This is the
Go port of the [TypeScript how-to guides](../../ts/doc/guide.md).

The examples use two import aliases: `tabnas` (the engine, and the
`Rule` / `Context` / `AltSpec` / … types,
`github.com/tabnas/parser/go`) and `tabnasdirective`
(`github.com/tabnas/directive/go`). They register against a `j` that
already has a host grammar installed:

```go
j := tabnas.Make()
j.Use(hostGrammar) // provides val / list / map / pair
```

`tabnasdirective.Apply` returns `(*tabnas.Tabnas, error)`; the examples
elide the error for brevity — check it in real code. The plugin never
panics.


## Wrap an arbitrary body with a close token

Use `Close` when a directive should consume everything up to a closing
token rather than a single value.

```go
tabnasdirective.Apply(j, tabnasdirective.DirectiveOptions{
	Name:  "group",
	Open:  "(",
	Close: ")",
	Action: func(r *tabnas.Rule, _ *tabnas.Context) {
		r.Node = r.Child.Node
	},
})
```


## Share a close token between two directives

Register a second directive with the same `Close` string. The plugin
detects the close character is already a registered fixed token and
**reuses** it (the lexer cannot hold two mappings for one sequence):

```go
tabnasdirective.Apply(j, tabnasdirective.DirectiveOptions{
	Name: "foo", Open: "foo<", Close: ">", Action: fooAction,
})
tabnasdirective.Apply(j, tabnasdirective.DirectiveOptions{
	Name: "bar", Open: "bar<", Close: ">", Action: barAction,
})
// j.Parse("[foo<a>, bar<b>]") -> []any{"FOO", "BAR"}
```

The **open** tokens (`foo<`, `bar<`) must still be unique. Reusing an
open token returns an error from `Apply` / `j.Use` (the plugin never
panics):

```go
_, err := tabnasdirective.Apply(j, tabnasdirective.DirectiveOptions{
	Name: "baz", Open: "foo<", Action: func(*tabnas.Rule, *tabnas.Context) {},
})
// err: "Directive open token already in use: foo<"
```


## Restrict where a directive is recognised

By default the open token is wired into `val`, so it matches in any
value position. Pass `Rules.Open` to narrow it. Unlike TypeScript's
comma-string shorthand, the Go form is always an explicit
`map[string]*RuleMod`:

```go
tabnasdirective.Apply(j, tabnasdirective.DirectiveOptions{
	Name: "inject",
	Open: "@",
	Rules: &tabnasdirective.RulesOption{
		Open: map[string]*tabnasdirective.RuleMod{
			"val":  {},
			"pair": {},
		},
	},
	Action: func(r *tabnas.Rule, _ *tabnas.Context) { /* … */ },
})
```

A directive that can appear as a whole map entry (a `pair`) as well as a
value can, in the `pair` branch, mutate `r.Parent.Node` instead of
setting `r.Node` — for example to merge a looked-up object into the
surrounding map:

```go
Action: func(r *tabnas.Rule, _ *tabnas.Context) {
	key := fmt.Sprintf("%v", r.Child.Node)
	val := src[key] // missing key -> nil
	if r.Parent != nil && r.Parent.Name == "pair" {
		if m, ok := r.Parent.Node.(map[string]any); ok {
			if sm, ok := val.(map[string]any); ok {
				for k, v := range sm {
					m[k] = v // merge into the map
				}
				return
			}
		}
	}
	r.Node = val
}
```


## Gate a rule modification with a condition

Set `RuleMod.C` to a `tabnas.AltCond`. The directive only matches inside
that host rule when the condition returns `true`:

```go
Rules: &tabnasdirective.RulesOption{
	Open: map[string]*tabnasdirective.RuleMod{
		"val": {},
		"pair": {C: func(r *tabnas.Rule, _ *tabnas.Context) bool {
			return r.Lte("pk", 0)
		}},
	},
},
```

The condition receives `(rule, ctx)` and returns a `bool` — the same
`AltCond` shape the engine uses everywhere.


## Read a value from options (no string-action form)

TypeScript's `action` accepts a dotted-path string that is resolved on
the instance options at fire time. Go's `Action` is a typed function, so
there is no string form — capture the value in a closure instead:

```go
x := 42
tabnasdirective.Apply(j, tabnasdirective.DirectiveOptions{
	Name: "constant",
	Open: "@",
	Action: func(r *tabnas.Rule, _ *tabnas.Context) { r.Node = x },
})
// j.Parse("@y") -> 42
```


## Run extra wiring after setup with `Custom`

`Custom` fires last and is handed the resolved `OPEN` / `CLOSE` Tins and
the directive `Name`, so you can install your own alternates that
reference the directive's tokens without re-resolving them:

```go
tabnasdirective.Apply(j, tabnasdirective.DirectiveOptions{
	Name:   "subobj",
	Open:   "@",
	Action: func(*tabnas.Rule, *tabnas.Context) { /* … */ },
	Custom: func(j *tabnas.Tabnas, cfg tabnasdirective.DirectiveConfig) {
		j.Rule("val", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
			rs.PrependOpen(&tabnas.AltSpec{
				S: [][]tabnas.Tin{{cfg.OPEN}},
				C: func(r *tabnas.Rule, _ *tabnas.Context) bool { return r.D == 0 },
				P: "map",
				B: 1,
				N: map[string]int{cfg.Name + "_top": 1},
				G: cfg.Name + "-top",
			})
		})
	},
})
```


## Turn off all default rule wiring

A `nil` `Rules` means "use the defaults". To modify **no** host rules,
pass an explicit empty `&RulesOption{}`. Only the directive's own rule
is then created, so its open token is unrecognised unless you install
alternates in `Custom`:

```go
tabnasdirective.Apply(j, tabnasdirective.DirectiveOptions{
	Name:   "none",
	Open:   "@",
	Action: func(*tabnas.Rule, *tabnas.Context) {},
	Rules:  &tabnasdirective.RulesOption{}, // explicit empty -> no rules
})

_, err := j.Parse("[@a]") // err: unexpected
```


## Test a directive against a shared spec file

Conformance rows live in `../test/spec/*.tsv`. Each row is one of:

```
<input><TAB><expected-json>
<input><TAB>!error <regex>
```

Blank lines and `#`-prefixed lines are ignored. Both the Go and
TypeScript suites load the same files, so a new row is exercised by both
runtimes. Run with `go test ./...` (Go) and `npm test` (TS), or
`make test` from the repo root.
