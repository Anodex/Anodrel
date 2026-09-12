<#
.SYNOPSIS
Reads the fixed local signed-update fixture preparation state on Windows.

.DESCRIPTION
Checks only the fixed local fixture's signed artifacts, temporary certificate
trust, and Windows HTTP Server configuration. It creates no files, changes no
trust, opens no listener, and does not install a product.
#>

$ErrorActionPreference = 'Stop'

$LocalData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
$FixtureRoot = Join-Path $LocalData 'Anodrel\LocalUpdateFixture'
$PublicationRoot = Join-Path $FixtureRoot 'publication'
$InitialInstaller = Join-Path $FixtureRoot 'AnodrelDevelopmentLocalUpdateFixtureInstaller.exe'
$CandidateInstaller = Join-Path $PublicationRoot 'releases\0.1.1\installer.exe'
$Catalogue = Join-Path $PublicationRoot 'stable.p7s'
$PublisherSubject = 'CN=Anodrel Development Local Update Fixture'
$TlsSubject = 'CN=localhost'
$TlsFriendlyName = 'Anodrel Local Update Fixture TLS'
$TlsEndpoints = @('127.0.0.1:45863', '[::1]:45863')
$HttpsPrefix = 'https://localhost:45863/anodrel/local-update/'
$HttpApplicationId = '{9b0f5f1f-58cf-4e70-8c24-1a23258b6c82}'

function Get-CertificateFingerprint {
    param([Parameter(Mandatory)] $Certificate)

    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
        return -join ($algorithm.ComputeHash($Certificate.RawData) | ForEach-Object { $_.ToString('x2') })
    }
    finally {
        $algorithm.Dispose()
    }
}

function Get-HttpCertificateHash {
    param([Parameter(Mandatory)] $Certificate)

    $hash = ($Certificate.Thumbprint -replace '\s', '')
    if ($hash -notmatch '^[0-9A-Fa-f]{40}$') {
        throw 'The fixed local update TLS certificate has no valid Windows certificate hash.'
    }
    return $hash
}

function Get-OneCertificate {
    param(
        [Parameter(Mandatory)] [string] $Store,
        [Parameter(Mandatory)] [scriptblock] $Filter,
        [Parameter(Mandatory)] [string] $Name
    )

    $certificates = @(Get-ChildItem -LiteralPath $Store | Where-Object $Filter)
    if ($certificates.Count -ne 1) {
        throw "Expected exactly one $Name certificate in $Store."
    }
    return $certificates[0]
}

function Assert-CertificateTrusted {
    param(
        [Parameter(Mandatory)] $Certificate,
        [Parameter(Mandatory)] [string] $Store
    )

    $fingerprint = Get-CertificateFingerprint -Certificate $Certificate
    $match = Get-ChildItem -LiteralPath $Store | Where-Object {
        (Get-CertificateFingerprint -Certificate $_) -eq $fingerprint
    } | Select-Object -First 1
    if ($null -eq $match) {
        throw "The fixed local update certificate is not trusted in $Store."
    }
}

function Assert-ValidArtifact {
    param([Parameter(Mandatory)] [string] $Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "The fixed local update artifact is missing: $Path"
    }
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -ne $PublisherSubject) {
        throw "The fixed local update artifact has an invalid publisher signature: $Path"
    }
}

function Get-NetshHttpOutput {
    param([Parameter(Mandatory)] [string[]] $Arguments)

    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = & netsh.exe @Arguments 2>&1
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previous
    }
    if ($exitCode -ne 0) {
        throw 'The fixed local update HTTP Server configuration is unavailable.'
    }
    return ($output | Out-String).ToLowerInvariant()
}

Assert-ValidArtifact -Path $InitialInstaller
Assert-ValidArtifact -Path $CandidateInstaller
if (-not (Test-Path -LiteralPath $Catalogue -PathType Leaf) -or (Get-Item -LiteralPath $Catalogue).Length -eq 0) {
    throw 'The fixed local update catalogue is missing or empty.'
}

$publisher = Get-OneCertificate -Store 'Cert:\CurrentUser\My' -Name 'fixture publisher' -Filter {
    $_.Subject -eq $PublisherSubject -and $_.HasPrivateKey
}
$tls = Get-OneCertificate -Store 'Cert:\LocalMachine\My' -Name 'fixture TLS' -Filter {
    $_.Subject -eq $TlsSubject -and $_.FriendlyName -eq $TlsFriendlyName -and $_.HasPrivateKey
}
Assert-CertificateTrusted -Certificate $publisher -Store 'Cert:\LocalMachine\Root'
Assert-CertificateTrusted -Certificate $publisher -Store 'Cert:\LocalMachine\TrustedPublisher'
Assert-CertificateTrusted -Certificate $tls -Store 'Cert:\LocalMachine\Root'

$httpHash = (Get-HttpCertificateHash -Certificate $tls).ToLowerInvariant()
$applicationId = $HttpApplicationId.ToLowerInvariant()
foreach ($endpoint in $TlsEndpoints) {
    $binding = Get-NetshHttpOutput -Arguments @('http', 'show', 'sslcert', "ipport=$endpoint")
    if (-not $binding.Contains($httpHash) -or -not $binding.Contains($applicationId)) {
        throw "The fixed local update TLS binding is not selected for $endpoint."
    }
}
$reservation = Get-NetshHttpOutput -Arguments @('http', 'show', 'urlacl', "url=$HttpsPrefix")
if (-not $reservation.Contains($HttpsPrefix.ToLowerInvariant())) {
    throw 'The fixed local update URL reservation is not selected.'
}

[pscustomobject]@{
    ApplicationId = 'org.anodrel.local-update-fixture'
    InitialVersion = '0.1.0'
    CandidateVersion = '0.1.1'
    HttpsPrefix = $HttpsPrefix
    PublicationRoot = $PublicationRoot
    Status = 'prepared'
} | ConvertTo-Json
