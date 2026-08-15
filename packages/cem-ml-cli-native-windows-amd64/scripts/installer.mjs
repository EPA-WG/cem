import { readFileSync, writeFileSync } from 'node:fs';
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
    normalizeCompoundFileMetadata(destination, sourceEpoch);
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
        '$summary = $installer.SummaryInformation($env:CEM_ML_MSI_SUMMARY_PATH, 3)',
        '$binding = [System.Reflection.BindingFlags]::SetProperty',
        "$summary.GetType().InvokeMember('Property', $binding, $null, $summary, @(9, $env:CEM_ML_MSI_PACKAGE_CODE)) | Out-Null",
        '$timestamp = [DateTimeOffset]::FromUnixTimeSeconds([int64]$env:CEM_ML_MSI_SOURCE_EPOCH).UtcDateTime',
        "$summary.GetType().InvokeMember('Property', $binding, $null, $summary, @(12, $timestamp)) | Out-Null",
        "$summary.GetType().InvokeMember('Property', $binding, $null, $summary, @(13, $timestamp)) | Out-Null",
        '$summary.Persist()',
    ].join('\n');
    run('powershell.exe', ['-NoLogo', '-NoProfile', '-NonInteractive', '-Command', script], {
        env: {
            CEM_ML_MSI_SUMMARY_PATH: requireFile(path),
            CEM_ML_MSI_PACKAGE_CODE: installerPackageCode,
            CEM_ML_MSI_SOURCE_EPOCH: String(epoch),
        },
    });
}

function normalizeCompoundFileMetadata(path, epoch) {
    const image = readFileSync(path);
    const signature = Buffer.from([0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]);
    if (image.length < 512 || !image.subarray(0, signature.length).equals(signature)) {
        throw new Error(`${path} is not a Compound File Binary MSI`);
    }
    if (image.readUInt16LE(0x1c) !== 0xfffe) {
        throw new Error(`${path} has an unsupported Compound File byte order`);
    }

    const sectorSize = 2 ** image.readUInt16LE(0x1e);
    if (![512, 4096].includes(sectorSize) || image.length % sectorSize !== 0) {
        throw new Error(`${path} has an unsupported Compound File sector size ${sectorSize}`);
    }
    const sectorCount = image.length / sectorSize - 1;
    const sectorOffset = (sector, label) => {
        if (sector >= sectorCount) throw new Error(`${path} ${label} sector ${sector} is out of range`);
        return (sector + 1) * sectorSize;
    };

    const freeSector = 0xffffffff;
    const endOfChain = 0xfffffffe;
    const maximumRegularSector = 0xfffffffa;
    const fatSectorCount = image.readUInt32LE(0x2c);
    const fatSectors = [];
    for (let index = 0; index < 109; index += 1) {
        const sector = image.readUInt32LE(0x4c + index * 4);
        if (sector !== freeSector) fatSectors.push(sector);
    }

    let difatSector = image.readUInt32LE(0x44);
    const difatSectorCount = image.readUInt32LE(0x48);
    for (let chainIndex = 0; chainIndex < difatSectorCount; chainIndex += 1) {
        if (difatSector >= maximumRegularSector) {
            throw new Error(`${path} has an invalid DIFAT chain`);
        }
        const offset = sectorOffset(difatSector, 'DIFAT');
        for (let index = 0; index < sectorSize / 4 - 1; index += 1) {
            const sector = image.readUInt32LE(offset + index * 4);
            if (sector !== freeSector) fatSectors.push(sector);
        }
        difatSector = image.readUInt32LE(offset + sectorSize - 4);
    }
    if (fatSectors.length < fatSectorCount) {
        throw new Error(`${path} declares ${fatSectorCount} FAT sectors but exposes ${fatSectors.length}`);
    }

    const fat = [];
    for (const sector of fatSectors.slice(0, fatSectorCount)) {
        if (sector >= maximumRegularSector) throw new Error(`${path} has an invalid FAT sector ${sector}`);
        const offset = sectorOffset(sector, 'FAT');
        for (let index = 0; index < sectorSize / 4; index += 1) {
            fat.push(image.readUInt32LE(offset + index * 4));
        }
    }

    image.writeUInt32LE(0, 0x34);
    const sourceFiletime = (BigInt(epoch) + 11_644_473_600n) * 10_000_000n;
    const visited = new Set();
    let directorySector = image.readUInt32LE(0x30);
    while (directorySector !== endOfChain) {
        if (directorySector >= maximumRegularSector || visited.has(directorySector)) {
            throw new Error(`${path} has an invalid Compound File directory chain`);
        }
        visited.add(directorySector);
        const offset = sectorOffset(directorySector, 'directory');
        for (let entryOffset = offset; entryOffset < offset + sectorSize; entryOffset += 128) {
            const objectType = image[entryOffset + 66];
            if (![1, 2, 5].includes(objectType)) continue;
            const timestamp = objectType === 2 ? 0n : sourceFiletime;
            image.writeBigUInt64LE(timestamp, entryOffset + 100);
            image.writeBigUInt64LE(timestamp, entryOffset + 108);
        }
        directorySector = fat[directorySector];
    }
    writeFileSync(path, image);
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
