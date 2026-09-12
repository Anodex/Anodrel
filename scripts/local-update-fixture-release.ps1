<#
.SYNOPSIS
Internal owned-release assembly helpers for the local update fixture.

.DESCRIPTION
Dot-sourced only by prepare-local-update-fixture.ps1. The caller supplies two
fixed private release roots and uses this module to assemble one fresh signed
release for each declared fixture version. It does not install, elevate, or
publish an update.
#>

function Invoke-LocalUpdateFixtureNative {
    param(
        [Parameter(Mandatory)] [string] $FilePath,
        [Parameter(Mandatory)] [string[]] $Arguments,
        [Parameter(Mandatory)] [string] $FailureMessage
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
    if ($exitCode -ne 0) {
        throw $FailureMessage
    }
}

function Write-LocalUpdateFixturePlan {
    param(
        [Parameter(Mandatory)] [string] $Path,
        [Parameter(Mandatory)] [string] $Version,
        [Parameter(Mandatory)] [string] $PublisherFingerprint
    )

    $plan = @"
{
  "formatVersion": { "major": 1, "minor": 4 },
  "packageVersion": $Version,
  "executable": { "path": "bin/anodrel-product-fixture.exe" },
  "publisher": { "leafCertificateSha256": "$PublisherFingerprint" },
  "capabilities": ["ui.document.write", "ui.events.read", "session.close"],
  "networkOrigins": [],
  "updateCatalogue": {
    "origin": { "host": "localhost", "port": 45863 },
    "path": "/anodrel/local-update/stable.p7s"
  },
  "product": {
    "displayName": "Anodrel Local Update Fixture",
    "publisherName": "Anodrel",
    "startMenuName": "Anodrel Local Update Fixture"
  },
  "launcher": { "path": "bin/anodrel-windows-host.exe" }
}
"@
    [IO.File]::WriteAllText($Path, $plan, (New-Object Text.UTF8Encoding($false)))
}

function Sign-LocalUpdateFixtureImage {
    param(
        [Parameter(Mandatory)] [string] $Path,
        [Parameter(Mandatory)] $Certificate,
        [Parameter(Mandatory)] [string] $Label
    )

    $signature = Set-AuthenticodeSignature -FilePath $Path -Certificate $Certificate -HashAlgorithm SHA256
    if ($signature.Status -ne 'Valid') {
        throw "Signing the local update fixture $Label did not produce a valid signature."
    }
}

function New-LocalUpdateFixtureRelease {
    param(
        [Parameter(Mandatory)] [hashtable] $Tools,
        [Parameter(Mandatory)] [string] $ReleaseRoot,
        [Parameter(Mandatory)] [string] $Version,
        [Parameter(Mandatory)] [string] $PublisherFingerprint,
        [Parameter(Mandatory)] $PublisherCertificate,
        [Parameter(Mandatory)] [string] $InstallerOutput
    )

    $packageRoot = Join-Path $ReleaseRoot 'package'
    $planPath = Join-Path $ReleaseRoot 'release-plan.json'
    $bundlePath = Join-Path $ReleaseRoot 'release.bundle'
    $manifestPath = Join-Path $ReleaseRoot 'release.manifest.json'
    $unsignedPath = Join-Path $ReleaseRoot 'release.unsigned.exe'
    New-Item -ItemType Directory -Path $ReleaseRoot -ErrorAction Stop | Out-Null

    Invoke-LocalUpdateFixtureNative -FilePath $Tools.Provisioning -Arguments @('stage-local-update', $packageRoot) `
        -FailureMessage 'The local update fixture package could not be staged.'
    Copy-Item -LiteralPath $Tools.Child -Destination (Join-Path $packageRoot 'bin\anodrel-product-fixture.exe')
    Copy-Item -LiteralPath $Tools.Launcher -Destination (Join-Path $packageRoot 'bin\anodrel-windows-host.exe')
    Sign-LocalUpdateFixtureImage -Path (Join-Path $packageRoot 'bin\anodrel-product-fixture.exe') `
        -Certificate $PublisherCertificate -Label 'child executable'
    Sign-LocalUpdateFixtureImage -Path (Join-Path $packageRoot 'bin\anodrel-windows-host.exe') `
        -Certificate $PublisherCertificate -Label 'host launcher'

    Write-LocalUpdateFixturePlan -Path $planPath -Version $Version -PublisherFingerprint $PublisherFingerprint
    Invoke-LocalUpdateFixtureNative -FilePath $Tools.Bundle -Arguments @('create', $packageRoot, $bundlePath) `
        -FailureMessage 'The local update fixture bundle could not be authored.'
    Invoke-LocalUpdateFixtureNative -FilePath $Tools.Manifest -Arguments @('create', $planPath, $bundlePath, $manifestPath) `
        -FailureMessage 'The local update fixture manifest could not be derived.'
    Invoke-LocalUpdateFixtureNative -FilePath $Tools.Image -Arguments @('embed', $Tools.Installer, $manifestPath, $bundlePath, $unsignedPath) `
        -FailureMessage 'The local update fixture image could not be assembled.'
    Invoke-LocalUpdateFixtureNative -FilePath $Tools.Sign -Arguments @('sign', $unsignedPath, $PublisherFingerprint, $InstallerOutput) `
        -FailureMessage 'The local update fixture installer could not be signed and verified.'
    Invoke-LocalUpdateFixtureNative -FilePath $InstallerOutput -Arguments @('verify') `
        -FailureMessage 'The local update fixture installer did not pass read-only verification.'
}
