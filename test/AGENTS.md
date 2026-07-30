# Agents Guide — shared spec fixtures

`spec/*.tsv` holds the cross-runtime conformance fixtures. Both runtimes run
the same files, so a change here affects TypeScript and Go together — edit
with that in mind.

## Format

Tab-separated, one case per line. Blank lines and lines whose first character
is `#` are skipped — each file opens with a comment block describing the
directive it exercises.

    <input>	<expected-json>
    <input>	!error <regex>

Both loaders split at the **first** tab only, so a tab inside `expected` is
kept. `expected` is either a JSON value (the parse result) or `!error `
followed by a regular expression the error message must match.

**The `input` column is used verbatim.** Neither loader decodes escape
sequences, so `\n` in a fixture reaches the parser as backslash-then-n, not a
newline. A case that needs a real control character cannot be written here
today; add it as an in-language test instead, or teach *both* loaders the
same decoding first.

A trailing `\r` is stripped, so the files work with either line ending.

## Who runs what

- TypeScript: `ts/test/directive.test.ts` (`loadSpec`).
- Go: `go/directive_test.go` (`loadSpec`).

Both name the same files. A fixture only one runtime runs proves nothing, so
wire a new file into both.

## Rules

- Prefer adding a fixture here over a one-off in-language assertion when a
  case is expressible as input → output. That is what keeps the two runtimes
  honest against each other.
- TypeScript is canonical. If the two runtimes disagree, the TS behaviour is
  the expected value — unless Go has exposed a genuine TS defect, or the
  difference is one of the intentional divergences the root `AGENTS.md`
  records, which stay out of these shared fixtures.
- A new fixture must pass in BOTH runtimes before it counts:
  `go test ./...` from `go/`, and **`npm run build && npm test`** from `ts/`.
  Plain `npm test` runs the previously compiled `dist-test/`, so it can pass
  without ever loading a newly added fixture.
