[CmdletBinding()]
param(
    [switch]$Apply,
    [Parameter(DontShow)]
    [switch]$FunctionsOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script:SyndocalCleanupToolVersion = "1.0.0"
$script:SyndocalCleanupHarnessRelativePath = "qa/harnesses/invoke-syndocal-build-cache-cleanup.ps1"
$script:SyndocalCleanupTestMarkerName = ".syndocal-cleanup-selftest.json"
$script:SyndocalCleanupTestRootPrefix = "syndocal-cleanup-selftest-"

# The ASIO audit/v2 directory names were never recorded as exact authoritative
# paths in the tracked cleanup ledger.  They therefore remain deliberately
# unapproved.  Adding even one exact name here requires a new exact-path safety
# review before the next checkpoint can use -Apply.
$script:SyndocalCleanupExactAsioAllowlist = @()
$script:SyndocalCleanupCandidateSpecs = @(
    [pscustomobject]@{
        RelativePath = "target/debug/incremental"
        Kind = "cargo-debug-incremental"
        Recovery = "Regenerable by a frozen-lockfile Cargo debug build using the pinned Windows linker gate."
    }
)

$script:SyndocalCleanupProtectedRelativePaths = @(
    ".",
    "target",
    "target/release",
    "target/qa",
    "target/tmp",
    "target/asio-sdk-2.3.4",
    "target/debug/deps",
    "target/release/deps",
    "target/deps",
    "target/SDK",
    "target/sdk",
    "target/media",
    "SDK",
    "sdk",
    "media"
)

$script:SyndocalCleanupWriterNames = @(
    "cargo",
    "rustc",
    "rustdoc",
    "cl",
    "link",
    "lld-link",
    "lld",
    "mspdbsrv",
    "msbuild",
    "cmake",
    "ninja",
    "rc",
    "mt",
    "node",
    "npm",
    "npx",
    "pnpm",
    "pwsh",
    "powershell"
)

if (-not ("Syndocal.Cleanup.NativePathProbe" -as [type])) {
    Add-Type -TypeDefinition @'
namespace Syndocal.Cleanup
{
    using System;
    using System.ComponentModel;
    using System.Runtime.InteropServices;
    using System.Text;

    public sealed class EntryProbe
    {
        public string FinalPath { get; set; }
        public uint VolumeSerialNumber { get; set; }
        public ulong FileIndex { get; set; }
        public uint LinkCount { get; set; }
    }

    public static class NativePathProbe
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

        [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        private static extern uint GetFinalPathNameByHandleW(
            IntPtr hFile,
            StringBuilder lpszFilePath,
            uint cchFilePath,
            uint dwFlags);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern bool CloseHandle(IntPtr hObject);

        private const uint FileReadAttributes = 0x00000080;
        private const uint FileShareReadWriteDelete = 0x00000007;
        private const uint OpenExisting = 3;
        private const uint FileFlagBackupSemantics = 0x02000000;

        private static string ToDosPath(string path)
        {
            if (path.StartsWith(@"\\?\UNC\", StringComparison.OrdinalIgnoreCase))
            {
                return @"\\" + path.Substring(8);
            }
            if (path.StartsWith(@"\\?\", StringComparison.OrdinalIgnoreCase))
            {
                return path.Substring(4);
            }
            return path;
        }

        public static EntryProbe Probe(string path)
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
                    "Unable to open cleanup entry: " + path);
            }

            try
            {
                BY_HANDLE_FILE_INFORMATION info;
                if (!GetFileInformationByHandle(handle, out info))
                {
                    throw new Win32Exception(
                        Marshal.GetLastWin32Error(),
                        "Unable to query cleanup entry: " + path);
                }

                StringBuilder buffer = new StringBuilder(1024);
                uint length = GetFinalPathNameByHandleW(handle, buffer, (uint)buffer.Capacity, 0);
                if (length == 0)
                {
                    throw new Win32Exception(
                        Marshal.GetLastWin32Error(),
                        "Unable to resolve final cleanup path: " + path);
                }
                if (length >= buffer.Capacity)
                {
                    buffer = new StringBuilder((int)length + 1);
                    length = GetFinalPathNameByHandleW(handle, buffer, (uint)buffer.Capacity, 0);
                    if (length == 0 || length >= buffer.Capacity)
                    {
                        throw new Win32Exception(
                            Marshal.GetLastWin32Error(),
                            "Unable to resolve complete final cleanup path: " + path);
                    }
                }

                return new EntryProbe
                {
                    FinalPath = ToDosPath(buffer.ToString()),
                    VolumeSerialNumber = info.VolumeSerialNumber,
                    FileIndex = ((ulong)info.FileIndexHigh << 32) | info.FileIndexLow,
                    LinkCount = info.NumberOfLinks
                };
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

function New-SyndocalCleanupException {
    param(
        [Parameter(Mandatory)]
        [string]$Code,
        [Parameter(Mandatory)]
        [string]$Message
    )

    $exception = New-Object System.InvalidOperationException($Message)
    $exception.Data["SyndocalCleanupCode"] = $Code
    return $exception
}

function Throw-SyndocalCleanupGate {
    param(
        [Parameter(Mandatory)]
        [string]$Code,
        [Parameter(Mandatory)]
        [string]$Message
    )

    throw (New-SyndocalCleanupException -Code $Code -Message $Message)
}

function Get-SyndocalCleanupErrorCode {
    param(
        [Parameter(Mandatory)]
        [System.Management.Automation.ErrorRecord]$ErrorRecord
    )

    if ($null -ne $ErrorRecord.Exception.Data -and
        $ErrorRecord.Exception.Data.Contains("SyndocalCleanupCode")) {
        return [string]$ErrorRecord.Exception.Data["SyndocalCleanupCode"]
    }
    return "UnexpectedFailure"
}

function Get-SyndocalCleanupFullPath {
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        Throw-SyndocalCleanupGate -Code "EmptyPath" -Message "An empty cleanup path is forbidden."
    }
    try {
        $full = [IO.Path]::GetFullPath($Path)
    } catch {
        Throw-SyndocalCleanupGate -Code "InvalidPath" -Message "The cleanup path is invalid: $Path"
    }
    $root = [IO.Path]::GetPathRoot($full)
    if ($full.Length -gt $root.Length) {
        $full = $full.TrimEnd([char[]]@("\", "/"))
    }
    return $full
}

function Test-SyndocalCleanupPathEqual {
    param(
        [Parameter(Mandatory)]
        [string]$Left,
        [Parameter(Mandatory)]
        [string]$Right
    )

    return (Get-SyndocalCleanupFullPath -Path $Left).Equals(
        (Get-SyndocalCleanupFullPath -Path $Right),
        [StringComparison]::OrdinalIgnoreCase
    )
}

function Test-SyndocalCleanupPathUnder {
    param(
        [Parameter(Mandatory)]
        [string]$Path,
        [Parameter(Mandatory)]
        [string]$Root
    )

    $pathFull = Get-SyndocalCleanupFullPath -Path $Path
    $rootFull = Get-SyndocalCleanupFullPath -Path $Root
    if ($pathFull.Equals($rootFull, [StringComparison]::OrdinalIgnoreCase)) {
        return $false
    }
    return $pathFull.StartsWith(
        $rootFull + [IO.Path]::DirectorySeparatorChar,
        [StringComparison]::OrdinalIgnoreCase
    )
}

function Get-SyndocalCleanupEntryProbe {
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    try {
        return [Syndocal.Cleanup.NativePathProbe]::Probe($Path)
    } catch {
        Throw-SyndocalCleanupGate -Code "PathProbeFailed" -Message "Unable to resolve and identify cleanup path '$Path': $($_.Exception.Message)"
    }
}

function Assert-SyndocalCleanupNoReparseItem {
    param(
        [Parameter(Mandatory)]
        $Item
    )

    if (($Item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        Throw-SyndocalCleanupGate -Code "ReparsePointDetected" -Message "A reparse point is forbidden in a cleanup path: $($Item.FullName)"
    }
}

function Assert-SyndocalCleanupNoReparseAncestry {
    param(
        [Parameter(Mandatory)]
        [string]$Path
    )

    $full = Get-SyndocalCleanupFullPath -Path $Path
    if (-not (Test-Path -LiteralPath $full)) {
        Throw-SyndocalCleanupGate -Code "MissingPath" -Message "Cleanup ancestry path is missing: $full"
    }

    $cursor = Get-Item -LiteralPath $full -Force
    while ($null -ne $cursor) {
        Assert-SyndocalCleanupNoReparseItem -Item $cursor
        $parent = $cursor.Parent
        if ($null -eq $parent -or $parent.FullName -eq $cursor.FullName) {
            break
        }
        $cursor = $parent
    }
}

function Resolve-SyndocalCleanupStaticCandidate {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory)]
        [string]$RelativePath
    )

    if ([IO.Path]::IsPathRooted($RelativePath) -or
        $RelativePath.StartsWith("\\", [StringComparison]::Ordinal) -or
        $RelativePath.Contains("..")) {
        Throw-SyndocalCleanupGate -Code "CandidatePathTraversal" -Message "Only a closed, repository-relative candidate is accepted: $RelativePath"
    }

    $allowed = @($script:SyndocalCleanupCandidateSpecs | ForEach-Object { [string]$_.RelativePath })
    if ($allowed -cnotcontains $RelativePath) {
        Throw-SyndocalCleanupGate -Code "CandidateNotAllowlisted" -Message "The candidate is not in the exact reviewed allowlist: $RelativePath"
    }

    $repositoryFull = Get-SyndocalCleanupFullPath -Path $RepositoryRoot
    $candidateFull = Get-SyndocalCleanupFullPath -Path (Join-Path $repositoryFull ($RelativePath.Replace("/", "\")))
    $targetFull = Get-SyndocalCleanupFullPath -Path (Join-Path $repositoryFull "target")
    if (-not (Test-SyndocalCleanupPathUnder -Path $candidateFull -Root $targetFull)) {
        Throw-SyndocalCleanupGate -Code "CandidateOutsideTarget" -Message "The candidate escaped the exact target root: $candidateFull"
    }
    return $candidateFull
}

function Assert-SyndocalCleanupProtectedBoundary {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory)]
        [string]$CandidatePath
    )

    $repositoryFull = Get-SyndocalCleanupFullPath -Path $RepositoryRoot
    $targetFull = Get-SyndocalCleanupFullPath -Path (Join-Path $repositoryFull "target")
    $candidateFull = Get-SyndocalCleanupFullPath -Path $CandidatePath

    if ((Test-SyndocalCleanupPathEqual -Left $candidateFull -Right $repositoryFull) -or
        (Test-SyndocalCleanupPathEqual -Left $candidateFull -Right $targetFull) -or
        -not (Test-SyndocalCleanupPathUnder -Path $candidateFull -Root $targetFull)) {
        Throw-SyndocalCleanupGate -Code "BroadTargetForbidden" -Message "Repository root, target root, and paths outside target are never cleanup candidates: $candidateFull"
    }

    foreach ($relativeProtected in $script:SyndocalCleanupProtectedRelativePaths) {
        if ($relativeProtected -eq "." -or $relativeProtected -eq "target") {
            # Exact repository/target roots are handled by the broad-target
            # guard above; their reviewed descendants are not blanket-blocked.
            continue
        }
        $protectedFull = if ($relativeProtected -eq ".") {
            $repositoryFull
        } else {
            Get-SyndocalCleanupFullPath -Path (Join-Path $repositoryFull ($relativeProtected.Replace("/", "\")))
        }
        if ((Test-SyndocalCleanupPathEqual -Left $candidateFull -Right $protectedFull) -or
            (Test-SyndocalCleanupPathUnder -Path $candidateFull -Root $protectedFull) -or
            (Test-SyndocalCleanupPathUnder -Path $protectedFull -Root $candidateFull)) {
            Throw-SyndocalCleanupGate -Code "ProtectedPathOverlap" -Message "The candidate overlaps protected path '$protectedFull': $candidateFull"
        }
    }
}

function Assert-SyndocalCleanupTestSeamRoot {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory)]
        [string]$Nonce
    )

    $repositoryFull = Get-SyndocalCleanupFullPath -Path $RepositoryRoot
    $tempFull = Get-SyndocalCleanupFullPath -Path ([IO.Path]::GetTempPath())
    if (-not (Test-SyndocalCleanupPathUnder -Path $repositoryFull -Root $tempFull) -or
        -not ([IO.Path]::GetFileName($repositoryFull)).StartsWith($script:SyndocalCleanupTestRootPrefix, [StringComparison]::Ordinal)) {
        Throw-SyndocalCleanupGate -Code "TestSeamRootForbidden" -Message "The Apply test seam is restricted to a dedicated Syndocal temp mock root."
    }

    $markerPath = Join-Path $repositoryFull $script:SyndocalCleanupTestMarkerName
    if (-not (Test-Path -LiteralPath $markerPath -PathType Leaf)) {
        Throw-SyndocalCleanupGate -Code "TestSeamMarkerMissing" -Message "The dedicated cleanup test marker is missing."
    }
    $markerItem = Get-Item -LiteralPath $markerPath -Force
    Assert-SyndocalCleanupNoReparseItem -Item $markerItem
    try {
        $marker = Get-Content -LiteralPath $markerPath -Raw | ConvertFrom-Json
    } catch {
        Throw-SyndocalCleanupGate -Code "TestSeamMarkerInvalid" -Message "The dedicated cleanup test marker is invalid."
    }
    if ([int]$marker.schemaVersion -ne 1 -or
        [string]$marker.nonce -cne $Nonce -or
        -not (Test-SyndocalCleanupPathEqual -Left ([string]$marker.repositoryRoot) -Right $repositoryFull)) {
        Throw-SyndocalCleanupGate -Code "TestSeamMarkerMismatch" -Message "The dedicated cleanup test marker does not match this invocation."
    }
}

function Invoke-SyndocalCleanupGit {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory)]
        [string[]]$Arguments,
        [Parameter(Mandatory)]
        [string]$FailureCode
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
        $diagnostic = (($output | ForEach-Object { [string]$_ }) -join " ").Trim()
        Throw-SyndocalCleanupGate -Code $FailureCode -Message "Git gate failed ($FailureCode): $diagnostic"
    }
    return @($output | ForEach-Object { [string]$_ })
}

function Assert-SyndocalCleanupRepositoryCheckpoint {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [switch]$TestSeam
    )

    $repositoryFull = Get-SyndocalCleanupFullPath -Path $RepositoryRoot
    $topLevel = (Invoke-SyndocalCleanupGit -RepositoryRoot $repositoryFull -Arguments @("rev-parse", "--show-toplevel") -FailureCode "GitTopLevelUnavailable" | Out-String).Trim()
    $topLevel = Get-SyndocalCleanupFullPath -Path ($topLevel.Replace("/", "\"))
    if (-not (Test-SyndocalCleanupPathEqual -Left $topLevel -Right $repositoryFull)) {
        Throw-SyndocalCleanupGate -Code "CheckoutRootMismatch" -Message "Git top-level does not equal the exact cleanup checkout root: git=$topLevel expected=$repositoryFull"
    }

    $branch = (Invoke-SyndocalCleanupGit -RepositoryRoot $repositoryFull -Arguments @("symbolic-ref", "--quiet", "--short", "HEAD") -FailureCode "BranchUnavailable" | Out-String).Trim()
    $head = (Invoke-SyndocalCleanupGit -RepositoryRoot $repositoryFull -Arguments @("rev-parse", "HEAD") -FailureCode "HeadUnavailable" | Out-String).Trim()
    $upstream = (Invoke-SyndocalCleanupGit -RepositoryRoot $repositoryFull -Arguments @("rev-parse", "@{upstream}") -FailureCode "UpstreamUnavailable" | Out-String).Trim()
    if ($head -cne $upstream) {
        Throw-SyndocalCleanupGate -Code "Diverged" -Message "HEAD must equal its pushed upstream before cleanup: HEAD=$head upstream=$upstream"
    }

    $status = @(Invoke-SyndocalCleanupGit -RepositoryRoot $repositoryFull -Arguments @("status", "--porcelain=v1", "--untracked-files=all") -FailureCode "GitStatusUnavailable")
    $dirtyLines = @($status | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    if ($dirtyLines.Count -ne 0) {
        Throw-SyndocalCleanupGate -Code "DirtyWorktree" -Message "The checkout must be clean before cleanup."
    }

    if (-not $TestSeam) {
        $trackedHarness = @(Invoke-SyndocalCleanupGit -RepositoryRoot $repositoryFull -Arguments @("ls-files", "--", $script:SyndocalCleanupHarnessRelativePath) -FailureCode "HarnessTrackingCheckFailed")
        if (@($trackedHarness | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }).Count -ne 1) {
            Throw-SyndocalCleanupGate -Code "HarnessNotTracked" -Message "The cleanup harness must be tracked at the clean pushed checkpoint before -Apply."
        }
    }

    return [pscustomobject]@{
        Branch = $branch
        Head = $head
        Upstream = $upstream
        Clean = $true
        Toplevel = $topLevel
    }
}

function Assert-SyndocalCleanupCandidateUntracked {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory)]
        [string]$RelativePath
    )

    $gitPath = $RelativePath.Replace("\", "/")
    $tracked = @(Invoke-SyndocalCleanupGit -RepositoryRoot $RepositoryRoot -Arguments @("ls-files", "--", $gitPath) -FailureCode "TrackedFileCheckFailed")
    $tracked = @($tracked | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) })
    if ($tracked.Count -ne 0) {
        Throw-SyndocalCleanupGate -Code "TrackedContentDetected" -Message "The cleanup candidate contains tracked content: $($tracked -join ', ')"
    }
}

