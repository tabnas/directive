# Agent guide: @tabnas/directive

This file orients AI coding agents working in this repository. Keep it
accurate: when the structure, commands, or conventions below change,
update this file in the same change.

## What this repository is

A directive-syntax plugin for the
[`tabnas`](https://github.com/tabnas/parser) parser. A *directive* is a
token sequence — `@name` (open-only) or `add<1,2>` (open + close) — that
pushes into a dedicated rule and fires an action to transform the parsed
body. It is a plugin for the tabnas parser engine and layers onto
whatever host grammar provides the standard `val` / `list` / `map` /
`pair` / `elem` rules; it is not a parser of its own.

## Layout

| Path | What it is |
|---|---|
| `ts/` | TypeScript / JavaScript implementation (`@tabnas/directive`). **Canonical.** |
| `go/` | Go implementation (`github.com/tabnas/directive/go`). Tracks `ts/`. |
| `docs/` | Cross-language documentation (tutorial, how-to, reference, explanation). |
| `test/spec/*.tsv` | Shared conformance fixtures, run by both implementations. |
| `ts/test/mini-grammar.ts`, `go/mini_grammar_test.go` | The small host grammar the tests run against. |
| `scripts/fetch-parser.sh` | Downloads + builds the tabnas engine into `vendor/`. |
| `vendor/` | The fetched engine (git-ignored; created by the script). |
| `.github/workflows/build.yml` | CI: builds and tests both implementations. |

## The dependency

The **only** dependency is the **tabnas** parser engine (npm `tabnas`,
Go module `github.com/tabnas/parser/go`). The plugin source is written
against it: it imports the plugin API types (`Rule`, `Context`, `Tin`,
`Plugin`, `RuleSpec`, `AltSpec`, …) and registers tokens, rules and a
declarative grammar spec via the instance API.

- TypeScript: `import { Tabnas, Rule, Plugin, … } from 'tabnas'`;
  `tabnas` is the peer dependency (`vendor/tabnas-parser/ts`).
- Go: `import tabnas "github.com/tabnas/parser/go"`; a `go.mod` `replace`
  points the require at `vendor/tabnas-parser/go`.

The engine ships **no grammar** of its own (the directive modifies host
grammar rules), so the **tests bring their own**: a deliberately small
grammar (scalars, explicit lists `[a, b]`, explicit maps `{k: v}`) in
`ts/test/mini-grammar.ts` and `go/mini_grammar_test.go`. It is *not* a
JSON/jsonic grammar — just enough structure to exercise the plugin. Its
rule names (`val`, `list`, `map`, `pair`, `elem`) match the directive's
default rule targets. **Keep the two mini grammars in step.**

The engine is not published to a registry, so it is consumed from source
via `scripts/fetch-parser.sh`, which downloads its GitHub `main` branch
into `vendor/` (git-ignored). Pin a ref with `TABNAS_PARSER_REF`; set
`TABNAS_SKIP_TS_BUILD=1` for a Go-only fetch. Always fetch before
installing/building; the Makefile, CI, and the SessionStart hook do this
automatically.

## Build and test

From the repository root:

```bash
make build   # fetch engine, build both implementations
make test    # fetch engine, build + test both
```

Targeted: `make test-ts`, `make test-go` (each fetches the engine first).
Both currently pass: TS via Node's test runner, Go via `go test ./...`.

## The parity rule

**TypeScript is canonical.** `ts/src/directive.ts` is the source of
truth for behaviour, option names, defaults, the grammar spec it builds,
and the order of alts. Change TypeScript first, then update Go to match
as far as the Go engine API and Go's type system allow.

Both implementations must pass the identical shared `test/spec/*.tsv`
fixtures. A new behaviour means a new fixture row, exercised by both.

Some divergence is real and **intended**, not drift (Go static typing,
engine-API differences). The current set is listed in
`docs/reference.md` (§ "TypeScript / Go differences") — keep that table
in sync when behaviour changes. Notable items:

- Go's `Action` is a typed func; TypeScript also accepts a dotted-path
  string and an action may return a `Token`.
- `Rules` is a `map[string]*RuleMod` in Go (no comma-string shorthand);
  a non-`nil` `*RulesOption` is a complete override (`nil` = defaults,
  `&RulesOption{}` = no rules).
- Registration failures (duplicate open token, grammar build error) are
  thrown in TS and returned as an `error` in Go (propagated by `j.Use` /
  `Apply`); the Go plugin never panics.
- Go's `bc` hook walks the `Prev`-linked replacement chain to adopt the
  final child node (a Go slice-reallocation workaround); implicit-list
  bodies in `test/spec/implicit.tsv` exercise it.

## Conventions

- Tests mirror each other: `ts/test/directive.test.ts`,
  `go/directive_test.go`, both driven by `test/spec/*.tsv`.
- Go: run `gofmt` and `go vet ./...` before committing.
- The directive needs a host grammar with `val` / `list` / `map` /
  `pair` rules — examples and tests start from `makeMini()`, which builds
  a `Tabnas` instance with the small test grammar installed.
- Plugin registration follows the standard tabnas shape. TypeScript:
  `Directive` is a `Plugin` with `Directive.defaults`, registered via
  `j.use(Directive, options)`. Go: `Directive` is a `tabnas.Plugin`
  value that reads named keys from the option map; `Apply(j, opts)`
  (returns `(*Tabnas, error)`) is the typed convenience constructor that
  forwards them to `j.Use`.

## Documentation

Docs in `docs/` are organised by purpose: a learning-oriented tutorial,
task-oriented how-to guides, a reference, and an explanation. When you
add a capability, extend the reference and add a how-to if it introduces
a new task.

## Debugging

The [`@tabnas/debug`](https://github.com/tabnas/debug) plugin adds a
`describe()` grammar dump and parse tracing for tabnas instances — useful
when a directive's alts aren't matching as expected. Every alt this
plugin installs carries the `directive` group tag, so traces can be
filtered to directive activity.
