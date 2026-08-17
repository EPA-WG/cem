const { VersionActions } = require('nx/release');

const AUTHORITY_MANIFEST = 'packages/cem_ml/Cargo.toml';

class CemMlPlatformVersionActions extends VersionActions {
    // Nx uses the presence of a valid manifest kind to authorize the disk
    // resolver. Every member still reads the one Cargo authority below.
    validManifestFilenames = ['Cargo.toml'];

    async validate(tree) {
        readAuthority(tree);
    }

    async readCurrentVersionFromSourceManifest(tree) {
        return {
            currentVersion: readAuthority(tree),
            manifestPath: AUTHORITY_MANIFEST,
        };
    }

    async readCurrentVersionFromRegistry() {
        return null;
    }

    async readCurrentVersionOfDependency(tree) {
        return {
            currentVersion: `=${readAuthority(tree)}`,
            dependencyCollection: null,
        };
    }

    async updateProjectVersion(tree, newVersion) {
        const authority = readAuthority(tree);
        if (newVersion !== authority) {
            throw new Error(
                `Nx Release requested CEM-ML ${newVersion}, but ${AUTHORITY_MANIFEST} authorizes only ${authority}. ` +
                    'Edit the Cargo authority and run the sync target first.',
            );
        }
        return [`Verified ${this.projectGraphNode.name} at Cargo-authoritative version ${authority}`];
    }

    async updateProjectDependencies(tree, projectGraph, dependenciesToUpdate) {
        const authority = readAuthority(tree);
        for (const [project, version] of Object.entries(dependenciesToUpdate)) {
            if (version !== authority && version !== `=${authority}`) {
                throw new Error(
                    `${this.projectGraphNode.name} dependency ${project} drifted to ${version}; expected ${authority}`,
                );
            }
        }
        return [];
    }
}

function readAuthority(tree) {
    const source = tree.read(AUTHORITY_MANIFEST, 'utf8');
    if (source === null) throw new Error(`missing CEM-ML version authority ${AUTHORITY_MANIFEST}`);
    const packageStart = source.indexOf('[package]');
    const nextSection = source.indexOf('\n[', packageStart + '[package]'.length);
    const packageSection =
        packageStart < 0 ? undefined : source.slice(packageStart, nextSection < 0 ? undefined : nextSection);
    const version = packageSection?.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
    if (!version) throw new Error(`cannot read [package].version from ${AUTHORITY_MANIFEST}`);
    return version;
}

module.exports = CemMlPlatformVersionActions;
