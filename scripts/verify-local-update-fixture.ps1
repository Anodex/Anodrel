<#
.SYNOPSIS
Reads the fixed local signed update fixture's selected 0.1.1 release state.

.DESCRIPTION
This verifier changes no trust, HTTP configuration, installation, policy,
shortcut, process, or desktop state. It invokes only the fixed prepared 0.1.1
installer's read-only verification and checks the fixed registered surfaces.
It cannot observe native consent, UAC, the local server, or a dialog.
#>

$ErrorActionPreference = 'Stop'
$ApplicationId = 'org.anodrel.local-update-fixture'
$Version = '0.1.1'
$LocalData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
$FixtureRoot = [IO.Path]::GetFullPath((Join-Path $LocalData 'Anodrel\LocalUpdateFixture'))
$Installer = Join-Path $FixtureRoot 'publication\releases\0.1.1\installer.exe'
$ProgramFiles = [Environment]::GetFolderPath([Environment+SpecialFolder]::ProgramFiles)
$PackageRoot = Join-Path $ProgramFiles "Anodrel\Applications\$ApplicationId\$Version"
$Executable = Join-Path $PackageRoot 'bin\anodrel-product-fixture.exe'
$Launcher = Join-Path $PackageRoot 'bin\anodrel-windows-host.exe'
$AppsKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Anodrel.$ApplicationId"
$Shortcut = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::CommonPrograms)) 'Anodrel\Anodrel Local Update Fixture.lnk'

function Assert-RegularSignedFile {
    param([Parameter(Mandatory)] [string] $Path)

    $item = Get-Item -LiteralPath $Path -Force
    if ($item.PSIsContainer -or (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {
        throw 'The fixed local update fixture has an unsafe signed image path.'
    }
    if ((Get-AuthenticodeSignature -LiteralPath $Path).Status -ne 'Valid') {
        throw 'The fixed local update fixture signed image is invalid.'
    }
}

if (-not (Test-Path -LiteralPath $Installer -PathType Leaf)) {
    throw 'The prepared local update fixture candidate installer is unavailable.'
}
Assert-RegularSignedFile -Path $Installer
& $Installer verify
if ($LASTEXITCODE -ne 0) {
    throw 'The selected local update fixture release did not pass signed verification.'
}
foreach ($path in @($PackageRoot, $Executable, $Launcher, $Shortcut)) {
    if (-not (Test-Path -LiteralPath $path)) {
        throw 'A fixed local update fixture registered surface is missing.'
    }
}
Assert-RegularSignedFile -Path $Executable
Assert-RegularSignedFile -Path $Launcher
if (-not (Test-Path -LiteralPath $AppsKey)) {
    throw 'The fixed local update fixture Apps and features registration is missing.'
}

[pscustomobject]@{
    ApplicationId = $ApplicationId
    Version = $Version
    PackageRoot = $PackageRoot
    InstalledAppsKey = $AppsKey
    StartMenuShortcut = $Shortcut
    Status = 'verified'
} | ConvertTo-Json
