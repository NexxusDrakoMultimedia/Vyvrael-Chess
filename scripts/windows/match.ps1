# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Nexxus Drako Multimedia
#
# Runs a cutechess-cli match for Vyvrael: an SPRT between two builds, a
# gauntlet against opponents from the ladder, or a head-to-head between two
# builds. scripts/linux/match.sh does the same with the same options.
# See docs/TESTING.md. Works in Windows PowerShell 5.1 and PowerShell 7.

param(
    [Parameter(Position = 0)] [string] $Mode,
    [string] $Test,
    [string] $TestName,
    [string] $Base,
    [string] $BaseName,
    [string[]] $Opponent = @(),
    [string] $Elo0,
    [string] $Elo1,
    [string] $Alpha,
    [string] $Beta,
    [string] $Rounds,
    [string] $Tc = '10+0.1',
    [string] $Hash = '64',
    [string] $Concurrency,
    [string] $Openings,
    [string] $EnginesJson,
    [string] $Cutechess,
    [string] $OutDir,
    [string] $Label,
    [string] $Seed,
    [switch] $Deterministic,
    [string[]] $TestOption = @(),
    [string[]] $BaseOption = @(),
    [switch] $LowPriority,
    [switch] $NoLowPriority,
    [switch] $DryRun,
    [switch] $Help
)

$ErrorActionPreference = 'Stop'

$usage = @'
Usage:
  match.ps1 sprt     -Test PATH -TestName NAME -Base PATH -BaseName NAME
                     -Elo0 N -Elo1 N -Alpha N -Beta N -Rounds N [options]
  match.ps1 h2h      -Test PATH -TestName NAME -Base PATH -BaseName NAME
                     -Rounds N [options]
  match.ps1 gauntlet -Test PATH -TestName NAME -Opponent NAME[,NAME ...]
                     -Rounds N [options]

Modes:
  sprt      a sequential probability ratio test of the test build against
            the base build. The bounds, alpha and beta have no defaults.
  h2h       a fixed number of rounds between two builds.
  gauntlet  the test build against each opponent, named as in the engine
            list (-EnginesJson).

A round is two games: each opening is played twice with colours swapped.

Options:
  -Rounds N           rounds to play; in sprt the cap after which the test
                      stops undecided (required)
  -Tc TC              time control (default 10+0.1)
  -Hash MB            hash size for every engine (default 64)
  -Concurrency N      games at once (default: physical cores minus two)
  -Openings FILE      opening file in EPD format (required, or OPENINGS)
  -EnginesJson FILE   engine list for gauntlet opponents (or ENGINES_JSON)
  -Cutechess PATH     cutechess-cli to use (default: from PATH)
  -OutDir DIR         where the PGN, log and notes go (default: matches)
  -Label TEXT         added to the output file names
  -Seed N             seed for the opening order (default: random)
  -Deterministic      set Seed=822286742 and Variety=0 on the Vyvrael builds
  -TestOption NAME=VALUE[,NAME=VALUE ...]
                      UCI options for the test build, for example
                      UCI_Elo=1500 (not Threads or Hash)
  -BaseOption NAME=VALUE[,NAME=VALUE ...]
                      the same for the base build (sprt and h2h)
  -LowPriority       run at low priority (BelowNormal)
  -NoLowPriority      do not, whatever the local settings say
  -DryRun             print the cutechess-cli command and stop
  -Help               show this text

Several opponents: -Opponent "Stockfish 19-1320","Vice 1.1" (or one string
with the names separated by semicolons); several options likewise:
-TestOption "UCI_LimitStrength=true","UCI_Elo=1500".

Build names are ASCII only, for example "Vyvrael 0.1.0-a1" or
"Vyvrael dev-1a2b3c4". Settings can also come from the environment
(VYVRAEL_CONCURRENCY, ...) or scripts/local.env; see
scripts/local.env.example.

Beside the PGN a run leaves a .log and .notes.txt (the command, the git
commit and whether the repository had changes, each engine's file and
SHA-256, the times, the outcome, the number of games and the PGN's
SHA-256), and for a gauntlet a copy of the engine list (.engines.json).
'@

