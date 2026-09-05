<#
.SYNOPSIS
Runs Anodrel's fixed direct Windows UI Automation acceptance probes.

.DESCRIPTION
Builds the Windows host and the three compiled first-party probe children, then
runs the fixed property, focus, focus-event, Invoke, structure-event, and
live-status-event diagnostics. It verifies the locked native graph is
first-party before building. Each probe creates and closes only its own temporary
host window.

This script needs an interactive Windows desktop. It creates no certificate,
trust entry, installer, machine policy, network request, application package,
or persistent user state. It supplements, but does not replace, the manual
Narrator and Inspect checks in docs/ACCESSIBILITY_VERIFICATION.md.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$nativeManifest = Join-Path $repositoryRoot 'native\Cargo.toml'

function Invoke-NativeCommand {
    param(
        [Parameter(Mandatory)] [string] $Label,
        [Parameter(Mandatory)] [string] $FilePath,
        [string[]] $Arguments = @()
    )

    Write-Host "[$Label]"
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed."
    }
}

function Invoke-PowerShellCheck {
    param(
        [Parameter(Mandatory)] [string] $Label,
        [Parameter(Mandatory)] [string] $Path
    )

    Write-Host "[$Label]"
    & $Path
}

function Invoke-AccessibilityProbe {
    param(
        [Parameter(Mandatory)] [string] $Label,
        [Parameter(Mandatory)] [string[]] $HostArguments
    )

    Invoke-NativeCommand -Label $Label -FilePath $hostExecutable -Arguments $HostArguments
}

if (-not (Test-Path -LiteralPath $nativeManifest -PathType Leaf)) {
    throw 'The Anodrel native workspace was not found.'
}

Push-Location $repositoryRoot
try {
    Invoke-PowerShellCheck -Label 'Native ownership guard' -Path (Join-Path $repositoryRoot 'scripts\check-native-ownership.ps1')
    Invoke-NativeCommand -Label 'Build accessibility probes' -FilePath 'cargo' -Arguments @(
        'build', '--release', '--locked', '--manifest-path', $nativeManifest,
        '-p', 'anodrel-windows-host',
        '-p', 'anodrel-native-ui-client-sample',
        '-p', 'anodrel-native-structure-event-client',
        '-p', 'anodrel-native-live-status-event-client'
    )
    $metadata = (& cargo metadata --locked --manifest-path $nativeManifest --no-deps --format-version 1 | ConvertFrom-Json)
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($metadata.target_directory)) {
        throw 'Cargo did not report the native target directory.'
    }
    $releaseDirectory = Join-Path $metadata.target_directory 'release'
    $hostExecutable = Join-Path $releaseDirectory 'anodrel-windows-host.exe'
    $invokeClient = Join-Path $releaseDirectory 'anodrel-native-ui-client-sample.exe'
    $structureClient = Join-Path $releaseDirectory 'anodrel-native-structure-event-client.exe'
    $liveStatusClient = Join-Path $releaseDirectory 'anodrel-native-live-status-event-client.exe'
    foreach ($path in @($hostExecutable, $invokeClient, $structureClient, $liveStatusClient)) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "The expected accessibility probe executable was not built: $path"
        }
    }

    Invoke-AccessibilityProbe -Label 'UI Automation property probe' -HostArguments @('--uia-property-probe')
    Invoke-AccessibilityProbe -Label 'UI Automation focus probe' -HostArguments @('--uia-focus-probe')
    Invoke-AccessibilityProbe -Label 'UI Automation focus-event probe' -HostArguments @('--uia-focus-event-probe')
    Invoke-AccessibilityProbe -Label 'UI Automation Invoke probe' -HostArguments @('--uia-invoke-probe', $invokeClient)
    Invoke-AccessibilityProbe -Label 'UI Automation structure-event probe' -HostArguments @('--uia-structure-event-probe', $structureClient)
    Invoke-AccessibilityProbe -Label 'UI Automation live-status-event probe' -HostArguments @('--uia-live-status-event-probe', $liveStatusClient)
}
finally {
    Pop-Location
}

Write-Output 'Windows UI Automation acceptance probes passed.'
Write-Output 'Manual Narrator and Inspect acceptance checks remain required.'
