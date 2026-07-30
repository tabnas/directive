# Agents Guide — shared spec fixtures

`spec/*.tsv` holds the cross-runtime conformance fixtures. Both runtimes run
the same files, so a change here affects TypeScript and Go together — edit
with that in mind.

## Format

Tab-separated, one case per line. Lines starting with `#` are comments —
each file opens with a block describing the directive it exercises.

    <input>	<expected-json>
    <input>	!error <regex>

`input` is source for the mini test grammar, with `\n` `\r` `\t` `\\`
decoded. `expected-json` is the parse result; `!error <regex>` marks input
that must fail with a message matching the regex.

## Who runs what

- TypeScript: `ts/test/directive.test.ts`.
- Go: `go/directive_test.go`.

Both name the same files. A fixture that only one runtime runs proves
nothing, so wire a new file into both.

## Rules

- Prefer adding a fixture here over a one-off in-language assertion when a
  case is expressible as input → output. That is what keeps the two runtimes
  honest against each other.
- TypeScript is canonical. If the two runtimes disagree, the TS behaviour is
  the expected value — unless Go has exposed a genuine TS defect, in which
  case fix TS first and pin the corrected behaviour here.
- A new fixture must pass in BOTH runtimes: run `go test ./...` (from `go/`)
  and `npm test` (from `ts/`) before considering it done.
