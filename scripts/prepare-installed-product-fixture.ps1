<#
.SYNOPSIS
Prepares the development-only signed installer fixture for Anodrel on Windows.

.DESCRIPTION
This script assembles, but never installs, one fixed development fixture through
the same owned release chain used by the Windows installer: package staging,
bundle authoring, manifest derivation, resource embedding, and Authenticode
signing. It creates a local development code-signing certificate and adds it to
machine trust, which is a real machine change. Run it only on a development
machine and use -Remove after the signed installer has explicitly uninstalled
the fixture.

The script has no product, package, certificate, output, or installer command
inputs. Its only optional action is -Remove, which removes only this script's
known local output and certificate entries after no valid fixture record is
selected.

.EXAMPLE
PS> .\scripts\prepare-installed-product-fixture.ps1

.EXAMPLE
PS> .\scripts\prepare-installed-product-fixture.ps1 -Remove
#>

[CmdletBinding()]
param(
    [switch] $Remove
)

$ErrorActionPreference = 'Stop'

$CertificateSubject = 'CN=Anodrel Development Installed Fixture'
$FixtureVersion = '0.1.0'
$RepositoryRoot = Split-Path -Parent $PSScriptRoot
$CargoManifest = Join-Path $RepositoryRoot 'native\Cargo.toml'
$LocalData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
$FixtureRoot = [IO.Path]::GetFullPath((Join-Path $LocalData 'Anodrel\InstalledProductFixture'))
$PackageRoot = Join-Path $FixtureRoot 'package'
$PlanPath = Join-Path $FixtureRoot 'fixture.release-plan.json'
$BundlePath = Join-Path $FixtureRoot 'fixture.bundle'
$ManifestPath = Join-Path $FixtureRoot 'fixture.release.json'
$UnsignedInstallerPath = Join-Path $FixtureRoot 'fixture.unsigned-installer.exe'
$SignedInstallerPath = Join-Path $FixtureRoot 'AnodrelDevelopmentProductFixtureInstaller.exe'

function Assert-Elevated {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'This script adds and removes machine certificate trust. Run it from an elevated PowerShell session.'
    }
}

function Invoke-Native {
    param(
        [Parameter(Mandatory)] [string] $FilePath,
        [string[]] $Arguments = @(),
        [Parameter(Mandatory)] [string] $FailureMessage,
        [switch] $PassThroughExitCode
    )

    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $FilePath @Arguments
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previous
    }

    if ($PassThroughExitCode) {
        return $exitCode
    }
    if ($exitCode -ne 0) {
        throw $FailureMessage
    }
    return 0
}

function Build-Tools {
    param([string[]] $Packages)

    Write-Host 'Building the fixed development release tools.'
    $arguments = @('build', '--release', '--manifest-path', $CargoManifest)
    foreach ($package in $Packages) {
        $arguments += '-p'
        $arguments += $package
    }
    Invoke-Native -FilePath 'cargo' -Arguments $arguments `
        -FailureMessage 'The fixture build failed. No machine trust or installation state was changed.' | Out-Null
}

function Get-ToolPath {
    param([Parameter(Mandatory)] [string] $Name)

    $path = Join-Path $RepositoryRoot "native\target\release\$Name.exe"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "The build did not produce $Name.exe. Run the script again after resolving the build error."
    }
    return $path
}

function Get-CertificateFingerprint {
    param([Parameter(Mandatory)] $Certificate)

    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
        $bytes = $algorithm.ComputeHash($Certificate.RawData)
        return (-join ($bytes | ForEach-Object { $_.ToString('x2') }))
    }
    finally {
        $algorithm.Dispose()
    }
}

function Get-FixtureCertificate {
    $certificates = @(
        Get-ChildItem 'Cert:\CurrentUser\My' |
            Where-Object {
                $_.Subject -eq $CertificateSubject -and
                $_.HasPrivateKey -and
                $_.NotAfter -gt (Get-Date).AddDays(1)
            }
    )
    if ($certificates.Count -gt 1) {
        throw 'More than one usable development installer certificate exists. Remove the duplicate before preparing the fixture.'
    }
    if ($certificates.Count -eq 1) {
        return @{ Certificate = $certificates[0]; Created = $false }
    }

    Write-Host 'Creating the development installer code-signing certificate.'
    $certificate = New-SelfSignedCertificate `
        -Subject $CertificateSubject `
        -Type CodeSigningCert `
        -KeyUsage DigitalSignature `
        -KeyAlgorithm RSA `
        -KeyLength 3072 `
        -CertStoreLocation 'Cert:\CurrentUser\My' `
        -NotAfter (Get-Date).AddMonths(6)
    return @{ Certificate = $certificate; Created = $true }
}

