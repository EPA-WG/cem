#!/usr/bin/env node
/**
 * Build `trang.jar` from the upstream source tree (build/source).
 *
 * jing-trang's Ant target is `trang.jar` (produces `build/trang.jar`
 * relative to the upstream source root). This script invokes Ant and
 * copies the resulting JAR to `packages/trang-native/build/trang.jar`
 * so subsequent native-image builds have a stable input path.
 *
 * Requires Apache Ant + a JDK on PATH. Honors:
 *   - `JAVA_HOME=/abs/path` — Ant respects it automatically.
 *   - `TRANG_ANT=/abs/path/ant` — overrides the Ant binary lookup.
 */
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const packageRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const sourceDir = process.env.TRANG_SOURCE_DIR || path.join(packageRoot, 'build', 'source');
const outDir = path.join(packageRoot, 'build');
const outJar = path.join(outDir, 'trang.jar');

if (!existsSync(sourceDir)) {
  exitWith(
    1,
    `source dir ${sourceDir} not found — run nx run @epa-wg/trang-native:fetch-source first`,
  );
}

const ant = process.env.TRANG_ANT || (process.platform === 'win32' ? 'ant.bat' : 'ant');
if (!commandExists(ant)) {
  exitWith(127, `\`${ant}\` not on PATH; install Apache Ant to build trang.jar`);
}

console.log(`[trang-native] running ant from ${sourceDir}`);
const result = spawnSync(ant, ['trang.jar'], { cwd: sourceDir, stdio: 'inherit' });
if (result.status !== 0) {
  exitWith(result.status ?? 1, `ant trang.jar failed (exit ${result.status})`);
}

const upstreamJar = path.join(sourceDir, 'build', 'trang.jar');
if (!existsSync(upstreamJar)) {
  exitWith(1, `ant succeeded but ${upstreamJar} is missing — upstream layout changed?`);
}

mkdirSync(outDir, { recursive: true });
copyFileSync(upstreamJar, outJar);
console.log(`[trang-native] wrote ${outJar}`);

function commandExists(name) {
  const probe = process.platform === 'win32'
    ? spawnSync('where', [name], { encoding: 'utf8' })
    : spawnSync('command', ['-v', name], { encoding: 'utf8', shell: true });
  return probe.status === 0;
}

function exitWith(code, msg) {
  console.error(`[trang-native] ${msg}`);
  process.exit(code);
}
