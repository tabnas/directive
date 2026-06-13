See [AGENTS.md](AGENTS.md) for the full agent guide to this repository.

Quick reminders:

- `ts/` (TypeScript) is canonical; `go/` tracks it. Change TypeScript
  first, then update Go to match as far as the Go engine API and Go's
  type system allow.
- The directive extends the `jsonic` relaxed-JSON grammar, which runs on
  the `tabnas` engine. Neither is published; run `scripts/fetch-deps.sh`
  to download + build their GitHub `main` branches into `vendor/` before
  building or testing.
- `make build` / `make test` fetch the dependencies and cover both
  implementations.
- Both implementations must pass the shared `test/spec/*.tsv` fixtures.
  Some TS/Go differences are intentional (static typing, engine-API
  limits) and are recorded in `docs/reference.md`.
- Use the `@tabnas/debug` plugin (`describe()` + tracing) to diagnose
  grammar/alt issues.
