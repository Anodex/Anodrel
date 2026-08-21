<#
.SYNOPSIS
Focused verification for scripts/new-application.ps1.

.DESCRIPTION
Creates one package in a unique temporary directory, verifies its exact content
bytes and manifest digest, confirms overwrite and invalid-identity refusal, and
removes only that temporary directory.
#>

[CmdletBinding()]
param(
    [switch] $VerifyHost
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$generator = Join-Path $repositoryRoot 'scripts\new-application.ps1'
$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) "anodrel-package-template-$([guid]::NewGuid().ToString('N'))"
$destination = Join-Path (Join-Path $testRoot 'generated') 'starter'
$invalidIdDestination = Join-Path $testRoot 'invalid-id'
$invalidDisplayDestination = Join-Path $testRoot 'invalid-display'
$invalidContentDestination = Join-Path $testRoot 'invalid-content'

function Get-Sha256LowerHex {
    param([Parameter(Mandatory)] [byte[]] $Bytes)

    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
        return (($algorithm.ComputeHash($Bytes) | ForEach-Object { $_.ToString('x2') }) -join '')
    }
    finally {
        $algorithm.Dispose()
    }
}

function Assert-Rejected {
    param(
        [Parameter(Mandatory)] [scriptblock] $Action,
        [Parameter(Mandatory)] [string] $Description
    )

    $rejected = $false
    try {
        & $Action
    }
    catch {
        $rejected = $true
    }
    if (-not $rejected) {
        throw "Expected rejection: $Description"
    }
}

try {
    New-Item -ItemType Directory -Path $testRoot -ErrorAction Stop | Out-Null
    & $generator `
        -Destination $destination `
        -ApplicationId 'org.example.starter' `
        -DisplayName 'Starter package' `
        -Content "First line`r`nSecond line"

    $contentPath = Join-Path $destination 'content\main.txt'
    $manifestPath = Join-Path $destination 'anodrel.application.json'
    $attributesPath = Join-Path $destination '.gitattributes'
    $contentBytes = [System.IO.File]::ReadAllBytes($contentPath)
    $expectedContent = [System.Text.Encoding]::UTF8.GetBytes("First line`nSecond line")
    if ([Convert]::ToBase64String($contentBytes) -cne [Convert]::ToBase64String($expectedContent)) {
        throw 'Generated content bytes were not LF-normalised UTF-8.'
    }
    if ($contentBytes.Length -ge 3 -and $contentBytes[0] -eq 0xEF -and $contentBytes[1] -eq 0xBB -and $contentBytes[2] -eq 0xBF) {
        throw 'Generated content includes a UTF-8 byte-order mark.'
    }
    $attributesBytes = [System.IO.File]::ReadAllBytes($attributesPath)
    $expectedAttributes = [System.Text.Encoding]::UTF8.GetBytes("content/main.txt -text`n")
    if ([Convert]::ToBase64String($attributesBytes) -cne [Convert]::ToBase64String($expectedAttributes)) {
        throw 'Generated Git attributes do not preserve the content bytes.'
    }

    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($manifest.manifestVersion.major -ne 1 -or $manifest.manifestVersion.minor -ne 0) {
        throw 'Generated manifest version is incorrect.'
    }
    if ($manifest.applicationId -cne 'org.example.starter' -or $manifest.displayName -cne 'Starter package') {
        throw 'Generated manifest identity is incorrect.'
    }
    if ($manifest.content.format -cne 'anodrel.text.v1' -or $manifest.content.path -cne 'content/main.txt') {
        throw 'Generated manifest content declaration is incorrect.'
    }
    if ($manifest.content.sha256 -cne (Get-Sha256LowerHex -Bytes $contentBytes)) {
        throw 'Generated manifest digest does not match the content bytes.'
    }

    Assert-Rejected -Description 'an existing destination' -Action {
        & $generator -Destination $destination -ApplicationId 'org.example.starter' -DisplayName 'Starter package'
    }
    Assert-Rejected -Description 'an invalid application ID' -Action {
        & $generator -Destination $invalidIdDestination -ApplicationId 'Invalid.Id' -DisplayName 'Invalid package'
    }
    Assert-Rejected -Description 'a display name with a control character' -Action {
        & $generator -Destination $invalidDisplayDestination -ApplicationId 'org.example.invalid' -DisplayName "Invalid`tpackage"
    }
    Assert-Rejected -Description 'content over the line limit' -Action {
        & $generator -Destination $invalidContentDestination -ApplicationId 'org.example.invalid' -DisplayName 'Invalid package' -Content ('x' * 161)
    }
    foreach ($invalidDestination in @($invalidIdDestination, $invalidDisplayDestination, $invalidContentDestination)) {
        if (Test-Path -LiteralPath $invalidDestination) {
            throw 'Invalid input created a destination directory.'
        }
    }

    if ($VerifyHost) {
        $nativeManifest = Join-Path $repositoryRoot 'native\Cargo.toml'
        $previousPreference = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try {
            & cargo run --release --manifest-path $nativeManifest -p anodrel-windows-host -- --startup-report $manifestPath
            $exitCode = $LASTEXITCODE
        }
        finally {
            $ErrorActionPreference = $previousPreference
        }
        if ($exitCode -ne 0) {
            throw 'The native host rejected the generated package.'
        }
    }

    Write-Output 'new-application template check passed.'
}
finally {
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
