<#
.SYNOPSIS
Verifies the fixed no-restart fixture's completed registered-surface removal.

.DESCRIPTION
Reads only the fixed no-restart fixture policy, package, Start-menu, and
Installed Apps locations. It makes no trust, filesystem, registry, process,
network, installation, or desktop change. It does not observe a reboot, a
native consent dialog, UAC, or the helper's signed cache retirement.
#>

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$applicationId = 'org.anodrel.no-restart-fixture'
$displayName = 'Anodrel No-Restart Fixture'
$version = '0.1.0'
$programFiles = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
$packageRoot = Join-Path $programFiles "Anodrel\Applications\$applicationId\$version"
$policyPath = "HKLM:\Software\Anodrel\Applications\$applicationId"
$appsKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Anodrel.$applicationId"
$shortcut = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::CommonPrograms)) "Anodrel\$displayName.lnk"

function Assert-Absent {
    param([Parameter(Mandatory)] [string] $Path, [Parameter(Mandatory)] [string] $Label)

    if (Test-Path -LiteralPath $Path) {
        throw "The fixed no-restart fixture $Label remains."
    }
}

if (Test-Path -LiteralPath $policyPath) {
    $policy = Get-ItemProperty -LiteralPath $policyPath
    if ($null -ne $policy.PSObject.Properties['record']) {
        throw 'The fixed no-restart fixture machine policy record remains.'
    }
}

Assert-Absent -Path $packageRoot -Label 'package directory'
Assert-Absent -Path $appsKey -Label 'Installed Apps registration'
Assert-Absent -Path $shortcut -Label 'Start-menu shortcut'

[PSCustomObject]@{
    ApplicationId = $applicationId
    Version = $version
    PackageRoot = $packageRoot
    PolicyRecord = 'absent'
    InstalledAppsRegistration = 'absent'
    StartMenuShortcut = 'absent'
    Status = 'registered-surface-removal-verified'
} | ConvertTo-Json
