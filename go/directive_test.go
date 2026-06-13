/* Copyright (c) 2021-2025 Richard Rodger and other contributors, MIT License */

package directive

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"strings"
	"testing"

	jsonic "github.com/jsonicjs/jsonic/go"
)

// --- TSV spec loader ---
// Format:
//   <input>\t<expected-json>
//   <input>\t!error <regex>
// Blank lines and lines starting with # are ignored.

type specCase struct {
	Input    string
	Expected string
}

func loadSpec(t *testing.T, name string) []specCase {
	t.Helper()
	p := filepath.Join("..", "test", "spec", name)
	f, err := os.Open(p)
	if err != nil {
		t.Fatalf("open %s: %v", p, err)
	}
	defer f.Close()

	var cases []specCase
	scanner := bufio.NewScanner(f)
	scanner.Buffer(make([]byte, 1024*1024), 1024*1024)
	for scanner.Scan() {
		line := strings.TrimSuffix(scanner.Text(), "\r")
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		i := strings.Index(line, "\t")
		if i < 0 {
			continue
		}
		cases = append(cases, specCase{
			Input:    line[:i],
			Expected: line[i+1:],
		})
	}
	if err := scanner.Err(); err != nil {
		t.Fatalf("scan %s: %v", p, err)
	}
	return cases
}

// numberToFloat64 recursively converts json.Number values to float64 so
// a parsed spec tree deep-equals a jsonic-parsed tree (which uses float64
// for all numbers). Not needed for encoding/json's default behavior, but
// kept here for clarity — jsonic also returns float64 for numbers.
func normalizeSpec(v any) any {
	switch x := v.(type) {
	case map[string]any:
		out := make(map[string]any, len(x))
		for k, vv := range x {
			out[k] = normalizeSpec(vv)
		}
		return out
	case []any:
		out := make([]any, len(x))
		for i, vv := range x {
			out[i] = normalizeSpec(vv)
		}
		return out
	}
	return v
}

func runSpec(t *testing.T, j *jsonic.Jsonic, name string) {
	t.Helper()
	cases := loadSpec(t, name)
	for _, c := range cases {
		if strings.HasPrefix(c.Expected, "!error ") {
			pattern := c.Expected[len("!error "):]
			re, err := regexp.Compile(pattern)
			if err != nil {
				t.Errorf("%q: bad regex %q: %v", c.Input, pattern, err)
				continue
			}
			_, err = j.Parse(c.Input)
			if err == nil {
				t.Errorf("%q: expected error matching %q, got nil", c.Input, pattern)
				continue
			}
			if !re.MatchString(err.Error()) {
				t.Errorf("%q: error %q does not match %q",
					c.Input, err.Error(), pattern)
			}
			continue
		}

		var want any
		if err := json.Unmarshal([]byte(c.Expected), &want); err != nil {
			t.Errorf("%q: bad expected JSON %q: %v", c.Input, c.Expected, err)
			continue
		}
		want = normalizeSpec(want)

		got, err := j.Parse(c.Input)
		if err != nil {
			t.Errorf("%q: parse error: %v", c.Input, err)
			continue
		}
		if !reflect.DeepEqual(got, want) {
			t.Errorf("%q:\n  got:  %#v\n  want: %#v", c.Input, got, want)
		}
	}
}

// mustPanic asserts that fn panics. Used for duplicate-open-token tests.
func mustPanic(t *testing.T, fn func()) {
	t.Helper()
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic, got none")
		}
	}()
	fn()
}

// --- tests ---

func TestHappy(t *testing.T) {
	j := jsonic.Make()
	Apply(j, DirectiveOptions{
		Name: "upper",
		Open: "@",
		Action: func(rule *jsonic.Rule, ctx *jsonic.Context) {
			rule.Node = strings.ToUpper(fmt.Sprintf("%v", rule.Child.Node))
		},
	})

	runSpec(t, j, "happy.tsv")
}

func TestClose(t *testing.T) {
	j := jsonic.Make()
	Apply(j, DirectiveOptions{
		Name:  "foo",
		Open:  "foo<",
		Close: ">",
		Action: func(rule *jsonic.Rule, ctx *jsonic.Context) {
			rule.Node = "FOO"
		},
	})

	runSpec(t, j, "close-foo.tsv")

	// Register a second directive sharing the same close token.
	Apply(j, DirectiveOptions{
		Name:  "bar",
		Open:  "bar<",
		Close: ">",
		Action: func(rule *jsonic.Rule, ctx *jsonic.Context) {
			rule.Node = "BAR"
		},
	})

	runSpec(t, j, "close-foo-bar.tsv")

	// Duplicate open token should panic.
	mustPanic(t, func() {
		Apply(j, DirectiveOptions{
			Name:   "baz",
			Open:   "bar<",
			Action: func(rule *jsonic.Rule, ctx *jsonic.Context) {},
		})
	})
}

