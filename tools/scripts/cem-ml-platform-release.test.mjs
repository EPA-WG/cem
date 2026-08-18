import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import { attestNpmReleaseEvidence, releaseAttestationSubjects } from './cem-ml-npm-release-evidence.mjs';
import {
    releaseGpgSigningInvocation,
    releasePublicationReady,
} from '../../packages/cem-ml-cli-native-linux-amd64/scripts/lib.mjs';
import {
    ciReleaseUnits,
    createOrResumePlatformReleaseDraft,
    expectedReleaseUnits,
    finalizeCiProducerEvidence,
    githubDraftCreateArguments,
    preflightNativeHostRelease,
    recordCiProducerEvidence,
    stagePlatformRelease,
    uploadImmutableDraftAssetSet,
    uploadPlatformReleaseUnits,
    uploadPlatformReleaseDraft,
    validateCiProducerEvidence,
    verifyPlatformReleaseUnit,
    verifyPlatformRelease,
} from './cem-ml-platform-release.mjs';

const workspaceRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const version = '3.4.5';
const sourceCommit = '1234567890abcdef1234567890abcdef12345678';
const releaseTag = `cem-ml-v${version}`;
const releaseTitle = `CEM-ML ${version}`;
const nativeDeploymentPaths = new Map([
    ['native-linux-amd64', 'packages/cem-ml-cli-native-linux-amd64/deployment.json'],
    ['native-macos-arm64', 'packages/cem-ml-cli-native-brew-arm64/deployment.json'],
    ['native-windows-amd64', 'packages/cem-ml-cli-native-windows-amd64/deployment.json'],
]);

test('GitHub Release scope contains only the two WASM/npm units and Linux AMD64', () => {
    const identities = ['@epa-wg/cem-ml', '@epa-wg/cem-ml-cli', 'native-linux-amd64'];
    assert.deepEqual(
        expectedReleaseUnits.map(({ identity }) => identity),
        identities,
    );
    assert.deepEqual(
        ciReleaseUnits.map(({ identity }) => identity),
        identities,
    );
});

test('absent GitHub release creates one notes-bounded draft without a publish path', () => {
    const github = createFakeGithubReleaseClient();
    const result = createOrResumePlatformReleaseDraft({
        authorized: true,
        version,
        sourceCommit,
        releaseTag,
        taggedCommit: sourceCommit,
        previousReleaseTag: 'cem-ml-v3.4.4',
        github,
    });

    assert.equal(result.action, 'created');
    assert.deepEqual(github.createRequests, [
        {
            tag: releaseTag,
            title: releaseTitle,
            draft: true,
            prerelease: false,
            generateNotes: true,
            notesStartTag: 'cem-ml-v3.4.4',
        },
    ]);
    assert.equal(github.publishCalls, 0, 'draft coordinator must not expose publication');
});

test('GitHub CLI draft creation uses the existing tag and bounded generated notes', () => {
    assert.deepEqual(
        githubDraftCreateArguments({
            tag: releaseTag,
            title: releaseTitle,
            draft: true,
            prerelease: false,
            generateNotes: true,
            notesStartTag: 'cem-ml-v3.4.4',
        }),
        [
            'release',
            'create',
            releaseTag,
            '--verify-tag',
            '--draft',
            '--title',
            releaseTitle,
            '--generate-notes',
            '--notes-start-tag',
            'cem-ml-v3.4.4',
        ],
    );
});

test('identical existing GitHub draft resumes without mutation', () => {
    const existing = githubDraft();
    const github = createFakeGithubReleaseClient(existing);
    const result = createOrResumePlatformReleaseDraft({
        authorized: true,
        version,
        sourceCommit,
        releaseTag,
        taggedCommit: sourceCommit,
        previousReleaseTag: 'cem-ml-v3.4.4',
        github,
    });

    assert.equal(result.action, 'resumed');
    assert.deepEqual(result.release, existing);
    assert.deepEqual(github.createRequests, []);
    assert.equal(github.publishCalls, 0);
});

for (const [label, release, message] of [
    ['wrong tag', githubDraft({ tagName: 'cem-ml-v3.4.4' }), /tag drift/],
    ['published release', githubDraft({ isDraft: false }), /must remain a draft/],
    ['wrong title', githubDraft({ name: 'Unexpected title' }), /title drift/],
    ['wrong prerelease state', githubDraft({ isPrerelease: true }), /prerelease drift/],
]) {
    test(`draft coordinator rejects ${label}`, () => {
        const github = createFakeGithubReleaseClient(release);
        assert.throws(
            () =>
                createOrResumePlatformReleaseDraft({
                    authorized: true,
                    version,
                    sourceCommit,
                    releaseTag,
                    taggedCommit: sourceCommit,
                    github,
                }),
            message,
        );
        assert.deepEqual(github.createRequests, []);
        assert.equal(github.publishCalls, 0);
    });
}

test('draft coordinator rejects a tag that does not resolve to the checked-out source commit', () => {
    const github = createFakeGithubReleaseClient();
    assert.throws(
        () =>
            createOrResumePlatformReleaseDraft({
                authorized: true,
                version,
                sourceCommit,
                releaseTag,
                taggedCommit: 'f'.repeat(40),
                github,
            }),
        /tagged source-commit drift/,
    );
    assert.deepEqual(github.createRequests, []);
});

test('draft coordinator rejects a manual tag outside the exact CEM-ML version contract', () => {
    const github = createFakeGithubReleaseClient();
    assert.throws(
        () =>
            createOrResumePlatformReleaseDraft({
                authorized: true,
                version,
                sourceCommit,
                releaseTag: `cem-ml-v${version}-unexpected`,
                taggedCommit: sourceCommit,
                github,
            }),
        /release tag drift/,
    );
    assert.deepEqual(github.createRequests, []);
});

test('draft coordinator remains inert without protected release authorization', () => {
    const github = createFakeGithubReleaseClient();
    assert.throws(
        () =>
            createOrResumePlatformReleaseDraft({
                authorized: false,
                version,
                sourceCommit,
                releaseTag,
                taggedCommit: sourceCommit,
                github,
            }),
        /draft creation is disabled/,
    );
    assert.deepEqual(github.createRequests, []);
});

