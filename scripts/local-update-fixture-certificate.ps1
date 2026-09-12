<#
.SYNOPSIS
Internal certificate identity helpers for the fixed local update fixture.

.DESCRIPTION
Dot-sourced only by the local-update fixture preparation and read-only
preparation-verification scripts. It creates no certificate or trust entry.
#>

function Get-LocalUpdateFixtureCertificateFingerprint {
    param([Parameter(Mandatory)] $Certificate)

    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
        return -join ($algorithm.ComputeHash($Certificate.RawData) | ForEach-Object { $_.ToString('x2') })
    }
    finally {
        $algorithm.Dispose()
    }
}

function Get-LocalUpdateFixtureHttpCertificateHash {
    param([Parameter(Mandatory)] $Certificate)

    $hash = ($Certificate.Thumbprint -replace '\s', '')
    if ($hash -notmatch '^[0-9A-Fa-f]{40}$') {
        throw 'The fixed local update TLS certificate has no valid Windows certificate hash.'
    }
    return $hash
}
