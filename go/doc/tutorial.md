# Tutorial: your first directive (Go)

This tutorial takes you from nothing to a working parser that
understands a custom *directive*. It is the Go port of the
[TypeScript tutorial](../../ts/doc/tutorial.md); the TypeScript
implementation is canonical and this package (`tabnasdirective`) tracks
it.

A directive is a plugin for the
[tabnas](https://github.com/tabnas/parser) parser engine. The engine
ships **no grammar of its own**, so a directive cannot stand alone: you
bring a **host grammar** (one that defines the usual `val` / `list` /
`map` / `pair` rules) and the directive layers onto it. You apply it to
a `*tabnas.Tabnas` instance that already has that grammar installed.

Below, `hostGrammar` stands for your host grammar plugin. The repo's own
test host (scalars, `[a, b]` lists, `{k: v}` maps) lives in
[`mini_grammar_test.go`](../mini_grammar_test.go).


## 1. Install

```bash
go get github.com/tabnas/parser/go
go get github.com/tabnas/directive/go
```

You import two packages: the engine `tabnas` (for the `Rule` / `Context`
types your action uses) and the directive plugin
`tabnasdirective`.


## 2. Register an open-only directive

The simplest directive has just an **open** token; it consumes the one
value that follows. Here `@` uppercases the next value.

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
	j.Use(hostGrammar) // provides val / list / map / pair

	// Apply returns (instance, error). The plugin never panics — a
	// duplicate open token or a grammar build failure comes back as err.
	if _, err := tabnasdirective.Apply(j, tabnasdirective.DirectiveOptions{
		Name: "upper",
		Open: "@",
		Action: func(r *tabnas.Rule, _ *tabnas.Context) {
			r.Node = strings.ToUpper(fmt.Sprintf("%v", r.Child.Node))
		},
	}); err != nil {
		fmt.Println("directive setup failed:", err)
		return
	}

	v, _ := j.Parse("@hello")
	fmt.Printf("%#v\n", v) // "HELLO"
}
```

What each option does:

- `Name` — the directive's name. The plugin creates a parse rule with
  this name and uses it as a token-name suffix.
- `Open` — the character sequence that triggers the directive.
- `Action` — a callback run once the body has parsed. The body's value
  is `r.Child.Node`; assign `r.Node` to set the result.


## 3. Use it inside structures

The open token is wired (by default) into the host's `val` rule, so the
directive works anywhere a value is allowed:

```go
for _, src := range []string{"@hello", "[@a, @b, 1]", "{x:@a, y:@b}"} {
	v, _ := j.Parse(src)
	fmt.Printf("%s -> %#v\n", src, v)
}
// @hello      -> "HELLO"
// [@a, @b, 1] -> []interface {}{"A", "B", 1}
// {x:@a, y:@b}-> map[string]interface {}{"x":"A", "y":"B"}
```

(The bare-word and unquoted-key syntax above is what the repo's mini
host grammar accepts; a strict-JSON host would require quotes.)


## 4. Add a close token

An open-only directive grabs one value. To wrap an arbitrary body, add a
**close** token; the directive then consumes everything between open and
close. This `sum<...>` directive sums the numbers in its list body:

```go
j := tabnas.Make()
j.Use(hostGrammar)
tabnasdirective.Apply(j, tabnasdirective.DirectiveOptions{
	Name:  "sum",
	Open:  "sum<",
	Close: ">",
	Action: func(r *tabnas.Rule, _ *tabnas.Context) {
		out := float64(0)
		if arr, ok := r.Child.Node.([]any); ok {
			for _, v := range arr {
				if n, ok := v.(float64); ok {
					out += n
				}
			}
		}
		r.Node = out
	},
})

v, _ := j.Parse("sum<[1, 2, 3]>")
fmt.Printf("%#v\n", v) // float64(6)
```


## 5. Boundary closing

A close token also terminates a list or map opened **inside** the
directive — you do not have to close the inner bracket first:

```go
v, _ := j.Parse("sum<[1, 2>") // note: no ']' before '>'
fmt.Printf("%#v\n", v)        // float64(3)
```

The `>` closes both the open list and the directive at once. See the
[concepts](concepts.md) doc for why.


## Where to go next

- [How-to guides](guide.md) — focused recipes.
- [Reference](reference.md) — every option, type and counter.
- [Concepts](concepts.md) — the engine relationship, the design
  trade-offs, and the differences from the TypeScript version.