function Add-MachineTrust {
    param([Parameter(Mandatory)] $Certificate)

    $fingerprint = Get-CertificateFingerprint -Certificate $Certificate
    $addedStores = @()
    try {
        foreach ($storeName in @('Root', 'TrustedPublisher')) {
            $store = New-Object Security.Cryptography.X509Certificates.X509Store($storeName, 'LocalMachine')
            $store.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
            try {
                $alreadyPresent = @(
                    $store.Certificates |
                        Where-Object { (Get-CertificateFingerprint -Certificate $_) -eq $fingerprint }
                ).Count -gt 0
                if (-not $alreadyPresent) {
                    $store.Add($Certificate)
                    $addedStores += $storeName
                    Write-Host "Installed the development certificate into LocalMachine\$storeName."
                }
            }
            finally {
                $store.Close()
            }
        }
    }
    catch {
        if ($addedStores.Count -gt 0) {
            Remove-CertificateEntries -Certificate $Certificate -Stores $addedStores
        }
        throw
    }
    return $addedStores
}

function Remove-CertificateEntries {
    param(
        [Parameter(Mandatory)] $Certificate,
        [Parameter(Mandatory)] [string[]] $Stores
    )

    $fingerprint = Get-CertificateFingerprint -Certificate $Certificate
    foreach ($storePath in $Stores) {
        Get-ChildItem $storePath |
            Where-Object { (Get-CertificateFingerprint -Certificate $_) -eq $fingerprint } |
            ForEach-Object { Remove-Item -LiteralPath $_.PSPath -Force }
    }
}

function Remove-FixtureCertificate {
    $allStores = @('Cert:\CurrentUser\My', 'Cert:\LocalMachine\Root', 'Cert:\LocalMachine\TrustedPublisher')
    $certificates = @(
        foreach ($storePath in $allStores) {
            Get-ChildItem $storePath |
                Where-Object { $_.Subject -eq $CertificateSubject }
        }
    )
    foreach ($certificate in $certificates) {
        $fingerprint = Get-CertificateFingerprint -Certificate $certificate
        foreach ($storePath in $allStores) {
            Get-ChildItem $storePath |
                Where-Object { (Get-CertificateFingerprint -Certificate $_) -eq $fingerprint } |
                ForEach-Object { Remove-Item -LiteralPath $_.PSPath -Force }
        }
    }
}

function Remove-FixtureDirectory {
    if (-not (Test-Path -LiteralPath $FixtureRoot)) {
        return
    }

    $anodrelRoot = [IO.Path]::GetFullPath((Join-Path $LocalData 'Anodrel'))
    $expectedRoot = Join-Path $anodrelRoot 'InstalledProductFixture'
    if ($FixtureRoot -ne $expectedRoot) {
        throw 'The fixed fixture output path was not resolved as expected.'
    }
    Remove-Item -LiteralPath $FixtureRoot -Recurse -Force
}

function Test-FixturePolicySelected {
    param([Parameter(Mandatory)] [string] $ProvisioningTool)

    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $ProvisioningTool 'verify' *> $null
        return $LASTEXITCODE -eq 0
    }
    finally {
        $ErrorActionPreference = $previous
    }
}

function Sign-Image {
    param(
        [Parameter(Mandatory)] [string] $Path,
        [Parameter(Mandatory)] $Certificate,
        [Parameter(Mandatory)] [string] $Label
    )

    Write-Host "Signing the fixture $Label."
    $signature = Set-AuthenticodeSignature -FilePath $Path -Certificate $Certificate -HashAlgorithm SHA256
    if ($signature.Status -ne 'Valid') {
        throw "Signing the fixture $Label did not produce a valid signature."
    }
}

function Write-ReleasePlan {
    param([Parameter(Mandatory)] [string] $PublisherFingerprint)

    $plan = @"
{
  "formatVersion": { "major": 1, "minor": 4 },
  "packageVersion": { "major": 0, "minor": 1, "patch": 0 },
  "executable": { "path": "bin/anodrel-product-fixture.exe" },
  "publisher": { "leafCertificateSha256": "$PublisherFingerprint" },
  "capabilities": ["ui.document.write", "ui.events.read", "session.close"],
  "networkOrigins": [],
  "updateCatalogue": {
    "origin": { "host": "updates.example.test", "port": 443 },
    "path": "/anodrel/development-fixture.p7s"
  },
  "product": {
    "displayName": "Anodrel Product Fixture",
    "publisherName": "Anodrel",
    "startMenuName": "Anodrel Product Fixture"
  },
  "launcher": { "path": "bin/anodrel-windows-host.exe" }
}
"@
    $encoding = New-Object Text.UTF8Encoding($false)
    [IO.File]::WriteAllText($PlanPath, $plan, $encoding)
}

if ($Remove) {
    Build-Tools -Packages @('anodrel-product-provisioning')
    $provisioningTool = Get-ToolPath -Name 'anodrel-product-provisioning'
    Assert-Elevated
    if (Test-FixturePolicySelected -ProvisioningTool $provisioningTool) {
        throw 'A valid product-fixture record is still selected. Run the matching signed installer uninstall command, or remove the staged fixture through its own script, before using -Remove.'
    }
    Remove-FixtureDirectory
    Remove-FixtureCertificate
    Write-Host 'The prepared installed product fixture has been removed.'
    return
}

