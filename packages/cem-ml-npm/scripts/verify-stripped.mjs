import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
  mkdirSync,
} from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, relative, resolve, sep } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const defaultRoot = resolve(projectRoot, 'dist');
const strippedRoot = resolve(projectRoot, 'dist-stripped');
const defaultManifest = readJson(resolve(defaultRoot, 'cem-ml-runtime.json'));
const strippedManifest = readJson(resolve(strippedRoot, 'cem-ml-runtime.json'));

assert.equal(defaultManifest.featureProfile, 'debug-control');
assert.equal(strippedManifest.featureProfile, 'stripped');
assert.match(defaultManifest.abi.identity, /;profile=debug-control$/);
assert.match(strippedManifest.abi.identity, /;profile=stripped$/);
assert.notEqual(defaultManifest.abi.identity, strippedManifest.abi.identity);
assert.equal(defaultManifest.commonVersion, strippedManifest.commonVersion);
assert.deepEqual(defaultManifest.protocol, strippedManifest.protocol);

for (const runtime of ['browser', 'node']) {
  const defaultCapability = defaultManifest.capabilities[runtime];
  const strippedCapability = strippedManifest.capabilities[runtime];
  assert.equal(defaultCapability.debugControl.compiled, true);
  assert.equal(strippedCapability.debugControl.compiled, false);
  assert.equal(strippedCapability.debugControl.active, false);
  assert.equal(strippedCapability.debugControl.dapAdapterVersion, undefined);
  assert.equal(strippedCapability.debugControl.cemDebugRequestVersion, undefined);
  for (const control of [
    'pause',
    'source-breakpoints',
    'stepping',
    'suspended-inspection',
    'dap',
    'cem-debug-requests',
  ]) {
    const entry = strippedCapability.controls.find((candidate) => candidate.control === control);
    assert.equal(entry?.availability, 'unavailable', `${runtime} ${control} must be unavailable`);
    assert.equal(entry?.coverage, 'none', `${runtime} ${control} must have no stripped coverage`);
  }
  for (const control of ['root-cancellation', 'operation-handles', 'bounded-subscriptions']) {
    const entry = strippedCapability.controls.find((candidate) => candidate.control === control);
    assert.notEqual(entry?.availability, 'unavailable', `${runtime} ${control} must remain available`);
  }
}

const defaultDeclarations = readFileSync(
  resolve(defaultRoot, defaultManifest.artifacts.node.types),
  'utf8',
);
const strippedDeclarations = readFileSync(
  resolve(strippedRoot, strippedManifest.artifacts.node.types),
  'utf8',
);
const coreWasmBindings = [
  'initializeResumableOperationHost',
  'startResumableOperation',
  'pollResumableOperation',
  'acceptResumableOperationResult',
  'cancelResumableOperation',
  'replaceResumableOperationWorker',
  'executeOperationWork',
];
const debugWasmBindings = [
  'pauseResumableOperation',
  'acknowledgeResumableOperationStop',
  'continueResumableOperation',
  'stepResumableOperation',
];
for (const binding of coreWasmBindings) {
  assert.match(strippedDeclarations, new RegExp(`\\b${binding}\\b`));
}
for (const binding of debugWasmBindings) {
  assert.match(defaultDeclarations, new RegExp(`\\b${binding}\\b`));
  assert.doesNotMatch(strippedDeclarations, new RegExp(`\\b${binding}\\b`));
}

const defaultRuntime = createRequire(import.meta.url)(
  resolve(defaultRoot, defaultManifest.artifacts.node.module),
);
const strippedRuntime = createRequire(import.meta.url)(
  resolve(strippedRoot, strippedManifest.artifacts.node.module),
);
for (const binding of coreWasmBindings) assert.equal(typeof strippedRuntime[binding], 'function');
for (const binding of debugWasmBindings) {
  assert.equal(typeof defaultRuntime[binding], 'function');
  assert.equal(strippedRuntime[binding], undefined);
}
const rejectedDebugRequest = JSON.parse(
  strippedRuntime.capabilityManifest(
    JSON.stringify({
      runtime: 'wasm-node',
      targetIdentity: 'wasm32-unknown-unknown:nodejs',
      abiIdentity: strippedManifest.abi.identity,
      debugControlActive: true,
    }),
  ),
);
assert.equal(rejectedDebugRequest.error?.code, 'cem.capability.debug_control_unavailable');

