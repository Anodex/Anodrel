<#
.SYNOPSIS
Verifies the fixed installed Anodrel development fixture without changing it.

.DESCRIPTION
Checks only one fixed fixture's signed installer verification, package layout,
Windows Installed Apps record, and Start-menu launcher/icon. It neither installs,
uninstalls, changes certificate trust, nor writes registry or filesystem state.
#>

[CmdletBinding()]
param([switch] $NoRestartAcceptance)

$ErrorActionPreference = 'Stop'

$fixture = if ($NoRestartAcceptance) {
    @{ ApplicationId = 'org.anodrel.no-restart-fixture'; DisplayName = 'Anodrel No-Restart Fixture'; LocalDirectory = 'InstalledNoRestartFixture'; InstallerName = 'AnodrelDevelopmentNoRestartFixtureInstaller.exe' }
} else {
    @{ ApplicationId = 'org.anodrel.product-fixture'; DisplayName = 'Anodrel Product Fixture'; LocalDirectory = 'InstalledProductFixture'; InstallerName = 'AnodrelDevelopmentProductFixtureInstaller.exe' }
}
$applicationId = $fixture.ApplicationId
$version = '0.1.0'
$displayName = $fixture.DisplayName
$programFiles = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
$packageRoot = Join-Path $programFiles "Anodrel\Applications\$applicationId\$version"
$launcher = Join-Path $packageRoot 'bin\anodrel-windows-host.exe'
$installer = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)) "Anodrel\$($fixture.LocalDirectory)\$($fixture.InstallerName)"
$shortcut = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::CommonPrograms)) "Anodrel\$displayName.lnk"
$icon = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::CommonPrograms)) 'Anodrel\Anodrel.ico'
$registryPath = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Anodrel.$applicationId"

function Assert-File {
    param([Parameter(Mandatory)] [string] $Path, [Parameter(Mandatory)] [string] $Label)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "The fixed fixture $Label is missing."
    }
}

Assert-File -Path $installer -Label 'installer image'
Assert-File -Path $launcher -Label 'host launcher'

& $installer verify
if ($LASTEXITCODE -ne 0) {
    throw 'The signed fixture installer did not verify the selected release.'
}

if (-not (Test-Path -LiteralPath $registryPath)) {
    throw 'The fixed Installed Apps registry entry is missing.'
}
$entry = Get-ItemProperty -LiteralPath $registryPath
if ($entry.DisplayName -ne $displayName -or $entry.DisplayVersion -ne $version -or $entry.Publisher -ne 'Anodrel') {
    throw 'The fixed Installed Apps registry entry does not match signed product metadata.'
}

Assert-File -Path $shortcut -Label 'Start-menu shortcut'
Assert-File -Path $icon -Label 'Start-menu brand icon'
$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut($shortcut)
if ($link.TargetPath -ne $launcher -or $link.WorkingDirectory -ne $packageRoot -or $link.Arguments -ne "--product-launch $applicationId" -or $link.IconLocation -ne "$icon,0") {
    throw 'The fixed Start-menu shortcut does not select the verified branded launcher route.'
}

[PSCustomObject]@{
    ApplicationId = $applicationId
    Version = $version
    PackageRoot = $packageRoot
    InstalledAppsKey = $registryPath
    StartMenuShortcut = $shortcut
    StartMenuIcon = $icon
    Status = 'verified'
} | ConvertTo-Json
