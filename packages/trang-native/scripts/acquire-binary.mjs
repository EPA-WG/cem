#!/usr/bin/env node
/**
 * `nx run @epa-wg/trang-native:build` entry point.
 *
 * Resolves a host-platform Trang binary as cheaply as possible and only
 * falls back to a full GraalVM `native-image` compile (minutes) when no
 * prebuilt binary can be obtained:
 *
 *   1. Local — `build/native/<triple>/` whose metadata.json records the
 *      current package.json version. Reused as-is, zero work.
 *   2. Release — the matching `trang-native-v<version>` GitHub Release
 *      (the same artifact a published-npm consumer's postinstall pulls).
 *      Downloaded, SHA-256 verified, extracted.
 *   3. From source — `fetch-source -> build-jar -> build-native`.
 *      Requires GraalVM native-image + Apache Ant on the host.
 *
 * A native-image rebuild therefore happens ONLY when:
 *   - the package version was bumped (no Release exists for it yet, so
 *     step 2 misses), or
 *   - a force is requested via `--force` or `TRANG_NATIVE_FORCE_BUILD=1`.
 *
 * Otherwise the expensive compile is skipped entirely. CI/release
 * validation sets `TRANG_NATIVE_REQUIRE_PREBUILT=1` to fail instead of
 * taking the source-build fallback. To compile unconditionally
 * regardless of available binaries, call the explicit `build-from-source`
 * target instead.
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import {
  detectHostTarget,
  binaryNameFor,
  downloadRelease,
  readBinaryVersion,
  DEFAULT_RELEASE_BASE,
} from './lib/native-binary.mjs';

const packageRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const releaseBase = process.env.TRANG_NATIVE_RELEASE_BASE || DEFAULT_RELEASE_BASE;
const force =
  process.argv.includes('--force') ||
  process.env.TRANG_NATIVE_FORCE_BUILD === '1';
const requirePrebuilt = process.env.TRANG_NATIVE_REQUIRE_PREBUILT === '1';

const pkg = JSON.parse(readFileSync(path.join(packageRoot, 'package.json'), 'utf8'));
const version = pkg.version;
const { triple, supported } = detectHostTarget();
const binaryName = binaryNameFor(triple);

const nativeDir = path.join(packageRoot, 'build', 'native', triple);
const binaryPath = path.join(nativeDir, binaryName);
const metadataPath = path.join(nativeDir, 'metadata.json');

await main();

async function main() {
  if (force && requirePrebuilt) {
    fail(
      'TRANG_NATIVE_REQUIRE_PREBUILT=1 conflicts with a forced source build; ' +
        'unset TRANG_NATIVE_FORCE_BUILD/--force or unset TRANG_NATIVE_REQUIRE_PREBUILT',
    );
  }

  if (force) {
    log(`force requested — rebuilding v${version} from source`);
    buildFromSource();
    return;
  }

  // 1. Local build output already at the requested version.
  if (existsSync(binaryPath)) {
    const localVersion = readBinaryVersion(metadataPath);
    if (localVersion === version) {
      log(`build/native/${triple}/${binaryName} already at v${version} — reusing`);
      return;
    }
    log(
      `local binary is ${localVersion ? `v${localVersion}` : 'unversioned'}, ` +
        `need v${version} — refreshing`,
    );
  }

  // 2. Prebuilt binary from the GitHub Release (npm/git release).
  if (supported) {
    try {
      log(`resolving prebuilt binary from release trang-native-v${version}`);
      await downloadRelease({ version, triple, destDir: nativeDir, releaseBase, log });
      log(`installed prebuilt v${version} binary into build/native/${triple}/`);
      return;
    } catch (err) {
      if (requirePrebuilt) {
        fail(
          `no usable prebuilt binary for v${version}/${triple} and ` +
            'TRANG_NATIVE_REQUIRE_PREBUILT=1 is set; refusing to build from source. ' +
            `Release probe failed with: ${err.message || err}`,
        );
      }
      log(
        `no usable release binary (${err.message || err}) — ` +
          `falling back to a source build`,
      );
    }
  } else {
    if (requirePrebuilt) {
      fail(
        `host triple ${triple} is outside the shipped matrix and ` +
          'TRANG_NATIVE_REQUIRE_PREBUILT=1 is set; refusing to build from source',
      );
    }
    log(`host triple ${triple} is outside the shipped matrix — building from source`);
  }

  // 3. From source.
  buildFromSource();
}

function buildFromSource() {
  log('building from source (requires GraalVM native-image + Apache Ant)');
  for (const script of ['fetch-source.mjs', 'build-jar.mjs', 'build-native.mjs']) {
    const result = spawnSync(
      process.execPath,
      [path.join(packageRoot, 'scripts', script)],
      { stdio: 'inherit', cwd: packageRoot },
    );
    if (result.status !== 0) {
      console.error(`[trang-native] ${script} failed (exit ${result.status})`);
      process.exit(result.status ?? 1);
    }
  }
  // Record provenance so the next `build` short-circuits at step 1.
  // (`package` later overwrites this with a fuller metadata.json.)
  mkdirSync(nativeDir, { recursive: true });
  writeFileSync(
    metadataPath,
    JSON.stringify(
      {
        name: '@epa-wg/trang-native',
        package_version: version,
        target: triple,
        source: 'native-image-from-source',
        built_at: new Date().toISOString(),
      },
      null,
      2,
    ) + '\n',
    'utf8',
  );
  log(`built v${version} from source into build/native/${triple}/`);
}

function log(msg) {
  console.log(`[trang-native] ${msg}`);
}

function fail(msg) {
  console.error(`[trang-native] ${msg}`);
  process.exit(1);
}
