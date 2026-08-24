<#
.SYNOPSIS
    Focused hermetic self-test for the qa/run-soak.ps1 Windows MSVC linker gate.

.DESCRIPTION
    Never launches Cargo, vcvars cmd.exe, where.exe, or any soak-harness process:
    gate functions are exercised through injected recording scriptblocks, and the
    full-script scenarios drive qa/run-soak.ps1 -PreflightOnly through the
    $global:SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES seam, asserting Git-linker-first
    rejection and that Cargo is never launched.
#>
$ErrorActionPreference = "Stop"

$soakScriptPath = Join-Path $PSScriptRoot "run-soak.ps1"
if (-not (Test-Path -LiteralPath $soakScriptPath)) {
    throw "run-soak.ps1 not found next to this self-test: $soakScriptPath"
}

$script:AssertionCount = 0
function Assert-True {
    param([Parameter(Mandatory = $true)][bool]$Condition, [Parameter(Mandatory = $true)][string]$Message)
    $script:AssertionCount++
    if (-not $Condition) { throw "FAILED: $Message" }
}
function Assert-Equal {
    param([AllowNull()]$Expected, [AllowNull()]$Actual, [Parameter(Mandatory = $true)][string]$Message)
    $script:AssertionCount++
    if ("$Expected" -ne "$Actual") { throw "FAILED: $Message (expected '$Expected', actual '$Actual')" }
}
function Assert-ThrowsLike {
    param(
        [Parameter(Mandatory = $true)][scriptblock]$Action,
        [Parameter(Mandatory = $true)][string]$ExpectedWildcard,
        [Parameter(Mandatory = $true)][string]$Message
    )
    $script:AssertionCount++
    $threw = $false
    $actualMessage = ""
    try { & $Action } catch { $threw = $true; $actualMessage = "$($_.Exception.Message)" }
    if (-not $threw) { throw "FAILED: $Message (expected a terminating error)" }
    if ($actualMessage -notlike $ExpectedWildcard) {
        throw "FAILED: $Message (error '$actualMessage' did not match '$ExpectedWildcard')"
    }
}

Push-Location
$locationBackup = (Get-Location).Path
$environmentBackup = @{
    PATH                                       = $env:PATH
    VCToolsInstallDir                          = $env:VCToolsInstallDir
    CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER
}
$restoreEnvironment = {
    foreach ($key in @("PATH", "VCToolsInstallDir", "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER")) {
        if ($null -eq $environmentBackup[$key]) {
            Remove-Item -LiteralPath "env:$key" -ErrorAction SilentlyContinue
        } else {
            Set-Item -LiteralPath "env:$key" -Value $environmentBackup[$key]
        }
    }
}

