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

// Use `ant` on both Unix and Windows — `where ant` on Windows resolves
// PATHEXT, finding ant.exe / ant.cmd / ant.bat as installed. The
// Chocolatey shim is `ant.exe`, not `ant.bat`, so an explicit `.bat`
// here would miss the choco-installed Ant on GitHub Windows runners.
const ant = process.env.TRANG_ANT || 'ant';
// TRANG_ANT may be an absolute path; commandExists is a PATH probe and
// would reject it. Accept either an existing file path or a PATH name.
if (!(path.isAbsolute(ant) && existsSync(ant)) && !commandExists(ant)) {
  exitWith(127, `\`${ant}\` not on PATH (and not an existing absolute path); install Apache Ant to build trang.jar`);
}

// `ant-jar` is jing-trang's default target — it compiles all modules
// (jing.jar, trang.jar, dtdinst.jar) into ${build.dir}. The legacy
// `trang.jar` target name does not exist in V20241231.
console.log(`[trang-native] running ant from ${sourceDir}`);
const result = spawnSync(ant, ['ant-jar'], { cwd: sourceDir, stdio: 'inherit' });
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

// resolver.jar is declared in trang.jar's manifest as Class-Path
// (Apache xml-resolver). `java -jar` looks it up next to trang.jar; we
// need it on disk in the same place so native-image's classpath build
// can pick it up.
const upstreamResolver = path.join(sourceDir, 'lib', 'resolver.jar');
if (existsSync(upstreamResolver)) {
  const outResolver = path.join(outDir, 'resolver.jar');
  copyFileSync(upstreamResolver, outResolver);
  console.log(`[trang-native] wrote ${outResolver}`);
} else {
  console.warn(`[trang-native] warning: ${upstreamResolver} not found; native-image build may fail to resolve xml-resolver classes`);
}

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
