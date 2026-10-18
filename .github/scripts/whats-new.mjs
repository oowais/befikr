#!/usr/bin/env node
// "What's new" for the app: the last 10 releases' New / Fixes, from the same
// Conventional Commits as release-notes.sh (feat → New, fix/revert → Fixes),
// written to public/whats-new.json and bundled with the UI. Includes the
// version being built (VERSION, from next-version.sh / desktop-version.sh)
// when it isn't tagged yet. Usage: VERSION=0.37.0 node .github/scripts/whats-new.mjs
import { execFileSync } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'

const KEEP = 10
const git = (...args) => execFileSync('git', args, { encoding: 'utf8', maxBuffer: 64 << 20 })
const CC = /^([a-z]+)(\([^)]*\))?!?: (.+)$/

/** Change lines of a range: a squash merge's "* type: …" bullets, else its subject. */
function changes(range) {
  const out = []
  for (const commit of git('log', '--no-merges', '--format=%x00%s%n%b', range).split('\0').slice(1)) {
    const [subject, ...body] = commit.split('\n')
    const bullets = body.filter((l) => /^\* [a-z]+(\([^)]*\))?!?: /.test(l)).map((l) => l.slice(2))
    out.push(...(bullets.length ? bullets : [subject]).filter((l) => CC.test(l)))
  }
  return out
}

function release(version, date, range, before) {
  // A squash body also lists branch commits an earlier release already had.
  const seen = new Set(before ? changes(before) : [])
  const pick = (types) => [
    ...new Set(
      changes(range)
        .filter((l) => !seen.has(l))
        .map((l) => l.match(CC))
        .filter((m) => types.includes(m[1]))
        .map((m) => m[3].charAt(0).toUpperCase() + m[3].slice(1)),
    ),
  ]
  return { version, date, new: pick(['feat']), fixes: pick(['fix', 'revert']) }
}

const tags = git('tag', '--list', 'v0.*', '--sort=-v:refname', '--merged', 'HEAD').split('\n').filter(Boolean)
const date = (ref) => git('log', '-1', '--format=%cs', ref).trim()
const releases = []
const version = process.env.VERSION
if (version && !tags.includes(`v${version}`)) {
  releases.push(release(version, date('HEAD'), tags[0] ? `${tags[0]}..HEAD` : 'HEAD', tags[0]))
}
for (let i = 0; releases.length < KEEP && i < tags.length; i++) {
  const prev = tags[i + 1]
  releases.push(release(tags[i].slice(1), date(tags[i]), prev ? `${prev}..${tags[i]}` : tags[i], prev))
}

mkdirSync('public', { recursive: true })
const kept = releases.filter((r) => r.new.length || r.fixes.length)
writeFileSync('public/whats-new.json', JSON.stringify(kept, null, 1) + '\n')
console.log(`whats-new.json: ${kept.map((r) => r.version).join(', ')}`)
