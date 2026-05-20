#!/usr/bin/env node
/**
 * Package the host-built native binary into a release archive +
 * SHA256SUMS line.
 *
 * Layout produced under `dist/`:
 *   dist/trang-<target>.tar.gz   (Linux / macOS)
 *   dist/trang-<target>.zip      (Windows)
 *   dist/trang-<target>.sha256   (single-file checksum sidecar)
 *
 * The release workflow appends each sidecar into a top-level
 * SHA256SUMS file at release-assembly time.
 */
import { copyFileSync, createReadStream, createWriteStream, existsSync, mkdirSync, rmSync, statSync } from 'node:fs';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { createGzip } from 'node:zlib';
import { fileURLToPath } from 'node:url';
import { pipeline } from 'node:stream/promises';
import path from 'node:path';

const packageRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));

const target = pickArg(process.argv.slice(2), '--target') || detectHostTarget();
const nativeDir = path.join(packageRoot, 'build', 'native', target);
const distDir = path.join(packageRoot, 'dist');

const binaryName = target.startsWith('windows-') ? 'trang.exe' : 'trang';
const binaryPath = path.join(nativeDir, binaryName);
if (!existsSync(binaryPath)) {
  exitWith(
    1,
    `${binaryPath} not found — run nx run @epa-wg/trang-native:build:${target} first`,
  );
}

mkdirSync(distDir, { recursive: true });

const upstream = JSON.parse(
  await readFile(path.join(packageRoot, 'upstream.json'), 'utf8'),
);
const pkg = JSON.parse(
  await readFile(path.join(packageRoot, 'package.json'), 'utf8'),
);

const metadata = {
  name: '@epa-wg/trang-native',
  package_version: pkg.version,
  target,
  trang_ref: upstream.ref,
  trang_repository: upstream.repository,
  built_at: new Date().toISOString(),
  builder_node: process.version,
  builder_platform: process.platform,
  builder_arch: process.arch,
};
const metadataPath = path.join(nativeDir, 'metadata.json');
await writeFile(metadataPath, JSON.stringify(metadata, null, 2) + '\n', 'utf8');

const isWindows = target.startsWith('windows-');
const archiveName = isWindows
  ? `trang-${target}.zip`
  : `trang-${target}.tar.gz`;
const archivePath = path.join(distDir, archiveName);

if (isWindows) {
  await createZipArchive(archivePath, [
    { absPath: binaryPath, archivePath: binaryName },
    { absPath: metadataPath, archivePath: 'metadata.json' },
    { absPath: path.join(packageRoot, 'LICENSE'), archivePath: 'LICENSE' },
    { absPath: path.join(packageRoot, 'NOTICE'), archivePath: 'NOTICE' },
  ]);
} else {
  await createTarGzArchive(archivePath, [
    { absPath: binaryPath, archivePath: binaryName },
    { absPath: metadataPath, archivePath: 'metadata.json' },
    { absPath: path.join(packageRoot, 'LICENSE'), archivePath: 'LICENSE' },
    { absPath: path.join(packageRoot, 'NOTICE'), archivePath: 'NOTICE' },
  ]);
}

const checksum = await sha256(archivePath);
const sidecar = `${checksum}  ${archiveName}\n`;
await writeFile(`${archivePath}.sha256`, sidecar, 'utf8');

console.log(`[trang-native] packaged ${archivePath} (${statSync(archivePath).size} bytes)`);
console.log(`[trang-native] sha256: ${checksum}`);

async function createTarGzArchive(outPath, entries) {
  // Use the system `tar` for deterministic, portable archives. Falls
  // back to a Node-side implementation if unavailable (rare on CI
  // runners but possible on Windows).
  if (commandExists('tar')) {
    const stagedDir = path.join(distDir, `.stage-${target}`);
    mkdirSync(stagedDir, { recursive: true });
    for (const entry of entries) {
      copyToStage(entry.absPath, path.join(stagedDir, entry.archivePath));
    }
    const tarArgs = [
      '--owner=0',
      '--group=0',
      '--numeric-owner',
      // Reproducible mtime so the archive bytes are stable across
      // builds of the same source.
      '--mtime=2026-01-01T00:00:00Z',
      '-czf',
      outPath,
      '-C',
      stagedDir,
      '.',
    ];
    const result = spawnSync('tar', tarArgs, { stdio: 'inherit' });
    if (result.status !== 0) {
      exitWith(result.status ?? 1, `tar failed (exit ${result.status})`);
    }
    rmSync(stagedDir, { recursive: true, force: true });
    return;
  }
  // Pure-Node fallback: write a single-entry gzipped concatenation.
  // Real-world consumers shouldn't hit this path — tar is universally
  // available on the Linux/macOS CI runners — but we keep the path so
  // the script is testable without external deps.
  exitWith(127, 'tar binary not found and the pure-Node fallback is not yet implemented');
}

async function createZipArchive(outPath, entries) {
  if (commandExists('powershell')) {
    const stagedDir = path.join(distDir, `.stage-${target}`);
    mkdirSync(stagedDir, { recursive: true });
    for (const entry of entries) {
      copyToStage(entry.absPath, path.join(stagedDir, entry.archivePath));
    }
    const psArgs = [
      '-NoProfile',
      '-Command',
      `Compress-Archive -Path "${stagedDir}\\*" -DestinationPath "${outPath}" -Force`,
    ];
    const result = spawnSync('powershell', psArgs, { stdio: 'inherit' });
    if (result.status !== 0) {
      exitWith(result.status ?? 1, `Compress-Archive failed (exit ${result.status})`);
    }
    rmSync(stagedDir, { recursive: true, force: true });
    return;
  }
  if (commandExists('zip')) {
    const stagedDir = path.join(distDir, `.stage-${target}`);
    mkdirSync(stagedDir, { recursive: true });
    for (const entry of entries) {
      copyToStage(entry.absPath, path.join(stagedDir, entry.archivePath));
    }
    const result = spawnSync('zip', ['-rj', outPath, stagedDir], { stdio: 'inherit' });
    if (result.status !== 0) {
      exitWith(result.status ?? 1, `zip failed (exit ${result.status})`);
    }
    rmSync(stagedDir, { recursive: true, force: true });
    return;
  }
  exitWith(127, 'no zip / powershell available to package the Windows archive');
}

function copyToStage(src, dst) {
  mkdirSync(path.dirname(dst), { recursive: true });
  copyFileSync(src, dst);
}

async function sha256(filePath) {
  const hash = createHash('sha256');
  await pipeline(createReadStream(filePath), hash);
  return hash.digest('hex');
}

function pickArg(argv, flag) {
  const idx = argv.indexOf(flag);
  if (idx < 0 || idx === argv.length - 1) return null;
  return argv[idx + 1];
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

// Suppress unused-import warnings for streaming helpers exposed for
// the (future) pure-Node tar fallback.
void createWriteStream;
void createGzip;
