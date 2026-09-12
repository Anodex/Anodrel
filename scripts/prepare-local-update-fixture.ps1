<#
.SYNOPSIS
Prepares the isolated signed local-update acceptance fixture on Windows.

.DESCRIPTION
Builds fixed 0.1.0 and 0.1.1 signed release images, a CMS catalogue, and the
temporary localhost HTTPS configuration required by the direct Anodrel
fixture server. It never installs a release or starts the server. All identity,
path, hostname, port, version, and certificate names are constants.
#>

[CmdletBinding()]
param([switch] $Remove)

$ErrorActionPreference = 'Stop'
$RepositoryRoot = Split-Path -Parent $PSScriptRoot
$CargoManifest = Join-Path $RepositoryRoot 'native\Cargo.toml'
$LocalData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
$FixtureRoot = [IO.Path]::GetFullPath((Join-Path $LocalData 'Anodrel\LocalUpdateFixture'))
$InitialInstaller = Join-Path $FixtureRoot 'AnodrelDevelopmentLocalUpdateFixtureInstaller.exe'
$PublicationRoot = Join-Path $FixtureRoot 'publication'
$CandidateInstaller = Join-Path $PublicationRoot 'releases\0.1.1\installer.exe'
$CatalogueJson = Join-Path $FixtureRoot 'candidate\catalogue.json'
$CatalogueSignature = Join-Path $PublicationRoot 'stable.p7s'
$ApplicationId = 'org.anodrel.local-update-fixture'
$CertificateSubject = 'CN=Anodrel Development Local Update Fixture'
$TlsSubject = 'CN=localhost'
$TlsFriendlyName = 'Anodrel Local Update Fixture TLS'
$CertificateProvider = 'Microsoft Enhanced RSA and AES Cryptographic Provider'
$ProgramFiles = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
$InstalledRoot = Join-Path $ProgramFiles "Anodrel\Applications\$ApplicationId"
$PolicyPath = "HKLM:\Software\Anodrel\Applications\$ApplicationId"

. (Join-Path $PSScriptRoot 'local-update-fixture-http.ps1')
. (Join-Path $PSScriptRoot 'local-update-fixture-release.ps1')

function Assert-Elevated {
    $principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'This fixture changes temporary machine trust and HTTP Server configuration. Run it from an elevated PowerShell session.'
    }
}

function Invoke-FixtureBuild {
    param([Parameter(Mandatory)] [string[]] $Packages)

    $arguments = @('build', '--release', '--manifest-path', $CargoManifest)
    foreach ($package in $Packages) {
        $arguments += '-p'
        $arguments += $package
    }
    Invoke-LocalUpdateFixtureNative -FilePath 'cargo' -Arguments $arguments `
        -FailureMessage 'The local update fixture tools could not be built.'
}

function Get-FixtureTool {
    param([Parameter(Mandatory)] [string] $Name)

    $path = Join-Path $RepositoryRoot "native\target\release\$Name.exe"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "The local update fixture build did not produce $Name.exe."
    }
    return $path
}

function Get-CertificateFingerprint {
    param([Parameter(Mandatory)] $Certificate)

    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
        return -join ($algorithm.ComputeHash($Certificate.RawData) | ForEach-Object { $_.ToString('x2') })
    }
    finally {
        $algorithm.Dispose()
    }
}

function Find-PublisherCertificate {
    $certificates = @(Get-ChildItem 'Cert:\CurrentUser\My' | Where-Object {
        $_.Subject -eq $CertificateSubject -and $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date).AddDays(1)
    })
    if ($certificates.Count -gt 1) {
        throw 'More than one local update fixture publisher certificate exists. It was not selected.'
    }
    if ($certificates.Count -eq 1) {
        return @{ Certificate = $certificates[0]; Created = $false }
    }
    $certificate = New-SelfSignedCertificate -Subject $CertificateSubject -Type CodeSigningCert `
        -KeyUsage DigitalSignature -KeyAlgorithm RSA -KeyLength 3072 -Provider $CertificateProvider `
        -KeySpec Signature -CertStoreLocation 'Cert:\CurrentUser\My' -NotAfter (Get-Date).AddMonths(6)
    return @{ Certificate = $certificate; Created = $true }
}

function Find-TlsCertificate {
    $certificates = @(Get-ChildItem 'Cert:\LocalMachine\My' | Where-Object {
        $_.Subject -eq $TlsSubject -and $_.FriendlyName -eq $TlsFriendlyName -and $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date).AddDays(1)
    })
    if ($certificates.Count -gt 1) {
        throw 'More than one local update fixture TLS certificate exists. It was not selected.'
    }
    if ($certificates.Count -eq 1) {
        return @{ Certificate = $certificates[0]; Created = $false }
    }
    $certificate = New-SelfSignedCertificate -Subject $TlsSubject -DnsName 'localhost' `
        -Type SSLServerAuthentication -KeyAlgorithm RSA -KeyLength 3072 -Provider $CertificateProvider `
        -KeySpec KeyExchange -FriendlyName $TlsFriendlyName -CertStoreLocation 'Cert:\LocalMachine\My' `
        -NotAfter (Get-Date).AddMonths(6)
    return @{ Certificate = $certificate; Created = $true }
}

