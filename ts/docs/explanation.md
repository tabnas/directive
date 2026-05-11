# Explanation: how the directive plugin works

This document explains the design and internals of the plugin. It
complements the [Reference](reference.md) (which lists *what*) with
the *why* and *how*. You do not need to read it to use the plugin;
read it when you want to debug a failing grammar or build something
unusual with `custom`.


## What is a directive?

A directive is a user-defined token sequence that Jsonic treats as a
call-out into custom logic. The plugin makes Jsonic recognise:

- an **open** token (and optionally a **close** token),
- push into a new parse rule while the body is parsed,
- fire an **action** when the body finishes,
- let the action assign or transform the resulting node.

Two shapes are supported:

- **Open-only** — the directive consumes a single value after the
  open token. Example: `@foo` consumes `foo`.
- **Open + close** — the directive consumes everything between the
  open and close tokens, including structured bodies. Example:
  `sum<1, 2, 3>`.


## The rule model

Jsonic is rule-based. Every parse step sits inside some rule — `val`,
`list`, `elem`, `map`, `pair`. Each rule has **alts** (alternatives):
ordered lists of tokens / conditions that determine which branch to
take when opening or closing the rule.

The plugin weaves the directive into this model in three places:

1. **Open-rules** — existing rules (default: `val`) get an extra
   open-alt that matches the directive's open token and *pushes*
   into a new rule named after the directive.
2. **Close-rules** — existing rules (default: `list, elem, map,
   pair`) get an extra close-alt that matches the close token so
   they stop consuming siblings at the right place.
3. **Directive rule** — a brand-new rule is created for the directive
   itself, whose only job is to parse one value (`p: 'val'`),
   optionally look for the close token, and call the action.


## The counter `dr_<NAME>`

Close tokens are ambiguous: `>` could be a `foo<>` closer, a `bar<>`
closer, or a syntax error. The plugin disambiguates using a rule
counter: when a directive opens, it sets `n.dr_<NAME> = 1`. The
close-alt only fires when the counter is `1`, so a stray `>` that
doesn't have a matching open raises an "unexpected" error instead.

The counter also scopes directive-close recognition to the current
parse frame. When the directive rule resolves, the counter
decrements, and outer rules see the close token as untagged again.


## Implicit lists and maps

Jsonic supports implicit containers: `1 2 3` parses as a list
without brackets, and `a:1 b:2` as a map without braces. This
interacts badly with an open-only directive, because once the
directive pushes into `val` it would keep consuming siblings forever.

The plugin guards against this with two counters:

| `close` present? | `dlist`, `dmap` set inside the directive |
| ---------------- | ---------------------------------------- |
| yes              | reset to 0 — implicits are allowed       |
| no               | raised to 1 — implicits are suppressed   |

With a close token, implicits are safe because the close bounds the
directive body. Without one, implicits are suppressed so the
directive consumes exactly one value.


## Why the plugin uses `jsonic.grammar()` with `g: 'directive'`

Every alt the plugin installs is tagged with the group `directive`
in addition to its per-alt tag (`start`, `end`, etc). This is done
by passing a single `grammar()` spec plus the setting:

```
{ rule: { alt: { g: 'directive' } } }
```

Jsonic appends `'directive'` to every alt's group list. This makes
it easy to:

- Identify plugin-added alts when inspecting a rule spec.
- Filter traces via `Debug` to only directive-related events.
- Write custom alts in a `custom` callback that interact predictably.


## Why shared close tokens reuse the existing fixed token

Two directives with the same close character (e.g. both using `>`)
must resolve to the same Jsonic Tin so the lexer produces a single
token type. If each directive registered its own `#CD_<NAME>`, the
lexer's fixed-token table would only keep one mapping and the other
directive would never see its close.

The plugin therefore checks `cfg.FixedTokens[close]` before
registering. If the token already exists it is reused; otherwise a
fresh `#CD_<NAME>` is registered. The grammar spec references the
token by the name it was actually registered under (retrieved via
`j.TinName`).


## Why `custom` receives resolved tokens

The `custom` callback gives you the resolved `OPEN` / `CLOSE` Tins
so you can install additional alts that match the directive's
tokens without re-resolving them from names. Those Tins are only
stable after the plugin has finished its own wiring — hence the
callback fires last.


## Relationship to Jsonic itself

The plugin is not a fork or a modified Jsonic. It is purely
additive: it uses public Jsonic APIs (`options`, `rule`, `grammar`,
`fixed`) to register tokens and extend rule specs. You can mix it
freely with other Jsonic plugins. The one interaction point to be
aware of is the fixed-token table — any two plugins that want the
same character sequence as their open token will collide.


## Design principles

- **Declarative first.** Rule modifications are expressed as a
  single `GrammarSpec`; imperative `rs.clear()` / `bo` / `bc` calls
  are reserved for state-action hooks that aren't expressible
  declaratively.
- **Language parity.** TypeScript and Go ports share option names,
  default behaviour, and test specs (`test/spec/*.tsv`). Behaviour
  that is inherently language-specific (NaN, null-prototype
  objects, error wording) lives in language-specific tests.
- **Fail loudly.** Re-registering an open token throws / panics
  rather than silently overwriting. A close token without its open
  produces a parse error, not a wrong parse.