try {
    # --- Parser load and static contract checks -------------------------------
    $parseTokens = $null
    $parseErrors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile(
        (Resolve-Path -LiteralPath $soakScriptPath).ProviderPath, [ref]$parseTokens, [ref]$parseErrors)
    Assert-Equal 0 @($parseErrors).Count "run-soak.ps1 must parse without errors"

    $source = Get-Content -LiteralPath $soakScriptPath -Raw
    $allCommands = @($ast.FindAll(
        { param($candidate) $candidate -is [System.Management.Automation.Language.CommandAst] }, $true))
    $cargoCommands = @($allCommands | Where-Object { $_.GetCommandName() -eq "cargo" })
    Assert-Equal 1 $cargoCommands.Count "run-soak.ps1 must contain exactly one cargo launch"
    $cargoCommand = $cargoCommands[0]
    $preflightCommands = @($allCommands | Where-Object { $_.GetCommandName() -eq "Invoke-SoakMsvcPreflight" })
    Assert-True ($preflightCommands.Count -ge 1) "the MSVC preflight must be invoked"
    Assert-True ($preflightCommands[0].Extent.StartOffset -lt $cargoCommand.Extent.StartOffset) "the MSVC preflight must run before Cargo"

    $preflightOnlyIfStatements = @($ast.FindAll(
        { param($candidate) $candidate -is [System.Management.Automation.Language.IfStatementAst] }, $true) |
        Where-Object { $_.Clauses[0].Item1.Extent.Text -eq '$PreflightOnly' })
    Assert-Equal 1 $preflightOnlyIfStatements.Count "there must be exactly one PreflightOnly branch"
    $preflightOnlyIf = $preflightOnlyIfStatements[0]
    Assert-True ($preflightOnlyIf.Extent.EndOffset -le $cargoCommand.Extent.StartOffset) "the PreflightOnly branch must complete before the cargo launch"
    Assert-True ($preflightCommands[0].Extent.StartOffset -lt $preflightOnlyIf.Extent.StartOffset) "the gate must run before the PreflightOnly early exit"
    $commandsInsidePreflightOnly = @($allCommands | Where-Object {
            $_.Extent.StartOffset -ge $preflightOnlyIf.Extent.StartOffset -and
            $_.Extent.EndOffset -le $preflightOnlyIf.Extent.EndOffset })
    $forbiddenPreflightOnlyCommands = @($commandsInsidePreflightOnly | Where-Object {
            $_.GetCommandName() -in @("cargo", "Start-Process", "where.exe", "cmd.exe") })
    Assert-Equal 0 $forbiddenPreflightOnlyCommands.Count "the PreflightOnly branch must never launch Cargo or native processes"

    foreach ($literal in @(
            "14.44.35207",
            "-vcvars_ver=14.44",
            "Auxiliary\Build\vcvars64.bat",
            "bin\Hostx64\x64\link.exe",
            "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER",
            "without VCToolsInstallDir",
            "does not match",
            "asserted MSVC linker is missing",
            "returned no linker",
            "where.exe link.exe failed",
            "resolves '",
            "initialization failed",
            "pinned CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=",
            "[run-soak] where.exe link.exe:",
            '$IsWindows -or $env:OS -eq "Windows_NT"',
            "SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES",
            "must never be present on a build or soak-harness run"
        )) {
        Assert-True $source.Contains($literal) "run-soak.ps1 must contain '$literal'"
    }

    # --- Load the gate members hermetically (constants + functions only) ------
    $functionMembers = @($ast.FindAll(
        { param($candidate) $candidate -is [System.Management.Automation.Language.FunctionDefinitionAst] }, $false))
    $constantMembers = @($ast.FindAll(
        { param($candidate) $candidate -is [System.Management.Automation.Language.AssignmentStatementAst] }, $false) |
        Where-Object { $_.Left.Extent.Text -match '^\$script:' })
    $sortedMembers = @(@($constantMembers + $functionMembers) | Sort-Object { $_.Extent.StartOffset })
    Assert-True ($sortedMembers.Count -gt 5) "gate members must be extractable"
    foreach ($member in $sortedMembers) {
        Invoke-Expression $member.Extent.Text
    }

    $requiredToolset = $script:RequiredVcToolsVersion
    $requiredInstallDir = $script:RequiredVcToolsInstallDir
    $requiredLinker = $script:RequiredMsvcLinker
    $staleInstallDir = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.43.34808"
    $gitLinkerPath = "C:/Program Files/Git/usr/bin/link.exe"

    # --- Pure-function units ---------------------------------------------------
    Assert-Equal "c:\a b\d" (ConvertTo-PathComparisonKey "C:/A B\D ") "path comparison key normalization"
    $parsedSetOutput = ConvertFrom-CommandLineSetOutput (@(
            "********** vcvars banner **********",
            "[vcvarsall.bat Environment initialized]",
            "VCToolsInstallDir=$requiredInstallDir\",
            "PATH=C:\stub-msvc;C:\Windows",
            ""
        ) -join "`r`n")
    Assert-Equal "$requiredInstallDir\" $parsedSetOutput["VCToolsInstallDir"] "set-output parsing keeps values"
    Assert-Equal "C:\stub-msvc;C:\Windows" $parsedSetOutput["PATH"] "set-output parsing keeps PATH"
    Assert-True ($null -eq (ConvertFrom-CommandLineSetOutput "no assignments here")) "banner-only set output parses to null"
    Assert-True ($null -eq (ConvertFrom-CommandLineSetOutput "")) "empty set output parses to null"
    Assert-Equal $requiredLinker (Get-MsvcHostLinkerPath "$requiredInstallDir\") "host linker resolution tolerates trailing slash"

    # --- Toolset assertion units -----------------------------------------------
    $properEnvironment = @{ VCToolsInstallDir = "$requiredInstallDir\" }
    Assert-Equal $requiredLinker (Assert-ExactMsvcToolset $properEnvironment { param($path) $true }) "exact toolset asserts to the pinned linker"
    Assert-ThrowsLike { Assert-ExactMsvcToolset @{} { param($path) $true } } "*without VCToolsInstallDir*" "missing VCToolsInstallDir fails closed"
    Assert-ThrowsLike { Assert-ExactMsvcToolset @{ VCToolsInstallDir = "   " } { param($path) $true } } "*without VCToolsInstallDir*" "empty VCToolsInstallDir fails closed"
    Assert-ThrowsLike { Assert-ExactMsvcToolset @{ VCToolsInstallDir = $staleInstallDir } { param($path) $true } } "*does not match*${requiredToolset}*" "stale 14.43 toolset fails closed even when present"
    Assert-ThrowsLike { Assert-ExactMsvcToolset $properEnvironment { param($path) $false } } "*asserted MSVC linker is missing*" "missing linker file fails closed"

    # --- Linker-order assertion units ------------------------------------------
    $orderedLinkers = @(Assert-ExactMsvcLinkerFirst $properEnvironment { param($environment) @($requiredLinker, $gitLinkerPath) })
    Assert-Equal 2 $orderedLinkers.Count "linker-first success preserves resolution order"
    Assert-Equal $requiredLinker $orderedLinkers[0] "the exact linker must be first"
    Assert-ThrowsLike { Assert-ExactMsvcLinkerFirst $properEnvironment { param($environment) @($gitLinkerPath, $requiredLinker) } } "*resolves '${gitLinkerPath}' first*" "Git usr/bin/link.exe resolving first must be rejected"
    Assert-ThrowsLike { Assert-ExactMsvcLinkerFirst $properEnvironment { param($environment) @() } } "*returned no linker*" "empty linker resolution fails closed"

    # --- Verified-environment units --------------------------------------------
    $locateRecording = New-Object System.Collections.Generic.List[object]
    $poisonedInput = @{ VCToolsInstallDir = $requiredInstallDir; CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = $gitLinkerPath; KEEP = "yes" }
    $verifiedPoisoned = Get-VerifiedNativeBuildEnvironment -Environment $poisonedInput -IsWindowsHost $true `
        -FileIsRegular { param($path) $true } `
        -LocateLinkers { param($environment) $locateRecording.Add($environment); @($requiredLinker, $gitLinkerPath) }
    Assert-Equal $requiredLinker $verifiedPoisoned["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER"] "verified environment pins the exact linker over an inherited Git linker"
    Assert-Equal $gitLinkerPath $poisonedInput["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER"] "verification must not mutate the caller's environment"
    Assert-Equal "yes" $verifiedPoisoned["KEEP"] "verified environment preserves unrelated entries"
    Assert-Equal $requiredLinker $locateRecording[0]["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER"] "linker-order verification sees the pinned environment"

    $nonWindowsInput = @{ VCToolsInstallDir = ""; CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = $gitLinkerPath }
    $nonWindowsLocateRecording = New-Object System.Collections.Generic.List[object]
    $nonWindowsResult = Get-VerifiedNativeBuildEnvironment -Environment $nonWindowsInput -IsWindowsHost $false `
        -FileIsRegular { param($path) $false } `
        -LocateLinkers { param($environment) $nonWindowsLocateRecording.Add($environment); @($gitLinkerPath) }
    Assert-True ([object]::ReferenceEquals($nonWindowsInput, $nonWindowsResult)) "non-Windows hosts keep their environment untouched"
    Assert-Equal 0 $nonWindowsLocateRecording.Count "non-Windows hosts never probe linker resolution"

    # --- Preflight healing units -------------------------------------------------
    $unitCaptureRecording = New-Object System.Collections.Generic.List[object]
    $unitCaptureResult = $properEnvironment
    $unitLocateRecording = New-Object System.Collections.Generic.List[object]
    $global:SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES = @{
        captureVcvarsEnvironment = {
            param($initialEnvironment)
            $script:UnitCaptureRecording.Add($initialEnvironment)
            return $script:UnitCaptureResult
        }
        locateLinkers            = {
            param($environment)
            $script:UnitLocateRecording.Add($environment)
            return @($requiredLinker)
        }
        fileIsRegular            = { param($path) $true }
    }
    $healedEnvironment = Invoke-SoakMsvcPreflight -Environment @{ VCToolsInstallDir = $staleInstallDir }
    Assert-Equal $requiredLinker $healedEnvironment["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER"] "automatic vcvars healing pins the exact linker"
    Assert-Equal 1 $unitCaptureRecording.Count "healing initializes vcvars exactly once"
    Assert-Equal $staleInstallDir $unitCaptureRecording[0]["VCToolsInstallDir"] "vcvars initialization seeds from the caller's environment"
    Assert-Equal 1 $unitLocateRecording.Count "healed verification probes linker resolution once"

    $unitCaptureResult = $null
    Assert-ThrowsLike { Invoke-SoakMsvcPreflight -Environment @{ VCToolsInstallDir = "" } } "*initialization failed*" "failed vcvars initialization fails closed"
    Assert-Equal 2 $unitCaptureRecording.Count "the failing preflight attempted initialization exactly once"
    $global:SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES = $null

    # --- Full-script hermetic scenarios -----------------------------------------
    Set-Location $locationBackup
    $recorder = @{
        CaptureCalls     = New-Object System.Collections.Generic.List[object]
        LocateCalls      = New-Object System.Collections.Generic.List[object]
        RegularCalls     = New-Object System.Collections.Generic.List[object]
        CargoInvocations = New-Object System.Collections.Generic.List[object]
    }
    $captureResult = $null
    $locateResult = @()
    $regularResult = $true
    $global:SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES = @{
        captureVcvarsEnvironment = {
            param($initialEnvironment)
            $recorder.CaptureCalls.Add($initialEnvironment)
            return $captureResult
        }
        locateLinkers            = {
            param($environment)
            $recorder.LocateCalls.Add($environment)
            return $locateResult
        }
        fileIsRegular            = {
            param($path)
            $recorder.RegularCalls.Add($path)
            return $regularResult
        }
        invokeCargo              = {
            param($cargoArguments)
            $recorder.CargoInvocations.Add($cargoArguments)
            throw "invokeCargo reached: the gate failed to prevent Cargo"
        }
    }

    # S0: the hermetic dependency seam can never bypass a real build/harness run.
    Assert-ThrowsLike {
        & $soakScriptPath -SkipBuild 6>&1 | Out-Null
    } "*must never be present on a build or soak-harness run*" "test dependency overrides are rejected outside -PreflightOnly"
    Assert-Equal 0 $recorder.CargoInvocations.Count "S0 rejects test overrides before Cargo"

    # S1: stale shell heals through vcvars, pins, prints evidence, exits without Cargo.
    $env:VCToolsInstallDir = $staleInstallDir
    $captureResult = @{ VCToolsInstallDir = "$requiredInstallDir\"; PATH = "C:\synodocal-stub-msvc;C:\Windows" }
    $locateResult = @($requiredLinker, $gitLinkerPath)
    $scenarioOneOutput = (@(& $soakScriptPath -PreflightOnly 6>&1) | ForEach-Object { "$_" }) -join "`n"
    Assert-Equal 1 $recorder.CaptureCalls.Count "S1 initializes vcvars exactly once"
    Assert-Equal $staleInstallDir $recorder.CaptureCalls[0]["VCToolsInstallDir"] "S1 seeds vcvars from the ambient shell"
    Assert-True ($recorder.LocateCalls.Count -ge 1) "S1 verifies linker resolution"
    foreach ($locatedEnvironment in $recorder.LocateCalls) {
        Assert-Equal $requiredLinker $locatedEnvironment["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER"] "S1 verifies the pinned environment"
    }
    Assert-Equal $requiredLinker $recorder.RegularCalls[0] "S1 verifies the exact linker file exists"
    Assert-Equal 0 $recorder.CargoInvocations.Count "S1 must never launch Cargo"
    Assert-True $scenarioOneOutput.Contains("pinned CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=$requiredLinker") "S1 prints the linker pin"
    Assert-True $scenarioOneOutput.Contains("[run-soak] where.exe link.exe:") "S1 prints the where.exe link.exe order"
    Assert-True $scenarioOneOutput.Contains($gitLinkerPath) "S1 prints the full resolution order"
    Assert-True $scenarioOneOutput.Contains("preflight-only completed") "S1 reports the preflight-only boundary"
    Assert-Equal $requiredLinker $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER "S1 applies the pin to the session environment"

    # S2: Git link.exe resolving first fails closed even after healing; no Cargo.
    $env:VCToolsInstallDir = "$requiredInstallDir\"
    $captureResult = $properEnvironment
    $locateResult = @($gitLinkerPath, $requiredLinker)
    $captureBeforeScenarioTwo = $recorder.CaptureCalls.Count
    $scenarioTwoThrew = $false
    $scenarioTwoMessage = ""
    try { & $soakScriptPath -PreflightOnly 6>&1 | Out-Null } catch {
        $scenarioTwoThrew = $true
        $scenarioTwoMessage = "$($_.Exception.Message)"
    }
    Assert-True $scenarioTwoThrew "S2 must reject Git-first linker resolution"
    Assert-True $scenarioTwoMessage.Contains("resolves '$gitLinkerPath' first") "S2 error names the Git linker resolved first"
    Assert-Equal ($captureBeforeScenarioTwo + 1) $recorder.CaptureCalls.Count "S2 attempts vcvars healing exactly once before rejecting"
    Assert-Equal 0 $recorder.CargoInvocations.Count "S2 must never launch Cargo"

    # S3: failed vcvars initialization fails closed without any resolution probe.
    $env:VCToolsInstallDir = ""
    $captureResult = $null
    $locateBeforeScenarioThree = $recorder.LocateCalls.Count
    $scenarioThreeThrew = $false
    $scenarioThreeMessage = ""
    try { & $soakScriptPath -PreflightOnly 6>&1 | Out-Null } catch {
        $scenarioThreeThrew = $true
        $scenarioThreeMessage = "$($_.Exception.Message)"
    }
    Assert-True $scenarioThreeThrew "S3 must reject failed vcvars initialization"
    Assert-True $scenarioThreeMessage.Contains("initialization failed") "S3 error names the failed initialization"
    Assert-Equal $locateBeforeScenarioThree $recorder.LocateCalls.Count "S3 never probes linker resolution"
    Assert-Equal 0 $recorder.CargoInvocations.Count "S3 must never launch Cargo"

    & $restoreEnvironment
    $global:SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES = $null
} finally {
    & $restoreEnvironment
    $global:SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES = $null
    Set-Location $locationBackup
    Pop-Location
}

Write-Host "run-soak linker gate self-test passed ($($script:AssertionCount) assertions)"
