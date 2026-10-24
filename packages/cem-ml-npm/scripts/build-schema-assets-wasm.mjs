import { ownedWasmFactory, ownedWasmFactoryTypes } from '../../../tools/scripts/cem-ql-wasm-factory.mjs';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
if (process.argv.length !== 2) {
  throw new Error('usage: node build-schema-assets-wasm.mjs');
}
const outputRoot = resolve(projectRoot, 'dist-schema-assets-wasm');
const defaultRoot = resolve(projectRoot, 'dist');
const cargoTargetRoot = resolve(workspaceRoot, 'dist/target/cem_ml_npm');
const cargoManifestPath = resolve(workspaceRoot, 'packages/cem_ml/Cargo.toml');
const packageMetadata = JSON.parse(readFileSync(resolve(projectRoot, 'package.json'), 'utf8'));
const cargoVersion = readFileSync(cargoManifestPath, 'utf8').match(
  /^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
)?.[1];
if (!cargoVersion) {
  throw new Error(`Could not read the common version from ${cargoManifestPath}`);
}
if (packageMetadata.version !== cargoVersion) {
  throw new Error(
    `Version drift: ${packageMetadata.name}@${packageMetadata.version} must match cem_ml@${cargoVersion}`,
  );
}
if ('bin' in packageMetadata) {
  throw new Error(`${packageMetadata.name} is a low-level runtime and must not define an npm bin`);
}

const cargoTree = capture('cargo', [
  'tree',
  '--locked',
  '--package',
  'cem-ml-transform-cem-ql',
  '--features',
  'external-schema-assets',
  '--invert',
  'wasm-bindgen',
  '--target',
  'wasm32-unknown-unknown',
  '--depth',
  '0',
]);
const resolvedBindgenVersion = cargoTree.match(/wasm-bindgen v([^\s]+)/)?.[1];
const cliBindgenVersion = capture('wasm-bindgen', ['--version']).match(
  /wasm-bindgen ([^\s]+)/,
)?.[1];
if (!resolvedBindgenVersion || !cliBindgenVersion) {
  throw new Error('Could not determine the resolved crate and CLI wasm-bindgen versions');
}
if (resolvedBindgenVersion !== cliBindgenVersion) {
  throw new Error(
    `wasm-bindgen version drift: crate ${resolvedBindgenVersion}, CLI ${cliBindgenVersion}`,
  );
}

// The default package is a comparison input only. Its embedded catalog and
// generated bindings stay in place; this profile excludes optional source bodies.
const defaultWasm = {
  browser: measure(resolve(defaultRoot, 'wasm/browser/cem_ml_bg.wasm')),
  node: measure(resolve(defaultRoot, 'wasm/node/cem_ml_bg.wasm')),
};
// Refuse stale identity metadata; generating it requires the complete source build.
run('cargo', ['run', '--locked', '--package', 'cem-ml', '--no-default-features', '--bin', 'cem-ml-schema-assets-emit',
  '--target-dir', resolve(workspaceRoot, 'dist/target/cem_ml_schema_assets'), '--', '--check-registry',
  resolve(workspaceRoot, 'packages/cem_ml/schema/schema-assets-registry-v1.json')]);
