# @tabnas/directive

Directive syntax for the [tabnas](https://github.com/tabnas/parser)
parser. A directive is a token sequence (e.g. `@name`, `add<1,2>`) that
triggers custom parsing behaviour. It is a plugin for the tabnas parser
engine — its only dependency — and layers onto whatever host grammar you
supply (it modifies the standard `val` / `list` / `map` / `pair` rules).

This repository contains:

| Path | Description |
|---|---|
| [`ts/`](ts/) | TypeScript / JavaScript implementation (`@tabnas/directive`). **Canonical.** |
| [`go/`](go/) | Go port (`github.com/tabnas/directive/go`). Kept at parity with `ts/`. |
| [`test/spec/`](test/spec/) | Shared conformance fixtures, exercised by both runtimes. |

The TypeScript implementation is the source of truth; the Go port
mirrors its behaviour, options, defaults and test specs. A small set of
intentional differences (Go static typing, engine-API limits) is
recorded in
[`go/doc/concepts.md`](go/doc/concepts.md#differences-from-the-ts-version).

## Tiny example

A directive is a token sequence that triggers custom parsing behaviour.
Here `@` uppercases the following value (using `@tabnas/json` as the host
grammar):

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
```

## Documentation

The four-quadrant docs come in both languages.

**TypeScript** (canonical) — [tutorial](ts/doc/tutorial.md) ·
[how-to guide](ts/doc/guide.md) · [reference](ts/doc/reference.md) ·
[concepts](ts/doc/concepts.md)

**Go** — [tutorial](go/doc/tutorial.md) · [how-to guide](go/doc/guide.md)
· [reference](go/doc/reference.md) · [concepts](go/doc/concepts.md)

Per-language quickstarts live in [`ts/README.md`](ts/README.md) and
[`go/README.md`](go/README.md).

## Build and test

The only dependency is the `tabnas` parser engine, which is not published
to a registry, so both implementations consume it from source.
`scripts/fetch-parser.sh` downloads its GitHub `main` branch over HTTPS
into `vendor/` (git-ignored) and builds the TypeScript engine. The tests
bring their own small grammar ([`ts/test/mini-grammar.ts`](ts/test/mini-grammar.ts),
[`go/mini_grammar_test.go`](go/mini_grammar_test.go)) — just enough
structure (scalars, explicit lists and maps) to exercise the plugin. The
Makefile runs the fetch for you:

```bash
make build   # fetch engine, build both implementations
make test    # fetch engine, build + test both
```

Targeted: `make test-ts`, `make test-go` (each fetches the engine first).
Pin a different engine ref with `TABNAS_PARSER_REF`.

Contributors and AI agents: see [`AGENTS.md`](AGENTS.md) for repository
conventions and the parity rules.

## License

MIT. Copyright (c) Richard Rodger.
