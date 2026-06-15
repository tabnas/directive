/* Copyright (c) 2025 Richard Rodger, MIT License */

/*  test/mini-grammar.ts
 *
 *  A deliberately small grammar — just enough structure to exercise the
 *  directive plugin without depending on a full JSON / jsonic grammar.
 *  It defines scalar values, explicit lists `[a, b]`, and explicit maps
 *  `{k: v}` (unquoted keys), reusing the engine's default lexer (bare
 *  words, numbers, quoted strings).
 *
 *  The rule names (val, list, map, pair, elem) match the directive's
 *  default open/close rule targets, so a directive registers against
 *  this grammar exactly as it would against any host grammar. State
 *  actions are auto-wired by the `@<rule>-bo` / `@<rule>-bc` naming
 *  convention the engine's grammar() applies.
 */

import { Tabnas } from '@tabnas/parser'
import type { Plugin, Rule, Context } from '@tabnas/parser'

export const mini: Plugin = (tn: Tabnas) => {
  tn.grammar({
    ref: {
      '@val-bo': (r: Rule) => (r.node = undefined),
      '@val-bc': (r: Rule, ctx: Context) => {
        if (undefined !== r.node) return
        if (undefined !== r.child.node) {
          r.node = r.child.node
        } else if (0 !== r.os) {
          r.node = r.o0.resolveVal(r, ctx)
        }
      },
      '@map-bo': (r: Rule) => (r.node = {}),
      '@list-bo': (r: Rule) => (r.node = []),
      '@pairkey': (r: Rule) => {
        const t = r.o0
        r.u.key = undefined !== t.val ? t.val : t.src
      },
      '@pair-bc': (r: Rule) => {
        if (r.u.pair) r.node[r.u.key] = r.child.node
      },
      '@elem-bc': (r: Rule) => {
        if (undefined !== r.child.node) r.node.push(r.child.node)
      },

      // Implicit list: a standalone value followed by a comma starts a
      // bracketless list. Only fires outside an elem/pair/ilist position
      // (so explicit `[a, b]` and `{k: v}` are unaffected) and where
      // implicit lists are permitted (`n.dlist !== 1`, the counter the
      // directive plugin sets to 1 to suppress them).
      '@implicit?': (r: Rule) =>
        1 !== r.n.dlist &&
        'elem' !== r.parent.name &&
        'pair' !== r.parent.name &&
        'ilist' !== r.parent.name,
      // Seed the list with the already-parsed scalar value.
      '@ilist-seed': (r: Rule) => (r.node = [r.node]),
      // Push each subsequent element onto the inherited list node.
      '@ilist-push': (r: Rule) => {
        if (undefined !== r.child.node) r.node.push(r.child.node)
      },
    },

    rule: {
      val: {
        open: [
          { s: '#OB', p: 'map', b: 1 }, // a map: `{ …`
          { s: '#OS', p: 'list', b: 1 }, // a list: `[ …`
          { s: '#VAL' }, // a plain scalar
        ],
        close: [
          { s: '#ZZ' },
          // Implicit list: scalar then comma → `[scalar, …]`.
          { s: '#CA', b: 1, c: '@implicit?', r: 'ilist', a: '@ilist-seed' },
          { b: 1 },
        ],
      },

      // Bracketless list continuation. Created by replacing a val (so it
      // inherits the seeded `[first]` node) and then absorbs `, value`
      // pairs until something else closes it.
      ilist: {
        open: [{ s: '#CA', p: 'val' }], // consume comma, parse next value
        close: [
          { s: '#CA', b: 1, r: 'ilist', a: '@ilist-push' }, // more elements
          { b: 1, a: '@ilist-push' }, // final element
        ],
      },

      map: {
        open: [
          { s: '#OB #CB', b: 1, n: { pk: 0 } }, // empty map `{}`
          { s: '#OB', p: 'pair', n: { pk: 0 } },
        ],
        close: [{ s: '#CB' }],
      },

      list: {
        open: [
          { s: '#OS #CS', b: 1 }, // empty list `[]`
          { s: '#OS', p: 'elem' },
        ],
        close: [{ s: '#CS' }],
      },

      pair: {
        open: [{ s: '#KEY #CL', p: 'val', u: { pair: true }, a: '@pairkey' }],
        close: [
          { s: '#CA', r: 'pair' }, // next pair
          { s: '#CB', b: 1 }, // end of map
        ],
      },

      elem: {
        open: [{ p: 'val' }],
        close: [
          { s: '#CA', r: 'elem' }, // next element
          { s: '#CS', b: 1 }, // end of list
        ],
      },
    },
  })
}

// makeMini builds a parser with just the mini grammar installed.
export const makeMini = (): Tabnas => new Tabnas().use(mini)
