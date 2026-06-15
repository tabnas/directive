# @tabnas/directive — directive syntax for the tabnas parser

Adds directive syntax to the [tabnas](https://github.com/tabnas/parser)
parser. A directive is a token sequence (e.g. `@name`, `add<1,2>`) that
triggers custom parsing behaviour. It is a plugin for the tabnas engine —
its only dependency — and layers onto whatever host grammar provides the
standard `val` / `list` / `map` / `pair` rules. TypeScript (canonical)
and Go ports share the same API shape and test specs.

[![npm version](https://img.shields.io/npm/v/@tabnas/directive.svg)](https://npmjs.com/package/@tabnas/directive)
[![build](https://github.com/tabnas/directive/actions/workflows/build.yml/badge.svg)](https://github.com/tabnas/directive/actions/workflows/build.yml)


## Documentation

The cross-language docs live in [`../docs/`](../docs): a
[tutorial](../docs/tutorial.md), [how-to guides](../docs/how-to.md), a
[reference](../docs/reference.md), and an [explanation](../docs/explanation.md)
of how the plugin works.


## Quickstart

The directive modifies host-grammar rules, so apply it to a `Tabnas`
instance that already has a grammar (here `hostGrammar`, which provides
`val` / `list` / `map` / `pair`).

### TypeScript

```ts
import { Tabnas } from '@tabnas/parser'
import { Directive } from '@tabnas/directive'

const j = new Tabnas().use(hostGrammar).use(Directive, {
  name: 'upper',
  open: '@',
  action: (rule) => (rule.node = String(rule.child.node).toUpperCase()),
})

j.parse('[@a, @b, 1]') // → ['A', 'B', 1]
```

### Go

```go
import (
    "fmt"
    "strings"

    tabnas "github.com/tabnas/parser/go"
    directive "github.com/tabnas/directive/go"
)

j := tabnas.Make()
j.Use(hostGrammar) // provides val / list / map / pair
directive.Apply(j, directive.DirectiveOptions{
    Name: "upper",
    Open: "@",
    Action: func(r *tabnas.Rule, _ *tabnas.Context) {
        r.Node = strings.ToUpper(fmt.Sprintf("%v", r.Child.Node))
    },
})

j.Parse("[@a, @b, 1]") // → []any{"A", "B", float64(1)}
```

A minimal host grammar (used by the tests) lives in
[`test/mini-grammar.ts`](test/mini-grammar.ts).


## Build and test

The `tabnas` engine is the only dependency and is consumed from source.
From the repository root, `make build` / `make test` fetch it into
`vendor/` and build/test both implementations. The tests bring their own
small grammar. See the [root README](../README.md) and
[`../AGENTS.md`](../AGENTS.md).


## License

MIT — see [LICENSE](LICENSE).