function Add-CertificateTrust {
    param(
        [Parameter(Mandatory)] $Certificate,
        [Parameter(Mandatory)] [string[]] $StoreNames
    )

    $fingerprint = Get-CertificateFingerprint -Certificate $Certificate
    $added = @()
    foreach ($storeName in $StoreNames) {
        $store = New-Object Security.Cryptography.X509Certificates.X509Store($storeName, 'LocalMachine')
        $store.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
        try {
            $present = @($store.Certificates | Where-Object {
                (Get-CertificateFingerprint -Certificate $_) -eq $fingerprint
            }).Count -ne 0
            if (-not $present) {
                $store.Add($Certificate)
                $added += "Cert:\LocalMachine\$storeName"
            }
        }
        finally {
            $store.Close()
        }
    }
    return $added
}

function Remove-CertificateTrust {
    param(
        [Parameter(Mandatory)] $Certificate,
        [Parameter(Mandatory)] [string[]] $Stores
    )

    $fingerprint = Get-CertificateFingerprint -Certificate $Certificate
    foreach ($storePath in $Stores) {
        Get-ChildItem $storePath | Where-Object {
            (Get-CertificateFingerprint -Certificate $_) -eq $fingerprint
        } | ForEach-Object { Remove-Item -LiteralPath $_.PSPath -Force }
    }
}

function Assert-FixturePolicyAbsent {
    if ((Test-Path -LiteralPath $PolicyPath) -and $null -ne (Get-ItemProperty -LiteralPath $PolicyPath).PSObject.Properties['record']) {
        throw 'The local update fixture is still selected. Remove it with its signed uninstaller before changing fixture trust.'
    }
}

