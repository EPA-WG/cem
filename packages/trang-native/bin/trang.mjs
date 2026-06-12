#!/usr/bin/env node
/**
 * `npx trang …` entrypoint.
 *
 * Locates the native binary in this order:
 *   1. `TRANG_NATIVE_BINARY` env var (escape hatch / system Trang)
 *   2. `bin/native/manifest.json` written by postinstall.mjs
 *   3. `bin/native/<host-triple>/trang(.exe)` direct probe
 *
 * Exits with the binary's exit code; stdio is inherited so consumers
 * see Trang's output verbatim.
 */
import { existsSync, readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const packageRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));

const binary = resolveBinary();
if (!binary) {
  console.error(
    '[trang-native] no Trang binary found.\n' +
      '  Try `npm install @epa-wg/trang-native` to fetch one, or\n' +
      '  set TRANG_NATIVE_BINARY=/abs/path/trang to use a system install.',
  );
  process.exit(127);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });
process.exit(result.status ?? 1);

function resolveBinary() {
  if (process.env.TRANG_NATIVE_BINARY && existsSync(process.env.TRANG_NATIVE_BINARY)) {
    return process.env.TRANG_NATIVE_BINARY;
  }
  const manifestPath = path.join(packageRoot, 'bin', 'native', 'manifest.json');
  if (existsSync(manifestPath)) {
    try {
      const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
      const resolved = path.join(packageRoot, manifest.binary);
      if (existsSync(resolved)) return resolved;
    } catch {
      // fall through to direct probe
    }
  }
  const target = detectHostTriple();
  const binaryName = target.startsWith('windows-') ? 'trang.exe' : 'trang';
  const probe = path.join(packageRoot, 'bin', 'native', target, binaryName);
  return existsSync(probe) ? probe : null;
}

function detectHostTriple() {
  const arch = process.arch === 'x64' ? 'x86_64'
    : process.arch === 'arm64' ? 'aarch64'
    : process.arch;
  const platform = process.platform === 'linux' ? 'linux'
    : process.platform === 'darwin' ? 'macos'
    : process.platform === 'win32' ? 'windows'
    : process.platform;
  return `${platform}-${arch}`;
}
