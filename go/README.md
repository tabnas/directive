# tabnas-directive (Go)

Directive-syntax plugin for the
[`tabnas`](https://github.com/tabnas/parser) parser.

A *directive* is a token sequence, `@name` (open-only) or `add<1,2>`
(open + close), that pushes into a dedicated rule and fires an action
to transform the parsed body. This is the Go port of the canonical
TypeScript implementation in [`../ts`](../ts); the TypeScript version is
authoritative and this package tracks it. A few intentional differences
(Go static typing, engine-API limits) are listed in
[the concepts doc](doc/concepts.md#differences-from-the-ts-version).

## Documentation

The four-quadrant Go docs live in [`doc/`](doc):
[tutorial](doc/tutorial.md) · [how-to guide](doc/guide.md) ·
[reference](doc/reference.md) · [concepts](doc/concepts.md). The
canonical TypeScript docs are in [`../ts/doc/`](../ts/doc).

The plugin's only dependency is the tabnas engine
(`github.com/tabnas/parser/go`). It modifies host-grammar rules (`val`,
`list`, `map`, `pair`), so you apply it to a `*tabnas.Tabnas` instance
that already has a grammar installed, not a bare engine. A minimal host
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
	tabnasdirective "github.com/tabnas/directive/go"
)

func main() {
	j := tabnas.Make()
	j.Use(hostGrammar) // your grammar: provides val / list / map / pair
	tabnasdirective.Apply(j, tabnasdirective.DirectiveOptions{
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

The module requires the published engine, and `github.com/tabnas/support/go`
for the tests, from the Go module proxy, with no `replace`, so there is
nothing to fetch first:

```bash
cd go && go build ./... && go vet ./... && go test ./...
```

Or, from the repository root, `make test-go` runs the tests above with
`GOWORK=off`, against the versions `go.mod` requires.

## License

MIT.
