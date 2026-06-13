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

The plugin extends the relaxed-JSON
[`jsonic`](https://github.com/tabnas/jsonic) grammar
(`github.com/jsonicjs/jsonic/go`), so it operates on a `jsonic.Make()`
instance — not a bare engine.

## Install

```bash
go get github.com/jsonicjs/jsonic/go
go get github.com/tabnas/directive/go
```

## Use

```go
package main

import (
	"fmt"
	"strings"

	jsonic "github.com/jsonicjs/jsonic/go"
	directive "github.com/tabnas/directive/go"
)

func main() {
	j := jsonic.Make()
	directive.Apply(j, directive.DirectiveOptions{
		Name: "upper",
		Open: "@",
		Action: func(r *jsonic.Rule, _ *jsonic.Context) {
			r.Node = strings.ToUpper(fmt.Sprintf("%v", r.Child.Node))
		},
	})

	v, _ := j.Parse("[@a, @b, 1]") // []any{"A", "B", float64(1)}
	fmt.Printf("%#v\n", v)
}
```

## Build and test

This repository consumes its dependencies from source. From the
repository root, fetch them first, then build and test:

```bash
TABNAS_SKIP_TS_BUILD=1 ./scripts/fetch-deps.sh  # Go-only: skips the TS builds
cd go && go build ./... && go vet ./... && go test ./...
```

Or, from the repository root, `make test-go` does all of the above. The
`go.mod` `replace` directive points the `github.com/jsonicjs/jsonic/go`
requirement at the fetched copy in `../vendor/tabnas-jsonic/go`.

## License

MIT.
