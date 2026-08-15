import { readdirSync } from 'node:fs';

import {
    artifactPath,
    artifactRoot,
    assetNames,
    authoritativeVersion,
    capture,
    deployment,
    readJson,
    releaseTag,
    requireFile,
    run,
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
const release = JSON.parse(capture('gh', ['release', 'view', tag, '--json', 'isDraft,tagName']));
if (release.tagName !== tag || release.isDraft !== true) {
    throw new Error(`${tag} must already exist as a draft GitHub Release`);
}

const assets = readdirSync(artifactRoot)
    .filter((filename) => filename.startsWith(`${names.base}.`))
    .sort()
    .map((filename) => artifactPath(filename));
for (const asset of assets) requireFile(asset);
run('gh', ['release', 'upload', tag, ...assets]);
console.log(
    `Uploaded ${assets.length} immutable ${deployment.runtimeIdentity} assets to draft release ${tag}; APT consumes ${names.apt}.`,
);