function Die([string] $Message) {
    [Console]::Error.WriteLine("match.ps1: $Message")
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

# Runs a program and returns its output lines, stdout and stderr together.
# Windows PowerShell 5.1 turns what a program writes to stderr into error
# records, which under 'Stop' would end the script at the first warning the
# program prints. Inside this function such lines are just text.
function Invoke-Tool([string] $Exe, [string[]] $Arguments) {
    $ErrorActionPreference = 'Continue'
    & $Exe @Arguments 2>&1 | ForEach-Object { "$_" }
}

function Test-Whole([string] $Text) { return $Text -match '^[0-9]+$' }
function Test-Number([string] $Text) { return $Text -match '^-?[0-9]+([.][0-9]+)?$' }

# Physical cores, not hardware threads; 0 if it cannot be told.
function Get-PhysicalCores {
    # PHYSICAL_CORES gives the count where it cannot be detected.
    $given = Get-Setting 'PHYSICAL_CORES'
    if ($given) {
        if (-not (Test-Whole $given)) { Die "PHYSICAL_CORES must be a whole number, got '$given'" }
        return [int] $given
    }
    try {
        $sum = (Get-CimInstance Win32_Processor -ErrorAction Stop |
            Measure-Object -Property NumberOfCores -Sum).Sum
        return [int] $sum
    } catch {
        return 0
    }
}

if ($Help -or $Mode -in @('help', '--help', '-h')) {
    Write-Output $usage
    exit 0
}
if (-not $Mode) {
    [Console]::Error.WriteLine($usage)
    exit 2
}
if ($Mode -notin @('sprt', 'h2h', 'gauntlet')) {
    Die "unknown mode '$Mode' (expected sprt, h2h or gauntlet); see -Help"
}
$Mode = $Mode.ToLowerInvariant()

if (-not $Concurrency) { $Concurrency = Get-Setting 'CONCURRENCY' }
if (-not $Openings) { $Openings = Get-Setting 'OPENINGS' }
if (-not $EnginesJson) { $EnginesJson = Get-Setting 'ENGINES_JSON' }
if (-not $Cutechess) { $Cutechess = Get-Setting 'CUTECHESS_CLI' }
if (-not $OutDir) { $OutDir = Get-Setting 'MATCH_DIR' }
if (-not $OutDir) { $OutDir = 'matches' }
# An argument replaces the setting before it is checked, as in match.sh.
$low = Get-Setting 'LOW_PRIORITY'
if (-not $low) { $low = 'false' }
if ($LowPriority) { $low = 'true' }
if ($NoLowPriority) { $low = 'false' }
if ($low -cnotin @('true', 'false')) { Die "LOW_PRIORITY must be true or false, got '$low'" }
$opponents = @($Opponent | ForEach-Object { $_ -split ';' } | Where-Object { $_ -ne '' })
$testOptions = @($TestOption | ForEach-Object { $_ -split ';' } | Where-Object { $_ -ne '' })
$baseOptions = @($BaseOption | ForEach-Object { $_ -split ';' } | Where-Object { $_ -ne '' })

# --- what every mode needs -------------------------------------------------

if (-not $Test) { Die '-Test is required: the build under test' }
if (-not $TestName) { Die '-TestName is required' }
if (-not $Rounds) { Die '-Rounds is required: a match must have an end' }
if (-not (Test-Whole $Rounds) -or [long] $Rounds -le 0) {
    Die "-Rounds must be a whole number above zero, got '$Rounds'"
}
if (-not (Test-Whole $Hash) -or [long] $Hash -le 0) {
    Die "-Hash must be a whole number above zero, got '$Hash'"
}
# "08" is 8, and is passed on as 8.
$Rounds = [string] [long] $Rounds
$Hash = [string] [long] $Hash
if ($Tc -notmatch '^[0-9./:+]+$') {
    Die "-Tc '$Tc' does not look like a time control (for example 10+0.1)"
}
if ($Seed -and -not (Test-Whole $Seed)) { Die "-Seed must be a whole number, got '$Seed'" }
if ($Label -and $Label -notmatch '^[A-Za-z0-9._-]+$') {
    Die "-Label may only hold letters, digits, '.', '_' and '-'"
}

function Test-BuildName([string] $What, [string] $Name) {
    if ($Name -cnotmatch '^[ -~]+$') {
        Die "$What '$Name' must be plain ASCII (for example `"Vyvrael 0.1.0-a1`")"
    }
    if ($Name -match '["''=]') { Die "$What '$Name' may not contain quotes or '='" }
    if ($Name -ne $Name.Trim(' ')) { Die "$What '$Name' may not start or end with a space" }
}
function Test-EngineFile([string] $What, [string] $Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { Die "$What not found: $Path" }
}
# The UCI options given to one build, each NAME=VALUE.
function Test-Options([string] $What, [string[]] $Options) {
    $seen = @{}
    foreach ($option in $Options) {
        if ($option -cnotmatch '^[^=]+=[^=]+$') {
            if ($option -match '^[^=]+=[^=]*=') { Die "$What '$option' may hold only one '='" }
            Die "$What '$option' must be NAME=VALUE (for example UCI_Elo=1500)"
        }
        if ($option -cnotmatch '^[ -~]+$') { Die "$What '$option' must be plain ASCII" }
        if ($option -match '["'';]') { Die "$What '$option' may not contain quotes or ';'" }
        $name = $option.Substring(0, $option.IndexOf('='))
        if ($name -ne $name.Trim(' ')) {
            Die "$What '$option': the name may not start or end with a space"
        }
        # UCI option names are not case-sensitive.
        $lower = $name.ToLowerInvariant()
        if ($lower -in @('threads', 'hash')) {
            Die "$What '$option': the script sets $name (one thread; -Hash for the hash size)"
        }
        if ($Deterministic -and $lower -in @('seed', 'variety')) {
            Die "$What '$option': -Deterministic already sets $name"
        }
        if ($seen.ContainsKey($lower)) { Die "${What}: the option $name is given twice" }
        $seen[$lower] = $true
    }
}

Test-BuildName '-TestName' $TestName
Test-EngineFile 'the test build (-Test)' $Test
Test-Options '-TestOption' $testOptions

# --- what each mode needs --------------------------------------------------

if ($Mode -in @('sprt', 'h2h')) {
    if (-not $Base) { Die "-Base is required in $Mode mode: the build to compare with" }
    if (-not $BaseName) { Die "-BaseName is required in $Mode mode" }
    Test-BuildName '-BaseName' $BaseName
    Test-EngineFile 'the base build (-Base)' $Base
    if ($TestName -ceq $BaseName) {
        Die "the two builds have the same name '$TestName'; the results could not be told apart"
    }
    if ($opponents.Count -gt 0) { Die '-Opponent is for gauntlet mode' }
    Test-Options '-BaseOption' $baseOptions
} else {
    if ($opponents.Count -eq 0) { Die 'gauntlet mode needs at least one -Opponent' }
    if ($Base -or $BaseName) { Die '-Base and -BaseName are for sprt and h2h modes' }
    if ($baseOptions.Count -gt 0) {
        Die "-BaseOption is for sprt and h2h modes; an opponent's options are in the engine list"
    }
    if (-not $EnginesJson) {
        Die 'gauntlet mode needs the engine list: -EnginesJson, or ENGINES_JSON in scripts/local.env'
    }
    if (-not (Test-Path -LiteralPath $EnginesJson -PathType Leaf)) {
        Die "engine list not found: $EnginesJson"
    }
}

if ($Mode -eq 'sprt') {
    foreach ($pair in @(@('Elo0', $Elo0), @('Elo1', $Elo1), @('Alpha', $Alpha), @('Beta', $Beta))) {
        if (-not $pair[1]) {
            Die "-$($pair[0]) is required in sprt mode; there are no default bounds (docs/TESTING.md has examples)"
        }
        if (-not (Test-Number $pair[1])) { Die "-$($pair[0]) must be a number, got '$($pair[1])'" }
    }
} elseif ($Elo0 -or $Elo1 -or $Alpha -or $Beta) {
    Die '-Elo0, -Elo1, -Alpha and -Beta are for sprt mode'
}

# --- opponents from the engine list ----------------------------------------

$opponentArgs = @()
$opponentFiles = @()
if ($Mode -eq 'gauntlet') {
    $seen = @()
    foreach ($name in $opponents) {
        if ($name -ceq $TestName) { Die "the build under test has the same name as the opponent '$name'" }
        if ($seen -ccontains $name) { Die "opponent '$name' is listed twice" }
        $seen += $name
    }
    try {
        $engines = @(Get-Content -LiteralPath $EnginesJson -Raw -Encoding UTF8 | ConvertFrom-Json)
    } catch {
        Die "cannot read the engine list ${EnginesJson}: $($_.Exception.Message)"
    }
    foreach ($name in $opponents) {
        $entry = $engines | Where-Object { $_.name -ceq $name } | Select-Object -First 1
        if (-not $entry) {
            $known = (@($engines | ForEach-Object { $_.name } | Where-Object { $_ } | Sort-Object) -join ', ')
            if (-not $known) { $known = 'none' }
            Die "no engine named `"$name`" in $EnginesJson (known: $known)"
        }
        $command = [string] $entry.command
        $directory = [string] $entry.workingDirectory
        if ($directory -and $command -notmatch '[\\/]') { $command = Join-Path $directory $command }
        if (-not (Test-Path -LiteralPath $command -PathType Leaf)) {
            Die "opponent engine not found: $command (from $EnginesJson)"
        }
        $protocol = [string] $entry.protocol
        if (-not $protocol) { $protocol = 'uci' }
        $opponentArgs += @('-engine', "name=$name", "cmd=$command", "proto=$protocol")
        if ($directory) { $opponentArgs += "dir=$directory" }
        foreach ($option in @($entry.options)) {
            if ($null -eq $option -or $null -eq $option.value -or -not $option.name) { continue }
            if ($option.name -ceq 'Hash') { continue }
            $value = $option.value
            if ($value -is [bool]) { $value = $value.ToString().ToLowerInvariant() }
            $opponentArgs += "option.$($option.name)=$value"
        }
        $opponentFiles += , @($name, $command)
    }
}

