#requires -Version 7.0

[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script:CleanupScriptPath = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\cleanup-stale-targets.ps1"))
$script:TestBasePath = "C:\TEMP\opencode\cleanup-stale-targets-tests"
$script:FailedChecks = 0
$script:ScenarioCleanupActions = @()
$script:SentinelProcesses = @()

$validatedBasePrefix = "C:\TEMP\opencode"
if (-not (Test-Path -LiteralPath $validatedBasePrefix -PathType Container)) {
    throw "Approved temp prefix is missing: $validatedBasePrefix"
}
$resolvedBasePrefix = ([IO.Path]::GetFullPath($validatedBasePrefix)).TrimEnd("\")
$resolvedTestBase = [IO.Path]::GetFullPath($script:TestBasePath)
if (-not $resolvedTestBase.StartsWith($resolvedBasePrefix + "\", [StringComparison]::OrdinalIgnoreCase)) {
    throw "Test base escaped approved prefix: $resolvedTestBase"
}
if (-not (Test-Path -LiteralPath $resolvedTestBase -PathType Container)) {
    New-Item -ItemType Directory -Path $resolvedTestBase | Out-Null
}

function Add-ScenarioCleanup {
    param([Parameter(Mandatory)][scriptblock]$Action)
    $script:ScenarioCleanupActions += $action
}

function Invoke-ScenarioCleanups {
    foreach ($cleanupAction in $script:ScenarioCleanupActions) {
        try {
            & $cleanupAction
        } catch {
            Write-Warning "Scenario cleanup action failed: $($_.Exception.Message)"
        }
    }
    $script:ScenarioCleanupActions = @()
}

function Assert-True {
    param(
        [Parameter(Mandatory)]
        [bool]$Condition,
        [Parameter(Mandatory)]
        [string]$Label
    )

    if ($Condition) {
        Write-Output "PASS $Label"
    } else {
        $script:FailedChecks += 1
        Write-Output "FAIL $Label"
    }
}

function Assert-ThrowsLike {
    param(
        [Parameter(Mandatory)]
        [scriptblock]$Action,
        [Parameter(Mandatory)]
        [string]$ExpectedPattern,
        [Parameter(Mandatory)]
        [string]$Label
    )

    try {
        & $Action
    } catch {
        if ($_.Exception.Message -match $ExpectedPattern) {
            Write-Output "PASS $Label"
        } else {
            $script:FailedChecks += 1
            Write-Output "FAIL $Label wrong-message=$($_.Exception.Message)"
        }
        return
    }

    $script:FailedChecks += 1
    Write-Output "FAIL $Label no-exception-thrown"
}

function New-ScenarioDirectory {
    $scenarioDirectory = Join-Path $resolvedTestBase ([Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $scenarioDirectory | Out-Null
    Add-ScenarioCleanup -Action { Remove-Item -LiteralPath $scenarioDirectory -Recurse -Force -ErrorAction SilentlyContinue }.GetNewClosure()
    return $scenarioDirectory
}

function Start-NamedSentinelProcess {
    param(
        [Parameter(Mandatory)]
        [string]$ExecutablePath,
        [Parameter(Mandatory)]
        [int]$LifetimeSeconds
    )

    $sentinelProcess = Start-Process -FilePath $ExecutablePath -ArgumentList @("/c", "ping -n $LifetimeSeconds 127.0.0.1 > NUL") -PassThru -WindowStyle Hidden
    $script:SentinelProcesses += $sentinelProcess
    Add-ScenarioCleanup -Action {
        if (-not $sentinelProcess.HasExited) {
            Stop-Process -Id $sentinelProcess.Id -Force -ErrorAction SilentlyContinue
        }
    }.GetNewClosure()
    return $sentinelProcess
}

function New-TestRepository {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot
    )

    New-Item -ItemType Directory -Path $RepositoryRoot | Out-Null
    git init -q $RepositoryRoot 2>$null
    git -C $RepositoryRoot config user.email "cleanup-test@example.invalid"
    git -C $RepositoryRoot config user.name "cleanup-test"
    Set-Content -LiteralPath (Join-Path $RepositoryRoot ".gitignore") -Value "/target/"
    Set-Content -LiteralPath (Join-Path $RepositoryRoot "seed.txt") -Value "seed"
    git -C $RepositoryRoot add -A
    git -C $RepositoryRoot commit -q -m "seed" 2>$null
    $bareUpstream = "$RepositoryRoot.bare"
    git clone -q --bare $RepositoryRoot $bareUpstream 2>$null
    git -C $RepositoryRoot remote add origin $bareUpstream
    git -C $RepositoryRoot fetch -q origin 2>$null
    $branchName = (git -C $RepositoryRoot rev-parse --abbrev-ref HEAD | Out-String).Trim()
    git -C $RepositoryRoot branch "--set-upstream-to=origin/$branchName" $branchName 2>$null | Out-Null
    return $branchName
}

function Set-AgeOld {
    param([Parameter(Mandatory)][string]$Path)
    (Get-Item -LiteralPath $Path -Force).LastWriteTime = (Get-Date).AddDays(-30)
}

function New-StaleCargoRootCandidate {
    param(
        [Parameter(Mandatory)]
        [string]$TargetRoot,
        [string]$Relative = "codex-dvc",
        [switch]$Fresh
    )

    $candidatePath = Join-Path $TargetRoot $Relative
    New-Item -ItemType Directory -Path $candidatePath -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $candidatePath "CACHEDIR.TAG") -Value "Signature: 8a477f597d28d172789f06886806bc55"
    Set-Content -LiteralPath (Join-Path $candidatePath ".rustc_info.json") -Value "{}"
    Set-Content -LiteralPath (Join-Path $candidatePath "artifact.bin") -Value "payload"

    if (-not $Fresh) {
        Set-AgeOld -Path $candidatePath
        Set-AgeOld -Path (Join-Path $candidatePath "CACHEDIR.TAG")
        Set-AgeOld -Path (Join-Path $candidatePath ".rustc_info.json")
        Set-AgeOld -Path (Join-Path $candidatePath "artifact.bin")
    }

    return $candidatePath
}

function New-StaleIncrementalCandidate {
    param(
        [Parameter(Mandatory)]
        [string]$TargetRoot,
        [switch]$Fresh
    )

    $candidatePath = Join-Path $TargetRoot "debug\incremental"
    New-Item -ItemType Directory -Path $candidatePath -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $candidatePath "incremental.dat") -Value "state"

    if (-not $Fresh) {
        Set-AgeOld -Path $candidatePath
        Set-AgeOld -Path (Join-Path $candidatePath "incremental.dat")
    }

    return $candidatePath
}

function Get-TreeSnapshot {
    param([Parameter(Mandatory)][string]$Root)

    return @(
        Get-ChildItem -LiteralPath $Root -Recurse -Force |
        Sort-Object -Property FullName |
        ForEach-Object {
            $entryLength = if ($_.PSIsContainer) { "<dir>" } else { $_.Length }
            "{0}|{1}|{2}" -f $_.FullName, $entryLength, $_.LastWriteTimeUtc.Ticks
        }
    )
}

function Invoke-CleanupRun {
    param([string[]]$RunArguments)

    $rawOutput = & pwsh -NoProfile -File $script:CleanupScriptPath @RunArguments 2>&1
    $runExitCode = $LASTEXITCODE
    $outputLines = @($rawOutput | ForEach-Object { $_.ToString() })
    $outcomeLines = @($outputLines | Where-Object { $_ -like "CLEANUP-OUTCOME *" })
    $outcome = $null
    if ($outcomeLines.Count -eq 1) {
        $outcome = ($outcomeLines[0].Substring("CLEANUP-OUTCOME ".Length) | ConvertFrom-Json)
    }
    return [pscustomobject]@{
        Code = $runExitCode
        Output = $outputLines
        Outcome = $outcome
    }
}

function Get-NewestQuarantineRun {
    param([Parameter(Mandatory)][string]$TargetRoot)

    $quarantineBase = Join-Path $TargetRoot ".cleanup-quarantine"
    if (-not (Test-Path -LiteralPath $quarantineBase -PathType Container)) {
        return $null
    }

    $runs = @(Get-ChildItem -LiteralPath $quarantineBase -Directory | Sort-Object -Property Name)
    if ($runs.Count -eq 0) {
        return $null
    }

    return $runs[-1].FullName
}

Write-Output "=== parser checks ==="
foreach ($parseTarget in @($script:CleanupScriptPath, $PSCommandPath)) {
    $parseTokens = $null
    $parseErrors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile($parseTarget, [ref]$parseTokens, [ref]$parseErrors)
    Assert-True (@($parseErrors).Count -eq 0) "parser-clean $parseTarget"
}

Write-Output "=== dot-source FunctionsOnly ==="
. $script:CleanupScriptPath -FunctionsOnly
Assert-True ($null -ne (Get-Command Get-CleanupEntryObservation -ErrorAction SilentlyContinue)) "functions-only exposes observational handle metadata"
foreach ($requiredWriter in @("cargo.exe", "rustc.exe", "link.exe", "lld-link.exe", "mspdbsrv.exe", "msbuild.exe", "cl.exe", "dotnet.exe", "node.exe", "pnpm.exe", "esbuild.exe", "syndocal.exe")) {
    Assert-True ($script:DefaultWriterProcessNames -contains $requiredWriter) "default writer list contains $requiredWriter"
}

Write-Output "=== observational handles are not authorization ==="
$observationScenario = New-ScenarioDirectory
$observationDir = Join-Path $observationScenario "candidate"
New-Item -ItemType Directory -Path $observationDir | Out-Null
$observationA = Get-CleanupEntryObservation -Path $observationDir
$observationB = Get-CleanupEntryObservation -Path $observationDir
Assert-True ($observationA -ceq $observationB) "same live path has a stable observation"
Assert-True (-not (Get-CleanupExecutionCapability).Enabled) "execution capability is intentionally disabled"
Assert-ThrowsLike -Action {
    Assert-CleanupExecutionCapability -Operation "test destructive operation"
} -ExpectedPattern "disabled fail-closed" -Label "no command-line or function bypass enables execution"

Write-Output "=== lexical and protected-path boundaries ==="
$boundaryScenario = New-ScenarioDirectory
$boundaryRoot = Join-Path $boundaryScenario "repo-target"
New-Item -ItemType Directory -Path $boundaryRoot | Out-Null
$boundarySibling = Join-Path $boundaryScenario "repo-target-sibling\child"
New-Item -ItemType Directory -Path $boundarySibling -Force | Out-Null
Assert-ThrowsLike -Action {
    Assert-CleanupAncestryNoReparse -Root $boundaryRoot -Leaf $boundarySibling
} -ExpectedPattern "not physically contained" -Label "lexical-prefix sibling leaf rejected"
New-Item -ItemType Directory -Path (Join-Path $boundaryRoot "release\sub") -Force | Out-Null
Assert-True ($null -ne (Test-CleanupProtectedOverlap -TargetRoot $boundaryRoot -CandidatePath (Join-Path $boundaryRoot "release") -ProtectedPaths @("release"))) "protected exact match detected"
Assert-True ($null -ne (Test-CleanupProtectedOverlap -TargetRoot $boundaryRoot -CandidatePath (Join-Path $boundaryRoot "release\sub") -ProtectedPaths @("release"))) "protected descendant match detected"
Assert-True ($null -eq (Test-CleanupProtectedOverlap -TargetRoot $boundaryRoot -CandidatePath (Join-Path $boundaryRoot "release-x") -ProtectedPaths @("release"))) "lexical prefix sibling not treated as protected overlap"

try {
    Write-Output "=== deterministic deep-descendant staleness ==="
    $deepScenario = New-ScenarioDirectory
    $deepRepo = Join-Path $deepScenario "repo"
    [void](New-TestRepository -RepositoryRoot $deepRepo)
    $deepTarget = Join-Path $deepRepo "target"
    $deepCandidate = New-StaleCargoRootCandidate -TargetRoot $deepTarget -Relative "audio-qa-final"
    $deepFreshChild = Join-Path $deepCandidate "nested\a\b\fresh.txt"
    New-Item -ItemType Directory -Path (Split-Path -Parent $deepFreshChild) -Force | Out-Null
    Set-Content -LiteralPath $deepFreshChild -Value "fresh"
    $deepNewest = Get-CleanupNewestWriteTime -Path $deepCandidate
    Assert-True ($deepNewest.ToUniversalTime().Ticks -eq (Get-Item -LiteralPath $deepFreshChild -Force).LastWriteTime.ToUniversalTime().Ticks) "newest write includes deterministic deep fresh descendant"
    $deepRun = Invoke-CleanupRun -RunArguments @("-RepositoryRoot", $deepRepo, "-TargetRoot", $deepTarget)
    Assert-True ($deepRun.Code -eq 0) "deep-descendant dry-run exits zero"
    Assert-True ($null -ne $deepRun.Outcome) "dry-run emits exactly one structured outcome"
    Assert-True ($deepRun.Outcome.Outcome -ceq "PlanOnly") "dry-run outcome is PlanOnly"
    $deepEntry = @($deepRun.Outcome.Entries | Where-Object { $_.Relative -ceq "audio-qa-final" })[0]
    Assert-True ($deepEntry.Status -ceq "SkippedNotProvablyStale") "fresh deep descendant blocks stale plan"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $deepTarget ".cleanup-quarantine"))) "dry-run created no quarantine"

    Write-Output "=== x86 target triple is not eligible ==="
    $tripleScenario = New-ScenarioDirectory
    $tripleRepo = Join-Path $tripleScenario "repo"
    [void](New-TestRepository -RepositoryRoot $tripleRepo)
    $tripleTarget = Join-Path $tripleRepo "target"
    New-Item -ItemType Directory -Path (Join-Path $tripleTarget "x86_64-pc-windows-msvc\release") -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $tripleTarget "x86_64-pc-windows-msvc\release\syndocal.exe") -Value "current-artifact"
    Set-AgeOld -Path (Join-Path $tripleTarget "x86_64-pc-windows-msvc")
    $tripleRun = Invoke-CleanupRun -RunArguments @("-RepositoryRoot", $tripleRepo, "-TargetRoot", $tripleTarget)
    Assert-True ($tripleRun.Code -eq 0) "triple-only dry-run exits zero"
    Assert-True ((@($tripleRun.Outcome.Entries | Where-Object { $_.Relative -match "x86_64" }).Count) -eq 0) "x86_64-pc-windows-msvc is absent from the recurring candidate set"
    Assert-True (Test-Path -LiteralPath (Join-Path $tripleTarget "x86_64-pc-windows-msvc\release\syndocal.exe")) "current triple artifact remains untouched"

    Write-Output "=== snapshot negative checks without FileIndex-reuse claims ==="
    $safetyScenario = New-ScenarioDirectory
    $safetyTarget = Join-Path $safetyScenario "target"
    $safetyCandidate = New-StaleCargoRootCandidate -TargetRoot $safetyTarget -Relative "codex-dvc"
    $safetySnapshot = Get-CleanupCandidateSafetySnapshot -TargetRoot $safetyTarget -CandidatePath $safetyCandidate
    $rootMismatch = $safetySnapshot.psobject.Copy()
    $rootMismatch.TargetRootObservation = "forced-root-mismatch"
    Assert-ThrowsLike -Action {
        Assert-CleanupCandidateSafetySnapshotMatches -ExpectedSnapshot $rootMismatch
    } -ExpectedPattern "Target root observation changed" -Label "root observation mismatch denies lifecycle"
    $candidateMismatch = $safetySnapshot.psobject.Copy()
    $candidateMismatch.CandidateObservation = "forced-candidate-mismatch"
    Assert-ThrowsLike -Action {
        Assert-CleanupCandidateSafetySnapshotMatches -ExpectedSnapshot $candidateMismatch
    } -ExpectedPattern "Candidate observation changed" -Label "candidate observation mismatch denies lifecycle"
    Remove-Item -LiteralPath $safetyCandidate -Recurse -Force
    [void](New-StaleCargoRootCandidate -TargetRoot $safetyTarget -Relative "codex-dvc")
    $swapMismatch = $safetySnapshot.psobject.Copy()
    $swapMismatch.CandidateObservation = "forced-swap-mismatch-no-fileindex-reuse-assumption"
    Assert-ThrowsLike -Action {
        Assert-CleanupCandidateSafetySnapshotMatches -ExpectedSnapshot $swapMismatch
    } -ExpectedPattern "Candidate observation changed" -Label "candidate replacement has an explicit negative path without claiming FileIndex uniqueness"
    $candidateOutside = Join-Path $safetyScenario "outside-candidate"
    New-Item -ItemType Directory -Path $candidateOutside | Out-Null
    Set-Content -LiteralPath (Join-Path $candidateOutside "sentinel.txt") -Value "KEEP"
    Remove-Item -LiteralPath $safetyCandidate -Recurse -Force
    New-Item -ItemType Junction -Path $safetyCandidate -Target $candidateOutside | Out-Null
    Assert-ThrowsLike -Action {
        Get-CleanupCandidateSafetySnapshot -TargetRoot $safetyTarget -CandidatePath $safetyCandidate
    } -ExpectedPattern "reparse point" -Label "candidate reparse point denies snapshot"
    Assert-True ((Get-Content -LiteralPath (Join-Path $candidateOutside "sentinel.txt") -Raw).Trim() -ceq "KEEP") "candidate reparse destination remains untouched"
    $rootReparseScenario = New-ScenarioDirectory
    $rootOutside = Join-Path $rootReparseScenario "outside-root"
    New-Item -ItemType Directory -Path (Join-Path $rootOutside "codex-dvc") -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $rootOutside "sentinel.txt") -Value "KEEP"
    $rootJunction = Join-Path $rootReparseScenario "target"
    New-Item -ItemType Junction -Path $rootJunction -Target $rootOutside | Out-Null
    Assert-ThrowsLike -Action {
        Get-CleanupCandidateSafetySnapshot -TargetRoot $rootJunction -CandidatePath (Join-Path $rootJunction "codex-dvc")
    } -ExpectedPattern "Target root is a reparse point" -Label "root reparse point denies snapshot"
    Assert-True ((Get-Content -LiteralPath (Join-Path $rootOutside "sentinel.txt") -Raw).Trim() -ceq "KEEP") "root reparse destination remains untouched"

    Write-Output "=== lifecycle and purge cannot delete ==="
    $guardScenario = New-ScenarioDirectory
    $guardTarget = Join-Path $guardScenario "target"
    $guardCandidate = New-StaleCargoRootCandidate -TargetRoot $guardTarget -Relative "codex-dvc"
    $guardSnapshot = Get-CleanupCandidateSafetySnapshot -TargetRoot $guardTarget -CandidatePath $guardCandidate
    $guardPlan = [pscustomobject]@{
        Path = $guardCandidate
        FlattenName = "codex-dvc"
        SafetySnapshot = $guardSnapshot
    }
    $guardQuarantine = Join-Path $guardTarget ".test-quarantine"
    New-Item -ItemType Directory -Path $guardQuarantine | Out-Null
    Assert-ThrowsLike -Action {
        Invoke-CleanupCandidateLifecycle -PlanEntry $guardPlan -QuarantineRunDir $guardQuarantine -TargetRoot $guardTarget
    } -ExpectedPattern "Candidate quarantine lifecycle is disabled" -Label "lifecycle rejects even direct function calls"
    Assert-True (Test-Path -LiteralPath (Join-Path $guardCandidate "CACHEDIR.TAG")) "direct lifecycle rejection preserves source candidate"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $guardQuarantine "codex-dvc"))) "direct lifecycle rejection creates no quarantine candidate"
    Assert-ThrowsLike -Action {
        Invoke-GuardedQuarantinePurge -Path $guardCandidate -FailureInjection "BeforeFirstRemoval"
    } -ExpectedPattern "Injected quarantine purge failure" -Label "deterministic purge failure injection preserves recovery tree"
    Assert-True (Test-Path -LiteralPath (Join-Path $guardCandidate "artifact.bin")) "failure injection preserved all candidate data"

    Write-Output "=== execute request is structured and blocked ==="
    $executeScenario = New-ScenarioDirectory
    $executeRepo = Join-Path $executeScenario "repo"
    [void](New-TestRepository -RepositoryRoot $executeRepo)
    $executeTarget = Join-Path $executeRepo "target"
    [void](New-StaleCargoRootCandidate -TargetRoot $executeTarget -Relative "codex-dvc")
    $executeRun = Invoke-CleanupRun -RunArguments @("-Execute", "-RepositoryRoot", $executeRepo, "-TargetRoot", $executeTarget)
    Assert-True ($executeRun.Code -ne 0) "execute request exits nonzero while safety primitive is absent"
    Assert-True ($null -ne $executeRun.Outcome) "execute request emits structured outcome before error text"
    Assert-True ($executeRun.Outcome.Outcome -ceq "ExecutionBlocked") "structured outcome records ExecutionBlocked"
    Assert-True (-not [bool]$executeRun.Outcome.ExecutionCapability.Enabled) "structured outcome records disabled capability"
    Assert-True ($executeRun.Outcome.PlannedCount -eq 1) "structured outcome carries planned entry count"
    Assert-True ((@($executeRun.Outcome.Entries | Where-Object { $_.Status -ceq "Planned" }).Count) -eq 1) "structured outcome carries planned entry status"
    Assert-True (Test-Path -LiteralPath (Join-Path $executeTarget "codex-dvc\CACHEDIR.TAG")) "blocked execute leaves candidate intact"
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $executeTarget ".cleanup-quarantine"))) "blocked execute creates no quarantine"
    Assert-True ((@($executeRun.Output | Where-Object { $_ -match "Execution is intentionally disabled" }).Count) -ge 1) "secondary diagnostic explains disabled execution"

    Write-Output "=== Git checkpoint and writer boundaries remain fail-closed ==="
    $gitScenario = New-ScenarioDirectory
    $gitRepo = Join-Path $gitScenario "repo"
    [void](New-TestRepository -RepositoryRoot $gitRepo)
    $checkpoint = Assert-CleanupGitCheckpoint -RepositoryRoot $gitRepo
    Assert-True ($checkpoint.Head.Length -eq 40) "clean checkpoint returns structured head"
    Set-Content -LiteralPath (Join-Path $gitRepo "untracked.txt") -Value "dirty"
    Assert-ThrowsLike -Action {
        Assert-CleanupGitCheckpoint -RepositoryRoot $gitRepo
    } -ExpectedPattern "not clean" -Label "dirty Git worktree is denied before any future execution"
    Remove-Item -LiteralPath (Join-Path $gitRepo "untracked.txt") -Force
    $sentinelName = "kdmx-cleanup-sentinel-$([Guid]::NewGuid().ToString("N").Substring(0, 8)).exe"
    $sentinelExe = Join-Path $gitScenario $sentinelName
    Copy-Item -LiteralPath "$env:SystemRoot\System32\cmd.exe" -Destination $sentinelExe
    [void](Start-NamedSentinelProcess -ExecutablePath $sentinelExe -LifetimeSeconds 30)
    Assert-ThrowsLike -Action {
        Assert-CleanupNoActiveWriters -Names @($sentinelName) -ForbiddenRoots @($gitRepo)
    } -ExpectedPattern "Active build/tool process" -Label "active writer process is denied before any future execution"
} finally {
    Invoke-ScenarioCleanups
}

Write-Output "=== summary ==="
Write-Output "FAILED=$($script:FailedChecks)"
if ($script:FailedChecks -gt 0) {
    exit 1
}
exit 0
