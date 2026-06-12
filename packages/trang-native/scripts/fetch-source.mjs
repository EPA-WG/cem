#!/usr/bin/env node
/**
 * Fetch the pinned upstream Trang source into `build/source/`.
 *
 * Reads `upstream.json` for the repository URL + ref. Honors:
 *   - `TRANG_REF=<sha-or-tag>` to override the pinned ref (dev hacks)
 *   - `TRANG_SOURCE_DIR=/abs/path` to use a pre-cloned copy (CI cache hit)
 *
 * Always idempotent: re-runs reset `build/source/` to the pinned ref.
 */
import { readFile } from 'node:fs/promises';
import { existsSync, rmSync, statSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const packageRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const buildDir = path.join(packageRoot, 'build');
const sourceDir = process.env.TRANG_SOURCE_DIR || path.join(buildDir, 'source');

const upstream = JSON.parse(
  await readFile(path.join(packageRoot, 'upstream.json'), 'utf8'),
);
const ref = process.env.TRANG_REF || upstream.ref;
if (!ref || typeof ref !== 'string') {
  exitWith(1, `upstream.json missing required string field "ref"`);
}

if (process.env.TRANG_SOURCE_DIR) {
  if (!existsSync(process.env.TRANG_SOURCE_DIR)) {
    exitWith(1, `TRANG_SOURCE_DIR does not exist: ${process.env.TRANG_SOURCE_DIR}`);
  }
  console.log(`[trang-native] using TRANG_SOURCE_DIR=${process.env.TRANG_SOURCE_DIR}`);
  process.exit(0);
}

if (!commandExists('git')) {
  exitWith(127, '`git` not on PATH; install git to fetch Trang source');
}

if (existsSync(sourceDir)) {
  if (!isGitWorkdir(sourceDir)) {
    console.log(`[trang-native] ${sourceDir} exists but is not a git workdir; removing and re-cloning`);
    rmSync(sourceDir, { recursive: true, force: true });
  } else {
    console.log(`[trang-native] reusing existing clone at ${sourceDir}`);
    run('git', ['fetch', '--all', '--tags', '--prune'], { cwd: sourceDir });
    run('git', ['reset', '--hard', ref], { cwd: sourceDir });
    run('git', ['clean', '-fdx'], { cwd: sourceDir });
    console.log(`[trang-native] checked out ${ref}`);
    process.exit(0);
  }
}

console.log(`[trang-native] cloning ${upstream.repository} @ ${ref}`);
run('git', ['clone', '--depth', '1', '--branch', ref, upstream.repository, sourceDir]);
const head = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: sourceDir, encoding: 'utf8' });
console.log(`[trang-native] cloned ${head.stdout.trim()}`);

function run(cmd, args, opts = {}) {
  const result = spawnSync(cmd, args, { stdio: 'inherit', ...opts });
  if (result.status !== 0) {
    exitWith(result.status ?? 1, `\`${cmd} ${args.join(' ')}\` failed (exit ${result.status})`);
  }
}

function commandExists(name) {
  const probe = process.platform === 'win32'
    ? spawnSync('where', [name], { encoding: 'utf8' })
    : spawnSync('command', ['-v', name], { encoding: 'utf8', shell: true });
  return probe.status === 0;
}

function isGitWorkdir(dir) {
  try {
    return statSync(path.join(dir, '.git')).isDirectory()
      || statSync(path.join(dir, '.git')).isFile(); // submodule-style .git file
  } catch {
    return false;
  }
}

function exitWith(code, msg) {
  console.error(`[trang-native] ${msg}`);
  process.exit(code);
}
