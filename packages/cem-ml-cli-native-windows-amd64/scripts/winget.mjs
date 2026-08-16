import { writeFileSync } from 'node:fs';

import {
    artifactPath,
    deployment,
    productCode,
    releaseAssetUrl,
    releaseTag,
    sha256File,
    sourceCommit,
    writeJson,
} from './lib.mjs';

export function writeWingetArtifacts({ names, version }) {
    const identifier = deployment.windowsInstaller.packageIdentifier;
    const manifestVersion = deployment.windowsInstaller.wingetManifestVersion;
    const msiSha256 = sha256File(artifactPath(names.msi)).toUpperCase();
    const installerCode = productCode(version);
    writeFileSync(
        artifactPath(names.wingetInstaller),
        `# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.${manifestVersion}.schema.json
PackageIdentifier: ${identifier}
PackageVersion: ${version}
InstallerLocale: en-US
InstallerType: wix
Scope: machine
InstallModes:
- interactive
- silent
- silentWithProgress
UpgradeBehavior: install
Commands:
- cem-ml
ElevationRequirement: elevationRequired
Installers:
- Architecture: x64
  InstallerUrl: ${releaseAssetUrl(names.msi, version)}
  InstallerSha256: ${msiSha256}
  ProductCode: '${installerCode}'
  AppsAndFeaturesEntries:
  - DisplayName: CEM-ML
    Publisher: ${deployment.windowsInstaller.publisher}
    DisplayVersion: ${version}
    ProductCode: '${installerCode}'
    UpgradeCode: '${deployment.windowsInstaller.upgradeCode}'
  InstallationMetadata:
    DefaultInstallLocation: '%ProgramFiles%\\EPA-WG\\CEM-ML'
ManifestType: installer
ManifestVersion: ${manifestVersion}
`,
    );
    writeFileSync(
        artifactPath(names.wingetLocale),
        `# yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.${manifestVersion}.schema.json
PackageIdentifier: ${identifier}
PackageVersion: ${version}
PackageLocale: en-US
Publisher: ${deployment.windowsInstaller.publisher}
PublisherUrl: https://github.com/EPA-WG
PublisherSupportUrl: https://github.com/EPA-WG/cem/issues
PackageName: CEM-ML
PackageUrl: https://github.com/EPA-WG/cem
License: MIT
LicenseUrl: https://github.com/EPA-WG/cem/blob/${sourceCommit()}/LICENSE
ShortDescription: CEM schema-defined parser, validator, query, and transformation CLI
Moniker: cem-ml
Tags:
- cem
- cli
- parser
- schema
ManifestType: defaultLocale
ManifestVersion: ${manifestVersion}
`,
    );
    writeFileSync(
        artifactPath(names.wingetVersion),
        `# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.${manifestVersion}.schema.json
PackageIdentifier: ${identifier}
PackageVersion: ${version}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: ${manifestVersion}
`,
    );

    const repositoryRoot = `manifests/e/EPA-WG/CEM-ML/${version}`;
    writeJson(artifactPath(names.winget), {
        schemaVersion: 1,
        channel: 'winget',
        repository: deployment.windowsInstaller.wingetRepository,
        packageIdentifier: identifier,
        version,
        immutableSource: {
            releaseTag: releaseTag(version),
            filename: names.msi,
            url: releaseAssetUrl(names.msi, version),
            sha256: msiSha256.toLowerCase(),
            productCode: installerCode,
            upgradeCode: deployment.windowsInstaller.upgradeCode,
        },
        manifestProjection: {
            rebuildExecutable: false,
            files: [
                manifestRecord(names.wingetInstaller, `${repositoryRoot}/${identifier}.installer.yaml`),
                manifestRecord(names.wingetLocale, `${repositoryRoot}/${identifier}.locale.en-US.yaml`),
                manifestRecord(names.wingetVersion, `${repositoryRoot}/${identifier}.yaml`),
            ],
        },
    });
}

function manifestRecord(filename, repositoryPath) {
    return {
        filename,
        repositoryPath,
        sha256: sha256File(artifactPath(filename)),
    };
}
