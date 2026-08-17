import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptPath = fileURLToPath(import.meta.url);

export function emitNpmReleaseEvidence({
    workspaceRoot,
    projectRoot,
    outputRoot,
    archivePath,
    coordinate,
    runtimeManifestPath,
    integrityManifestPath,
}) {
    const packageMetadata = readJson(resolve(projectRoot, 'package.json'));
    const version = authoritativeVersion(workspaceRoot);
    if (packageMetadata.version !== version) {
        throw new Error(`${packageMetadata.name}@${packageMetadata.version} drifted from CEM-ML ${version}`);
    }
    const runtimeManifest = readJson(runtimeManifestPath);
    const integrityManifest = readJson(integrityManifestPath);
    if (runtimeManifest.commonVersion !== version || integrityManifest.commonVersion !== version) {
        throw new Error(`${packageMetadata.name} runtime/integrity metadata drifted from CEM-ML ${version}`);
    }

    const sourceRevision = sourceCommit(workspaceRoot);
    const sourceDateEpoch = sourceEpoch(workspaceRoot);
    const base = `cem-ml-${version}-${coordinate}`;
    const names = {
        archive: `${base}.tgz`,
        capability: `${base}.capabilities.json`,
        integrity: `${base}.integrity.json`,
        sbom: `${base}.spdx.json`,
        provenance: `${base}.provenance.json`,
        releaseEntry: `${base}.release-index-entry.json`,
        checksum: `${base}.sha256`,
        signing: `${base}.signing.json`,
        attestation: `${base}.attestation.jsonl`,
    };
    const artifactRoot = resolve(outputRoot, 'artifacts');
    rmSync(artifactRoot, { recursive: true, force: true });
    mkdirSync(artifactRoot, { recursive: true });
    copyFileSync(archivePath, resolve(artifactRoot, names.archive));
    copyFileSync(runtimeManifestPath, resolve(artifactRoot, names.capability));
    copyFileSync(integrityManifestPath, resolve(artifactRoot, names.integrity));

    const archiveSha256 = sha256File(resolve(artifactRoot, names.archive));
    writeJson(resolve(artifactRoot, names.sbom), {
        spdxVersion: 'SPDX-2.3',
        dataLicense: 'CC0-1.0',
        SPDXID: 'SPDXRef-DOCUMENT',
        name: `${packageMetadata.name}-${version}`,
        documentNamespace: `https://cem.dev/spdx/cem-ml/${version}/${coordinate}/${archiveSha256}`,
        creationInfo: {
            created: new Date(sourceDateEpoch * 1000).toISOString().replace('.000Z', 'Z'),
            creators: [`Tool: nx:${packageMetadata.name}:package`],
        },
        packages: [
            {
                SPDXID: 'SPDXRef-Package',
                name: packageMetadata.name,
                versionInfo: version,
                downloadLocation: 'NOASSERTION',
                filesAnalyzed: false,
                checksums: [{ algorithm: 'SHA256', checksumValue: archiveSha256 }],
                licenseConcluded: 'NOASSERTION',
                licenseDeclared: packageMetadata.license ?? 'NOASSERTION',
                copyrightText: 'NOASSERTION',
                externalRefs: [
                    {
                        referenceCategory: 'PACKAGE-MANAGER',
                        referenceType: 'purl',
                        referenceLocator: npmPurl(packageMetadata.name, version),
                    },
                ],
            },
        ],
        relationships: [
            {
                spdxElementId: 'SPDXRef-DOCUMENT',
                relationshipType: 'DESCRIBES',
                relatedSpdxElement: 'SPDXRef-Package',
            },
        ],
    });

    const primarySubjects = [names.archive, names.capability, names.integrity, names.sbom].map((filename) =>
        artifactRecord(artifactRoot, filename),
    );
    writeJson(resolve(artifactRoot, names.provenance), {
        schemaVersion: 1,
        predicateType: 'https://slsa.dev/provenance/v1',
        status: 'unsigned-build-record',
        builder: { id: `nx:${packageMetadata.name}:package`, runner: 'github-ubuntu-24.04-x64' },
        buildDefinition: {
            buildType: 'https://cem.dev/build/npm-package/yarn-v1',
            externalParameters: {
                packageName: packageMetadata.name,
                packageVersion: version,
                coordinate,
                immutableArchive: names.archive,
            },
            resolvedDependencies: [
                { uri: 'git+https://github.com/EPA-WG/cem.git', digest: { gitCommit: sourceRevision } },
                { uri: 'file:yarn.lock', digest: { sha256: sha256File(resolve(workspaceRoot, 'yarn.lock')) } },
            ],
        },
        runDetails: { metadata: { sourceDateEpoch } },
        subject: primarySubjects,
    });

    const releaseArtifacts = [...primarySubjects, artifactRecord(artifactRoot, names.provenance)];
    const runtimeIdentities =
        runtimeManifest.runtimeIdentities ??
        Object.values(runtimeManifest.capabilities ?? {}).map(({ runtime }) => runtime);
    const targetIdentities =
        runtimeManifest.targetIdentities ??
        Object.values(runtimeManifest.capabilities ?? {}).map(({ targetIdentity }) => targetIdentity);
    const abiIdentities = [
        ...new Set(
            Object.values(runtimeManifest.capabilities ?? {})
                .map(({ abiIdentity }) => abiIdentity)
                .filter(Boolean),
        ),
    ];
    writeJson(resolve(artifactRoot, names.releaseEntry), {
        schemaVersion: 1,
        product: 'cem-ml',
        commonVersion: version,
        sourceCommit: sourceRevision,
        releaseTag: `cem-ml-v${version}`,
        npmIdentity: packageMetadata.name,
        runtimeIdentities: [...new Set(runtimeIdentities)].sort(),
        targetIdentities: [...new Set(targetIdentities)].sort(),
        abiIdentities: abiIdentities.sort(),
        capabilityManifestDigest: sha256File(resolve(artifactRoot, names.capability)),
        integrityManifestDigest: sha256File(resolve(artifactRoot, names.integrity)),
        artifacts: releaseArtifacts,
        checksumManifest: names.checksum,
        signingRecord: names.signing,
        publicationState: 'staged-local',
    });
    const checksummed = [...releaseArtifacts.map(({ filename }) => filename), names.releaseEntry].sort();
    writeFileSync(
        resolve(artifactRoot, names.checksum),
        `${checksummed.map((filename) => `${sha256File(resolve(artifactRoot, filename))}  ${filename}`).join('\n')}\n`,
    );
    writeJson(resolve(artifactRoot, names.signing), {
        schemaVersion: 1,
        product: 'cem-ml',
        commonVersion: version,
        npmIdentity: packageMetadata.name,
        releaseTag: `cem-ml-v${version}`,
        checksumManifest: {
            filename: names.checksum,
            sha256: sha256File(resolve(artifactRoot, names.checksum)),
        },
        githubArtifactAttestation: { status: 'awaiting-github-oidc', bundle: null, sha256: null },
        publicationReady: false,
        mode: 'unsigned-local',
    });
    return {
        artifactRoot,
        names,
        version,
        sourceCommit: sourceRevision,
        releaseTag: `cem-ml-v${version}`,
        packageName: packageMetadata.name,
    };
}

