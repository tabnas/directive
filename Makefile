.PHONY: all build test clean deps build-ts test-ts build-go test-go \
        publish-go tags-go tidy-go reset

# The TypeScript implementation in ts/ is canonical. The Go
# implementation in go/ is kept at parity with it. `all` builds and
# tests both.
#
# Both implementations consume source dependencies from GitHub main: the
# tabnas parser engine and the jsonic relaxed-JSON grammar. The `deps`
# target downloads and builds them into vendor/ (git-ignored); build and
# test targets depend on it.

all: build test

deps:
	./scripts/fetch-deps.sh

build: build-ts build-go

test: test-ts test-go

clean:
	cd ts && npm run clean || true
	cd go && go clean -cache
	rm -rf vendor

# TypeScript (canonical)
build-ts: deps
	cd ts && npm install && npm run build

test-ts: build-ts
	cd ts && npm test

# Go (parity). The Go grammar lives in the vendored engine, so the Go
# build needs no TypeScript build of the dependencies.
build-go:
	TABNAS_SKIP_TS_BUILD=1 ./scripts/fetch-deps.sh
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