function Retire-FixtureCleanupCache {
    if (-not (Test-Path -LiteralPath $InstalledRoot)) { return }
    if (-not (Test-Path -LiteralPath $CandidateInstaller -PathType Leaf)) {
        throw 'The local update fixture cleanup cache remains but its prepared signed candidate is unavailable.'
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $CandidateInstaller
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -ne $CertificateSubject) {
        throw 'The prepared local update fixture candidate signature is invalid. Fixture trust was not removed.'
    }
    Invoke-LocalUpdateFixtureNative -FilePath $CandidateInstaller -Arguments @('cleanup-cache') `
        -FailureMessage 'The local update fixture cleanup cache could not be retired. Close every Anodrel removal-result dialog and retry.'
    if (Test-Path -LiteralPath $InstalledRoot) {
        throw 'The local update fixture package or cleanup cache remains after signed retirement. Fixture trust was not removed.'
    }
}

function Remove-FixtureOutput {
    if (-not (Test-Path -LiteralPath $FixtureRoot)) { return }
    $expected = [IO.Path]::GetFullPath((Join-Path $LocalData 'Anodrel\LocalUpdateFixture'))
    $item = Get-Item -LiteralPath $FixtureRoot -Force
    if ($FixtureRoot -ne $expected -or -not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'The fixed local update fixture output directory is unsafe. It was not removed.'
    }
    $nestedReparsePoint = Get-ChildItem -LiteralPath $FixtureRoot -Force -Recurse |
        Where-Object { ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 } |
        Select-Object -First 1
    if ($null -ne $nestedReparsePoint) {
        throw 'The fixed local update fixture output directory contains a reparse point. It was not removed.'
    }
    Remove-Item -LiteralPath $FixtureRoot -Recurse -Force
}

function Remove-FixtureCertificate {
    param(
        [Parameter(Mandatory)] $Certificate,
        [Parameter(Mandatory)] [string[]] $Stores,
        [Parameter(Mandatory)] [string] $SourceStore
    )

    Remove-CertificateTrust -Certificate $Certificate -Stores $Stores
    Remove-CertificateTrust -Certificate $Certificate -Stores @($SourceStore)
}

Assert-Elevated

if ($Remove) {
    Assert-FixturePolicyAbsent
    Retire-FixtureCleanupCache
    $tls = @(Get-ChildItem 'Cert:\LocalMachine\My' | Where-Object {
        $_.Subject -eq $TlsSubject -and $_.FriendlyName -eq $TlsFriendlyName
    })
    if ($tls.Count -gt 1) { throw 'More than one local update fixture TLS certificate exists. Nothing was removed.' }
    if ($tls.Count -eq 1) { Remove-FixtureHttpEndpoint -CertificateFingerprint (Get-CertificateFingerprint -Certificate $tls[0]) }
    Remove-FixtureOutput
    if ($tls.Count -eq 1) {
        Remove-FixtureCertificate -Certificate $tls[0] -Stores @('Cert:\LocalMachine\Root') -SourceStore 'Cert:\LocalMachine\My'
    }
    $publishers = @(Get-ChildItem 'Cert:\CurrentUser\My' | Where-Object { $_.Subject -eq $CertificateSubject })
    if ($publishers.Count -gt 1) { throw 'More than one local update fixture publisher certificate exists. Nothing was removed.' }
    if ($publishers.Count -eq 1) {
        Remove-FixtureCertificate -Certificate $publishers[0] -Stores @('Cert:\LocalMachine\Root', 'Cert:\LocalMachine\TrustedPublisher') -SourceStore 'Cert:\CurrentUser\My'
    }
    Write-Host 'The prepared local signed update fixture has been removed.'
    return
}

Assert-FixturePolicyAbsent
if (Test-Path -LiteralPath $InstalledRoot) {
    throw 'The local update fixture package or cleanup cache remains. Complete its signed removal before preparing another fixture.'
}
if (Test-Path -LiteralPath $FixtureRoot) {
    throw 'The fixed local update fixture output directory already exists. Run this script with -Remove after uninstalling the fixture.'
}

$packages = @(
    'anodrel-product-fixture', 'anodrel-product-provisioning', 'anodrel-windows-host',
    'anodrel-windows-installer-shell', 'anodrel-windows-installer', 'anodrel-release-bundle-tool',
    'anodrel-release-manifest', 'anodrel-release-image', 'anodrel-release-sign',
    'anodrel-update-catalogue-create', 'anodrel-update-catalogue-sign',
    'anodrel-local-update-fixture-server', 'anodrel-product-update-acceptance'
)
Invoke-FixtureBuild -Packages $packages

$publisherState = $null
$tlsState = $null
$publisherTrust = @()
$tlsTrust = @()
$endpointConfigured = $false
try {
    $tools = @{
        Provisioning = Get-FixtureTool -Name 'anodrel-product-provisioning'
        Child = Get-FixtureTool -Name 'anodrel-product-fixture'
        Launcher = Get-FixtureTool -Name 'anodrel-windows-host'
        Installer = Get-FixtureTool -Name 'anodrel-windows-installer'
        Bundle = Get-FixtureTool -Name 'anodrel-release-bundle-tool'
        Manifest = Get-FixtureTool -Name 'anodrel-release-manifest'
        Image = Get-FixtureTool -Name 'anodrel-release-image'
        Sign = Get-FixtureTool -Name 'anodrel-release-sign'
    }
    $publisherState = Find-PublisherCertificate
    $publisher = $publisherState.Certificate
    $publisherTrust = @(Add-CertificateTrust -Certificate $publisher -StoreNames @('Root', 'TrustedPublisher'))
    $publisherFingerprint = Get-CertificateFingerprint -Certificate $publisher
    New-Item -ItemType Directory -Path (Join-Path $PublicationRoot 'releases\0.1.1') -Force | Out-Null
    New-LocalUpdateFixtureRelease -Tools $tools -ReleaseRoot (Join-Path $FixtureRoot 'initial') `
        -Version '{ "major": 0, "minor": 1, "patch": 0 }' -PublisherFingerprint $publisherFingerprint `
        -PublisherCertificate $publisher -InstallerOutput $InitialInstaller
    New-LocalUpdateFixtureRelease -Tools $tools -ReleaseRoot (Join-Path $FixtureRoot 'candidate') `
        -Version '{ "major": 0, "minor": 1, "patch": 1 }' -PublisherFingerprint $publisherFingerprint `
        -PublisherCertificate $publisher -InstallerOutput $CandidateInstaller
    Invoke-LocalUpdateFixtureNative -FilePath (Get-FixtureTool -Name 'anodrel-update-catalogue-create') `
        -Arguments @('create', $CandidateInstaller, 'localhost', '45863', '/anodrel/local-update/releases/0.1.1/installer.exe', $CatalogueJson) `
        -FailureMessage 'The local update fixture catalogue could not be derived.'
    Invoke-LocalUpdateFixtureNative -FilePath (Get-FixtureTool -Name 'anodrel-update-catalogue-sign') `
        -Arguments @('sign', $CatalogueJson, $publisherFingerprint, $CatalogueSignature) `
        -FailureMessage 'The local update fixture catalogue could not be signed.'
    $tlsState = Find-TlsCertificate
    $tlsTrust = @(Add-CertificateTrust -Certificate $tlsState.Certificate -StoreNames @('Root'))
    Add-FixtureHttpEndpoint -CertificateFingerprint (Get-CertificateFingerprint -Certificate $tlsState.Certificate) `
        -AccountName ([Security.Principal.WindowsIdentity]::GetCurrent().Name)
    $endpointConfigured = $true
}
catch {
    $preparationError = $_
    if ($endpointConfigured) {
        try { Remove-FixtureHttpEndpoint -CertificateFingerprint (Get-CertificateFingerprint -Certificate $tlsState.Certificate) } catch { Write-Warning 'The failed preparation left its local HTTPS configuration in place.' }
    }
    try { Remove-FixtureOutput } catch { Write-Warning 'The failed preparation left its local files for inspection.' }
    if ($tlsTrust.Count -gt 0) { try { Remove-CertificateTrust -Certificate $tlsState.Certificate -Stores $tlsTrust } catch { Write-Warning 'The failed preparation could not remove every TLS trust entry it created.' } }
    if ($publisherTrust.Count -gt 0) { try { Remove-CertificateTrust -Certificate $publisherState.Certificate -Stores $publisherTrust } catch { Write-Warning 'The failed preparation could not remove every publisher trust entry it created.' } }
    if ($null -ne $tlsState -and $tlsState.Created) { try { Remove-Item -LiteralPath $tlsState.Certificate.PSPath -Force } catch { Write-Warning 'The failed preparation could not remove its TLS certificate.' } }
    if ($null -ne $publisherState -and $publisherState.Created) { try { Remove-Item -LiteralPath $publisherState.Certificate.PSPath -Force } catch { Write-Warning 'The failed preparation could not remove its publisher certificate.' } }
    throw $preparationError
}

Write-Host 'The local signed update fixture is prepared and passed read-only release verification.'
Write-Host 'Install the fixed 0.1.0 release normally:'
Write-Host "  & `"$InitialInstaller`""
Write-Host 'Then run the fixed local server in a second normal PowerShell window and invoke the no-argument local update acceptance command.'
