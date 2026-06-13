# Agent guide: go/ (parity)

This is the Go port of `@tabnas/directive`. It is **not** canonical: it
tracks the TypeScript implementation in `../ts`, which is the source of
truth. See [../AGENTS.md](../AGENTS.md) for the parity rules and the full
list of intentional TS/Go differences.

- Source: `directive.go`. Provides `Directive` (a `tabnas.Plugin`),
  `Apply(j, opts)` (a convenience wrapper over `j.Use`), and the option
  types `DirectiveOptions`, `RulesOption`, `RuleMod`, `Action`,
  `CustomFunc`, `DirectiveConfig`.
- Tests: `directive_test.go`, driven by the shared `../test/spec/*.tsv`
  fixtures, mirroring `../ts/test/directive.test.ts`.
- Module `github.com/tabnas/directive/go`. The engine module
  `github.com/tabnas/parser/go` is required with a `replace` pointing at
  `../vendor/tabnas-parser/go`; fetch it with `../scripts/fetch-deps.sh`
  first.

```bash
TABNAS_SKIP_TS_BUILD=1 ../scripts/fetch-deps.sh
go build ./... && go vet ./... && go test ./...
```

## Dependency wiring

- The plugin source imports the **engine** for its types:
  `import tabnas "github.com/tabnas/parser/go"` (`tabnas.Rule`,
  `tabnas.Context`, `tabnas.RuleSpec`, `tabnas.AltSpec`, `tabnas.Tin`,
  `tabnas.GrammarSpec`, …).
- The tests obtain a relaxed-JSON parser from the engine's grammar
  subpackage: `import jsonic "github.com/tabnas/parser/go/jsonic"` then
  `j := jsonic.Make()`. That subpackage lives inside the vendored engine
  module, so the single `replace` covers it — there is no separate
  jsonic vendor for Go (unlike TypeScript).

## Parity notes

Keep behaviour and option semantics aligned with `../ts`. The Go engine
and Go's type system force a few intentional differences (all recorded
in `../docs/reference.md`):

- `Action` is `func(rule *tabnas.Rule, ctx *tabnas.Context)` — no
  dotted-path string form, no `Token` return.
- `Rules` is `*RulesOption` with `map[string]*RuleMod` fields: `nil`
  uses defaults, `&RulesOption{}` modifies no rules.
- The `<name>_close` error/hint templates are omitted: calling
  `SetOptions` from inside the plugin would re-apply plugins and re-enter
  `Directive` (panic on the duplicate open token), and the templates are
  dormant in both runtimes anyway.
- The `@<name>-bc` state action walks the `Prev`-linked replacement
  chain to adopt the final child node before calling the action — a
  workaround for Go slice reallocation when a `val` becomes an implicit
  list.

When TS gains or loses behaviour, port it here if the engine API allows;
if it cannot be matched, record the difference in
`../docs/reference.md`.
