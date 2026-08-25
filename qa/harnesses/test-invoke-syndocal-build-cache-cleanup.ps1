[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script:AssertionCount = 0
$script:TestRunRoot = $null
$script:FixtureRoots = New-Object System.Collections.ArrayList

function Assert-True {
    param(
        [Parameter(Mandatory)]
        [bool]$Condition,
        [Parameter(Mandatory)]
        [string]$Message
    )

    if (-not $Condition) {
        throw "ASSERTION FAILED: $Message"
    }
    $script:AssertionCount++
}

function Assert-Equal {
    param(
        [Parameter(Mandatory)]
        $Expected,
        [Parameter(Mandatory)]
        $Actual,
        [Parameter(Mandatory)]
        [string]$Message
    )

    if ($Expected -cne $Actual) {
        throw "ASSERTION FAILED: $Message expected='$Expected' actual='$Actual'"
    }
    $script:AssertionCount++
}

function Assert-GateCode {
    param(
        [Parameter(Mandatory)]
        [string]$ExpectedCode,
        [Parameter(Mandatory)]
        [scriptblock]$Action,
        [Parameter(Mandatory)]
        [string]$Message
    )

    $caught = $null
    try {
        & $Action
    } catch {
        $caught = $_
    }
    Assert-True -Condition ($null -ne $caught) -Message "$Message throws"
    $actualCode = if ($null -eq $caught) { "" } else { [string]$caught.Exception.Data["SyndocalCleanupCode"] }
    Assert-Equal -Expected $ExpectedCode -Actual $actualCode -Message "$Message returns its typed code"
}

function Invoke-TestGit {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory)]
        [string[]]$Arguments
    )

    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $output = @(& git -C $RepositoryRoot @Arguments 2>&1)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($exitCode -ne 0) {
        throw "Test Git command failed in '$RepositoryRoot': git $($Arguments -join ' ') :: $($output -join ' ')"
    }
    return @($output)
}

function New-CleanupTestFixture {
    param(
        [switch]$WithoutCandidate
    )

    $nonce = [Guid]::NewGuid().ToString("N")
    $repositoryRoot = Join-Path $script:TestRunRoot ("syndocal-cleanup-selftest-" + $nonce)
    $originRoot = Join-Path $script:TestRunRoot ("origin-" + $nonce + ".git")
    [void](New-Item -ItemType Directory -Path $repositoryRoot -Force)
    [void](New-Item -ItemType Directory -Path (Join-Path $repositoryRoot "target") -Force)
    if (-not $WithoutCandidate) {
        $candidate = Join-Path $repositoryRoot "target/debug/incremental"
        [void](New-Item -ItemType Directory -Path $candidate -Force)
        [IO.File]::WriteAllBytes((Join-Path $candidate "cache.bin"), [byte[]](1..64))
    } else {
        $candidate = Join-Path $repositoryRoot "target/debug/incremental"
    }

    $marker = [ordered]@{
        schemaVersion = 1
        nonce = $nonce
        repositoryRoot = [IO.Path]::GetFullPath($repositoryRoot)
    }
    $marker | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $repositoryRoot ".syndocal-cleanup-selftest.json") -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $repositoryRoot ".gitignore") -Value "/target/" -Encoding ASCII
    Set-Content -LiteralPath (Join-Path $repositoryRoot "README.test") -Value "cleanup fixture" -Encoding ASCII

    & git init --bare $originRoot *> $null
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to initialize cleanup test bare origin."
    }
    & git init -b main $repositoryRoot *> $null
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to initialize cleanup test repository."
    }
    [void](Invoke-TestGit -RepositoryRoot $repositoryRoot -Arguments @("config", "user.name", "Syndocal Cleanup Test"))
    [void](Invoke-TestGit -RepositoryRoot $repositoryRoot -Arguments @("config", "user.email", "cleanup-test@example.invalid"))
    [void](Invoke-TestGit -RepositoryRoot $repositoryRoot -Arguments @("add", ".gitignore", "README.test", ".syndocal-cleanup-selftest.json"))
    [void](Invoke-TestGit -RepositoryRoot $repositoryRoot -Arguments @("commit", "-m", "fixture"))
    [void](Invoke-TestGit -RepositoryRoot $repositoryRoot -Arguments @("remote", "add", "origin", $originRoot))
    [void](Invoke-TestGit -RepositoryRoot $repositoryRoot -Arguments @("push", "-u", "origin", "main"))

    [void]$script:FixtureRoots.Add($repositoryRoot)
    return [pscustomobject]@{
        RepositoryRoot = [IO.Path]::GetFullPath($repositoryRoot)
        TargetRoot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot "target"))
        Candidate = [IO.Path]::GetFullPath($candidate)
        OriginRoot = [IO.Path]::GetFullPath($originRoot)
        Nonce = $nonce
    }
}

