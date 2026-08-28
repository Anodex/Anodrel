<#
.SYNOPSIS
Checks repository-local links in maintained Markdown and the project site.

.DESCRIPTION
The public documentation is a maintained part of Anodrel's interface. This
check resolves local Markdown links and HTML href values from every document
under docs/, rejecting a missing target or a target outside the repository.
Network and anchor-only links are intentionally left to their remote owners.
#>

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$docsRoot = Join-Path $repositoryRoot 'docs'
$documents = Get-ChildItem -LiteralPath $docsRoot -Recurse -File -Include '*.md', '*.html'
$problems = [System.Collections.Generic.List[string]]::new()

function Test-ExternalLink {
    param([Parameter(Mandatory)] [string] $Target)

    return $Target.StartsWith('#') -or
        $Target.StartsWith('//') -or
        $Target -match '^[a-zA-Z][a-zA-Z0-9+.-]*:'
}

function Test-LocalTarget {
    param(
        [Parameter(Mandatory)] [System.IO.FileInfo] $Document,
        [Parameter(Mandatory)] [string] $Target
    )

    $path = $Target.Split('#', 2)[0].Trim('<', '>')
    if ($path.Length -eq 0 -or (Test-ExternalLink $path)) {
        return
    }

    $candidate = [System.IO.Path]::GetFullPath((Join-Path $Document.DirectoryName $path))
    $rootWithSeparator = $repositoryRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $candidate.StartsWith($rootWithSeparator, [System.StringComparison]::OrdinalIgnoreCase)) {
        $problems.Add("$($Document.FullName): link escapes the repository: $Target")
        return
    }
    if (-not (Test-Path -LiteralPath $candidate)) {
        $problems.Add("$($Document.FullName): link target does not exist: $Target")
    }
}

foreach ($document in $documents) {
    $content = [System.IO.File]::ReadAllText($document.FullName)
    $matches = [System.Text.RegularExpressions.Regex]::Matches(
        $content,
        '(?i)(?:\[[^\]]*\]\((?<markdown>[^\s)]+)(?:\s+[^)]*)?\)|href\s*=\s*["''](?<html>[^"'']+)["''])'
    )
    foreach ($match in $matches) {
        $target = if ($match.Groups['markdown'].Success) {
            $match.Groups['markdown'].Value
        } else {
            $match.Groups['html'].Value
        }
        Test-LocalTarget -Document $document -Target $target
    }
}

if ($problems.Count -gt 0) {
    $problems | ForEach-Object { Write-Error $_ }
    throw "$($problems.Count) documentation link check(s) failed."
}

Write-Output "Documentation link check passed for $($documents.Count) files."
