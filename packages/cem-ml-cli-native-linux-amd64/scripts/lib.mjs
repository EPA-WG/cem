import { createHash } from 'node:crypto';
import {
    chmodSync,
    copyFileSync,
    existsSync,
    mkdirSync,
    readFileSync,
    readdirSync,
    rmSync,
    statSync,
    utimesSync,
    writeFileSync,
} from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

export const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const workspaceRoot = resolve(projectRoot, '../..');
export const outputRoot = resolve(
    workspaceRoot,
    'dist/packages/cem-ml-cli-native-linux-amd64',
);
export const compileRoot = resolve(outputRoot, 'compiled');
export const buildRoot = resolve(outputRoot, 'build');
export const artifactRoot = resolve(outputRoot, 'artifacts');
export const cargoTargetRoot = resolve(
    workspaceRoot,
    'dist/target/cem_ml_cli_native_linux_amd64',
);
export const deployment = readJson(resolve(projectRoot, 'deployment.json'));

export function authoritativeVersion() {
    const version = cargoPackageVersion(resolve(workspaceRoot, 'packages/cem_ml/Cargo.toml'));
    if (deployment.commonVersion !== version) {
        throw new Error(`deployment commonVersion ${deployment.commonVersion} drifted from Cargo ${version}`);
    }
    return version;
}

export function cargoPackageVersion(path) {
    const manifest = readFileSync(path, 'utf8');
    const packageStart = manifest.indexOf('[package]');
    if (packageStart < 0) throw new Error(`cannot find [package] in ${path}`);
    const sectionStart = packageStart + '[package]'.length;
    const nextSection = manifest.indexOf('\n[', sectionStart);
    const packageSection = manifest.slice(sectionStart, nextSection < 0 ? undefined : nextSection);
    const version = packageSection.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
    if (version === undefined) throw new Error(`cannot resolve package version from ${path}`);
    return version;
}

export function assetNames(version = authoritativeVersion()) {
    const base = `cem-ml-${version}-${deployment.assetCoordinate}`;
    return Object.freeze({
        base,
        archive: `${base}.tar.gz`,
        deb: `${base}.deb`,
        checksum: `${base}.sha256`,
        checksumSignature: `${base}.sha256.asc`,
        sbom: `${base}.spdx.json`,
        capability: `${base}.capabilities.json`,
        provenance: `${base}.provenance.json`,
        apt: `${base}.apt.json`,
        releaseEntry: `${base}.release-index-entry.json`,
        signing: `${base}.signing.json`,
        attestation: `${base}.attestation.jsonl`,
    });
}

export function assertNativeHost() {
    if (
        process.platform !== deployment.host.platform ||
        process.arch !== deployment.host.architecture
    ) {
        throw new Error(
            `${deployment.nxProject} requires ${deployment.host.platform}/${deployment.host.architecture}; ` +
                `current host is ${process.platform}/${process.arch}`,
        );
    }
}

export function sourceCommit() {
    return capture('git', ['rev-parse', 'HEAD'], { cwd: workspaceRoot }).trim();
}

export function sourceEpoch() {
    const configured = process.env.SOURCE_DATE_EPOCH;
    const value = configured ?? capture('git', ['show', '-s', '--format=%ct', 'HEAD'], { cwd: workspaceRoot }).trim();
    if (!/^[1-9][0-9]*$/.test(value)) throw new Error(`invalid SOURCE_DATE_EPOCH ${value}`);
    return Number(value);
}

export function releaseTag(version = authoritativeVersion()) {
    return `cem-ml-v${version}`;
}

export function releaseAssetUrl(filename, version = authoritativeVersion()) {
    return `https://github.com/EPA-WG/cem/releases/download/${releaseTag(version)}/${filename}`;
}

export function run(command, args, options = {}) {
    const result = spawnSync(command, args, {
        cwd: options.cwd ?? workspaceRoot,
        env: { ...process.env, ...options.env },
        encoding: 'utf8',
        stdio: options.stdio ?? 'inherit',
    });
    if (result.status !== 0) {
        const detail =
            result.stderr || result.stdout || result.error?.message || `exit ${result.status}`;
        throw new Error(`${command} ${args.join(' ')} failed: ${detail}`);
    }
    return result;
}

