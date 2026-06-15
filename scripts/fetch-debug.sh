#!/usr/bin/env bash
# Fetch and build the @tabnas/debug plugin from its GitHub main branch.
#
# @tabnas/debug (github.com/tabnas/debug) is a development-only
# diagnostic: a `describe()` grammar dump and parse tracing, handy when a
# directive's alts aren't matching as expected. It is NOT a dependency of
# the directive — the directive's only dependency is the tabnas parser
# engine — so this script vendors it into ./vendor (git-ignored) purely
# for local diagnostics, and (optionally) links it into ts/node_modules
# so `import { Debug } from '@tabnas/debug'` resolves.
#
# Run scripts/fetch-parser.sh first: the debug plugin builds against the
# same vendored engine (it picks it up via a symlink created below).
#
# Re-run to refresh to the latest main. Pin a different ref with
# TABNAS_DEBUG_REF. Set TABNAS_SKIP_TS_BUILD=1 to skip the TS build, and
# TABNAS_SKIP_TS_LINK=1 to skip linking it into ts/node_modules.
set -euo pipefail

REF="${TABNAS_DEBUG_REF:-main}"
REPO="tabnas/debug"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/vendor/tabnas-debug"
PARSER="$ROOT/vendor/tabnas-parser"

if [ ! -d "$PARSER" ]; then
  echo "fetch-debug: vendor/tabnas-parser missing — run scripts/fetch-parser.sh first" >&2
  exit 1
fi

echo "fetch-debug: downloading $REPO@$REF ..."
rm -rf "$DEST"
mkdir -p "$DEST"
curl -fsSL --retry 4 --retry-delay 2 --max-time 120 \
  "https://codeload.github.com/$REPO/tar.gz/refs/heads/$REF" \
  | tar xz -C "$DEST" --strip-components=1

# The debug plugin consumes the engine from source the same way this repo
# does (TS: file:../vendor/tabnas-parser/ts; Go: replace => ../vendor/
# tabnas-parser/go). Point those at the engine already vendored here.
mkdir -p "$DEST/vendor"
ln -sfn ../../tabnas-parser "$DEST/vendor/tabnas-parser"

if [ "${TABNAS_SKIP_TS_BUILD:-0}" = "1" ]; then
  echo "fetch-debug: skipping TypeScript build (TABNAS_SKIP_TS_BUILD=1)"
else
  echo "fetch-debug: building TypeScript debug plugin ..."
  ( cd "$DEST/ts" && npm install --no-audit --no-fund && npm run build )

  if [ "${TABNAS_SKIP_TS_LINK:-0}" != "1" ] && [ -d "$ROOT/ts/node_modules" ]; then
    echo "fetch-debug: linking @tabnas/debug into ts/node_modules ..."
    # --no-save: keep it out of package.json (it is not a dependency).
    # It shares the directive's single `tabnas` via its peerDependency.
    ( cd "$ROOT/ts" && npm install --no-save --no-audit --no-fund "$DEST/ts" )
  fi
fi

echo "fetch-debug: done -> $DEST"
