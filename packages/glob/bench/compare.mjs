// Regression check: is this build slower than a reference build?
//
// Usage: node bench/compare.mjs <base.node> <head.node> [--max-regression=1.1]
//
// Both binaries are loaded into one process and run alternately on the same
// fixture, so the ratio between them is unaffected by how fast (or how busy)
// the machine is — only the relative numbers are claimed to mean anything.
// Each variant is scored by its *fastest* run, because noise on a shared
// machine only ever adds time.
//
// Exits non-zero if head is more than --max-regression times base on any
// case, or if the two builds disagree about which files match. It does not
// require an improvement: a build that is merely no slower passes.

import { appendFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { availableParallelism } from 'node:os'
import { resolve } from 'node:path'

import { fixture, fixtureGit, generateFixture, generateGitFixture } from './fixture.mjs'

const require = createRequire(import.meta.url)

const [basePath, headPath] = process.argv.slice(2).filter((a) => !a.startsWith('--'))
if (!basePath || !headPath) {
  console.error('usage: node bench/compare.mjs <base.node> <head.node> [--max-regression=1.1]')
  process.exit(2)
}

const maxRegression = Number(
  process.argv.find((a) => a.startsWith('--max-regression='))?.split('=')[1] ?? 1.1
)
const rounds = Number(process.env.BENCH_ROUNDS ?? 15)

// Resolved against the shell's cwd so a relative path works on every
// platform (a bash-style absolute path would not survive on Windows).
const base = require(resolve(basePath))
const head = require(resolve(headPath))

generateFixture()
generateGitFixture()

const cases = [
  ['**/*.js', (g) => g.globSync('**/*.js', { cwd: fixture })],
  ['**/*.{js,ts}', (g) => g.globSync('**/*.{js,ts}', { cwd: fixture })],
  [
    '**/* with exclude',
    (g) => g.globSync('**/*', { cwd: fixture, exclude: ['**/pkg5/**', '**/*.md'] }),
  ],
  ['mod1/**/*.rs (narrow base)', (g) => g.globSync('mod1/**/*.rs', { cwd: fixture })],
  ['absolute pattern', (g) => g.globSync(`${fixture}/**/*.js`, { cwd: fixture })],
  ['git tree, gitignore on', (g) => g.globSync('**/*', { cwd: fixtureGit })],
  ['git tree, gitignore off', (g) => g.globSync('**/*', { cwd: fixtureGit, gitignore: false })],
  ['async', (g) => g.glob('**/*.js', { cwd: fixture })],
]

async function time(fn, binding) {
  const start = process.hrtime.bigint()
  const result = await fn(binding)
  return [Number(process.hrtime.bigint() - start) / 1e6, result]
}

// The same set of paths, in the same order — a faster build that lost files
// is not a faster build.
function sameResult(a, b) {
  if (a.length !== b.length) return false
  return a.every((path, i) => path === b[i])
}

const rows = []
let regressed = 0
let mismatched = 0

for (const [label, fn] of cases) {
  const [, baseResult] = await time(fn, base)
  const [, headResult] = await time(fn, head)
  const agrees = sameResult([...baseResult].sort(), [...headResult].sort())
  if (!agrees) mismatched++

  for (let i = 0; i < 2; i++) {
    await time(fn, base)
    await time(fn, head)
  }

  let baseBest = Infinity
  let headBest = Infinity
  for (let round = 0; round < rounds; round++) {
    // Alternate which build goes first so neither keeps the warmer cache.
    const order = round % 2 === 0 ? [base, head] : [head, base]
    for (const binding of order) {
      const [ms] = await time(fn, binding)
      if (binding === base) baseBest = Math.min(baseBest, ms)
      else headBest = Math.min(headBest, ms)
    }
  }

  const ratio = headBest / baseBest
  if (ratio > maxRegression) regressed++
  rows.push({ label, baseBest, headBest, ratio, agrees, count: headResult.length })
}

const verdict = (row) =>
  !row.agrees ? 'RESULTS DIFFER' : row.ratio > maxRegression ? 'SLOWER' : 'ok'

console.log(`base: ${resolve(basePath)}`)
console.log(`head: ${resolve(headPath)}`)
console.log(
  `${process.platform}-${process.arch}, availableParallelism=${availableParallelism()}, ` +
    `${rounds} rounds per case (best of each), regression threshold ${maxRegression}x\n`
)
console.log(
  `${'case'.padEnd(28)}${'base'.padStart(10)}${'head'.padStart(10)}${'head/base'.padStart(12)}   status`
)
for (const row of rows) {
  console.log(
    row.label.padEnd(28) +
      `${row.baseBest.toFixed(2)} ms`.padStart(10) +
      `${row.headBest.toFixed(2)} ms`.padStart(10) +
      `${row.ratio.toFixed(3)}x`.padStart(12) +
      `   ${verdict(row)}`
  )
}

if (process.env.GITHUB_STEP_SUMMARY) {
  const lines = [
    `### glob walk: head vs base (${process.platform}-${process.arch})`,
    '',
    `Best of ${rounds} interleaved rounds. Threshold: fail above ${maxRegression}x.`,
    '',
    '| case | base | head | head/base | |',
    '| :--- | ---: | ---: | ---: | :--- |',
    ...rows.map(
      (row) =>
        `| \`${row.label}\` | ${row.baseBest.toFixed(2)} ms | ${row.headBest.toFixed(2)} ms | ` +
        `${row.ratio.toFixed(3)}x | ${verdict(row) === 'ok' ? '✅' : '❌ ' + verdict(row)} |`
    ),
    '',
  ]
  appendFileSync(process.env.GITHUB_STEP_SUMMARY, lines.join('\n'))
}

if (mismatched > 0) {
  console.error(`\n${mismatched} case(s) returned different files — not a like-for-like comparison.`)
  process.exit(1)
}
if (regressed > 0) {
  console.error(`\n${regressed} case(s) slower than ${maxRegression}x base.`)
  process.exit(1)
}
console.log('\nNo regression against base.')
