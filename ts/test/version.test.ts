/* Copyright (c) 2026 Richard Rodger, MIT License */

// The exported VERSION must equal package.json "version".
//
// This is the CI check for version drift. It exists because the constant HAS
// drifted: @tabnas/json exported Version = '1.0.0' for several releases while
// the package shipped 0.4.x, because nothing rewrote it and AGENTS.md wrongly
// claimed `make publish-go` kept it in sync. A release that bumps
// package.json and forgets the constant now fails here.
//
// The reads below are deliberately unguarded: if package.json or the built
// package cannot be loaded, this file throws and the test FAILS. A version
// check that silently skips is the exact failure mode being designed out.

import { describe, it } from 'node:test'
import assert from 'node:assert'

// Read at runtime, relative to dist-test/, so the check sees the same
// package.json that npm publishes.
const pkg = require('../package.json')
const api = require('..')

describe('version', () => {
  it('VERSION matches package.json', () => {
    assert.equal(
      api.VERSION,
      pkg.version,
      `VERSION drift: ${pkg.name} exports ${api.VERSION} but package.json is ` +
        `${pkg.version}. Both are rewritten by admin/publish.sh at release; ` +
        `if you bumped one by hand, bump the other.`,
    )
  })

  it('VERSION is exported and looks like a semver', () => {
    assert.equal(
      typeof api.VERSION,
      'string',
      'VERSION must be exported as a string',
    )
    assert.match(api.VERSION, /^\d+\.\d+\.\d+/, 'VERSION must be a semver')
  })
})
