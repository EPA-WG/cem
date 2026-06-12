/**
 * Programmatic entrypoint: `import { runTrang } from '@epa-wg/trang-native'`.
 *
 * Mirrors the lookup logic in bin/trang.mjs but returns an awaitable
 * `{ status, stdout, stderr }` result instead of inheriting stdio.
 * Use the CLI shim (`bin/trang.mjs`) when you want streaming output;
 * use this when you want the captured strings.
 */
import { existsSync, readFileSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const packageRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));

export class TrangNotInstalledError extends Error {
  constructor(triple) {
    super(
      `no @epa-wg/trang-native binary found for ${triple}; ` +
        `run \`npm install @epa-wg/trang-native\` or set TRANG_NATIVE_BINARY`,
    );
    this.name = 'TrangNotInstalledError';
    this.triple = triple;
  }
}

/**
 * Run Trang with the given argv. Resolves with the buffered stdio
 * regardless of exit status; consumers branch on `result.status`.
 *
 * @param {string[]} argv - arguments forwarded verbatim to the binary
 * @param {object} [opts]
 * @param {string} [opts.cwd]
 * @param {NodeJS.ProcessEnv} [opts.env]
 * @param {number} [opts.timeoutMs]
 * @returns {Promise<{ status: number, stdout: string, stderr: string }>}
 */
export async function runTrang(argv, opts = {}) {
  const binary = resolveBinary();
  if (!binary) {
    throw new TrangNotInstalledError(detectHostTriple());
  }
  return new Promise((resolve, reject) => {
    const child = spawn(binary, argv, {
      cwd: opts.cwd,
      env: opts.env ?? process.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    const timer = opts.timeoutMs
      ? setTimeout(() => {
          child.kill('SIGKILL');
          reject(new Error(`trang timed out after ${opts.timeoutMs} ms`));
        }, opts.timeoutMs)
      : null;
    child.on('error', (err) => {
      if (timer) clearTimeout(timer);
      reject(err);
    });
    child.on('close', (code) => {
      if (timer) clearTimeout(timer);
      resolve({ status: code ?? 1, stdout, stderr });
    });
  });
}

/**
 * Resolve the on-disk path to the native Trang binary, or `null` when
 * none is available. Exposed so consumers can advertise the path to
 * subprocesses (e.g. the cem-ml Rust fixtures spawn `trang` directly).
 */
export function resolveBinary() {
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
      // fall through
    }
  }
  const triple = detectHostTriple();
  const binaryName = triple.startsWith('windows-') ? 'trang.exe' : 'trang';
  const probe = path.join(packageRoot, 'bin', 'native', triple, binaryName);
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
