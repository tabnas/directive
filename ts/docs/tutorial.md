# Tutorial: your first directive

In this tutorial you will build a Jsonic parser that treats `@x` as a
custom directive which uppercases the following word. By the end you
will have a running program that parses `[@a, @b, 1]` and prints
`['A', 'B', 1]`.

Choose your language and work through the steps in order. You do not
need to understand every line — the [Explanation](explanation.md)
covers the why.


## TypeScript

### 1. Install

```sh
npm install jsonic @jsonic/directive
```

### 2. Create a file `upper.ts`

```ts
import { Jsonic } from 'jsonic'
import { Directive } from '@jsonic/directive'

const j = Jsonic.make().use(Directive, {
  name: 'upper',
  open: '@',
  action: (rule) => {
    rule.node = String(rule.child.node).toUpperCase()
  },
})
```

### 3. Parse some input

Append to `upper.ts`:

```ts
console.log(j('@hello'))
console.log(j('[@a, @b, 1]'))
console.log(j('{x:@a, y:@b}'))
```

### 4. Run it

```sh
npx ts-node upper.ts
```

You should see:

```
HELLO
[ 'A', 'B', 1 ]
{ x: 'A', y: 'B' }
```

### 5. Add a close token

Replace the `.use(Directive, ...)` call with:

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
console.log(j('U<hello world>'))         // HELLO WORLD
console.log(j('[U<a>, U<b>, 1]'))        // [ 'A', 'B', 1 ]
```

You have now built a directive with both forms: open-only (consumes
one value) and open+close (consumes everything up to the close token).


## Go

### 1. Install

```sh
go get github.com/jsonicjs/jsonic/go
go get github.com/jsonicjs/directive/go
```

### 2. Create a file `upper.go`

```go
package main

import (
    "fmt"
    "strings"

    jsonic "github.com/jsonicjs/jsonic/go"
    directive "github.com/jsonicjs/directive/go"
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

    for _, src := range []string{"@hello", "[@a, @b, 1]", "{x:@a, y:@b}"} {
        v, _ := j.Parse(src)
        fmt.Printf("%s → %#v\n", src, v)
    }
}
```

### 3. Run it

```sh
go run upper.go
```

You should see:

```
@hello → "HELLO"
[@a, @b, 1] → []interface {}{"A", "B", 1}
{x:@a, y:@b} → map[string]interface {}{"x":"A", "y":"B"}
```

### 4. Add a close token

Replace the `directive.Apply(...)` call with:

```go
directive.Apply(j, directive.DirectiveOptions{
    Name:  "upper",
    Open:  "U<",
    Close: ">",
    Action: func(r *jsonic.Rule, _ *jsonic.Context) {
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
