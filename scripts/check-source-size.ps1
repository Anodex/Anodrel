[CmdletBinding()]
param(
    [ValidateRange(1, 10000)]
    [int]$MaximumLines = 550
)

$scriptRoot = Split-Path -Parent $PSCommandPath
$repositoryRoot = Split-Path -Parent $scriptRoot
$trackedFiles = & git -C $repositoryRoot ls-files
if ($LASTEXITCODE -ne 0) {
    throw 'Unable to read the repository file list.'
}

$managedExtensions = @('.bat', '.css', '.html', '.js', '.json', '.md', '.ps1', '.rs', '.ts')
$violations = foreach ($trackedFile in $trackedFiles) {
    $extension = [System.IO.Path]::GetExtension($trackedFile).ToLowerInvariant()
    if ($extension -notin $managedExtensions) {
        continue
    }
    $fullPath = Join-Path $repositoryRoot $trackedFile
    $lineCount = [System.IO.File]::ReadAllLines($fullPath).Length
    if ($lineCount -gt $MaximumLines) {
        [PSCustomObject]@{
            Lines = $lineCount
            Path = $trackedFile
        }
    }
}

if ($violations) {
    $violations | Sort-Object Lines -Descending | Format-Table -AutoSize | Out-Host
    throw "Maintained files must stay at or below $MaximumLines lines."
}

Write-Host "All tracked maintained files are at or below $MaximumLines lines."
