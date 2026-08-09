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
certificate from both machine stores. Needs elevation.

.PARAMETER Verify
Reports whether the machine record currently validates and changes nothing.
This is a query only, so it does not need elevation.

.EXAMPLE
PS> .\scripts\provision-product-fixture.ps1

.EXAMPLE
PS> .\scripts\provision-product-fixture.ps1 -Verify

.EXAMPLE
PS> .\scripts\provision-product-fixture.ps1 -Remove
#>

[CmdletBinding()]
param(
    [string] $StagingRoot = (Join-Path $env:LOCALAPPDATA 'Anodrel\ProductFixture'),
    [switch] $Remove,
    [switch] $Verify
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

<#
Runs an external program and decides success from its exit code alone.

Cargo writes its progress to standard error, and the provisioning helper writes
its safe failure categories there too. Under the script-wide 'Stop' preference
Windows PowerShell turns any such line into a terminating error, so calling
these programs directly would abort on ordinary output. Exit codes are the
contract both programs actually document, so this relaxes the preference for the
call and then restores it.
#>
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
    Write-Host 'Building in release.'
    $arguments = @('build', '--release', '--manifest-path', $Manifest)
    $arguments += @($Packages | ForEach-Object { '-p'; $_ })
    Invoke-Native -FilePath 'cargo' -Arguments $arguments `
        -FailureMessage 'The fixture build failed. Nothing was changed.' | Out-Null
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

if ($Remove -and $Verify) {
    throw 'Choose either -Remove or -Verify, not both.'
}

if ($Verify) {
    # A query only: it opens the machine policy key for reading and reports a
    # safe category. Nothing here needs elevation or changes machine state.
    Build-Tools -Packages @('anodrel-product-provisioning')
    # A record that does not validate is an answer, not a script failure, so the
    # helper's exit code is passed through unchanged.
    exit (Invoke-Native -FilePath (Get-ToolPath -Name 'anodrel-product-provisioning') `
            -Arguments @('verify') -FailureMessage 'unused' -PassThroughExitCode)
}

Assert-Elevated

if ($Remove) {
    Write-Host 'Removing the Anodrel development product fixture.'

    # Removal builds the helper if needed: a checkout that was cleaned since
    # provisioning must still be able to take the record back out.
    Build-Tools -Packages @('anodrel-product-provisioning')
    Invoke-Native -FilePath (Get-ToolPath -Name 'anodrel-product-provisioning') `
        -Arguments @('remove') `
        -FailureMessage 'The machine-policy record could not be removed.' | Out-Null

    if (Test-Path $StagingRoot) {
        Write-Host "Removing the staged package at $StagingRoot."
        Remove-Item $StagingRoot -Recurse -Force
    }

    Remove-FixtureCertificates
    Write-Host 'The development product fixture has been removed.'
    return
}

Build-Tools -Packages @('anodrel-product-fixture', 'anodrel-product-provisioning')

$fixtureBinary = Get-ToolPath -Name 'anodrel-product-fixture'
$helper = Get-ToolPath -Name 'anodrel-product-provisioning'

Write-Host "Staging the fixture package at $StagingRoot."
Invoke-Native -FilePath $helper -Arguments @('stage', $StagingRoot) `
    -FailureMessage 'The fixture package could not be staged. Nothing was provisioned.' | Out-Null

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
Invoke-Native -FilePath $helper -Arguments @('provision', $StagingRoot) `
    -FailureMessage 'The machine-policy record was not written.' | Out-Null

Write-Host ''
Write-Host 'The development product fixture is provisioned.'
Write-Host 'Run the host route:'
Write-Host '  cargo run --release --manifest-path native/Cargo.toml -p anodrel-windows-host -- --product-session org.anodrel.product-fixture'
Write-Host ''
Write-Host 'Check the state at any time with -Verify; no elevation is needed for that.'
Write-Host 'Run this script with -Remove when you are finished.'
