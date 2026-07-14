/* Copyright (c) 2021-2025 Richard Rodger and other contributors, MIT License */

package tabnasdirective

import (
	"fmt"
	"strings"

	tabnas "github.com/tabnas/parser/go"
)

const Version = "0.2.0"

// Action is called when a directive is processed.
// It receives the directive rule and parse context. The rule's Child.Node
// contains the parsed content between open (and optional close) tokens.
// Set rule.Node to the directive's result value.
type Action func(rule *tabnas.Rule, ctx *tabnas.Context)

// TokenAction is an Action variant that may return a value. When the
// returned value is a *tabnas.Token, the directive's before-close hook
// propagates it exactly as the TypeScript plugin does: an error token
// (Err set) halts the parse with that token; any other token is passed
// through and otherwise ignored. Non-token return values are discarded.
type TokenAction func(rule *tabnas.Rule, ctx *tabnas.Context) any

// RuleMod configures how a directive integrates with an existing grammar rule.
type RuleMod struct {
	// C is an optional condition that must be true for the directive to match
	// within this rule.
	C tabnas.AltCond
}

// RulesOption configures which grammar rules are modified by the directive.
// Open rules detect the directive open token and push to the directive rule.
// Close rules detect the close token (if any) to end sibling parsing.
type RulesOption struct {
	Open  map[string]*RuleMod
	Close map[string]*RuleMod
}

// CustomFunc allows additional customization of the tabnas instance
// after the directive rule is created.
type CustomFunc func(j *tabnas.Tabnas, config DirectiveConfig)

// DirectiveConfig holds the resolved token Tins for a directive,
// passed to CustomFunc callbacks.
type DirectiveConfig struct {
	OPEN  tabnas.Tin
	CLOSE tabnas.Tin // -1 if no close token
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
	// Mirroring the TypeScript `action: StateAction | string` option it
	// accepts several forms:
	//
	//	Action / func(*tabnas.Rule, *tabnas.Context)
	//	    — classic action; set rule.Node to the result value.
	//	TokenAction / func(*tabnas.Rule, *tabnas.Context) any
	//	    — action that may return a *tabnas.Token to propagate from
	//	      the before-close hook (see TokenAction).
	//	string
	//	    — a dot-path resolved against the parser options at
	//	      directive-execution time; the resolved value becomes the
	//	      directive's node. In TypeScript the options object is open,
	//	      so the path starts at its top level; the Go options struct
	//	      is closed, so the path is resolved in the plugin-options
	//	      namespace (TS options.plugin): "custom.x" reads
	//	      j.PluginOptions("custom")["x"].
	Action any

	// Rules controls which existing grammar rules are modified.
	// nil means use defaults: open="val", close="list,elem,map,pair".
	// Set to &RulesOption{} to override defaults with no rules.
	Rules *RulesOption

	// Custom allows additional tabnas customization after directive setup.
	Custom CustomFunc
}