# --- the tools and the files -----------------------------------------------

if (-not $Openings) { Die 'no opening file: give -Openings, or set OPENINGS in scripts/local.env' }
if (-not (Test-Path -LiteralPath $Openings -PathType Leaf)) { Die "opening file not found: $Openings" }

$cores = Get-PhysicalCores
if (-not $Concurrency) {
    if ($cores -gt 3) { $Concurrency = [string] ($cores - 2) } else { $Concurrency = '1' }
}
if (-not (Test-Whole $Concurrency) -or [long] $Concurrency -le 0) {
    Die "-Concurrency must be a whole number above zero, got '$Concurrency'"
}
$Concurrency = [string] [long] $Concurrency
if ($cores -gt 0 -and [long] $Concurrency -gt $cores) {
    Die "-Concurrency $Concurrency is more than the $cores physical cores of this machine; games would steal time from each other"
}

if (-not $Cutechess) { $Cutechess = 'cutechess-cli' }
$needed = @('-engine', '-each', '-openings', '-games', '-rounds', '-repeat', '-recover',
    '-concurrency', '-pgnout', '-ratinginterval', '-resign', '-draw')
if ($Mode -eq 'sprt') { $needed += '-sprt' }
if ($Mode -eq 'gauntlet') { $needed += @('-tournament', '-seeds') }
if ($Seed) { $needed += '-srand' }
$tool = Get-Command -Name $Cutechess -ErrorAction SilentlyContinue | Select-Object -First 1
if ($tool) {
    $helpText = (Invoke-Tool $Cutechess @('-help')) -join "`n"
    foreach ($flag in $needed) {
        if ($helpText -notmatch "(?m)^\s*$([regex]::Escape($flag))\b") {
            Die "this cutechess-cli ($Cutechess) does not list the option $flag in its -help; the script needs it"
        }
    }
} elseif ($DryRun) {
    [Console]::Error.WriteLine("match.ps1: warning: cutechess-cli not found ($Cutechess); its options were not checked")
} else {
    Die "cutechess-cli not found ($Cutechess). Install it (docs/TESTING.md), or give -Cutechess or CUTECHESS_CLI in scripts/local.env"
}

