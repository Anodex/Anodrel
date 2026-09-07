<#
.SYNOPSIS
Rejects unowned runtime packages from Anodrel's TypeScript workspace.

.DESCRIPTION
Decision 0005 permits only Anodrel modules and language-standard facilities in
the production runtime. This read-only guard checks every committed workspace
manifest and package lock. Workspace runtime dependencies must resolve only to
local @anodrel packages. The root may retain only the reviewed compiler and
type-only development tools required to build and check that workspace.
#>

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$rootManifestPath = Join-Path $repositoryRoot 'package.json'
$lockfilePath = Join-Path $repositoryRoot 'package-lock.json'
$workspaceRoots = @(
    (Join-Path $repositoryRoot 'apps'),
    (Join-Path $repositoryRoot 'packages')
)
$approvedRootDevelopmentDependencies = @{
    '@types/node' = '^25.0.0'
    'typescript' = '^5.9.3'
}
$approvedLockDevelopmentPackages = @(
    '@types/node',
    'typescript',
    'undici-types'
)
$violations = [System.Collections.Generic.List[string]]::new()
$usesWindowsPowerShell = $PSVersionTable.PSEdition -eq 'Desktop'

if ($usesWindowsPowerShell) {
    Add-Type -AssemblyName System.Web.Extensions
}

function Add-Violation {
    param([Parameter(Mandatory)] [string] $Message)

    $violations.Add($Message)
}

function Get-Json {
    param([Parameter(Mandatory)] [string] $Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Required workspace file is missing: $Path"
    }
    $content = [System.IO.File]::ReadAllText($Path)
    if ($usesWindowsPowerShell) {
        return ([System.Web.Script.Serialization.JavaScriptSerializer]::new()).DeserializeObject($content)
    }
    return $content | ConvertFrom-Json -AsHashtable
}

function Get-RepositoryRelativePath {
    param(
        [Parameter(Mandatory)] [string] $Path,
        [Parameter(Mandatory)] [string] $Root
    )

    $separator = [System.IO.Path]::DirectorySeparatorChar
    $fullRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $prefix = "$fullRoot$separator"
    if (-not $fullPath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Workspace manifest is outside the repository: $Path"
    }
    return $fullPath.Substring($prefix.Length)
}

function Test-KeyedMap {
    param([AllowNull()] [object] $Object)

    return $Object -is [System.Collections.IDictionary] -or (
        $null -ne $Object.PSObject.Methods['ContainsKey'] -and
        $null -ne $Object.PSObject.Properties['Keys']
    )
}