function Test-SyndocalCleanupWriterName {
    param(
        [Parameter(Mandatory)]
        [string]$Name
    )

    $baseName = [IO.Path]::GetFileNameWithoutExtension($Name).ToLowerInvariant()
    if ($script:SyndocalCleanupWriterNames -contains $baseName) {
        return $true
    }
    return $baseName.StartsWith("build-script-", [StringComparison]::Ordinal)
}

function Test-SyndocalCleanupOwnershipAnchor {
    param(
        [Parameter(Mandatory)]
        [string]$ExecutablePath,
        [Parameter(Mandatory)]
        [string]$CommandLine,
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory)]
        [string]$TargetRoot
    )

    foreach ($needle in @($RepositoryRoot, $TargetRoot)) {
        if (-not [string]::IsNullOrWhiteSpace($ExecutablePath) -and
            ($ExecutablePath.Equals($needle, [StringComparison]::OrdinalIgnoreCase) -or
             $ExecutablePath.StartsWith($needle + "\", [StringComparison]::OrdinalIgnoreCase))) {
            return $true
        }
        if (-not [string]::IsNullOrWhiteSpace($CommandLine)) {
            $forwardNeedle = $needle.Replace("\", "/")
            $forwardCommand = $CommandLine.Replace("\", "/")
            if ($CommandLine.IndexOf($needle, [StringComparison]::OrdinalIgnoreCase) -ge 0 -or
                $forwardCommand.IndexOf($forwardNeedle, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
                return $true
            }
        }
    }
    return $false
}

function Assert-SyndocalCleanupNoOwnedWriters {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory)]
        [string]$TargetRoot,
        [scriptblock]$CimProvider
    )

    try {
        $processes = if ($null -ne $CimProvider) {
            @(& $CimProvider)
        } else {
            @(Get-CimInstance -ClassName Win32_Process -ErrorAction Stop)
        }
    } catch {
        Throw-SyndocalCleanupGate -Code "CimUnavailable" -Message "Win32_Process inventory failed closed: $($_.Exception.Message)"
    }

    $records = @{}
    foreach ($process in $processes) {
        $nameProperty = $process.PSObject.Properties["Name"]
        $pidProperty = $process.PSObject.Properties["ProcessId"]
        $parentProperty = $process.PSObject.Properties["ParentProcessId"]
        if ($null -eq $nameProperty -or $null -eq $pidProperty -or $null -eq $parentProperty) {
            Throw-SyndocalCleanupGate -Code "CimRecordIncomplete" -Message "Win32_Process returned an incomplete identity record."
        }
        $processId = [int]$pidProperty.Value
        if ($processId -eq $PID) {
            continue
        }
        $name = [string]$nameProperty.Value
        $executableProperty = $process.PSObject.Properties["ExecutablePath"]
        $commandProperty = $process.PSObject.Properties["CommandLine"]
        $executablePath = if ($null -eq $executableProperty -or $null -eq $executableProperty.Value) { "" } else { [string]$executableProperty.Value }
        $commandLine = if ($null -eq $commandProperty -or $null -eq $commandProperty.Value) { "" } else { [string]$commandProperty.Value }

        if ((Test-SyndocalCleanupWriterName -Name $name) -and
            [string]::IsNullOrWhiteSpace($executablePath) -and
            [string]::IsNullOrWhiteSpace($commandLine)) {
            Throw-SyndocalCleanupGate -Code "WriterOwnershipUnverifiable" -Message "Writer ownership metadata is unavailable for PID $processId ($name)."
        }

        $records[$processId] = [pscustomobject]@{
            ProcessId = $processId
            ParentProcessId = [int]$parentProperty.Value
            Name = $name
            ExecutablePath = $executablePath
            CommandLine = $commandLine
            IsWriter = Test-SyndocalCleanupWriterName -Name $name
            IsAnchor = Test-SyndocalCleanupOwnershipAnchor -ExecutablePath $executablePath -CommandLine $commandLine -RepositoryRoot $RepositoryRoot -TargetRoot $TargetRoot
        }
    }

    $children = @{}
    foreach ($record in $records.Values) {
        $parentId = [int]$record.ParentProcessId
        if (-not $children.ContainsKey($parentId)) {
            $children[$parentId] = New-Object System.Collections.ArrayList
        }
        [void]$children[$parentId].Add([int]$record.ProcessId)
    }

    $offenders = @()
    foreach ($record in $records.Values) {
        if (-not $record.IsWriter) {
            continue
        }
        $queue = New-Object System.Collections.Queue
        $seen = @{}
        $queue.Enqueue([int]$record.ProcessId)
        $owned = $false
        while ($queue.Count -gt 0 -and -not $owned) {
            $currentId = [int]$queue.Dequeue()
            if ($seen.ContainsKey($currentId)) {
                continue
            }
            $seen[$currentId] = $true
            if ($records.ContainsKey($currentId)) {
                $current = $records[$currentId]
                if ($current.IsAnchor) {
                    $owned = $true
                    break
                }
                $parentId = [int]$current.ParentProcessId
                if ($parentId -gt 0 -and $records.ContainsKey($parentId)) {
                    $queue.Enqueue($parentId)
                }
            }
            if ($children.ContainsKey($currentId)) {
                foreach ($childId in $children[$currentId]) {
                    $queue.Enqueue([int]$childId)
                }
            }
        }
        if ($owned) {
            $offenders += [pscustomobject]@{
                ProcessId = [int]$record.ProcessId
                Name = [string]$record.Name
                ExecutablePath = [string]$record.ExecutablePath
                CommandLine = [string]$record.CommandLine
            }
        }
    }

    if ($offenders.Count -ne 0) {
        $summary = @($offenders | ForEach-Object { "PID=$($_.ProcessId) name=$($_.Name)" }) -join "; "
        Throw-SyndocalCleanupGate -Code "ActiveOwnedWriter" -Message "Checkout/target-owned build writers are active: $summary"
    }
    return [pscustomobject]@{
        ProcessCount = $records.Count
        OwnedWriterCount = 0
    }
}

function Assert-SyndocalCleanupNoUnknownAsioTargets {
    param(
        [Parameter(Mandatory)]
        [string]$TargetRoot
    )

    $unknown = @(
        Get-ChildItem -LiteralPath $TargetRoot -Force -Directory |
        Where-Object { $_.Name -like "asio-audit-*" -or $_.Name -like "asio-v2-*" } |
        Where-Object { $script:SyndocalCleanupExactAsioAllowlist -cnotcontains $_.Name } |
        Sort-Object Name
    )
    if ($unknown.Count -ne 0) {
        Throw-SyndocalCleanupGate -Code "UnknownAsioCandidate" -Message "Unreviewed ASIO cleanup candidates were found and remain protected: $(@($unknown.Name) -join ', ')"
    }
}

function Get-SyndocalCleanupTreeSnapshot {
    param(
        [Parameter(Mandatory)]
        [string]$CandidatePath,
        [Parameter(Mandatory)]
        [string]$ExpectedPath
    )

    if (-not (Test-Path -LiteralPath $CandidatePath -PathType Container)) {
        Throw-SyndocalCleanupGate -Code "CandidateMissingDuringValidation" -Message "The cleanup candidate disappeared during validation: $CandidatePath"
    }

    Assert-SyndocalCleanupNoReparseAncestry -Path $CandidatePath
    $candidateItem = Get-Item -LiteralPath $CandidatePath -Force
    Assert-SyndocalCleanupNoReparseItem -Item $candidateItem
    $candidateProbe = Get-SyndocalCleanupEntryProbe -Path $CandidatePath
    $resolvedCandidate = Get-SyndocalCleanupFullPath -Path $candidateProbe.FinalPath
    if (-not (Test-SyndocalCleanupPathEqual -Left $resolvedCandidate -Right $ExpectedPath)) {
        Throw-SyndocalCleanupGate -Code "ResolvedCandidateMismatch" -Message "The final resolved candidate path changed: resolved=$resolvedCandidate expected=$ExpectedPath"
    }

    $stack = New-Object System.Collections.Stack
    $stack.Push($candidateItem)
    $entries = New-Object System.Collections.ArrayList
    while ($stack.Count -gt 0) {
        $directory = $stack.Pop()
        foreach ($child in @(Get-ChildItem -LiteralPath $directory.FullName -Force -ErrorAction Stop)) {
            Assert-SyndocalCleanupNoReparseItem -Item $child
            [void]$entries.Add($child)
            if ($child.PSIsContainer) {
                $stack.Push($child)
            }
        }
    }

    [int64]$logicalBytes = 0
    [int64]$newestWriteTicks = $candidateItem.LastWriteTimeUtc.Ticks
    [int]$fileCount = 0
    [int]$directoryCount = 1
    $fingerprintLines = New-Object System.Collections.ArrayList
    foreach ($entry in @($entries | Sort-Object FullName)) {
        $relative = $entry.FullName.Substring($CandidatePath.Length).TrimStart("\")
        if ($entry.LastWriteTimeUtc.Ticks -gt $newestWriteTicks) {
            $newestWriteTicks = $entry.LastWriteTimeUtc.Ticks
        }
        if ($entry.PSIsContainer) {
            $directoryCount++
            [void]$fingerprintLines.Add("D|$relative|$($entry.LastWriteTimeUtc.Ticks)")
        } else {
            $fileCount++
            $probe = Get-SyndocalCleanupEntryProbe -Path $entry.FullName
            if ([uint32]$probe.LinkCount -gt 1) {
                Throw-SyndocalCleanupGate -Code "HardlinkDetected" -Message "A file with link count $($probe.LinkCount) is protected from cleanup: $($entry.FullName)"
            }
            $logicalBytes += [int64]$entry.Length
            [void]$fingerprintLines.Add("F|$relative|$($entry.Length)|$($entry.LastWriteTimeUtc.Ticks)|$($probe.VolumeSerialNumber):$($probe.FileIndex):$($probe.LinkCount)")
        }
    }

    $fingerprintText = @($fingerprintLines) -join "`n"
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $fingerprintBytes = [Text.Encoding]::UTF8.GetBytes($fingerprintText)
        $fingerprint = ([BitConverter]::ToString($sha.ComputeHash($fingerprintBytes))).Replace("-", "")
    } finally {
        $sha.Dispose()
    }

    return [pscustomobject]@{
        Path = $CandidatePath
        ResolvedPath = $resolvedCandidate
        VolumeSerialNumber = [uint32]$candidateProbe.VolumeSerialNumber
        FileIndex = [uint64]$candidateProbe.FileIndex
        LogicalBytes = $logicalBytes
        NewestWriteTicks = $newestWriteTicks
        NewestWriteTimeUtc = ([DateTime]::new($newestWriteTicks, [DateTimeKind]::Utc)).ToString("o")
        FileCount = $fileCount
        DirectoryCount = $directoryCount
        FingerprintSha256 = $fingerprint
    }
}

function Assert-SyndocalCleanupSnapshotsEqual {
    param(
        [Parameter(Mandatory)]
        $Expected,
        [Parameter(Mandatory)]
        $Actual,
        [string]$FailureCode = "CandidateChanged"
    )

    $same =
        $Expected.ResolvedPath.Equals($Actual.ResolvedPath, [StringComparison]::OrdinalIgnoreCase) -and
        [uint32]$Expected.VolumeSerialNumber -eq [uint32]$Actual.VolumeSerialNumber -and
        [uint64]$Expected.FileIndex -eq [uint64]$Actual.FileIndex -and
        [int64]$Expected.LogicalBytes -eq [int64]$Actual.LogicalBytes -and
        [int64]$Expected.NewestWriteTicks -eq [int64]$Actual.NewestWriteTicks -and
        [int]$Expected.FileCount -eq [int]$Actual.FileCount -and
        [int]$Expected.DirectoryCount -eq [int]$Actual.DirectoryCount -and
        [string]$Expected.FingerprintSha256 -ceq [string]$Actual.FingerprintSha256
    if (-not $same) {
        Throw-SyndocalCleanupGate -Code $FailureCode -Message "Candidate size/mtime/identity changed between safety samples: $($Expected.Path)"
    }
}

function New-SyndocalCleanupReport {
    param(
        [Parameter(Mandatory)]
        [string]$Mode,
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory)]
        [string]$TargetRoot
    )

    return [ordered]@{
        SchemaVersion = 1
        ToolVersion = $script:SyndocalCleanupToolVersion
        Mode = $Mode
        Outcome = "Evaluating"
        RepositoryRoot = $RepositoryRoot
        TargetRoot = $TargetRoot
        ExactAllowlist = @($script:SyndocalCleanupCandidateSpecs | ForEach-Object { [string]$_.RelativePath })
        ExactAsioAllowlist = @($script:SyndocalCleanupExactAsioAllowlist)
        Repository = $null
        Candidates = @()
        Blocker = $null
        PlannedLogicalBytes = [int64]0
        ReclaimedLogicalBytes = [int64]0
        Recoverability = "Deleted artifacts are not sent to the Recycle Bin; successful deletion is recoverable only by the recorded rebuild instruction."
    }
}

