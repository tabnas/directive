# Agents Guide — shared spec fixtures

`spec/*.tsv` holds the cross-runtime conformance fixtures. All three
runtimes run the same files, so a change here affects TypeScript, Go and
Rust together — edit with that in mind.

## Format

Tab-separated, one case per line. Blank lines and lines whose first character
is `#` are skipped — each file opens with a comment block describing the
directive it exercises.

    <input>	<expected-json>
    <input>	ERROR:<code>

`expected` is either a JSON value (the parse result) or `ERROR:` followed
by the error's CODE, compared exactly. It used to be `!error ` followed by
a regular expression matched against the message — every such row said
`!error unexpected`, and `unexpected` turns out to be the code the engine
actually answers, so the rows now pin the code and are read by the same
`ERROR:` contract as every other tabnas fixture.

Escapes are the shared codec's: `\n`, `\r`, `\t` and `\\` are decoded in
the `input` column, and every other backslash sequence survives verbatim.
The loaders used to decode nothing at all, so a case needing a real
control character could not be written; it can now. No fixture cell
changes meaning — none of them contains a backslash.

Rows are split on every tab, and the columns are positional (these files
have no header line — each opens with a comment block describing the
directive it exercises). No row has a tab inside `expected`; one that did
would now be a third column rather than part of the second.

A trailing `\r` is stripped, so the files work with either line ending.

## Who runs what

- TypeScript: `ts/test/directive.test.ts` — `runSpec(j, name)`, a
  `makeRunner(...)` over the file.
- Go: `go/directive_test.go` — `runSpec(t, j, name)`, a
  `support.Runner{...}` over the file.
- Rust: `rs/tests/directive_test.rs` — `run_spec(&parser, name)`, from
  `rs/tests/common/spec.rs`.

The TypeScript and Go runners are a handful of lines. Everything else —
finding `test/spec`, reading the file, decoding escapes, the `ERROR:`
contract, the comparison, the `<file>:<line>` in a failure message —
comes from [`@tabnas/support`](https://github.com/tabnas/support) and its
Go half, so those two loaders cannot drift from each other either.

**There is no Rust half of `@tabnas/support`,** so the Rust loader is
written out in `rs/tests/common/spec.rs`. It is the one loader that CAN
drift, and the codec below is the contract it has to keep: the same
comment and blank-line skipping, the same escapes, the same headerless
positional columns, the same `ERROR:` handling, the same `<file>:<line>`
in a failure. Change the codec and you change three files, not two. (The
Rust runner reports every failing row in a file rather than stopping at
the first — a presentation difference, not a codec one.)

A fixture is named by the test that supplies its directive, because what
varies per case is the DIRECTIVE and a directive is a function — it cannot
live in an `opts` column. So this is the one repo where a new file does
have to be wired into all three runtimes by hand. A fixture only one
runtime runs proves nothing.

In TypeScript and Go each ROW is its own test case, rather than one
assertion inside a per-file test, so a failure names the file and line it
came from. The Rust runner is one test per file but names the same
`<file>:<line>` for every row that failed.

## Rules

- Prefer adding a fixture here over a one-off in-language assertion when a
  case is expressible as input → output. That is what keeps the two runtimes
  honest against each other.
- TypeScript is canonical. If the runtimes disagree, the TS behaviour is
  the expected value — unless a port has exposed a genuine TS defect, or
  the difference is one of the intentional divergences the root
  `AGENTS.md` records, which stay out of these shared fixtures.
- A new fixture must pass in ALL THREE runtimes before it counts:
  `go test ./...` from `go/`, `cargo test --all-targets` from `rs/`, and
  **`npm run build && npm test`** from `ts/`. Plain `npm test` runs the
  previously compiled `dist-test/`, so it can pass without ever loading a
  newly added fixture.
