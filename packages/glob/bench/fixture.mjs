// Deterministic synthetic trees (~50k files each) shared by the benchmarks.
//
// Generated on first use and left in place; the `.done` marker makes repeat
// runs free. Both trees are gitignored, so nothing here reaches the repo.

import { mkdirSync, writeFileSync, existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const benchDir = dirname(fileURLToPath(import.meta.url))

export const fixture = join(benchDir, 'fixture')
export const fixtureGit = join(benchDir, 'fixture-git')

const exts = ['js', 'ts', 'rs', 'json', 'css', 'md']

function fillTree(root) {
  for (let a = 0; a < 20; a++) {
    for (let b = 0; b < 10; b++) {
      for (let c = 0; c < 10; c++) {
        const dir = join(root, `mod${a}`, `sub${b}`, `pkg${c}`)
        mkdirSync(dir, { recursive: true })
        for (let f = 0; f < 25; f++) {
          writeFileSync(join(dir, `file${f}.${exts[f % exts.length]}`), '')
        }
      }
    }
  }
}

export function generateFixture() {
  if (existsSync(join(fixture, '.done'))) return
  console.log('generating fixture tree (~50k files)...')
  fillTree(fixture)
  writeFileSync(join(fixture, '.done'), '')
}

// Same shape as the main fixture but laid out like a real git repo:
// a .git marker at the root and a .gitignore in every mod*/ directory.
export function generateGitFixture() {
  if (existsSync(join(fixtureGit, '.done'))) return
  console.log('generating git-like fixture tree (~50k files)...')
  mkdirSync(join(fixtureGit, '.git'), { recursive: true })
  writeFileSync(join(fixtureGit, '.gitignore'), 'dist/\n*.log\n')
  for (let a = 0; a < 20; a++) {
    mkdirSync(join(fixtureGit, `mod${a}`), { recursive: true })
    writeFileSync(join(fixtureGit, `mod${a}`, '.gitignore'), '*.md\n')
  }
  fillTree(fixtureGit)
  writeFileSync(join(fixtureGit, '.done'), '')
}
