/* Copyright (c) 2021-2025 Richard Rodger and other contributors, MIT License */

import { test, describe } from 'node:test'
import assert from 'node:assert'
import path from 'node:path'

import type { Rule } from '@tabnas/parser'
import { findSpecDir, makeRunner } from '@tabnas/support'
import { Directive } from '../dist/directive'
import { makeMini } from './mini-grammar'


// Normalize null-prototype objects so deepStrictEqual's prototype check
// doesn't spuriously fail.
const normalize = (v: any): any => {
  if (v === null || typeof v !== 'object') return v
  if (Array.isArray(v)) return v.map(normalize)
  const out: Record<string, any> = {}
  for (const k of Object.keys(v)) out[k] = normalize(v[k])
  return out
}
const expect = (actual: any) => ({
  equal: (expected: any) =>
    assert.deepStrictEqual(normalize(actual), normalize(expected)),
  exist: () => assert.ok(actual != null),
  throws: (matcher: RegExp) => assert.throws(actual, matcher),
})


// --- TSV spec runner ---
//
// The fixtures live at the repo root in `test/spec/*.tsv` and are read by
// @tabnas/support, whose Go half `go/directive_test.go` uses to run the
// SAME files — so the two implementations cannot drift without one going
// red, and neither can the two loaders.
//
// A row is `<input>\t<expected-json>`, or `<input>\tERROR:<code>` for
// input that must be rejected. These files have no header line and no
// `opts` column: what varies per case is the DIRECTIVE, and a directive is
// a function, so each test builds its own parser and hands it here.

const SPEC = findSpecDir(__dirname)

const runSpec = (j: { parse: (s: string) => any }, name: string) =>
  makeRunner({
    parse: (input) => j.parse(input),

    // `header: false` — the first line of these fixtures is a comment, not
    // a header, and the columns are positional.
    spec: { header: false },
  }).file(path.join(SPEC, name))