// Apply registers the Directive plugin on the given tabnas instance with
// typed options. It is the convenience constructor mirroring the
// TypeScript `j.use(Directive, options)` call; under the hood it forwards
// the options to j.Use as the plugin option map. It returns the tabnas
// instance (for chaining) together with any registration error — e.g. a
// duplicate open token or a grammar build failure. The plugin never
// panics: every failure path is reported through this error.
//
// To register the raw plugin directly — e.g. from a JSON-driven config —
// call j.Use(directive.Directive, opts) with the same option keys
// ("name", "open", "close", "action", "rules", "custom").
func Apply(j *tabnas.Tabnas, opts DirectiveOptions) (*tabnas.Tabnas, error) {
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
	if err := j.Use(Directive, pluginOpts); err != nil {
		return j, err
	}
	return j, nil
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

// resolveAction normalizes the polymorphic "action" option (see
// DirectiveOptions.Action) into a single TokenAction. A string is
// resolved as a dot-path against the parser options at
// directive-execution time (TS: `rule.node = tabnas.util.prop(
// tabnas.options, path)`); func forms are wrapped as needed. An absent
// or unrecognized value yields nil (no action).
func resolveAction(j *tabnas.Tabnas, opt any) TokenAction {
	switch a := opt.(type) {
	case string:
		if a == "" {
			return nil
		}
		path := a
		return func(r *tabnas.Rule, ctx *tabnas.Context) any {
			r.Node = optionProp(j, path)
			return nil
		}
	case Action:
		return func(r *tabnas.Rule, ctx *tabnas.Context) any {
			a(r, ctx)
			return nil
		}
	case func(*tabnas.Rule, *tabnas.Context):
		return func(r *tabnas.Rule, ctx *tabnas.Context) any {
			a(r, ctx)
			return nil
		}
	case TokenAction:
		return a
	case func(*tabnas.Rule, *tabnas.Context) any:
		return a
	}
	return nil
}

// optionProp resolves a dot-path within the parser's plugin-options
// namespace (the Go home for arbitrary option data; the TS options
// object is open, so TS resolves from its top level). The lookup happens
// at call time, so options set after plugin registration are seen —
// matching the TS closure over `tabnas.options`. Missing segments
// resolve to nil.
func optionProp(j *tabnas.Tabnas, path string) any {
	parts := strings.Split(path, ".")
	var cur any = j.PluginOptions(parts[0])
	for _, p := range parts[1:] {
		m, ok := cur.(map[string]any)
		if !ok {
			return nil
		}
		cur = m[p]
	}
	return cur
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

// Directive is the tabnas plugin that adds directive syntax support. A
// directive defines a custom token sequence (open and optional close)
// that triggers an action callback to transform the parsed content.
//
// It follows the standard tabnas plugin shape — a Plugin value that reads
// its configuration from the option map passed to j.Use. Recognised keys:
//
//	"name"   string      — directive/rule name (required)
//	"open"   string      — open token source (required)
//	"close"  string      — optional close token source
//	"action" Action | TokenAction | string — content transform callback
//	         (all forms accepted by DirectiveOptions.Action)
//	"rules"  *RulesOption — rules to modify; omit for defaults
//	"custom" CustomFunc   — extra setup callback
//
// Most callers use the typed Apply constructor rather than calling this
// directly.
var Directive tabnas.Plugin = func(j *tabnas.Tabnas, opts map[string]any) error {
	name, _ := opts["name"].(string)
	open, _ := opts["open"].(string)
	close_, _ := opts["close"].(string)
	action := resolveAction(j, opts["action"])
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

	// The open token must not already be registered. (The TypeScript
	// plugin throws here; the idiomatic Go port returns an error, which
	// j.Use propagates to the caller — the plugin never panics.)
	cfg := j.Config()
	if _, exists := cfg.FixedTokens[open]; exists {
		return fmt.Errorf("Directive open token already in use: %s", open)
	}

	// Register the open fixed token.
	openTN := "#OD_" + name
	OPEN := j.Token(openTN, open)

	// Register or look up the close fixed token.
	var CLOSE tabnas.Tin = -1
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
	ref := map[tabnas.FuncRef]any{}

	// Auto-wired state actions on the directive rule (@<name>-bo, @<name>-bc).
	ref[tabnas.FuncRef("@"+name+"-bo")] = tabnas.StateAction(
		func(r *tabnas.Rule, ctx *tabnas.Context) {
			r.Node = make(map[string]any)
		},
	)
	ref[tabnas.FuncRef("@"+name+"-bc")] = tabnas.StateAction(
		func(r *tabnas.Rule, ctx *tabnas.Context) {
			// Follow the replacement chain to get the final child node.
			// When a val rule is replaced by a list rule (implicit list),
			// the original child's Node may be stale in Go because slice
			// append can reallocate. Walk the Prev-linked replacement
			// chain to find the last replacement and adopt its Node.
			if r.Child != nil && r.Child != tabnas.NoRule {
				final := r.Child
				for final.Next != nil && final.Next != tabnas.NoRule &&
					final.Next.Prev == final {
					final = final.Next
				}
				if final != r.Child {
					r.Child.Node = final.Node
				}
			}
			if action != nil {
				out := action(r, ctx)
				// An action may return a token (TS: `if (out?.isToken)
				// return out` in the bc hook). The TS engine then halts
				// the parse only when the token carries an error code;
				// mirror that by setting ctx.ParseErr for error tokens
				// and ignoring plain tokens.
				if tkn, ok := out.(*tabnas.Token); ok && tkn != nil && tkn.Err != "" {
					ctx.ParseErr = tkn
				}
			}
		},
	)

	// Declarative grammar spec built up below and applied via j.Grammar().
	gs := &tabnas.GrammarSpec{
		Ref:  ref,
		Rule: map[string]*tabnas.GrammarRuleSpec{},
	}
	ruleFor := func(rn string) *tabnas.GrammarRuleSpec {
		if existing, ok := gs.Rule[rn]; ok {
			return existing
		}
		r := &tabnas.GrammarRuleSpec{}
		gs.Rule[rn] = r
		return r
	}

	// ---- Modify existing rules for OPEN token detection ----

	for rulename, rulemod := range openRules {
		rn := rulename
		rm := rulemod

		var openAlts []*tabnas.GrammarAltSpec
		var closeAlts []*tabnas.GrammarAltSpec

		if hasClose {
			// OPEN+CLOSE (empty directive) must be tried before OPEN alone.
			openAlts = append(openAlts, &tabnas.GrammarAltSpec{
				S: openTN + " " + closeTN,
				B: 1,
				P: name,
				N: map[string]int{"dr_" + name: 1},
				G: "start,end",
			})
			closeAlts = append(closeAlts, &tabnas.GrammarAltSpec{
				S: closeTN,
				B: 1,
				G: "end",
			})
		}

		openAlt := &tabnas.GrammarAltSpec{
			S: openTN,
			P: name,
			N: map[string]int{"dr_" + name: 1},
			G: "start",
		}
		if rm.C != nil {
			cref := tabnas.FuncRef("@dr-open-c-" + name + "-" + rn)
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

			closeCRef := tabnas.FuncRef("@dr-close-c-" + name + "-" + rn)
			ref[closeCRef] = tabnas.AltCond(
				func(r *tabnas.Rule, ctx *tabnas.Context) bool {
					if r.N["dr_"+name] != 1 {
						return false
					}
					if rm.C != nil {
						return rm.C(r, ctx)
					}
					return true
				},
			)
			commaCRef := tabnas.FuncRef("@dr-close-ca-c-" + name + "-" + rn)
			ref[commaCRef] = tabnas.AltCond(
				func(r *tabnas.Rule, ctx *tabnas.Context) bool {
					return r.N["dr_"+name] == 1
				},
			)

			closeAlts := []*tabnas.GrammarAltSpec{
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

	var dirOpen []*tabnas.GrammarAltSpec
	if hasClose {
		// Check for immediate close (empty directive).
		dirOpen = append(dirOpen, &tabnas.GrammarAltSpec{
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
	dirOpen = append(dirOpen, &tabnas.GrammarAltSpec{
		P: "val",
		N: counters,
	})

	var dirClose []*tabnas.GrammarAltSpec
	if hasClose {
		dirClose = []*tabnas.GrammarAltSpec{
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
	j.Rule(name, func(rs *tabnas.RuleSpec, _ *tabnas.Parser) {
		rs.Clear()
	})

	// Apply grammar with 'directive' group tag appended to every alt.
	setting := &tabnas.GrammarSetting{
		Rule: &tabnas.GrammarSettingRule{
			Alt: &tabnas.GrammarSettingAlt{G: "directive"},
		},
	}
	if err := j.Grammar(gs, setting); err != nil {
		return err
	}

	// ---- Custom callback ----

	if custom != nil {
		closeTin := tabnas.Tin(-1)
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
