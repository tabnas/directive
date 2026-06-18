# Concepts: how the directive plugin works (TypeScript)

This is the *why* and *how*. You do not need it to use the plugin —
reach for it when you are debugging a grammar, building something
unusual with `custom`, or want to understand the engine relationship.
The [reference](reference.md) lists *what*; this explains the
mechanism.


## Three layers

```
your input ──▶ [ tabnas engine ] ──▶ value
                     ▲   ▲
              host grammar   directive plugin
```

- **tabnas** — the parser engine, and the plugin's only dependency. It
  ships *no* grammar: just a matcher-based lexer and a rule-based parser
  driven by grammar specs. The `Rule`, `Context`, `Tin`, `RuleSpec` and
  `StateAction` types are all tabnas types.
- **a host grammar** — any grammar installed onto a tabnas instance that
  defines the usual `val` / `list` / `map` / `pair` / `elem` rules. The
  directive layers onto it. This repo's tests use a deliberately small
  one (`ts/test/mini-grammar.ts`); a full relaxed-JSON grammar such as
  `@tabnas/json` is another.
- **directive** — this plugin. It extends the host grammar's rules to
  recognise directive tokens. It needs those rules to exist, which is
  why it operates on an instance with a grammar installed rather than a
  bare engine.


## What a directive is

A directive is a user-defined token sequence that the parser treats as
a call-out into custom logic. The plugin makes the parser:

- recognise an **open** token (and optionally a **close** token),
- push into a new parse rule while the body is parsed,
- fire an **action** when the body finishes,
- let the action assign or transform the resulting node.

Two shapes:

- **Open-only** — consumes a single value after the open token (`@foo`
  consumes `foo`).
- **Open + close** — consumes everything between the open and close
  tokens, including structured bodies (`sum<1, 2, 3>`).


## The rule model it plugs into

The parser is rule-based. Every parse step sits inside some rule —
`val`, `list`, `elem`, `map`, `pair`. Each rule has **alternates**:
ordered lists of token/condition specs that decide which branch to take
when the rule opens or closes.

The plugin weaves the directive into this model in three places:

1. **Open-rules** (default `val`) — get an extra open-alt that matches
   the directive's open token and *pushes* into a new rule named after
   the directive.
2. **Close-rules** (default `list, elem, map, pair`) — get an extra
   close-alt that matches the close token so they stop consuming
   siblings at the right place.
3. **Directive rule** — a brand-new rule for the directive itself, whose
   job is to parse one value (`p: 'val'`), optionally look for the close
   token, and fire the action.

All of this is expressed as a single declarative grammar spec applied
with one `tabnas.grammar(spec, setting)` call. The only imperative part
is the directive rule's `bo`/`bc` state hooks (`rs.clear().bo(...).bc(...)`),
because seeding `rule.node = {}` on open and calling the action on close
is state behaviour, not declarative structure.


## The `dr_<NAME>` counter

Close tokens are ambiguous: a `>` could close `foo<>`, close `bar<>`, or
be a syntax error. The plugin disambiguates with a rule counter. When a
directive opens it sets `n.dr_<NAME> = 1`. The close-alt's condition
only fires when that counter is `1`, so a stray `>` with no matching
open raises an `unexpected` error instead of being silently accepted.

The counter also scopes close recognition to the current parse frame.
When the directive rule resolves, the counter unwinds, and outer rules
again see the close token as untagged.


## Boundary closing

A close token closes more than the directive: it also terminates an
enclosing list or map opened **inside** the directive body. That is why
`foo<[1, 2>` parses — the `>` closes the open `[ ` list and the
directive together, no `]` needed. This works because the close-rules
(`list`, `elem`, `map`, `pair`) each gained a close-alt that fires on
the close token while `dr_<NAME>` is `1`, plus a `, CLOSE` variant for a
trailing comma (`foo<[1, 2,>`).


## Implicit lists and maps

Some host grammars support implicit containers: `1 2 3` is a list
without brackets, `a:1 b:2` a map without braces. (The small test
grammar supports implicit lists; `@tabnas/json` differs.) This
interacts badly with an *open-only* directive: once it pushes into
`val` it would keep consuming siblings forever.

The plugin guards with two counters set on the directive-rule push:

| `close` present? | `dlist`, `dmap` inside the body |
| ---------------- | ------------------------------- |
| yes              | reset to 0 — implicits allowed  |
| no               | raised to 1 — implicits suppressed |

With a close token, implicits are safe because the close bounds the
body. Without one, they are suppressed so the directive consumes exactly
one value. The host grammar reads these counters (e.g. the mini grammar
only starts an implicit list when `n.dlist !== 1`).


## Why one grammar spec tagged `g: 'directive'`

Every alt the plugin installs is tagged with the group `directive` in
addition to its per-alt tag (`start`, `end`, …), via the setting
`{ rule: { alt: { g: 'directive' } } }` passed to `grammar()`. The
engine appends `'directive'` to each alt's group list. This makes it
easy to identify plugin-added alternates, filter `@tabnas/debug` traces
to directive events, and reason about `custom` alts that should behave
predictably.


## Why shared close tokens reuse the existing fixed token

Two directives with the same close character (both `>`) must resolve to
the *same* engine Tin, so the lexer produces one token type. If each
registered its own `#CD_<NAME>`, the fixed-token table would keep only
one mapping and the other directive's close would never lex. So the
plugin checks the fixed-token table first: if the close character is
already registered it reuses that token; otherwise it registers a fresh
`#CD_<NAME>`.


## Why `custom` receives resolved tokens

`custom` runs last and is handed the resolved `OPEN` / `CLOSE` Tins so
you can install extra alternates that match the directive's tokens
without re-resolving them from names. Those Tins are only stable after
the plugin has finished its own wiring — hence the callback fires at the
end.


## Relationship to the engine

The plugin is not a fork or a patched engine. It is purely additive: it
uses public engine APIs (`options`, `rule`, `grammar`, `fixed`) to
register tokens and extend rule specs. Mix it freely with other plugins.
The one shared resource to watch is the fixed-token table — two plugins
that want the same character sequence as their open token will collide,
and the second registration throws.


## Design principles

- **Declarative first.** Rule modifications are one grammar spec;
  imperative `clear`/`bo`/`bc` calls are reserved for state hooks that
  cannot be expressed declaratively.
- **Language parity.** TypeScript is canonical; the Go port mirrors its
  option names, defaults, and the shared `test/spec/*.tsv` fixtures.
  Intentional differences are listed in the Go
  [concepts](../../go/doc/concepts.md#differences-from-the-ts-version).
- **Fail loudly.** Re-registering an open token throws rather than
  silently overwriting. A close token with no open is a parse error, not
  a wrong parse.