$packages = @(
    'anodrel-product-fixture',
    'anodrel-product-provisioning',
    'anodrel-windows-host',
    'anodrel-windows-installer-shell',
    'anodrel-release-bundle-tool',
    'anodrel-release-manifest',
    'anodrel-release-image',
    'anodrel-release-sign'
)

Build-Tools -Packages $packages
$provisioningTool = Get-ToolPath -Name 'anodrel-product-provisioning'

if (Test-FixturePolicySelected -ProvisioningTool $provisioningTool) {
    throw 'A valid product-fixture record is already selected. Remove that fixture before preparing an initial-install acceptance run.'
}
if (Test-Path -LiteralPath $FixtureRoot) {
    throw 'The fixed fixture output directory already exists. Run this script with -Remove after confirming no fixture record is selected.'
}

Assert-Elevated

$fixtureDirectoryCreated = $false
$certificateState = $null
$addedTrustStores = @()
try {
    New-Item -ItemType Directory -Path $FixtureRoot | Out-Null
    $fixtureDirectoryCreated = $true

    $fixtureChild = Get-ToolPath -Name 'anodrel-product-fixture'
    $hostLauncher = Get-ToolPath -Name 'anodrel-windows-host'
    $installerTemplate = Get-ToolPath -Name 'anodrel-windows-installer'
    $bundleTool = Get-ToolPath -Name 'anodrel-release-bundle-tool'
    $manifestTool = Get-ToolPath -Name 'anodrel-release-manifest'
    $imageTool = Get-ToolPath -Name 'anodrel-release-image'
    $signTool = Get-ToolPath -Name 'anodrel-release-sign'

    Invoke-Native -FilePath $provisioningTool -Arguments @('stage', $PackageRoot) `
        -FailureMessage 'The fixed fixture package could not be staged.' | Out-Null
    Copy-Item -LiteralPath $fixtureChild -Destination (Join-Path $PackageRoot 'bin\anodrel-product-fixture.exe')
    Copy-Item -LiteralPath $hostLauncher -Destination (Join-Path $PackageRoot 'bin\anodrel-windows-host.exe')

    $certificateState = Get-FixtureCertificate
    $certificate = $certificateState.Certificate
    $addedTrustStores = @(Add-MachineTrust -Certificate $certificate)
    Sign-Image -Path (Join-Path $PackageRoot 'bin\anodrel-product-fixture.exe') -Certificate $certificate -Label 'child executable'
    Sign-Image -Path (Join-Path $PackageRoot 'bin\anodrel-windows-host.exe') -Certificate $certificate -Label 'host launcher'

    $fingerprint = Get-CertificateFingerprint -Certificate $certificate
    Write-ReleasePlan -PublisherFingerprint $fingerprint
    Invoke-Native -FilePath $bundleTool -Arguments @('create', $PackageRoot, $BundlePath) `
        -FailureMessage 'The fixed fixture release bundle could not be authored.' | Out-Null
    Invoke-Native -FilePath $manifestTool -Arguments @('create', $PlanPath, $BundlePath, $ManifestPath) `
        -FailureMessage 'The fixed fixture release manifest could not be derived.' | Out-Null
    Invoke-Native -FilePath $imageTool -Arguments @('embed', $installerTemplate, $ManifestPath, $BundlePath, $UnsignedInstallerPath) `
        -FailureMessage 'The fixed unsigned fixture installer could not be assembled.' | Out-Null
    Invoke-Native -FilePath $signTool -Arguments @('sign', $UnsignedInstallerPath, $fingerprint, $SignedInstallerPath) `
        -FailureMessage 'The fixed fixture installer could not be signed and verified.' | Out-Null
    Invoke-Native -FilePath $SignedInstallerPath -Arguments @('verify') `
        -FailureMessage 'The signed fixture installer did not pass its read-only verification.' | Out-Null
}
catch {
    if ($fixtureDirectoryCreated) {
        Remove-FixtureDirectory
    }
    if ($null -ne $certificateState) {
        if ($addedTrustStores.Count -gt 0) {
            Remove-CertificateEntries -Certificate $certificateState.Certificate -Stores $addedTrustStores
        }
        if ($certificateState.Created) {
            Remove-CertificateEntries -Certificate $certificateState.Certificate -Stores @('Cert:\CurrentUser\My')
        }
    }
    throw
}

Write-Host ''
Write-Host 'The signed development installer fixture is prepared and has passed read-only verification.'
Write-Host 'Start this signed installer normally to exercise native consent and the fixed UAC handoff:'
Write-Host "  & `"$SignedInstallerPath`""
Write-Host ''
Write-Host 'After installation, launch “Anodrel Product Fixture” from the Start menu, use its action, and confirm that it closes.'
Write-Host 'To remove it, first run the same signed installer with “uninstall” from an elevated PowerShell session, then run this script with -Remove.'