verifyIntegrity(defaultRoot);
verifyIntegrity(strippedRoot);
const binaryComparisons = {};
for (const target of ['browser', 'node']) {
  const defaultWasm = statSync(
    resolve(defaultRoot, defaultManifest.artifacts[target].wasm),
  ).size;
  const strippedWasm = statSync(
    resolve(strippedRoot, strippedManifest.artifacts[target].wasm),
  ).size;
  assert.ok(strippedWasm < defaultWasm, `${target} stripped WASM must omit debugger code`);
  binaryComparisons[target] = { defaultWasm, strippedWasm };
}

const executableName = process.platform === 'win32' ? 'cem-ml.exe' : 'cem-ml';
const defaultCli = resolve(workspaceRoot, 'dist/target/cem_ml_cli/debug', executableName);
const strippedCli = resolve(
  workspaceRoot,
  'dist/target/cem_ml_cli_stripped/debug',
  executableName,
);
const defaultHelp = capture(defaultCli, ['--help']);
const strippedHelp = capture(strippedCli, ['--help']);
assert.match(defaultHelp, /^\s*debug\s+/m);
assert.doesNotMatch(strippedHelp, /^\s*debug\s+/m);
const strippedDebug = run(strippedCli, ['debug', '--stdio']);
assert.notEqual(strippedDebug.status, 0);
assert.match(`${strippedDebug.stderr}\n${strippedDebug.stdout}`, /unrecognized subcommand ['`]debug['`]/i);

if (process.platform !== 'win32') {
  const defaultSymbols = capture('nm', ['-C', defaultCli], 128 * 1024 * 1024);
  const strippedSymbols = capture('nm', ['-C', strippedCli], 128 * 1024 * 1024);
  assert.match(defaultSymbols, /cem_ml(?:::|_cli::)(?:dap|debug_control|debug_transport)::/);
  for (const symbol of [
    'cem_ml::dap::',
    'cem_ml::debug_control::',
    'cem_ml_cli::debug_transport::',
    'cem_ql::debug_control::',
  ]) {
    assert.ok(!strippedSymbols.includes(symbol), `stripped native symbol leaked: ${symbol}`);
  }
}

const featureTree = capture('cargo', [
  'tree',
  '--package',
  'cem-ml-cli',
  '--no-default-features',
  '--edges',
  'features',
]);
assert.doesNotMatch(featureTree, /feature "debug-control"/);
verifyCompileSurfaceAbsence();

const defaultCliBytes = statSync(defaultCli).size;
const strippedCliBytes = statSync(strippedCli).size;
assert.ok(strippedCliBytes < defaultCliBytes, 'stripped native CLI must omit debugger code');
binaryComparisons.native = { defaultCliBytes, strippedCliBytes };

console.log(
  `Verified stripped CEM-ML profile: debug APIs/transports/symbols absent, core controls retained, isolated ABI/integrity identities, binary sizes ${JSON.stringify(binaryComparisons)}.`,
);

function verifyCompileSurfaceAbsence() {
  const fixtureRoot = mkdtempSync(resolve(tmpdir(), 'cem-ml-stripped-surface-'));
  try {
    mkdirSync(resolve(fixtureRoot, 'src'));
    const dependencyPath = (path) => path.replaceAll('\\', '\\\\');
    writeFileSync(
      resolve(fixtureRoot, 'Cargo.toml'),
      `[package]\nname = "cem-ml-stripped-surface"\nversion = "0.0.0"\nedition = "2021"\n\n[workspace]\n\n[dependencies]\ncem-ml = { path = "${dependencyPath(resolve(workspaceRoot, 'packages/cem_ml'))}", default-features = false }\ncem-ql = { path = "${dependencyPath(resolve(workspaceRoot, 'packages/cem_ql'))}", default-features = false }\ncem-ml-transform-cem-ql = { path = "${dependencyPath(resolve(workspaceRoot, 'packages/cem_ml_transform_cem_ql'))}", default-features = false }\n`,
    );
    writeFileSync(
      resolve(fixtureRoot, 'src/main.rs'),
      `use cem_ml::dap::DapSession;\nuse cem_ml::debug_control::StopToken;\nuse cem_ml_transform_cem_ql::CemQlDebugConditionEvaluator;\nuse cem_ql::debug_control::CemQlDebugConditionEvaluator as DirectEvaluator;\n\nfn main() {\n    let _ = core::mem::size_of::<DapSession<()>>();\n    let _ = core::mem::size_of::<StopToken>();\n    let _ = core::mem::size_of::<CemQlDebugConditionEvaluator>();\n    let _ = core::mem::size_of::<DirectEvaluator>();\n}\n`,
    );
    const result = run('cargo', [
      'check',
      '--offline',
      '--manifest-path',
      resolve(fixtureRoot, 'Cargo.toml'),
      '--target-dir',
      resolve(workspaceRoot, 'dist/target/cem_ml_stripped_surface'),
    ]);
    assert.notEqual(result.status, 0, 'stripped debugger imports must not compile');
    const diagnostics = `${result.stdout}\n${result.stderr}`;
    for (const absentSurface of [
      'cem_ml::dap',
      'cem_ml::debug_control',
      'CemQlDebugConditionEvaluator',
      'cem_ql::debug_control',
    ]) {
      assert.ok(
        diagnostics.includes(absentSurface.split('::').at(-1)),
        `compile failure did not identify stripped surface ${absentSurface}`,
      );
    }
  } finally {
    rmSync(fixtureRoot, { recursive: true, force: true });
  }
}

function verifyIntegrity(root) {
  const integrity = readJson(resolve(root, 'integrity.json'));
  assert.equal(integrity.algorithm, 'sha256');
  const recordedPaths = integrity.files.map((entry) => entry.path);
  const actualPaths = listFiles(root).filter((path) => path !== 'integrity.json');
  assert.deepEqual(recordedPaths, actualPaths, 'integrity must cover each profile exactly once');
  const wasmEntries = integrity.files.filter((entry) => entry.path.endsWith('_bg.wasm'));
  assert.equal(wasmEntries.length, 2);
  for (const entry of integrity.files) {
    const bytes = readFileSync(resolve(root, entry.path));
    assert.equal(bytes.byteLength, entry.bytes, `byte length drift for ${entry.path}`);
    assert.equal(
      createHash('sha256').update(bytes).digest('hex'),
      entry.sha256,
      `SHA-256 drift for ${entry.path}`,
    );
  }
}

function listFiles(root) {
  const files = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const absolute = resolve(directory, entry.name);
      if (entry.isDirectory()) visit(absolute);
      else if (entry.isFile()) files.push(relative(root, absolute).split(sep).join('/'));
    }
  };
  visit(root);
  return files.sort();
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function capture(command, args, maxBuffer = 16 * 1024 * 1024) {
  const result = run(command, args, maxBuffer);
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(' ')} failed:\n${result.stderr || result.stdout || result.error}`,
    );
  }
  return result.stdout.trim();
}

function run(command, args, maxBuffer = 16 * 1024 * 1024) {
  return spawnSync(platformCommand(command), args, {
    cwd: workspaceRoot,
    encoding: 'utf8',
    maxBuffer,
  });
}

function platformCommand(command) {
  if (process.platform !== 'win32' || command.includes('/') || command.includes('\\')) {
    return command;
  }
  return `${command}.cmd`;
}
