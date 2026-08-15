import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

import {
    capture,
    deployment,
    ensureDirectory,
    packageCode,
    productCode,
    requireFile,
    resetDirectory,
    run,
    sha256File,
} from './lib.mjs';

export function buildMsi({ destination, payloadRoot, version, workRoot, sourceEpoch }) {
    const wix = process.env.CEM_ML_WIX?.trim() || 'wix.exe';
    const wixVersion = capture(wix, ['--version']).trim();
    if (!new RegExp(`^${escapeRegex(deployment.wix.version)}(?:[+-]|$)`).test(wixVersion)) {
        throw new Error(`expected WiX ${deployment.wix.version}, got ${wixVersion}`);
    }
    const payload = payloadPaths(payloadRoot);
    for (const [label, path] of Object.entries(payload)) requireFile(path, `MSI ${label}`);
    const identity = [version, ...Object.entries(payload).map(([label, path]) => `${label}:${sha256File(path)}`)].join(
        '|',
    );
    const installerPackageCode = packageCode(identity);
    const source = resolve(workRoot, `cem-ml-${version}.wxs`);
    const intermediate = resolve(workRoot, `wix-${version}`);
    resetDirectory(intermediate);
    writeFileSync(
        source,
        renderInstallerSource({
            installerPackageCode,
            payload,
            version,
        }),
    );
    ensureDirectory(resolve(destination, '..'));
    run(wix, [
        'build',
        '-arch',
        'x64',
        '-dcl',
        'high',
        '-pdbtype',
        'none',
        '-intermediateFolder',
        intermediate,
        '-o',
        destination,
        source,
    ]);
    normalizeMsiSummary(destination, installerPackageCode, sourceEpoch);
    requireFile(destination, 'WiX MSI');
    return {
        packageCode: installerPackageCode,
        productCode: productCode(version),
        upgradeCode: deployment.windowsInstaller.upgradeCode,
        wixVersion,
    };
}

export function renderInstallerSource({ installerPackageCode, payload, version }) {
    const msiVersion = windowsInstallerVersion(version);
    const { componentGuids, publisher, upgradeCode } = deployment.windowsInstaller;
    return `<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Package Name="CEM-ML" Manufacturer="${escapeXml(publisher)}" Version="${msiVersion}"
           ProductCode="${productCode(version)}" UpgradeCode="${upgradeCode}"
           Language="1033" Scope="perMachine" InstallerVersion="500" Compressed="yes">
    <SummaryInformation Description="CEM-ML Installation Database"
                        Manufacturer="${escapeXml(publisher)}" />
    <MajorUpgrade DowngradeErrorMessage="A newer version of CEM-ML is already installed."
                  Schedule="afterInstallInitialize" />
    <MediaTemplate EmbedCab="yes" CompressionLevel="high" />
    <Property Id="ARPNOMODIFY" Value="1" />
    <Property Id="ARPURLINFOABOUT" Value="https://github.com/EPA-WG/cem" />
    <Property Id="ARPHELPLINK" Value="https://github.com/EPA-WG/cem/issues" />

    <StandardDirectory Id="ProgramFiles64Folder">
      <Directory Id="EPAWGFolder" Name="EPA-WG">
        <Directory Id="INSTALLFOLDER" Name="CEM-ML">
          <Component Id="CemMlExecutable" Guid="${componentGuids.executable}" Bitness="always64">
            <File Id="CemMlExe" Source="${escapeXml(payload.binary)}" KeyPath="yes" Checksum="yes" />
            <Environment Id="CemMlMachinePath" Name="PATH" Value="[INSTALLFOLDER]"
                         Action="set" Part="last" System="yes" Permanent="no" />
          </Component>
          <Directory Id="CemMlShareFolder" Name="share">
            <Directory Id="CemMlShareProductFolder" Name="cem-ml">
              <Component Id="CemMlMetadata" Guid="${componentGuids.metadata}" Bitness="always64">
                <File Id="CemMlCapabilities" Source="${escapeXml(payload.capabilities)}" KeyPath="yes" />
                <File Id="CemMlBuildMetadata" Source="${escapeXml(payload.buildMetadata)}" />
              </Component>
            </Directory>
          </Directory>
          <Component Id="CemMlDocumentation" Guid="${componentGuids.documentation}" Bitness="always64">
            <File Id="CemMlLicense" Source="${escapeXml(payload.license)}" KeyPath="yes" />
            <File Id="CemMlReadme" Source="${escapeXml(payload.readme)}" />
          </Component>
        </Directory>
      </Directory>
    </StandardDirectory>

    <Feature Id="Complete" Title="CEM-ML" Level="1">
      <ComponentRef Id="CemMlExecutable" />
      <ComponentRef Id="CemMlMetadata" />
      <ComponentRef Id="CemMlDocumentation" />
    </Feature>
  </Package>
</Wix>
<!-- deterministic-package-code: ${installerPackageCode} -->
`;
}

function payloadPaths(payloadRoot) {
    return {
        binary: resolve(payloadRoot, 'bin/cem-ml.exe'),
        capabilities: resolve(payloadRoot, 'share/cem-ml/capabilities.json'),
        buildMetadata: resolve(payloadRoot, 'share/cem-ml/build-metadata.json'),
        license: resolve(payloadRoot, 'LICENSE'),
        readme: resolve(payloadRoot, 'README.md'),
    };
}

function normalizeMsiSummary(path, installerPackageCode, epoch) {
    const script = [
        '$installer = New-Object -ComObject WindowsInstaller.Installer',
        '$summary = $installer.SummaryInformation($args[0], 3)',
        '$binding = [System.Reflection.BindingFlags]::SetProperty',
        "$summary.GetType().InvokeMember('Property', $binding, $null, $summary, @(9, $args[1])) | Out-Null",
        '$timestamp = [DateTimeOffset]::FromUnixTimeSeconds([int64]$args[2]).UtcDateTime',
        "$summary.GetType().InvokeMember('Property', $binding, $null, $summary, @(12, $timestamp)) | Out-Null",
        "$summary.GetType().InvokeMember('Property', $binding, $null, $summary, @(13, $timestamp)) | Out-Null",
        '$summary.Persist()',
    ].join('\n');
    run('powershell.exe', [
        '-NoLogo',
        '-NoProfile',
        '-NonInteractive',
        '-Command',
        script,
        requireFile(path),
        installerPackageCode,
        String(epoch),
    ]);
}

function windowsInstallerVersion(version) {
    const match = version.match(/^(\d+)\.(\d+)\.(\d+)(?:[-+].*)?$/);
    if (match === null) throw new Error(`version ${version} cannot be represented by Windows Installer`);
    const parts = match.slice(1).map(Number);
    if (parts[0] > 255 || parts[1] > 255 || parts[2] > 65535) {
        throw new Error(`version ${version} exceeds Windows Installer version ranges`);
    }
    return parts.join('.');
}

function escapeXml(value) {
    return value.replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}

function escapeRegex(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
