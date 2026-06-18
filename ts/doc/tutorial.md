# Tutorial: your first directive (TypeScript)

This tutorial takes you from nothing to a working parser that
understands a custom *directive*. By the end you will have parsed
`@"x"` into `"X"` and `foo<...>` into a constant, and you will
understand the two directive shapes.

A directive is a plugin for the
[tabnas](https://github.com/tabnas/parser) parser engine. The engine
ships **no grammar of its own** — it is just a lexer plus a rule-based
parser. So a directive cannot stand alone: you bring a **host grammar**
(any grammar that defines the usual `val` / `list` / `map` / `pair`
rules) and the directive layers onto it.

Throughout this tutorial the host grammar is
[`@tabnas/json`](https://github.com/tabnas/json), a relaxed-JSON
grammar. That is why the inputs below use JSON syntax: quoted strings
(`"x"`), quoted map keys, and bracketed lists. Every numbered example
runs against `@tabnas/json` and is checked by this repository's
doc-example test.


## 1. Install the pieces

You need the engine, a host grammar, and this plugin:

```bash
npm install @tabnas/parser @tabnas/json @tabnas/directive
```

```js ignore
const { Tabnas } = require('@tabnas/parser')
const { json } = require('@tabnas/json')
const { Directive } = require('@tabnas/directive')
```


## 2. Register an open-only directive

The simplest directive has just an **open** token. It consumes the one
value that follows. Here `@` uppercases the next value:

```js
const { Tabnas } = require('@tabnas/parser')
const { json } = require('@tabnas/json')
const { Directive } = require('@tabnas/directive')

const j = new Tabnas({ plugins: [json] }).use(Directive, {
  name: 'upper',
  open: '@',
  action: (rule) => {
    rule.node = String(rule.child.node).toUpperCase()
  },
})

j.parse('@"x"')   // => 'X'
```

What each option does:

- `name` — the directive's name. The plugin creates a parse rule with
  this name and uses it as a token-name suffix.
- `open` — the character sequence that triggers the directive.
- `action` — a callback that runs once the body has been parsed. The
  body's value is `rule.child.node`; assign `rule.node` to set the
  directive's result.


## 3. Use it inside structures

Because the plugin (by default) wires the open token into the host's
`val` rule, the directive works **anywhere a value is allowed** —
top-level, inside lists, inside maps:

```js
const { Tabnas } = require('@tabnas/parser')
const { json } = require('@tabnas/json')
const { Directive } = require('@tabnas/directive')

const j = new Tabnas({ plugins: [json] }).use(Directive, {
  name: 'upper',
  open: '@',
  action: (rule) => (rule.node = String(rule.child.node).toUpperCase()),
})

j.parse('[@"a", @"b", 1]')   // => ['A', 'B', 1]
j.parse('{"k":@"a"}')        // => { k: 'A' }
```

The plain `1` passes through untouched — only values preceded by `@`
are transformed.


## 4. Add a close token

An open-only directive grabs exactly one value. To wrap an **arbitrary
body** — a list, a map, anything — add a **close** token. The directive
then consumes everything between open and close.

This `foo<...>` directive ignores its body and always yields `"FOO"`:

```js
const { Tabnas } = require('@tabnas/parser')
const { json } = require('@tabnas/json')
const { Directive } = require('@tabnas/directive')

const j = new Tabnas({ plugins: [json] }).use(Directive, {
  name: 'foo',
  open: 'foo<',
  close: '>',
  action: (rule) => (rule.node = 'FOO'),
})

j.parse('foo<"a">')        // => 'FOO'
j.parse('foo<[1, 2, 3]>')  // => 'FOO'
j.parse('foo<{"x":1}>')    // => 'FOO'
```

It nests and composes with the host grammar like any other value:

```js
const { Tabnas } = require('@tabnas/parser')
const { json } = require('@tabnas/json')
const { Directive } = require('@tabnas/directive')

const j = new Tabnas({ plugins: [json] }).use(Directive, {
  name: 'foo',
  open: 'foo<',
  close: '>',
  action: (rule) => (rule.node = 'FOO'),
})

j.parse('[foo<"a">, 1]')   // => ['FOO', 1]
j.parse('foo<foo<"a">>')   // => 'FOO'
```


## 5. Boundary closing

A neat property of the close token: it also terminates an enclosing
list or map opened **inside** the directive. You do not have to close
the inner bracket before the directive's close token — the close token
closes both at once:

```js
const { Tabnas } = require('@tabnas/parser')
const { json } = require('@tabnas/json')
const { Directive } = require('@tabnas/directive')

const j = new Tabnas({ plugins: [json] }).use(Directive, {
  name: 'foo',
  open: 'foo<',
  close: '>',
  action: (rule) => (rule.node = 'FOO'),
})

j.parse('foo<[1, 2>')   // => 'FOO'
```

Note there is no `]` before the `>` — the `>` closes the list and the
directive together. See the [explanation](concepts.md) for why.


## Where to go next

- [How-to guides](guide.md) — focused recipes (shared close tokens,
  restricting where a directive matches, conditions, reading from
  options).
- [Reference](reference.md) — every option, type, token and counter.
- [Concepts](concepts.md) — how the plugin weaves into the engine's
  rule model, and the design trade-offs.
