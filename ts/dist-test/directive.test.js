"use strict";
/* Copyright (c) 2021-2025 Richard Rodger and other contributors, MIT License */
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const node_test_1 = require("node:test");
const node_assert_1 = __importDefault(require("node:assert"));
const node_fs_1 = __importDefault(require("node:fs"));
const node_path_1 = __importDefault(require("node:path"));
const jsonic_1 = require("jsonic");
const directive_1 = require("../dist/directive");
// Jsonic produces null-prototype objects; normalize both sides so
// deepStrictEqual's prototype check doesn't spuriously fail.
const normalize = (v) => {
    if (v === null || typeof v !== 'object')
        return v;
    if (Array.isArray(v))
        return v.map(normalize);
    const out = {};
    for (const k of Object.keys(v))
        out[k] = normalize(v[k]);
    return out;
};
const expect = (actual) => ({
    equal: (expected) => node_assert_1.default.deepStrictEqual(normalize(actual), normalize(expected)),
    exist: () => node_assert_1.default.ok(actual != null),
    throws: (matcher) => node_assert_1.default.throws(actual, matcher),
});
const loadSpec = (name) => {
    const text = node_fs_1.default.readFileSync(node_path_1.default.join(__dirname, '..', 'test', 'spec', name), 'utf8');
    const cases = [];
    for (const raw of text.split('\n')) {
        const line = raw.replace(/\r$/, '');
        if (line.length === 0 || line.startsWith('#'))
            continue;
        const i = line.indexOf('\t');
        if (i < 0)
            continue;
        cases.push({ input: line.slice(0, i), expected: line.slice(i + 1) });
    }
    return cases;
};
const runSpec = (j, name) => {
    for (const { input, expected } of loadSpec(name)) {
        if (expected.startsWith('!error ')) {
            const pattern = new RegExp(expected.slice('!error '.length));
            node_assert_1.default.throws(() => j(input), pattern, `input: ${JSON.stringify(input)}`);
        }
        else {
            const want = JSON.parse(expected);
            node_assert_1.default.deepStrictEqual(normalize(j(input)), normalize(want), `input: ${JSON.stringify(input)}`);
        }
    }
};
(0, node_test_1.describe)('directive', () => {
    (0, node_test_1.test)('happy', () => {
        const j = jsonic_1.Jsonic.make()
            .use(directive_1.Directive, {
            name: 'upper',
            open: '@',
            action: (rule) => (rule.node = ('' + rule.child.node).toUpperCase()),
        });
        expect(j.token.OD_upper).exist();
        expect(j.rule('upper')).exist();
        runSpec(j, 'happy.tsv');
        // Implicit-list special: pair after directive is silently dropped from
        // the array but still accessible as a property on the result.
        const clone = (x) => JSON.parse(JSON.stringify(x));
        expect(clone(j('1, @a, b:2'))).equal([1, 'A']);
        expect(j('1, @a, b:2').b).equal(2);
    });
    (0, node_test_1.test)('subobj', () => {
        const { deep } = jsonic_1.Jsonic.util;
        const j = jsonic_1.Jsonic.make()
            .use(directive_1.Directive, {
            name: 'subobj',
            open: '@',
            rules: {
                open: {
                    val: {},
                    pair: { c: (r) => r.lte('pk') },
                },
            },
            action: (rule) => {
                const key = rule.child.node;
                const res = { [key]: ('' + key).toUpperCase() };
                rule.parent.parent.node = deep(rule.parent.parent.node, res);
                return undefined;
            },
            custom: (jsonic, { OPEN, name }) => {
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
                    ]);
                });
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
                    });
                });
                jsonic.rule('pair', (rs) => {
                    rs.close({
                        s: [OPEN],
                        c: (r) => 0 < r.n.pk,
                        b: 1,
                        g: name + '-undive',
                    });
                });
            },
        });
        runSpec(j, 'subobj.tsv');
    });
    (0, node_test_1.test)('close', () => {
        const j = jsonic_1.Jsonic.make().use(directive_1.Directive, {
            name: 'foo',
            open: 'foo<',
            close: '>',
            action: (rule) => (rule.node = 'FOO'),
        });
        runSpec(j, 'close-foo.tsv');
        // Trailing-comma-before-close acceptance (TS-only meta option).
        expect(j('a:foo<y:2,>', { xlog: -1 })).equal({ a: 'FOO' });
        const k = j.use(directive_1.Directive, {
            name: 'bar',
            open: 'bar<',
            close: '>',
            action: (rule) => (rule.node = 'BAR'),
        });
        runSpec(k, 'close-foo-bar.tsv');
        // Re-registering the same open token should error.
        expect(() => j.use(directive_1.Directive, {
            name: 'bar',
            open: 'bar<',
            action: () => null,
        })).throws(/bar</);
    });
    (0, node_test_1.test)('inject', () => {
        const SRC = {
            a: 'A',
            b: { b: 1 },
            bb: { bb: 1 },
            c: [2, 3],
        };
        const j = jsonic_1.Jsonic.make().use(directive_1.Directive, {
            name: 'inject',
            open: '@',
            rules: { open: 'val,pair' },
            action: (rule) => {
                const srcname = '' + rule.child.node;
                const src = SRC[srcname];
                const from = rule.parent.name;
                if ('pair' === from) {
                    Object.assign(rule.parent.node, src);
                }
                else {
                    rule.node = src;
                }
            },
            custom: (jsonic, { OPEN, name }) => {
                jsonic.rule('val', (rs) => {
                    rs.open({
                        s: [OPEN],
                        c: (r) => 0 === r.d,
                        p: 'map',
                        b: 1,
                        n: { [name + '_top']: 1 },
                    });
                });
                jsonic.rule('map', (rs) => {
                    rs.open({
                        s: [OPEN],
                        c: (r) => 1 === r.d && 1 === r.n[name + '_top'],
                        p: 'pair',
                        b: 1,
                    });
                });
            },
        });
        runSpec(j, 'inject.tsv');
        // Top-level directive without key: result shape is TS-specific.
        expect(j('@a')).equal({ 0: 'A' });
        expect(j('@c')).equal({ 0: 2, 1: 3 });
    });
    (0, node_test_1.test)('adder', () => {
        const j = jsonic_1.Jsonic.make().use(directive_1.Directive, {
            name: 'adder',
            open: 'add<',
            close: '>',
            action: (rule) => {
                let out = 0;
                if (Array.isArray(rule.child.node)) {
                    out = rule.child.node.reduce((a, v) => a + v);
                }
                rule.node = out;
            },
        });
        runSpec(j, 'adder.tsv');
        const k = j.use(directive_1.Directive, {
            name: 'multiplier',
            open: 'mul<',
            close: '>',
            action: (rule) => {
                let out = 0;
                if (Array.isArray(rule.child.node)) {
                    out = rule.child.node.reduce((a, v) => a * v);
                }
                rule.node = out;
            },
        });
        runSpec(k, 'multiplier.tsv');
        // Non-numeric multiplication produces NaN in TS (language-specific).
        expect(k('[mul<a,1>]')).equal([NaN]);
        // Original adder still works after second registration.
        runSpec(j, 'adder.tsv');
    });
    (0, node_test_1.test)('edges', () => {
        const j = jsonic_1.Jsonic.make().use(directive_1.Directive, {
            name: 'none',
            open: '@',
            action: () => null,
            rules: null,
        });
        expect(() => j('a:@x')).throws(/unexpected/);
    });
    (0, node_test_1.test)('error', () => {
        const j = jsonic_1.Jsonic.make().use(directive_1.Directive, {
            name: 'bad',
            open: '@',
            action: (rule) => rule.parent?.o0.bad('bad'),
        });
        expect(() => j('a:@x')).throws(/bad.*:1:3/s);
    });
    (0, node_test_1.test)('action-option-prop', () => {
        const j = jsonic_1.Jsonic.make().use(directive_1.Directive, {
            name: 'constant',
            open: '@',
            action: 'custom.x',
        });
        j.options({ custom: { x: 11 } });
        expect(j('@')).equal(11);
    });
    (0, node_test_1.test)('annotate', () => {
        const j = jsonic_1.Jsonic.make().use(directive_1.Directive, {
            name: 'annotate',
            open: '@',
            rules: { open: 'val' },
            action: (rule) => {
                rule.parent.u.note = '<' + rule.child.node + '>';
            },
            custom: (jsonic) => {
                jsonic.rule('annotate', (rs) => {
                    rs.close([{ r: 'val', g: 'replace' }]).ac((rule, _ctx, next) => {
                        rule.parent.child = next;
                    });
                });
                jsonic.rule('val', (rs) => {
                    rs.bc((r) => {
                        if (r.u.note) {
                            r.node['@'] = r.u.note;
                        }
                    });
                });
            },
        });
        runSpec(j, 'annotate.tsv');
    });
});
//# sourceMappingURL=directive.test.js.map