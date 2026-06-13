# @tabnas/directive — directive syntax for the tabnas parser

Adds directive syntax to the [tabnas](https://github.com/tabnas/parser)
parser. A directive is a token sequence (e.g. `@name`, `add<1,2>`) that
triggers custom parsing behaviour. The plugin extends the
[jsonic](https://github.com/tabnas/jsonic) relaxed-JSON grammar.
TypeScript (canonical) and Go ports share the same API shape and test
specs.

[![npm version](https://img.shields.io/npm/v/@tabnas/directive.svg)](https://npmjs.com/package/@tabnas/directive)
[![build](https://github.com/tabnas/directive/actions/workflows/build.yml/badge.svg)](https://github.com/tabnas/directive/actions/workflows/build.yml)


## Documentation

The cross-language docs live in [`../docs/`](../docs): a
[tutorial](../docs/tutorial.md), [how-to guides](../docs/how-to.md), a
[reference](../docs/reference.md), and an [explanation](../docs/explanation.md)
of how the plugin works.


## Quickstart

### TypeScript

```ts
import { Jsonic } from 'jsonic'
import { Directive } from '@tabnas/directive'

const j = Jsonic.make().use(Directive, {
  name: 'upper',
  open: '@',
  action: (rule) => (rule.node = String(rule.child.node).toUpperCase()),
})

j('[@a, @b, 1]') // → ['A', 'B', 1]
```

### Go

```go
import (
    "fmt"
    "strings"

    tabnas "github.com/tabnas/parser/go"
    jsonic "github.com/tabnas/parser/go/jsonic"
    directive "github.com/tabnas/directive/go"
)

j := jsonic.Make()
directive.Apply(j, directive.DirectiveOptions{
    Name: "upper",
    Open: "@",
    Action: func(r *tabnas.Rule, _ *tabnas.Context) {
        r.Node = strings.ToUpper(fmt.Sprintf("%v", r.Child.Node))
    },
})

j.Parse("[@a, @b, 1]") // → []any{"A", "B", float64(1)}
```


## Build and test

The `tabnas` engine and `jsonic` grammar are consumed from source. From
the repository root, `make build` / `make test` fetch them into
`vendor/` and build/test both implementations. See the
[root README](../README.md) and [`../AGENTS.md`](../AGENTS.md).


## License

MIT — see [LICENSE](LICENSE).
