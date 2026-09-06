<#
.SYNOPSIS
Runs Anodrel's non-interactive Windows release evidence checks.

.DESCRIPTION
Runs formatting, TypeScript and native ownership checks, strict native lint,
source-size, documentation-link, complete native-workspace, release-frame-budget,
and startup-report checks from one clean checkout. With
-IncludeIdleReport, it also records the fixed 30-second static-window idle
measurement. The default check prints evidence only and creates no certificate,
trust entry, installer, machine policy, product shortcut, update request, or
desktop window.

It cannot prove native consent, UAC, Start-menu, Explorer, file-picker, menu,
or screen-reader behaviour. Those remain explicit operator checks in
docs/WINDOWS_RELEASE.md.
#>

[CmdletBinding()]
param(
    # Opt in to the real desktop measurement required for release-candidate
    # performance evidence. It shows a fixed host window for 30 seconds.
    [switch] $IncludeIdleReport
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$nativeManifest = Join-Path $repositoryRoot 'native\Cargo.toml'
$sampleManifest = Join-Path $repositoryRoot 'apps\sample\anodrel.application.json'

function Invoke-NativeCheck {
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

if (-not (Test-Path -LiteralPath $nativeManifest -PathType Leaf)) {
    throw 'The Anodrel native workspace was not found.'
}
if (-not (Test-Path -LiteralPath $sampleManifest -PathType Leaf)) {
    throw 'The Anodrel sample package was not found.'
}

Push-Location $repositoryRoot
try {
    Invoke-NativeCheck -Label 'Rust formatting' -FilePath 'cargo' -Arguments @(
        'fmt', '--manifest-path', $nativeManifest, '--all', '--', '--check'
    )
    Invoke-PowerShellCheck -Label 'TypeScript ownership guard' -Path (Join-Path $repositoryRoot 'scripts\check-typescript-ownership.ps1')
    Invoke-PowerShellCheck -Label 'Native ownership guard' -Path (Join-Path $repositoryRoot 'scripts\check-native-ownership.ps1')
    Invoke-NativeCheck -Label 'Native workspace lint' -FilePath 'cargo' -Arguments @(
        'clippy', '--manifest-path', $nativeManifest, '--workspace', '--all-targets', '--', '-D', 'warnings'
    )
    Invoke-PowerShellCheck -Label 'Source-size guard' -Path (Join-Path $repositoryRoot 'scripts\check-source-size.ps1')
    Invoke-PowerShellCheck -Label 'Documentation links' -Path (Join-Path $repositoryRoot 'scripts\check-documentation-links.ps1')
    Invoke-NativeCheck -Label 'Whitespace diff guard' -FilePath 'git' -Arguments @('diff', '--check')
    Invoke-NativeCheck -Label 'Native workspace tests' -FilePath 'cargo' -Arguments @(
        'test', '--manifest-path', $nativeManifest, '--workspace', '--quiet'
    )
    Invoke-NativeCheck -Label 'Release frame budget' -FilePath 'cargo' -Arguments @(
        'test', '--release', '--manifest-path', $nativeManifest, '-p', 'anodrel-windows-host',
        'frame_budget', '--', '--nocapture'
    )
    Invoke-NativeCheck -Label 'Release startup report' -FilePath 'cargo' -Arguments @(
        'run', '--release', '--manifest-path', $nativeManifest, '-p', 'anodrel-windows-host',
        '--', '--startup-report', $sampleManifest
    )
    if ($IncludeIdleReport) {
        Invoke-NativeCheck -Label 'Release idle-window report' -FilePath 'cargo' -Arguments @(
            'run', '--release', '--manifest-path', $nativeManifest, '-p', 'anodrel-windows-host',
            '--', '--idle-performance-report'
        )
    }
}
finally {
    Pop-Location
}

Write-Output 'Automated Windows release evidence passed.'
Write-Output 'Manual desktop and machine-trust acceptance checks remain required.'
