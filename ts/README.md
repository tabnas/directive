# @jsonic/directive — directive syntax for Jsonic

Adds directive syntax to the [Jsonic](https://jsonic.senecajs.org) JSON
parser. A directive is a token sequence (e.g. `@name`, `add<1,2>`) that
triggers custom parsing behaviour. TypeScript and Go ports share the
same API shape and test specs.

[![npm version](https://img.shields.io/npm/v/@jsonic/directive.svg)](https://npmjs.com/package/@jsonic/directive)
[![build](https://github.com/jsonicjs/directive/actions/workflows/build.yml/badge.svg)](https://github.com/jsonicjs/directive/actions/workflows/build.yml)


## Documentation

This project's documentation follows the [Diátaxis](https://diataxis.fr)
framework. Each section has one job — pick the one that matches what
you're trying to do.

| If you want to…                                | Read                                     |
| ---------------------------------------------- | ---------------------------------------- |
| Build your first directive step-by-step        | [Tutorial](docs/tutorial.md)             |
| Solve a specific problem                       | [How-to guides](docs/how-to.md)          |
| Look up an option, type, or default            | [Reference](docs/reference.md)           |
| Understand how the plugin works internally     | [Explanation](docs/explanation.md)       |


## Quickstart

### TypeScript

```ts
import { Jsonic } from 'jsonic'
import { Directive } from '@jsonic/directive'

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
    jsonic "github.com/jsonicjs/jsonic/go"
    directive "github.com/jsonicjs/directive/go"
)

j := jsonic.Make()
directive.Apply(j, directive.DirectiveOptions{
    Name: "upper",
    Open: "@",
    Action: func(r *jsonic.Rule, _ *jsonic.Context) {
        r.Node = strings.ToUpper(fmt.Sprintf("%v", r.Child.Node))
    },
})

j.Parse("[@a, @b, 1]") // → []any{"A", "B", float64(1)}
```


## License

MIT — see [LICENSE](LICENSE).
