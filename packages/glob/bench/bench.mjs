// Benchmark for @rust-gear/glob vs fast-glob.
//
// Usage: node bench/bench.mjs
// fast-glob comparison is skipped when the package is not installed.
//
// A deterministic synthetic tree (~50k files) is generated into
// bench/fixture on first run so results are reproducible across machines.

import { mkdirSync, writeFileSync, existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const { globSync, glob } = require('../index.js')

let fg = null
try {
  fg = require('fast-glob')
} catch {
  console.log('fast-glob not installed — skipping comparison\n')
}

const benchDir = dirname(fileURLToPath(import.meta.url))
const fixture = join(benchDir, 'fixture')

function generateFixture() {
  if (existsSync(join(fixture, '.done'))) return
  console.log('generating fixture tree (~50k files)...')
  const exts = ['js', 'ts', 'rs', 'json', 'css', 'md']
  for (let a = 0; a < 20; a++) {
    for (let b = 0; b < 10; b++) {
      for (let c = 0; c < 10; c++) {
        const dir = join(fixture, `mod${a}`, `sub${b}`, `pkg${c}`)
        mkdirSync(dir, { recursive: true })
        for (let f = 0; f < 25; f++) {
          writeFileSync(join(dir, `file${f}.${exts[f % exts.length]}`), '')
        }
      }
    }
  }
  writeFileSync(join(fixture, '.done'), '')
}

// Same shape as the main fixture but laid out like a real git repo:
// a .git marker at the root and a .gitignore in every mod*/ directory.
const fixtureGit = join(benchDir, 'fixture-git')

function generateGitFixture() {
  if (existsSync(join(fixtureGit, '.done'))) return
  console.log('generating git-like fixture tree (~50k files)...')
  const exts = ['js', 'ts', 'rs', 'json', 'css', 'md']
  mkdirSync(join(fixtureGit, '.git'), { recursive: true })
  writeFileSync(join(fixtureGit, '.gitignore'), 'dist/\n*.log\n')
  for (let a = 0; a < 20; a++) {
    mkdirSync(join(fixtureGit, `mod${a}`), { recursive: true })
    writeFileSync(join(fixtureGit, `mod${a}`, '.gitignore'), '*.md\n')
    for (let b = 0; b < 10; b++) {
      for (let c = 0; c < 10; c++) {
        const dir = join(fixtureGit, `mod${a}`, `sub${b}`, `pkg${c}`)
        mkdirSync(dir, { recursive: true })
        for (let f = 0; f < 25; f++) {
          writeFileSync(join(dir, `file${f}.${exts[f % exts.length]}`), '')
        }
      }
    }
  }
  writeFileSync(join(fixtureGit, '.done'), '')
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