export function capture(command, args, options = {}) {
    return run(command, args, { ...options, stdio: 'pipe' }).stdout;
}

export function resetDirectory(path) {
    assertOutputPath(path);
    rmSync(path, { recursive: true, force: true });
    mkdirSync(path, { recursive: true });
}

export function ensureDirectory(path) {
    mkdirSync(path, { recursive: true });
}

export function copyExecutable(source, destination) {
    ensureDirectory(dirname(destination));
    copyFileSync(source, destination);
    chmodSync(destination, 0o755);
}

export function writeJson(path, value) {
    ensureDirectory(dirname(path));
    writeFileSync(path, `${JSON.stringify(sortValue(value), null, 2)}\n`);
}

export function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}

export function sha256File(path) {
    return createHash('sha256').update(readFileSync(path)).digest('hex');
}

export function sha256Text(value) {
    return createHash('sha256').update(value).digest('hex');
}

export function requireFile(path, label = path) {
    if (!statSync(path, { throwIfNoEntry: false })?.isFile()) {
        throw new Error(`missing ${label}: ${path}`);
    }
    return path;
}

export function setTreeTimestamp(root, epoch = sourceEpoch()) {
    const timestamp = new Date(epoch * 1000);
    const visit = (path) => {
        const stat = statSync(path);
        if (stat.isDirectory()) {
            for (const entry of readdirSync(path)) visit(resolve(path, entry));
        }
        utimesSync(path, timestamp, timestamp);
    };
    visit(root);
}

export function listFiles(root) {
    const files = [];
    const visit = (directory) => {
        for (const entry of readdirSync(directory, { withFileTypes: true })) {
            const path = resolve(directory, entry.name);
            if (entry.isDirectory()) visit(path);
            else if (entry.isFile()) files.push(path);
        }
    };
    visit(root);
    return files.sort();
}

export async function acquireSyft() {
    const { version, linuxAmd64ArchiveSha256 } = deployment.syft;
    const toolRoot = resolve(workspaceRoot, `dist/tools/syft/${version}/linux-amd64`);
    const executable = resolve(toolRoot, 'syft');
    if (existsSync(executable)) {
        assertSyftVersion(executable, version);
        return executable;
    }
    resetDirectory(toolRoot);
    const archiveName = `syft_${version}_linux_amd64.tar.gz`;
    const archive = resolve(toolRoot, archiveName);
    const url = `https://github.com/anchore/syft/releases/download/v${version}/${archiveName}`;
    const response = await fetch(url);
    if (!response.ok) throw new Error(`failed to download pinned Syft ${version}: HTTP ${response.status}`);
    writeFileSync(archive, new Uint8Array(await response.arrayBuffer()));
    const actual = sha256File(archive);
    if (actual !== linuxAmd64ArchiveSha256) {
        throw new Error(`pinned Syft archive digest mismatch: expected ${linuxAmd64ArchiveSha256}, got ${actual}`);
    }
    run('tar', ['-xzf', archive, '-C', toolRoot]);
    rmSync(archive);
    chmodSync(executable, 0o755);
    assertSyftVersion(executable, version);
    return executable;
}

export function artifactPath(name) {
    return resolve(artifactRoot, name);
}

function assertSyftVersion(executable, version) {
    const output = capture(executable, ['version']);
    if (!new RegExp(`^Version:\\s+${version.replaceAll('.', '\\.')}\\s*$`, 'm').test(output)) {
        throw new Error(`expected Syft ${version}, got ${output.trim()}`);
    }
}

function assertOutputPath(path) {
    const normalized = resolve(path);
    if (normalized !== outputRoot && !normalized.startsWith(`${outputRoot}/`) && !normalized.startsWith(`${workspaceRoot}/dist/tools/`)) {
        throw new Error(`refusing to reset path outside native deployment outputs: ${normalized}`);
    }
}

function sortValue(value) {
    if (Array.isArray(value)) return value.map(sortValue);
    if (value !== null && typeof value === 'object') {
        return Object.fromEntries(
            Object.entries(value)
                .sort(([left], [right]) => left.localeCompare(right))
                .map(([key, child]) => [key, sortValue(child)]),
        );
    }
    return value;
}
