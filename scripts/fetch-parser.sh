#!/usr/bin/env bash
# Fetch and build the tabnas parser engine from its GitHub main branch.
#
# The directive is a plugin for the tabnas parser engine
# (github.com/tabnas/parser — npm package `tabnas`, Go module
# github.com/tabnas/parser/go), and that engine is its only dependency.
# The engine is not published to a registry, so it is consumed from
# source: this script downloads its main branch over HTTPS into ./vendor
# (git-ignored) and builds the TypeScript engine so its dist/ is
# importable. The tests bring their own small grammar (see
# ts/test/mini-grammar.ts and go/mini_grammar_test.go), so no grammar
# package is fetched.
#
# Re-run it to refresh to the latest main. Pin a different ref with
# TABNAS_PARSER_REF. Set TABNAS_SKIP_TS_BUILD=1 to skip the TypeScript
# build (Go-only).
set -euo pipefail

REF="${TABNAS_PARSER_REF:-main}"
REPO="tabnas/parser"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/vendor/tabnas-parser"

echo "fetch-parser: downloading $REPO@$REF ..."
rm -rf "$DEST"
mkdir -p "$DEST"
curl -fsSL --retry 4 --retry-delay 2 --max-time 120 \
  "https://codeload.github.com/$REPO/tar.gz/refs/heads/$REF" \
  | tar xz -C "$DEST" --strip-components=1

if [ "${TABNAS_SKIP_TS_BUILD:-0}" = "1" ]; then
  echo "fetch-parser: skipping TypeScript engine build (TABNAS_SKIP_TS_BUILD=1)"
else
  echo "fetch-parser: building TypeScript engine ..."
  ( cd "$DEST/ts" && npm install --no-audit --no-fund && npm run build )
fi

echo "fetch-parser: done -> $DEST"
