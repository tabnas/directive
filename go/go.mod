module github.com/tabnas/directive/go

go 1.24.7

require github.com/jsonicjs/jsonic/go v0.0.0

// The jsonic relaxed-JSON grammar (a self-contained parser, built on the
// tabnas engine) is consumed from source — it is not published to a
// module proxy. scripts/fetch-deps.sh downloads its GitHub main branch
// into ./vendor; this replace points the require there.
replace github.com/jsonicjs/jsonic/go => ../vendor/tabnas-jsonic/go
