import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const archivePath = resolve(workspaceRoot, 'dist/packages/cem-ml-npm/package.tgz');
const packageMetadata = JSON.parse(readFileSync(resolve(projectRoot, 'package.json'), 'utf8'));
const consumerRoot = mkdtempSync(join(tmpdir(), 'cem-ml-consumer-'));

try {
  writeFileSync(
    resolve(consumerRoot, 'package.json'),
    `${JSON.stringify({ name: 'cem-ml-clean-consumer', private: true, type: 'module' }, null, 2)}\n`,
  );
  run(process.platform === 'win32' ? 'npm.cmd' : 'npm', [
    'install',
    archivePath,
    '--ignore-scripts',
    '--no-audit',
    '--no-fund',
  ]);

  const installedRoot = resolve(consumerRoot, 'node_modules/@epa-wg/cem-ml');
  const installedMetadata = JSON.parse(readFileSync(resolve(installedRoot, 'package.json'), 'utf8'));
  assert.equal(installedMetadata.name, packageMetadata.name);
  assert.equal(installedMetadata.version, packageMetadata.version);
  assert.equal(installedMetadata.bin, undefined);
  assert.deepEqual(installedMetadata.dependencies, undefined);
  assert.match(readFileSync(resolve(installedRoot, 'LICENSE'), 'utf8'), /^MIT License/);

  writeFileSync(
    resolve(consumerRoot, 'probe-node.mjs'),
    `import * as runtime from '@epa-wg/cem-ml/wasm';
import { readFile } from 'node:fs/promises';

const runtimeUrl = new URL(import.meta.resolve('@epa-wg/cem-ml/runtime.json'));
const integrityUrl = new URL(import.meta.resolve('@epa-wg/cem-ml/integrity.json'));
const schemaUrl = new URL(
  import.meta.resolve('@epa-wg/cem-ml/schema-packages/cem-ml/v1/package.cem'),
);
const runtimeMetadata = JSON.parse(await readFile(runtimeUrl, 'utf8'));
const integrity = JSON.parse(await readFile(integrityUrl, 'utf8'));
const schemaPackage = await readFile(schemaUrl, 'utf8');
console.log(JSON.stringify({
  version: runtime.version(),
  commonVersion: runtimeMetadata.commonVersion,
  abiIdentity: runtimeMetadata.abi.identity,
  integrityAlgorithm: integrity.algorithm,
  schemaPresent: schemaPackage.includes('{package @id="cem-ml"'),
}));
`,
  );
  const nodeProbe = JSON.parse(capture(process.execPath, ['probe-node.mjs']));
  assert.equal(nodeProbe.version, packageMetadata.version);
  assert.equal(nodeProbe.commonVersion, packageMetadata.version);
  assert.match(nodeProbe.abiIdentity, /^wasm-bindgen@/);
  assert.equal(nodeProbe.integrityAlgorithm, 'sha256');
  assert.equal(nodeProbe.schemaPresent, true);

  writeFileSync(
    resolve(consumerRoot, 'probe-browser.mjs'),
    `import init, { version } from '@epa-wg/cem-ml/wasm';
import { readFile } from 'node:fs/promises';

const packageUrl = import.meta.resolve('@epa-wg/cem-ml/package.json');
const wasm = await readFile(new URL('./dist/wasm/browser/cem_ml_bg.wasm', packageUrl));
await init({ module_or_path: wasm });
console.log(JSON.stringify({ version: version() }));
`,
  );
  const browserProbe = JSON.parse(
    capture(process.execPath, ['--conditions=browser', 'probe-browser.mjs']),
  );
  assert.equal(browserProbe.version, packageMetadata.version);

  const integrity = JSON.parse(readFileSync(resolve(installedRoot, 'dist/integrity.json'), 'utf8'));
  assert.equal(integrity.algorithm, 'sha256');
  assert.ok(integrity.files.some((entry) => entry.path === 'wasm/browser/cem_ml_bg.wasm'));
  assert.ok(integrity.files.some((entry) => entry.path === 'wasm/node/cem_ml_bg.wasm'));
  for (const entry of integrity.files) {
    const bytes = readFileSync(resolve(installedRoot, 'dist', entry.path));
    assert.equal(entry.bytes, bytes.byteLength, `installed byte length drift: ${entry.path}`);
    assert.equal(
      entry.sha256,
      createHash('sha256').update(bytes).digest('hex'),
      `installed SHA-256 drift: ${entry.path}`,
    );
  }

  console.log(
    `Clean consumer verified ${installedMetadata.name}@${installedMetadata.version}: Node/browser initialization, schema assets, ABI metadata, and integrity records.`,
  );
} finally {
  assert.ok(consumerRoot.startsWith(`${tmpdir()}/cem-ml-consumer-`));
  rmSync(consumerRoot, { recursive: true, force: true });
}

function run(command, args) {
  const result = spawnSync(command, args, { cwd: consumerRoot, stdio: 'inherit' });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with status ${result.status}`);
  }
}

function capture(command, args) {
  const result = spawnSync(command, args, { cwd: consumerRoot, encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(' ')} failed:\n${result.stderr || result.stdout || result.error}`,
    );
  }
  return result.stdout.trim();
}
