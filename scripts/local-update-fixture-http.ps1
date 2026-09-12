<#
.SYNOPSIS
Internal Windows HTTP Server API configuration helpers for the local update fixture.

.DESCRIPTION
Dot-sourced only by the fixed local-update fixture preparation script. These
functions create or remove one loopback TLS binding and one exact URL
reservation through Windows' built-in HTTP Server configuration command. They
never accept a host, port, prefix, application ID, or certificate selector.
#>

$script:FixtureHttpsPrefix = 'https://localhost:45863/anodrel/local-update/'
$script:FixtureTlsEndpoints = @('127.0.0.1:45863', '[::1]:45863')
$script:FixtureHttpApplicationId = '{9b0f5f1f-58cf-4e70-8c24-1a23258b6c82}'

function Invoke-FixtureHttpConfiguration {
    param(
        [Parameter(Mandatory)] [string[]] $Arguments,
        [Parameter(Mandatory)] [string] $FailureMessage
    )

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
        throw $FailureMessage
    }
    return ($output | Out-String)
}

function Get-FixtureTlsBinding {
    param([Parameter(Mandatory)] [string] $Endpoint)

    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = & netsh.exe http show sslcert "ipport=$Endpoint" 2>&1
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previous
    }
    if ($exitCode -eq 0) {
        return ($output | Out-String)
    }
    return $null
}

function Add-FixtureHttpEndpoint {
    param(
        [Parameter(Mandatory)] [string] $CertificateFingerprint,
        [Parameter(Mandatory)] [string] $AccountName
    )

    $existing = @($script:FixtureTlsEndpoints | Where-Object {
        $null -ne (Get-FixtureTlsBinding -Endpoint $_)
    })
    if ($existing.Count -ne 0) {
        throw 'The fixed local update HTTPS endpoint is already bound. It was not replaced.'
    }

    $addedBindings = @()
    try {
        foreach ($endpoint in $script:FixtureTlsEndpoints) {
            Invoke-FixtureHttpConfiguration -Arguments @(
                'http', 'add', 'sslcert', "ipport=$endpoint",
                "certhash=$CertificateFingerprint",
                "appid=$script:FixtureHttpApplicationId",
                'certstorename=MY'
            ) -FailureMessage 'Windows could not create the fixed local update TLS binding.' | Out-Null
            $addedBindings += $endpoint
        }
        Invoke-FixtureHttpConfiguration -Arguments @(
            'http', 'add', 'urlacl', "url=$script:FixtureHttpsPrefix", "user=$AccountName"
        ) -FailureMessage 'Windows could not reserve the fixed local update URL.' | Out-Null
    }
    catch {
        foreach ($endpoint in $addedBindings) {
            & netsh.exe http delete sslcert "ipport=$endpoint" *> $null
        }
        throw
    }
}

function Remove-FixtureHttpEndpoint {
    param([Parameter(Mandatory)] [string] $CertificateFingerprint)

    $lowerFingerprint = $CertificateFingerprint.ToLowerInvariant()
    $ownedBindings = @()
    foreach ($endpoint in $script:FixtureTlsEndpoints) {
        $binding = Get-FixtureTlsBinding -Endpoint $endpoint
        if ($null -eq $binding) {
            continue
        }
        if (-not $binding.ToLowerInvariant().Contains($lowerFingerprint)) {
            throw 'The fixed local update TLS endpoint has another certificate. It was not removed.'
        }
        $ownedBindings += $endpoint
    }
    if ($ownedBindings.Count -eq 0) {
        return
    }
    foreach ($endpoint in $ownedBindings) {
        Invoke-FixtureHttpConfiguration -Arguments @('http', 'delete', 'sslcert', "ipport=$endpoint") `
            -FailureMessage 'Windows could not remove the fixed local update TLS binding.' | Out-Null
    }
    Invoke-FixtureHttpConfiguration -Arguments @('http', 'delete', 'urlacl', "url=$script:FixtureHttpsPrefix") `
        -FailureMessage 'Windows could not remove the fixed local update URL reservation.' | Out-Null
}
