import { uploadImmutableDraftAssetSet } from '../../../tools/scripts/cem-ml-platform-release.mjs';

import {
    artifactPath,
    artifactRoot,
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

if (process.env.CEM_ML_NATIVE_PUBLISH !== '1') {
    throw new Error('native publication is disabled; set CEM_ML_NATIVE_PUBLISH=1 in the protected release job');
}
const version = authoritativeVersion();
const names = assetNames(version);
const signing = readJson(requireFile(artifactPath(names.signing)));
if (signing.publicationReady !== true) {
    throw new Error('native publication requires a publication-ready signing record');
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
});
console.log(
    `Verified ${result.filenames.length} immutable ${deployment.runtimeIdentity} assets in draft release ${tag}; ` +
        `uploaded ${result.uploaded.length} missing assets and APT consumes ${names.apt}.`,
);
