#!/bin/bash
# SessionStart hook: prepare the repo so both implementations can build,
# lint, and test. The directive's source dependencies — the tabnas
# parser engine and the jsonic relaxed-JSON grammar — are not published
# to a registry, so they are fetched from GitHub main into vendor/ and
# the TypeScript/Go dependencies are wired to that copy.
#
# Runs only in Claude Code on the web (remote) sessions, which start from
# a fresh container. Safe to run repeatedly.
set -euo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$ROOT"

echo "session-start: fetching and building the tabnas parser + jsonic grammar ..."
./scripts/fetch-deps.sh

echo "session-start: installing TypeScript dependencies ..."
( cd ts && npm install --no-audit --no-fund )

echo "session-start: warming the Go build cache ..."
( cd go && go build ./... )

echo "session-start: ready."
