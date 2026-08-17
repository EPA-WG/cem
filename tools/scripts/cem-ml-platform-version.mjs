import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptPath = fileURLToPath(import.meta.url);
const defaultWorkspaceRoot = resolve(scriptPath, '../../..');

export const cargoPackages = Object.freeze([
    ['packages/cem_ml/Cargo.toml', 'cem-ml'],
    ['packages/cem_ml_cli/Cargo.toml', 'cem-ml-cli'],
    ['packages/cem_ml_transform_cem_ql/Cargo.toml', 'cem-ml-transform-cem-ql'],
    ['packages/cem_ql/Cargo.toml', 'cem-ql'],
    ['packages/cem_ml/grammar/tree-sitter-cem/Cargo.toml', 'tree-sitter-cem'],
]);

const cargoDependencies = Object.freeze([
    ['packages/cem_ml_cli/Cargo.toml', 'dependencies', 'cem-ml'],
    ['packages/cem_ml_cli/Cargo.toml', 'dependencies', 'cem-ml-transform-cem-ql'],
    ['packages/cem_ml_cli/Cargo.toml', 'dev-dependencies', 'cem-ml'],
    ['packages/cem_ml_transform_cem_ql/Cargo.toml', 'dependencies', 'cem-ml'],
    ['packages/cem_ml_transform_cem_ql/Cargo.toml', 'dependencies', 'cem-ql'],
    ['packages/cem_ql/Cargo.toml', 'dependencies', 'cem-ml'],
]);

const npmPackages = Object.freeze(['packages/cem-ml-npm/package.json', 'packages/cem-ml-cli-npm/package.json']);

const nativeDeployments = Object.freeze([
    'packages/cem-ml-cli-native-linux-amd64/deployment.json',
    'packages/cem-ml-cli-native-brew-arm64/deployment.json',
    'packages/cem-ml-cli-native-windows-amd64/deployment.json',
]);

export const governedPlatformVersionFiles = Object.freeze([
    ...cargoPackages.map(([path]) => path),
    ...npmPackages,
    ...nativeDeployments,
    'Cargo.lock',
    'yarn.lock',
]);

export function authoritativePlatformVersion(workspaceRoot = defaultWorkspaceRoot) {
    const manifest = readFileSync(resolve(workspaceRoot, 'packages/cem_ml/Cargo.toml'), 'utf8');
    const version = packageVersion(manifest, 'packages/cem_ml/Cargo.toml');
    if (!isSemver(version)) throw new Error(`authoritative CEM-ML version is not SemVer: ${version}`);
    return version;
}

export function synchronizePlatformVersion({ workspaceRoot = defaultWorkspaceRoot, write = false } = {}) {
    const version = authoritativePlatformVersion(workspaceRoot);
    const expectedDependency = `=${version}`;
    const updates = new Map();

    for (const [relativePath] of cargoPackages.slice(1)) {
        transform(relativePath, (source) => updatePackageVersion(source, version, relativePath));
    }
    for (const [relativePath, section, dependency] of cargoDependencies) {
        transform(relativePath, (source) =>
            updateCargoDependency(source, section, dependency, expectedDependency, relativePath),
        );
    }
    for (const relativePath of npmPackages) {
        transform(relativePath, (source) => {
            const metadata = JSON.parse(source);
            metadata.version = version;
            if (metadata.name === '@epa-wg/cem-ml-cli') {
                metadata.dependencies ??= {};
                metadata.dependencies['@epa-wg/cem-ml'] = version;
            }
            return formatJsonLike(source, metadata);
        });
    }
    for (const relativePath of nativeDeployments) {
        transform(relativePath, (source) => {
            const deployment = JSON.parse(source);
            deployment.commonVersion = version;
            return formatJsonLike(source, deployment);
        });
    }
    transform('Cargo.lock', (source) => updateCargoLock(source, version));
    transform('yarn.lock', (source) => updateYarnLock(source, version));

    const changed = [...updates.entries()].filter(([relativePath, next]) => {
        const current = readFileSync(resolve(workspaceRoot, relativePath), 'utf8');
        return current !== next;
    });
    if (!write && changed.length > 0) {
        throw new Error(
            `CEM-ML platform version drift from ${version}:\n${changed
                .map(([relativePath]) => `- ${relativePath}`)
                .join('\n')}\nRun: yarn nx run cem_ml:sync:platform-version`,
        );
    }
    if (write) {
        for (const [relativePath, next] of changed) writeFileSync(resolve(workspaceRoot, relativePath), next);
    }
    return { version, changedFiles: changed.map(([relativePath]) => relativePath) };

    function transform(relativePath, update) {
        const current = updates.get(relativePath) ?? readFileSync(resolve(workspaceRoot, relativePath), 'utf8');
        updates.set(relativePath, update(current));
    }
}

