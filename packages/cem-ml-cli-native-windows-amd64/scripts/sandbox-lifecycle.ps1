param(
    [Parameter(Mandatory = $true)][string]$InputRoot,
    [Parameter(Mandatory = $true)][string]$OutputRoot,
    [Parameter(Mandatory = $true)][string]$CurrentProductCode,
    [Parameter(Mandatory = $true)][string]$FixtureProductCode,
    [Parameter(Mandatory = $true)][string]$ExpectedVersion
)

$ErrorActionPreference = 'Stop'
$installRoot = Join-Path $env:ProgramFiles 'EPA-WG\CEM-ML'
$binary = Join-Path $installRoot 'cem-ml.exe'
$metadata = Join-Path $installRoot 'share\cem-ml\build-metadata.json'
$currentMsi = Join-Path $InputRoot 'current.msi'
$fixtureMsi = Join-Path $InputRoot 'fixture.msi'
$smokeInput = Join-Path $InputRoot 'smoke-input.cem'
$completed = [System.Collections.Generic.List[string]]::new()
$result = $null

function Invoke-Msi {
    param(
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$LogName,
        [switch]$Cleanup
    )
    $log = Join-Path $OutputRoot $LogName
    $allArguments = @($Arguments) + @('/qn', '/norestart', '/l*v', "`"$log`"")
    $process = Start-Process -FilePath 'msiexec.exe' -ArgumentList $allArguments -PassThru -Wait
    $accepted = if ($Cleanup) { @(0, 1605, 1614, 1641, 3010) } else { @(0, 1641, 3010) }
    if ($process.ExitCode -notin $accepted) {
        throw "msiexec $($Arguments -join ' ') failed with exit code $($process.ExitCode); see $log"
    }
}

function Get-ProductState {
    param([Parameter(Mandatory = $true)][string]$ProductCode)
    $installer = New-Object -ComObject WindowsInstaller.Installer
    return $installer.ProductState($ProductCode)
}

function Assert-CurrentInstallation {
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "installed executable is missing: $binary"
    }
    $reported = (& $binary version | Select-Object -First 1).Trim()
    if ($LASTEXITCODE -ne 0 -or $reported -ne "cem-ml $ExpectedVersion") {
        throw "installed executable reported '$reported', expected 'cem-ml $ExpectedVersion'"
    }
    $validation = (& $binary validate $smokeInput --format json | Out-String | ConvertFrom-Json)
    if ($LASTEXITCODE -ne 0 -or $validation.summary.inputCount -ne 1 -or
        $validation.summary.hardViolationCount -ne 0) {
        throw 'installed executable failed validation smoke'
    }
    $conversion = (& $binary convert $smokeInput --to-format dom-json --preserve-source-offsets |
        Out-String | ConvertFrom-Json)
    if ($LASTEXITCODE -ne 0 -or $conversion.kind -ne 'document') {
        throw 'installed executable failed conversion smoke'
    }
    $installedMetadata = Get-Content -LiteralPath $metadata -Raw | ConvertFrom-Json
    if ($installedMetadata.commonVersion -ne $ExpectedVersion) {
        throw "installed metadata version is $($installedMetadata.commonVersion)"
    }
}

function Remove-SmokeProducts {
    Invoke-Msi -Arguments @('/x', $CurrentProductCode) -LogName 'cleanup-current.log' -Cleanup
    Invoke-Msi -Arguments @('/x', $FixtureProductCode) -LogName 'cleanup-fixture.log' -Cleanup
}

try {
    New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
    Remove-SmokeProducts

    Invoke-Msi -Arguments @('/i', "`"$currentMsi`"") -LogName 'install.log'
    Assert-CurrentInstallation
    $completed.Add('install')

    Invoke-Msi -Arguments @('/x', $CurrentProductCode) -LogName 'uninstall.log'
    if ((Get-ProductState $CurrentProductCode) -eq 5 -or (Test-Path -LiteralPath $installRoot)) {
        throw 'current product or installation directory survived uninstall'
    }
    $completed.Add('uninstall')

    Invoke-Msi -Arguments @('/i', "`"$fixtureMsi`"") -LogName 'fixture-install.log'
    $fixtureMetadata = Get-Content -LiteralPath $metadata -Raw | ConvertFrom-Json
    if ($fixtureMetadata.commonVersion -ne '0.0.0-fixture') {
        throw 'fixture MSI did not install its predecessor metadata'
    }
    Invoke-Msi -Arguments @('/i', "`"$currentMsi`"") -LogName 'upgrade.log'
    Assert-CurrentInstallation
    if ((Get-ProductState $FixtureProductCode) -eq 5) {
        throw 'fixture MSI survived the major upgrade'
    }
    $completed.Add('upgrade')

    $result = [ordered]@{
        status = 'passed'
        completed = @($completed)
        error = $null
    }
}
catch {
    $result = [ordered]@{
        status = 'failed'
        completed = @($completed)
        error = ($_ | Out-String).Trim()
    }
}
finally {
    try { Remove-SmokeProducts } catch { }
    $json = $result | ConvertTo-Json -Depth 6
    $utf8 = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText((Join-Path $OutputRoot 'result.json'), $json, $utf8)
    Start-Process -FilePath 'shutdown.exe' -ArgumentList @('/s', '/t', '0', '/f') -WindowStyle Hidden
}
