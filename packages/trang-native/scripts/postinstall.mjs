#!/usr/bin/env node
/**
 * Consumer-side postinstall: detect platform + arch, download the
 * matching Trang native archive from the matching GitHub Release,
 * verify its SHA-256, and extract it to `bin/native/<target>/`.
 *
 * The download + checksum + extract core lives in
 * `scripts/lib/native-binary.mjs` (shared with the workspace
 * `acquire-binary` build entry).
 *
 * Behavior:
 *  - Auto-skips when the script is not running under `node_modules/`
 *    (workspace dev clones build via nx instead).
 *  - Auto-skips when `TRANG_NATIVE_SKIP_DOWNLOAD=1` is set.
 *  - Auto-skips when `TRANG_NATIVE_BINARY=/abs/path` is set (consumer
 *    wants to use a system-installed Trang).
 *  - Auto-skips when the binary is already present (offline / cached).
 *  - Warns and exits 0 (never breaks `npm install`) on download failures;
 *    consumers see the warning and can manually populate bin/native/ or
 *    set TRANG_NATIVE_BINARY.
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import {
  detectHostTarget,
  binaryNameFor,
  downloadRelease,
  DEFAULT_RELEASE_BASE,
} from './lib/native-binary.mjs';

const packageRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const RELEASE_BASE = process.env.TRANG_NATIVE_RELEASE_BASE || DEFAULT_RELEASE_BASE;

// Skip in workspace dev clones. Yarn's node-modules linker symlinks
// `node_modules/@epa-wg/trang-native` to `packages/trang-native`, so
// `import.meta.url` would *look* like an install. The real signal is
// the presence of `scripts/build-native.mjs` — it lives in the source
// tree but is NOT in the published `files` allowlist, so a real
// consumer install never has it. (Keep this in sync with package.json
// `files`: build-native.mjs must stay out of the published tarball.)
{
  const buildScript = path.join(packageRoot, 'scripts', 'build-native.mjs');
  if (existsSync(buildScript)) {
    log('workspace install detected (build-native.mjs present); skipping download');
    process.exit(0);
  }
}

if (process.env.TRANG_NATIVE_SKIP_DOWNLOAD === '1') {
  log('TRANG_NATIVE_SKIP_DOWNLOAD=1; skipping download');
  process.exit(0);
}

if (process.env.TRANG_NATIVE_BINARY) {
  if (!existsSync(process.env.TRANG_NATIVE_BINARY)) {
    warn(`TRANG_NATIVE_BINARY=${process.env.TRANG_NATIVE_BINARY} does not exist; ignoring`);
  } else {
    log(`TRANG_NATIVE_BINARY=${process.env.TRANG_NATIVE_BINARY}; skipping download`);
    process.exit(0);
  }
}

const { triple, supported } = detectHostTarget();
if (!supported) {
  warn(
    `no prebuilt Trang for ${triple}; ` +
      `set TRANG_NATIVE_BINARY=/path/to/trang or install Trang via your package manager`,
  );
  process.exit(0);
}

const pkg = JSON.parse(readFileSync(path.join(packageRoot, 'package.json'), 'utf8'));
const version = pkg.version;
const binaryName = binaryNameFor(triple);

const binDir = path.join(packageRoot, 'bin', 'native', triple);
const binaryPath = path.join(binDir, binaryName);
if (existsSync(binaryPath)) {
  log(`binary already present at ${binaryPath}; skipping download`);
  process.exit(0);
}

try {
  mkdirSync(binDir, { recursive: true });
  await downloadRelease({ version, triple, destDir: binDir, releaseBase: RELEASE_BASE, log });
  log(`installed ${binaryPath}`);

  // Drop a marker file so the bin shim can fast-path to the binary
  // without re-detecting the host.
  writeFileSync(
    path.join(packageRoot, 'bin', 'native', 'manifest.json'),
    JSON.stringify(
      {
        version,
        target: triple,
        binary: path.relative(packageRoot, binaryPath),
        installed_at: new Date().toISOString(),
      },
      null,
      2,
    ) + '\n',
    'utf8',
  );
} catch (err) {
  warn(`failed to install native Trang for ${triple}: ${err.message || err}`);
  warn('the package shim will fall back to TRANG_NATIVE_BINARY or system `trang` at runtime');
  // Exit 0 — postinstall failures should never block `npm install`.
  process.exit(0);
}

function log(msg) {
  console.log(`[trang-native] ${msg}`);
}

function warn(msg) {
  console.warn(`[trang-native] warning: ${msg}`);
}
