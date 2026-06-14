.PHONY: all build test clean deps build-ts test-ts build-go test-go \
        publish-go tags-go tidy-go reset

# The TypeScript implementation in ts/ is canonical. The Go
# implementation in go/ is kept at parity with it. `all` builds and
# tests both.
#
# The only dependency is the tabnas parser engine, consumed from its
# GitHub main branch. The `parser` target downloads and builds it into
# vendor/ (git-ignored); build and test targets depend on it. The tests
# bring their own small grammar (ts/test/mini-grammar.ts,
# go/mini_grammar_test.go).

all: build test

parser:
	./scripts/fetch-parser.sh

build: build-ts build-go

test: test-ts test-go

clean:
	cd ts && npm run clean || true
	cd go && go clean -cache
	rm -rf vendor

# TypeScript (canonical)
build-ts: parser
	cd ts && npm install && npm run build

test-ts: build-ts
	cd ts && npm test

# Go (parity). Go consumes the engine source directly, so it skips the
# TypeScript engine build.
build-go:
	TABNAS_SKIP_TS_BUILD=1 ./scripts/fetch-parser.sh
	cd go && go build ./...

test-go: build-go
	cd go && go vet ./... && go test ./...

# Publish Go module: make publish-go V=0.1.5
publish-go: test-go
	@test -n "$(V)" || (echo "Usage: make publish-go V=x.y.z" && exit 1)
	sed -i 's/^const Version = ".*"/const Version = "$(V)"/' go/directive.go
	git add go/directive.go
	git commit -m "go: v$(V)"
	git tag go/v$(V)
	git push origin main go/v$(V)

tidy-go:
	cd go && go mod tidy

tags-go:
	git tag -l 'go/v*' --sort=-version:refname

reset:
	cd ts && npm run reset
	cd go && go clean -cache && go build ./... && go test ./...