for (const [identity, platform, architecture] of [['native-linux-amd64', 'linux', 'x64']]) {
    test(`${identity} release preflight accepts only its exact tagged source and draft`, () => {
        const github = createFakeGithubReleaseClient(githubDraft());
        const result = preflightNativeHostRelease({
            identity,
            version,
            sourceCommit,
            releaseTag,
            taggedCommit: sourceCommit,
            sourceTreeStatus: '',
            platform,
            architecture,
            deployment: nativeDeployment(identity),
            github,
        });

        assert.equal(result.identity, identity);
        assert.equal(result.releaseTag, releaseTag);
        assert.equal(result.sourceCommit, sourceCommit);
        assert.equal(result.release.isDraft, true);
        assert.deepEqual(github.viewRequests, [releaseTag]);
        assert.deepEqual(github.createRequests, []);
        assert.equal(github.publishCalls, 0);
    });
}

for (const [identity, platform, architecture] of [
    ['native-macos-arm64', 'darwin', 'arm64'],
    ['native-windows-amd64', 'win32', 'x64'],
]) {
    test(`${identity} release preflight is disabled outside the GitHub Release contract`, () => {
        const github = createFakeGithubReleaseClient(githubDraft());
        assert.throws(
            () =>
                preflightNativeHostRelease({
                    identity,
                    version,
                    sourceCommit,
                    releaseTag,
                    taggedCommit: sourceCommit,
                    sourceTreeStatus: '',
                    platform,
                    architecture,
                    deployment: nativeDeployment(identity),
                    github,
                }),
            /native release-unit contract is missing/,
        );
        assert.deepEqual(github.viewRequests, []);
    });
}

for (const [label, overrides, message] of [
    ['missing requested tag', { releaseTag: '' }, /CEM_ML_RELEASE_TAG is required/],
    ['wrong requested tag', { releaseTag: `${releaseTag}-unexpected` }, /release tag drift/],
    ['different tagged commit', { taggedCommit: 'f'.repeat(40) }, /tagged source-commit drift/],
    ['dirty source tree', { sourceTreeStatus: ' M docs/todo.md' }, /clean source tree/],
    ['wrong host architecture', { architecture: 'arm64' }, /host architecture drift/],
]) {
    test(`native-host release preflight rejects ${label} before reading GitHub`, () => {
        const github = createFakeGithubReleaseClient(githubDraft());
        assert.throws(() => preflightNativeHostRelease(nativePreflightArguments({ ...overrides, github })), message);
        assert.deepEqual(github.viewRequests, []);
        assert.deepEqual(github.createRequests, []);
        assert.equal(github.publishCalls, 0);
    });
}

for (const [label, release, message] of [
    ['an absent draft', null, /does not exist/],
    ['a published release', githubDraft({ isDraft: false }), /must remain a draft/],
    ['release tag drift', githubDraft({ tagName: 'cem-ml-v3.4.4' }), /release tag drift/],
    ['release title drift', githubDraft({ name: 'Unexpected title' }), /release title drift/],
    ['release prerelease drift', githubDraft({ isPrerelease: true }), /release prerelease drift/],
]) {
    test(`native-host release preflight rejects ${label} without mutation`, () => {
        const github = createFakeGithubReleaseClient(release);
        assert.throws(() => preflightNativeHostRelease(nativePreflightArguments({ github })), message);
        assert.deepEqual(github.viewRequests, [releaseTag]);
        assert.deepEqual(github.createRequests, []);
        assert.equal(github.publishCalls, 0);
    });
}

test('native-host release preflight rejects deployment identity drift before reading GitHub', () => {
    const github = createFakeGithubReleaseClient(githubDraft());
    const deployment = nativeDeployment('native-linux-amd64');
    deployment.rustTarget = 'aarch64-unknown-linux-gnu';
    assert.throws(
        () => preflightNativeHostRelease(nativePreflightArguments({ deployment, github })),
        /deployment Rust target drift/,
    );
    assert.deepEqual(github.viewRequests, []);
    assert.deepEqual(github.createRequests, []);
});

