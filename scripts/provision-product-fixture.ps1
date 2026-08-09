<#
.SYNOPSIS
Provisions or removes the development-only Anodrel Windows product fixture.

.DESCRIPTION
This script exists so the verified product-session path — machine policy, locked
digest revalidation, Authenticode publisher match, private bootstrap delivery,
authenticated pipe, and a host-owned native window — can be exercised end to end
on a development machine.

It uses Windows tooling only: Cargo for the build, New-SelfSignedCertificate for
the development certificate, Set-AuthenticodeSignature for signing, the machine
certificate stores for trust, and the first-party
anodrel-product-provisioning helper for the machine-policy record.

IT INSTALLS A LOCALLY GENERATED CODE-SIGNING CERTIFICATE INTO THE MACHINE ROOT
AND TRUSTED PUBLISHER STORES. That is a real machine trust change. Run this only
on a development machine, and run -Remove when you are finished.

Provisioning and removal both need an elevated PowerShell session.

.PARAMETER StagingRoot
Where the fixture package is staged. Defaults to a per-user location outside the
repository so generated output never lands in tracked source directories.

.PARAMETER Remove
Removes the machine-policy record, the staged package, and the development
certificate from both machine stores.

.EXAMPLE
PS> .\scripts\provision-product-fixture.ps1

.EXAMPLE
PS> .\scripts\provision-product-fixture.ps1 -Remove
#>

[CmdletBinding()]
param(
    [string] $StagingRoot = (Join-Path $env:LOCALAPPDATA 'Anodrel\ProductFixture'),
    [switch] $Remove
)

$ErrorActionPreference = 'Stop'

$CertificateSubject = 'CN=Anodrel Development Product Fixture'
$RepositoryRoot = Split-Path -Parent $PSScriptRoot
$Manifest = Join-Path $RepositoryRoot 'native\Cargo.toml'

function Assert-Elevated {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'This script changes machine policy and machine certificate trust. Run it from an elevated PowerShell session.'
    }
}

function Get-ToolPath {
    param([string] $Name)
    $path = Join-Path $RepositoryRoot "native\target\release\$Name.exe"
    if (-not (Test-Path $path)) {
        throw "The build did not produce $Name.exe. Run the script again after resolving the Cargo error above."
    }
    return $path
}

function Remove-FixtureCertificates {
    # The private key lives only in the machine personal store. Removing all
    # three entries leaves no trust behind.
    foreach ($store in @('My', 'Root', 'TrustedPublisher')) {
        Get-ChildItem "Cert:\LocalMachine\$store" |
            Where-Object { $_.Subject -eq $CertificateSubject } |
            ForEach-Object {
                Write-Host "Removing the development certificate from LocalMachine\$store."
                Remove-Item $_.PSPath -Force
            }
    }
}

Assert-Elevated

if ($Remove) {
    Write-Host 'Removing the Anodrel development product fixture.'

    $helper = Join-Path $RepositoryRoot 'native\target\release\anodrel-product-provisioning.exe'
    if (Test-Path $helper) {
        & $helper remove
        if ($LASTEXITCODE -ne 0) {
            throw 'The machine-policy record could not be removed.'
        }
    }
    else {
        Write-Warning 'The provisioning helper is not built; the machine-policy record was left in place. Build it and re-run -Remove.'
    }

    if (Test-Path $StagingRoot) {
        Write-Host "Removing the staged package at $StagingRoot."
        Remove-Item $StagingRoot -Recurse -Force
    }

    Remove-FixtureCertificates
    Write-Host 'The development product fixture has been removed.'
    return
}

Write-Host 'Building the fixture and the provisioning helper in release.'
& cargo build --release --manifest-path $Manifest -p anodrel-product-fixture -p anodrel-product-provisioning
if ($LASTEXITCODE -ne 0) {
    throw 'The fixture build failed. Nothing was provisioned.'
}

$fixtureBinary = Get-ToolPath -Name 'anodrel-product-fixture'
$helper = Get-ToolPath -Name 'anodrel-product-provisioning'

Write-Host "Staging the fixture package at $StagingRoot."
& $helper stage $StagingRoot
if ($LASTEXITCODE -ne 0) {
    throw 'The fixture package could not be staged. Nothing was provisioned.'
}

$stagedExecutable = Join-Path $StagingRoot 'bin\anodrel-product-fixture.exe'
Copy-Item $fixtureBinary $stagedExecutable -Force

$certificate = Get-ChildItem Cert:\LocalMachine\My |
    Where-Object { $_.Subject -eq $CertificateSubject -and $_.NotAfter -gt (Get-Date) } |
    Select-Object -First 1

if ($null -eq $certificate) {
    Write-Host 'Creating a development code-signing certificate.'
    $certificate = New-SelfSignedCertificate `
        -Subject $CertificateSubject `
        -Type CodeSigningCert `
        -KeyUsage DigitalSignature `
        -KeyAlgorithm RSA `
        -KeyLength 3072 `
        -CertStoreLocation 'Cert:\LocalMachine\My' `
        -NotAfter (Get-Date).AddMonths(6)

    # Authenticode accepts a signature only when its chain is trusted, so the
    # generated certificate is installed as its own root and publisher. This is
    # the fixture's cost and is reversed by -Remove.
    foreach ($store in @('Root', 'TrustedPublisher')) {
        $target = New-Object System.Security.Cryptography.X509Certificates.X509Store($store, 'LocalMachine')
        $target.Open('ReadWrite')
        $target.Add($certificate)
        $target.Close()
        Write-Host "Installed the development certificate into LocalMachine\$store."
    }
}
else {
    Write-Host 'Reusing the existing development code-signing certificate.'
}

Write-Host 'Signing the staged fixture executable.'
$signature = Set-AuthenticodeSignature -FilePath $stagedExecutable -Certificate $certificate -HashAlgorithm SHA256
if ($signature.Status -ne 'Valid') {
    throw "Signing did not produce a valid signature (status: $($signature.Status)). Nothing was provisioned."
}

Write-Host 'Writing the machine-policy record.'
& $helper provision $StagingRoot
if ($LASTEXITCODE -ne 0) {
    throw 'The machine-policy record was not written.'
}

Write-Host ''
Write-Host 'The development product fixture is provisioned.'
Write-Host 'Run the host route:'
Write-Host '  cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --product-session org.anodrel.product-fixture'
Write-Host ''
Write-Host 'Run this script with -Remove when you are finished.'