# --- the command -----------------------------------------------------------

$date = $env:VYVRAEL_MATCH_DATE
if (-not $date) { $date = Get-Date -Format 'yyyy-MM-dd' }
$slug = $TestName -replace '[^A-Za-z0-9._-]', '-'
$stemName = "$date-$Mode-$slug"
if ($Label) { $stemName += "-$Label" }
$stem = Join-Path $OutDir $stemName
$pgn = "$stem.pgn"

$buildOptions = @('proto=uci', 'option.Threads=1')
# A fixed seed (the project's default, 0x31031996) and no variety.
if ($Deterministic) { $buildOptions += @('option.Seed=822286742', 'option.Variety=0') }

$arguments = @()
if ($Mode -eq 'gauntlet') { $arguments += @('-tournament', 'gauntlet', '-seeds', '1') }
$testExtra = @($testOptions | ForEach-Object { "option.$_" })
$baseExtra = @($baseOptions | ForEach-Object { "option.$_" })
$arguments += @('-engine', "name=$TestName", "cmd=$Test") + $buildOptions + $testExtra
if ($Mode -eq 'gauntlet') {
    $arguments += $opponentArgs
} else {
    $arguments += @('-engine', "name=$BaseName", "cmd=$Base") + $buildOptions + $baseExtra
}
$arguments += @('-each', "tc=$Tc", "option.Hash=$Hash",
    '-openings', "file=$Openings", 'format=epd', 'order=random',
    '-games', '2', '-repeat', '-rounds', $Rounds,
    '-resign', 'movecount=3', 'score=700', 'twosided=true',
    '-draw', 'movenumber=34', 'movecount=10', 'score=20',
    '-recover', '-concurrency', $Concurrency, '-ratinginterval', '10')
