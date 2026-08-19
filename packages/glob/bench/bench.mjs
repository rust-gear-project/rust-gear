// Benchmark for @rust-gear/glob vs fast-glob.
//
// Usage: node bench/bench.mjs
// fast-glob comparison is skipped when the package is not installed.
//
// A deterministic synthetic tree (~50k files) is generated into
// bench/fixture on first run so results are reproducible across machines.
// See fixture.mjs.

import { createRequire } from 'node:module'

import { fixture, fixtureGit, generateFixture, generateGitFixture } from './fixture.mjs'

const require = createRequire(import.meta.url)
const { globSync, glob } = require('../index.js')

let fg = null
try {
  fg = require('fast-glob')
} catch {
  console.log('fast-glob not installed — skipping comparison\n')
}

async function bench(name, fn, { warmup = 3, runs = 10 } = {}) {
  for (let i = 0; i < warmup; i++) await fn()
  const times = []
  let count = 0
  for (let i = 0; i < runs; i++) {
    const start = process.hrtime.bigint()
    const result = await fn()
    times.push(Number(process.hrtime.bigint() - start) / 1e6)
    count = result.length
  }
  times.sort((x, y) => x - y)
  const median = times[Math.floor(times.length / 2)]
  const min = times[0]
  console.log(
    `${name.padEnd(46)} median ${median.toFixed(2).padStart(8)} ms   min ${min.toFixed(2).padStart(8)} ms   (${count} files)`
  )
  return median
}

generateFixture()
generateGitFixture()

const cases = [
  { label: '**/*.js', patterns: '**/*.js', options: {} },
  { label: '**/*.{js,ts}', patterns: ['**/*.{js,ts}'], options: {} },
  {
    label: '**/* with exclude',
    patterns: '**/*',
    options: { exclude: ['**/pkg5/**', '**/*.md'] },
  },
  { label: 'mod1/**/*.rs (narrow base)', patterns: 'mod1/**/*.rs', options: {} },
]

for (const { label, patterns, options } of cases) {
  console.log(`\n--- ${label} ---`)
  await bench(`globSync`, () => globSync(patterns, { cwd: fixture, ...options }))
  await bench(`glob (async)`, () => glob(patterns, { cwd: fixture, ...options }))
  if (fg) {
    await bench(`fast-glob sync`, () =>
      fg.sync(patterns, { cwd: fixture, ignore: options.exclude ?? [] })
    )
    await bench(`fast-glob async`, () =>
      fg(patterns, { cwd: fixture, ignore: options.exclude ?? [] })
    )
  }
}

console.log('\n--- git-like tree: **/*.{js,ts} (gitignore on/off) ---')
await bench(`gitignore: true (default)`, () =>
  globSync('**/*.{js,ts}', { cwd: fixtureGit })
)
await bench(`gitignore: false`, () =>
  globSync('**/*.{js,ts}', { cwd: fixtureGit, gitignore: false })
)
if (fg) {
  await bench(`fast-glob sync`, () => fg.sync('**/*.{js,ts}', { cwd: fixtureGit }))
  await bench(`fast-glob async`, () => fg('**/*.{js,ts}', { cwd: fixtureGit }))
}