function Invoke-FixtureCleanup {
    param(
        [Parameter(Mandatory)]
        $Fixture,
        [switch]$Apply,
        [hashtable]$Hooks
    )

    if ($null -eq $Hooks) {
        $Hooks = @{
            CimProvider = { @() }
            StabilityDelayMilliseconds = 0
        }
    } elseif (-not $Hooks.ContainsKey("StabilityDelayMilliseconds")) {
        $Hooks["StabilityDelayMilliseconds"] = 0
    }
    return Invoke-SyndocalBuildCacheCleanupCore `
        -RepositoryRoot $Fixture.RepositoryRoot `
        -Apply:$Apply `
        -TestSeam `
        -TestSeamNonce $Fixture.Nonce `
        -TestHooks $Hooks
}

$harnessPath = Join-Path $PSScriptRoot "invoke-syndocal-build-cache-cleanup.ps1"
try {
    foreach ($parsePath in @($harnessPath, $PSCommandPath)) {
        $tokens = $null
        $errors = $null
        [void][System.Management.Automation.Language.Parser]::ParseFile($parsePath, [ref]$tokens, [ref]$errors)
        Assert-Equal -Expected 0 -Actual $errors.Count -Message "PowerShell parser accepts $([IO.Path]::GetFileName($parsePath))"
    }

    . $harnessPath -FunctionsOnly

    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd("\")
    $script:TestRunRoot = Join-Path $tempRoot ("syndocal-cleanup-suite-" + [Guid]::NewGuid().ToString("N"))
    [void](New-Item -ItemType Directory -Path $script:TestRunRoot -Force)
    Assert-True -Condition ((Get-SyndocalCleanupFullPath -Path $script:TestRunRoot).StartsWith($tempRoot + "\", [StringComparison]::OrdinalIgnoreCase)) -Message "suite root is under the resolved Windows temp root"

    Assert-Equal -Expected 1 -Actual $script:SyndocalCleanupCandidateSpecs.Count -Message "closed candidate enum has exactly one reviewed entry"
    Assert-Equal -Expected "target/debug/incremental" -Actual $script:SyndocalCleanupCandidateSpecs[0].RelativePath -Message "the sole reviewed candidate is debug incremental"
    Assert-Equal -Expected 0 -Actual $script:SyndocalCleanupExactAsioAllowlist.Count -Message "ASIO exact allowlist remains empty until separately reviewed"

    $boundaryFixture = New-CleanupTestFixture
    Assert-GateCode -ExpectedCode "CandidatePathTraversal" -Message "parent traversal is rejected" -Action {
        Resolve-SyndocalCleanupStaticCandidate -RepositoryRoot $boundaryFixture.RepositoryRoot -RelativePath "target/../release"
    }
    Assert-GateCode -ExpectedCode "CandidatePathTraversal" -Message "UNC input is rejected" -Action {
        Resolve-SyndocalCleanupStaticCandidate -RepositoryRoot $boundaryFixture.RepositoryRoot -RelativePath "\\server\share\target"
    }
    Assert-GateCode -ExpectedCode "CandidateNotAllowlisted" -Message "an arbitrary target child is rejected" -Action {
        Resolve-SyndocalCleanupStaticCandidate -RepositoryRoot $boundaryFixture.RepositoryRoot -RelativePath "target/debug"
    }
    Assert-GateCode -ExpectedCode "BroadTargetForbidden" -Message "the target root is never deletable" -Action {
        Assert-SyndocalCleanupProtectedBoundary -RepositoryRoot $boundaryFixture.RepositoryRoot -CandidatePath $boundaryFixture.TargetRoot
    }
    $protectedRelease = Join-Path $boundaryFixture.RepositoryRoot "target/release"
    Assert-GateCode -ExpectedCode "ProtectedPathOverlap" -Message "release is protected even when absent" -Action {
        Assert-SyndocalCleanupProtectedBoundary -RepositoryRoot $boundaryFixture.RepositoryRoot -CandidatePath $protectedRelease
    }
    $protectedDepsParent = Join-Path $boundaryFixture.RepositoryRoot "target/debug"
    Assert-GateCode -ExpectedCode "ProtectedPathOverlap" -Message "a parent that would consume protected deps is rejected" -Action {
        Assert-SyndocalCleanupProtectedBoundary -RepositoryRoot $boundaryFixture.RepositoryRoot -CandidatePath $protectedDepsParent
    }
    Assert-GateCode -ExpectedCode "TestSeamRootForbidden" -Message "test Apply seam rejects the real checkout" -Action {
        Assert-SyndocalCleanupTestSeamRoot -RepositoryRoot ([IO.Path]::GetFullPath((Join-Path $PSScriptRoot "../.."))) -Nonce "hostile"
    }
    Assert-GateCode -ExpectedCode "TestSeamMarkerMismatch" -Message "test Apply seam rejects a wrong nonce" -Action {
        Assert-SyndocalCleanupTestSeamRoot -RepositoryRoot $boundaryFixture.RepositoryRoot -Nonce "wrong"
    }

    $planFixture = New-CleanupTestFixture
    $plan = Invoke-FixtureCleanup -Fixture $planFixture
    Assert-Equal -Expected "PlanReady" -Actual $plan.Outcome -Message "default mode produces a ready plan"
    Assert-Equal -Expected "Plan" -Actual $plan.Mode -Message "default mode is Plan"
    Assert-Equal -Expected 64 -Actual ([int64]$plan.PlannedLogicalBytes) -Message "plan records exact logical bytes"
    Assert-True -Condition (Test-Path -LiteralPath $planFixture.Candidate -PathType Container) -Message "Plan never deletes its candidate"
    Assert-Equal -Expected "Planned" -Actual $plan.Candidates[0].Status -Message "plan candidate status is typed"
    Assert-True -Condition (-not [string]::IsNullOrWhiteSpace([string]$plan.Candidates[0].FingerprintSha256)) -Message "plan records a stable identity fingerprint"

    $missingFixture = New-CleanupTestFixture -WithoutCandidate
    $missingPlan = Invoke-FixtureCleanup -Fixture $missingFixture
    Assert-Equal -Expected "PlanEmpty" -Actual $missingPlan.Outcome -Message "a missing fixed candidate is a typed empty plan"
    Assert-Equal -Expected "Missing" -Actual $missingPlan.Candidates[0].Status -Message "missing candidate is recorded rather than synthesized"

    $unknownFixture = New-CleanupTestFixture
    [void](New-Item -ItemType Directory -Path (Join-Path $unknownFixture.TargetRoot "asio-audit-unreviewed") -Force)
    $unknownReport = Invoke-FixtureCleanup -Fixture $unknownFixture
    Assert-Equal -Expected "Blocked" -Actual $unknownReport.Outcome -Message "unknown ASIO name blocks the complete plan"
    Assert-Equal -Expected "UnknownAsioCandidate" -Actual $unknownReport.Blocker.Code -Message "unknown ASIO name has a typed blocker"
    Assert-True -Condition (Test-Path -LiteralPath $unknownFixture.Candidate -PathType Container) -Message "unknown ASIO block leaves the reviewed candidate untouched"

    $dirtyFixture = New-CleanupTestFixture
    Add-Content -LiteralPath (Join-Path $dirtyFixture.RepositoryRoot "README.test") -Value "dirty"
    $dirtyReport = Invoke-FixtureCleanup -Fixture $dirtyFixture -Apply
    Assert-Equal -Expected "Blocked" -Actual $dirtyReport.Outcome -Message "dirty checkout blocks Apply"
    Assert-Equal -Expected "DirtyWorktree" -Actual $dirtyReport.Blocker.Code -Message "dirty checkout has a typed blocker"
    Assert-True -Condition (Test-Path -LiteralPath $dirtyFixture.Candidate) -Message "dirty checkout is not deleted"

    $divergedFixture = New-CleanupTestFixture
    Add-Content -LiteralPath (Join-Path $divergedFixture.RepositoryRoot "README.test") -Value "local-only"
    [void](Invoke-TestGit -RepositoryRoot $divergedFixture.RepositoryRoot -Arguments @("add", "README.test"))
    [void](Invoke-TestGit -RepositoryRoot $divergedFixture.RepositoryRoot -Arguments @("commit", "-m", "local divergence"))
    $divergedReport = Invoke-FixtureCleanup -Fixture $divergedFixture -Apply
    Assert-Equal -Expected "Blocked" -Actual $divergedReport.Outcome -Message "unpushed HEAD blocks Apply"
    Assert-Equal -Expected "Diverged" -Actual $divergedReport.Blocker.Code -Message "unpushed HEAD has a typed blocker"
    Assert-True -Condition (Test-Path -LiteralPath $divergedFixture.Candidate) -Message "diverged checkout is not deleted"

    $trackedFixture = New-CleanupTestFixture
    [void](Invoke-TestGit -RepositoryRoot $trackedFixture.RepositoryRoot -Arguments @("add", "-f", "target/debug/incremental/cache.bin"))
    [void](Invoke-TestGit -RepositoryRoot $trackedFixture.RepositoryRoot -Arguments @("commit", "-m", "hostile tracked cache"))
    [void](Invoke-TestGit -RepositoryRoot $trackedFixture.RepositoryRoot -Arguments @("push"))
    $trackedReport = Invoke-FixtureCleanup -Fixture $trackedFixture -Apply
    Assert-Equal -Expected "Blocked" -Actual $trackedReport.Outcome -Message "tracked content blocks Apply"
    Assert-Equal -Expected "TrackedContentDetected" -Actual $trackedReport.Blocker.Code -Message "tracked content has a typed blocker"
    Assert-True -Condition (Test-Path -LiteralPath $trackedFixture.Candidate) -Message "tracked candidate is not deleted"

    $cimFailureFixture = New-CleanupTestFixture
    $cimFailureReport = Invoke-FixtureCleanup -Fixture $cimFailureFixture -Apply -Hooks @{
        CimProvider = { throw "injected CIM denial" }
        StabilityDelayMilliseconds = 0
    }
    Assert-Equal -Expected "Blocked" -Actual $cimFailureReport.Outcome -Message "CIM failure blocks Apply"
    Assert-Equal -Expected "CimUnavailable" -Actual $cimFailureReport.Blocker.Code -Message "CIM failure has a typed blocker"
    Assert-True -Condition (Test-Path -LiteralPath $cimFailureFixture.Candidate) -Message "CIM failure cannot delete"

    $ownedWriterFixture = New-CleanupTestFixture
    $ownedWriterTarget = $ownedWriterFixture.TargetRoot
    $ownedWriterReport = Invoke-FixtureCleanup -Fixture $ownedWriterFixture -Apply -Hooks @{
        CimProvider = {
            @([pscustomobject]@{
                Name = "node.exe"
                ProcessId = 900001
                ParentProcessId = 0
                ExecutablePath = "C:\Program Files\nodejs\node.exe"
                CommandLine = "node build.js --target-dir `"$ownedWriterTarget`""
            })
        }
        StabilityDelayMilliseconds = 0
    }
    Assert-Equal -Expected "Blocked" -Actual $ownedWriterReport.Outcome -Message "target-owned node writer blocks Apply"
    Assert-Equal -Expected "ActiveOwnedWriter" -Actual $ownedWriterReport.Blocker.Code -Message "owned writer has a typed blocker"
    Assert-True -Condition (Test-Path -LiteralPath $ownedWriterFixture.Candidate) -Message "owned writer cannot race deletion"

    $unrelatedWriterFixture = New-CleanupTestFixture
    $unrelatedWriterPlan = Invoke-FixtureCleanup -Fixture $unrelatedWriterFixture -Hooks @{
        CimProvider = {
            @([pscustomobject]@{
                Name = "node.exe"
                ProcessId = 900002
                ParentProcessId = 0
                ExecutablePath = "C:\Program Files\nodejs\node.exe"
                CommandLine = "node C:\unrelated\server.js"
            })
        }
        StabilityDelayMilliseconds = 0
    }
    Assert-Equal -Expected "PlanReady" -Actual $unrelatedWriterPlan.Outcome -Message "same-name unrelated process is not misclassified as checkout-owned"
    Assert-True -Condition (Test-Path -LiteralPath $unrelatedWriterFixture.Candidate) -Message "unrelated writer test remains Plan-only"

    $missingMetadataFixture = New-CleanupTestFixture
    $missingMetadataReport = Invoke-FixtureCleanup -Fixture $missingMetadataFixture -Apply -Hooks @{
        CimProvider = {
            @([pscustomobject]@{
                Name = "cargo.exe"
                ProcessId = 900003
                ParentProcessId = 0
                ExecutablePath = $null
                CommandLine = $null
            })
        }
        StabilityDelayMilliseconds = 0
    }
    Assert-Equal -Expected "Blocked" -Actual $missingMetadataReport.Outcome -Message "unverifiable writer ownership blocks Apply"
    Assert-Equal -Expected "WriterOwnershipUnverifiable" -Actual $missingMetadataReport.Blocker.Code -Message "missing writer metadata has a typed blocker"

    $unstableFixture = New-CleanupTestFixture
    $unstableReport = Invoke-FixtureCleanup -Fixture $unstableFixture -Apply -Hooks @{
        CimProvider = { @() }
        BeforeSecondSample = {
            param([string]$CandidatePath)
            [IO.File]::AppendAllText((Join-Path $CandidatePath "cache.bin"), "changed-between-samples")
        }
        StabilityDelayMilliseconds = 0
    }
    Assert-Equal -Expected "Blocked" -Actual $unstableReport.Outcome -Message "size/mtime drift between samples blocks Apply"
    Assert-Equal -Expected "CandidateUnstable" -Actual $unstableReport.Blocker.Code -Message "sample drift has a typed blocker"
    Assert-True -Condition (Test-Path -LiteralPath $unstableFixture.Candidate) -Message "unstable candidate is not deleted"

    $toctouFixture = New-CleanupTestFixture
    $toctouReport = Invoke-FixtureCleanup -Fixture $toctouFixture -Apply -Hooks @{
        CimProvider = { @() }
        BeforeFinalValidation = {
            param([string]$CandidatePath)
            [IO.File]::AppendAllText((Join-Path $CandidatePath "cache.bin"), "changed-before-final-validation")
        }
        StabilityDelayMilliseconds = 0
    }
    Assert-Equal -Expected "Blocked" -Actual $toctouReport.Outcome -Message "TOCTOU mutation blocks Apply"
    Assert-Equal -Expected "CandidateChanged" -Actual $toctouReport.Blocker.Code -Message "TOCTOU mutation has a typed blocker"
    Assert-Equal -Expected "BlockedBeforeDelete" -Actual $toctouReport.Candidates[0].Status -Message "TOCTOU report distinguishes a pre-delete block"
    Assert-True -Condition (Test-Path -LiteralPath $toctouFixture.Candidate) -Message "TOCTOU candidate is not deleted"

    $hardlinkFixture = New-CleanupTestFixture
    $hardlinkSource = Join-Path $hardlinkFixture.Candidate "cache.bin"
    $hardlinkAlias = Join-Path $hardlinkFixture.Candidate "cache-alias.bin"
    [void](New-Item -ItemType HardLink -Path $hardlinkAlias -Target $hardlinkSource -Force)
    $hardlinkReport = Invoke-FixtureCleanup -Fixture $hardlinkFixture -Apply
    Assert-Equal -Expected "Blocked" -Actual $hardlinkReport.Outcome -Message "hardlinked content blocks Apply"
    Assert-Equal -Expected "HardlinkDetected" -Actual $hardlinkReport.Blocker.Code -Message "hardlink has a typed blocker"
    Assert-True -Condition (Test-Path -LiteralPath $hardlinkFixture.Candidate) -Message "hardlinked candidate is not deleted"

    $reparseFixture = New-CleanupTestFixture
    $junctionTarget = Join-Path $script:TestRunRoot ("junction-target-" + [Guid]::NewGuid().ToString("N"))
    [void](New-Item -ItemType Directory -Path $junctionTarget -Force)
    $junctionPath = Join-Path $reparseFixture.Candidate "hostile-junction"
    [void](New-Item -ItemType Junction -Path $junctionPath -Target $junctionTarget -Force)
    $reparseReport = Invoke-FixtureCleanup -Fixture $reparseFixture -Apply
    Assert-Equal -Expected "Blocked" -Actual $reparseReport.Outcome -Message "reparse content blocks Apply"
    Assert-Equal -Expected "ReparsePointDetected" -Actual $reparseReport.Blocker.Code -Message "reparse content has a typed blocker"
    Assert-True -Condition (Test-Path -LiteralPath $reparseFixture.Candidate) -Message "reparse candidate is not deleted"
    Remove-Item -LiteralPath $junctionPath -Force -ErrorAction Stop

    $partialFixture = New-CleanupTestFixture
    [IO.File]::WriteAllBytes((Join-Path $partialFixture.Candidate "second.bin"), [byte[]](1..16))
    $partialReport = Invoke-FixtureCleanup -Fixture $partialFixture -Apply -Hooks @{
        CimProvider = { @() }
        RemoveOperation = {
            param([string]$CandidatePath)
            $firstFile = Get-ChildItem -LiteralPath $CandidatePath -File -Force | Sort-Object Name | Select-Object -First 1
            Remove-Item -LiteralPath $firstFile.FullName -Force -ErrorAction Stop
            throw "injected partial removal failure"
        }
        StabilityDelayMilliseconds = 0
    }
    Assert-Equal -Expected "PartialFailure" -Actual $partialReport.Outcome -Message "partial Remove-Item failure is not reported as success"
    Assert-Equal -Expected "DeleteFailedPartial" -Actual $partialReport.Candidates[0].Status -Message "partial failure has a typed candidate status"
    Assert-True -Condition $partialReport.Candidates[0].ExistsAfterFailure -Message "partial failure records post-delete existence"
    Assert-True -Condition ([int64]$partialReport.Candidates[0].LogicalBytesAfter -lt [int64]$partialReport.Candidates[0].LogicalBytesBefore) -Message "partial failure records remaining bytes"
    Assert-True -Condition ([string]$partialReport.Candidates[0].RecoveryDisposition -ceq "RebuildOnly") -Message "partial failure retains recoverability disposition"

    $applyFixture = New-CleanupTestFixture
    $applyReport = Invoke-FixtureCleanup -Fixture $applyFixture -Apply
    Assert-Equal -Expected "Applied" -Actual $applyReport.Outcome -Message "dedicated temp seam can exercise the real LiteralPath removal"
    Assert-Equal -Expected "Deleted" -Actual $applyReport.Candidates[0].Status -Message "successful deletion has a typed status"
    Assert-True -Condition (-not (Test-Path -LiteralPath $applyFixture.Candidate)) -Message "successful temp Apply removes only the exact candidate"
    Assert-Equal -Expected 64 -Actual ([int64]$applyReport.ReclaimedLogicalBytes) -Message "successful Apply records reclaimed logical bytes"
    Assert-Equal -Expected 0 -Actual ([int64]$applyReport.Candidates[0].LogicalBytesAfter) -Message "successful Apply records zero post-delete bytes"
    Assert-Equal -Expected "RebuildOnly" -Actual $applyReport.Candidates[0].RecoveryDisposition -Message "successful Apply records recoverability"
    Assert-True -Condition (Test-Path -LiteralPath $applyFixture.RepositoryRoot -PathType Container) -Message "Apply never removes the repository root"
    Assert-True -Condition (Test-Path -LiteralPath $applyFixture.TargetRoot -PathType Container) -Message "Apply never removes the target root"

    $json = $applyReport | ConvertTo-Json -Depth 12 -Compress
    $roundTrip = $json | ConvertFrom-Json
    Assert-Equal -Expected 1 -Actual ([int]$roundTrip.SchemaVersion) -Message "report is JSON round-trippable with a schema version"
    Assert-Equal -Expected "Applied" -Actual ([string]$roundTrip.Outcome) -Message "report JSON preserves the typed outcome"

    Write-Output "PASS test-invoke-syndocal-build-cache-cleanup assertions=$script:AssertionCount"
} finally {
    if ($null -ne $script:TestRunRoot -and (Test-Path -LiteralPath $script:TestRunRoot)) {
        $resolvedSuite = [IO.Path]::GetFullPath($script:TestRunRoot).TrimEnd("\")
        $resolvedTemp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd("\")
        $suiteLeaf = [IO.Path]::GetFileName($resolvedSuite)
        if (-not $resolvedSuite.StartsWith($resolvedTemp + "\", [StringComparison]::OrdinalIgnoreCase) -or
            -not $suiteLeaf.StartsWith("syndocal-cleanup-suite-", [StringComparison]::Ordinal)) {
            throw "Refusing unsafe self-test cleanup target: $resolvedSuite"
        }
        Remove-Item -LiteralPath $resolvedSuite -Recurse -Force -ErrorAction Stop
    }
}
