import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import {
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
import { dirname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

export const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const workspaceRoot = resolve(projectRoot, '../..');
export const outputRoot = resolve(workspaceRoot, 'dist/packages/cem-ml-cli-native-windows-amd64');
export const buildRoot = resolve(outputRoot, 'build');
export const artifactRoot = resolve(outputRoot, 'artifacts');
export const cargoTargetRoot = resolve(workspaceRoot, 'dist/target/cem_ml_cli_native_windows_amd64');
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
        archive: `${base}.zip`,
        msi: `${base}.msi`,
        checksum: `${base}.sha256`,
        sbom: `${base}.spdx.json`,
        capability: `${base}.capabilities.json`,
        provenance: `${base}.provenance.json`,
        winget: `${base}.winget.json`,
        wingetInstaller: `${base}.winget-installer.yaml`,
        wingetLocale: `${base}.winget-defaultLocale.yaml`,
        wingetVersion: `${base}.winget-version.yaml`,
        releaseEntry: `${base}.release-index-entry.json`,
        signing: `${base}.signing.json`,
        attestation: `${base}.attestation.jsonl`,
    });
}

export function assertNativeHost() {
    if (process.platform !== deployment.host.platform || process.arch !== deployment.host.architecture) {
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
    const result = runResult(command, args, options);
    if (result.status !== 0) {
        const detail = result.stderr || result.stdout || result.error?.message || `exit ${result.status}`;
        throw new Error(`${command} ${args.join(' ')} failed: ${detail}`);
    }
    return result;
}

export function runResult(command, args, options = {}) {
    return spawnSync(command, args, {
        cwd: options.cwd ?? workspaceRoot,
        env: { ...process.env, ...options.env },
        encoding: 'utf8',
        stdio: options.stdio ?? 'inherit',
        timeout: options.timeout,
        windowsHide: true,
    });
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

export function copyFile(source, destination) {
    ensureDirectory(dirname(destination));
    copyFileSync(requireFile(source), destination);
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

export function requireFile(path, label = path) {
    if (!statSync(path, { throwIfNoEntry: false })?.isFile()) {
        throw new Error(`missing ${label}: ${path}`);
    }
    return path;
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

export async function acquireSyft() {
    const { version, windowsAmd64ArchiveSha256 } = deployment.syft;
    const toolRoot = resolve(workspaceRoot, `dist/tools/syft/${version}/windows-amd64`);
    const executable = resolve(toolRoot, 'syft.exe');
    if (existsSync(executable)) {
        assertSyftVersion(executable, version);
        return executable;
    }
    resetDirectory(toolRoot);
    const archiveName = `syft_${version}_windows_amd64.zip`;
    const archive = resolve(toolRoot, archiveName);
    const url = `https://github.com/anchore/syft/releases/download/v${version}/${archiveName}`;
    const response = await fetch(url);
    if (!response.ok) throw new Error(`failed to download pinned Syft ${version}: HTTP ${response.status}`);
    writeFileSync(archive, new Uint8Array(await response.arrayBuffer()));
    const actual = sha256File(archive);
    if (actual !== windowsAmd64ArchiveSha256) {
        throw new Error(`pinned Syft archive digest mismatch: expected ${windowsAmd64ArchiveSha256}, got ${actual}`);
    }
    run('tar.exe', ['-xf', archive, '-C', toolRoot]);
    rmSync(archive);
    assertSyftVersion(executable, version);
    return executable;
}

export function artifactPath(name) {
    return resolve(artifactRoot, name);
}

export function productCode(version = authoritativeVersion()) {
    return `{${uuidV5(
        deployment.windowsInstaller.productCodeNamespace,
        `${deployment.windowsInstaller.packageIdentifier}:${version}`,
    ).toUpperCase()}}`;
}

export function packageCode(identity) {
    return `{${uuidV5(deployment.windowsInstaller.packageCodeNamespace, identity).toUpperCase()}}`;
}

export function authenticodeSignature(path) {
    const script = [
        '$signature = Get-AuthenticodeSignature -LiteralPath $env:CEM_ML_SIGNATURE_PATH',
        '[pscustomobject]@{',
        'status = $signature.Status.ToString()',
        'statusMessage = $signature.StatusMessage',
        'signerSubject = $signature.SignerCertificate.Subject',
        'signerThumbprint = $signature.SignerCertificate.Thumbprint',
        'timeStamperSubject = $signature.TimeStamperCertificate.Subject',
        'timeStamperThumbprint = $signature.TimeStamperCertificate.Thumbprint',
        '} | ConvertTo-Json -Compress',
    ].join('\n');
    return JSON.parse(
        capture('pwsh.exe', ['-NoLogo', '-NoProfile', '-NonInteractive', '-Command', script], {
            env: { CEM_ML_SIGNATURE_PATH: path },
        }),
    );
}

export function assertValidAuthenticode(path, label = path) {
    const signature = authenticodeSignature(path);
    if (signature.status !== 'Valid') {
        throw new Error(`${label} Authenticode status is ${signature.status}: ${signature.statusMessage}`);
    }
    if (!nonEmptyString(signature.signerSubject) || !nonEmptyString(signature.signerThumbprint)) {
        throw new Error(`${label} has no Authenticode signer identity`);
    }
    if (!nonEmptyString(signature.timeStamperSubject) || !nonEmptyString(signature.timeStamperThumbprint)) {
        throw new Error(`${label} has no trusted Authenticode timestamp`);
    }
    return signature;
}

export function assertUnsignedAuthenticode(path, label = path) {
    const signature = authenticodeSignature(path);
    if (signature.status !== 'NotSigned') {
        throw new Error(`${label} expected unsigned-local state, got ${signature.status}`);
    }
    return signature;
}

export function assertPeAmd64(path) {
    const { image, peOffset } = readPeImage(path);
    const machine = image.readUInt16LE(peOffset + 4);
    if (machine !== 0x8664) {
        throw new Error(`${path} PE machine is 0x${machine.toString(16)}, expected AMD64 0x8664`);
    }
}

export function assertStaticMsvcRuntime(path, label = path) {
    const dynamicRuntimeImports = peImportedDlls(path).filter((name) =>
        /^(?:api-ms-win-crt-.+|ucrtbase(?:d)?|vcruntime.*|msvcp.*|msvcr.*)\.dll$/i.test(name),
    );
    if (dynamicRuntimeImports.length > 0) {
        throw new Error(`${label} dynamically imports the MSVC runtime: ${dynamicRuntimeImports.join(', ')}`);
    }
}

export function peImportedDlls(path) {
    const { image, peOffset } = readPeImage(path);
    const sectionCount = image.readUInt16LE(peOffset + 6);
    const optionalHeaderSize = image.readUInt16LE(peOffset + 20);
    const optionalHeaderOffset = peOffset + 24;
    requireBufferRange(image, optionalHeaderOffset, optionalHeaderSize, `${path} optional PE header`);
    const magic = image.readUInt16LE(optionalHeaderOffset);
    if (magic !== 0x20b) throw new Error(`${path} is not a PE32+ executable`);

    const dataDirectoryOffset = optionalHeaderOffset + 112;
    requireBufferRange(image, dataDirectoryOffset, 16, `${path} PE data directories`);
    const importTableRva = image.readUInt32LE(dataDirectoryOffset + 8);
    if (importTableRva === 0) return [];

    const sectionTableOffset = optionalHeaderOffset + optionalHeaderSize;
    requireBufferRange(image, sectionTableOffset, sectionCount * 40, `${path} PE section table`);
    const rvaToOffset = (rva) => {
        for (let index = 0; index < sectionCount; index += 1) {
            const sectionOffset = sectionTableOffset + index * 40;
            const virtualSize = image.readUInt32LE(sectionOffset + 8);
            const virtualAddress = image.readUInt32LE(sectionOffset + 12);
            const rawSize = image.readUInt32LE(sectionOffset + 16);
            const rawOffset = image.readUInt32LE(sectionOffset + 20);
            if (rva >= virtualAddress && rva < virtualAddress + Math.max(virtualSize, rawSize)) {
                const offset = rawOffset + (rva - virtualAddress);
                requireBufferRange(image, offset, 1, `${path} PE RVA 0x${rva.toString(16)}`);
                return offset;
            }
        }
        throw new Error(`${path} PE RVA 0x${rva.toString(16)} is outside its sections`);
    };

    const imports = [];
    let descriptorOffset = rvaToOffset(importTableRva);
    for (let index = 0; index < 4096; index += 1) {
        requireBufferRange(image, descriptorOffset, 20, `${path} PE import descriptor`);
        const fields = Array.from({ length: 5 }, (_, field) => image.readUInt32LE(descriptorOffset + field * 4));
        if (fields.every((value) => value === 0)) return [...new Set(imports)].sort();
        imports.push(readNullTerminatedAscii(image, rvaToOffset(fields[3]), `${path} PE import name`));
        descriptorOffset += 20;
    }
    throw new Error(`${path} PE import table has no terminator`);
}

function readPeImage(path) {
    const image = readFileSync(path);
    if (image.length < 0x40 || image.toString('ascii', 0, 2) !== 'MZ') {
        throw new Error(`${path} is not a PE executable`);
    }
    const peOffset = image.readUInt32LE(0x3c);
    requireBufferRange(image, peOffset, 24, `${path} PE header`);
    if (image.toString('ascii', peOffset, peOffset + 4) !== 'PE\0\0') {
        throw new Error(`${path} has no PE header`);
    }
    return { image, peOffset };
}

function readNullTerminatedAscii(image, offset, label) {
    const limit = Math.min(image.length, offset + 4096);
    let end = offset;
    while (end < limit && image[end] !== 0) end += 1;
    if (end === limit) throw new Error(`${label} is not null-terminated`);
    return image.toString('ascii', offset, end);
}

function requireBufferRange(image, offset, length, label) {
    if (!Number.isSafeInteger(offset) || offset < 0 || length < 0 || offset + length > image.length) {
        throw new Error(`${label} is outside the executable image`);
    }
}

function assertSyftVersion(executable, version) {
    const output = capture(executable, ['version']);
    if (!new RegExp(`^Version:\\s+${version.replaceAll('.', '\\.')}\\s*$`, 'm').test(output)) {
        throw new Error(`expected Syft ${version}, got ${output.trim()}`);
    }
}

function assertOutputPath(path) {
    const normalized = resolve(path);
    if (
        normalized !== outputRoot &&
        !normalized.startsWith(`${outputRoot}${sep}`) &&
        !normalized.startsWith(`${resolve(workspaceRoot, 'dist/tools')}${sep}`)
    ) {
        throw new Error(`refusing to reset path outside native deployment outputs: ${normalized}`);
    }
}

function uuidV5(namespace, name) {
    const namespaceBytes = Buffer.from(namespace.replaceAll('-', ''), 'hex');
    if (namespaceBytes.byteLength !== 16) throw new Error(`invalid UUID namespace ${namespace}`);
    const digest = createHash('sha1').update(namespaceBytes).update(name).digest().subarray(0, 16);
    digest[6] = (digest[6] & 0x0f) | 0x50;
    digest[8] = (digest[8] & 0x3f) | 0x80;
    const hex = digest.toString('hex');
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

function nonEmptyString(value) {
    return typeof value === 'string' && value.trim().length > 0;
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
