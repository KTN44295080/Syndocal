#requires -Version 7.0

[CmdletBinding()]
param(
    [switch]$Execute,
    [string]$RepositoryRoot,
    [string]$TargetRoot,
    [string[]]$WriterProcessNames,
    [switch]$FunctionsOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script:CleanupToolVersion = "2.1.0-plan-only-fail-closed"
$script:MinStalenessDays = 7
$script:ExecutionCapability = [ordered]@{
    Enabled = $false
    Reason = "Execution is intentionally disabled: the PowerShell implementation cannot prove a plan-to-rename path is safe against NTFS file-ID reuse and namespace swaps without a verified handle-based rename primitive."
    RequiredPrimitive = "A reviewed handle-based rename implementation that pins the opened candidate through the rename, plus a new exact-path adversarial review."
}
$script:DefaultWriterProcessNames = @(
    "cargo.exe",
    "rustc.exe",
    "rustdoc.exe",
    "link.exe",
    "lld-link.exe",
    "lld.exe",
    "mspdbsrv.exe",
    "msbuild.exe",
    "cl.exe",
    "rc.exe",
    "mt.exe",
    "csc.exe",
    "vbc.exe",
    "dotnet.exe",
    "node.exe",
    "npm.exe",
    "npx.exe",
    "pnpm.exe",
    "esbuild.exe",
    "syndocal.exe"
)

if (-not ("Kdmx.Cleanup.FileIdTools" -as [type])) {
    Add-Type -TypeDefinition @'
namespace Kdmx.Cleanup
{
    using System;
    using System.ComponentModel;
    using System.Runtime.InteropServices;

    public static class FileIdTools
    {
        [StructLayout(LayoutKind.Sequential)]
        private struct FILETIME
        {
            public uint LowPart;
            public uint HighPart;
        }

        [StructLayout(LayoutKind.Sequential)]
        private struct BY_HANDLE_FILE_INFORMATION
        {
            public uint FileAttributes;
            public FILETIME CreationTime;
            public FILETIME LastAccessTime;
            public FILETIME LastWriteTime;
            public uint VolumeSerialNumber;
            public uint FileSizeHigh;
            public uint FileSizeLow;
            public uint NumberOfLinks;
            public uint FileIndexHigh;
            public uint FileIndexLow;
        }

        [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode, ExactSpelling = true)]
        private static extern IntPtr CreateFileW(
            string lpFileName,
            uint dwDesiredAccess,
            uint dwShareMode,
            IntPtr lpSecurityAttributes,
            uint dwCreationDisposition,
            uint dwFlagsAndAttributes,
            IntPtr hTemplateFile);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool GetFileInformationByHandle(
            IntPtr hFile,
            out BY_HANDLE_FILE_INFORMATION lpFileInformation);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool CloseHandle(IntPtr hObject);

        private const uint FileReadAttributes = 0x00000080;
        private const uint FileShareReadWriteDelete = 0x00000007;
        private const uint OpenExisting = 3;
        private const uint FileFlagBackupSemantics = 0x02000000;

        // This is an observation, not a persistent identity guarantee.  NTFS may
        // reuse a file index after deletion, so callers must never treat this
        // value as sufficient authorization for a later destructive operation.
        public static string GetEntryObservation(string path)
        {
            IntPtr handle = CreateFileW(
                path,
                FileReadAttributes,
                FileShareReadWriteDelete,
                IntPtr.Zero,
                OpenExisting,
                FileFlagBackupSemantics,
                IntPtr.Zero);
            if (handle == IntPtr.Zero || handle == new IntPtr(-1))
            {
                throw new Win32Exception(
                    Marshal.GetLastWin32Error(),
                    "Unable to open entry for identity probe: " + path);
            }

            try
            {
                if (!GetFileInformationByHandle(handle, out BY_HANDLE_FILE_INFORMATION info))
                {
                    throw new Win32Exception(
                        Marshal.GetLastWin32Error(),
                        "Unable to query entry identity: " + path);
                }

                ulong fileIndex = ((ulong)info.FileIndexHigh << 32) | (ulong)info.FileIndexLow;
                ulong creationTime = ((ulong)info.CreationTime.HighPart << 32) | (ulong)info.CreationTime.LowPart;
                return string.Format(
                    System.Globalization.CultureInfo.InvariantCulture,
                    "{0:X8}:{1:X16}:{2:X16}",
                    info.VolumeSerialNumber,
                    fileIndex,
                    creationTime);
            }
            finally
            {
                CloseHandle(handle);
            }
        }
    }
}
'@
}

