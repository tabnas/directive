/* Copyright (c) 2021-2022 Richard Rodger, MIT License */

import {
  Jsonic,
  Rule,
  RuleSpec,
  StateAction,
  Plugin,
  Context,
  Token,
  Tin,
} from 'jsonic'


type DirectiveOptions = {
  name: string
  open: string
  action: StateAction | string
  close?: string
  rules?: {
    open?: string | string[] | Record<string, { c?: Function }>
    close?: string | string[] | Record<string, { c?: Function }>
  }
  custom?: (
    jsonic: Jsonic,
    config: { OPEN: Tin; CLOSE: Tin | null | undefined; name: string },
  ) => void
}


const resolveRules = (
  rules: undefined | string | string[] | Record<string, { c?: Function }>
) => {
  const rulemap = {}

  if ('string' == typeof rules) {
    rules = rules.split(/\s*,\s*/)
  }

  if (Array.isArray(rules)) {
    rules.reduce((a: any, n: any) => ((null != n && '' !== n ? a[n] = {} : null), a), rulemap)
  }
  else if (null != rules) {
    Object.keys(rules)
      .reduce((a: any, k: any) => ((null != rules[k] ? a[k] = rules[k] : null), a), rulemap)
  }

  return rulemap
}


const Directive: Plugin = (jsonic: Jsonic, options: DirectiveOptions) => {
  let rules = {
    open: resolveRules(options?.rules?.open),
    close: resolveRules(options?.rules?.close),
  }

  let name = options.name
  let open = options.open
  let close = options.close
  let action: StateAction
  let custom = options.custom

  if ('string' === typeof options.action) {
    let path = options.action
    action = (rule: Rule) =>
      (rule.node = jsonic.util.prop(jsonic.options, path))
  } else {
    action = options.action
  }

  let token: Record<string, string> = {}

  let openTN = '#OD_' + name
  let closeTN = '#CD_' + name

  let OPEN: Tin = jsonic.fixed(open) as Tin
  let CLOSE = null == close ? null : jsonic.fixed(close)

  // OPEN must be unique
  if (null != OPEN) {
    throw new Error('Directive open token already in use: ' + open)
  } else {
    token[openTN] = open
  }

  // Only create CLOSE if not already defined as a fixed token
  if (null == CLOSE && null != close) {
    token[closeTN] = close
  }

  jsonic.options({
    fixed: {
      token,
    },
    error: {
      [name + '_close']:
        null == close
          ? null
          : 'directive ' +
          name +
          ' close "' +
          close +
          '" without open "' +
          open +
          '"',
    },
    hint: {
      [name + '_close']:
        null == close
          ? null
          : `
The {name} directive must start with the characters "{open}" and end
with the characters "{close}". The end characters "{close}" may not
appear without the start characters "{open}" appearing first:
"{open}...{close}".
`,
    },
  })

  let CA = jsonic.token.CA
  OPEN = jsonic.fixed(open) as Tin
  CLOSE = null == close ? null : jsonic.fixed(close)

  // NOTE: RuleSpec.open|close refers to Rule state, whereas
  // OPEN|CLOSE refers to opening and closing tokens for the directive.

  // Pre-seed the directive rule's hooks (clear existing alts, set bo/bc).
  // `grammar()` below will then install the open/close alts with the
  // `directive` group tag appended via the setting arg.
  jsonic.rule(name, (rs) =>
    rs
      .clear()
      .bo((rule: Rule) => ((rule.node = {}), undefined))
      .bc(function(
        this: RuleSpec,
        rule: Rule,
        ctx: Context,
        next: Rule,
        tkn?: Token | void,
      ) {
        let out = action.call(this, rule, ctx, next, tkn)
        if (out?.isToken) {
          return out
        }
      }),
  )

  // Build a declarative grammar spec covering every rule modification
  // plus the directive rule's own alts.
  const grammarSpec: any = { rule: {} }
  const ruleFor = (rn: string) =>
    (grammarSpec.rule[rn] = grammarSpec.rule[rn] || {})

  Object.entries(rules.open).forEach((entry: any[]) => {
    const [rulename, rulespec] = entry
    const openAlts: any[] = []
    const closeAlts: any[] = []
    if (null != close) {
      // More-specific OPEN+CLOSE alt first so it's tried before OPEN alone.
      openAlts.push({
        s: [OPEN, CLOSE],
        b: 1,
        p: name,
        n: { ['dr_' + name]: 1 },
        g: 'start,end',
      })
      closeAlts.push({
        s: [CLOSE],
        b: 1,
        g: 'end',
      })
    }
    openAlts.push({
      s: [OPEN],
      p: name,
      n: { ['dr_' + name]: 1 },
      g: 'start',
      c: rulespec.c,
    })
    const r = ruleFor(rulename)
    r.open = openAlts
    if (closeAlts.length) r.close = closeAlts
  })

  if (null != close) {
    Object.entries(rules.close).forEach((entry: any[]) => {
      const [rulename, rulespec] = entry
      const r = ruleFor(rulename)
      r.close = [
        {
          s: [CLOSE],
          c: (r: Rule, ctx: Context) =>
            1 === r.n['dr_' + name] &&
            (rulespec.c ? rulespec.c(r, ctx) : true),
          b: 1,
          g: 'end',
        },
        {
          s: [CA, CLOSE],
          c: (r: Rule) => 1 === r.n['dr_' + name],
          b: 1,
          g: 'end,comma',
        },
      ]
    })
  }

  const directiveOpenAlts: any[] = []
  if (null != close) {
    directiveOpenAlts.push({ s: [CLOSE], b: 1 })
  }
  directiveOpenAlts.push({
    p: 'val',
    // Only accept implicits when there is a CLOSE token,
    // otherwise we'll eat all following siblings.
    n: null == close ? { dlist: 1, dmap: 1 } : { dlist: 0, dmap: 0 },
  })
  const dr = ruleFor(name)
  dr.open = directiveOpenAlts
  if (null != close) {
    dr.close = [{ s: [CLOSE] }, { s: [CA, CLOSE] }]
  }

  jsonic.grammar(grammarSpec, { rule: { alt: { g: 'directive' } } })

  if (custom) {
    custom(jsonic, { OPEN, CLOSE, name })
  }
}

Directive.defaults = {
  rules: {
    // By default, directives only operate where vals occur.
    open: 'val',
    close: 'list,elem,map,pair',
  },
} as DirectiveOptions

export { Directive }

export type { DirectiveOptions }