export function attestNpmReleaseEvidence({ workspaceRoot, packageName }) {
    const outputDirectory =
        packageName === '@epa-wg/cem-ml'
            ? 'cem-ml-npm'
            : packageName === '@epa-wg/cem-ml-cli'
              ? 'cem-ml-cli-npm'
              : null;
    if (!outputDirectory) throw new Error(`unsupported CEM-ML npm release identity: ${packageName}`);
    const artifactRoot = resolve(workspaceRoot, 'dist/packages', outputDirectory, 'artifacts');
    const entryFilename = readdirSync(artifactRoot).find((filename) => filename.endsWith('.release-index-entry.json'));
    if (!entryFilename) throw new Error(`missing ${packageName} release-index entry`);
    const entry = readJson(resolve(artifactRoot, entryFilename));
    if (entry.npmIdentity !== packageName) throw new Error(`${packageName} release-index identity drift`);
    const signingPath = resolve(artifactRoot, entry.signingRecord);
    const signing = readJson(signingPath);
    const suppliedAttestation = process.env.CEM_ML_GITHUB_ATTESTATION_BUNDLE;
    if (suppliedAttestation) {
        const attestationBundle = requireFile(suppliedAttestation, 'GitHub artifact-attestation bundle');
        for (const subject of releaseAttestationSubjects({ artifactRoot, entry })) {
            run(
                'gh',
                ['attestation', 'verify', subject, '--repo', 'EPA-WG/cem', '--bundle', attestationBundle],
                workspaceRoot,
            );
        }
        const attestationFilename = entry.signingRecord.replace('.signing.json', '.attestation.jsonl');
        const attestationPath = resolve(artifactRoot, attestationFilename);
        copyFileSync(suppliedAttestation, attestationPath);
        signing.githubArtifactAttestation = {
            status: 'verified',
            bundle: attestationFilename,
            sha256: sha256File(attestationPath),
        };
        signing.publicationReady = true;
        signing.mode = 'release';
        writeJson(signingPath, signing);
    }
    if (process.env.CEM_ML_RELEASE_SIGNING === 'required' && signing.publicationReady !== true) {
        throw new Error(`${packageName} release signing requires CEM_ML_GITHUB_ATTESTATION_BUNDLE`);
    }
    return { artifactRoot, entry, signing };
}

