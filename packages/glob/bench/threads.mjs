// Thread-count sweep for the glob walk.
//
// Usage: node bench/threads.mjs
//
// The pool size is decided once per process, so each thread count runs in its
// own child with RUST_GEAR_GLOB_THREADS set. That variable is also the knob
// for a machine whose optimum differs from the built-in policy.
//
// What to look for: the walk is kernel-bound, so wall time does not simply
// keep falling with more threads. On a platform whose directory lookup path
// serializes (macOS/APFS), it bottoms out and then *regresses* — on Apple
// Silicon the floor sits exactly at the performance-core count. On a platform
// that scales, the curve should keep improving up to the logical CPU count.
//
// A curve that keeps improving to the full width of the machine means
// WalkWidth::Full is right for it; one that bottoms out earlier means the
// pool wants capping there.

import { execFileSync } from 'node:child_process'
import { availableParallelism } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { fixture, generateFixture } from './fixture.mjs'

const benchDir = dirname(fileURLToPath(import.meta.url))
const pattern = process.argv[2] ?? '**/*.js'

// A missing fixture would not fail here, it would just glob nothing and
// report a flat curve, so generate it up front.
generateFixture()

const child = `
  const { globSync } = require(${JSON.stringify(join(benchDir, '..', 'index.js'))})
  const opts = { cwd: ${JSON.stringify(fixture)} }
  for (let i = 0; i < 3; i++) globSync(${JSON.stringify(pattern)}, opts)
  let best = Infinity
  for (let i = 0; i < 10; i++) {
    const start = process.hrtime.bigint()
    const found = globSync(${JSON.stringify(pattern)}, opts)
    const ms = Number(process.hrtime.bigint() - start) / 1e6
    if (ms < best) best = ms
    if (i === 0) process.stdout.write(found.length + ' ')
  }
  process.stdout.write(String(best))
`

const cpus = availableParallelism()
const counts = []
for (let n = 1; n <= cpus * 2; n = n < 8 ? n + 1 : n * 2) counts.push(n)

console.log(`pattern ${pattern}   availableParallelism=${cpus}\n`)
let floor = Infinity
const rows = []
for (const threads of counts) {
  const out = execFileSync(process.execPath, ['-e', child], {
    env: { ...process.env, RUST_GEAR_GLOB_THREADS: String(threads) },
    encoding: 'utf8',
  })
  const [found, ms] = out.trim().split(' ')
  rows.push([threads, Number(ms), Number(found)])
  floor = Math.min(floor, Number(ms))
}

for (const [threads, ms, found] of rows) {
  const bar = '#'.repeat(Math.round((floor / ms) * 40))
  const mark = ms === floor ? '  <- fastest' : ''
  console.log(
    `threads ${String(threads).padStart(3)}  ${ms.toFixed(2).padStart(8)} ms  ${bar.padEnd(40)}${mark}  (${found} files)`
  )
}

const best = rows.find(([, ms]) => ms === floor)[0]
const last = rows[rows.length - 1][1]
console.log(
  `\nfastest at ${best} threads; ${last.toFixed(2)} ms at ${rows[rows.length - 1][0]} threads ` +
    `(${(last / floor).toFixed(2)}x the floor).`
)
console.log(
  last / floor > 1.15
    ? 'Regression past the optimum: this machine is contention-bound, so capping the pool helps.'
    : 'No meaningful regression: the full width of the machine is right here.'
)