function packageVersion(manifest, relativePath) {
    const section = tomlSection(manifest, 'package', relativePath);
    const version = section.body.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
    if (!version) throw new Error(`cannot find [package].version in ${relativePath}`);
    return version;
}

function updatePackageVersion(manifest, version, relativePath) {
    const section = tomlSection(manifest, 'package', relativePath);
    const body = section.body.replace(/^(version\s*=\s*)"[^"]+"\s*$/m, `$1"${version}"`);
    if (body === section.body && packageVersion(manifest, relativePath) !== version) {
        throw new Error(`cannot update [package].version in ${relativePath}`);
    }
    return `${manifest.slice(0, section.start)}${body}${manifest.slice(section.end)}`;
}

function updateCargoDependency(manifest, sectionName, dependency, version, relativePath) {
    const section = tomlSection(manifest, sectionName, relativePath);
    const escapedDependency = escapeRegex(dependency);
    const linePattern = new RegExp(`^(${escapedDependency}\\s*=\\s*\\{)([^\\n}]*)(\\}\\s*)$`, 'm');
    const match = section.body.match(linePattern);
    if (!match) throw new Error(`cannot find ${dependency} in [${sectionName}] of ${relativePath}`);
    let fields = match[2];
    if (/\bversion\s*=/.test(fields)) {
        fields = fields.replace(/\bversion\s*=\s*"[^"]+"/, `version = "${version}"`);
    } else {
        fields = `${fields.startsWith(' ') ? ' ' : ''}version = "${version}", ${fields.trimStart()}`;
    }
    const body = section.body.replace(linePattern, `$1${fields}$3`);
    return `${manifest.slice(0, section.start)}${body}${manifest.slice(section.end)}`;
}

function updateCargoLock(lock, version) {
    let next = lock;
    for (const [, packageName] of cargoPackages) {
        const packagePattern = new RegExp(
            `(\\[\\[package\\]\\]\\nname = "${escapeRegex(packageName)}"\\nversion = ")[^"]+("\\n)`,
        );
        if (!packagePattern.test(next)) throw new Error(`cannot find ${packageName} package in Cargo.lock`);
        next = next.replace(packagePattern, `$1${version}$2`);
    }
    return next;
}

function updateYarnLock(lock, version) {
    const dependencyPattern = /( {4}"@epa-wg\/cem-ml": "npm:)[^"]+("\n)/;
    const selectorPattern = /("@epa-wg\/cem-ml@npm:)[^,]+(, @epa-wg\/cem-ml@workspace:packages\/cem-ml-npm":\n)/;
    if (!dependencyPattern.test(lock) || !selectorPattern.test(lock)) {
        throw new Error('cannot find the exact @epa-wg/cem-ml workspace dependency in yarn.lock');
    }
    return lock.replace(dependencyPattern, `$1${version}$2`).replace(selectorPattern, `$1${version}$2`);
}

function tomlSection(manifest, sectionName, relativePath) {
    const header = `[${sectionName}]`;
    const headerStart = manifest.indexOf(header);
    if (headerStart < 0) throw new Error(`cannot find ${header} in ${relativePath}`);
    const start = headerStart + header.length;
    const nextSection = manifest.indexOf('\n[', start);
    return {
        start,
        end: nextSection < 0 ? manifest.length : nextSection,
        body: manifest.slice(start, nextSection < 0 ? undefined : nextSection),
    };
}

function formatJsonLike(source, value) {
    const indent = source.match(/\n( +)"/)?.[1]?.length ?? 2;
    return `${JSON.stringify(value, null, indent)}\n`;
}

function isSemver(version) {
    return /^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/.test(
        version,
    );
}

function escapeRegex(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
    const arguments_ = process.argv.slice(2);
    const write = arguments_.includes('--write');
    if (arguments_.some((argument) => !['--write', '--check'].includes(argument))) {
        throw new Error('usage: node tools/scripts/cem-ml-platform-version.mjs [--check|--write]');
    }
    const result = synchronizePlatformVersion({ write });
    console.log(
        write
            ? `Synchronized CEM-ML platform ${result.version} across ${result.changedFiles.length} file(s).`
            : `Verified CEM-ML platform version ${result.version} across ${governedPlatformVersionFiles.length} files.`,
    );
}
