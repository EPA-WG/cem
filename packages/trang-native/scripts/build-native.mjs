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
  captureReflectionMetadata({ jarPath, packageRoot });
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

// Classic mode (not `-jar`): trang.jar's manifest writes Main-Class
// with `/` separators ("com/thaiopensource/.../Driver"), which
// native-image rejects in -jar mode. Pass classpath + main class
// explicitly. resolver.jar (declared as Class-Path in the manifest)
// must be included for xml-resolver classes to link.
const cpSep = process.platform === 'win32' ? ';' : ':';
const resolverJar = path.join(packageRoot, 'build', 'resolver.jar');
const classpath = existsSync(resolverJar)
  ? `${jarPath}${cpSep}${resolverJar}`
  : jarPath;
niArgs.push('-cp', classpath);
niArgs.push('com.thaiopensource.relaxng.translate.Driver');

console.log(`[trang-native] native-image build for ${target}`);
console.log(`[trang-native] command: ${nativeImage} ${niArgs.join(' ')}`);
// On Windows, GraalVM ships `native-image.cmd` (a batch shim).
// spawnSync can't execute .cmd/.bat directly — it returns status:null
// immediately. Route through cmd.exe explicitly (safer than shell:true
// since args bypass shell parsing).
const isCmdShim = process.platform === 'win32' && /\.(cmd|bat)$/i.test(nativeImage);
const result = isCmdShim
  ? spawnSync('cmd.exe', ['/c', nativeImage, ...niArgs], { cwd: outDir, stdio: 'inherit' })
  : spawnSync(nativeImage, niArgs, { cwd: outDir, stdio: 'inherit' });
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

function captureReflectionMetadata({ jarPath, packageRoot }) {
  // Run Trang under the agent across every conversion direction so the
  // resulting reflect-/resource-config.json covers the full CLI surface.
  // Uses `config-merge-dir` so each successive run extends the same
  // config files rather than overwriting them.
  const java = resolveJava();
  if (!java) {
    exitWith(127, 'java not found on PATH; cannot run native-image-agent');
  }
  const agentDir = path.join(packageRoot, 'reflect-config', '.agent-scratch');
  mkdirSync(agentDir, { recursive: true });

  // resolver.jar sits next to trang.jar in build/; trang.jar's manifest
  // Class-Path picks it up automatically when invoked via -jar.
  const conversions = prepareConversionFixtures(packageRoot);
  for (const [index, conversion] of conversions.entries()) {
    const firstRun = index === 0;
    const agentOpt = firstRun
      ? `config-output-dir=${agentDir}`
      : `config-merge-dir=${agentDir}`;
    console.log(
      `[trang-native] capture ${index + 1}/${conversions.length}: ${conversion.label}`,
    );
    const result = spawnSync(
      java,
      [`-agentlib:native-image-agent=${agentOpt}`, '-jar', jarPath, ...conversion.args],
      { stdio: 'inherit' },
    );
    if (result.status !== 0) {
      exitWith(
        result.status ?? 1,
        `native-image-agent run failed for ${conversion.label} (exit ${result.status})`,
      );
    }
  }
  console.log(
    `[trang-native] wrote merged agent config to ${agentDir}.\n` +
      `[trang-native] Review and move into reflect-config/ (drop .agent-scratch/) before committing.`,
  );
  process.exit(0);
}

function prepareConversionFixtures(packageRoot) {
  // Lay out a representative pair of schemas covering the conversion
  // graph. The .rnc seed is the source of truth; we let Trang produce
  // the other formats on disk first, then exercise every direction.
  const fixtureDir = path.join(packageRoot, 'build', 'agent-fixture');
  mkdirSync(fixtureDir, { recursive: true });
  const rncPath = path.join(fixtureDir, 'fixture.rnc');
  const rngPath = path.join(fixtureDir, 'fixture.rng');
  const xsdPath = path.join(fixtureDir, 'fixture.xsd');
  const dtdPath = path.join(fixtureDir, 'fixture.dtd');
  const rncOut = path.join(fixtureDir, 'fixture.out.rnc');
  const rngOut = path.join(fixtureDir, 'fixture.out.rng');

  if (!existsSync(rncPath)) {
    // Schema deliberately exercises elements, attributes, repetition,
    // optional groups, and a typed datatype so the agent observes the
    // datatype-library and resolver code paths.
    const rnc = [
      'default namespace = "https://example.com/ns"',
      'datatypes xsd = "http://www.w3.org/2001/XMLSchema-datatypes"',
      '',
      'start = element root {',
      '  attribute id { xsd:ID },',
      '  attribute version { xsd:string }?,',
      '  element child {',
      '    attribute name { text },',
      '    text',
      '  }*',
      '}',
      '',
    ].join('\n');
    writeFileSync(rncPath, rnc, 'utf8');
  }

  return [
    { label: 'rnc → rng', args: [rncPath, rngPath] },
    { label: 'rng → rnc', args: [rngPath, rncOut] },
    { label: 'rnc → xsd', args: [rncPath, xsdPath] },
    { label: 'rng → xsd', args: [rngPath, xsdPath] },
    { label: 'rnc → dtd', args: [rncPath, dtdPath] },
    { label: 'rng → dtd', args: [rngPath, dtdPath] },
    // Trang accepts rng|rnc|dtd|xml as input formats — XSD is output-only.
    { label: 'dtd → rng', args: [dtdPath, rngOut] },
  ];
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
