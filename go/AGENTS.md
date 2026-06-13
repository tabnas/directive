# Agent guide: go/ (parity)

This is the Go port of `@tabnas/directive`. It is **not** canonical: it
tracks the TypeScript implementation in `../ts`, which is the source of
truth. See [../AGENTS.md](../AGENTS.md) for the parity rules and the full
list of intentional TS/Go differences.

- Source: `directive.go`. Provides `Directive` (a `tabnas.Plugin` value),
  `Apply(j, opts)` (the typed convenience constructor over `j.Use`), and
  the option types `DirectiveOptions`, `RulesOption`, `RuleMod`,
  `Action`, `CustomFunc`, `DirectiveConfig`.
- Tests: `directive_test.go`, driven by the shared `../test/spec/*.tsv`
  fixtures, mirroring `../ts/test/directive.test.ts`. The host grammar
  the tests run against is `mini_grammar_test.go` (`makeMini()`), the Go
  twin of `../ts/test/mini-grammar.ts` — keep the two in step.
- Module `github.com/tabnas/directive/go`. The engine module
  `github.com/tabnas/parser/go` is required with a `replace` pointing at
  `../vendor/tabnas-parser/go`; fetch it with `../scripts/fetch-parser.sh`
  first.

```bash
TABNAS_SKIP_TS_BUILD=1 ../scripts/fetch-parser.sh
go build ./... && go vet ./... && go test ./...
```

## Dependency wiring

- The only dependency is the tabnas parser engine. The plugin imports its
  plugin-API types from it:
  `import tabnas "github.com/tabnas/parser/go"` (`tabnas.Rule`,
  `tabnas.Context`, `tabnas.RuleSpec`, `tabnas.AltSpec`, `tabnas.Tin`,
  `tabnas.GrammarSpec`, …).
- The engine ships no grammar, so the tests install the small
  `mini_grammar_test.go` grammar (`val` / `list` / `map` / `pair` /
  `elem`) on a bare `tabnas.Make()` instance, then apply the directive.

## Plugin registration

`Directive` is a `tabnas.Plugin` — `func(j *tabnas.Tabnas, opts
map[string]any) error` — that reads named option keys (`"name"`,
`"open"`, `"close"`, `"action"`, `"rules"`, `"custom"`), matching the
house plugin style (cf. the engine's `Debug` plugin reading
`opts["trace"]`). `Apply(j, DirectiveOptions{…})` is the typed front door
that builds that map and calls `j.Use`. An absent `"rules"` key selects
the defaults; a present `*RulesOption` (even empty) is honoured verbatim.

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