function Get-PropertyValue {
    param(
        [Parameter(Mandatory)] [object] $Object,
        [Parameter(Mandatory)] [string] $Name
    )

    if (Test-KeyedMap $Object) {
        if (-not $Object.ContainsKey($Name)) {
            return $null
        }
        return $Object[$Name]
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function Get-ObjectProperties {
    param([AllowNull()] [object] $Object)

    if ($null -eq $Object) {
        return @()
    }
    if (Test-KeyedMap $Object) {
        return @($Object.Keys | ForEach-Object {
                [PSCustomObject]@{ Name = [string]$_; Value = $Object[$_] }
            })
    }
    return @($Object.PSObject.Properties)
}

function Test-ExactDependencyMap {
    param(
        [AllowNull()] [object] $Actual,
        [Parameter(Mandatory)] [hashtable] $Expected,
        [Parameter(Mandatory)] [string] $Description
    )

    $properties = @(Get-ObjectProperties $Actual)
    if ($properties.Count -ne $Expected.Count) {
        Add-Violation "$Description does not contain the reviewed dependency set."
        return
    }
    foreach ($name in $Expected.Keys) {
        $value = Get-PropertyValue -Object $Actual -Name $name
        if ($null -eq $value -or [string]$value -ne $Expected[$name]) {
            Add-Violation "$Description must declare '$name' as '$($Expected[$name])'."
        }
    }
}

function Test-FirstPartyRuntimeDependencies {
    param(
        [Parameter(Mandatory)] [object] $Manifest,
        [Parameter(Mandatory)] [System.Collections.Generic.HashSet[string]] $WorkspaceNames,
        [Parameter(Mandatory)] [string] $Description
    )

    foreach ($field in @('dependencies', 'optionalDependencies', 'peerDependencies')) {
        $dependencies = Get-PropertyValue -Object $Manifest -Name $field
        foreach ($dependency in Get-ObjectProperties $dependencies) {
            if ($dependency.Name -notlike '@anodrel/*') {
                Add-Violation "$Description declares external runtime dependency '$($dependency.Name)' in $field."
                continue
            }
            if (-not $WorkspaceNames.Contains($dependency.Name)) {
                Add-Violation "$Description declares unknown local runtime dependency '$($dependency.Name)' in $field."
            }
            if ([string]$dependency.Value -ne '0.1.0') {
                Add-Violation "$Description must pin runtime dependency '$($dependency.Name)' to workspace version 0.1.0."
            }
        }
    }
}

function Test-NoRuntimeDependencies {
    param(
        [Parameter(Mandatory)] [object] $Manifest,
        [Parameter(Mandatory)] [string] $Description
    )

    foreach ($field in @('dependencies', 'optionalDependencies', 'peerDependencies')) {
        if (@(Get-ObjectProperties (Get-PropertyValue -Object $Manifest -Name $field)).Count -gt 0) {
            Add-Violation "$Description must not declare $field."
        }
    }
}

$rootManifest = Get-Json $rootManifestPath
$lockfile = Get-Json $lockfilePath
if ([string](Get-PropertyValue -Object $rootManifest -Name 'name') -ne 'anodrel' -or
    [string](Get-PropertyValue -Object $rootManifest -Name 'version') -ne '0.1.0') {
    Add-Violation 'The root package must remain the anodrel 0.1.0 workspace manifest.'
}
if ((Get-PropertyValue -Object $rootManifest -Name 'private') -ne $true -or
    [string](Get-PropertyValue -Object $rootManifest -Name 'type') -ne 'module') {
    Add-Violation 'The root package must remain private and ESM-only.'
}
if (@(Compare-Object -ReferenceObject @('apps/*', 'packages/*') -DifferenceObject @(Get-PropertyValue -Object $rootManifest -Name 'workspaces')).Count -ne 0) {
    Add-Violation 'The root workspace set must remain exactly apps/* and packages/*.'
}
Test-NoRuntimeDependencies -Manifest $rootManifest -Description 'The root package'
Test-ExactDependencyMap -Actual (Get-PropertyValue -Object $rootManifest -Name 'devDependencies') -Expected $approvedRootDevelopmentDependencies -Description 'The root development dependencies'

$workspaceFiles = Get-ChildItem -LiteralPath $workspaceRoots -Recurse -File -Filter package.json
if ($workspaceFiles.Count -eq 0) {
    Add-Violation 'No TypeScript workspace manifests were found.'
}
$workspaceManifests = @{}
$workspaceNames = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
foreach ($file in $workspaceFiles) {
    $relativeManifestPath = Get-RepositoryRelativePath -Path $file.FullName -Root $repositoryRoot
    $relativePath = [System.IO.Path]::GetDirectoryName($relativeManifestPath).Replace([char]92, [char]47)
    $manifest = Get-Json $file.FullName
    $workspaceName = [string](Get-PropertyValue -Object $manifest -Name 'name')
    if ([string]::IsNullOrWhiteSpace($workspaceName) -or $workspaceName -notlike '@anodrel/*') {
        Add-Violation "Workspace manifest '$relativePath' must use an @anodrel name."
        continue
    }
    if (-not $workspaceNames.Add($workspaceName)) {
        Add-Violation "Workspace package '$workspaceName' is declared more than once."
    }
    $workspaceManifests[$relativePath] = $manifest
}

foreach ($relativePath in $workspaceManifests.Keys) {
    $manifest = $workspaceManifests[$relativePath]
    if ([string](Get-PropertyValue -Object $manifest -Name 'version') -ne '0.1.0' -or
        (Get-PropertyValue -Object $manifest -Name 'private') -ne $true -or
        [string](Get-PropertyValue -Object $manifest -Name 'type') -ne 'module') {
        Add-Violation "Workspace manifest '$relativePath' must remain private ESM version 0.1.0."
    }
    Test-FirstPartyRuntimeDependencies -Manifest $manifest -WorkspaceNames $workspaceNames -Description "Workspace manifest '$relativePath'"
    if (@(Get-ObjectProperties (Get-PropertyValue -Object $manifest -Name 'devDependencies')).Count -gt 0) {
        Add-Violation "Workspace manifest '$relativePath' must not add independent development dependencies."
    }
}

$lockPackages = Get-PropertyValue -Object $lockfile -Name 'packages'
if ($null -eq $lockPackages) {
    throw 'package-lock.json does not contain a packages map.'
}
$lockEntries = @{}
foreach ($entry in Get-ObjectProperties $lockPackages) {
    $lockEntries[$entry.Name] = $entry.Value
}
if (-not $lockEntries.ContainsKey('')) {
    Add-Violation 'package-lock.json does not contain its root package entry.'
} else {
    $rootLockEntry = $lockEntries['']
    if ([string](Get-PropertyValue -Object $rootLockEntry -Name 'name') -ne 'anodrel' -or
        [string](Get-PropertyValue -Object $rootLockEntry -Name 'version') -ne '0.1.0') {
        Add-Violation 'The root package-lock entry does not match the workspace manifest.'
    }
    Test-NoRuntimeDependencies -Manifest $rootLockEntry -Description 'The root package-lock entry'
    Test-ExactDependencyMap -Actual (Get-PropertyValue -Object $rootLockEntry -Name 'devDependencies') -Expected $approvedRootDevelopmentDependencies -Description 'The root package-lock development dependencies'
}

foreach ($relativePath in $workspaceManifests.Keys) {
    if (-not $lockEntries.ContainsKey($relativePath)) {
        Add-Violation "package-lock.json is missing workspace entry '$relativePath'."
        continue
    }
    $manifest = $workspaceManifests[$relativePath]
    $lockEntry = $lockEntries[$relativePath]
    $workspaceName = [string](Get-PropertyValue -Object $manifest -Name 'name')
    if ([string](Get-PropertyValue -Object $lockEntry -Name 'name') -ne $workspaceName -or
        [string](Get-PropertyValue -Object $lockEntry -Name 'version') -ne '0.1.0') {
        Add-Violation "package-lock workspace entry '$relativePath' does not match its manifest."
    }
    Test-FirstPartyRuntimeDependencies -Manifest $lockEntry -WorkspaceNames $workspaceNames -Description "package-lock workspace entry '$relativePath'"

    $linkPath = "node_modules/$workspaceName"
    if (-not $lockEntries.ContainsKey($linkPath)) {
        Add-Violation "package-lock.json is missing local link '$linkPath'."
        continue
    }
    $linkEntry = $lockEntries[$linkPath]
    if ((Get-PropertyValue -Object $linkEntry -Name 'link') -ne $true -or
        [string](Get-PropertyValue -Object $linkEntry -Name 'resolved') -ne $relativePath) {
        Add-Violation "package-lock local link '$linkPath' must resolve only to '$relativePath'."
    }
}

foreach ($packageName in $approvedLockDevelopmentPackages) {
    $entryPath = "node_modules/$packageName"
    if (-not $lockEntries.ContainsKey($entryPath)) {
        Add-Violation "package-lock.json is missing approved development package '$packageName'."
        continue
    }
    $entry = $lockEntries[$entryPath]
    if ((Get-PropertyValue -Object $entry -Name 'dev') -ne $true -or
        (Get-PropertyValue -Object $entry -Name 'link') -eq $true -or
        [string]::IsNullOrWhiteSpace([string](Get-PropertyValue -Object $entry -Name 'version')) -or
        [string](Get-PropertyValue -Object $entry -Name 'resolved') -notlike 'https://registry.npmjs.org/*' -or
        [string](Get-PropertyValue -Object $entry -Name 'integrity') -notlike 'sha512-*') {
        Add-Violation "Development package '$packageName' must remain a locked npm registry development-only package."
    }
}

if ($lockEntries.ContainsKey('node_modules/@types/node')) {
    Test-ExactDependencyMap -Actual (Get-PropertyValue -Object $lockEntries['node_modules/@types/node'] -Name 'dependencies') -Expected @{ 'undici-types' = '>=7.24.0 <7.24.7' } -Description 'The @types/node lock entry'
}
foreach ($entryPath in $lockEntries.Keys) {
    if ($entryPath -eq '' -or $workspaceManifests.ContainsKey($entryPath) -or $entryPath -like 'node_modules/@anodrel/*') {
        continue
    }
    if ($entryPath -notlike 'node_modules/*') {
        Add-Violation "package-lock.json contains unexpected entry '$entryPath'."
        continue
    }
    $packageName = $entryPath.Substring('node_modules/'.Length)
    if ($packageName -notin $approvedLockDevelopmentPackages) {
        Add-Violation "package-lock.json contains unapproved external package '$packageName'."
    }
}

if ($violations.Count -gt 0) {
    $violations | Sort-Object -Unique | ForEach-Object { Write-Error $_ }
    throw "$($violations.Count) TypeScript ownership check(s) failed."
}

Write-Output "TypeScript ownership guard passed for $($workspaceManifests.Count) local workspace packages and $($approvedLockDevelopmentPackages.Count) approved development packages."
