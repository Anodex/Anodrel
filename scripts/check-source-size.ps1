[CmdletBinding()]
param(
    [ValidateRange(1, 10000)]
    [int]$MaximumLines = 550
)

$scriptRoot = Split-Path -Parent $PSCommandPath
$repositoryRoot = Split-Path -Parent $scriptRoot
$workingFiles = & git -C $repositoryRoot ls-files --cached --others --exclude-standard
if ($LASTEXITCODE -ne 0) {
    throw 'Unable to read the repository file list.'
}

$managedExtensions = @('.bat', '.css', '.html', '.js', '.json', '.md', '.ps1', '.rs', '.ts')
$violations = foreach ($workingFile in $workingFiles) {
    $extension = [System.IO.Path]::GetExtension($workingFile).ToLowerInvariant()
    if ($extension -notin $managedExtensions) {
        continue
    }
    $fullPath = Join-Path $repositoryRoot $workingFile
    if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) {
        continue
    }
    $lineCount = [System.IO.File]::ReadAllLines($fullPath).Length
    if ($lineCount -gt $MaximumLines) {
        [PSCustomObject]@{
            Lines = $lineCount
            Path = $workingFile
        }
    }
}

if ($violations) {
    $violations | Sort-Object Lines -Descending | Format-Table -AutoSize | Out-Host
    throw "Maintained files must stay at or below $MaximumLines lines."
}

Write-Host "All maintained files are at or below $MaximumLines lines."