func TestAdder(t *testing.T) {
	j := jsonic.Make()
	Apply(j, DirectiveOptions{
		Name:  "adder",
		Open:  "add<",
		Close: ">",
		Action: func(rule *jsonic.Rule, ctx *jsonic.Context) {
			if arr, ok := rule.Child.Node.([]any); ok && len(arr) > 0 {
				// If any element is a string, concatenate; otherwise sum.
				allNum := true
				for _, v := range arr {
					if _, ok := v.(float64); !ok {
						allNum = false
						break
					}
				}
				if allNum {
					var out float64
					for _, v := range arr {
						out += v.(float64)
					}
					rule.Node = out
					return
				}
				var out string
				for _, v := range arr {
					out += fmt.Sprintf("%v", v)
				}
				rule.Node = out
				return
			}
			rule.Node = float64(0)
		},
	})

	runSpec(t, j, "adder.tsv")

	Apply(j, DirectiveOptions{
		Name:  "multiplier",
		Open:  "mul<",
		Close: ">",
		Action: func(rule *jsonic.Rule, ctx *jsonic.Context) {
			if arr, ok := rule.Child.Node.([]any); ok && len(arr) > 0 {
				out := 1.0
				for _, v := range arr {
					if n, ok := v.(float64); ok {
						out *= n
					}
				}
				rule.Node = out
				return
			}
			rule.Node = float64(0)
		},
	})

	runSpec(t, j, "multiplier.tsv")

	// Adder still works after second registration.
	runSpec(t, j, "adder.tsv")
}

func TestEdges(t *testing.T) {
	j := jsonic.Make()
	Apply(j, DirectiveOptions{
		Name:   "none",
		Open:   "@",
		Action: func(rule *jsonic.Rule, ctx *jsonic.Context) {},
		Rules:  &RulesOption{}, // Empty rules: no existing rules modified.
	})

	_, err := j.Parse("a:@x")
	if err == nil {
		t.Fatal("expected error for a:@x with empty rules")
	}
}

func TestInject(t *testing.T) {
	src := map[string]any{
		"a":  "A",
		"b":  map[string]any{"b": float64(1)},
		"bb": map[string]any{"bb": float64(1)},
		"c":  []any{float64(2), float64(3)},
	}

	j := jsonic.Make()
	Apply(j, DirectiveOptions{
		Name: "inject",
		Open: "@",
		Rules: &RulesOption{
			Open: map[string]*RuleMod{"val": {}, "pair": {}},
		},
		Action: func(rule *jsonic.Rule, ctx *jsonic.Context) {
			srcname := fmt.Sprintf("%v", rule.Child.Node)
			val := src[srcname]
			if rule.Parent != nil && rule.Parent.Name == "pair" {
				if m, ok := rule.Parent.Node.(map[string]any); ok {
					if sm, ok := val.(map[string]any); ok {
						for k, v := range sm {
							m[k] = v
						}
						return
					}
				}
			}
			rule.Node = val
		},
		Custom: func(j *jsonic.Jsonic, cfg DirectiveConfig) {
			OPEN := cfg.OPEN
			name := cfg.Name
			j.Rule("val", func(rs *jsonic.RuleSpec, _ *jsonic.Parser) {
				rs.PrependOpen(&jsonic.AltSpec{
					S: [][]jsonic.Tin{{OPEN}},
					C: func(r *jsonic.Rule, ctx *jsonic.Context) bool {
						return r.D == 0
					},
					P: "map",
					B: 1,
					N: map[string]int{name + "_top": 1},
				})
			})
			j.Rule("map", func(rs *jsonic.RuleSpec, _ *jsonic.Parser) {
				rs.PrependOpen(&jsonic.AltSpec{
					S: [][]jsonic.Tin{{OPEN}},
					C: func(r *jsonic.Rule, ctx *jsonic.Context) bool {
						return r.D == 1 && r.N[name+"_top"] == 1
					},
					P: "pair",
					B: 1,
				})
			})
		},
	})

	runSpec(t, j, "inject.tsv")
}

