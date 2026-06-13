/* Copyright (c) 2021-2025 Richard Rodger and other contributors, MIT License */

import { test, describe } from 'node:test'
import assert from 'node:assert'
import fs from 'node:fs'
import path from 'node:path'

import { Jsonic, Rule } from 'jsonic'
import { Directive } from '../dist/directive'


// The parser produces null-prototype objects; normalize both sides so
// deepStrictEqual's prototype check doesn't spuriously fail.
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

const runSpec = (j: (s: string) => any, name: string) => {
  for (const { input, expected } of loadSpec(name)) {
    if (expected.startsWith('!error ')) {
      const pattern = new RegExp(expected.slice('!error '.length))
      assert.throws(() => j(input), pattern, `input: ${JSON.stringify(input)}`)
    } else {
      const want = JSON.parse(expected)
      assert.deepStrictEqual(
        normalize(j(input)),
        normalize(want),
        `input: ${JSON.stringify(input)}`,
      )
    }
  }
}


describe('directive', () => {

  test('happy', () => {
    const j = Jsonic.make()
      .use(Directive, {
        name: 'upper',
        open: '@',
        action: (rule: Rule) =>
          (rule.node = ('' + rule.child.node).toUpperCase()),
      })

    expect(j.token.OD_upper).exist()
    expect(j.rule('upper')).exist()

    runSpec(j, 'happy.tsv')

    // Implicit-list special: pair after directive is silently dropped from
    // the array but still accessible as a property on the result.
    const clone = (x: any) => JSON.parse(JSON.stringify(x))
    expect(clone(j('1, @a, b:2'))).equal([1, 'A'])
    expect(j('1, @a, b:2').b).equal(2)
  })


  test('subobj', () => {
    const { deep } = Jsonic.util

    const j = Jsonic.make()
      .use(Directive, {
        name: 'subobj',
        open: '@',
        rules: {
          open: {
            val: {},
            pair: { c: (r: Rule) => r.lte('pk') },
          },
        },
        action: (rule: Rule) => {
          const key = rule.child.node
          const res = { [key]: ('' + key).toUpperCase() }
          rule.parent.parent.node = deep(rule.parent.parent.node, res)
          return undefined
        },
        custom: (jsonic: Jsonic, { OPEN, name }: any) => {
          // Handle special case of @foo first token — assume a map.
          jsonic.rule('val', (rs) => {
            rs.open([
              {
                s: [OPEN],
                c: (r) => 0 < r.n.pk,
                b: 1,
                g: name + '-undive',
              },
              {
                s: [OPEN],
                c: (r) => 0 === r.d,
                p: 'map',
                b: 1,
                n: { [name + '_top']: 1 },
                g: name + '-top',
              },
            ])
          })
          jsonic.rule('map', (rs) => {
            rs.open({
              s: [OPEN],
              c: (r) => 1 === r.d && 1 === r.n[name + '_top'],
              p: 'pair',
              b: 1,
              g: name + '-top',
            }).close({
              s: [OPEN],
              c: (r) => 0 < r.n.pk,
              b: 1,
              g: name + '-undive',
            })
          })
          jsonic.rule('pair', (rs) => {
            rs.close({
              s: [OPEN],
              c: (r) => 0 < r.n.pk,
              b: 1,
              g: name + '-undive',
            })
          })
        },
      })

    runSpec(j, 'subobj.tsv')
  })


  test('close', () => {
    const j = Jsonic.make().use(Directive, {
      name: 'foo',
      open: 'foo<',
      close: '>',
      action: (rule: Rule) => (rule.node = 'FOO'),
    })

    runSpec(j, 'close-foo.tsv')

    // Trailing-comma-before-close acceptance (TS-only meta option).
    expect(j('a:foo<y:2,>', { xlog: -1 })).equal({ a: 'FOO' })

    const k = j.use(Directive, {
      name: 'bar',
      open: 'bar<',
      close: '>',
      action: (rule: Rule) => (rule.node = 'BAR'),
    })

    runSpec(k, 'close-foo-bar.tsv')

    // Re-registering the same open token should error.
    expect(() =>
      j.use(Directive, {
        name: 'bar',
        open: 'bar<',
        action: () => null,
      }),
    ).throws(/bar</)
  })


  test('inject', () => {
    const SRC: any = {
      a: 'A',
      b: { b: 1 },
      bb: { bb: 1 },
      c: [2, 3],
    }

    const j = Jsonic.make().use(Directive, {
      name: 'inject',
      open: '@',
      rules: { open: 'val,pair' },
      action: (rule: Rule) => {
        const srcname = '' + rule.child.node
        const src = SRC[srcname]
        const from = rule.parent.name
        if ('pair' === from) {
          Object.assign(rule.parent.node, src)
        } else {
          rule.node = src
        }
      },
      custom: (jsonic: Jsonic, { OPEN, name }: any) => {
        jsonic.rule('val', (rs) => {
          rs.open({
            s: [OPEN],
            c: (r) => 0 === r.d,
            p: 'map',
            b: 1,
            n: { [name + '_top']: 1 },
          })
        })
        jsonic.rule('map', (rs) => {
          rs.open({
            s: [OPEN],
            c: (r) => 1 === r.d && 1 === r.n[name + '_top'],
            p: 'pair',
            b: 1,
          })
        })
      },
    })

    runSpec(j, 'inject.tsv')

    // Top-level directive without key: result shape is TS-specific.
    expect(j('@a')).equal({ 0: 'A' })
    expect(j('@c')).equal({ 0: 2, 1: 3 })
  })


  test('adder', () => {
    const j = Jsonic.make().use(Directive, {
      name: 'adder',
      open: 'add<',
      close: '>',
      action: (rule: Rule) => {
        let out: any = 0
        if (Array.isArray(rule.child.node)) {
          out = rule.child.node.reduce((a, v) => a + v)
        }
        rule.node = out
      },
    })

    runSpec(j, 'adder.tsv')

    const k = j.use(Directive, {
      name: 'multiplier',
      open: 'mul<',
      close: '>',
      action: (rule: Rule) => {
        let out: any = 0
        if (Array.isArray(rule.child.node)) {
          out = rule.child.node.reduce((a, v) => a * v)
        }
        rule.node = out
      },
    })

    runSpec(k, 'multiplier.tsv')

    // Non-numeric multiplication produces NaN in TS (language-specific).
    expect(k('[mul<a,1>]')).equal([NaN])

    // Original adder still works after second registration.
    runSpec(j, 'adder.tsv')
  })


  test('edges', () => {
    const j = Jsonic.make().use(Directive, {
      name: 'none',
      open: '@',
      action: () => null,
      rules: null,
    })
    expect(() => j('a:@x')).throws(/unexpected/)
  })


  test('error', () => {
    const j = Jsonic.make().use(Directive, {
      name: 'bad',
      open: '@',
      action: (rule: Rule) => rule.parent?.o0.bad('bad'),
    })
    expect(() => j('a:@x')).throws(/bad.*:1:3/s)
  })


  test('action-option-prop', () => {
    const j = Jsonic.make().use(Directive, {
      name: 'constant',
      open: '@',
      action: 'custom.x',
    })
    j.options({ custom: { x: 11 } })
    expect(j('@')).equal(11)
  })


  test('annotate', () => {
    const j = Jsonic.make().use(Directive, {
      name: 'annotate',
      open: '@',
      rules: { open: 'val' },
      action: (rule: Rule) => {
        rule.parent.u.note = '<' + rule.child.node + '>'
      },
      custom: (jsonic: Jsonic) => {
        jsonic.rule('annotate', (rs) => {
          rs.close([{ r: 'val', g: 'replace' }]).ac((rule, _ctx, next) => {
            rule.parent.child = next
          })
        })
        jsonic.rule('val', (rs) => {
          rs.bc((r) => {
            if (r.u.note) {
              r.node['@'] = r.u.note
            }
          })
        })
      },
    })

    runSpec(j, 'annotate.tsv')
  })

})
