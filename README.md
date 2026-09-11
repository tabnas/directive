# @tabnas/directive

<!-- tabnas-badges -->
[![npm](https://tabnas.github.io/status/badges/directive-npm.svg)](https://www.npmjs.com/package/@tabnas/directive)
[![CI](https://github.com/tabnas/directive/actions/workflows/ci.yml/badge.svg)](https://github.com/tabnas/directive/actions/workflows/ci.yml)
[![go](https://tabnas.github.io/status/badges/directive-go.svg)](https://pkg.go.dev/github.com/tabnas/directive/go)
[![tabnas standard](https://tabnas.github.io/status/badges/directive-standard.svg)](https://tabnas.github.io/status/)
<!-- /tabnas-badges -->

Directive syntax for the [tabnas](https://github.com/tabnas/parser)
parser. A directive is a token sequence (e.g. `@name`, `add<1,2>`) that
triggers custom parsing behaviour. It is a plugin for the tabnas parser
engine — its only dependency — and layers onto whatever host grammar you
supply (it modifies the standard `val` / `list` / `map` / `pair` rules).

Docs, guides, the error reference and the playground: **[tabnas.dev](https://tabnas.dev)**.

This repository contains:

| Path | Description |
|---|---|
| [`ts/`](ts/) | TypeScript / JavaScript implementation (`@tabnas/directive`). **Canonical.** |
| [`go/`](go/) | Go port (`github.com/tabnas/directive/go`). Kept at parity with `ts/`. |
| [`rs/`](rs/) | Rust port (the `tabnas-directive` crate). Kept at parity with `ts/`. |
| [`test/spec/`](test/spec/) | Shared conformance fixtures, exercised by all three runtimes. |

The TypeScript implementation is the source of truth; the Go and Rust
ports mirror its behaviour, options, defaults and test specs. A small set
of intentional differences (static typing, engine-API limits) is
tabulated in
[`docs/reference.md`](docs/reference.md#typescript--go--rust-differences)
and explained per port in
[`go/doc/concepts.md`](go/doc/concepts.md#differences-from-the-ts-version)
and [`rs/doc/concepts.md`](rs/doc/concepts.md#differences-from-the-ts-version).

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

The four-quadrant docs come in all three languages.

**TypeScript** (canonical) — [tutorial](ts/doc/tutorial.md) ·
[how-to guide](ts/doc/guide.md) · [reference](ts/doc/reference.md) ·
[concepts](ts/doc/concepts.md)

**Go** — [tutorial](go/doc/tutorial.md) · [how-to guide](go/doc/guide.md)
· [reference](go/doc/reference.md) · [concepts](go/doc/concepts.md)

**Rust** — [tutorial](rs/doc/tutorial.md) ·
[how-to guide](rs/doc/guide.md) · [reference](rs/doc/reference.md) ·
[concepts](rs/doc/concepts.md)

Per-language quickstarts live in [`ts/README.md`](ts/README.md),
[`go/README.md`](go/README.md) and [`rs/README.md`](rs/README.md).

## Build and test

The only dependency is the `tabnas` parser engine, which is not published
to a registry, so the implementations consume it from source — normally
as a **sibling checkout** of `https://github.com/tabnas/parser` (built
first with `cd parser/ts && npm install && npm run build`), which the Go
module reaches through the `vendor/tabnas-parser` symlink and the Rust
crate reaches through a `path` dependency on `../parser/rs`. The tests
bring their own small grammar ([`ts/test/mini-grammar.ts`](ts/test/mini-grammar.ts),
[`go/mini_grammar_test.go`](go/mini_grammar_test.go),
[`rs/tests/common/mini_grammar.rs`](rs/tests/common/mini_grammar.rs)) —
just enough structure (scalars, explicit lists and maps) to exercise the
plugin.

The Makefile does **not** fetch; it assumes the engine is already in
place:

```bash
make build   # build all three implementations
make test    # test all three implementations
```

Targeted: `make test-ts`, `make test-go`, `make test-rs`.

If you cannot keep a sibling checkout, run `scripts/fetch-parser.sh`
first — it downloads the engine's GitHub `main` branch over HTTPS into
`vendor/` (git-ignored) and builds the TypeScript engine. Pin a different
engine ref with `TABNAS_PARSER_REF`.

Contributors and AI agents: see [`AGENTS.md`](AGENTS.md) for repository
conventions and the parity rules.

## License

MIT. Copyright (c) Richard Rodger.
