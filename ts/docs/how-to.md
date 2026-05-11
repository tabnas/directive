# How-to guides

Practical recipes for common problems. Each guide assumes you have
already followed the [Tutorial](tutorial.md) and know how to register
a basic directive.


## How to add a close token

Use `close` when a directive wraps an arbitrary body instead of
consuming a single value.

**TypeScript:**

```ts
Jsonic.make().use(Directive, {
  name: 'group',
  open: '(',
  close: ')',
  action: (rule) => { rule.node = rule.child.node },
})
```

**Go:**

```go
directive.Apply(j, directive.DirectiveOptions{
    Name:  "group",
    Open:  "(",
    Close: ")",
    Action: func(r *jsonic.Rule, _ *jsonic.Context) {
        r.Node = r.Child.Node
    },
})
```


## How to share a close token between directives

Register a second directive with the same `close` string. The plugin
reuses the existing fixed token rather than creating a duplicate.

**TypeScript:**

```ts
const j = Jsonic.make()
  .use(Directive, { name: 'foo', open: 'foo<', close: '>', action: fooAction })
  .use(Directive, { name: 'bar', open: 'bar<', close: '>', action: barAction })
```

**Go:**

```go
directive.Apply(j, directive.DirectiveOptions{Name: "foo", Open: "foo<", Close: ">", Action: fooAction})
directive.Apply(j, directive.DirectiveOptions{Name: "bar", Open: "bar<", Close: ">", Action: barAction})
```

The open tokens (`foo<`, `bar<`) must still be unique — attempting to
reuse an open token throws / panics.


## How to restrict where a directive is recognised

Pass the `rules` option to tell the plugin which existing grammar
rules should detect the directive's open token.

**TypeScript:** only recognise `@x` as a list element:

```ts
Jsonic.make().use(Directive, {
  name: 'elem-only',
  open: '@',
  rules: { open: 'elem' },
  action: /* … */,
})
```

**Go:**

```go
directive.Apply(j, directive.DirectiveOptions{
    Name: "elem-only",
    Open: "@",
    Rules: &directive.RulesOption{
        Open: map[string]*directive.RuleMod{"elem": {}},
    },
    Action: /* … */,
})
```

The default when `rules` is omitted is `open: "val", close:
"list,elem,map,pair"` (see [Reference](reference.md#defaults)).


## How to attach a condition to a rule modification

Use `RuleMod.c` (Go: `RuleMod.C`) to gate whether the directive
matches inside a specific rule.

**TypeScript:** only match `@x` in a pair when `pk` is 0:

```ts
rules: {
  open: {
    val: {},
    pair: { c: (r) => r.lte('pk') },
  },
}
```

**Go:**

```go
Rules: &directive.RulesOption{
    Open: map[string]*directive.RuleMod{
        "val":  {},
        "pair": {C: func(r *jsonic.Rule, _ *jsonic.Context) bool { return r.Lte("pk", 0) }},
    },
},
```


## How to read a value from parser options

In TypeScript the `action` field accepts a dotted-path string. The
plugin looks up that path on `jsonic.options` every time the
directive fires.

```ts
const j = Jsonic.make().use(Directive, {
  name: 'constant',
  open: '@',
  action: 'custom.x',
})
j.options({ custom: { x: 42 } })

j('@')     // → 42
```

Go does not support string `action`; write a closure that captures
the value:

```go
x := 42
directive.Apply(j, directive.DirectiveOptions{
    Name: "constant",
    Open: "@",
    Action: func(r *jsonic.Rule, _ *jsonic.Context) { r.Node = x },
})
```


## How to run additional rule tweaks after the directive is set up

Use `custom` (Go: `Custom`) to install your own grammar alts after
the plugin has finished wiring up. You receive the resolved
`OPEN` / `CLOSE` Tins and the directive name.

**TypeScript:**

```ts
Jsonic.make().use(Directive, {
  name: 'subobj',
  open: '@',
  action: /* … */,
  custom: (jsonic, { OPEN, name }) => {
    jsonic.rule('val', (rs) => {
      rs.open({
        s: [OPEN],
        c: (r) => 0 === r.d,
        p: 'map',
        b: 1,
        n: { [name + '_top']: 1 },
        g: name + '-top',
      })
    })
  },
})
```

**Go:**

```go
directive.Apply(j, directive.DirectiveOptions{
    Name:   "subobj",
    Open:   "@",
    Action: /* … */,
    Custom: func(j *jsonic.Jsonic, cfg directive.DirectiveConfig) {
        j.Rule("val", func(rs *jsonic.RuleSpec) {
            rs.PrependOpen(&jsonic.AltSpec{
                S: [][]jsonic.Tin{{cfg.OPEN}},
                C: func(r *jsonic.Rule, _ *jsonic.Context) bool { return r.D == 0 },
                P: "map",
                B: 1,
                N: map[string]int{cfg.Name + "_top": 1},
                G: cfg.Name + "-top",
            })
        })
    },
})
```


## How to test a directive against a shared spec file

Spec rows live in `test/spec/*.tsv`. Each row is one of:

```
<input><TAB><expected-json>
<input><TAB>!error <regex>
```

Blank lines and `#`-prefixed lines are ignored.

Run the existing specs:

```sh
npm test          # TypeScript
go test ./go/...  # Go
```

Add a new case by appending a row to the relevant `.tsv` file; both
test suites pick it up automatically.


## How to disable the default rule wiring

Set `rules` to an empty object to skip all default rule
modifications. Only the directive rule itself is created; its open
token will only match via rules you install via `custom`.

**TypeScript:**

```ts
.use(Directive, { name: 'none', open: '@', action: () => null, rules: null })
```

**Go:**

```go
directive.Apply(j, directive.DirectiveOptions{
    Name:   "none",
    Open:   "@",
    Action: func(*jsonic.Rule, *jsonic.Context) {},
    Rules:  &directive.RulesOption{},
})
```
