#!/usr/bin/env node
/**
 * Compile `trang.jar` into a native binary using GraalVM `native-image`.
 *
 * GraalVM does NOT cross-compile: this script always builds for the
 * host platform. The optional `--target <triple>` arg is a guardrail —
 * it errors out when the host doesn't match the requested triple, so
 * matrix-CI workflows fail fast on misconfigured runners.
 *
 * Honors:
 *   - `GRAALVM_HOME=/abs/path` — picks `<GRAALVM_HOME>/bin/native-image`
 *   - `TRANG_NATIVE_IMAGE=/abs/path/native-image` — overrides the binary
 *   - `--capture-reflect` — runs the JAR under `native-image-agent`
 *     against the canonical fixture to generate reflect-config seeds.
 *
 * Output:
 *   build/native/<target>/trang(.exe)
 */
import { existsSync, mkdirSync, copyFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const packageRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const jarPath = path.join(packageRoot, 'build', 'trang.jar');
const reflectConfigDir = path.join(packageRoot, 'reflect-config');

const args = process.argv.slice(2);
const targetArg = pickArg(args, '--target');
const captureReflect = args.includes('--capture-reflect');

const hostTarget = detectHostTarget();
if (targetArg && targetArg !== hostTarget) {
  exitWith(
    2,
    `--target ${targetArg} requested but host is ${hostTarget}. ` +
      `GraalVM native-image does not cross-compile; rerun on a ${targetArg} host ` +
      `(the matrix CI workflow handles this automatically).`,
  );
}
const target = targetArg || hostTarget;

if (!existsSync(jarPath)) {
  exitWith(
    1,
    `${jarPath} not found — run nx run @epa-wg/trang-native:build-jar first`,
  );
}

const nativeImage = resolveNativeImage();
if (!nativeImage) {
  exitWith(
    127,
    'native-image not found on PATH. ' +
      'Install GraalVM (https://www.graalvm.org/) and set GRAALVM_HOME, ' +
      'or set TRANG_NATIVE_IMAGE=/abs/path/native-image.',
  );
}

const outDir = path.join(packageRoot, 'build', 'native', target);
mkdirSync(outDir, { recursive: true });

if (captureReflect) {
  captureReflectionMetadata({ nativeImage, jarPath, packageRoot });
  process.exit(0);
}

const binaryName = target.startsWith('windows-') ? 'trang.exe' : 'trang';
const outBinary = path.join(outDir, binaryName);

const niArgs = [
  '--no-fallback',
  '--enable-url-protocols=file',
  `-H:IncludeResources=.*\\.properties`,
  `-H:IncludeResources=.*\\.xml`,
  `-H:Name=trang`,
];

if (existsSync(reflectConfigDir)) {
  // Pass the whole config dir; native-image accepts the convention
  // "<dir>/reflect-config.json", "<dir>/resource-config.json", etc.
  niArgs.push(`-H:ConfigurationFileDirectories=${reflectConfigDir}`);
}

niArgs.push('-jar', jarPath);
niArgs.push('trang');

console.log(`[trang-native] native-image build for ${target}`);
console.log(`[trang-native] command: ${nativeImage} ${niArgs.join(' ')}`);
const result = spawnSync(nativeImage, niArgs, { cwd: outDir, stdio: 'inherit' });
if (result.status !== 0) {
  exitWith(result.status ?? 1, `native-image failed (exit ${result.status})`);
}

// native-image produces the binary in the cwd with the -H:Name value.
const produced = path.join(outDir, binaryName);
if (!existsSync(produced)) {
  // Some native-image builds emit at a slightly different path; copy
  // from a small allowlist of known fallbacks.
  for (const candidate of ['trang', 'trang.exe']) {
    const alt = path.join(outDir, candidate);
    if (existsSync(alt) && alt !== outBinary) {
      copyFileSync(alt, outBinary);
      break;
    }
  }
}
if (!existsSync(outBinary)) {
  exitWith(1, `native-image succeeded but ${outBinary} is missing`);
}
console.log(`[trang-native] wrote ${outBinary}`);

function captureReflectionMetadata({ nativeImage, jarPath, packageRoot }) {
  // Run Trang under the agent against a representative input — the
  // canonical CEM-emitted .rng + .rnc pair if present; otherwise a
  // minimal smoke fixture.
  const java = resolveJava();
  if (!java) {
    exitWith(127, 'java not found on PATH; cannot run native-image-agent');
  }
  const agentDir = path.join(packageRoot, 'reflect-config', '.agent-scratch');
  mkdirSync(agentDir, { recursive: true });

  const fixture = resolveFixture(packageRoot);
  console.log(`[trang-native] capturing reflection with fixture: ${fixture.join(' ')}`);
  const result = spawnSync(
    java,
    [
      `-agentlib:native-image-agent=config-output-dir=${agentDir}`,
      '-jar',
      jarPath,
      ...fixture,
    ],
    { stdio: 'inherit' },
  );
  if (result.status !== 0) {
    exitWith(result.status ?? 1, `native-image-agent run failed (exit ${result.status})`);
  }
  console.log(
    `[trang-native] wrote agent config to ${agentDir}.\n` +
      `[trang-native] Review and merge into reflect-config/reflect-config.json before committing.`,
  );
  process.exit(0);
}

function resolveFixture(packageRoot) {
  // Minimal smoke: convert this README's snippet via a tiny inline .rnc.
  // The CI fixture-generation step replaces this with the
  // cem-ml-emitted canonical fixture.
  const fixtureDir = path.join(packageRoot, 'build', 'agent-fixture');
  mkdirSync(fixtureDir, { recursive: true });
  const rncPath = path.join(fixtureDir, 'smoke.rnc');
  const rngPath = path.join(fixtureDir, 'smoke.rng');
  if (!existsSync(rncPath)) {
    // Write a minimal .rnc on demand.
    const rnc = 'default namespace = "https://example.com/ns"\nstart = element root { empty }\n';
    writeFileSync(rncPath, rnc, 'utf8');
  }
  return [rncPath, rngPath];
}

function pickArg(argv, flag) {
  const idx = argv.indexOf(flag);
  if (idx < 0 || idx === argv.length - 1) return null;
  return argv[idx + 1];
}

function resolveNativeImage() {
  if (process.env.TRANG_NATIVE_IMAGE && existsSync(process.env.TRANG_NATIVE_IMAGE)) {
    return process.env.TRANG_NATIVE_IMAGE;
  }
  if (process.env.GRAALVM_HOME) {
    const candidate = path.join(
      process.env.GRAALVM_HOME,
      'bin',
      process.platform === 'win32' ? 'native-image.cmd' : 'native-image',
    );
    if (existsSync(candidate)) return candidate;
  }
  const probe = process.platform === 'win32'
    ? spawnSync('where', ['native-image'], { encoding: 'utf8' })
    : spawnSync('which', ['native-image'], { encoding: 'utf8' });
  if (probe.status === 0) {
    const first = probe.stdout.split(/\r?\n/)[0]?.trim();
    if (first) return first;
  }
  return null;
}

function resolveJava() {
  if (process.env.JAVA_HOME) {
    const candidate = path.join(
      process.env.JAVA_HOME,
      'bin',
      process.platform === 'win32' ? 'java.exe' : 'java',
    );
    if (existsSync(candidate)) return candidate;
  }
  const probe = process.platform === 'win32'
    ? spawnSync('where', ['java'], { encoding: 'utf8' })
    : spawnSync('which', ['java'], { encoding: 'utf8' });
  if (probe.status === 0) {
    const first = probe.stdout.split(/\r?\n/)[0]?.trim();
    if (first) return first;
  }
  return null;
}

function detectHostTarget() {
  const arch = process.arch === 'x64' ? 'x86_64'
    : process.arch === 'arm64' ? 'aarch64'
    : process.arch;
  const platform = process.platform === 'linux' ? 'linux'
    : process.platform === 'darwin' ? 'macos'
    : process.platform === 'win32' ? 'windows'
    : process.platform;
  return `${platform}-${arch}`;
}

function exitWith(code, msg) {
  console.error(`[trang-native] ${msg}`);
  process.exit(code);
}