function Get-CleanupEntryObservation {
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    return [Kdmx.Cleanup.FileIdTools]::GetEntryObservation($Path)
}

function Get-CleanupExecutionCapability {
    return [pscustomobject]$script:ExecutionCapability
}

function Assert-CleanupExecutionCapability {
    param(
        [Parameter(Mandatory)]
        [string]$Operation
    )

    $capability = Get-CleanupExecutionCapability
    if (-not $capability.Enabled) {
        throw "$Operation is disabled fail-closed. $($capability.Reason) Required before enabling: $($capability.RequiredPrimitive)"
    }

    # Deliberately no bypass exists in this script.  Turning this on requires a
    # separately reviewed implementation, not a command-line flag or a test hook.
    throw "$Operation is disabled fail-closed: no reviewed handle-based rename capability is available."
}

function Test-CleanupReparseItem {
    param(
        [Parameter(Mandatory)]
        $Item
    )

    return (($Item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)
}

function Assert-CleanupVerifiedContainer {
    param(
        [Parameter(Mandatory)]
        [string]$Path,
        [Parameter(Mandatory)]
        [string]$Label
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "$Label is not an existing directory: $Path"
    }

    $item = Get-Item -LiteralPath $Path -Force
    if (Test-CleanupReparseItem -Item $item) {
        throw "$Label is a reparse point; refusing: $Path"
    }

    if (-not $item.PSIsContainer) {
        throw "$Label is not a directory: $Path"
    }
}

function Assert-CleanupAncestryNoReparse {
    param(
        [Parameter(Mandatory)]
        [string]$Root,
        [Parameter(Mandatory)]
        [string]$Leaf
    )

    $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd("\")
    $leafFull = [IO.Path]::GetFullPath($Leaf)

    if (-not $leafFull.StartsWith(
        $rootFull + "\",
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Leaf path is not physically contained by root (lexical check): root=$rootFull leaf=$leafFull"
    }

    $relativePart = $leafFull.Substring($rootFull.Length + 1)
    $segments = $relativePart.Split([char[]]@("\"), [StringSplitOptions]::RemoveEmptyEntries)
    $accumulator = $rootFull
    foreach ($segment in $segments) {
        $accumulator = Join-Path $accumulator $segment
        Assert-CleanupVerifiedContainer -Path $accumulator -Label "Ancestor/descendant chain element"
    }
}

function Assert-CleanupTreeNoReparse {
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    $items = @(Get-ChildItem -LiteralPath $Path -Recurse -Force)
    foreach ($item in $items) {
        if (Test-CleanupReparseItem -Item $item) {
            throw "Reparse point found inside tree; refusing: $($item.FullName)"
        }
    }
}

function Assert-CleanupGitCheckpoint {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot
    )

    $headOutput = & git -C $RepositoryRoot rev-parse HEAD
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to resolve Git HEAD."
    }
    $head = ($headOutput | Out-String).Trim()

    $upstreamOutput = & git -C $RepositoryRoot rev-parse "@{upstream}"
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to resolve Git upstream."
    }
    $upstream = ($upstreamOutput | Out-String).Trim()

    if ($head -cne $upstream) {
        throw "HEAD is not synchronized with upstream: $head / $upstream"
    }

    $topLevelOutput = & git -C $RepositoryRoot rev-parse --show-toplevel
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to resolve physical Git toplevel."
    }
    $toplevelRaw = ($topLevelOutput | Out-String).Trim()
    $toplevelPhysical = [IO.Path]::GetFullPath(($toplevelRaw -replace "/", "\"))
    $repositoryPhysical = [IO.Path]::GetFullPath($RepositoryRoot).TrimEnd("\")
    if (-not $toplevelPhysical.Equals(
        $repositoryPhysical,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Physical Git toplevel does not match the verified repository root: toplevel=$toplevelPhysical expected=$repositoryPhysical"
    }

    $statusOutput = & git -C $RepositoryRoot status --porcelain=v1
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to verify Git worktree status."
    }
    $statusLines = @(
        $statusOutput |
        ForEach-Object { [string]$_ } |
        Where-Object { $_.Length -gt 0 }
    )
    if ($statusLines.Count -ne 0) {
        throw "Git worktree is not clean; refusing Execute cleanup:`n$($statusLines -join [Environment]::NewLine)"
    }

    return [pscustomobject]@{
        Head = $head
        Upstream = $upstream
        PhysicalToplevel = $toplevelPhysical
    }
}

function Assert-CleanupNoActiveWriters {
    param(
        [Parameter(Mandatory)]
        [AllowEmptyCollection()]
        [string[]]$Names,
        [Parameter(Mandatory)]
        [AllowEmptyCollection()]
        [string[]]$ForbiddenRoots
    )

    $processes = @(Get-CimInstance -ClassName Win32_Process)
    $offenders = @()
    foreach ($process in $processes) {
        $processName = [string]$process.Name
        $executablePath = ""
        try {
            if ($null -ne $process.ExecutablePath) {
                $executablePath = [string]$process.ExecutablePath
            }
        } catch {
            $executablePath = ""
        }

        $nameHit = ($Names.Count -gt 0) -and ($Names -contains $processName)
        $pathHit = $false
        if ($executablePath.Length -gt 0) {
            foreach ($forbiddenRoot in $ForbiddenRoots) {
                $forbiddenPrefix = ([IO.Path]::GetFullPath($forbiddenRoot)).TrimEnd("\") + "\"
                if ($executablePath.StartsWith(
                    $forbiddenPrefix,
                    [StringComparison]::OrdinalIgnoreCase
                )) {
                    $pathHit = $true
                    break
                }
            }
        }

        if ($nameHit -or $pathHit) {
            $offenders += "name=$processName exe=$executablePath" + $(if ($pathHit) { " reason=executable-under-protected-root" } else { " reason=known-writer-name" })
        }
    }

    if ($offenders.Count -ne 0) {
        throw ("Active build/tool process denies cleanup; remaining fail-closed:`n" + ($offenders -join [Environment]::NewLine))
    }
}

function Test-CleanupProtectedOverlap {
    param(
        [Parameter(Mandatory)]
        [string]$TargetRoot,
        [Parameter(Mandatory)]
        [string]$CandidatePath,
        [Parameter(Mandatory)]
        [AllowEmptyCollection()]
        [string[]]$ProtectedPaths
    )

    $targetFull = ([IO.Path]::GetFullPath($TargetRoot)).TrimEnd("\")
    $candidateFull = [IO.Path]::GetFullPath($CandidatePath)

    foreach ($protectedPath in $ProtectedPaths) {
        $protectedFull = [IO.Path]::GetFullPath((Join-Path $targetFull $protectedPath)).TrimEnd("\")
        if ($candidateFull.Equals($protectedFull, [StringComparison]::OrdinalIgnoreCase) -or
            $candidateFull.StartsWith(
                $protectedFull + "\",
                [StringComparison]::OrdinalIgnoreCase
            )) {
            return $protectedFull
        }
    }

    return $null
}

function Get-CleanupNewestWriteTime {
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    # A shallow listing is not a staleness proof: Cargo output may have a fresh
    # file arbitrarily far below an old cache root.  Include the root and every
    # descendant, and refuse a tree containing a reparse point rather than
    # accidentally following it while deriving the age gate.
    $items = @((Get-Item -LiteralPath $Path -Force)) + @(
        Get-ChildItem -LiteralPath $Path -Recurse -Force
    )
    foreach ($item in $items) {
        if (Test-CleanupReparseItem -Item $item) {
            throw "Reparse point found while calculating newest write time; refusing: $($item.FullName)"
        }
    }

    return (($items | Measure-Object -Property LastWriteTime -Maximum).Maximum)
}

function Get-CleanupCandidateSafetySnapshot {
    param(
        [Parameter(Mandatory)]
        [string]$TargetRoot,
        [Parameter(Mandatory)]
        [string]$CandidatePath
    )

    Assert-CleanupVerifiedContainer -Path $TargetRoot -Label "Target root"
    Assert-CleanupAncestryNoReparse -Root $TargetRoot -Leaf $CandidatePath
    Assert-CleanupVerifiedContainer -Path $CandidatePath -Label "Candidate"
    Assert-CleanupTreeNoReparse -Path $CandidatePath

    return [pscustomobject]@{
        TargetRoot = [IO.Path]::GetFullPath($TargetRoot).TrimEnd("\")
        CandidatePath = [IO.Path]::GetFullPath($CandidatePath)
        TargetRootObservation = Get-CleanupEntryObservation -Path $TargetRoot
        CandidateObservation = Get-CleanupEntryObservation -Path $CandidatePath
        HasReparsePoint = $false
    }
}

function Assert-CleanupCandidateSafetySnapshotMatches {
    param(
        [Parameter(Mandatory)]
        $ExpectedSnapshot
    )

    $actualSnapshot = Get-CleanupCandidateSafetySnapshot -TargetRoot $ExpectedSnapshot.TargetRoot -CandidatePath $ExpectedSnapshot.CandidatePath

    if ($actualSnapshot.TargetRootObservation -cne $ExpectedSnapshot.TargetRootObservation) {
        throw "Target root observation changed after planning; refusing namespace swap: $($ExpectedSnapshot.TargetRoot)"
    }
    if ($actualSnapshot.CandidateObservation -cne $ExpectedSnapshot.CandidateObservation) {
        throw "Candidate observation changed after planning; refusing namespace swap: $($ExpectedSnapshot.CandidatePath)"
    }
    if ($actualSnapshot.HasReparsePoint) {
        throw "Reparse point observed after planning; refusing: $($ExpectedSnapshot.CandidatePath)"
    }

    return $actualSnapshot
}

function Get-CleanupRebuildInstruction {
    param(
        [Parameter(Mandatory)]
        [string]$Kind
    )

    switch ($Kind) {
        "cargo-root" {
            return "Regenerated on demand by cargo build/test in the owning workspace (CACHEDIR.TAG/.rustc_info.json are recreated automatically)."
        }
        "debug-incremental" {
            return "Regenerated on demand by any cargo debug build (recreates target/debug/incremental)."
        }
        default {
            return "Unknown kind; manual rebuild required."
        }
    }
}

function Write-CleanupOutcome {
    param(
        [Parameter(Mandatory)]
        $Outcome
    )

    # This is deliberately one parseable record.  Tests and operators must use
    # it as the primary result; prose lines below are only secondary diagnostics.
    $json = $Outcome | ConvertTo-Json -Depth 10 -Compress
    Write-Output "CLEANUP-OUTCOME $json"
}

function Invoke-GuardedQuarantinePurge {
    param(
        [Parameter(Mandatory)]
        [string]$Path,
        [ValidateSet("None", "BeforeFirstRemoval")]
        [string]$FailureInjection = "None"
    )

    if ($FailureInjection -eq "BeforeFirstRemoval") {
        throw "Injected quarantine purge failure before first removal; preserving: $Path"
    }

    Assert-CleanupExecutionCapability -Operation "Quarantine purge"
}

function Invoke-CleanupCandidateLifecycle {
    param(
        [Parameter(Mandatory)]
        $PlanEntry,
        [Parameter(Mandatory)]
        [string]$QuarantineRunDir,
        [Parameter(Mandatory)]
        [string]$TargetRoot
    )

    Assert-CleanupExecutionCapability -Operation "Candidate quarantine lifecycle"
}

if ($FunctionsOnly) {
    return
}

$cleanupRepositoryRoot = if ($RepositoryRoot) {
    [IO.Path]::GetFullPath($RepositoryRoot)
} else {
    [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
}
$cleanupRepositoryRoot = $cleanupRepositoryRoot.TrimEnd("\")

$cleanupTargetRoot = if ($TargetRoot) {
    [IO.Path]::GetFullPath($TargetRoot)
} else {
    [IO.Path]::GetFullPath((Join-Path $cleanupRepositoryRoot "target"))
}
$cleanupTargetRoot = $cleanupTargetRoot.TrimEnd("\")

Assert-CleanupVerifiedContainer -Path $cleanupRepositoryRoot -Label "Repository root"
Assert-CleanupVerifiedContainer -Path $cleanupTargetRoot -Label "Target root"

if (($cleanupTargetRoot | Split-Path -Leaf) -cne "target") {
    throw "Refusing unexpected target root leaf name: $cleanupTargetRoot"
}

Assert-CleanupAncestryNoReparse -Root $cleanupRepositoryRoot -Leaf $cleanupTargetRoot

$protectedPaths = @(
    "release",
    "qa",
    "root-warning-review",
    "vendor-wry-review",
    "asio-qa",
    "warning-capture"
)

foreach ($protectedPath in $protectedPaths) {
    $protectedFullPath = [IO.Path]::GetFullPath(
        (Join-Path $cleanupTargetRoot $protectedPath)
    )
    if (-not $protectedFullPath.StartsWith(
        $cleanupTargetRoot + "\",
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Protected path escaped target root: $protectedFullPath"
    }
}

$staleTargetSpecs = @(
    [pscustomobject]@{ Relative = "asio-bridge-check"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "asio-bridge-sdk-free"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "asio-code-review-check"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "asio-hardware-qa"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "asio-loader-qa"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "asio-manifest-default-qa"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "atomic-live-frame-check"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "audio-qa"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "audio-qa-final"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "audio-tests-final"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "audio-vj-check"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "codex-dvc"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "head-bench"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "isf-stack-check"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "protocol-auto-vj-final"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "tauri-asio-native-qa"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "tauri-asio-test"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "tauri-audio-auto-vj-check"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "tauri-audio-rack-check"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "tauri-audio-transport-check"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "tauri-default-test"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "tauri-f11-focus-qa"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "tauri-isf-stack-final-qa"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "tauri-isf-stack-qa"; Kind = "cargo-root" },
    [pscustomobject]@{ Relative = "debug\incremental"; Kind = "debug-incremental" }
)

if ($staleTargetSpecs.Count -ne 25) {
    throw "Expected 25 fixed stale-target specifications."
}

$plannedEntries = @()
$outcomeEntries = @()
foreach ($staleTargetSpec in $staleTargetSpecs) {
    $candidateFullPath = [IO.Path]::GetFullPath(
        (Join-Path $cleanupTargetRoot $staleTargetSpec.Relative)
    )

    if (-not $candidateFullPath.StartsWith(
        $cleanupTargetRoot + "\",
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Deletion candidate escaped target root: $candidateFullPath"
    }

    if ($candidateFullPath.Equals(
        $cleanupTargetRoot,
        [StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Deletion candidate equals target root: $candidateFullPath"
    }

    $overlap = Test-CleanupProtectedOverlap -TargetRoot $cleanupTargetRoot -CandidatePath $candidateFullPath -ProtectedPaths $protectedPaths
    if ($null -ne $overlap) {
        throw "Deletion candidate overlaps a protected path ($overlap): $candidateFullPath"
    }

    if (-not (Test-Path -LiteralPath $candidateFullPath -PathType Container)) {
        $outcomeEntries += [pscustomobject]@{
            Relative = $staleTargetSpec.Relative
            Path = $candidateFullPath
            Kind = $staleTargetSpec.Kind
            Status = "SkippedMissing"
        }
        Write-Output "SKIP missing $candidateFullPath"
        continue
    }

    $candidateItem = Get-Item -LiteralPath $candidateFullPath -Force
    if (Test-CleanupReparseItem -Item $candidateItem) {
        throw "Refusing reparse-point candidate: $candidateFullPath"
    }

    Assert-CleanupAncestryNoReparse -Root $cleanupTargetRoot -Leaf $candidateFullPath

    switch ($staleTargetSpec.Kind) {
        "cargo-root" {
            if (
                -not (Test-Path -LiteralPath (
                    Join-Path $candidateFullPath "CACHEDIR.TAG"
                ) -PathType Leaf) -or
                -not (Test-Path -LiteralPath (
                    Join-Path $candidateFullPath ".rustc_info.json"
                ) -PathType Leaf)
            ) {
                throw "Cargo cache markers missing; spec drift requires human review: $candidateFullPath"
            }
        }
        "debug-incremental" {
            $expectedIncrementalPath = [IO.Path]::GetFullPath(
                (Join-Path $cleanupTargetRoot "debug\incremental")
            )
            if (-not $candidateFullPath.Equals(
                $expectedIncrementalPath,
                [StringComparison]::OrdinalIgnoreCase
            )) {
                throw "Unexpected incremental target: $candidateFullPath"
            }
        }
        default {
            throw "Unknown cleanup kind: $($staleTargetSpec.Kind)"
        }
    }

    $descendantItems = @(Get-ChildItem -LiteralPath $candidateFullPath -Recurse -Force)
    foreach ($descendantItem in $descendantItems) {
        if (Test-CleanupReparseItem -Item $descendantItem) {
            throw "Reparse point found inside candidate at plan time; refusing: $($descendantItem.FullName)"
        }
    }

    $newestWrite = Get-CleanupNewestWriteTime -Path $candidateFullPath
    $stalenessAgeDays = ((Get-Date) - $newestWrite).TotalDays
    if ($stalenessAgeDays -lt $script:MinStalenessDays) {
        $outcomeEntries += [pscustomobject]@{
            Relative = $staleTargetSpec.Relative
            Path = $candidateFullPath
            Kind = $staleTargetSpec.Kind
            Status = "SkippedNotProvablyStale"
            NewestWriteTimeUtc = $newestWrite.ToUniversalTime().ToString("o")
            StalenessAgeDays = [math]::Round($stalenessAgeDays, 1)
        }
        Write-Output (
            "SKIP not-provably-stale ageDays={0:F1} min={1} {2}" -f $stalenessAgeDays, $script:MinStalenessDays, $candidateFullPath
        )
        continue
    }

    $logicalBytesMeasure = $descendantItems |
        Where-Object { -not $_.PSIsContainer } |
        Measure-Object -Property Length -Sum
    $candidateLogicalBytes = if ($null -eq $logicalBytesMeasure.Sum) {
        [int64]0
    } else {
        [int64]$logicalBytesMeasure.Sum
    }

    $safetySnapshot = Get-CleanupCandidateSafetySnapshot -TargetRoot $cleanupTargetRoot -CandidatePath $candidateFullPath

    $flattenName = $staleTargetSpec.Relative -replace "\\", "__"

    $plannedEntry = [pscustomobject]@{
        Relative = $staleTargetSpec.Relative
        Path = $candidateFullPath
        FlattenName = $flattenName
        Kind = $staleTargetSpec.Kind
        SafetySnapshot = $safetySnapshot
        LogicalBytes = $candidateLogicalBytes
        StalenessAgeDays = [math]::Round($stalenessAgeDays, 1)
        Status = "Planned"
        RecoveryPath = $null
        RebuildInstruction = Get-CleanupRebuildInstruction -Kind $staleTargetSpec.Kind
    }
    $plannedEntries += $plannedEntry
    $outcomeEntries += $plannedEntry
    Write-Output "PLAN $candidateLogicalBytes $candidateFullPath observation=$($safetySnapshot.CandidateObservation) ageDays=$([math]::Round($stalenessAgeDays, 1))"
}

$plannedBytes = if ($plannedEntries.Count -eq 0) {
    [int64]0
} else {
    [int64](($plannedEntries | Measure-Object -Property LogicalBytes -Sum).Sum)
}
Write-Output "PLAN-SUMMARY paths=$($plannedEntries.Count) logicalBytes=$plannedBytes byteMeasure=logical-file-length-sum-not-allocated-disk-usage"

$cleanupOutcome = [ordered]@{
    OutcomeVersion = 1
    ToolVersion = $script:CleanupToolVersion
    Mode = if ($Execute) { "execute-requested" } else { "dry-run" }
    Outcome = if ($Execute) { "ExecutionBlocked" } else { "PlanOnly" }
    ExecutionCapability = Get-CleanupExecutionCapability
    RepositoryRoot = $cleanupRepositoryRoot
    TargetRoot = $cleanupTargetRoot
    MinimumStalenessDays = $script:MinStalenessDays
    PlannedCount = $plannedEntries.Count
    PlannedLogicalBytes = $plannedBytes
    Entries = @($outcomeEntries)
}

if (-not $Execute) {
    Write-CleanupOutcome -Outcome $cleanupOutcome
    Write-Output "DRY-RUN use -Execute after reviewing the fixed absolute paths"
    return
}

# Do not move this capability check below quarantine creation, Git checks, or a
# writer check: an Execute request must make no filesystem mutation while the
# only available rename primitive is path-based and therefore TOCTOU-unsafe.
Write-CleanupOutcome -Outcome $cleanupOutcome
Assert-CleanupExecutionCapability -Operation "Execute cleanup"