test('CEM-ML workflow owns its tag family and generic publishing excludes it', () => {
    const workflow = readFileSync(resolve(workspaceRoot, '.github/workflows/cem-ml-release.yml'), 'utf8');
    const generic = readFileSync(resolve(workspaceRoot, '.github/workflows/publish.yml'), 'utf8');

    assert.match(workflow, /tags:\s*\n\s+- 'cem-ml-v\*'/);
    assert.match(workflow, /release_tag:\s*\n\s+description:/);
    assert.match(workflow, /group: cem-ml-release-/);
    assert.match(workflow, /name: cem-ml-release/);
    assert.match(workflow, /contents: write/);
    assert.match(workflow, /cem_ml:release:create-draft/);
    assert.match(workflow, /npm-producers:/);
    assert.match(workflow, /linux-producer:/);
    assert.match(workflow, /upload-ci-units:/);
    assert.ok(workflow.match(/uses: actions\/attest@v4/g)?.length >= 4);
    assert.match(workflow, /artifact-metadata: write/);
    assert.match(workflow, /CEM_ML_RELEASE_GPG_PRIVATE_KEY_BASE64: \$\{\{ secrets\./);
    assert.match(workflow, /CEM_ML_RELEASE_GPG_PASSPHRASE: \$\{\{ secrets\./);
    assert.match(workflow, /uses: actions\/upload-artifact@v4/);
    assert.match(workflow, /uses: actions\/download-artifact@v5/);
    assert.match(workflow, /CEM_ML_ARTIFACT_ATTESTATION_ID: \$\{\{ steps\.attestation\.outputs\.attestation-id \}\}/);
    assert.match(workflow, /CEM_ML_ARTIFACT_ATTESTATION_URL: \$\{\{ steps\.attestation\.outputs\.attestation-url \}\}/);
    assert.match(workflow, /producer-evidence\.json/);
    assert.match(workflow, /record:producer-evidence/);
    assert.match(workflow, /verify:producer-evidence/);
    const producerJobs = workflow.slice(workflow.indexOf('  npm-producers:'), workflow.indexOf('  upload-ci-units:'));
    assert.doesNotMatch(producerJobs, /contents: write/);
    assert.match(workflow, /CEM_ML_PLATFORM_DRAFT: \$\{\{ vars\.CEM_ML_PLATFORM_DRAFT \}\}/);
    assert.doesNotMatch(workflow, /CEM_ML_PLATFORM_DRAFT: ['"]?1/);
    assert.doesNotMatch(workflow, /release edit|draft=false|nx release publish/);
    assert.match(generic, /- '!cem-ml-v\*'/);
});

test('CI producer Nx targets are explicit and package evidence is keyed by the source commit', () => {
    const npmRuntime = readProject('packages/cem-ml-npm/project.json');
    const npmCli = readProject('packages/cem-ml-cli-npm/project.json');
    const linux = readProject('packages/cem-ml-cli-native-linux-amd64/project.json');
    const coordinator = readProject('packages/cem_ml/project.json');

    for (const project of [npmRuntime, npmCli]) {
        assert.deepEqual(project.targets.package.inputs.at(-2), { runtime: 'git rev-parse HEAD' });
        assert.ok(project.targets['verify:release']);
        assert.ok(project.targets['record:producer-evidence']);
        assert.ok(project.targets['verify:producer-evidence']);
    }
    assert.ok(linux.targets['verify:release']);
    assert.ok(linux.targets['smoke:release']);
    assert.deepEqual(linux.targets['smoke:release'].dependsOn, ['verify', 'verify:release']);
    assert.ok(linux.targets['record:producer-evidence']);
    assert.ok(linux.targets['verify:producer-evidence']);
    assert.ok(coordinator.targets['release:upload-ci-units']);
    assert.deepEqual(
        coordinator.targets['release:stage'].inputs.filter(
            (input) =>
                typeof input === 'string' && input.includes('/dist/packages/') && input.endsWith('/artifacts/**/*'),
        ),
        [
            '{workspaceRoot}/dist/packages/cem-ml-npm/artifacts/**/*',
            '{workspaceRoot}/dist/packages/cem-ml-cli-npm/artifacts/**/*',
            '{workspaceRoot}/dist/packages/cem-ml-cli-native-linux-amd64/artifacts/**/*',
        ],
    );
});

test('all native publishers require the shared uncached exact-tag preflight', () => {
    for (const [identity, projectPath] of nativeDeploymentPaths) {
        const project = readProject(projectPath.replace('/deployment.json', '/project.json'));
        const preflight = project.targets['preflight:release'];
        assert.ok(preflight, `${identity} is missing preflight:release`);
        assert.equal(preflight.cache, false);
        assert.equal(preflight.options.cwd, '{workspaceRoot}');
        assert.equal(
            preflight.options.command,
            `node tools/scripts/cem-ml-platform-release.mjs preflight-native-host ${identity}`,
        );
        assert.ok(preflight.inputs.includes('{workspaceRoot}/tools/scripts/cem-ml-platform-release.mjs'));
        assert.ok(preflight.inputs.includes(`{workspaceRoot}/${projectPath}`));
        assert.ok(preflight.inputs.some((input) => input.runtime === 'git rev-parse HEAD'));
        assert.ok(preflight.inputs.some((input) => input.env === 'CEM_ML_RELEASE_TAG'));
        assert.deepEqual(project.targets.publish.dependsOn, ['preflight:release']);
    }
});

test('macOS and Windows retain one deferred signed lifecycle recipe', () => {
    for (const [identity, projectPath] of [...nativeDeploymentPaths].slice(1)) {
        const project = readProject(projectPath.replace('/deployment.json', '/project.json'));
        const smoke = project.targets['smoke:release'];
        assert.ok(smoke, `${identity} is missing smoke:release`);
        assert.deepEqual(smoke.dependsOn, ['verify']);
        assert.equal(smoke.options.parallel, false);
        assert.deepEqual(smoke.options.commands, [
            'node scripts/smoke.mjs install',
            'node scripts/smoke.mjs upgrade',
            'node scripts/smoke.mjs uninstall',
        ]);
        const finalize = project.targets['finalize:release'];
        assert.ok(finalize, `${identity} is missing finalize:release`);
        assert.equal(finalize.cache, false);
        assert.deepEqual(finalize.dependsOn, undefined, `${identity} finalization must not rebuild signed bytes`);
        assert.ok(finalize.inputs.some((input) => input.env === 'CEM_ML_GITHUB_ATTESTATION_BUNDLE'));
    }
});

test('deferred native workflow keeps its recipes but disables every self-hosted job', () => {
    const workflow = readFileSync(resolve(workspaceRoot, '.github/workflows/cem-ml-native-release.yml'), 'utf8');

    assert.match(workflow, /name: CEM-ML Native Release \(deferred\)/);
    assert.match(workflow, /GitHub Releases contain only the WASM\/npm and Linux AMD64 units/);
    assert.match(workflow, /workflow_dispatch:/);
    assert.match(workflow, /type: choice/);
    assert.equal(workflow.match(/if: \$\{\{ false \}\}/g)?.length, 4);
    assert.doesNotMatch(workflow, /if: inputs\.native_host/);
    assert.match(workflow, /runs-on: \[self-hosted, macOS, ARM64, local-macos-arm64\]/);
    assert.match(workflow, /runs-on: \[self-hosted, Windows, X64, local-windows-amd64\]/);
    assert.ok(workflow.match(/environment:\s*\n\s+name: cem-ml-release/g)?.length === 4);
    assert.ok(workflow.match(/id-token: write/g)?.length === 2);
    assert.ok(workflow.match(/artifact-metadata: write/g)?.length === 2);
    assert.ok(workflow.match(/uses: actions\/attest@v4/g)?.length === 2);
    assert.match(workflow, /cem_ml_cli_native_brew_arm64:preflight:release/);
    assert.match(workflow, /cem_ml_cli_native_brew_arm64:smoke:release/);
    assert.match(workflow, /cem_ml_cli_native_brew_arm64:finalize:release/);
    assert.match(workflow, /cem_ml_cli_native_brew_arm64:publish/);
    assert.match(workflow, /cem_ml_cli_native_windows_amd64:preflight:release/);
    assert.match(workflow, /cem_ml_cli_native_windows_amd64:smoke:release/);
    assert.match(workflow, /cem_ml_cli_native_windows_amd64:finalize:release/);
    assert.match(workflow, /cem_ml_cli_native_windows_amd64:publish/);
    assert.ok(
        workflow.match(/CEM_ML_GITHUB_ATTESTATION_BUNDLE: \$\{\{ steps\.attestation\.outputs\.bundle-path \}\}/g)
            ?.length === 2,
    );
    assert.ok(workflow.match(/uses: actions\/upload-artifact@v4/g)?.length >= 2);
    assert.ok(workflow.match(/uses: actions\/download-artifact@v5/g)?.length === 2);
    const macosProducer = workflow.slice(
        workflow.indexOf('  macos-arm64-producer:'),
        workflow.indexOf('  macos-arm64-publisher:'),
    );
    const macosPublisher = workflow.slice(
        workflow.indexOf('  macos-arm64-publisher:'),
        workflow.indexOf('  windows-amd64-producer:'),
    );
    const windowsProducer = workflow.slice(
        workflow.indexOf('  windows-amd64-producer:'),
        workflow.indexOf('  windows-amd64-publisher:'),
    );
    const windowsPublisher = workflow.slice(workflow.indexOf('  windows-amd64-publisher:'));
    for (const producer of [macosProducer, windowsProducer]) {
        assert.match(producer, /contents: read/);
        assert.doesNotMatch(producer, /contents: write/);
        assert.match(producer, /id-token: write/);
    }
    for (const publisher of [macosPublisher, windowsPublisher]) {
        assert.match(publisher, /contents: write/);
        assert.doesNotMatch(publisher, /id-token: write/);
        assert.doesNotMatch(publisher, /actions\/attest/);
    }
    assert.doesNotMatch(workflow, /push:\s*\n\s+tags:/);
    assert.doesNotMatch(workflow, /--clobber/);
});

test('native publishers delegate to the shared immutable uploader and never request clobbering', () => {
    for (const projectPath of nativeDeploymentPaths.values()) {
        const publisherPath = projectPath.replace('/deployment.json', '/scripts/publish.mjs');
        const publisher = readFileSync(resolve(workspaceRoot, publisherPath), 'utf8');
        assert.match(publisher, /uploadImmutableDraftAssetSet/);
        assert.match(publisher, /CEM_ML_RELEASE_VERIFY/);
        assert.doesNotMatch(publisher, /--clobber/);
    }
});

test('native finalizers verify the GitHub attestation for every checksum-listed subject', () => {
    for (const projectPath of [...nativeDeploymentPaths.values()].slice(1)) {
        const finalizerPath = projectPath.replace('/deployment.json', '/scripts/finalize.mjs');
        const finalizer = readFileSync(resolve(workspaceRoot, finalizerPath), 'utf8');
        assert.match(finalizer, /readFileSync\(checksum, 'utf8'\)\.trim\(\)\.split/);
        assert.match(finalizer, /'attestation',\s*\n\s*'verify'/);
        assert.match(finalizer, /artifactPath\(match\[1\]\)/);
        assert.match(finalizer, /CEM_ML_RELEASE_VERIFY/);
    }
});

test('npm release attestations cover every checksum-listed subject', () => {
    const fixture = createFixture();
    try {
        const entry = readJson(fixture.npmRuntime.entry);
        const subjects = releaseAttestationSubjects({ artifactRoot: dirname(fixture.npmRuntime.entry), entry });
        const expected = readFileSync(fixture.npmRuntime.checksum, 'utf8')
            .trim()
            .split('\n')
            .map((line) => resolve(dirname(fixture.npmRuntime.entry), line.slice(66)));
        assert.deepEqual(subjects, expected);
    } finally {
        fixture.dispose();
    }
});

test('Linux release GPG signing supplies a protected passphrase only through standard input', () => {
    const passphrase = 'never-in-arguments';
    const invocation = releaseGpgSigningInvocation({
        releaseKey: 'a'.repeat(40),
        passphrase,
        signature: '/tmp/checksum.asc',
        checksum: '/tmp/checksum',
    });

    assert.ok(invocation.args.includes('--pinentry-mode'));
    assert.ok(invocation.args.includes('--passphrase-fd'));
    assert.equal(invocation.args.includes(passphrase), false);
    assert.equal(invocation.input, passphrase);
    assert.deepEqual(invocation.stdio, ['pipe', 'inherit', 'inherit']);
    assert.equal(releasePublicationReady({ gpgStatus: 'signed', attestationStatus: 'verified' }), true);
    assert.equal(releasePublicationReady({ gpgStatus: 'signed', attestationStatus: 'supplied' }), false);
});

test('CI release-unit verification accepts each publication-ready CI-owned unit independently', () => {
    const fixture = createFixture();
    try {
        makePublicationReady(fixture.root, ciReleaseUnits);
        for (const unit of ciReleaseUnits) {
            const verified = verifyPlatformReleaseUnit({
                workspaceRoot: fixture.root,
                identity: unit.identity,
                version,
                sourceCommit,
                releaseTag,
                taggedCommit: sourceCommit,
            });
            assert.equal(verified.identity, unit.identity);
        }
    } finally {
        fixture.dispose();
    }
});

test('CI producer evidence records the exact run, completed gates, toolchain, target, and artifact attestation', () => {
    const fixture = createFixture();
    try {
        makePublicationReady(fixture.root, ciReleaseUnits);
        const unit = ciReleaseUnits[1];
        const root = resolve(fixture.root, unit.root);
        const entryBefore = readFileSync(findReleaseEntry(root));
        const checksumBefore = readFileSync(findChecksum(root));
        const result = recordCiProducerEvidence({
            workspaceRoot: fixture.root,
            identity: unit.identity,
            version,
            sourceCommit,
            releaseTag,
            taggedCommit: sourceCommit,
            workflow: producerWorkflow(),
            runner: producerRunner(),
            toolchain: producerToolchain(),
            artifactAttestation: producerArtifactAttestation(),
        });

        assert.match(result.evidenceFilename, /^cem-ml-3\.4\.5-.+\.producer-evidence\.json$/);
        assert.equal(result.evidence.unitIdentity, unit.identity);
        assert.equal(result.evidence.workflow.runId, '123456789');
        assert.equal(result.evidence.workflow.runAttempt, 2);
        assert.equal(result.evidence.workflow.url, 'https://github.com/EPA-WG/cem/actions/runs/123456789/attempts/2');
        assert.deepEqual(result.evidence.targetIdentities, [...unit.targets]);
        assert.deepEqual(result.evidence.toolchain, producerToolchain());
        assert.ok(result.evidence.nx.gates.every(({ status }) => status === 'passed'));
        assert.ok(result.evidence.nx.gates.some(({ name }) => name === 'platform-parity'));
        assert.equal(result.evidence.artifactAttestation.id, producerArtifactAttestation().id);
        assert.equal(result.evidence.artifactAttestation.url, producerArtifactAttestation().url);
        assert.equal(result.evidence.artifactAttestation.status, 'verified');
        assert.match(result.evidence.artifactAttestation.bundle, /\.attestation\.jsonl$/);
        assert.match(result.evidence.artifactAttestation.sha256, /^[a-f0-9]{64}$/);
        assert.equal(readFileSync(findReleaseEntry(root)).equals(entryBefore), true);
        assert.equal(readFileSync(findChecksum(root)).equals(checksumBefore), true);
    } finally {
        fixture.dispose();
    }
});

test('detached producer-evidence attestation is verified before becoming a release asset', () => {
    const fixture = createFixture();
    const suppliedBundle = resolve(fixture.root, 'supplied-producer-evidence-attestation.json');
    try {
        makePublicationReady(fixture.root, ciReleaseUnits);
        const unit = ciReleaseUnits[0];
        const recorded = recordProducerEvidenceFixture(fixture.root, unit);
        writeFileSync(suppliedBundle, '{"verificationMaterial":{}}\n');
        const calls = [];
        const finalized = finalizeCiProducerEvidence({
            workspaceRoot: fixture.root,
            identity: unit.identity,
            version,
            sourceCommit,
            releaseTag,
            taggedCommit: sourceCommit,
            suppliedAttestation: suppliedBundle,
            attestationVerifier: ({ subject, bundle }) => calls.push({ subject, bundle }),
        });

        assert.deepEqual(calls, [{ subject: recorded.evidencePath, bundle: finalized.bundlePath }]);
        assert.equal(finalized.evidence.evidenceAttestation.status, 'required-detached');
        assert.equal(statSync(finalized.bundlePath).isFile(), true);
        assert.equal(
            validateCiProducerEvidence({
                root: resolve(fixture.root, unit.root),
                unit,
                version,
                sourceCommit,
                releaseTag,
                attestationVerifier: ({ subject, bundle }) => calls.push({ subject, bundle }),
            }).evidenceFilename,
            recorded.evidenceFilename,
        );
        assert.equal(calls.length, 2);
    } finally {
        fixture.dispose();
    }
});

for (const [label, mutate, message] of [
    ['source commit', (evidence) => (evidence.sourceCommit = 'f'.repeat(40)), /source-commit drift/],
    ['workflow URL', (evidence) => (evidence.workflow.url = 'https://example.com/run'), /workflow run URL drift/],
    ['target identity', (evidence) => (evidence.targetIdentities = ['wrong-target']), /target drift/],
    ['gate result', (evidence) => (evidence.nx.gates[0].status = 'failed'), /gate did not pass/],
    [
        'attestation URL',
        (evidence) => (evidence.artifactAttestation.url = 'https://example.com'),
        /attestation URL drift/,
    ],
]) {
    test(`CI producer evidence rejects ${label} drift`, () => {
        const fixture = createFixture();
        try {
            makePublicationReady(fixture.root, ciReleaseUnits);
            const unit = ciReleaseUnits[0];
            const recorded = recordProducerEvidenceFixture(fixture.root, unit);
            writeFileSync(resolve(fixture.root, unit.root, recorded.bundleFilename), '{"bundle":true}\n');
            updateJson(recorded.evidencePath, mutate);
            assert.throws(
                () =>
                    validateCiProducerEvidence({
                        root: resolve(fixture.root, unit.root),
                        unit,
                        version,
                        sourceCommit,
                        releaseTag,
                        attestationVerifier: () => true,
                    }),
                message,
            );
        } finally {
            fixture.dispose();
        }
    });
}

test('CI-owned unit upload is idempotent, byte-verifies existing assets, and preserves foreign units', () => {
    const fixture = createFixture();
    try {
        makePublicationReady(fixture.root, ciReleaseUnits);
        makeProducerEvidenceReady(fixture.root, ciReleaseUnits);
        const github = createFakeGithubAssetClient({
            'cem-ml-3.4.5-native-macos-arm64.manual': 'manual host bytes',
        });
        const first = uploadPlatformReleaseUnits({
            workspaceRoot: fixture.root,
            units: ciReleaseUnits,
            authorized: true,
            version,
            sourceCommit,
            releaseTag,
            taggedCommit: sourceCommit,
            github,
            attestationVerifier: () => true,
        });
        assert.equal(first.uploaded.length > 0, true);
        assert.equal(github.assets.has('cem-ml-3.4.5-native-macos-arm64.manual'), true);

        const second = uploadPlatformReleaseUnits({
            workspaceRoot: fixture.root,
            units: ciReleaseUnits,
            authorized: true,
            version,
            sourceCommit,
            releaseTag,
            taggedCommit: sourceCommit,
            github,
            attestationVerifier: () => true,
        });
        assert.deepEqual(second.uploaded, []);
        assert.equal(github.uploadCalls, 1);
    } finally {
        fixture.dispose();
    }
});

test('CI-owned unit upload rejects remote byte drift without clobbering', () => {
    const fixture = createFixture();
    try {
        makePublicationReady(fixture.root, ciReleaseUnits);
        makeProducerEvidenceReady(fixture.root, ciReleaseUnits);
        const ownedRoot = resolve(fixture.root, ciReleaseUnits[0].root);
        const ownedFilename = readdirSync(ownedRoot).find((filename) => filename.endsWith('.tgz'));
        assert.ok(ownedFilename);
        const github = createFakeGithubAssetClient({ [ownedFilename]: 'drifted remote bytes' });
        assert.throws(
            () =>
                uploadPlatformReleaseUnits({
                    workspaceRoot: fixture.root,
                    units: ciReleaseUnits,
                    authorized: true,
                    version,
                    sourceCommit,
                    releaseTag,
                    taggedCommit: sourceCommit,
                    github,
                    attestationVerifier: () => true,
                }),
            /immutable|drift/,
        );
        assert.equal(github.uploadCalls, 0);
    } finally {
        fixture.dispose();
    }
});

test('native draft upload preserves foreign units, uploads only missing assets, and is idempotent', () => {
    const root = mkdtempSync(resolve(tmpdir(), 'cem-ml-native-upload-'));
    const base = `cem-ml-${version}-linux-amd64-gnu`;
    try {
        writeFileSync(resolve(root, `${base}.tar.gz`), 'native archive bytes');
        writeFileSync(resolve(root, `${base}.sha256`), 'native checksum bytes');
        const github = createFakeGithubAssetClient({
            [`${base}.tar.gz`]: 'native archive bytes',
            [`cem-ml-${version}-windows-amd64-msvc.zip`]: 'foreign unit bytes',
        });
        const verified = [];

        const first = uploadImmutableDraftAssetSet({
            identity: 'native-linux-amd64',
            version,
            releaseTag,
            assetRoot: root,
            ownedBase: base,
            github,
            verifyDownloaded: ({ paths }) => verified.push([...paths.keys()].sort()),
        });
        assert.deepEqual(first.uploaded, [`${base}.sha256`]);
        assert.equal(github.assets.has(`cem-ml-${version}-windows-amd64-msvc.zip`), true);
        assert.deepEqual(verified, [[`${base}.sha256`, `${base}.tar.gz`]]);

        const second = uploadImmutableDraftAssetSet({
            identity: 'native-linux-amd64',
            version,
            releaseTag,
            assetRoot: root,
            ownedBase: base,
            github,
        });
        assert.deepEqual(second.uploaded, []);
        assert.equal(github.uploadCalls, 1);
    } finally {
        rmSync(root, { recursive: true, force: true });
    }
});

test('native draft upload rejects an unexpected owned remote asset without mutation', () => {
    const root = mkdtempSync(resolve(tmpdir(), 'cem-ml-native-upload-'));
    const base = `cem-ml-${version}-linux-amd64-gnu`;
    try {
        writeFileSync(resolve(root, `${base}.zip`), 'native archive bytes');
        const github = createFakeGithubAssetClient({ [`${base}.unexpected`]: 'unexpected bytes' });
        assert.throws(
            () =>
                uploadImmutableDraftAssetSet({
                    identity: 'native-linux-amd64',
                    version,
                    releaseTag,
                    assetRoot: root,
                    ownedBase: base,
                    github,
                }),
            /unexpected assets owned by native-linux-amd64/,
        );
        assert.equal(github.uploadCalls, 0);
    } finally {
        rmSync(root, { recursive: true, force: true });
    }
});

test('native draft upload rejects macOS and Windows units outside the GitHub Release contract', () => {
    for (const identity of ['native-macos-arm64', 'native-windows-amd64']) {
        assert.throws(
            () =>
                uploadImmutableDraftAssetSet({
                    identity,
                    version,
                    releaseTag,
                    assetRoot: '/unused',
                    ownedBase: `cem-ml-${version}-unused`,
                }),
            /is not in the CEM-ML GitHub Release contract/,
        );
    }
});

test('native draft upload rejects existing byte drift without clobbering', () => {
    const root = mkdtempSync(resolve(tmpdir(), 'cem-ml-native-upload-'));
    const base = `cem-ml-${version}-linux-amd64-gnu`;
    try {
        writeFileSync(resolve(root, `${base}.tar.gz`), 'local native bytes');
        const github = createFakeGithubAssetClient({ [`${base}.tar.gz`]: 'drifted remote bytes' });
        assert.throws(
            () =>
                uploadImmutableDraftAssetSet({
                    identity: 'native-linux-amd64',
                    version,
                    releaseTag,
                    assetRoot: root,
                    ownedBase: base,
                    github,
                }),
            /existing native-linux-amd64 draft asset is not immutable/,
        );
        assert.equal(github.uploadCalls, 0);
    } finally {
        rmSync(root, { recursive: true, force: true });
    }
});

test('three GitHub Release fixtures stage one complete immutable release index', () => {
    const fixture = createFixture();
    try {
        const staged = stagePlatformRelease({
            workspaceRoot: fixture.root,
            version,
            sourceCommit,
            units: expectedReleaseUnits,
        });
        assert.equal(staged.index.units.length, 3);
        assert.equal(staged.index.releaseTag, `cem-ml-v${version}`);
        assert.ok(staged.index.assets.length > 15);
        assert.equal(
            verifyPlatformRelease({
                workspaceRoot: fixture.root,
                outputRoot: staged.outputRoot,
                version,
                sourceCommit,
                units: expectedReleaseUnits,
            }).sourceCommit,
            sourceCommit,
        );
    } finally {
        fixture.dispose();
    }
});

test('aggregate release evidence indexes each authoritative CI producer sidecar and detached attestation', () => {
    const fixture = createFixture();
    try {
        makePublicationReady(fixture.root, ciReleaseUnits);
        makeProducerEvidenceReady(fixture.root, ciReleaseUnits);
        const staged = stagePlatformRelease({
            workspaceRoot: fixture.root,
            version,
            sourceCommit,
            units: expectedReleaseUnits,
            attestationVerifier: () => true,
        });
        for (const unit of ciReleaseUnits) {
            const summary = staged.index.units.find(({ identity }) => identity === unit.identity);
            assert.match(summary.producerEvidence, /\.producer-evidence\.json$/);
            assert.match(summary.producerEvidenceAttestation, /\.producer-evidence\.attestation\.jsonl$/);
            assert.ok(staged.index.assets.some(({ filename }) => filename === summary.producerEvidence));
            assert.ok(staged.index.assets.some(({ filename }) => filename === summary.producerEvidenceAttestation));
        }
    } finally {
        fixture.dispose();
    }
});

test('GitHub draft upload remains inert without the protected release opt-in', () => {
    assert.throws(() => uploadPlatformReleaseDraft(), /draft upload is disabled/);
});

test('required npm signing rejects an absent GitHub attestation bundle', () => {
    const fixture = createFixture();
    const previousSigning = process.env.CEM_ML_RELEASE_SIGNING;
    const previousBundle = process.env.CEM_ML_GITHUB_ATTESTATION_BUNDLE;
    try {
        process.env.CEM_ML_RELEASE_SIGNING = 'required';
        delete process.env.CEM_ML_GITHUB_ATTESTATION_BUNDLE;
        assert.throws(
            () => attestNpmReleaseEvidence({ workspaceRoot: fixture.root, packageName: '@epa-wg/cem-ml' }),
            /requires CEM_ML_GITHUB_ATTESTATION_BUNDLE/,
        );
    } finally {
        restoreEnvironment('CEM_ML_RELEASE_SIGNING', previousSigning);
        restoreEnvironment('CEM_ML_GITHUB_ATTESTATION_BUNDLE', previousBundle);
        fixture.dispose();
    }
});

const driftCases = [
    ['version', ({ entry }) => updateJson(entry, (value) => (value.commonVersion = '9.9.9'))],
    ['source commit', ({ entry }) => updateJson(entry, (value) => (value.sourceCommit = 'f'.repeat(40)))],
    ['target', ({ entry }) => updateJson(entry, (value) => (value.targetIdentities = ['wrong-target']))],
    ['checksum', ({ primary }) => writeFileSync(primary, 'changed bytes')],
    ['signature', ({ signing }) => updateJson(signing, (value) => (value.checksumManifest.sha256 = '0'.repeat(64)))],
    ['SBOM', ({ sbom }) => updateJson(sbom, (value) => (value.spdxVersion = 'SPDX-2.2'))],
    [
        'provenance',
        ({ provenance }) =>
            updateJson(
                provenance,
                (value) => (value.buildDefinition.resolvedDependencies[0].digest.gitCommit = 'a'.repeat(40)),
            ),
    ],
    ['capability', ({ capability }) => updateJson(capability, (value) => (value.commonVersion = '0.0.0'))],
    ['release index', ({ entry }) => updateJson(entry, (value) => (value.capabilityManifestDigest = '1'.repeat(64)))],
];

for (const [label, mutate] of driftCases) {
    test(`platform staging rejects independent ${label} drift`, () => {
        const fixture = createFixture();
        try {
            mutate(fixture.npmRuntime);
            assert.throws(
                () =>
                    stagePlatformRelease({
                        workspaceRoot: fixture.root,
                        version,
                        sourceCommit,
                        units: expectedReleaseUnits,
                    }),
                /drift|expected|missing|version|target|source|checksum|SBOM|provenance|capability/i,
            );
        } finally {
            fixture.dispose();
        }
    });
}

function githubDraft(overrides = {}) {
    return {
        tagName: releaseTag,
        name: releaseTitle,
        isDraft: true,
        isPrerelease: false,
        assets: [],
        ...overrides,
    };
}

function createFakeGithubReleaseClient(initialRelease = null) {
    let release = initialRelease;
    const client = {
        createRequests: [],
        publishCalls: 0,
        viewRequests: [],
        view(tag) {
            client.viewRequests.push(tag);
            return release;
        },
        create(request) {
            client.createRequests.push(request);
            release = githubDraft({ isPrerelease: request.prerelease });
        },
    };
    return client;
}

function nativeDeployment(identity) {
    const path = nativeDeploymentPaths.get(identity);
    assert.ok(path, `missing native deployment fixture for ${identity}`);
    return { ...readProject(path), commonVersion: version };
}

function nativePreflightArguments(overrides = {}) {
    return {
        identity: 'native-linux-amd64',
        version,
        sourceCommit,
        releaseTag,
        taggedCommit: sourceCommit,
        sourceTreeStatus: '',
        platform: 'linux',
        architecture: 'x64',
        deployment: nativeDeployment('native-linux-amd64'),
        github: createFakeGithubReleaseClient(githubDraft()),
        ...overrides,
    };
}

function createFakeGithubAssetClient(initialAssets = {}) {
    const assets = new Map(Object.entries(initialAssets).map(([name, content]) => [name, Buffer.from(content)]));
    return {
        assets,
        uploadCalls: 0,
        view(tag) {
            return {
                tagName: tag,
                isDraft: true,
                assets: [...assets.keys()].sort().map((name) => ({ name })),
            };
        },
        download(_tag, filename, destinationRoot) {
            const content = assets.get(filename);
            if (!content) throw new Error(`missing fake remote asset ${filename}`);
            writeFileSync(resolve(destinationRoot, filename), content);
        },
        upload(_tag, paths) {
            this.uploadCalls += 1;
            for (const path of paths) {
                const filename = basename(path);
                if (assets.has(filename)) throw new Error(`fake upload would clobber ${filename}`);
                assets.set(filename, readFileSync(path));
            }
        },
    };
}

function createFixture() {
    const root = mkdtempSync(resolve(tmpdir(), 'cem-ml-platform-release-'));
    let npmRuntime;
    for (const unit of expectedReleaseUnits) {
        const created = writeUnit(root, unit);
        if (unit.identity === '@epa-wg/cem-ml') npmRuntime = created;
    }
    return { root, npmRuntime, dispose: () => rmSync(root, { recursive: true, force: true }) };
}

function makePublicationReady(workspaceRoot, units) {
    for (const unit of units) {
        const root = resolve(workspaceRoot, unit.root);
        const entryFilename = readdirSync(root).find((filename) => filename.endsWith('.release-index-entry.json'));
        assert.ok(entryFilename);
        const entry = JSON.parse(readFileSync(resolve(root, entryFilename), 'utf8'));
        const signingPath = resolve(root, entry.signingRecord);
        const attestationFilename = entry.signingRecord.replace('.signing.json', '.attestation.jsonl');
        const attestationPath = resolve(root, attestationFilename);
        writeFileSync(attestationPath, `${JSON.stringify({ unit: unit.identity })}\n`);
        updateJson(signingPath, (signing) => {
            signing.githubArtifactAttestation = {
                status: 'verified',
                bundle: attestationFilename,
                sha256: sha256File(attestationPath),
            };
            signing.publicationReady = true;
        });
    }
}

function producerWorkflow(unit) {
    return {
        repository: 'EPA-WG/cem',
        workflowRef: 'EPA-WG/cem/.github/workflows/cem-ml-release.yml@refs/tags/cem-ml-v3.4.5',
        workflowSha: sourceCommit,
        runId: '123456789',
        runAttempt: 2,
        job: unit?.identity === 'native-linux-amd64' ? 'linux-producer' : 'npm-producers',
        actor: 'release-operator',
        triggeringActor: 'release-operator',
        url: 'https://github.com/EPA-WG/cem/actions/runs/123456789/attempts/2',
    };
}

function producerRunner() {
    return {
        name: 'GitHub Actions 7',
        os: 'Linux',
        architecture: 'X64',
        environment: 'github-hosted',
        image: 'ubuntu24',
        imageVersion: '20260810.1',
    };
}

function producerToolchain(unit) {
    const toolchain = {
        node: 'v24.6.0',
        yarn: '4.9.2',
        rustc: 'rustc 1.89.0 (fixture)',
        cargo: 'cargo 1.89.0 (fixture)',
        githubCli: 'gh version 2.76.2 (fixture)',
    };
    if (unit?.identity === 'native-linux-amd64') toolchain.gpg = 'gpg (GnuPG) 2.4.7';
    else toolchain.wasmBindgen = 'wasm-bindgen 0.2.122';
    return toolchain;
}

function producerArtifactAttestation() {
    return {
        id: '987654321',
        url: 'https://github.com/EPA-WG/cem/attestations/987654321',
    };
}

function recordProducerEvidenceFixture(workspaceRoot, unit) {
    return recordCiProducerEvidence({
        workspaceRoot,
        identity: unit.identity,
        version,
        sourceCommit,
        releaseTag,
        taggedCommit: sourceCommit,
        workflow: producerWorkflow(unit),
        runner: producerRunner(),
        toolchain: producerToolchain(unit),
        artifactAttestation: producerArtifactAttestation(),
    });
}

function makeProducerEvidenceReady(workspaceRoot, units) {
    for (const unit of units) {
        const recorded = recordProducerEvidenceFixture(workspaceRoot, unit);
        writeFileSync(resolve(workspaceRoot, unit.root, recorded.bundleFilename), '{"bundle":true}\n');
    }
}

function findReleaseEntry(root) {
    const filename = readdirSync(root).find((candidate) => candidate.endsWith('.release-index-entry.json'));
    assert.ok(filename);
    return resolve(root, filename);
}

function findChecksum(root) {
    const filename = readdirSync(root).find((candidate) => candidate.endsWith('.sha256'));
    assert.ok(filename);
    return resolve(root, filename);
}

function writeUnit(workspaceRoot, unit) {
    const root = resolve(workspaceRoot, unit.root);
    mkdirSync(root, { recursive: true });
    const coordinate = unit.identity.replace('@epa-wg/', '').replaceAll(/[^a-z0-9]+/g, '-');
    const base = `cem-ml-${version}-${coordinate}`;
    const primaryName = `${base}.${unit.identity.startsWith('native-') ? 'zip' : 'tgz'}`;
    const capabilityName = `${base}.capabilities.json`;
    const integrityName = `${base}.integrity.json`;
    const sbomName = `${base}.spdx.json`;
    const provenanceName = `${base}.provenance.json`;
    const channelName = unit.channel ? `${base}.${unit.channel}.json` : null;
    const entryName = `${base}.release-index-entry.json`;
    const checksumName = `${base}.sha256`;
    const signingName = `${base}.signing.json`;
    const primary = resolve(root, primaryName);
    const capability = resolve(root, capabilityName);
    const integrity = resolve(root, integrityName);
    const sbom = resolve(root, sbomName);
    const provenance = resolve(root, provenanceName);
    const entry = resolve(root, entryName);
    const signing = resolve(root, signingName);
    writeFileSync(primary, `${unit.identity} package bytes`);
    writeJson(capability, {
        commonVersion: version,
        runtime: unit.identity,
        targetIdentity: unit.target ?? unit.targets[0],
    });
    if (unit.identity.startsWith('@')) writeJson(integrity, { commonVersion: version, algorithm: 'sha256', files: [] });
    writeJson(sbom, {
        spdxVersion: 'SPDX-2.3',
        dataLicense: 'CC0-1.0',
        name: `${unit.identity}-${version}`,
        packages: [{ name: unit.identity, versionInfo: version }],
    });
    const provenanceSubjects = [primaryName, capabilityName, sbomName];
    if (unit.identity.startsWith('@')) provenanceSubjects.push(integrityName);
    writeJson(provenance, {
        predicateType: 'https://slsa.dev/provenance/v1',
        buildDefinition: {
            resolvedDependencies: [
                { uri: 'git+https://github.com/EPA-WG/cem.git', digest: { gitCommit: sourceCommit } },
            ],
        },
        subject: provenanceSubjects.map((filename) => artifactRecord(root, filename)),
    });

    const artifactNames = [primaryName, capabilityName, sbomName, provenanceName];
    if (unit.identity.startsWith('@')) artifactNames.splice(2, 0, integrityName);
    if (channelName) {
        writeJson(resolve(root, channelName), {
            channel: unit.channel,
            version,
            immutableSource: {
                releaseTag: `cem-ml-v${version}`,
                filename: primaryName,
                url: `https://github.com/EPA-WG/cem/releases/download/cem-ml-v${version}/${primaryName}`,
                sha256: sha256File(primary),
            },
        });
        artifactNames.push(channelName);
    }
    const releaseEntry = {
        schemaVersion: 1,
        product: 'cem-ml',
        commonVersion: version,
        sourceCommit,
        releaseTag: `cem-ml-v${version}`,
        capabilityManifestDigest: sha256File(capability),
        artifacts: artifactNames.map((filename) => artifactRecord(root, filename)),
        checksumManifest: checksumName,
        signingRecord: signingName,
        publicationState: 'staged-local',
    };
    if (unit.identity.startsWith('@')) {
        releaseEntry.npmIdentity = unit.identity;
        releaseEntry.runtimeIdentities = ['wasm-browser-worker', 'wasm-node'];
        releaseEntry.targetIdentities = [...unit.targets];
        releaseEntry.abiIdentities = ['fixture-abi'];
        releaseEntry.integrityManifestDigest = sha256File(integrity);
    } else {
        releaseEntry.runtimeIdentity = unit.identity;
        releaseEntry.targetIdentity = unit.target;
        releaseEntry.abiIdentity = 'fixture-abi';
    }
    writeJson(entry, releaseEntry);
    const checksumFiles = [...artifactNames, entryName].sort();
    writeFileSync(
        resolve(root, checksumName),
        `${checksumFiles.map((filename) => `${sha256File(resolve(root, filename))}  ${filename}`).join('\n')}\n`,
    );
    writeJson(signing, {
        commonVersion: version,
        releaseTag: `cem-ml-v${version}`,
        checksumManifest: { filename: checksumName, sha256: sha256File(resolve(root, checksumName)) },
        githubArtifactAttestation: { status: 'awaiting-github-oidc', bundle: null, sha256: null },
        publicationReady: false,
    });
    return {
        root,
        primary,
        capability,
        integrity,
        sbom,
        provenance,
        entry,
        checksum: resolve(root, checksumName),
        signing,
    };
}

function artifactRecord(root, filename) {
    const path = resolve(root, filename);
    return { filename, byteLength: statSync(path).size, sha256: sha256File(path) };
}

function sha256File(path) {
    return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function writeJson(path, value) {
    writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}

function readProject(relativePath) {
    return readJson(resolve(workspaceRoot, relativePath));
}

function updateJson(path, mutate) {
    const value = JSON.parse(readFileSync(path, 'utf8'));
    mutate(value);
    writeJson(path, value);
}

function restoreEnvironment(name, value) {
    if (value === undefined) delete process.env[name];
    else process.env[name] = value;
}
