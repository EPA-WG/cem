import { resolve } from 'node:path';

import { uploadImmutableDraftAssetSet } from '../../../tools/scripts/cem-ml-platform-release.mjs';

import {
    artifactPath,
    artifactRoot,
    assertNativeHost,
    assertValidAuthenticode,
    assetNames,
    authoritativeVersion,
    deployment,
    projectRoot,
    readJson,
    releaseTag,
    requireFile,
    run,
    workspaceRoot,
} from './lib.mjs';

assertNativeHost();
if (process.env.CEM_ML_NATIVE_PUBLISH !== '1') {
    throw new Error('native publication is disabled; set CEM_ML_NATIVE_PUBLISH=1 for an authorized local release');
}
const version = authoritativeVersion();
const names = assetNames(version);
const signing = readJson(requireFile(artifactPath(names.signing)));
if (signing.publicationReady !== true) {
    throw new Error('native Windows publication requires a finalized attested signing record');
}

const tag = releaseTag(version);
run('node', ['scripts/verify.mjs'], {
    cwd: projectRoot,
    env: { CEM_ML_RELEASE_VERIFY: '1' },
});
const result = uploadImmutableDraftAssetSet({
    workspaceRoot,
    identity: deployment.runtimeIdentity,
    version,
    releaseTag: tag,
    assetRoot: artifactRoot,
    ownedBase: names.base,
    verifyDownloaded: ({ downloadRoot, paths }) => {
        const downloadedArchive = requireFile(paths.get(names.archive), 'downloaded Windows archive');
        const downloadedMsi = requireFile(paths.get(names.msi), 'downloaded Windows installer');
        const extractedRoot = resolve(downloadRoot, 'archive');
        run(
            'pwsh.exe',
            [
                '-NoLogo',
                '-NoProfile',
                '-NonInteractive',
                '-Command',
                'Expand-Archive -LiteralPath $env:CEM_ML_ARCHIVE_PATH -DestinationPath $env:CEM_ML_ARCHIVE_DESTINATION -Force',
            ],
            {
                env: {
                    CEM_ML_ARCHIVE_PATH: downloadedArchive,
                    CEM_ML_ARCHIVE_DESTINATION: extractedRoot,
                },
            },
        );
        const downloadedBinary = requireFile(resolve(extractedRoot, names.base, 'bin/cem-ml.exe'));
        assertValidAuthenticode(downloadedBinary, 'downloaded ZIP executable');
        assertValidAuthenticode(downloadedMsi, 'downloaded MSI');
    },
});
console.log(
    `Verified ${result.filenames.length} immutable ${deployment.runtimeIdentity} assets in draft release ${tag}; ` +
        `uploaded ${result.uploaded.length} missing assets, post-download Authenticode verification passed, and ` +
        `${deployment.windowsInstaller.wingetRepository} ` +
        `consumes the versioned manifest projections.`,
);
