See [AGENTS.md](AGENTS.md) for the full agent guide to this repository.

Quick reminders:

- `ts/` (TypeScript) is canonical; `go/` tracks it. Change TypeScript
  first, then update Go to match as far as the Go engine API and Go's
  type system allow.
- The directive is a plugin for the `tabnas` parser engine — its only
  dependency. It is not published; run `scripts/fetch-parser.sh` to
  download + build its GitHub `main` branch into `vendor/` before
  building or testing. The tests bring their own small grammar
  (`ts/test/mini-grammar.ts`, `go/mini_grammar_test.go`) — keep the two
  in step.
- `make build` / `make test` fetch the engine and cover both
  implementations.
- Both implementations must pass the shared `test/spec/*.tsv` fixtures.
  Some TS/Go differences are intentional (static typing, engine-API
  limits) and are recorded in `docs/reference.md`.
- Use the `@tabnas/debug` plugin (`describe()` + tracing) to diagnose
  grammar/alt issues.
