import { assembleArtifacts } from './assemble.mjs';
import { deployment } from './lib.mjs';

const { names, version } = await assembleArtifacts();
console.log(
    `Packaged ${deployment.runtimeIdentity} ${version}: ${names.archive}, SPDX SBOM, and Homebrew release metadata.`,
);
