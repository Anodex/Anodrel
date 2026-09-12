<#
.SYNOPSIS
Internal helpers for retiring signed development-fixture cleanup caches.
.DESCRIPTION
Dot-sourced by prepare-installed-product-fixture.ps1. No action runs on import.
Only the fixed signed fixture installer can retire a cache or resume committed
package cleanup. PowerShell never deletes Program Files package/cache content.
#>

function Invoke-FixtureCleanupCache {
    $identityRoot = Split-Path -Parent $InstalledFixturePackageRoot
    if (-not (Test-Path -LiteralPath $identityRoot)) { return }
    $rootItem = Get-Item -LiteralPath $identityRoot -Force
    if (-not $rootItem.PSIsContainer -or ($rootItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'The fixed fixture identity root is unsafe. Development trust was not removed.'
    }
    $caches = @(Get-ChildItem -LiteralPath $identityRoot -Force |
        Where-Object { $_.Name.StartsWith('.anodrel-cleanup-', [StringComparison]::Ordinal) })
    if ($caches.Count -eq 0) { return }

    if (-not (Test-Path -LiteralPath $SignedInstallerPath -PathType Leaf)) {
        throw 'Cleanup evidence remains but the prepared signed fixture installer is missing. Keep development trust and investigate.'
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $SignedInstallerPath
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -ne $CertificateSubject) {
        throw 'The prepared fixture installer signature is not valid. Development trust was not removed.'
    }
    Invoke-Native -FilePath $SignedInstallerPath -Arguments @('cleanup-cache') `
        -FailureMessage 'Signed cleanup is incomplete. Close the application and any Anodrel removal-result dialog, then retry -Remove. Development trust was not removed.' | Out-Null
    $remaining = @(Get-ChildItem -LiteralPath $identityRoot -Force |
        Where-Object { $_.Name.StartsWith('.anodrel-cleanup-', [StringComparison]::Ordinal) })
    if ($remaining.Count -ne 0) {
        throw 'A fixture cleanup cache remains. Development trust was not removed.'
    }
}
