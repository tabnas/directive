const { Jsonic, Debug } = require('@jsonic/jsonic-next')
const { Directive } = require('..')

// const j = Jsonic.make().use(Debug,{trace:true}).use(Directive, {
//   name: 'constant',
//   open: '@',
//   action: (rule) => rule.node = (''+rule.child.node).toUpperCase()
// })

// console.log(j('@a'))
// console.log(j('[@a]'))
// console.log(j('[1, @a]'))
// console.log(j('[1, @a, 2]'))
// console.log(j('[@a, 2]'))
// console.log(j('[@a, @b]'))
// console.log(j('{x:@a}'))
// console.log(j('{y:1, x:@a}'))
// console.log(j('{y:1, x:@a, z:2}'))
// console.log(j('{x:@a, z:2}'))

// console.log(j('[1 @a]'))
// console.log(j('[1 @a 2]'))
// console.log(j('[@a 2]'))

// console.log(j('{ y:1 x:@a z:2 }'))

// console.log(j('1 2 @a'))
// console.log(j('1 @a'))
// console.log(j('1 @a 2'))
// console.log(j('@a 2'))
// console.log(j('@a @b'))
// console.log(j('@a @b 2'))

// console.log(j('@a,2'))

// const j = Jsonic.make().use(Debug,{trace:true}).use(Directive, {
//   name: 'constant',
//   open: '@',
//   rules: {
//     open: 'val,pair'
//   },
//   action: (rule) => {
//     console.log('DA', rule.d, rule.name, rule.child.node, rule.parent.name)
//     let from = rule.parent.name

//     if('pair' === from) {
//       rule.parent.use.pair=true
//       rule.parent.use.key='@'
//     }

//     rule.node = (''+rule.child.node).toUpperCase()
//   },
//   custom: (jsonic, {OPEN, name}) => {

//     // Handle special case of @foo first token - assume a map
//     jsonic
//       .rule('val', (rs) => {
// 	rs.open({
// 	  s: [OPEN],
// 	  c: (r)=>0===r.d,
// 	  p: 'map',
// 	  b: 1,
// 	  n: { [name+'_top']:1 }
// 	})
//       })
//       .rule('map', (rs) => {
// 	rs.open({
// 	  s: [OPEN],
// 	  c: (r)=> (1===r.d && 1===r.n[name+'_top']),
// 	  p: 'pair',
// 	  b: 1,
// 	})
//       })
//   }
// })

// // console.log(j('{x:@a,@b,z:@c}'))
// // console.log(j('{x:@a @b z:@c}'))
// // console.log(j('{x:1 @b @c z:2}'))
// // console.log(j('x:1 @a'))
// console.log(j('@a x:1'))

// const j = Jsonic.make().use(Debug,{trace:true}).use(Directive, {
//   name: 'adder',
//   open: 'add<',
//   close: '>',
//   action: (rule) => {
//     let out = 0
//     if (Array.isArray(rule.child.node)) {
//       out = rule.child.node.reduce((a, v) => a + v)
//     }
//     rule.node = out
//   }
// })

// console.log(j('add<1,2>'))

// const SRC = {
//   a: 'A',
//   b: {b:1},
//   c: [2,3],
// }

// const j = Jsonic.make().use(Debug,{trace:true}).use(Directive, {
//   name: 'constant',
//   open: '@',
//   rules: {
//     open: 'val,pair'
//   },
//   action: (rule) => {
//     // console.log('DA', rule.d, rule.name, rule.child.node, rule.parent.name)
//     let srcname = ''+rule.child.node
//     let src = SRC[srcname]
//     let from = rule.parent.name

//     if('pair' === from) {
//       Object.assign(rule.parent.node, src)
//     }
//     else {
//       rule.node = src
//     }
//   },
//   custom: (jsonic, {OPEN, name}) => {

//     // Handle special case of @foo first token - assume a map
//     jsonic
//       .rule('val', (rs) => {
// 	rs.open({
// 	  s: [OPEN],
// 	  c: (r)=>0===r.d,
// 	  p: 'map',
// 	  b: 1,
// 	  n: { [name+'_top']:1 }
// 	})
//       })
//       .rule('map', (rs) => {
// 	rs.open({
// 	  s: [OPEN],
// 	  c: (r)=> (1===r.d && 1===r.n[name+'_top']),
// 	  p: 'pair',
// 	  b: 1,
// 	})
//       })
//   }
// })

// console.log(j('a:@a'))
// console.log(j('c:@c'))
// console.log(j('a:@a c:@c'))
// console.log(j('@b'))
// console.log(j('a:1 @b'))
// console.log(j('a:1 @b c:2'))
// console.log(j('b:@b'))

// console.log(j('a:@[1]'))

const j = Jsonic.make()
  .use(Debug, { trace: true })
  .use(Directive, {
    name: 'annotate',
    open: '@',
    rules: {
      open: 'val',
    },
    action: (rule) => {
      // console.log('DA', rule.d, rule.name, rule.child.node, rule.parent.name)
      rule.parent.use.note = '<' + rule.child.node + '>'
    },
    custom: (jsonic, { OPEN, name }) => {
      jsonic
        .rule('annotate', (rs) => {
          rs.close([
            {
              r: 'val',
              g: 'replace',
            },
          ]).ac((r, c, next) => {
            console.log('AC PARENT', r.parent.i, r.parent.name)
            console.log('AC NEXT', next.i, next.name)
            r.parent.child = next
          })
        })
        .rule('val', (rs) => {
          rs.bc((r) => {
            if (r.use.note) {
              r.node['@'] = r.use.note
            }
          })
        })
    },
  })

console.log(j('[@a {x:1}]'))