function Invoke-SyndocalBuildCacheCleanupCore {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [switch]$Apply,
        [switch]$TestSeam,
        [string]$TestSeamNonce,
        [hashtable]$TestHooks
    )

    $repositoryFull = Get-SyndocalCleanupFullPath -Path $RepositoryRoot
    $targetFull = Get-SyndocalCleanupFullPath -Path (Join-Path $repositoryFull "target")
    $report = New-SyndocalCleanupReport -Mode $(if ($Apply) { "Apply" } else { "Plan" }) -RepositoryRoot $repositoryFull -TargetRoot $targetFull

    try {
        if ($TestSeam) {
            Assert-SyndocalCleanupTestSeamRoot -RepositoryRoot $repositoryFull -Nonce $TestSeamNonce
        } elseif ($null -ne $TestHooks) {
            Throw-SyndocalCleanupGate -Code "TestHookForbidden" -Message "Dependency-injection hooks are restricted to the dedicated temp test seam."
        }

        if (-not (Test-Path -LiteralPath $repositoryFull -PathType Container)) {
            Throw-SyndocalCleanupGate -Code "CheckoutMissing" -Message "The exact checkout root is missing: $repositoryFull"
        }
        if (-not (Test-Path -LiteralPath $targetFull -PathType Container)) {
            Throw-SyndocalCleanupGate -Code "TargetRootMissing" -Message "The exact target root is missing: $targetFull"
        }
        Assert-SyndocalCleanupNoReparseAncestry -Path $repositoryFull
        Assert-SyndocalCleanupNoReparseAncestry -Path $targetFull
        $repositoryProbe = Get-SyndocalCleanupEntryProbe -Path $repositoryFull
        $targetProbe = Get-SyndocalCleanupEntryProbe -Path $targetFull
        if (-not (Test-SyndocalCleanupPathEqual -Left $repositoryProbe.FinalPath -Right $repositoryFull)) {
            Throw-SyndocalCleanupGate -Code "CheckoutResolvedMismatch" -Message "The checkout final path does not equal its absolute root."
        }
        if (-not (Test-SyndocalCleanupPathEqual -Left $targetProbe.FinalPath -Right $targetFull)) {
            Throw-SyndocalCleanupGate -Code "TargetResolvedMismatch" -Message "The target final path does not equal the exact checkout target root."
        }

        Assert-SyndocalCleanupNoUnknownAsioTargets -TargetRoot $targetFull
        $report.Repository = Assert-SyndocalCleanupRepositoryCheckpoint -RepositoryRoot $repositoryFull -TestSeam:$TestSeam

        $cimProvider = $null
        if ($TestSeam -and $null -ne $TestHooks -and $TestHooks.ContainsKey("CimProvider")) {
            $cimProvider = [scriptblock]$TestHooks["CimProvider"]
        }
        [void](Assert-SyndocalCleanupNoOwnedWriters -RepositoryRoot $repositoryFull -TargetRoot $targetFull -CimProvider $cimProvider)

        $stabilityDelayMilliseconds = 750
        if ($TestSeam -and $null -ne $TestHooks -and $TestHooks.ContainsKey("StabilityDelayMilliseconds")) {
            $stabilityDelayMilliseconds = [int]$TestHooks["StabilityDelayMilliseconds"]
        }
        if ($stabilityDelayMilliseconds -lt 0 -or $stabilityDelayMilliseconds -gt 5000) {
            Throw-SyndocalCleanupGate -Code "InvalidStabilityDelay" -Message "The safety sample delay is outside its bounded range."
        }

        $planned = @()
        foreach ($spec in $script:SyndocalCleanupCandidateSpecs) {
            $candidateFull = Resolve-SyndocalCleanupStaticCandidate -RepositoryRoot $repositoryFull -RelativePath $spec.RelativePath
            Assert-SyndocalCleanupProtectedBoundary -RepositoryRoot $repositoryFull -CandidatePath $candidateFull
            if (-not (Test-Path -LiteralPath $candidateFull -PathType Container)) {
                $planned += [pscustomobject]@{
                    RelativePath = $spec.RelativePath
                    AbsolutePath = $candidateFull
                    Status = "Missing"
                    LogicalBytesBefore = [int64]0
                    LogicalBytesAfter = [int64]0
                    ReclaimedLogicalBytes = [int64]0
                    Recoverability = $spec.Recovery
                    RecoveryDisposition = "NotNeeded"
                }
                continue
            }

            Assert-SyndocalCleanupCandidateUntracked -RepositoryRoot $repositoryFull -RelativePath $spec.RelativePath
            $first = Get-SyndocalCleanupTreeSnapshot -CandidatePath $candidateFull -ExpectedPath $candidateFull
            if ($TestSeam -and $null -ne $TestHooks -and $TestHooks.ContainsKey("BeforeSecondSample")) {
                & ([scriptblock]$TestHooks["BeforeSecondSample"]) $candidateFull
            }
            if ($stabilityDelayMilliseconds -gt 0) {
                Start-Sleep -Milliseconds $stabilityDelayMilliseconds
            }
            $second = Get-SyndocalCleanupTreeSnapshot -CandidatePath $candidateFull -ExpectedPath $candidateFull
            Assert-SyndocalCleanupSnapshotsEqual -Expected $first -Actual $second -FailureCode "CandidateUnstable"

            $planned += [pscustomobject]@{
                RelativePath = $spec.RelativePath
                AbsolutePath = $candidateFull
                Status = "Planned"
                LogicalBytesBefore = [int64]$second.LogicalBytes
                LogicalBytesAfter = [int64]$second.LogicalBytes
                ReclaimedLogicalBytes = [int64]0
                NewestWriteTimeUtc = $second.NewestWriteTimeUtc
                FingerprintSha256 = $second.FingerprintSha256
                CandidateSnapshot = $second
                Recoverability = $spec.Recovery
                RecoveryDisposition = "RebuildOnly"
            }
        }

        $report.Candidates = @($planned)
        $report.PlannedLogicalBytes = if ($planned.Count -eq 0) { [int64]0 } else { [int64](($planned | Measure-Object -Property LogicalBytesBefore -Sum).Sum) }
        if (-not $Apply) {
            $report.Outcome = if (@($planned | Where-Object { $_.Status -eq "Planned" }).Count -eq 0) { "PlanEmpty" } else { "PlanReady" }
            return [pscustomobject]$report
        }

        $removeOperation = {
            param([string]$Path)
            Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction Stop
        }
        if ($TestSeam -and $null -ne $TestHooks -and $TestHooks.ContainsKey("RemoveOperation")) {
            $removeOperation = [scriptblock]$TestHooks["RemoveOperation"]
        }

        foreach ($entry in @($planned | Where-Object { $_.Status -eq "Planned" })) {
            try {
                # Every destructive candidate re-runs the mutable global and
                # per-path gates.  The final path probe is deliberately adjacent
                # to the LiteralPath-only Remove-Item call.
                $report.Repository = Assert-SyndocalCleanupRepositoryCheckpoint -RepositoryRoot $repositoryFull -TestSeam:$TestSeam
                [void](Assert-SyndocalCleanupNoOwnedWriters -RepositoryRoot $repositoryFull -TargetRoot $targetFull -CimProvider $cimProvider)
                Assert-SyndocalCleanupNoUnknownAsioTargets -TargetRoot $targetFull
                Assert-SyndocalCleanupCandidateUntracked -RepositoryRoot $repositoryFull -RelativePath $entry.RelativePath
                Assert-SyndocalCleanupProtectedBoundary -RepositoryRoot $repositoryFull -CandidatePath $entry.AbsolutePath

                if ($TestSeam -and $null -ne $TestHooks -and $TestHooks.ContainsKey("BeforeFinalValidation")) {
                    & ([scriptblock]$TestHooks["BeforeFinalValidation"]) $entry.AbsolutePath
                }

                $final = Get-SyndocalCleanupTreeSnapshot -CandidatePath $entry.AbsolutePath -ExpectedPath $entry.AbsolutePath
                Assert-SyndocalCleanupSnapshotsEqual -Expected $entry.CandidateSnapshot -Actual $final -FailureCode "CandidateChanged"
                $finalProbe = Get-SyndocalCleanupEntryProbe -Path $entry.AbsolutePath
                if (-not (Test-SyndocalCleanupPathEqual -Left $finalProbe.FinalPath -Right $entry.AbsolutePath)) {
                    Throw-SyndocalCleanupGate -Code "FinalResolvedPathMismatch" -Message "The final resolved deletion path is not the exact reviewed candidate."
                }
                if (-not (Test-SyndocalCleanupPathUnder -Path $finalProbe.FinalPath -Root $targetFull)) {
                    Throw-SyndocalCleanupGate -Code "FinalResolvedPathOutsideTarget" -Message "The final resolved deletion path escaped target."
                }

                & $removeOperation ([string]$entry.AbsolutePath)
                if (Test-Path -LiteralPath $entry.AbsolutePath) {
                    Throw-SyndocalCleanupGate -Code "DeleteIncomplete" -Message "Remove-Item returned but the exact candidate still exists."
                }
                $entry.Status = "Deleted"
                $entry.LogicalBytesAfter = [int64]0
                $entry.ReclaimedLogicalBytes = [int64]$entry.LogicalBytesBefore
                $entry.RecoveryDisposition = "RebuildOnly"
                $report.ReclaimedLogicalBytes += [int64]$entry.ReclaimedLogicalBytes
            } catch {
                $code = Get-SyndocalCleanupErrorCode -ErrorRecord $_
                $remainingBytes = [int64]0
                $stillExists = Test-Path -LiteralPath $entry.AbsolutePath
                if ($stillExists) {
                    try {
                        $remaining = Get-SyndocalCleanupTreeSnapshot -CandidatePath $entry.AbsolutePath -ExpectedPath $entry.AbsolutePath
                        $remainingBytes = [int64]$remaining.LogicalBytes
                    } catch {
                        $remainingBytes = [int64]-1
                    }
                }
                $entry.Status = if ($stillExists -and $remainingBytes -ge 0 -and $remainingBytes -lt [int64]$entry.LogicalBytesBefore) { "DeleteFailedPartial" } else { "BlockedBeforeDelete" }
                $entry.LogicalBytesAfter = $remainingBytes
                $entry.ReclaimedLogicalBytes = if ($remainingBytes -ge 0) { [Math]::Max([int64]0, [int64]$entry.LogicalBytesBefore - $remainingBytes) } else { [int64]0 }
                $entry | Add-Member -NotePropertyName FailureCode -NotePropertyValue $code -Force
                $entry | Add-Member -NotePropertyName FailureMessage -NotePropertyValue $_.Exception.Message -Force
                $entry | Add-Member -NotePropertyName ExistsAfterFailure -NotePropertyValue $stillExists -Force
                $report.ReclaimedLogicalBytes += [int64]$entry.ReclaimedLogicalBytes
                $report.Outcome = if ($entry.Status -eq "DeleteFailedPartial") { "PartialFailure" } else { "Blocked" }
                $report.Blocker = [pscustomobject]@{ Code = $code; Message = $_.Exception.Message }
                return [pscustomobject]$report
            }
        }

        $report.Outcome = if (@($planned | Where-Object { $_.Status -eq "Deleted" }).Count -eq 0) { "ApplyEmpty" } else { "Applied" }
        return [pscustomobject]$report
    } catch {
        $report.Outcome = "Blocked"
        $report.Blocker = [pscustomobject]@{
            Code = Get-SyndocalCleanupErrorCode -ErrorRecord $_
            Message = $_.Exception.Message
        }
        return [pscustomobject]$report
    }
}

if ($FunctionsOnly) {
    return
}

$productionRepositoryRoot = Get-SyndocalCleanupFullPath -Path (Join-Path $PSScriptRoot "../..")
$productionReport = Invoke-SyndocalBuildCacheCleanupCore -RepositoryRoot $productionRepositoryRoot -Apply:$Apply
$productionJson = $productionReport | ConvertTo-Json -Depth 12 -Compress
Write-Output "SYNDOCAL-CLEANUP-REPORT $productionJson"

if ($Apply -and $productionReport.Outcome -notin @("Applied", "ApplyEmpty")) {
    exit 2
}
