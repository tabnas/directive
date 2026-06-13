# @tabnas/directive

Directive syntax for the [tabnas](https://github.com/tabnas/parser)
parser. A directive is a token sequence (e.g. `@name`, `add<1,2>`) that
triggers custom parsing behaviour. The plugin extends the
[jsonic](https://github.com/tabnas/jsonic) relaxed-JSON grammar, which
runs on the tabnas engine.

This repository contains:

| Path | Description |
|---|---|
| [`ts/`](ts/) | TypeScript / JavaScript implementation (`@tabnas/directive`). **Canonical.** |
| [`go/`](go/) | Go port (`github.com/tabnas/directive/go`). Kept at parity with `ts/`. |
| [`docs/`](docs/) | Cross-language documentation. |
| [`test/spec/`](test/spec/) | Shared conformance fixtures, exercised by both runtimes. |

The TypeScript implementation is the source of truth; the Go port
mirrors its behaviour, options, defaults and test specs. A small set of
intentional differences (Go static typing, engine-API limits) is
recorded in [`docs/reference.md`](docs/reference.md#typescript--go-differences).

## Documentation

Start with the [tutorial](docs/tutorial.md), reach for a
[how-to guide](docs/how-to.md) when you have a specific task, look up
options in the [reference](docs/reference.md), and read the
[explanation](docs/explanation.md) to understand the grammar model.
Per-language usage lives in [`ts/README.md`](ts/README.md) and
[`go/README.md`](go/README.md).

## Build and test

The directive's source dependencies — the `tabnas` parser engine and
the `jsonic` relaxed-JSON grammar — are not published to a registry, so
both implementations consume them from source. `scripts/fetch-deps.sh`
downloads their GitHub `main` branches over HTTPS into `vendor/`
(git-ignored) and builds the TypeScript packages. The Makefile runs it
for you:

```bash
make build   # fetch deps, build both implementations
make test    # fetch deps, build + test both
```

Targeted: `make test-ts`, `make test-go` (each fetches deps first). Pin
different dependency refs with `TABNAS_PARSER_REF` / `TABNAS_JSONIC_REF`.

Contributors and AI agents: see [`AGENTS.md`](AGENTS.md) for repository
conventions and the parity rules.

## License

MIT. Copyright (c) Richard Rodger.
