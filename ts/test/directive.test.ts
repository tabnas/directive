/* Copyright (c) 2021-2025 Richard Rodger and other contributors, MIT License */

import { test, describe } from 'node:test'
import assert from 'node:assert'
import fs from 'node:fs'
import path from 'node:path'

import type { Rule } from 'tabnas'
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


// --- TSV spec loader ---
// Format:
//   <input>\t<expected-json>
//   <input>\t!error <regex>
// Blank lines and lines starting with # are ignored.

type SpecCase = { input: string; expected: string }

const loadSpec = (name: string): SpecCase[] => {
  const text = fs.readFileSync(
    path.join(__dirname, '..', '..', 'test', 'spec', name),
    'utf8',
  )
  const cases: SpecCase[] = []
  for (const raw of text.split('\n')) {
    const line = raw.replace(/\r$/, '')
    if (line.length === 0 || line.startsWith('#')) continue
    const i = line.indexOf('\t')
    if (i < 0) continue
    cases.push({ input: line.slice(0, i), expected: line.slice(i + 1) })
  }
  return cases
}

const runSpec = (j: { parse: (s: string) => any }, name: string) => {
  for (const { input, expected } of loadSpec(name)) {
    if (expected.startsWith('!error ')) {
      const pattern = new RegExp(expected.slice('!error '.length))
      assert.throws(
        () => j.parse(input),
        pattern,
        `input: ${JSON.stringify(input)}`,
      )
    } else {
      const want = JSON.parse(expected)
      assert.deepStrictEqual(
        normalize(j.parse(input)),
        normalize(want),
        `input: ${JSON.stringify(input)}`,
      )
    }
  }
}


describe('directive', () => {

  test('happy', () => {
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


  test('close', () => {
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


  test('adder', () => {
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


  test('inject', () => {
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