export function releaseAttestationSubjects({ artifactRoot, entry }) {
    const checksumPath = resolve(artifactRoot, entry.checksumManifest);
    const lines = readFileSync(checksumPath, 'utf8').trim().split('\n');
    const subjects = lines.map((line) => {
        const filename = line.match(/^[a-f0-9]{64} {2}([^/]+)$/)?.[1];
        if (!filename) throw new Error(`${entry.npmIdentity} checksum manifest contains an invalid subject`);
        return requireFile(resolve(artifactRoot, filename), 'attestation subject');
    });
    if (subjects.length === 0) throw new Error(`${entry.npmIdentity} checksum manifest has no attestation subjects`);
    return subjects;
}

function authoritativeVersion(workspaceRoot) {
    const manifest = readFileSync(resolve(workspaceRoot, 'packages/cem_ml/Cargo.toml'), 'utf8');
    const version = manifest.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];
    if (!version) throw new Error('cannot read the common CEM-ML Cargo version');
    return version;
}

function sourceCommit(workspaceRoot) {
    return capture('git', ['rev-parse', 'HEAD'], workspaceRoot).trim();
}

function sourceEpoch(workspaceRoot) {
    const configured = process.env.SOURCE_DATE_EPOCH;
    const value = configured ?? capture('git', ['show', '-s', '--format=%ct', 'HEAD'], workspaceRoot).trim();
    if (!/^[1-9]\d*$/.test(value)) throw new Error(`invalid SOURCE_DATE_EPOCH ${value}`);
    return Number(value);
}

function capture(command, args, cwd) {
    const result = spawnSync(command, args, { cwd, encoding: 'utf8', stdio: 'pipe' });
    if (result.status !== 0) throw new Error(`${command} ${args.join(' ')} failed: ${result.stderr}`);
    return result.stdout;
}

function run(command, args, cwd) {
    const result = spawnSync(command, args, { cwd, encoding: 'utf8', stdio: 'inherit' });
    if (result.status !== 0) throw new Error(`${command} ${args.join(' ')} failed with status ${result.status}`);
}

function artifactRecord(root, filename) {
    const path = resolve(root, filename);
    return { filename, byteLength: statSync(path).size, sha256: sha256File(path) };
}

function npmPurl(name, version) {
    const locator = name.startsWith('@') ? `%40${name.slice(1)}` : name;
    return `pkg:npm/${locator}@${version}`;
}

function sha256File(path) {
    return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function requireFile(path, label = path) {
    if (!statSync(path, { throwIfNoEntry: false })?.isFile()) throw new Error(`missing ${label}: ${path}`);
    return path;
}

function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
    writeFileSync(path, `${JSON.stringify(sortValue(value), null, 2)}\n`);
}

function sortValue(value) {
    if (Array.isArray(value)) return value.map(sortValue);
    if (value && typeof value === 'object') {
        return Object.fromEntries(
            Object.entries(value)
                .sort(([left], [right]) => left.localeCompare(right))
                .map(([key, child]) => [key, sortValue(child)]),
        );
    }
    return value;
}

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
    const [command, packageName] = process.argv.slice(2);
    if (command !== 'attest' || !packageName || process.argv.length !== 4) {
        throw new Error(
            'usage: node tools/scripts/cem-ml-npm-release-evidence.mjs attest <@epa-wg/cem-ml|@epa-wg/cem-ml-cli>',
        );
    }
    const result = attestNpmReleaseEvidence({ workspaceRoot: resolve(dirname(scriptPath), '../..'), packageName });
    console.log(
        result.signing.publicationReady
            ? `Verified publication-ready attestation for ${packageName}.`
            : `Recorded unsigned-local attestation state for ${packageName}.`,
    );
}
