# tabnas-directive (Go)

Directive-syntax plugin for the
[`tabnas`](https://github.com/tabnas/parser) parser.

A *directive* is a token sequence — `@name` (open-only) or `add<1,2>`
(open + close) — that pushes into a dedicated rule and fires an action
to transform the parsed body. This is the Go port of the canonical
TypeScript implementation in [`../ts`](../ts); the TypeScript version is
authoritative and this package tracks it. A few intentional differences
(Go static typing, engine-API limits) are listed in
[the reference](../docs/reference.md#typescript--go-differences).

The plugin's only dependency is the tabnas engine
(`github.com/tabnas/parser/go`). It modifies host-grammar rules (`val`,
`list`, `map`, `pair`), so you apply it to a `*tabnas.Tabnas` instance
that already has a grammar installed — not a bare engine. A minimal host
grammar is in [`mini_grammar_test.go`](mini_grammar_test.go).

## Install

```bash
go get github.com/tabnas/parser/go
go get github.com/tabnas/directive/go
```

## Use

```go
package main

import (
	"fmt"
	"strings"

	tabnas "github.com/tabnas/parser/go"
	directive "github.com/tabnas/directive/go"
)

func main() {
	j := tabnas.Make()
	j.Use(hostGrammar) // your grammar: provides val / list / map / pair
	directive.Apply(j, directive.DirectiveOptions{
		Name: "upper",
		Open: "@",
		Action: func(r *tabnas.Rule, _ *tabnas.Context) {
			r.Node = strings.ToUpper(fmt.Sprintf("%v", r.Child.Node))
		},
	})

	v, _ := j.Parse("[@a, @b, 1]") // []any{"A", "B", float64(1)}
	fmt.Printf("%#v\n", v)
}
```

## Build and test

This repository consumes the engine from source. From the repository
root, fetch it first, then build and test:

```bash
TABNAS_SKIP_TS_BUILD=1 ./scripts/fetch-parser.sh  # Go-only: skips the TS build
cd go && go build ./... && go vet ./... && go test ./...
```

Or, from the repository root, `make test-go` does all of the above. The
`go.mod` `replace` directive points the `github.com/tabnas/parser/go`
requirement at the fetched copy in `../vendor/tabnas-parser/go`.

## License

MIT.
