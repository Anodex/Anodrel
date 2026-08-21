<#
.SYNOPSIS
Creates one strict, digest-verified Anodrel text application package.

.DESCRIPTION
Creates only the current anodrel.text.v1 content package: a strict manifest
and a plain-text document that the Windows host verifies and renders directly.
It does not start a process, run the host, sign content, grant capabilities,
write an installed application record, or change machine state.

The destination must not exist. All inputs are validated before the package
directory is created. See docs/APPLICATION_TEMPLATE.md and Decision 0077.
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string] $Destination,

    [Parameter(Mandatory)]
    [string] $ApplicationId,

    [Parameter(Mandatory)]
    [string] $DisplayName,

    [string] $Content = 'Welcome to Anodrel.'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Test-ApplicationId {
    param([Parameter(Mandatory)] [string] $Value)

    return $Value -cmatch '^[a-z0-9][a-z0-9._-]{1,126}[a-z0-9]$'
}

function Get-UnicodeScalarCount {
    param([Parameter(Mandatory)] [string] $Value)

    $count = 0
    for ($index = 0; $index -lt $Value.Length; $index++) {
        $unit = [int][char] $Value[$index]
        if ($unit -ge 0xD800 -and $unit -le 0xDBFF) {
            if ($index + 1 -ge $Value.Length) {
                throw 'Text contains an unpaired UTF-16 high surrogate.'
            }

            $next = [int][char] $Value[$index + 1]
            if ($next -lt 0xDC00 -or $next -gt 0xDFFF) {
                throw 'Text contains an unpaired UTF-16 high surrogate.'
            }

            $index++
        }
        elseif ($unit -ge 0xDC00 -and $unit -le 0xDFFF) {
            throw 'Text contains an unpaired UTF-16 low surrogate.'
        }

        $count++
    }

    return $count
}

function Assert-DisplayName {
    param([Parameter(Mandatory)] [string] $Value)

    if ([string]::IsNullOrWhiteSpace($Value)) {
        throw 'DisplayName must not be empty or whitespace.'
    }
    if ($Value -match '[\p{Cc}]') {
        throw 'DisplayName must not contain control characters.'
    }
    if ([System.Text.Encoding]::UTF8.GetByteCount($Value) -gt 80) {
        throw 'DisplayName exceeds the 80-byte UTF-8 package limit.'
    }
}

function Assert-TextContent {
    param([Parameter(Mandatory)] [string] $Value)

    foreach ($character in $Value.ToCharArray()) {
        if ([char]::IsControl($character) -and $character -ne "`n") {
            throw 'Content may contain only line-feed control characters.'
        }
    }

    $contentBytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
    if ($contentBytes.Length -gt 8KB) {
        throw 'Content exceeds the 8 KiB package limit.'
    }
    if ((Get-UnicodeScalarCount -Value $Value) -gt 4096) {
        throw 'Content exceeds the 4,096 Unicode-scalar package limit.'
    }

    $lines = [System.Text.RegularExpressions.Regex]::Split($Value, "`n")
    if ($lines.Count -gt 128) {
        throw 'Content exceeds the 128-line package limit.'
    }
    foreach ($line in $lines) {
        if ((Get-UnicodeScalarCount -Value $line) -gt 160) {
            throw 'Content exceeds the 160-Unicode-scalar line limit.'
        }
    }

    return $contentBytes
}

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

if (-not (Test-ApplicationId -Value $ApplicationId)) {
    throw 'ApplicationId must be 3–128 lowercase ASCII letters, digits, dots, hyphens, or underscores, beginning and ending with a letter or digit.'
}
Assert-DisplayName -Value $DisplayName

$normalisedContent = $Content.Replace("`r`n", "`n").Replace("`r", "`n")
[byte[]] $contentBytes = Assert-TextContent -Value $normalisedContent

$destinationPath = [System.IO.Path]::GetFullPath($Destination)
if (Test-Path -LiteralPath $destinationPath) {
    throw "Destination already exists: $destinationPath"
}

$parentPath = [System.IO.Path]::GetDirectoryName($destinationPath)
if ([string]::IsNullOrEmpty($parentPath)) {
    throw 'Destination must name a package directory, not a filesystem root.'
}

[System.IO.Directory]::CreateDirectory($parentPath) | Out-Null
New-Item -ItemType Directory -Path $destinationPath -ErrorAction Stop | Out-Null

$contentDirectory = [System.IO.Path]::Combine($destinationPath, 'content')
[System.IO.Directory]::CreateDirectory($contentDirectory) | Out-Null
$contentPath = [System.IO.Path]::Combine($contentDirectory, 'main.txt')
$manifestPath = [System.IO.Path]::Combine($destinationPath, 'anodrel.application.json')
$attributesPath = [System.IO.Path]::Combine($destinationPath, '.gitattributes')
$utf8WithoutBom = [System.Text.UTF8Encoding]::new($false)

[System.IO.File]::WriteAllBytes($contentPath, $contentBytes)
[System.IO.File]::WriteAllText($attributesPath, "content/main.txt -text`n", $utf8WithoutBom)
$manifest = [ordered]@{
    manifestVersion = [ordered]@{ major = 1; minor = 0 }
    applicationId = $ApplicationId
    displayName = $DisplayName
    content = [ordered]@{
        format = 'anodrel.text.v1'
        path = 'content/main.txt'
        sha256 = Get-Sha256LowerHex -Bytes $contentBytes
    }
}
[System.IO.File]::WriteAllText(
    $manifestPath,
    (ConvertTo-Json -InputObject $manifest -Depth 4),
    $utf8WithoutBom
)

Write-Output "Created Anodrel package: $destinationPath"
Write-Output "Manifest: $manifestPath"
