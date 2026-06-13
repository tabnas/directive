#!/usr/bin/env bash
# Fetch and build the directive's source dependencies from their GitHub
# main branches.
#
# The directive is a plugin for the tabnas parser engine, and its tests
# run against the relaxed-JSON ("jsonic") grammar. Neither dependency is
# published to a registry, so both are consumed from source:
#
#   - github.com/tabnas/parser  (npm `tabnas`, Go module
#     github.com/tabnas/parser/go) — the grammar-free engine. Provides
#     the plugin API types the directive is written against, and (Go
#     only) its relaxed-JSON grammar subpackage .../go/jsonic.
#   - github.com/tabnas/jsonic   (npm `jsonic`) — the relaxed-JSON
#     grammar layer for TypeScript, built on the engine. Used by the
#     TypeScript tests to obtain a grammar instance.
#
# This script downloads both main branches over HTTPS into ./vendor
# (git-ignored), points jsonic's engine dependency at the vendored
# engine, and builds the TypeScript packages so their dist/ is
# importable.
#
# Re-run it to refresh to the latest main. Pin different refs with
# TABNAS_PARSER_REF / TABNAS_JSONIC_REF. Set TABNAS_SKIP_TS_BUILD=1 to
# skip the TypeScript builds (Go-only; the Go grammar lives in the
# vendored engine and needs no TS build).
set -euo pipefail

PARSER_REF="${TABNAS_PARSER_REF:-main}"
JSONIC_REF="${TABNAS_JSONIC_REF:-main}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENDOR="$ROOT/vendor"
PARSER_DEST="$VENDOR/tabnas-parser"
JSONIC_DEST="$VENDOR/tabnas-jsonic"

fetch() {
  local repo="$1" ref="$2" dest="$3"
  echo "fetch-deps: downloading $repo@$ref ..."
  rm -rf "$dest"
  mkdir -p "$dest"
  curl -fsSL --retry 4 --retry-delay 2 --max-time 120 \
    "https://codeload.github.com/$repo/tar.gz/refs/heads/$ref" \
    | tar xz -C "$dest" --strip-components=1
}

fetch "tabnas/parser" "$PARSER_REF" "$PARSER_DEST"
fetch "tabnas/jsonic" "$JSONIC_REF" "$JSONIC_DEST"

# Point jsonic's engine dependency at the vendored parser (its committed
# package.json uses a sibling-checkout path that does not exist here).
sed -i.bak 's#"tabnas": *"file:[^"]*"#"tabnas": "file:../../tabnas-parser/ts"#' \
  "$JSONIC_DEST/ts/package.json"
rm -f "$JSONIC_DEST/ts/package.json.bak"

if [ "${TABNAS_SKIP_TS_BUILD:-0}" = "1" ]; then
  echo "fetch-deps: skipping TypeScript builds (TABNAS_SKIP_TS_BUILD=1)"
else
  echo "fetch-deps: building TypeScript engine (tabnas) ..."
  ( cd "$PARSER_DEST/ts" && npm install --no-audit --no-fund && npm run build )
  echo "fetch-deps: building TypeScript grammar (jsonic) ..."
  ( cd "$JSONIC_DEST/ts" && npm install --no-audit --no-fund && npm run build )
fi

echo "fetch-deps: done -> $VENDOR"
