# Tutorial: your first directive

In this tutorial you will build a parser that treats `@x` as a custom
directive which uppercases the following word. By the end you will have
a running program that parses `[@a, @b, 1]` and produces `['A', 'B', 1]`.

A directive is a plugin for the
[tabnas](https://github.com/tabnas/parser) parser engine. The engine
ships no grammar of its own, so you bring a **host grammar** — any
grammar that defines the usual `val` / `list` / `map` / `pair` rules —
and the directive layers onto it. In the snippets below `hostGrammar` is
that grammar plugin; a complete, minimal one (scalars, `[a, b]` lists and
`{k: v}` maps) lives in this repo at
[`ts/test/mini-grammar.ts`](../ts/test/mini-grammar.ts) /
[`go/mini_grammar_test.go`](../go/mini_grammar_test.go).

Choose your language and work through the steps in order. You do not
need to understand every line — the [Explanation](explanation.md)
covers the why.


## TypeScript

### 1. Add the dependencies

The directive plugin (`@tabnas/directive`) and the `tabnas` engine. See
the [README](../README.md) for how the engine is wired from source
during development.

```ts
import { Tabnas } from '@tabnas/parser'
import { Directive } from '@tabnas/directive'
```

### 2. Register the directive

Start from a bare engine, add your host grammar, then the directive:

```ts
const j = new Tabnas().use(hostGrammar).use(Directive, {
  name: 'upper',
  open: '@',
  action: (rule) => {
    rule.node = String(rule.child.node).toUpperCase()
  },
})
```

### 3. Parse some input

Call `parse` with a string:

```ts
console.log(j.parse('@hello'))         // HELLO
console.log(j.parse('[@a, @b, 1]'))    // [ 'A', 'B', 1 ]
console.log(j.parse('{x:@a, y:@b}'))   // { x: 'A', y: 'B' }
```

### 4. Add a close token

So far the directive consumes a single value. A *close* token lets it
wrap an arbitrary body. Replace the `.use(Directive, ...)` call with:

```ts
.use(Directive, {
  name: 'upper',
  open: 'U<',
  close: '>',
  action: (rule) => {
    rule.node = String(rule.child.node).toUpperCase()
  },
})
```

Try it:

```ts
console.log(j.parse('U<hello world>'))    // HELLO WORLD
console.log(j.parse('[U<a>, U<b>, 1]'))   // [ 'A', 'B', 1 ]
```

You have now built a directive with both forms: open-only (consumes
one value) and open+close (consumes everything up to the close token).


## Go

You import two packages: the engine (`tabnas`, for the `Rule`/`Context`
types your action uses) and the directive plugin. Your host grammar is
applied with `j.Use` before the directive.

### 1. Create a file `upper.go`

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
    j.Use(hostGrammar) // provides val / list / map / pair
    // Apply returns an error (a duplicate open token, a grammar build
    // failure); the plugin never panics.
    if _, err := directive.Apply(j, directive.DirectiveOptions{
        Name: "upper",
        Open: "@",
        Action: func(r *tabnas.Rule, _ *tabnas.Context) {
            r.Node = strings.ToUpper(fmt.Sprintf("%v", r.Child.Node))
        },
    }); err != nil {
        fmt.Println("directive setup failed:", err)
        return
    }

    for _, src := range []string{"@hello", "[@a, @b, 1]", "{x:@a, y:@b}"} {
        v, _ := j.Parse(src)
        fmt.Printf("%s → %#v\n", src, v)
    }
}
```

### 2. Run it

```sh
go run upper.go
```

You should see:

```
@hello → "HELLO"
[@a, @b, 1] → []interface {}{"A", "B", 1}
{x:@a, y:@b} → map[string]interface {}{"x":"A", "y":"B"}
```

### 3. Add a close token

Replace the `directive.Apply(...)` options with (error handling
elided here for brevity):

```go
directive.Apply(j, directive.DirectiveOptions{
    Name:  "upper",
    Open:  "U<",
    Close: ">",
    Action: func(r *tabnas.Rule, _ *tabnas.Context) {
        r.Node = strings.ToUpper(fmt.Sprintf("%v", r.Child.Node))
    },
})
```

Try:

```go
v, _ := j.Parse("U<hello world>")  // "HELLO WORLD"
```

You have now built a directive with both forms: open-only and open+close.


## Next steps

- Follow a [How-to guide](how-to.md) for a specific recipe.
- Read the full [Reference](reference.md) for every option.
- Read the [Explanation](explanation.md) to understand the grammar model.
