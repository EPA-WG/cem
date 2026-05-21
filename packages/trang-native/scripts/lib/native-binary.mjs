#!/usr/bin/env node
/**
 * Shared helpers for resolving / fetching the @epa-wg/trang-native
 * native binary. Used by both the consumer `postinstall` and the
 * workspace `acquire-binary` build entry, so the download + checksum
 * + extract logic lives in exactly one place.
 */
import {
  existsSync,
  mkdirSync,
  createWriteStream,
  createReadStream,
  chmodSync,
  readFileSync,
} from 'node:fs';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { pipeline } from 'node:stream/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

export const DEFAULT_RELEASE_BASE =
  'https://github.com/EPA-WG/cem/releases/download';

/** The four Tier A platforms shipped as GitHub Release archives. */
export const SUPPORTED_TRIPLES = Object.freeze([
  'linux-x86_64',
  'linux-aarch64',
  'windows-x86_64',
  'macos-aarch64',
]);

export function detectHostTarget() {
  const arch = process.arch === 'x64' ? 'x86_64'
    : process.arch === 'arm64' ? 'aarch64'
    : process.arch;
  const platform = process.platform === 'linux' ? 'linux'
    : process.platform === 'darwin' ? 'macos'
    : process.platform === 'win32' ? 'windows'
    : process.platform;
  const triple = `${platform}-${arch}`;
  return { triple, supported: SUPPORTED_TRIPLES.includes(triple) };
}

export function binaryNameFor(triple) {
  return triple.startsWith('windows-') ? 'trang.exe' : 'trang';
}

export function archiveNameFor(triple) {
  return triple.startsWith('windows-')
    ? `trang-${triple}.zip`
    : `trang-${triple}.tar.gz`;
}

export async function sha256(filePath) {
  const hash = createHash('sha256');
  await pipeline(createReadStream(filePath), hash);
  return hash.digest('hex');
}

/** Read `package_version` from a metadata.json sitting next to a binary. */
export function readBinaryVersion(metadataPath) {
  if (!existsSync(metadataPath)) return null;
  try {
    const meta = JSON.parse(readFileSync(metadataPath, 'utf8'));
    return typeof meta.package_version === 'string' ? meta.package_version : null;
  } catch {
    return null;
  }
}

/**
 * Download the release archive for `version`/`triple`, verify its
 * SHA-256 against the sidecar, and extract into `destDir`. Returns the
 * absolute path of the extracted binary. Throws on any failure.
 */
export async function downloadRelease({
  version,
  triple,
  destDir,
  releaseBase = DEFAULT_RELEASE_BASE,
  log = () => {},
}) {
  const isWindows = triple.startsWith('windows-');
  const archiveName = archiveNameFor(triple);
  const binaryName = binaryNameFor(triple);
  const archiveUrl = `${releaseBase}/trang-native-v${version}/${archiveName}`;
  const checksumUrl = `${archiveUrl}.sha256`;

  const tmpArchive = path.join(tmpdir(), `trang-native-${Date.now()}-${archiveName}`);
  const tmpChecksum = `${tmpArchive}.sha256`;

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

  extractArchive(tmpArchive, destDir, isWindows);
  const binaryPath = path.join(destDir, binaryName);
  if (!existsSync(binaryPath)) {
    throw new Error(`archive extracted but ${binaryName} not found in ${destDir}`);
  }
  if (!isWindows) {
    chmodSync(binaryPath, 0o755);
  }
  return binaryPath;
}

export function extractArchive(archivePath, destDir, isWindows) {
  mkdirSync(destDir, { recursive: true });
  if (isWindows) {
    // tar is bundled with Windows 10+ and handles .zip; PowerShell is
    // the fallback.
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

export function commandExists(name) {
  const probe = process.platform === 'win32'
    ? spawnSync('where', [name], { encoding: 'utf8' })
    : spawnSync('command', ['-v', name], { encoding: 'utf8', shell: true });
  return probe.status === 0;
}

async function downloadTo(url, dest) {
  // Native fetch (Node 18+); follow redirects (GitHub Release assets
  // 302 to a CDN).
  const response = await fetch(url, { redirect: 'follow' });
  if (!response.ok) {
    throw new Error(`HTTP ${response.status} ${response.statusText} for ${url}`);
  }
  const stream = createWriteStream(dest);
  await new Promise((resolve, reject) => {
    const reader = response.body.getReader();
    const pump = () =>
      reader
        .read()
        .then(({ done, value }) => {
          if (done) {
            stream.end(resolve);
            return;
          }
          if (!stream.write(value)) {
            stream.once('drain', pump);
          } else {
            pump();
          }
        })
        .catch(reject);
    pump();
  });
}

function runOrThrow(cmd, args) {
  const result = spawnSync(cmd, args, { stdio: 'inherit' });
  if (result.status !== 0) {
    throw new Error(`\`${cmd} ${args.join(' ')}\` failed (exit ${result.status})`);
  }
}
