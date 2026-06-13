/* Copyright (c) 2021-2025 Richard Rodger and other contributors, MIT License */

package directive

import (
	"fmt"

	jsonic "github.com/jsonicjs/jsonic/go"
)

const Version = "0.1.4"

// Action is called when a directive is processed.
// It receives the directive rule and parse context. The rule's Child.Node
// contains the parsed content between open (and optional close) tokens.
// Set rule.Node to the directive's result value.
type Action func(rule *jsonic.Rule, ctx *jsonic.Context)

// RuleMod configures how a directive integrates with an existing grammar rule.
type RuleMod struct {
	// C is an optional condition that must be true for the directive to match
	// within this rule.
	C jsonic.AltCond
}

// RulesOption configures which grammar rules are modified by the directive.
// Open rules detect the directive open token and push to the directive rule.
// Close rules detect the close token (if any) to end sibling parsing.
type RulesOption struct {
	Open  map[string]*RuleMod
	Close map[string]*RuleMod
}

// CustomFunc allows additional customization of the jsonic instance
// after the directive rule is created.
type CustomFunc func(j *jsonic.Jsonic, config DirectiveConfig)

// DirectiveConfig holds the resolved token Tins for a directive,
// passed to CustomFunc callbacks.
type DirectiveConfig struct {
	OPEN  jsonic.Tin
	CLOSE jsonic.Tin // -1 if no close token
	Name  string
}

// DirectiveOptions configures the Directive plugin.
type DirectiveOptions struct {
	// Name is the directive name, used as the rule name and token prefix.
	Name string

	// Open is the character sequence that starts the directive.
	// Must be unique (not already a registered fixed token).
	Open string

	// Close is the optional character sequence that ends the directive.
	// If empty, the directive consumes a single value after the open token.
	Close string

	// Action is called when the directive content has been parsed.
	Action Action

	// Rules controls which existing grammar rules are modified.
	// nil means use defaults: open="val", close="list,elem,map,pair".
	// Set to &RulesOption{} to override defaults with no rules.
	Rules *RulesOption

	// Custom allows additional jsonic customization after directive setup.
	Custom CustomFunc
}

// Apply registers the Directive plugin on the given jsonic instance with
// typed options. It is the convenience constructor mirroring the
// TypeScript `j.use(Directive, options)` call; under the hood it forwards
// the options to j.Use as the plugin option map. Returns the jsonic
// instance for chaining.
//
// To register the raw plugin directly — e.g. from a JSON-driven config —
// call j.Use(directive.Directive, opts) with the same option keys
// ("name", "open", "close", "action", "rules", "custom").
func Apply(j *jsonic.Jsonic, opts DirectiveOptions) *jsonic.Jsonic {
	pluginOpts := map[string]any{
		"name":   opts.Name,
		"open":   opts.Open,
		"close":  opts.Close,
		"action": opts.Action,
		"custom": opts.Custom,
	}
	// Distinguish "rules omitted" (use defaults) from an explicit empty
	// RulesOption (modify no rules): only set the key when provided.
	if opts.Rules != nil {
		pluginOpts["rules"] = opts.Rules
	}
	_ = j.Use(Directive, pluginOpts)
	return j
}

// defaultRules returns the default rules configuration.
func defaultRules() *RulesOption {
	return &RulesOption{
		Open: map[string]*RuleMod{
			"val": {},
		},
		Close: map[string]*RuleMod{
			"list": {},
			"elem": {},
			"map":  {},
			"pair": {},
		},
	}
}

// resolveRules normalizes a rules map, ensuring no nil entries.
func resolveRules(rules map[string]*RuleMod) map[string]*RuleMod {
	if rules == nil {
		return map[string]*RuleMod{}
	}
	result := make(map[string]*RuleMod, len(rules))
	for k, v := range rules {
		if v == nil {
			v = &RuleMod{}
		}
		result[k] = v
	}
	return result
}

