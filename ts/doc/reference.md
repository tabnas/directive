# Reference (TypeScript)

The exact public surface of `@tabnas/directive`. For orientation see the
[tutorial](tutorial.md); for tasks see the [how-to guides](guide.md).

The plugin's only dependency is the
[tabnas](https://github.com/tabnas/parser) engine; its types (`Rule`,
`Context`, `Tin`, `RuleSpec`, `StateAction`, `Token`, `Plugin`) come from
there. The plugin modifies host-grammar rules (`val` / `list` / `map` /
`pair`), so it is always applied to an instance that already has a
grammar installed.


## Exports

```ts
import { Directive } from '@tabnas/directive'
import type { DirectiveOptions } from '@tabnas/directive'
```

| Export             | Kind   | Description                                  |
| ------------------ | ------ | -------------------------------------------- |
| `Directive`        | value  | The plugin. Register with `j.use(Directive, options)`. |
| `DirectiveOptions` | type   | The options object accepted by the plugin.   |

`Directive.defaults` holds the default `rules` (see [Defaults](#defaults)).


## `Directive` (plugin)

```ts
j.use(Directive, options: DirectiveOptions)
```

A standard tabnas `Plugin`. On registration it:

1. Registers the open fixed token `#OD_<name>` (throws if `open` is
   already a fixed token).
2. Registers the close fixed token `#CD_<name>` if `close` is set and
   not already a fixed token.
3. Clears and re-seeds a rule named `name` (the directive rule).
4. Installs open/close alternates on the configured host rules and on
   the directive rule via a single `grammar()` call, every alt tagged
   with the group `directive`.
5. Invokes `custom` (if given) with the resolved tokens.


## `DirectiveOptions`

```ts
type DirectiveOptions = {
  name: string
  open: string
  action: StateAction | string
  close?: string
  rules?: {
    open?:  string | string[] | Record<string, { c?: Function }>
    close?: string | string[] | Record<string, { c?: Function }>
  }
  custom?: (
    tabnas: Tabnas,
    config: { OPEN: Tin; CLOSE: Tin | null | undefined; name: string },
  ) => void
}
```

| Field    | Type                                  | Required | Description |
| -------- | ------------------------------------- | -------- | ----------- |
| `name`   | `string`                              | yes      | Directive name. Also the parse-rule name and the token-name suffix (`#OD_<name>`, `#CD_<name>`). |
| `open`   | `string`                              | yes      | Character sequence that starts the directive. Must be unique per instance. |
| `action` | `StateAction \| string`               | yes      | Fired when the directive body finishes. A function transforms the result; a string is a dotted path resolved on the instance options at fire time. See [action](#action). |
| `close`  | `string`                              | no       | Character sequence that ends the directive. Omit it for the open-only shape (consumes one value). |
| `rules`  | `RulesOption \| null`                 | no       | Which host rules detect the directive. Omit for [defaults](#defaults); `null` modifies no rules. See [rules](#rules). |
| `custom` | `(tabnas, config) => void`            | no       | Callback run after setup. `config = { OPEN, CLOSE, name }` (resolved Tins; `CLOSE` is `null` when no close token). |


## `action`

```ts
type StateAction = (
  rule: Rule,
  ctx: Context,
  next: Rule,
  tkn?: Token | void,
) => any
```

Called from the directive rule's `bc` (before-close) hook once the body
has parsed.

| Arg    | Type      | Notes |
| ------ | --------- | ----- |
| `rule` | `Rule`    | The directive rule. Its `rule.child.node` holds the parsed body value (subject to how the host grammar propagates nodes). |
| `ctx`  | `Context` | The parse context. |
| `next` | `Rule`    | The rule scheduled next. |
| `tkn`  | `Token?`  | The matching token. |

Behaviour:

- Assign `rule.node` to set the directive's result value.
- If `rule.parent` is a `pair`, you may instead mutate
  `rule.parent.node` (e.g. `Object.assign`) and leave `rule.node`
  unset — useful for directives that expand into map entries.
- Returning a `Token` (something with `.isToken`) overrides the next
  token; the close hook propagates it. Advanced.

**String form.** When `action` is a string, the plugin builds the
action for you:

```ts
action = (rule) => (rule.node = tabnas.util.prop(tabnas.options, path))
```

i.e. it reads the dotted `path` from the live instance options every
time the directive fires, and assigns it as the result.


## `rules`

```ts
type RulesOption = {
  open?:  string | string[] | Record<string, { c?: Function }>
  close?: string | string[] | Record<string, { c?: Function }>
}
```

- **String / string-array form** — split on commas (and surrounding
  whitespace) and treated as a set of rule names with no condition.
  `'val, pair'` and `['val', 'pair']` are equivalent.
- **Record form** — each key is a host rule name; the value may carry a
  per-rule condition `c: (rule, ctx) => boolean`. The directive only
  matches inside that rule when `c` returns truthy.
- **`null`** for the whole `rules` option — modify no host rules.

`rules.open` rules get alternates that detect the open token and push
into the directive rule. `rules.close` rules (only meaningful when
`close` is set) get alternates that detect the close token so they stop
consuming siblings at the directive boundary.


## Defaults

`Directive.defaults.rules`:

| Direction | Default rule names    |
| --------- | --------------------- |
| `open`    | `val`                 |
| `close`   | `list,elem,map,pair`  |

So by default the open token is recognised in any value position, and
the close token ends sibling parsing in list / element / map / pair
rules.


## Tokens

For a directive named `NAME`:

| Token name   | When                                                       | Value   |
| ------------ | ---------------------------------------------------------- | ------- |
| `#OD_<NAME>` | always                                                     | `open`  |
| `#CD_<NAME>` | only if `close` is set **and** not already a fixed token   | `close` |

When `close` collides with an existing fixed token (e.g. a close
character shared across directives) the existing token is reused and no
new `#CD_<NAME>` is created. Access a registered token via
`j.token.OD_<NAME>`.


## Group tags

Every alternate the plugin installs carries `directive` as one of its
`g` tags. The additional per-alt tags are:

| Context                | Tag(s)        |
| ---------------------- | ------------- |
| Open-rule OPEN alt     | `start`       |
| Open-rule OPEN+CLOSE   | `start,end`   |
| Close-rule CLOSE       | `end`         |
| Close-rule `,CLOSE`    | `end,comma`   |

This lets tools (such as `@tabnas/debug`) filter to directive-related
alternates and traces.


## Counters

While inside a directive, `rule.n['dr_<NAME>'] === 1`. The close-rule
alternates only fire when this counter is `1`, which is how a stray
close token without a matching open becomes an `unexpected` error rather
than a wrong parse.

`dlist` and `dmap` control whether implicit (bracketless) lists / maps
are permitted inside the directive body:

| `close` present? | `dlist` / `dmap` inside the body |
| ---------------- | -------------------------------- |
| yes              | 0 — implicits allowed (the close bounds the body) |
| no               | 1 — implicits suppressed (so the directive consumes exactly one value and does not eat trailing siblings) |


## Errors

| Situation                                  | Behaviour              |
| ------------------------------------------ | ---------------------- |
| `open` is already a fixed token            | `throw new Error('Directive open token already in use: ' + open)` |
| Grammar build failure during registration  | the engine throws      |
| Parsing a close token with no matching open | engine `unexpected` error (`err.code === 'unexpected'`) |
| Open token in a value position with no following value | engine `unexpected` error |


## Spec file format (`test/spec/*.tsv`)

```
# comments start with '#'
# blank lines are ignored
<input><TAB><expected-json>
<input><TAB>!error <regex>
```

Loaded by both the TypeScript and Go test suites.
