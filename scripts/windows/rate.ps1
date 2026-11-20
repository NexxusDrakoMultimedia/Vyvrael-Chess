# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
#
# Computes a rating list with Ordo from one or more PGN files, relative to
# an anchor engine. scripts/linux/rate.sh does the same with the same
# options. See docs/TESTING.md. Works in Windows PowerShell 5.1 and
# PowerShell 7.

# Without this every option would also be positional, and the first PGN
# file would be taken for the anchor.
[CmdletBinding(PositionalBinding = $false)]
param(
    [string] $Anchor = 'Stockfish 19',
    [string] $AnchorRating = '0',
    [string] $Simulations = '500',
    [string] $Out,
    [string] $Ordo,
    [switch] $DryRun,
    [switch] $Help,
    [Parameter(ValueFromRemainingArguments = $true)] [string[]] $Pgn = @()
)

$ErrorActionPreference = 'Stop'

$usage = @'
Usage:
  rate.ps1 [options] FILE.pgn [FILE.pgn ...]

Runs Ordo over the games and writes a text table and a CSV file next to the
first PGN file. The defaults are those of Ladder Baseline 1.

Options:
  -Anchor NAME        the engine whose rating is fixed (default "Stockfish 19")
  -AnchorRating N     the rating it is fixed at (default 0)
  -Simulations N      simulations for the error bars (default 500; 0 for none)
  -Out STEM           output files STEM.txt and STEM.csv
                      (default: the first PGN's name with -ratings added)
  -Ordo PATH          ordo to use (default: from PATH, or ORDO in
                      scripts/local.env)
  -DryRun             print the ordo command and stop
  -Help               show this text

The anchor must have played in the games. Ratings are relative to it: they
are not CCRL or FIDE ratings.
'@

# Runs a program and returns its output lines, stdout and stderr together.
# Windows PowerShell 5.1 turns what a program writes to stderr into error
# records, which under 'Stop' would end the script at the first warning the
# program prints. Inside this function such lines are just text.
function Invoke-Tool([string] $Exe, [string[]] $Arguments) {
    $ErrorActionPreference = 'Continue'
    & $Exe @Arguments 2>&1 | ForEach-Object { "$_" }
}

function Die([string] $Message) {
    [Console]::Error.WriteLine("rate.ps1: $Message")
    exit 2
}

$localFile = $env:VYVRAEL_LOCAL_ENV
if (-not $localFile) { $localFile = Join-Path $PSScriptRoot '..\local.env' }

# The value of KEY: environment VYVRAEL_KEY first, then the local file.
function Get-Setting([string] $Key) {
    $fromEnvironment = [Environment]::GetEnvironmentVariable("VYVRAEL_$Key")
    if ($fromEnvironment) { return $fromEnvironment }
    if (Test-Path -LiteralPath $localFile -PathType Leaf) {
        foreach ($line in [IO.File]::ReadAllLines($localFile)) {
            if ($line.StartsWith("$Key=")) { return $line.Substring($Key.Length + 1) }
        }
    }
    return ''
}

# .NET file calls resolve against the process directory, which need not be
# PowerShell's current location, so paths are made absolute first.
function Get-FullPath([string] $Path) {
    return $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
}

if ($Help) {
    Write-Output $usage
    exit 0
}
$pgns = @($Pgn | Where-Object { $_ })
foreach ($file in $pgns) {
    if ($file.StartsWith('-')) { Die "unknown option '$file'; see -Help" }
}
if ($pgns.Count -eq 0) {
    [Console]::Error.WriteLine($usage)
    exit 2
}
if (-not $Anchor) { Die '-Anchor may not be empty' }
if ($Anchor.Contains('"')) { Die '-Anchor may not contain a double quote' }
if ($AnchorRating -notmatch '^-?[0-9]+([.][0-9]+)?$') {
    Die "-AnchorRating must be a number, got '$AnchorRating'"
}
if ($Simulations -notmatch '^[0-9]+$') {
    Die "-Simulations must be a whole number, got '$Simulations'"
}

$anchorSeen = $false
$players = @()
foreach ($file in $pgns) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { Die "PGN file not found: $file" }
    foreach ($line in [IO.File]::ReadLines((Get-FullPath $file))) {
        if ($line -cmatch '^\[(White|Black) "(.*)"\]\s*$') {
            if ($Matches[2] -ceq $Anchor) { $anchorSeen = $true }
            if ($players -cnotcontains $Matches[2]) { $players += $Matches[2] }
        }
    }
}
if (-not $anchorSeen) {
    $found = ($players | Sort-Object) -join ', '
    if (-not $found) { $found = 'no players found' }
    Die "the anchor '$Anchor' played in none of the games; give -Anchor one of: $found"
}

