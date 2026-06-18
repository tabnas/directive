/* Copyright (c) 2025 Richard Rodger and other contributors, MIT License */

package tabnasdirective

import (
	tabnas "github.com/tabnas/parser/go"
)

// registerMiniGrammar installs a deliberately small grammar — scalar
// values, explicit lists [a, b], and explicit maps {k: v} (unquoted
// keys) — on top of the engine's default lexer (bare words, numbers,
// quoted strings). It is just enough structure to exercise the directive
// plugin without depending on a full JSON / jsonic grammar.
//
// The rule names (val, list, map, pair, elem) match the directive's
// default open/close rule targets. This mirrors ts/test/mini-grammar.ts;
// keep the two in step.
//
// The engine exposes a rule's alternates and state actions through
// methods (AddOpen/AddClose, AddBO/AddBC, …) rather than exported
// fields, so each rule is built up via those calls.
func registerMiniGrammar(j *tabnas.Tabnas) {
	// val: a value is a map, a list, or a plain scalar token.
	j.Rule("val", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
		rs.AddBO(func(r *tabnas.Rule, _ *tabnas.Context) {
			r.Node = tabnas.Undefined
		})
		rs.AddBC(func(r *tabnas.Rule, ctx *tabnas.Context) {
			if !tabnas.IsUndefined(r.Node) {
				return
			}
			if !tabnas.IsUndefined(r.Child.Node) {
				r.Node = r.Child.Node
				return
			}
			if r.OS == 0 {
				r.Node = tabnas.Undefined
				return
			}
			r.Node = r.O0.ResolveVal(r, ctx)
		})
		rs.AddOpen(
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinOB}}, P: "map", B: 1},
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinOS}}, P: "list", B: 1},
			&tabnas.AltSpec{S: [][]tabnas.Tin{tabnas.TinSetVAL}},
		)
		rs.AddClose(
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinZZ}}},
			// Implicit list: a standalone value followed by a comma starts
			// a bracketless list. Only fires outside an elem/pair/ilist
			// position (so explicit `[a, b]` and `{k: v}` are unaffected)
			// and where implicit lists are permitted (n.dlist != 1, the
			// counter the directive plugin sets to 1 to suppress them).
			&tabnas.AltSpec{
				S: [][]tabnas.Tin{{tabnas.TinCA}},
				B: 1,
				C: func(r *tabnas.Rule, _ *tabnas.Context) bool {
					return r.N["dlist"] != 1 &&
						r.Parent != nil && r.Parent != tabnas.NoRule &&
						r.Parent.Name != "elem" &&
						r.Parent.Name != "pair" &&
						r.Parent.Name != "ilist"
				},
				R: "ilist",
				// Seed the list with the already-parsed scalar value.
				A: func(r *tabnas.Rule, _ *tabnas.Context) {
					r.Node = []any{r.Node}
				},
			},
			&tabnas.AltSpec{B: 1},
		)
	})

	// map: an object `{ k: v, ... }`.
	j.Rule("map", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
		rs.AddBO(func(r *tabnas.Rule, _ *tabnas.Context) {
			r.Node = make(map[string]any)
		})
		rs.AddOpen(
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinOB}, {tabnas.TinCB}}, B: 1, N: map[string]int{"pk": 0}},
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinOB}}, P: "pair", N: map[string]int{"pk": 0}},
		)
		rs.AddClose(
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinCB}}},
		)
	})

	// list: an array `[ a, b, ... ]`.
	j.Rule("list", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
		rs.AddBO(func(r *tabnas.Rule, _ *tabnas.Context) {
			r.Node = make([]any, 0)
		})
		rs.AddOpen(
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinOS}, {tabnas.TinCS}}, B: 1},
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinOS}}, P: "elem"},
		)
		rs.AddClose(
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinCS}}},
		)
	})

	// pair: a key:value entry inside a map.
	j.Rule("pair", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
		rs.AddBC(func(r *tabnas.Rule, _ *tabnas.Context) {
			if _, ok := r.U["pair"]; !ok {
				return
			}
			key, _ := r.U["key"].(string)
			val := r.Child.Node
			if tabnas.IsUndefined(val) {
				val = nil
			}
			if m, ok := r.Node.(map[string]any); ok {
				m[key] = val
			}
		})
		rs.AddOpen(
			&tabnas.AltSpec{
				S: [][]tabnas.Tin{tabnas.TinSetKEY, {tabnas.TinCL}},
				P: "val",
				U: map[string]any{"pair": true},
				A: func(r *tabnas.Rule, _ *tabnas.Context) {
					kt := r.O0
					if s, ok := kt.Val.(string); ok {
						r.U["key"] = s
					} else {
						r.U["key"] = kt.Src
					}
				},
			},
		)
		rs.AddClose(
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinCA}}, R: "pair"},
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinCB}}, B: 1},
		)
	})

	// elem: a value inside a list.
	j.Rule("elem", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
		rs.AddBC(func(r *tabnas.Rule, _ *tabnas.Context) {
			if tabnas.IsUndefined(r.Child.Node) {
				return
			}
			if s, ok := r.Node.([]any); ok {
				r.Node = append(s, r.Child.Node)
				if r.Parent != tabnas.NoRule && r.Parent != nil {
					r.Parent.Node = r.Node
				}
			}
		})
		rs.AddOpen(
			&tabnas.AltSpec{P: "val"},
		)
		rs.AddClose(
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinCA}}, R: "elem"},
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinCS}}, B: 1},
		)
	})

	// ilist: bracketless list continuation. Created by replacing a val
	// (so it inherits the seeded `[first]` node) and then absorbs `, value`
	// pairs until something else closes it.
	j.Rule("ilist", func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
		push := func(r *tabnas.Rule, _ *tabnas.Context) {
			if tabnas.IsUndefined(r.Child.Node) {
				return
			}
			if s, ok := r.Node.([]any); ok {
				r.Node = append(s, r.Child.Node)
			}
		}
		rs.AddOpen(
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinCA}}, P: "val"}, // consume comma, parse next value
		)
		rs.AddClose(
			&tabnas.AltSpec{S: [][]tabnas.Tin{{tabnas.TinCA}}, B: 1, R: "ilist", A: push}, // more elements
			&tabnas.AltSpec{B: 1, A: push}, // final element
		)
	})
}

// makeMini builds a parser with just the mini grammar installed.
func makeMini() *tabnas.Tabnas {
	j := tabnas.Make()
	registerMiniGrammar(j)
	return j
}