func TestAnnotate(t *testing.T) {
	j := jsonic.Make()
	Apply(j, DirectiveOptions{
		Name:  "annotate",
		Open:  "@",
		Rules: &RulesOption{Open: map[string]*RuleMod{"val": {}}},
		Action: func(rule *jsonic.Rule, ctx *jsonic.Context) {
			rule.Parent.U["note"] = "<" + fmt.Sprintf("%v", rule.Child.Node) + ">"
		},
		Custom: func(j *jsonic.Jsonic, cfg DirectiveConfig) {
			name := cfg.Name
			j.Rule(name, func(rs *jsonic.RuleSpec, _ *jsonic.Parser) {
				rs.PrependClose(&jsonic.AltSpec{
					R: "val",
					G: "replace",
				})
				rs.AddAC(func(rule *jsonic.Rule, ctx *jsonic.Context) {
					if rule.Parent != nil && rule.Parent != jsonic.NoRule &&
						rule.Next != nil && rule.Next != jsonic.NoRule {
						rule.Parent.Child = rule.Next
					}
				})
			})
			j.Rule("val", func(rs *jsonic.RuleSpec, _ *jsonic.Parser) {
				rs.AddBC(func(r *jsonic.Rule, ctx *jsonic.Context) {
					if note, ok := r.U["note"]; ok && note != nil {
						if m, ok := r.Node.(map[string]any); ok {
							m["@"] = note
						}
					}
				})
			})
		},
	})

	runSpec(t, j, "annotate.tsv")
}

func TestSubobj(t *testing.T) {
	j := jsonic.Make()
	Apply(j, DirectiveOptions{
		Name: "subobj",
		Open: "@",
		Rules: &RulesOption{
			Open: map[string]*RuleMod{
				"val": {},
				"pair": {
					C: func(r *jsonic.Rule, ctx *jsonic.Context) bool {
						return r.Lte("pk", 0)
					},
				},
			},
		},
		Action: func(rule *jsonic.Rule, ctx *jsonic.Context) {
			key := fmt.Sprintf("%v", rule.Child.Node)
			val := strings.ToUpper(key)
			res := map[string]any{key: val}

			// Merge into grandparent node if it's a map.
			if rule.Parent != nil && rule.Parent != jsonic.NoRule &&
				rule.Parent.Parent != nil && rule.Parent.Parent != jsonic.NoRule {
				if m, ok := rule.Parent.Parent.Node.(map[string]any); ok {
					for k, v := range res {
						m[k] = v
					}
					return
				}
			}
			rule.Node = res
		},
		Custom: func(j *jsonic.Jsonic, cfg DirectiveConfig) {
			OPEN := cfg.OPEN
			name := cfg.Name

			// Handle @foo at top level: assume a map.
			j.Rule("val", func(rs *jsonic.RuleSpec, _ *jsonic.Parser) {
				rs.PrependOpen(
					&jsonic.AltSpec{
						S: [][]jsonic.Tin{{OPEN}},
						C: func(r *jsonic.Rule, ctx *jsonic.Context) bool {
							return r.N["pk"] > 0
						},
						B: 1,
						G: name + "-undive",
					},
					&jsonic.AltSpec{
						S: [][]jsonic.Tin{{OPEN}},
						C: func(r *jsonic.Rule, ctx *jsonic.Context) bool {
							return r.D == 0
						},
						P: "map",
						B: 1,
						N: map[string]int{name + "_top": 1},
						G: name + "-top",
					},
				)
			})

			j.Rule("map", func(rs *jsonic.RuleSpec, _ *jsonic.Parser) {
				rs.PrependOpen(&jsonic.AltSpec{
					S: [][]jsonic.Tin{{OPEN}},
					C: func(r *jsonic.Rule, ctx *jsonic.Context) bool {
						return r.D == 1 && r.N[name+"_top"] == 1
					},
					P: "pair",
					B: 1,
					G: name + "-top",
				})
				rs.PrependClose(&jsonic.AltSpec{
					S: [][]jsonic.Tin{{OPEN}},
					C: func(r *jsonic.Rule, ctx *jsonic.Context) bool {
						return r.N["pk"] > 0
					},
					B: 1,
					G: name + "-undive",
				})
			})

			j.Rule("pair", func(rs *jsonic.RuleSpec, _ *jsonic.Parser) {
				rs.PrependClose(&jsonic.AltSpec{
					S: [][]jsonic.Tin{{OPEN}},
					C: func(r *jsonic.Rule, ctx *jsonic.Context) bool {
						return r.N["pk"] > 0
					},
					B: 1,
					G: name + "-undive",
				})
			})
		},
	})

	runSpec(t, j, "subobj.tsv")
}