const stagingRoot = mkdtempSync(join(tmpdir(), 'cem-schema-assets-wasm-'));
try {
  const browserRoot = resolve(stagingRoot, 'wasm/browser');
  const nodeRoot = resolve(stagingRoot, 'wasm/node');
  mkdirSync(browserRoot, { recursive: true });
  mkdirSync(nodeRoot, { recursive: true });
  run('cargo', [
    'build',
    '--locked',
    '--package',
    'cem-ml-transform-cem-ql',
    '--lib',
    '--release',
    '--target',
    'wasm32-unknown-unknown',
    '--target-dir',
    cargoTargetRoot,
    '--features',
    'external-schema-assets',
  ]);
  const wasmInput = resolve(
    cargoTargetRoot,
    'wasm32-unknown-unknown/release/cem_ml_transform_cem_ql.wasm',
  );
  for (const [target, targetRoot] of [['web', browserRoot], ['nodejs', nodeRoot]]) {
    run('wasm-bindgen', [
      wasmInput,
      '--target',
      target,
      '--out-dir',
      targetRoot,
      '--out-name',
      'cem_ml',
      '--remove-name-section',
      '--remove-producers-section',
    ]);
  }
  writeFileSync(
    resolve(browserRoot, 'cem_ml_factory.js'),
    ownedWasmFactory(readFileSync(resolve(browserRoot, 'cem_ml.js'), 'utf8'), 'cem_ml', 'CemMl'),
  );
  writeFileSync(resolve(browserRoot, 'cem_ml_factory.d.ts'), ownedWasmFactoryTypes('cem_ml', 'CemMl'));
  writeJson(resolve(nodeRoot, 'package.json'), { type: 'commonjs' });

  const runtime = createRequire(import.meta.url)(resolve(nodeRoot, 'cem_ml.js'));
  if (runtime.version() !== cargoVersion) {
    throw new Error('The generated optional Node runtime does not report the common Cargo version');
  }
  const optionalExports = [
    'schemaAssetEmbeddedSourcesV1',
    'schemaAssetPackBootstrapV1',
    'schemaAssetPackSummaryV1',
    'schemaAssetPackImportV1',
  ];
  for (const name of optionalExports) {
    if (typeof runtime[name] !== 'function') {
      throw new Error(`The optional Node runtime is missing ${name}`);
    }
  }
  const defaultModule = resolve(defaultRoot, 'wasm/node/cem_ml.js');
  if (existsSync(defaultModule)) {
    const defaultRuntime = createRequire(import.meta.url)(defaultModule);
    for (const name of optionalExports) {
      if (Object.hasOwn(defaultRuntime, name)) {
        throw new Error(`The default Node runtime must not expose the optional ${name} API`);
      }
    }
  }
  const optionalWasm = {
    browser: measure(resolve(browserRoot, 'cem_ml_bg.wasm')),
    node: measure(resolve(nodeRoot, 'cem_ml_bg.wasm')),
  };
  for (const target of ['browser', 'node']) {
    const currentDefault = measure(resolve(defaultRoot, `wasm/${target}/cem_ml_bg.wasm`));
    if (JSON.stringify(currentDefault) !== JSON.stringify(defaultWasm[target])) {
      throw new Error(`The default ${target} WASM artifact changed during the optional build`);
    }
  }
  const nodeModule = resolve(outputRoot, 'wasm/node/cem_ml.js');
  const report = {
    schemaVersion: 1,
    package: packageMetadata.name,
    commonVersion: cargoVersion,
    featureProfile: 'debug-control+schema-asset-packs+external-schema-assets',
    wasmBindgenVersion: resolvedBindgenVersion,
    gzipLevel: 9,
    defaultEmbeddingsRetained: false,
    embeddingProfile: JSON.parse(runtime.schemaAssetEmbeddedSourcesV1()),
    checkpointSavingsClaimed: false,
    caution: 'This opt-in profile retains bootstrap bodies and complete identity metadata; optional package bodies require explicit admission. Artifact byte counts do not establish checkpoint or allocator-memory savings.',
    artifacts: {
      browserModule: outputPath('wasm/browser/cem_ml.js'),
      browserFactory: outputPath('wasm/browser/cem_ml_factory.js'),
      browserFactoryTypes: outputPath('wasm/browser/cem_ml_factory.d.ts'),
      browserTypes: outputPath('wasm/browser/cem_ml.d.ts'),
      nodeModule: outputPath('wasm/node/cem_ml.js'),
      nodeTypes: outputPath('wasm/node/cem_ml.d.ts'),
    },
    defaultWasm,
    optionalWasm,
    verifier: {
      script: 'packages/cem-ml-npm/scripts/verify-schema-assets.mjs',
      wasm: outputPath('wasm/node/cem_ml.js'),
    },
  };
  writeJson(resolve(stagingRoot, 'schema-assets-wasm-build.json'), report);
  rmSync(outputRoot, { recursive: true, force: true });
  cpSync(stagingRoot, outputRoot, { recursive: true, force: false, errorOnExist: true });
  console.log(`Built optional ${packageMetadata.name}@${packageMetadata.version} WASM in ${outputRoot}`);
  console.log(report.caution);
  console.log(`Verify with: node packages/cem-ml-npm/scripts/verify-schema-assets.mjs --wasm=${nodeModule}`);
  console.log(JSON.stringify({ defaultWasm, optionalWasm }));
} finally {
  rmSync(stagingRoot, { recursive: true, force: true });
}

function outputPath(path) {
  return relative(workspaceRoot, resolve(outputRoot, path)).split(sep).join('/');
}

function measure(path) {
  if (!existsSync(path)) return { available: false };
  const bytes = readFileSync(path);
  return {
    available: true,
    bytes: bytes.byteLength,
    gzipBytes: gzipSync(bytes, { level: 9 }).byteLength,
    sha256: createHash('sha256').update(bytes).digest('hex'),
  };
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function capture(command, args) {
  const result = spawnSync(command, args, { cwd: workspaceRoot, encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed:\n${result.stderr || result.stdout || result.error}`);
  }
  return result.stdout.trim();
}

function run(command, args) {
  const result = spawnSync(command, args, { cwd: workspaceRoot, stdio: 'inherit' });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with status ${result.status}`);
  }
}
