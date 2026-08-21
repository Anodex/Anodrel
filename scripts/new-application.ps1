<#
.SYNOPSIS
Creates one strict Anodrel text application package on Windows.

.DESCRIPTION
This is only a PowerShell convenience wrapper. It delegates package creation,
validation, digest calculation, and post-write verification to the first-party
native anodrel-package-tool, which shares the Windows host's package validator.
It does not implement or relax those rules itself.

See docs/APPLICATION_TEMPLATE.md and Decisions 0077 and 0078.
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

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$nativeManifest = Join-Path $repositoryRoot 'native\Cargo.toml'
$arguments = @(
    'run', '--release', '--manifest-path', $nativeManifest,
    '-p', 'anodrel-package-tool', '--',
    'init', $Destination, $ApplicationId, $DisplayName, $Content
)

# Cargo writes ordinary build progress to standard error. The native tool's exit
# code is its command contract, so preserve a non-zero failure without turning
# ordinary progress output into a PowerShell terminating error.
$previousPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
try {
    & cargo @arguments
    $exitCode = $LASTEXITCODE
}
finally {
    $ErrorActionPreference = $previousPreference
}

if ($exitCode -ne 0) {
    throw 'The Anodrel native package tool did not create a package.'
}
