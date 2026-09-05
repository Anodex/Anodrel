<#
.SYNOPSIS
Rejects non-first-party Rust packages and dependencies in the native workspace.

.DESCRIPTION
Decision 0005 keeps Anodrel's native behavior in first-party modules over
operating-system APIs. This read-only guard asks Cargo for every locally declared
package and dependency, then checks the committed lockfile. It requires the
native workspace to contain only anodrel-* packages, local dependency paths
under native/, and no external package source.
#>

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$nativeRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot 'native'))
$nativeManifest = Join-Path $nativeRoot 'Cargo.toml'
$lockfile = Join-Path $nativeRoot 'Cargo.lock'
$violations = [System.Collections.Generic.List[string]]::new()

function Test-NativePath {
    param([Parameter(Mandatory)] [string] $Path)

    $candidate = [System.IO.Path]::GetFullPath($Path)
    $separator = [System.IO.Path]::DirectorySeparatorChar
    return $candidate.Equals($nativeRoot, [System.StringComparison]::OrdinalIgnoreCase) -or
        $candidate.StartsWith("$nativeRoot$separator", [System.StringComparison]::OrdinalIgnoreCase)
}

function Add-Violation {
    param([Parameter(Mandatory)] [string] $Message)

    $violations.Add($Message)
}

if (-not (Test-Path -LiteralPath $nativeManifest -PathType Leaf)) {
    throw 'The Anodrel native workspace manifest was not found.'
}
if (-not (Test-Path -LiteralPath $lockfile -PathType Leaf)) {
    throw 'The Anodrel native lockfile was not found.'
}

$metadataOutput = & cargo metadata --manifest-path $nativeManifest --format-version 1 --no-deps --locked
if ($LASTEXITCODE -ne 0) {
    throw 'Cargo could not read the locked native workspace metadata.'
}
$metadata = ($metadataOutput -join [System.Environment]::NewLine) | ConvertFrom-Json
$packagesByName = @{}

foreach ($package in $metadata.packages) {
    $packagesByName[$package.name] = $package
}

foreach ($package in $metadata.packages) {
    if ($package.name -notlike 'anodrel-*') {
        Add-Violation "Native package '$($package.name)' does not use the anodrel- prefix."
    }
    if (-not (Test-NativePath -Path $package.manifest_path)) {
        Add-Violation "Native package '$($package.name)' is outside native/: $($package.manifest_path)"
    }
    if (-not [string]::IsNullOrWhiteSpace([string]$package.source)) {
        Add-Violation "Native package '$($package.name)' declares an external source: $($package.source)"
    }

    foreach ($dependency in $package.dependencies) {
        if ([string]::IsNullOrWhiteSpace([string]$dependency.path)) {
            Add-Violation "Native package '$($package.name)' depends on '$($dependency.name)' without a local path."
            continue
        }
        if (-not (Test-NativePath -Path $dependency.path)) {
            Add-Violation "Native package '$($package.name)' depends on '$($dependency.name)' outside native/: $($dependency.path)"
        }
        if (-not [string]::IsNullOrWhiteSpace([string]$dependency.source)) {
            Add-Violation "Native package '$($package.name)' depends on '$($dependency.name)' from $($dependency.source)."
        }
        if (-not $packagesByName.ContainsKey($dependency.name)) {
            Add-Violation "Native package '$($package.name)' depends on unowned package '$($dependency.name)'."
        }
    }
}

$lockedNames = [System.Collections.Generic.List[string]]::new()
foreach ($line in [System.IO.File]::ReadAllLines($lockfile)) {
    if ($line -match '^\s*source\s*=') {
        Add-Violation "Cargo.lock declares an external package source: $line"
    }
    if ($line -match '^\s*name\s*=\s*"([^"]+)"\s*$') {
        $name = $Matches[1]
        $lockedNames.Add($name)
        if ($name -notlike 'anodrel-*') {
            Add-Violation "Cargo.lock contains non-Anodrel package '$name'."
        }
    }
}

if ($lockedNames.Count -eq 0) {
    Add-Violation 'Cargo.lock contains no package records.'
}

if ($violations.Count -gt 0) {
    $violations | Sort-Object -Unique | ForEach-Object { Write-Error $_ }
    throw "$($violations.Count) native ownership check(s) failed."
}

Write-Output "Native ownership guard passed for $($metadata.packages.Count) first-party packages and $($lockedNames.Count) locked packages."
