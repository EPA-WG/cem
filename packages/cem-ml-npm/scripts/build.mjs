import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  cpSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const distRoot = resolve(projectRoot, 'dist');
const browserRoot = resolve(distRoot, 'wasm/browser');
const nodeRoot = resolve(distRoot, 'wasm/node');
const schemaSourceRoot = resolve(workspaceRoot, 'packages/cem_ml/schema-packages');
const schemaOutputRoot = resolve(distRoot, 'schema-packages');
const cargoManifestPath = resolve(workspaceRoot, 'packages/cem_ml/Cargo.toml');
const packagePath = resolve(projectRoot, 'package.json');
const packageMetadata = JSON.parse(readFileSync(packagePath, 'utf8'));
const cargoManifest = readFileSync(cargoManifestPath, 'utf8');
const cargoVersion = cargoManifest.match(
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
  '--package',
  'cem-ml',
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

rmSync(distRoot, { recursive: true, force: true });
mkdirSync(browserRoot, { recursive: true });
mkdirSync(nodeRoot, { recursive: true });

const cargoTargetRoot = resolve(workspaceRoot, 'dist/target/cem_ml_npm');
run('cargo', [
  'build',
  '--locked',
  '--package',
  'cem-ml',
  '--lib',
  '--release',
  '--target',
  'wasm32-unknown-unknown',
  '--target-dir',
  cargoTargetRoot,
]);

const wasmInput = resolve(
  cargoTargetRoot,
  'wasm32-unknown-unknown/release/cem_ml.wasm',
);
for (const [target, outputRoot] of [
  ['web', browserRoot],
  ['nodejs', nodeRoot],
]) {
  run('wasm-bindgen', [
    wasmInput,
    '--target',
    target,
    '--out-dir',
    outputRoot,
    '--out-name',
    'cem_ml',
  ]);
}
writeJson(resolve(nodeRoot, 'package.json'), { type: 'commonjs' });

cpSync(schemaSourceRoot, schemaOutputRoot, {
  recursive: true,
  filter(sourcePath) {
    const sourceRelative = relative(schemaSourceRoot, sourcePath);
    if (!sourceRelative) return true;
    const segments = sourceRelative.split(sep);
    return (
      !segments.includes('dist') &&
      !segments.includes('scripts') &&
      !segments.includes('previews') &&
      !['project.json', 'README.md'].includes(segments.at(-1))
    );
  },
});

const nodeRuntime = createRequire(import.meta.url)(resolve(nodeRoot, 'cem_ml.js'));
if (nodeRuntime.version() !== cargoVersion) {
  throw new Error('The generated Node runtime does not report the common Cargo version');
}

const abiIdentity = `wasm-bindgen@${resolvedBindgenVersion}`;
const capabilityRequest = (runtime, targetIdentity) => ({
  runtime,
  targetIdentity,
  abiIdentity,
  debugControlActive: false,
});
const capabilities = {
  browser: parseCapability(
    nodeRuntime.capabilityManifest(
      JSON.stringify(capabilityRequest('wasm-browser-worker', 'wasm32-unknown-unknown:web')),
    ),
  ),
  node: parseCapability(
    nodeRuntime.capabilityManifest(
      JSON.stringify(capabilityRequest('wasm-node', 'wasm32-unknown-unknown:nodejs')),
    ),
  ),
};

const schemaFiles = listFiles(schemaOutputRoot);
const runtimeManifest = {
  schemaVersion: 1,
  package: {
    name: packageMetadata.name,
    version: packageMetadata.version,
  },
  commonVersion: cargoVersion,
  abi: {
    identity: abiIdentity,
    wasmBindgenVersion: resolvedBindgenVersion,
  },
  artifacts: {
    browser: {
      module: 'wasm/browser/cem_ml.js',
      wasm: 'wasm/browser/cem_ml_bg.wasm',
      types: 'wasm/browser/cem_ml.d.ts',
    },
    node: {
      module: 'wasm/node/cem_ml.js',
      wasm: 'wasm/node/cem_ml_bg.wasm',
      types: 'wasm/node/cem_ml.d.ts',
    },
  },
  schemaPackages: {
    root: 'schema-packages',
    fileCount: schemaFiles.length,
    manifestCount: schemaFiles.filter((path) => path.endsWith('/package.cem')).length,
  },
  capabilities,
};
writeJson(resolve(distRoot, 'cem-ml-runtime.json'), runtimeManifest);

const integrityFiles = listFiles(distRoot)
  .filter((path) => path !== 'integrity.json')
  .map((path) => {
    const bytes = readFileSync(resolve(distRoot, path));
    return {
      path,
      bytes: bytes.byteLength,
      sha256: createHash('sha256').update(bytes).digest('hex'),
    };
  });
writeJson(resolve(distRoot, 'integrity.json'), {
  schemaVersion: 1,
  algorithm: 'sha256',
  files: integrityFiles,
});

console.log(
  `Built ${packageMetadata.name}@${packageMetadata.version}: ${integrityFiles.length} integrity records, ${schemaFiles.length} schema-package assets.`,
);

function parseCapability(json) {
  const value = JSON.parse(json);
  if (value.error) {
    throw new Error(`Capability projection failed: ${value.error.code}: ${value.error.message}`);
  }
  return value;
}

function listFiles(root) {
  const files = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const absolute = resolve(directory, entry.name);
      if (entry.isDirectory()) {
        visit(absolute);
      } else if (entry.isFile()) {
        files.push(relative(root, absolute).split(sep).join('/'));
      }
    }
  };
  visit(root);
  return files.sort();
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function capture(command, args) {
  const result = spawnSync(platformCommand(command), args, {
    cwd: workspaceRoot,
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(' ')} failed:\n${result.stderr || result.stdout || result.error}`,
    );
  }
  return result.stdout.trim();
}

function run(command, args) {
  const result = spawnSync(platformCommand(command), args, {
    cwd: workspaceRoot,
    stdio: 'inherit',
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with status ${result.status}`);
  }
}

function platformCommand(command) {
  return process.platform === 'win32' ? `${command}.cmd` : command;
}