describe('directive', () => {

  describe('happy', () => {
    const j = makeMini().use(Directive, {
      name: 'upper',
      open: '@',
      action: (rule: Rule) =>
        (rule.node = ('' + rule.child.node).toUpperCase()),
    })

    expect(j.token.OD_upper).exist()
    expect(j.rule('upper')).exist()

    runSpec(j, 'happy.tsv')
  })


  describe('close', () => {
    const j = makeMini().use(Directive, {
      name: 'foo',
      open: 'foo<',
      close: '>',
      action: (rule: Rule) => (rule.node = 'FOO'),
    })

    runSpec(j, 'close-foo.tsv')

    // The close token also terminates an enclosing list/map opened inside
    // the directive (boundary closing).
    runSpec(j, 'close-boundary.tsv')

    // A second directive sharing the same close token ">".
    const k = j.use(Directive, {
      name: 'bar',
      open: 'bar<',
      close: '>',
      action: (rule: Rule) => (rule.node = 'BAR'),
    })

    runSpec(k, 'close-foo-bar.tsv')

    // Re-registering the same open token must throw.
    expect(() =>
      j.use(Directive, {
        name: 'baz',
        open: 'bar<',
        action: () => null,
      }),
    ).throws(/bar</)
  })


  describe('adder', () => {
    const j = makeMini().use(Directive, {
      name: 'adder',
      open: 'add<',
      close: '>',
      action: (rule: Rule) => {
        let out = 0
        if (Array.isArray(rule.child.node)) {
          out = rule.child.node.reduce((a: any, v: any) => a + v, 0)
        }
        rule.node = out
      },
    })

    runSpec(j, 'adder.tsv')

    // Implicit (bracketless) list bodies, e.g. add<1, 2, 3>.
    runSpec(j, 'implicit.tsv')

    const k = j.use(Directive, {
      name: 'multiplier',
      open: 'mul<',
      close: '>',
      action: (rule: Rule) => {
        let out = 0
        if (Array.isArray(rule.child.node) && rule.child.node.length > 0) {
          out = rule.child.node.reduce((a: any, v: any) => a * v, 1)
        }
        rule.node = out
      },
    })

    runSpec(k, 'multiplier.tsv')

    // Original adder still works after the second registration.
    runSpec(j, 'adder.tsv')
  })


  describe('inject', () => {
    const SRC: any = { a: 'A', b: { b: 1 }, c: [2, 3] }

    const j = makeMini().use(Directive, {
      name: 'inject',
      open: '@',
      rules: { open: 'val,pair' },
      action: (rule: Rule) => {
        const key = '' + rule.child.node
        const val = key in SRC ? SRC[key] : null
        if ('pair' === rule.parent.name) {
          Object.assign(rule.parent.node, val)
        } else {
          rule.node = val
        }
      },
    })

    runSpec(j, 'inject.tsv')
  })


  test('edges', () => {
    // rules:null modifies no host rules, so the open token is unrecognised.
    const j = makeMini().use(Directive, {
      name: 'none',
      open: '@',
      action: () => null,
      rules: null,
    })
    expect(() => j.parse('[@a]')).throws(/unexpected/)
  })


  test('rules-defaults-merge', () => {
    // The plugin defaults (open:'val', close:'list,elem,map,pair') are
    // deep-merged into a partial `rules` by the engine's plugin-defaults
    // mechanism, so an omitted direction keeps its default. Only an
    // explicit `rules: null` disables rule modification (see 'edges').
    // This is the TS-canonical semantics; Go treats a non-nil
    // *RulesOption as a complete override (docs/reference.md).
    const mk = (rules?: any) => {
      const opts: any = {
        name: 'mrg',
        open: 'mrg<',
        close: '>',
        action: (rule: Rule) => (rule.node = 'MRG'),
      }
      if (undefined !== rules) opts.rules = rules
      return makeMini().use(Directive, opts)
    }

    // Baseline: an absent `rules` uses both defaults. The close-direction
    // default is what lets '>' terminate a list opened inside the directive.
    expect(mk().parse('[mrg<1>]')).equal(['MRG'])
    expect(mk().parse('mrg<[1, 2>')).equal('MRG')

    // An empty `rules` object is NOT "modify no rules": it merges with the
    // defaults, so it behaves exactly like an absent `rules`.
    expect(mk({}).parse('[mrg<1>]')).equal(['MRG'])
    expect(mk({}).parse('mrg<[1, 2>')).equal('MRG')

    // A partial `rules` keeps the default of the direction it omits:
    // supplying only `open` retains the close-rule defaults, so the
    // boundary close still works.
    expect(mk({ open: 'val' }).parse('[mrg<1>]')).equal(['MRG'])
    expect(mk({ open: 'val' }).parse('mrg<[1, 2>')).equal('MRG')
  })


  test('action-option-prop', () => {
    // A string action resolves a dotted path on the instance options.
    const j = makeMini().use(Directive, {
      name: 'constant',
      open: '@',
      action: 'custom.x',
    })
    j.options({ custom: { x: 11 } })
    expect(j.parse('@y')).equal(11)
  })


  test('rules-object-form', () => {
    // rules.open / rules.close given as objects (Record form) carry a
    // per-alt `c` condition function for each modified host rule.
    let openC = 0
    let closeC = 0
    const j = makeMini().use(Directive, {
      name: 'cov',
      open: 'cov<',
      close: '>',
      action: (rule: Rule) => (rule.node = 'COV'),
      rules: {
        open: { val: { c: () => (openC++, true) } },
        close: {
          list: {},
          elem: { c: () => (closeC++, true) },
          map: {},
          pair: {},
        },
      },
    })

    expect(j.parse('cov<[1, 2>')).equal('COV')
    assert.ok(openC > 0, 'open condition ran')
    assert.ok(closeC > 0, 'close condition ran')
  })


  test('action-returns-token', () => {
    // When the action returns a token, the close hook propagates it.
    const j = makeMini().use(Directive, {
      name: 'tok',
      open: 'tok<',
      close: '>',
      action: (_rule: Rule, ctx: any) => ctx.t0,
    })

    // The action returns a token rather than setting rule.node, so the
    // directive node is the empty object seeded by the open hook.
    expect(j.parse('tok<a>')).equal({})
  })


  test('custom-callback', () => {
    // The `custom` option is invoked with the resolved token config.
    let seen: any = null
    makeMini().use(Directive, {
      name: 'cust',
      open: '@@',
      close: '>',
      action: () => null,
      custom: (_tabnas: any, config: any) => {
        seen = config
      },
    })

    assert.equal(seen?.name, 'cust')
    assert.ok(seen?.OPEN != null, 'OPEN tin resolved')
    assert.ok(seen?.CLOSE != null, 'CLOSE tin resolved')
  })

})
