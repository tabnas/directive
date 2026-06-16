# Agents Guide — directive

## What this project is

`directive` is a **directive-syntax plugin** for the
[`tabnas`](https://github.com/tabnas/parser) parser engine. A *directive*
is a token sequence — `@name` (open-only) or `add<1,2>` (open + close) —
that pushes into a dedicated rule and fires an action to transform the
parsed body. It is **not a parser of its own**: it layers onto whatever
host grammar supplies the standard `val` / `list` / `map` / `pair` /
`elem` rules (jsonic, json, or the tests' mini grammar), adds open/close
tokens, modifies those host rules, and installs one rule named after the
directive.

The engine ships **no grammar**, so the plugin only makes sense on top of
a host grammar. The tests therefore bring their own deliberately small
one (scalars, explicit lists `[a, b]`, explicit maps `{k: v}`) in
`ts/test/mini-grammar.ts` / `go/mini_grammar_test.go` — just enough
structure to exercise the plugin, with rule names (`val` / `list` / `map`
/ `pair` / `elem`) matching the directive's default targets.

## Repository map

| Path | What it is |
|---|---|
| [`ts/`](ts/) | **Canonical** TypeScript implementation — the `@tabnas/directive` package. Plugin in `src/directive.ts`. Builds to `dist/` (+ `dist-test/`). Depends on `@tabnas/parser` (peer). |
| [`go/`](go/) | Go port — `github.com/tabnas/directive/go`. Plugin in `directive.go`. Tracks `ts/`. Depends on `github.com/tabnas/parser/go` via a `replace`. |
| [`test/spec/*.tsv`](test/spec/) | Shared conformance fixtures (`input → expected`), run by both implementations. |
| `ts/test/mini-grammar.ts`, `go/mini_grammar_test.go` | The small host grammar (`makeMini()`) the tests run against. Keep the two in step. |
| [`docs/`](docs/) | Cross-language docs: `tutorial.md`, `how-to.md`, `reference.md`, `explanation.md`. |
| `scripts/fetch-parser.sh`, `scripts/fetch-debug.sh` | Standalone fetch-from-source helpers (alternative to the sibling checkout; see below). |
| `vendor/` | Git-ignored. Holds `tabnas-parser` — a symlink to the sibling `../parser` checkout (or a fetched copy), used by the Go `replace`. |

## The tabnas engine dependency

The **only** runtime dependency is the **tabnas** parser engine (npm
`@tabnas/parser`, Go module `github.com/tabnas/parser/go`). The plugin is
written against its plugin API — it imports `Tabnas`, `Rule`, `RuleSpec`,
`StateAction`, `Plugin`, `Context`, `Token`, `Tin` and registers tokens,
rule modifications and a declarative grammar spec via the instance API.

Both runtimes consume the engine as a **sibling checkout** (the standard
tabnas development model, until `tabnas/parser` publishes tagged
packages):

- TypeScript: `"@tabnas/parser": ">=2"` is the **peerDependency**, mirrored
  as `"@tabnas/parser": "file:../../parser/ts"` in `devDependencies` so
  local builds resolve it. (`@tabnas/debug` and `@tabnas/railroad` are
  also `file:` **devDependencies** — see below.) `engines.node` is `>=24`.
- Go: `go/go.mod` has `replace github.com/tabnas/parser/go =>
  ../vendor/tabnas-parser/go`, and `vendor/tabnas-parser` is a symlink to
  the sibling `../parser`. The module is **vendor-replaced and excluded
  from the repo-wide `go.work`**, so all Go commands run with **`GOWORK=off`**
  (the Makefile sets this for you).

Clone `https://github.com/tabnas/parser` as a sibling of this repo and
build its TS (`cd parser/ts && npm install && npm run build`) before
working here. CI clones the engine (and the other siblings) and builds
them first.

`scripts/fetch-parser.sh` is the **standalone** alternative: it downloads
the engine's GitHub `main` branch over HTTPS into `vendor/` (pin a ref
with `TABNAS_PARSER_REF`; `TABNAS_SKIP_TS_BUILD=1` for a Go-only fetch).
Use it only when you can't keep a sibling checkout. Note the `vendor/`
symlink and the fetch script populate the **same** path the Go `replace`
points at.

## Authority and alignment rules

1. **TypeScript is canonical.** `ts/src/directive.ts` is the source of
   truth for behaviour, option names, defaults, the grammar spec it
   builds, and the order of alts. Change TS first, then update Go to match
   as far as the Go engine API and Go's type system allow.
2. The shared `test/spec/*.tsv` fixtures are the **parity contract**. Both
   suites run them and both must stay green; a new behaviour means a new
   fixture row, exercised by both runtimes.
3. Some divergence is real and **intended**, not drift (Go static typing,
   engine-API differences). The current set is tabulated in
   `docs/reference.md` (§ "TypeScript / Go differences"); keep it in sync
   when behaviour changes. Notable items:
   - Go's `Action` is a typed `func(rule *tabnas.Rule, ctx *tabnas.Context)`;
     TS also accepts a dotted-path **string** (`tabnas.util.prop` lookup)
     and an action may return a `Token`.
   - Go `Rules` is `*RulesOption` (`map[string]*RuleMod` fields): `nil`
     selects the defaults, `&RulesOption{}` modifies no rules.
   - Registration failures (duplicate open token, grammar build error) are
     **thrown** in TS and returned as an `error` in Go (propagated by
     `j.Use` / `Apply`); the Go plugin never panics.
   - Go's `bc` hook walks the `Prev`-linked replacement chain to adopt the
     final child node (a Go slice-reallocation workaround); the
     implicit-list bodies in `test/spec/implicit.tsv` exercise it.
4. Keep the two mini grammars (`ts/test/mini-grammar.ts`,
   `go/mini_grammar_test.go`) in step — they define the rule surface the
   directive modifies.

## How the plugin works (the non-obvious parts)

- **Default targets.** `Directive.defaults.rules` is `{ open: 'val',
  close: 'list,elem,map,pair' }`: by default a directive operates where
  `val`s occur, and (when it has a `close` token) closes inside the
  container rules. An absent `rules` key uses these defaults; a present
  `rules` (even empty) is honoured verbatim.
- **Tokens.** `open` becomes the fixed token `#OD_<name>`; `close` (if
  given and not already a fixed token) becomes `#CD_<name>`. The **open
  token must be unique** — re-registering an existing fixed token throws
  (TS) / errors (Go).
- **Rule surface.** The plugin `clear()`s the `<name>` rule, sets `bo`
  (seed `rule.node = {}`) and `bc` (call the action; a returned token is
  forwarded), then installs open/close alts via `tabnas.grammar(spec, {
  rule: { alt: { g: 'directive' } } })`. **Every alt it installs carries
  the `directive` group tag**, so `@tabnas/debug` traces can be filtered
  to directive activity.
- **Open vs open+close.** With a `close` token the plugin emits a more
  specific `[OPEN, CLOSE]` alt before `[OPEN]`, and the directive rule
  consumes implicits only when bounded by a close (`dlist:0/dmap:0`);
  open-only directives set `dlist:1/dmap:1` to avoid eating following
  siblings. The `dr_<name>` counter guards close matching.

## Build & test

The standard tabnas Makefile (`GOWORK=off` for the vendor-replaced Go
module) drives both runtimes from the repo root:

```bash
make build   # build-ts (npm run build) + build-go (GOWORK=off go build)
make test    # test-ts (npm test) + test-go (GOWORK=off go test -v)
```

Targeted: `make build-ts` / `make test-ts`, `make build-go` /
`make test-go`, `make clean`, `make reset`. The Makefile does **not**
fetch — it assumes the sibling `../parser` (and the `vendor/tabnas-parser`
symlink) is in place; run `scripts/fetch-parser.sh` first only if you are
not using a sibling checkout.

Directly:

```bash
cd ts && npm install && npm test          # tsc --build src test, then node --test dist-test/*.test.js
cd go && GOWORK=off go test ./...          # also runs the shared spec fixtures
```

TS tests: `directive.test.ts` (spec-driven), `doc-examples.test.ts`
(checks the doc snippets), `debug.test.ts` (composition with
`@tabnas/debug`, below). Go: `directive_test.go`, driven by the same
`test/spec/*.tsv` and the Go mini grammar. Run `gofmt` and
`go vet ./...` before committing Go.

## @tabnas/debug and @tabnas/railroad (dev-only)

Neither is a runtime dependency — the directive's only dependency is the
engine — but both are `file:` **devDependencies** in `ts/package.json`:

- **`@tabnas/debug`** (`file:../../debug/ts`) is the diagnostic tool for
  this plugin: `j.debug.describe()` dumps the grammar/alts and
  `j.debug.model()` returns a structured grammar model.
  `ts/test/debug.test.ts` composes `makeMini().use(Directive,
  …).use(Debug, …)` and asserts `model()` captures the directive's
  `<name>` rule and `#OD_<name>` open token, the host rules
  (`val`/`list`/`map`/`pair`/`elem`), and the plugin order
  (`['mini','Directive','Debug']`). `scripts/fetch-debug.sh` vendors debug
  for local use when you don't have a sibling checkout (run
  `fetch-parser.sh` first).
