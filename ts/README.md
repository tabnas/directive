# @tabnas/directive — directive syntax for the tabnas parser

Adds directive syntax to the [tabnas](https://github.com/tabnas/parser)
parser. A directive is a token sequence (e.g. `@name`, `add<1,2>`) that
triggers custom parsing behaviour. It is a plugin for the tabnas engine —
its only dependency — and layers onto whatever host grammar provides the
standard `val` / `list` / `map` / `pair` rules. TypeScript (canonical)
and Go ports share the same API shape and test specs.

[![npm version](https://img.shields.io/npm/v/@tabnas/directive.svg)](https://npmjs.com/package/@tabnas/directive)
[![build](https://github.com/tabnas/directive/actions/workflows/build.yml/badge.svg)](https://github.com/tabnas/directive/actions/workflows/build.yml)


## Documentation

The four-quadrant TypeScript docs live in [`doc/`](doc):
[tutorial](doc/tutorial.md) · [how-to guide](doc/guide.md) ·
[reference](doc/reference.md) · [concepts](doc/concepts.md). The Go-port
docs are in [`../go/doc/`](../go/doc).


## Quickstart

The directive modifies host-grammar rules, so apply it to a `Tabnas`
instance that already has a grammar that provides `val` / `list` / `map`
/ `pair`. Here the host is [`@tabnas/json`](https://github.com/tabnas/json),
which is why the input uses JSON syntax (quoted strings):

```js
const { Tabnas } = require('@tabnas/parser')
const { json } = require('@tabnas/json')
const { Directive } = require('@tabnas/directive')

const j = new Tabnas({ plugins: [json] }).use(Directive, {
  name: 'upper',
  open: '@',
  action: (rule) => (rule.node = String(rule.child.node).toUpperCase()),
})

j.parse('@"x"')              // => 'X'
j.parse('[@"a", @"b", 1]')   // => ['A', 'B', 1]
```

A minimal host grammar (used by the tests) lives in
[`test/mini-grammar.ts`](test/mini-grammar.ts); see the
[tutorial](doc/tutorial.md) for a step-by-step walkthrough.


## Build and test

The `tabnas` engine is the only dependency and is consumed from source.
From the repository root, `make build` / `make test` fetch it into
`vendor/` and build/test both implementations. The tests bring their own
small grammar. See the [root README](../README.md) and
[`../AGENTS.md`](../AGENTS.md).


## License

MIT — see [LICENSE](LICENSE).