if (-not $Out) { $Out = ($pgns[0] -replace '\.pgn$', '') + '-ratings' }
$text = "$Out.txt"
$csv = "$Out.csv"

if (-not $Ordo) { $Ordo = Get-Setting 'ORDO' }
if (-not $Ordo) { $Ordo = 'ordo' }
$needed = @('-a', '-A', '-W', '-p', '-o', '-c')
if ([long] $Simulations -gt 0) { $needed += '-s' }
$manyByList = $true
$tool = Get-Command -Name $Ordo -ErrorAction SilentlyContinue | Select-Object -First 1
if ($tool) {
    $helpText = (Invoke-Tool $Ordo @('-h')) -join "`n"
    foreach ($flag in $needed) {
        if ($helpText -cnotmatch "(?m)^\s*$([regex]::Escape($flag))\b") {
            Die "this ordo ($Ordo) does not list the option $flag in its help; the script needs it"
        }
    }
    if ($helpText -cnotmatch '(?m)^\s*-P\b') { $manyByList = $false }
} elseif ($DryRun) {
    [Console]::Error.WriteLine("rate.ps1: warning: ordo not found ($Ordo); its options were not checked")
} else {
    Die "ordo not found ($Ordo). Install it (docs/TESTING.md), or give -Ordo or ORDO in scripts/local.env"
}

# Several files: Ordo's own list file where it has one, else one joined file.
$input_ = @('-p', $pgns[0])
$helper = ''
if ($pgns.Count -gt 1) {
    if ($manyByList) {
        $helper = "$Out.pgn-list.txt"
        $input_ = @('-P', $helper)
    } else {
        $helper = "$Out.joined.pgn"
        $input_ = @('-p', $helper)
    }
}

$arguments = @('-a', $AnchorRating, '-A', $Anchor, '-W')
if ([long] $Simulations -gt 0) { $arguments += @('-s', $Simulations) }
$arguments += $input_ + @('-o', $text, '-c', $csv)

if ($DryRun) {
    Write-Output ((@($Ordo) + $arguments | ForEach-Object {
                if ($_ -match '\s') { "`"$_`"" } else { $_ }
            }) -join ' ')
    exit 0
}

$folder = Split-Path -Parent (Get-FullPath $Out)
try {
    New-Item -ItemType Directory -Force -Path $folder | Out-Null
} catch {
    Die "cannot create the folder for $Out"
}
$utf8 = New-Object System.Text.UTF8Encoding($false)
if ($helper) {
    if ($manyByList) {
        [IO.File]::WriteAllLines((Get-FullPath $helper), [string[]] $pgns, $utf8)
    } else {
        $joined = New-Object System.Text.StringBuilder
        foreach ($file in $pgns) {
            [void] $joined.Append([IO.File]::ReadAllText((Get-FullPath $file)))
            [void] $joined.Append("`n")
        }
        [IO.File]::WriteAllText((Get-FullPath $helper), $joined.ToString(), $utf8)
    }
}

# Tables left by an earlier run must not pass for this run's.
Remove-Item -LiteralPath $text, $csv -Force -ErrorAction SilentlyContinue
$status = 0
try {
    & $Ordo @arguments
    if ($null -ne $LASTEXITCODE) { $status = $LASTEXITCODE }
} finally {
    if ($helper) { Remove-Item -LiteralPath $helper -Force -ErrorAction SilentlyContinue }
}
if ($status -ne 0) { Die "ordo failed with status $status" }
if (-not (Test-Path -LiteralPath $text -PathType Leaf) -or (Get-Item -LiteralPath $text).Length -eq 0) {
    Die "ordo wrote no table to $text"
}
if (-not (Test-Path -LiteralPath $csv -PathType Leaf)) { Die "ordo wrote no CSV file to $csv" }

$table = [IO.File]::ReadAllLines((Get-FullPath $text))
$table | ForEach-Object { Write-Output $_ }
Write-Output ''
Write-Output '================ ORDER (strongest first) ================'
# The table's rows start with a rank; the name runs up to the colon.
foreach ($line in $table) {
    if ($line -match '^\s*([0-9]+) (.*\S)\s*: ') { Write-Output "$($Matches[1]). $($Matches[2])" }
}
Write-Output '=========================================================='
Write-Output 'Read the order, not the distances. Lopsided results stretch the Elo'
Write-Output 'scale, and with few games the error bars are wide, so close neighbours'
Write-Output "are not really separated. Ratings are relative to $Anchor = $AnchorRating."
Write-Output "Written: $text and $csv"