- **`@tabnas/railroad`** (`file:../../railroad/ts`) is the railroad/syntax
  diagram generator, available as dev-only tooling for inspecting a host
  grammar with the directive applied. This repo ships no committed diagram
  (the directive has no grammar of its own — it modifies whatever host
  grammar it is layered on).

## Publishing & versioning

- TS: `make publish-ts` runs the tests then `npm publish` at the current
  `ts/package.json` version (`2.2.0`).
- Go: `make publish-go V=x.y.z` seds the top-level `const Version` in
  `go/directive.go` (currently `0.1.4`), commits, tags `go/vX.Y.Z`, pushes,
  and (if `gh` is present) cuts a GitHub release. `make tags-go` lists the
  Go tags newest-first.

## CI

`.github/workflows/build.yml` uses the **sibling-checkout** strategy
across `ubuntu` / `windows` / `macos` (Node 24): it sets
`core.autocrlf=false` (CRLF corrupts `.tsv` fixtures), clones the
transitive `@tabnas` closure (`parser debug json abnf railroad`) into
sibling dirs, `npm i && npm run build`s each (then this repo) in order,
and runs `npm test` from `directive/ts`. The current workflow exercises
only the **TypeScript** suite; build/test Go locally with `make test-go`
(`GOWORK=off`).
