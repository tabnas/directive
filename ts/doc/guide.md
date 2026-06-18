# How-to guides (TypeScript)

Focused recipes for real tasks. Each assumes you have read the
[tutorial](tutorial.md) and can register a basic directive.

Most recipes register against a host grammar that defines `val` /
`list` / `map` / `pair`. The runnable, asserted snippets here use
[`@tabnas/json`](https://github.com/tabnas/json); recipes that depend on
how a *specific* host propagates body nodes (for example, summing a
parsed list) are shown illustratively with a `hostGrammar` placeholder,
because that behaviour is a property of the host, not the plugin. The
repository's own test grammar (`ts/test/mini-grammar.ts`) is the
reference host for those.


## Wrap an arbitrary body with a close token

Use `close` when a directive should consume everything up to a closing
token rather than a single value.

```js
const { Tabnas } = require('@tabnas/parser')
const { json } = require('@tabnas/json')
const { Directive } = require('@tabnas/directive')

const j = new Tabnas({ plugins: [json] }).use(Directive, {
  name: 'group',
  open: '(',
  close: ')',
  action: (rule) => (rule.node = 'GROUP'),
})

j.parse('([1, 2, 3])')   // => 'GROUP'
```


## Share a close token between two directives

Register a second directive with the same `close` string. The plugin
detects that the close character is already a registered fixed token
and **reuses** it rather than creating a duplicate (which the lexer
could not represent):

```js ignore
const j = new Tabnas().use(hostGrammar)
  .use(Directive, { name: 'foo', open: 'foo<', close: '>', action: fooAction })
  .use(Directive, { name: 'bar', open: 'bar<', close: '>', action: barAction })

j.parse('[foo<a>, bar<b>]')   // → ['FOO', 'BAR']
```

The **open** tokens (`foo<`, `bar<`) must still be unique. Reusing an
open token throws:

```js ignore
// Throws: "Directive open token already in use: foo<"
j.use(Directive, { name: 'baz', open: 'foo<', action: () => null })
```


## Restrict where a directive is recognised

By default the open token is wired into the `val` rule, so it matches
in any value position. Pass `rules.open` to narrow it. The shorthand is
a comma string or string array of host rule names:

```js ignore
new Tabnas().use(hostGrammar).use(Directive, {
  name: 'inject',
  open: '@',
  rules: { open: 'val,pair' },   // value positions AND pair positions
  action: (rule) => { /* … */ },
})
```

A common use is a directive that can appear as a whole map entry (a
`pair`) as well as a value — for example `@key` that merges a looked-up
object into the surrounding map. In the `pair` branch your action
mutates `rule.parent.node` instead of setting `rule.node`:

```js ignore
action: (rule) => {
  const val = lookup('' + rule.child.node)
  if ('pair' === rule.parent.name) {
    Object.assign(rule.parent.node, val)   // merge into the map
  } else {
    rule.node = val                         // plain value position
  }
}
```


## Gate a rule modification with a condition

The record form of `rules.open` / `rules.close` attaches a per-rule
condition `c`. The directive only matches inside that host rule when
`c` returns truthy:

```js ignore
rules: {
  open: {
    val:  {},                       // always match in val
    pair: { c: (r) => r.lte('pk') },// match in pair only when pk <= its limit
  },
}
```

Conditions receive `(rule, ctx)` and return a boolean. They are the
same `AltCond` shape the engine uses everywhere.


## Read a value from the parser options at fire time

The `action` field also accepts a **dotted-path string**. Instead of a
callback, the plugin looks up that path on the instance options every
time the directive fires, and uses it as the result:

```js ignore
const j = new Tabnas().use(hostGrammar).use(Directive, {
  name: 'constant',
  open: '@',
  action: 'custom.x',     // resolves j.options().custom.x at fire time
})
j.options({ custom: { x: 42 } })

j.parse('@y')   // → 42
```

This is handy for injecting configuration without closing over it.


## Run extra wiring after setup with `custom`

`custom` fires last, after the plugin has resolved its tokens. You get
the resolved `OPEN` / `CLOSE` Tins and the directive `name`, so you can
install your own alternates that reference the directive's tokens
without re-resolving them:

```js ignore
new Tabnas().use(hostGrammar).use(Directive, {
  name: 'subobj',
  open: '@',
  action: (rule) => { /* … */ },
  custom: (tn, { OPEN, name }) => {
    tn.rule('val', (rs) => {
      rs.open({
        s: [OPEN],
        c: (r) => 0 === r.d,           // only at top level
        p: 'map',
        b: 1,
        n: { [name + '_top']: 1 },
        g: name + '-top',
      })
    })
  },
})
```


## Turn off all default rule wiring

Pass `rules: null` to modify **no** host rules. Only the directive's own
rule is created; its open token then matches only through alternates you
install yourself in `custom`. Without that, the open token is
unrecognised:

```js
const { Tabnas } = require('@tabnas/parser')
const { json } = require('@tabnas/json')
const { Directive } = require('@tabnas/directive')

const j = new Tabnas({ plugins: [json] }).use(Directive, {
  name: 'none',
  open: '@',
  action: () => null,
  rules: null,
})

let err = 'no error'
try { j.parse('[@"a"]') } catch (e) { err = e.code }
err   // => 'unexpected'
```


## Test a directive against a shared spec file

Conformance rows live in `test/spec/*.tsv`. Each row is one of:

```
<input><TAB><expected-json>
<input><TAB>!error <regex>
```

Blank lines and `#`-prefixed lines are ignored. Both the TypeScript and
Go suites load the same files, so a new row is exercised by both
runtimes. Run them with `npm test` (TS) and `go test ./...` (Go), or
`make test` from the repo root.