// Directive is the jsonic plugin that adds directive syntax support. A
// directive defines a custom token sequence (open and optional close)
// that triggers an action callback to transform the parsed content.
//
// It follows the standard jsonic plugin shape — a Plugin value that reads
// its configuration from the option map passed to j.Use. Recognised keys:
//
//	"name"   string      — directive/rule name (required)
//	"open"   string      — open token source (required)
//	"close"  string      — optional close token source
//	"action" Action      — content transform callback
//	"rules"  *RulesOption — rules to modify; omit for defaults
//	"custom" CustomFunc   — extra setup callback
//
// Most callers use the typed Apply constructor rather than calling this
// directly.
var Directive jsonic.Plugin = func(j *jsonic.Jsonic, opts map[string]any) error {
	name, _ := opts["name"].(string)
	open, _ := opts["open"].(string)
	close_, _ := opts["close"].(string)
	action, _ := opts["action"].(Action)
	custom, _ := opts["custom"].(CustomFunc)
	hasClose := close_ != ""

	// Resolve rules: an absent "rules" key means use defaults; a present
	// (even empty) *RulesOption is honoured as-is.
	var openRules, closeRules map[string]*RuleMod
	if rulesOpt, ok := opts["rules"].(*RulesOption); ok && rulesOpt != nil {
		openRules = resolveRules(rulesOpt.Open)
		closeRules = resolveRules(rulesOpt.Close)
	} else {
		defaults := defaultRules()
		openRules = resolveRules(defaults.Open)
		closeRules = resolveRules(defaults.Close)
	}

	// The open token must not already be registered.
	cfg := j.Config()
	if _, exists := cfg.FixedTokens[open]; exists {
		panic(fmt.Sprintf("Directive open token already in use: %s", open))
	}

	// Register the open fixed token.
	openTN := "#OD_" + name
	OPEN := j.Token(openTN, open)

	// Register or look up the close fixed token.
	var CLOSE jsonic.Tin = -1
	closeTN := ""
	if hasClose {
		if existing, exists := cfg.FixedTokens[close_]; exists {
			// Reuse an existing close token (e.g. shared with another
			// directive). Grab its registered name so the grammar spec
			// below resolves to the same Tin via j.Token(name).
			CLOSE = existing
			closeTN = j.TinName(existing)
		} else {
			closeTN = "#CD_" + name
			CLOSE = j.Token(closeTN, close_)
		}
	}

	// Build a Ref map for all state actions and condition functions
	// referenced by the grammar spec below.
	ref := map[jsonic.FuncRef]any{}

	// Auto-wired state actions on the directive rule (@<name>-bo, @<name>-bc).
	ref[jsonic.FuncRef("@"+name+"-bo")] = jsonic.StateAction(
		func(r *jsonic.Rule, ctx *jsonic.Context) {
			r.Node = make(map[string]any)
		},
	)
	ref[jsonic.FuncRef("@"+name+"-bc")] = jsonic.StateAction(
		func(r *jsonic.Rule, ctx *jsonic.Context) {
			// Follow the replacement chain to get the final child node.
			// When a val rule is replaced by a list rule (implicit list),
			// the original child's Node may be stale in Go because slice
			// append can reallocate. Walk the Prev-linked replacement
			// chain to find the last replacement and adopt its Node.
			if r.Child != nil && r.Child != jsonic.NoRule {
				final := r.Child
				for final.Next != nil && final.Next != jsonic.NoRule &&
					final.Next.Prev == final {
					final = final.Next
				}
				if final != r.Child {
					r.Child.Node = final.Node
				}
			}
			if action != nil {
				action(r, ctx)
			}
		},
	)

	// Declarative grammar spec built up below and applied via j.Grammar().
	gs := &jsonic.GrammarSpec{
		Ref:  ref,
		Rule: map[string]*jsonic.GrammarRuleSpec{},
	}
	ruleFor := func(rn string) *jsonic.GrammarRuleSpec {
		if existing, ok := gs.Rule[rn]; ok {
			return existing
		}
		r := &jsonic.GrammarRuleSpec{}
		gs.Rule[rn] = r
		return r
	}

	// ---- Modify existing rules for OPEN token detection ----

	for rulename, rulemod := range openRules {
		rn := rulename
		rm := rulemod

		var openAlts []*jsonic.GrammarAltSpec
		var closeAlts []*jsonic.GrammarAltSpec

		if hasClose {
			// OPEN+CLOSE (empty directive) must be tried before OPEN alone.
			openAlts = append(openAlts, &jsonic.GrammarAltSpec{
				S: openTN + " " + closeTN,
				B: 1,
				P: name,
				N: map[string]int{"dr_" + name: 1},
				G: "start,end",
			})
			closeAlts = append(closeAlts, &jsonic.GrammarAltSpec{
				S: closeTN,
				B: 1,
				G: "end",
			})
		}

		openAlt := &jsonic.GrammarAltSpec{
			S: openTN,
			P: name,
			N: map[string]int{"dr_" + name: 1},
			G: "start",
		}
		if rm.C != nil {
			cref := jsonic.FuncRef("@dr-open-c-" + name + "-" + rn)
			ref[cref] = rm.C
			openAlt.C = string(cref)
		}
		openAlts = append(openAlts, openAlt)

		r := ruleFor(rn)
		r.Open = openAlts
		if len(closeAlts) > 0 {
			r.Close = closeAlts
		}
	}

	// ---- Modify existing rules for CLOSE token detection ----

	if hasClose {
		for rulename, rulemod := range closeRules {
			rn := rulename
			rm := rulemod

			closeCRef := jsonic.FuncRef("@dr-close-c-" + name + "-" + rn)
			ref[closeCRef] = jsonic.AltCond(
				func(r *jsonic.Rule, ctx *jsonic.Context) bool {
					if r.N["dr_"+name] != 1 {
						return false
					}
					if rm.C != nil {
						return rm.C(r, ctx)
					}
					return true
				},
			)
			commaCRef := jsonic.FuncRef("@dr-close-ca-c-" + name + "-" + rn)
			ref[commaCRef] = jsonic.AltCond(
				func(r *jsonic.Rule, ctx *jsonic.Context) bool {
					return r.N["dr_"+name] == 1
				},
			)

			closeAlts := []*jsonic.GrammarAltSpec{
				{
					S: closeTN,
					C: string(closeCRef),
					B: 1,
					G: "end",
				},
				{
					S: "#CA " + closeTN,
					C: string(commaCRef),
					B: 1,
					G: "end,comma",
				},
			}

			r := ruleFor(rn)
			r.Close = closeAlts
		}
	}

	// ---- Directive rule alts ----

	var dirOpen []*jsonic.GrammarAltSpec
	if hasClose {
		// Check for immediate close (empty directive).
		dirOpen = append(dirOpen, &jsonic.GrammarAltSpec{
			S: closeTN,
			B: 1,
		})
	}
	// Push to val rule to parse directive content.
	// Counter settings control implicit list/map creation:
	//   With close: reset counters (allow implicits within boundaries)
	//   Without close: increment counters (prevent implicits consuming siblings)
	counters := map[string]int{}
	if hasClose {
		counters["dlist"] = 0
		counters["dmap"] = 0
	} else {
		counters["dlist"] = 1
		counters["dmap"] = 1
	}
	dirOpen = append(dirOpen, &jsonic.GrammarAltSpec{
		P: "val",
		N: counters,
	})

	var dirClose []*jsonic.GrammarAltSpec
	if hasClose {
		dirClose = []*jsonic.GrammarAltSpec{
			{S: closeTN},
			{S: "#CA " + closeTN},
		}
	}

	dr := ruleFor(name)
	dr.Open = dirOpen
	if len(dirClose) > 0 {
		dr.Close = dirClose
	}

	// Clear any pre-existing alts/state actions on the directive rule so
	// that j.Grammar() below installs a clean set via wireStateActions +
	// prepend onto empty slices.
	j.Rule(name, func(rs *jsonic.RuleSpec, _ *jsonic.Parser) {
		rs.Clear()
	})

	// Apply grammar with 'directive' group tag appended to every alt.
	setting := &jsonic.GrammarSetting{
		Rule: &jsonic.GrammarSettingRule{
			Alt: &jsonic.GrammarSettingAlt{G: "directive"},
		},
	}
	if err := j.Grammar(gs, setting); err != nil {
		return err
	}

	// ---- Custom callback ----

	if custom != nil {
		closeTin := jsonic.Tin(-1)
		if hasClose {
			closeTin = CLOSE
		}
		custom(j, DirectiveConfig{
			OPEN:  OPEN,
			CLOSE: closeTin,
			Name:  name,
		})
	}

	return nil
}