if ($Mode -eq 'sprt') { $arguments += @('-sprt', "elo0=$Elo0", "elo1=$Elo1", "alpha=$Alpha", "beta=$Beta") }
if ($Seed) { $arguments += @('-srand', $Seed) }
$arguments += @('-pgnout', $pgn)

# The command as one line, with every argument that holds a space quoted.
function Format-Command([string[]] $Words) {
    return (($Words | ForEach-Object { if ($_ -match '\s') { "`"$_`"" } else { $_ } }) -join ' ')
}
$commandLine = Format-Command (@($Cutechess) + $arguments)

if ($DryRun) {
    if ($low -eq 'true') { Write-Output "# at BelowNormal priority" }
    Write-Output $commandLine
    exit 0
}

try {
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
} catch {
    Die "cannot create the output folder $OutDir"
}
if (Test-Path -LiteralPath $pgn) {
    Die "$pgn already exists; games would be mixed. Move it away or give another -Label"
}

# --- run notes, the log and the result --------------------------------------

# .NET file calls resolve against the process directory, which need not be
# PowerShell's current location, so the paths are made absolute first.
function Get-FullPath([string] $Path) {
    return $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
}
$log = Get-FullPath "$stem.log"
$notes = Get-FullPath "$stem.notes.txt"
$utf8 = New-Object System.Text.UTF8Encoding($false)

function Get-Digest([string] $Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}
function Get-Now { return [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ') }

# The commit of the repository this script belongs to, and whether its
# working tree has changes: read-only git, nothing written.
function Get-RepoState {
    $repoDir = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    $git = $env:VYVRAEL_GIT
    if (-not $git) { $git = 'git' }
    $unknown = 'unknown (no git, or not a git repository)'
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $commit = & $git -C $repoDir rev-parse HEAD 2>$null
        if ($LASTEXITCODE -ne 0 -or -not $commit) { return $unknown }
        $changes = & $git -C $repoDir status --porcelain 2>$null
        if ($LASTEXITCODE -ne 0) { return $unknown }
        if ($changes) { return "$commit (dirty)" }
        return "$commit (clean)"
    } catch {
        return $unknown
    } finally {
        $ErrorActionPreference = $saved
    }
}

# The engine list a gauntlet used, copied beside the notes, so the
# opponents' options stay on record even if the list changes later.
$enginesCopy = ''
if ($Mode -eq 'gauntlet') {
    $enginesCopy = Get-FullPath "$stem.engines.json"
    try {
        Copy-Item -LiteralPath $EnginesJson -Destination $enginesCopy -Force
    } catch {
        Die "cannot copy the engine list to $enginesCopy"
    }
}
$enginesLine = 'engines:    no engine list used'
if ($enginesCopy) { $enginesLine = "engines:    $EnginesJson (copied to $enginesCopy)" }

$version = (Invoke-Tool $Cutechess @('-version') | Select-Object -First 1)
$lines = @(
    'Vyvrael match run notes',
    "mode:       $Mode",
    "started:    $(Get-Now)",
    "repository: $(Get-RepoState)",
    $enginesLine,
    "host cores: $cores physical, $Concurrency games at once",
    "cutechess:  $version",
    "openings:   $Openings",
    "pgn:        $pgn",
    '',
    'engines (name, file, SHA-256):',
    "  $TestName", "    $Test", "    $(Get-Digest $Test)"
)
if ($Mode -eq 'gauntlet') {
    foreach ($pair in $opponentFiles) {
        $lines += @("  $($pair[0])", "    $($pair[1])", "    $(Get-Digest $pair[1])")
    }
} else {
    $lines += @("  $BaseName", "    $Base", "    $(Get-Digest $Base)")
}
# The options given to a build, "; " between them, or "none".
function Format-Options([string[]] $Options) {
    if ($Options.Count -eq 0) { return 'none' }
    return ($Options -join '; ')
}
$lines += @('', 'options given (-TestOption, -BaseOption):', "  ${TestName}: $(Format-Options $testOptions)")
if ($Mode -ne 'gauntlet') { $lines += "  ${BaseName}: $(Format-Options $baseOptions)" }
$lines += @('', 'command:', "  $commandLine")
[IO.File]::WriteAllLines($notes, [string[]] $lines, $utf8)

Write-Output "match.ps1: log $log, notes $notes"
$process = [Diagnostics.Process]::GetCurrentProcess()
$oldPriority = $process.PriorityClass
if ($low -eq 'true') {
    # The engines are started by cutechess-cli, which this shell starts:
    # they inherit a below-normal priority class.
    $process.PriorityClass = [Diagnostics.ProcessPriorityClass]::BelowNormal
}
$writer = New-Object System.IO.StreamWriter($log, $false, $utf8)
$status = 0
try {
    # See Invoke-Tool: cutechess-cli's warnings on stderr must not end the run.
    $ErrorActionPreference = 'Continue'
    & $Cutechess @arguments 2>&1 | ForEach-Object {
        $line = "$_"
        [Console]::Out.WriteLine($line)
        $writer.WriteLine($line)
        $writer.Flush()
    }
    if ($null -ne $LASTEXITCODE) { $status = $LASTEXITCODE }
} finally {
    $ErrorActionPreference = 'Stop'
    $writer.Dispose()
    $process.PriorityClass = $oldPriority
}

$outcome = 'finished'
if ($Mode -eq 'sprt') {
    $last = @([IO.File]::ReadAllLines($log) | Where-Object { $_ -cmatch '^SPRT:' }) | Select-Object -Last 1
    $llr = '?'
    if ($last -and $last -match '^SPRT: llr (\S+)') { $llr = $Matches[1] }
    if (-not $last) {
        $outcome = 'no SPRT line in the log; nothing can be concluded'
    } elseif ($last -match 'H1 was accepted') {
        $outcome = "H1 accepted: the test build gains at least elo0 (LLR $llr)"
    } elseif ($last -match 'H0 was accepted') {
        $outcome = "H0 accepted: no gain shown; the change is rejected (LLR $llr)"
    } else {
        $outcome = "undecided: the round cap was reached first (LLR $llr)"
    }
    $shown = $last
    if (-not $shown) { $shown = 'none' }
    Write-Output ''
    Write-Output "SPRT final: $shown"
    Write-Output "SPRT outcome: $outcome"
}
if ($status -ne 0) { $outcome = "cutechess-cli failed with status $status; $outcome" }
$pgnPath = Get-FullPath $pgn
if (Test-Path -LiteralPath $pgnPath -PathType Leaf) {
    $games = @([IO.File]::ReadAllLines($pgnPath) | Where-Object { $_ -cmatch '^\[Event ' }).Count
    $pgnDigest = Get-Digest $pgnPath
} else {
    $games = '0 (no PGN was written)'
    $pgnDigest = 'none'
}
[IO.File]::AppendAllLines($notes, [string[]] @('', "ended:      $(Get-Now)", "status:     $status",
        "outcome:    $outcome", "games:      $games", "pgn sha256: $pgnDigest"), $utf8)
exit $status
