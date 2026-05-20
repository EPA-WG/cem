#!/usr/bin/env node
/**
 * Consumer-side postinstall: detect platform + arch, download the
 * matching Trang native archive from the matching GitHub Release,
 * verify its SHA-256, and extract it to `bin/native/<target>/`.
 *
 * Behavior:
 *  - Auto-skips when the script is not running under `node_modules/`
 *    (workspace dev clones of this repo build via nx instead).
 *  - Auto-skips when `TRANG_NATIVE_SKIP_DOWNLOAD=1` is set.
 *  - Auto-skips when `TRANG_NATIVE_BINARY=/abs/path` is set (consumer
 *    wants to use a system-installed Trang).
 *  - Auto-skips when the binary is already present (offline / cached).
 *  - Warns and exits 0 (never breaks `npm install`) on download failures;
 *    consumers see the warning and can manually populate bin/native/ or
 *    set TRANG_NATIVE_BINARY.
 */
import { existsSync, mkdirSync, createWriteStream, chmodSync, readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { tmpdir } from 'node:os';

const packageRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const RELEASE_BASE = process.env.TRANG_NATIVE_RELEASE_BASE
  || 'https://github.com/EPA-WG/cem/releases/download';

// Skip in workspace dev clones. Yarn's node-modules linker symlinks
// `node_modules/@epa-wg/trang-native` to `packages/trang-native`, so
// `import.meta.url` would *look* like an install. The real signal is
// the presence of `scripts/build-native.mjs` — it lives in the source
// tree but is NOT in the published `files` allowlist, so a real
// consumer install never has it.
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

const target = detectHostTarget();
if (!target.supported) {
  warn(
    `no prebuilt Trang for ${target.triple}; ` +
      `set TRANG_NATIVE_BINARY=/path/to/trang or install Trang via your package manager`,
  );
  process.exit(0);
}

const pkg = JSON.parse(readFileSync(path.join(packageRoot, 'package.json'), 'utf8'));
const version = pkg.version;
const isWindows = target.triple.startsWith('windows-');
const archiveName = isWindows
  ? `trang-${target.triple}.zip`
  : `trang-${target.triple}.tar.gz`;
const binaryName = isWindows ? 'trang.exe' : 'trang';

const binDir = path.join(packageRoot, 'bin', 'native', target.triple);
const binaryPath = path.join(binDir, binaryName);
if (existsSync(binaryPath)) {
  log(`binary already present at ${binaryPath}; skipping download`);
  process.exit(0);
}

mkdirSync(binDir, { recursive: true });

const archiveUrl = `${RELEASE_BASE}/trang-native-v${version}/${archiveName}`;
const checksumUrl = `${RELEASE_BASE}/trang-native-v${version}/${archiveName}.sha256`;

const tmpArchive = path.join(tmpdir(), `trang-native-${Date.now()}-${archiveName}`);
const tmpChecksum = `${tmpArchive}.sha256`;

try {
  log(`fetching ${archiveUrl}`);
  await downloadTo(archiveUrl, tmpArchive);
  log(`fetching ${checksumUrl}`);
  await downloadTo(checksumUrl, tmpChecksum);

  const sidecar = readFileSync(tmpChecksum, 'utf8').trim();
  const expected = sidecar.split(/\s+/)[0];
  if (!/^[0-9a-fA-F]{64}$/.test(expected)) {
    throw new Error(`malformed sha256 sidecar: ${sidecar}`);
  }

  const actual = await sha256(tmpArchive);
  if (actual.toLowerCase() !== expected.toLowerCase()) {
    throw new Error(`checksum mismatch: expected ${expected}, got ${actual}`);
  }
  log(`checksum verified (${actual})`);

  extractArchive(tmpArchive, binDir, isWindows);
  if (!existsSync(binaryPath)) {
    throw new Error(`archive extracted but ${binaryName} not found in ${binDir}`);
  }
  if (!isWindows) {
    chmodSync(binaryPath, 0o755);
  }
  log(`installed ${binaryPath}`);

  // Drop a marker file so the bin shim can fast-path to the binary
  // without re-detecting the host.
  writeFileSync(
    path.join(packageRoot, 'bin', 'native', 'manifest.json'),
    JSON.stringify(
      {
        version,
        target: target.triple,
        binary: path.relative(packageRoot, binaryPath),
        installed_at: new Date().toISOString(),
      },
      null,
      2,
    ) + '\n',
    'utf8',
  );
} catch (err) {
  warn(`failed to install native Trang for ${target.triple}: ${err.message || err}`);
  warn('the package shim will fall back to TRANG_NATIVE_BINARY or system `trang` at runtime');
  // Exit 0 — postinstall failures should never block `npm install`.
  process.exit(0);
}

async function downloadTo(url, dest) {
  // Use native fetch (Node 18+). Follow redirects (default behaviour
  // in undici/native fetch).
  const response = await fetch(url, { redirect: 'follow' });
  if (!response.ok) {
    throw new Error(`HTTP ${response.status} ${response.statusText} for ${url}`);
  }
  const stream = createWriteStream(dest);
  await new Promise((resolve, reject) => {
    // Convert the Web ReadableStream into a Node stream and pipe it.
    const reader = response.body.getReader();
    const pump = () =>
      reader.read().then(({ done, value }) => {
        if (done) {
          stream.end(resolve);
          return;
        }
        if (!stream.write(value)) {
          stream.once('drain', pump);
        } else {
          pump();
        }
      }).catch(reject);
    pump();
  });
}

async function sha256(filePath) {
  const hash = createHash('sha256');
  const { createReadStream } = await import('node:fs');
  const { pipeline } = await import('node:stream/promises');
  await pipeline(createReadStream(filePath), hash);
  return hash.digest('hex');
}

function extractArchive(archivePath, destDir, isWindows) {
  if (isWindows) {
    // Prefer tar (bundled with Win10+); fall back to PowerShell.
    if (commandExists('tar')) {
      runOrThrow('tar', ['-xf', archivePath, '-C', destDir]);
      return;
    }
    if (commandExists('powershell')) {
      runOrThrow('powershell', [
        '-NoProfile',
        '-Command',
        `Expand-Archive -Path "${archivePath}" -DestinationPath "${destDir}" -Force`,
      ]);
      return;
    }
    throw new Error('no `tar` or `powershell` available to extract the archive');
  }
  if (!commandExists('tar')) {
    throw new Error('`tar` not on PATH; cannot extract the archive');
  }
  runOrThrow('tar', ['-xzf', archivePath, '-C', destDir]);
}

function runOrThrow(cmd, args) {
  const result = spawnSync(cmd, args, { stdio: 'inherit' });
  if (result.status !== 0) {
    throw new Error(`\`${cmd} ${args.join(' ')}\` failed (exit ${result.status})`);
  }
}

function commandExists(name) {
  const probe = process.platform === 'win32'
    ? spawnSync('where', [name], { encoding: 'utf8' })
    : spawnSync('command', ['-v', name], { encoding: 'utf8', shell: true });
  return probe.status === 0;
}

function detectHostTarget() {
  const arch = process.arch === 'x64' ? 'x86_64'
    : process.arch === 'arm64' ? 'aarch64'
    : process.arch;
  const platform = process.platform === 'linux' ? 'linux'
    : process.platform === 'darwin' ? 'macos'
    : process.platform === 'win32' ? 'windows'
    : process.platform;
  const triple = `${platform}-${arch}`;
  // Mirror the four-target shipping matrix.
  const supported = [
    'linux-x86_64',
    'linux-aarch64',
    'windows-x86_64',
    'macos-aarch64',
  ].includes(triple);
  return { triple, supported };
}

function log(msg) {
  console.log(`[trang-native] ${msg}`);
}

function warn(msg) {
  console.warn(`[trang-native] warning: ${msg}`);
}
