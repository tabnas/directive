# Agent guide: @tabnas/directive

This file orients AI coding agents working in this repository. Keep it
accurate: when the structure, commands, or conventions below change,
update this file in the same change.

## What this repository is

A directive-syntax plugin for the
[`tabnas`](https://github.com/tabnas/parser) parser. A *directive* is a
token sequence — `@name` (open-only) or `add<1,2>` (open + close) — that
pushes into a dedicated rule and fires an action to transform the parsed
body. The plugin extends the
[`jsonic`](https://github.com/tabnas/jsonic) relaxed-JSON grammar; it is
not a parser of its own.

## Layout

| Path | What it is |
|---|---|
| `ts/` | TypeScript / JavaScript implementation (`@tabnas/directive`). **Canonical.** |
| `go/` | Go implementation (`github.com/tabnas/directive/go`). Tracks `ts/`. |
| `docs/` | Cross-language documentation (tutorial, how-to, reference, explanation). |
| `test/spec/*.tsv` | Shared conformance fixtures, run by both implementations. |
| `scripts/fetch-deps.sh` | Downloads + builds the source dependencies into `vendor/`. |
| `vendor/` | The fetched dependencies (git-ignored; created by the script). |
| `.github/workflows/build.yml` | CI: builds and tests both implementations. |

## The source dependencies

The directive sits on two packages, neither published to a registry, so
both are consumed from source via `scripts/fetch-deps.sh` (downloads
their GitHub `main` branches into `vendor/`):

- **tabnas** — the parser engine. npm `tabnas` (`vendor/tabnas-parser/ts`),
  Go module `github.com/tabnas/parser/go`. Supplies the plugin's API
  *types* (`Rule`, `Context`, `Tin`, `RuleSpec`, `AltSpec`, …). The
  plugin source is written against the engine.
- **jsonic** — the relaxed-JSON grammar (unquoted keys, implicit
  lists/maps, …) the directive extends. The directive needs grammar
  rules (`val`, `map`, `pair`, `list`, `elem`) to exist, so tests run
  against a jsonic instance.
  - TypeScript: npm `jsonic` (`vendor/tabnas-jsonic/ts`), used by the
    tests. The TS source imports its types from `jsonic` (which
    re-exports the engine types), and `jsonic` is the peer dependency.
  - Go: the grammar is a subpackage of the engine,
    `github.com/tabnas/parser/go/jsonic` — no separate vendored module.

Pin refs with `TABNAS_PARSER_REF` / `TABNAS_JSONIC_REF`; set
`TABNAS_SKIP_TS_BUILD=1` for a Go-only fetch. Always fetch before
installing/building; the Makefile, CI, and the SessionStart hook do this
automatically.

## Build and test

From the repository root:

```bash
make build   # fetch deps, build both implementations
make test    # fetch deps, build + test both
```

Targeted: `make test-ts`, `make test-go` (each fetches deps first). Both
currently pass: TS via Node's test runner, Go via `go test ./...`.

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
- The dormant `<name>_close` error/hint templates are registered in TS
  only (the Go engine's `SetOptions` re-applies plugins, so a plugin
  cannot call it without re-entering).
- Go's `bc` hook walks the `Prev`-linked replacement chain to adopt the
  final child node (a Go slice-reallocation workaround).

## Conventions

- Tests mirror each other: `ts/test/directive.test.ts`,
  `go/directive_test.go`, both driven by `test/spec/*.tsv`.
- Go: run `gofmt` and `go vet ./...` before committing.
- The directive operates on a *jsonic* instance, not a bare engine —
  examples and tests start from `Jsonic.make()` (TS) /
  `jsonic.Make()` (Go, from `.../go/jsonic`).

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
