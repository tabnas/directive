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

	tabnas "github.com/tabnas/parser/go"
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

// normalizeSpec recurses through the expected tree; numbers from
// encoding/json are float64, matching the parser's number representation.
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

func runSpec(t *testing.T, j *tabnas.Tabnas, name string) {
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

// mustPanic asserts that fn panics. Used for the duplicate-open-token test.
func mustPanic(t *testing.T, fn func()) {
	t.Helper()
	defer func() {
		if r := recover(); r == nil {
			t.Fatal("expected panic, got none")
		}
	}()
	fn()
}

// --- tests (mirroring ts/test/directive.test.ts) ---

func TestHappy(t *testing.T) {
	j := makeMini()
	Apply(j, DirectiveOptions{
		Name: "upper",
		Open: "@",
		Action: func(rule *tabnas.Rule, ctx *tabnas.Context) {
			rule.Node = strings.ToUpper(fmt.Sprintf("%v", rule.Child.Node))
		},
	})

	runSpec(t, j, "happy.tsv")
}

func TestClose(t *testing.T) {
	j := makeMini()
	Apply(j, DirectiveOptions{
		Name:  "foo",
		Open:  "foo<",
		Close: ">",
		Action: func(rule *tabnas.Rule, ctx *tabnas.Context) {
			rule.Node = "FOO"
		},
	})

	runSpec(t, j, "close-foo.tsv")

	// The close token also terminates an enclosing list/map opened inside
	// the directive (boundary closing).
	runSpec(t, j, "close-boundary.tsv")

	// A second directive sharing the same close token ">".
	Apply(j, DirectiveOptions{
		Name:  "bar",
		Open:  "bar<",
		Close: ">",
		Action: func(rule *tabnas.Rule, ctx *tabnas.Context) {
			rule.Node = "BAR"
		},
	})

	runSpec(t, j, "close-foo-bar.tsv")

	// Re-registering the same open token must panic.
	mustPanic(t, func() {
		Apply(j, DirectiveOptions{
			Name:   "baz",
			Open:   "bar<",
			Action: func(rule *tabnas.Rule, ctx *tabnas.Context) {},
		})
	})
}

func TestAdder(t *testing.T) {
	j := makeMini()
	Apply(j, DirectiveOptions{
		Name:  "adder",
		Open:  "add<",
		Close: ">",
		Action: func(rule *tabnas.Rule, ctx *tabnas.Context) {
			out := float64(0)
			if arr, ok := rule.Child.Node.([]any); ok {
				for _, v := range arr {
					if n, ok := v.(float64); ok {
						out += n
					}
				}
			}
			rule.Node = out
		},
	})

	runSpec(t, j, "adder.tsv")

	Apply(j, DirectiveOptions{
		Name:  "multiplier",
		Open:  "mul<",
		Close: ">",
		Action: func(rule *tabnas.Rule, ctx *tabnas.Context) {
			out := float64(0)
			if arr, ok := rule.Child.Node.([]any); ok && len(arr) > 0 {
				out = 1
				for _, v := range arr {
					if n, ok := v.(float64); ok {
						out *= n
					}
				}
			}
			rule.Node = out
		},
	})

	runSpec(t, j, "multiplier.tsv")

	// Adder still works after the second registration.
	runSpec(t, j, "adder.tsv")
}

func TestInject(t *testing.T) {
	src := map[string]any{
		"a": "A",
		"b": map[string]any{"b": float64(1)},
		"c": []any{float64(2), float64(3)},
	}

	j := makeMini()
	Apply(j, DirectiveOptions{
		Name: "inject",
		Open: "@",
		Rules: &RulesOption{
			Open: map[string]*RuleMod{"val": {}, "pair": {}},
		},
		Action: func(rule *tabnas.Rule, ctx *tabnas.Context) {
			key := fmt.Sprintf("%v", rule.Child.Node)
			val := src[key] // missing key → nil
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
	})

	runSpec(t, j, "inject.tsv")
}

func TestEdges(t *testing.T) {
	// An explicit empty RulesOption modifies no host rules, so the open
	// token is unrecognised.
	j := makeMini()
	Apply(j, DirectiveOptions{
		Name:   "none",
		Open:   "@",
		Action: func(rule *tabnas.Rule, ctx *tabnas.Context) {},
		Rules:  &RulesOption{},
	})

	if _, err := j.Parse("[@a]"); err == nil {
		t.Fatal("expected error for [@a] with empty rules")
	}
}

func TestResolveRulesNilEntry(t *testing.T) {
	// A nil RuleMod entry is normalized to an empty &RuleMod{}.
	got := resolveRules(map[string]*RuleMod{"val": nil})
	if got["val"] == nil {
		t.Fatal("resolveRules: nil entry was not replaced")
	}
}

func TestCoverageExtras(t *testing.T) {
	openCond := 0
	closeCond := 0
	customName := ""

	j := makeMini()
	Apply(j, DirectiveOptions{
		Name:  "cov",
		Open:  "cov<",
		Close: ">",
		Action: func(rule *tabnas.Rule, ctx *tabnas.Context) {
			rule.Node = "COV"
		},
		Rules: &RulesOption{
			// "elem" appears in both Open and Close, so the second lookup
			// reuses the existing grammar-rule spec.
			Open: map[string]*RuleMod{
				"val": {C: func(r *tabnas.Rule, ctx *tabnas.Context) bool {
					openCond++
					return true
				}},
				"elem": {},
			},
			Close: map[string]*RuleMod{
				"list": {},
				"elem": {C: func(r *tabnas.Rule, ctx *tabnas.Context) bool {
					closeCond++
					return true
				}},
				"map":  {},
				"pair": {},
			},
		},
		Custom: func(_ *tabnas.Tabnas, cfg DirectiveConfig) {
			customName = cfg.Name
		},
	})

	if customName != "cov" {
		t.Fatalf("custom callback name = %q, want %q", customName, "cov")
	}

	got, err := j.Parse("cov<[1, 2>")
	if err != nil || got != "COV" {
		t.Fatalf("cov<[1, 2> => %#v err=%v", got, err)
	}
	if openCond == 0 {
		t.Fatal("open condition was never evaluated")
	}
	if closeCond == 0 {
		t.Fatal("close condition was never evaluated")
	}
}
