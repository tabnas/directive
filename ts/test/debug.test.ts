/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */

/*  debug.test.ts
 *  The directive is a grammar-extension plugin; @tabnas/debug is the
 *  intended tool for inspecting what it registers (see CLAUDE.md). These
 *  tests use debug's structured model() to assert that a registered
 *  directive shows up in the host grammar's tokens/rules — exercising both
 *  the directive AND debug's structured output. (@tabnas/debug is a
 *  dev-only dependency; it is never part of directive's runtime graph.)
 */

import { test, describe } from 'node:test'
import assert from 'node:assert'

import type { Rule } from '@tabnas/parser'
import { Debug } from '@tabnas/debug'
import type { DebugModel } from '@tabnas/debug'
import { Directive } from '../dist/directive'
import { makeMini } from './mini-grammar'


function build() {
  return makeMini()
    .use(Directive, {
      name: 'upper',
      open: '@',
      action: (rule: Rule) =>
        (rule.node = ('' + rule.child.node).toUpperCase()),
    })
    .use(Debug, { print: false, trace: false })
}


describe('directive + debug', () => {

  test('debug.model() captures the directive rule and open token', () => {
    const j = build()
    const m: DebugModel = j.debug.model()

    // The directive registered an `upper` rule and an `#OD_upper` open token.
    assert.ok(
      m.rules.some((r) => r.name === 'upper'),
      'model.rules should include the directive rule',
    )
    assert.ok(
      m.tokens.some((t) => t.name.includes('OD_upper')),
      'model.tokens should include the directive open token',
    )

    // The mini host grammar's rules are present too.
    for (const rn of ['val', 'list', 'map', 'pair', 'elem']) {
      assert.ok(m.rules.some((r) => r.name === rn), 'missing host rule ' + rn)
    }

    // Plugins are listed in application order.
    assert.deepEqual(m.plugins.map((p) => p.name), ['mini', 'Directive', 'Debug'])
  })

  test('debug.model() alternates carry structured fields', () => {
    const m: DebugModel = build().debug.model()
    const val = m.rules.find((r) => r.name === 'val')
    assert.ok(val && Array.isArray(val.open) && val.open.length > 0)
    for (const alt of val.open) {
      assert.ok(Array.isArray(alt.seq), 'alt.seq should be an array')
      assert.ok(Array.isArray(alt.groups), 'alt.groups should be an array')
      assert.equal(typeof alt.action, 'boolean')
    }
  })

  test('the structured model is JSON-serialisable', () => {
    const m: DebugModel = build().debug.model()
    const grammar = {
      tokens: m.tokens, tokenSets: m.tokenSets,
      rules: m.rules, graph: m.graph, config: m.config, abnf: m.abnf,
    }
    const round = JSON.parse(JSON.stringify(grammar))
    assert.deepEqual(round.rules, m.rules)
  })

  test('debug.describe() mentions the directive rule', () => {
    const out = build().debug.describe()
    assert.equal(typeof out, 'string')
    assert.ok(out.includes('upper'), 'describe() should mention the upper directive rule')
  })

})
