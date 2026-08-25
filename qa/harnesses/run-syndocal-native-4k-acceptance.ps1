# Safe native 4K Syndocal acceptance runner.
#
# PURPOSE
#   Drives (only with explicit authorization) and verifies the physical 4K
#   acceptance state of THIS checkout's Syndocal main window on the pinned
#   physical monitor tuple: description "MSI3DD2", physical resolution
#   3840x2160, effective DPI 144, with a maximized, responsive main window
#   whose physical client area is 3840x2088 (logical 2560x1392 at 150%).
#
# MODE CONTRACT
#   Default invocation is a DRY-RUN: strictly read-only observation. It
#   resolves the target process/window, records provenance, display topology,
#   a before-sample, and an informational stability evaluation, and mutates
#   NOTHING: no window-state API is reachable without -Apply.
#   State changes require ALL of the following together:
#     -Apply
#     -ExpectedExecutablePath            exact absolute path, leaf syndocal.exe
#     -ExpectedSha256                    64 hex chars of that exact file
#     -ExpectedProductVersion            exact ProductVersion string
#     -ExpectedGitHead                   40 hex chars of this checkout HEAD
#     -ExpectedMonitorIdentifier         must equal the pinned "MSI3DD2"
#     -ExpectedMonitorPhysicalResolution must equal "3840x2160"
#     -ExpectedEffectiveDpi              must equal 144
#   Anything missing, malformed, or off-pin fails closed before any other
#   stage runs. Dry-run does not require these parameters; any that are
#   provided must still be well formed.
#
# IDENTITY CONTRACT (inherits qa/harnesses/observe-syndocal-window-geometry.ps1)
#   The observer harness is DOT-SOURCED below and its fail-closed machinery is
#   reused verbatim: candidate enumeration limited to the process name,
#   native image path via QueryFullProcessImageNameW, CreateFileW plus
#   GetFinalPathNameByHandleW plus volume-serial/file-index identity, and
#   exactly-one exact-match selection (zero/multiple abort). The main window
#   must be exactly one visible top-level window titled exactly "Syndocal"
#   (ordinal), alive, owned by the proven PID, and responsive
#   (IsHungAppWindow). Window-to-process ownership is re-proved through
#   GetWindowThreadProcessId at selection, before every state change, and in
#   every stability sample. DISPLAY ordinals and window titles are recorded
#   for diagnostics only and are NEVER accepted as identity proof: the target
#   physical monitor is selected solely by the exact tuple (description
#   string, physical pixel bounds, effective DPI); duplicates of that tuple
#   are ambiguous and fatal in apply mode, and zero matches are fatal.
#
# MUTATION CONTRACT (apply mode only, this tranche)
#   At most three operations, each immediately preceded by a full pre-change
#   revalidation (HWND alive, owner PID, responsiveness, deep process
#   identity re-proof including SHA256 and product version, and window still
#   on the target physical monitor):
#     1. SW_RESTORE   (ShowWindow) when the window is iconic or must leave
#                     the maximized state to be moved across monitors
#     2. SetWindowPos re-centering the window onto the target monitor work
#                     area when its nearest monitor is not the target
#     3. SW_MAXIMIZE  (ShowWindow) when not maximized
#   Every mutation goes exclusively through the seam wrappers
#   Invoke-ShowWindowState / Invoke-SetWindowPlacementRect. There is NO
#   process launch, NO process termination, NO pane-child operation, NO
#   screen capture, NO foreground stealing, and NO network access anywhere
#   in this file.
#
# STABILITY CONTRACT
#   Acceptance requires 3 CONSECUTIVE stable samples separated by at least
#   200 ms (parameter floor enforced; default 250 ms). Each sample must show:
#   target monitor tuple match, window effective DPI 144, maximized, not
#   minimized, responsive, alive and owned by the proven PID, physical client
#   3840x2088, logical client 2560x1392. In apply mode every sample also
#   re-proves deep process identity (fresh candidate selection plus SHA256
#   plus product version). Any unstable sample resets the consecutive count.
#   Dry-run evaluates the identical criteria informationally against the
#   pinned constants and never fails because criteria are unmet; it fails
#   only on operational errors.
#
# EVIDENCE CONTRACT
#   Evidence is written to a NEW, previously nonexistent, non-reparse DIRECT
#   child of the evidence root (default
#   <checkout>\qa\artifacts\native-physical-acceptance) whose name is a safe
#   slug: lowercase alphanumeric start, then [a-z0-9._-], 1..64 characters;
#   Windows reserved device names rejected; trailing dot rejected; traversal
#   impossible by construction and re-verified via leaf-name and prefix
#   assertions. The whole ancestry of the root is walked rejecting reparse
#   points, and the immediate parent must be owned by the current user,
#   BUILTIN\Administrators, or NT AUTHORITY\SYSTEM. An existing candidate is
#   never reused, merged, or overwritten. Artifacts (JSON emitted with stable
#   insertion-ordered keys; timestamps are the only run-varying values):
#     provenance.json  tool/mode/host/checkout/expectations/target identity
#     display.json     enumerated monitor identities + target selection
#     before.json      pre-operation sample of the main window
#     operation.json   per-operation revalidation + mutation audit
#                      (dry-run: performed=false with reason)
#     final.json       stability samples, delays, consecutive count, verdict
#     failure.json     written ONLY on failure (stage + message)
#     SHA256SUMS.txt   sha256 of every artifact, written/refreshed LAST
#   Evidence is ALWAYS retained, including on failure; the runner never
#   deletes evidence. Screenshots are NOT taken and NOT claimed.
#
# EXIT CODES
#   0 only when the mode's contract is satisfied (apply: all preconditions,
#   revalidations, mutations, and 3 consecutive stable samples; dry-run:
#   read-only observation and evidence completed). Any failure exits nonzero
#   with diagnostics on stderr and evidence retained.
#
# SELF-TEST SEAMS
#   Dot-sourcing this file defines functions only (invocation guard at the
#   bottom). qa/harnesses/test-run-syndocal-native-4k-acceptance.ps1 drives
#   every native and filesystem boundary through seam overrides (process
#   identity seams inherited from the observer plus every function marked
#   SEAM below) and never enumerates or mutates a real window or process.
#   Its only real filesystem traffic is inside its own temporary sandbox.

[CmdletBinding()]
param(
  [switch]$Apply,

  [string]$ExpectedExecutablePath = "",
  [string]$ExpectedSha256 = "",
  [string]$ExpectedProductVersion = "",
  [string]$ExpectedGitHead = "",
  [string]$ExpectedMonitorIdentifier = "",
  [string]$ExpectedMonitorPhysicalResolution = "",
  [int]$ExpectedEffectiveDpi = 0,

  [string]$EvidenceSlug = "",
  [string]$EvidenceRootPath = "",

  [int]$SampleIntervalMs = 250,
  [int]$MaxSampleAttempts = 24
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$script:AcceptanceHarnessFilePath = $PSCommandPath
$script:CheckoutRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$script:AcceptanceMainWindowTitle = "Syndocal"

$script:PinnedAcceptanceConstants = [pscustomobject]@{
  MonitorIdentifier = "MSI3DD2"
  MonitorPhysicalResolution = "3840x2160"
  EffectiveDpi = 144
  ClientPhysicalWidth = 3840
  ClientPhysicalHeight = 2088
  ClientLogicalWidth = 2560
  ClientLogicalHeight = 1392
  RequiredConsecutiveStableSamples = 3
  MinimumSampleIntervalMs = 200
}

$script:SwRestoreCommand = 9
$script:SwMaximizeCommand = 3
$script:SetWindowPosNoZOrder = 0x0004
$script:SetWindowPosNoActivate = 0x0010

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
  throw "The Syndocal native 4K acceptance runner requires Windows."
}

# Reuse the observer harness contracts when the running host can parse it.
# Its invocation guard defines functions only on dot-source, including its
# fail-closed process identity machinery and its read-only native probes.
# Windows PowerShell 5.1 cannot parse that file's uint literal suffixes; in
# that case the equivalent functions are defined below from this file so the
# contracts hold identically on both hosts (cast-based, same fail-closed
# semantics and seam names).
$script:ObserverContractsLoaded = $false
if (-not (Get-Command -Name "Select-SyndocalTargetProcessId" -ErrorAction SilentlyContinue)) {
  try {
    . (Join-Path $PSScriptRoot "observe-syndocal-window-geometry.ps1")
    $script:ObserverContractsLoaded = $true
  } catch {
    # Expected only on hosts that cannot parse the observer source; the
    # fallback region below supplies contract-equivalent implementations.
  }
}

if (-not $script:ObserverContractsLoaded) {
  # BEGIN PS 5.1 FALLBACK (contract-equivalent to the observer harness).
  $script:SyndocalProcessName = "syndocal"

  function Get-ObserverWindowTitle {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    $length = [SyndocalAcceptanceRunnerNative]::GetWindowTextLength($Handle)
    if ($length -le 0) { return "" }
    $builder = [Text.StringBuilder]::new($length + 1)
    [void][SyndocalAcceptanceRunnerNative]::GetWindowText($Handle, $builder, $builder.Capacity)
    $builder.ToString()
  }

  function Get-SyndocalCandidateProcessIds {
    $candidates =
      @(Get-Process -Name $script:SyndocalProcessName `
          -ErrorAction SilentlyContinue -ErrorVariable enumerationErrors)
    foreach ($errorRecord in @($enumerationErrors)) {
      if ("$($errorRecord.FullyQualifiedErrorId)" -notmatch "^NoProcessFound\b") {
        throw (
          "Fail closed: syndocal candidate enumeration failed " +
          "(error id '$($errorRecord.FullyQualifiedErrorId)'): $($errorRecord.Exception.Message)")
      }
    }
    [uint32[]]@($candidates | ForEach-Object { [uint32]$_.Id })
  }

  function ConvertTo-NormalizedFinalPath {
    param([Parameter(Mandatory = $true)][string]$FinalPath)

    if (-not $FinalPath.StartsWith("\\?\", [StringComparison]::Ordinal)) {
      throw "Unrecognized final path without the '\\?\' prefix: '$FinalPath'."
    }
    $withoutPrefix = $FinalPath.Substring(4)
    if ($withoutPrefix.StartsWith("UNC\", [StringComparison]::OrdinalIgnoreCase)) {
      return "\" + $withoutPrefix.Substring(3)
    }
    return $withoutPrefix
  }

  function Get-NativeProcessImagePath {
    param([Parameter(Mandatory = $true)][uint32]$ProcessId)

    $processHandle = [SyndocalAcceptanceRunnerNative]::OpenProcess([uint32]0x1000, $false, $ProcessId)
    if ($processHandle -eq [IntPtr]::Zero) {
      throw (
        "Fail closed: syndocal candidate PID $ProcessId image-path query failed; " +
        "OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION) returned no handle " +
        "(Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())).")
    }
    try {
      $capacityChars = [uint32]1024
      while ($true) {
        $buffer = [Text.StringBuilder]::new([int]$capacityChars)
        $lengthChars = $capacityChars
        $ok = [SyndocalAcceptanceRunnerNative]::QueryFullProcessImageNameW(
          $processHandle, [uint32]0, $buffer, [ref]$lengthChars)
        if ($ok) {
          $imagePath = $buffer.ToString()
          if ([string]::IsNullOrWhiteSpace($imagePath)) {
            throw "Fail closed: syndocal candidate PID $ProcessId image-path query is incomplete (empty result)."
          }
          return $imagePath
        }
        $lastError = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
        if ($lastError -eq 122 -and $capacityChars -lt [uint32]32768) {
          $capacityChars = [uint32]($capacityChars * 2)
          continue
        }
        throw (
          "Fail closed: syndocal candidate PID $ProcessId image-path query failed; " +
          "QueryFullProcessImageNameW returned FALSE (Win32 error $lastError).")
      }
    } finally {
      [void][SyndocalAcceptanceRunnerNative]::CloseHandle($processHandle)
    }
  }

  function Open-StableIdentityHandle {
    param([Parameter(Mandatory = $true)][string]$Path)

    $handle = [SyndocalAcceptanceRunnerNative]::CreateFileW(
      $Path, [uint32]0x80, [uint32]7, [IntPtr]::Zero, 3, 0x02000000, [IntPtr]::Zero)
    if ($handle -eq [IntPtr](-1)) {
      throw (
        "Fail closed: stable-identity handle could not be opened for '$Path' " +
        "(Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())).")
    }
    return $handle
  }

  function Get-FinalPathFromHandle {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    $requiredChars = [SyndocalAcceptanceRunnerNative]::GetFinalPathNameByHandleW($Handle, $null, [uint32]0, [uint32]0)
    if ($requiredChars -eq [uint32]0) {
      throw (
        "Fail closed: GetFinalPathNameByHandleW sizing call failed " +
        "(Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())).")
    }
    $buffer = [Text.StringBuilder]::new([int]$requiredChars)
    $writtenChars = [SyndocalAcceptanceRunnerNative]::GetFinalPathNameByHandleW($Handle, $buffer, $requiredChars, [uint32]0)
    if ($writtenChars -eq [uint32]0) {
      throw (
        "Fail closed: GetFinalPathNameByHandleW failed " +
        "(Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())).")
    }
    return $buffer.ToString()
  }

  function Get-FileIdentityFromHandle {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    $information = [SyndocalAcceptanceRunnerNative+BY_HANDLE_FILE_INFORMATION]::new()
    if (-not [SyndocalAcceptanceRunnerNative]::GetFileInformationByHandle($Handle, [ref]$information)) {
      throw (
        "Fail closed: GetFileInformationByHandle failed " +
        "(Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())).")
    }
    [pscustomobject]@{
      VolumeSerialNumber = [uint32]$information.VolumeSerialNumber
      FileIndex = (([uint64]$information.FileIndexHigh) -shl 32) -bor [uint64]$information.FileIndexLow
    }
  }

  function Test-FileIdentityEqual {
    param(
      [Parameter(Mandatory = $true)]$Left,
      [Parameter(Mandatory = $true)]$Right
    )

    [bool](
      ([uint32]$Left.VolumeSerialNumber -eq [uint32]$Right.VolumeSerialNumber) -and
      ([uint64]$Left.FileIndex -eq [uint64]$Right.FileIndex)
    )
  }

  function Get-CheckoutTargetExecutableFingerprint {
    param([Parameter(Mandatory = $true)][string]$TargetExecutablePath)

    if (-not (Test-Path -LiteralPath $TargetExecutablePath -PathType Leaf)) {
      throw "Exact checkout executable '$TargetExecutablePath' does not exist; refusing to guess another syndocal.exe."
    }
    $requestedPath = [IO.Path]::GetFullPath($TargetExecutablePath)
    $identityHandle = Open-StableIdentityHandle -Path $requestedPath
    try {
      $finalPath = ConvertTo-NormalizedFinalPath -FinalPath (Get-FinalPathFromHandle -Handle $identityHandle)
      $identity = Get-FileIdentityFromHandle -Handle $identityHandle
    } finally {
      [void][SyndocalAcceptanceRunnerNative]::CloseHandle($identityHandle)
    }
    [pscustomobject]@{
      RequestedPath = $requestedPath
      FinalPath = $finalPath
      Identity = $identity
    }
  }

  function Select-SyndocalTargetProcessId {
    param(
      [Parameter(Mandatory = $true)][AllowEmptyCollection()][uint32[]]$CandidateProcessIds,
      [Parameter(Mandatory = $true)]$TargetFingerprint
    )

    $candidateIds = @($CandidateProcessIds)
    if ($candidateIds.Count -eq 0) {
      throw (
        "Fail closed: found 0 syndocal-named processes ('$($script:SyndocalProcessName)'); " +
        "expected exactly one exact identity match for '$($TargetFingerprint.RequestedPath)'.")
    }

    $resolutions = [System.Collections.Generic.List[object]]::new()
    foreach ($candidateId in $candidateIds) {
      try {
        $nativeImagePath = Get-NativeProcessImagePath -ProcessId $candidateId
      } catch {
        throw "Fail closed: syndocal candidate PID $candidateId image-path query failed; $($_.Exception.Message)"
      }
      if ([string]::IsNullOrWhiteSpace($nativeImagePath)) {
        throw "Fail closed: syndocal candidate PID $candidateId image-path query is incomplete (empty result)."
      }
      $identityHandle = [IntPtr]::Zero
      try {
        $identityHandle = Open-StableIdentityHandle -Path $nativeImagePath
        $finalPath = ConvertTo-NormalizedFinalPath -FinalPath (Get-FinalPathFromHandle -Handle $identityHandle)
        $identity = Get-FileIdentityFromHandle -Handle $identityHandle
      } catch {
        throw "Fail closed: syndocal candidate PID $candidateId native file-identity query failed; $($_.Exception.Message)"
      } finally {
        if ($identityHandle -ne [IntPtr]::Zero) {
          [void][SyndocalAcceptanceRunnerNative]::CloseHandle($identityHandle)
        }
      }
      $isExactMatch =
        ([string]::Equals($finalPath, $TargetFingerprint.FinalPath, [StringComparison]::OrdinalIgnoreCase)) -and
        (Test-FileIdentityEqual -Left $identity -Right $TargetFingerprint.Identity)
      $resolutions.Add([pscustomobject]@{
        ProcessId = $candidateId
        FinalPath = $finalPath
        Identity = $identity
        IsExactMatch = $isExactMatch
      })
    }

    $exactMatches = @($resolutions | Where-Object { $_.IsExactMatch })
    if ($exactMatches.Count -gt 1) {
      $matchedPidList = (@($exactMatches | ForEach-Object { "$($_.ProcessId)" }) -join ", ")
      throw (
        "Fail closed: found $($exactMatches.Count) exact identity matches for " +
        "'$($TargetFingerprint.RequestedPath)' (PIDs $matchedPidList).")
    }
    if ($exactMatches.Count -eq 0) {
      throw (
        "Fail closed: found 0 exact identity matches for '$($TargetFingerprint.RequestedPath)' " +
        "among $($candidateIds.Count) syndocal candidate(s).")
    }
    return [uint32]$exactMatches[0].ProcessId
  }

  function Get-VisibleTopLevelWindowsForPid {
    param([Parameter(Mandatory = $true)][uint32]$OwnerPid)

    $visibleTopLevel = [System.Collections.Generic.List[object]]::new()
    $topLevelCallback = [SyndocalAcceptanceRunnerNative+SyndocalAcceptanceWindowEnumProc]{
      param([IntPtr]$Handle, [IntPtr]$Unused)
      [uint32]$windowOwnerPid = 0
      [void][SyndocalAcceptanceRunnerNative]::GetWindowThreadProcessId($Handle, [ref]$windowOwnerPid)
      if ($windowOwnerPid -eq $OwnerPid -and [SyndocalAcceptanceRunnerNative]::IsWindowVisible($Handle)) {
        $visibleTopLevel.Add([pscustomobject]@{
          Handle = $Handle
          Title = Get-ObserverWindowTitle -Handle $Handle
        })
      }
      return $true
    }
    [void][SyndocalAcceptanceRunnerNative]::EnumWindows($topLevelCallback, [IntPtr]::Zero)
    return @($visibleTopLevel | Sort-Object -Property { $_.Handle.ToInt64() })
  }

  function Test-NativeWindowAlive {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    [bool][SyndocalAcceptanceRunnerNative]::IsWindow($Handle)
  }

  function Get-WindowOwnerProcessId {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    [uint32]$windowOwnerPid = 0
    [void][SyndocalAcceptanceRunnerNative]::GetWindowThreadProcessId($Handle, [ref]$windowOwnerPid)
    $windowOwnerPid
  }

  function Test-WindowNotHung {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    -not [bool][SyndocalAcceptanceRunnerNative]::IsHungAppWindow($Handle)
  }

  function Get-ObservationWindowRectangles {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    $outerRect = [SyndocalAcceptanceRunnerNative+RECT]::new()
    if (-not [SyndocalAcceptanceRunnerNative]::GetWindowRect($Handle, [ref]$outerRect)) {
      throw "Fail closed: GetWindowRect failed for HWND $($Handle.ToInt64())."
    }
    $clientRect = [SyndocalAcceptanceRunnerNative+RECT]::new()
    if (-not [SyndocalAcceptanceRunnerNative]::GetClientRect($Handle, [ref]$clientRect)) {
      throw "Fail closed: GetClientRect failed for HWND $($Handle.ToInt64())."
    }
    $clientOrigin = [SyndocalAcceptanceRunnerNative+POINT]::new()
    if (-not [SyndocalAcceptanceRunnerNative]::ClientToScreen($Handle, [ref]$clientOrigin)) {
      throw "Fail closed: ClientToScreen failed for HWND $($Handle.ToInt64())."
    }
    [pscustomobject]@{
      Outer = $outerRect
      Client = $clientRect
      ClientOrigin = $clientOrigin
    }
  }

  function Get-ObservationMonitorInfo {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    $monitorHandle = [SyndocalAcceptanceRunnerNative]::MonitorFromWindow($Handle, [uint32]2)
    $monitorInfo = [SyndocalAcceptanceRunnerNative+MONITORINFOEXW]::new()
    $monitorInfo.Size =
      [Runtime.InteropServices.Marshal]::SizeOf([type][SyndocalAcceptanceRunnerNative+MONITORINFOEXW])
    if (
      $monitorHandle -eq [IntPtr]::Zero -or
      -not [SyndocalAcceptanceRunnerNative]::GetMonitorInfoW($monitorHandle, [ref]$monitorInfo)
    ) {
      throw "Fail closed: GetMonitorInfoW failed for HWND $($Handle.ToInt64())."
    }
    $monitorInfo
  }

  function Get-EffectiveDpiForWindow {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    [uint32][SyndocalAcceptanceRunnerNative]::GetDpiForWindow($Handle)
  }

  function Get-ObservationWindowStateFlags {
    param([Parameter(Mandatory = $true)][IntPtr]$Handle)

    [pscustomobject]@{
      Visible = [bool][SyndocalAcceptanceRunnerNative]::IsWindowVisible($Handle)
      Responding = Test-WindowNotHung -Handle $Handle
      Minimized = [bool][SyndocalAcceptanceRunnerNative]::IsIconic($Handle)
      Maximized = [bool][SyndocalAcceptanceRunnerNative]::IsZoomed($Handle)
    }
  }
  # END PS 5.1 FALLBACK
}

if (-not ("SyndocalAcceptanceRunnerNative" -as [type])) {
  Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Collections.Generic;

public delegate bool SyndocalAcceptanceMonitorEnumProc(IntPtr hMonitor, IntPtr hdcMonitor, ref SyndocalAcceptanceRunnerNative.RECT rect, IntPtr dwData);

public static class SyndocalAcceptanceRunnerNative {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
  }

  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct MONITORINFOEXW {
    public int Size;
    public RECT Monitor;
    public RECT Work;
    public uint Flags;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)]
    public string DeviceName;
  }

  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct PHYSICAL_MONITOR {
    public IntPtr Handle;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)]
    public string Description;
  }

  [DllImport("user32.dll")]
  public static extern bool EnumDisplayMonitors(IntPtr hdc, IntPtr lprcClip, SyndocalAcceptanceMonitorEnumProc proc, IntPtr dwData);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern bool GetMonitorInfoW(IntPtr hMonitor, ref MONITORINFOEXW info);

  [DllImport("user32.dll")]
  public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

  [DllImport("user32.dll")]
  public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int x, int y, int cx, int cy, uint uFlags);

  [DllImport("shcore.dll")]
  public static extern int GetDpiForMonitor(IntPtr hmonitor, int dpiType, out uint dpiX, out uint dpiY);

  [DllImport("dxva2.dll", SetLastError = true)]
  public static extern bool GetPhysicalMonitorsFromHMONITOR(IntPtr hMonitor, uint count, [In, Out] PHYSICAL_MONITOR[] monitors);

  [DllImport("dxva2.dll", SetLastError = true, CharSet = CharSet.Unicode)]
  public static extern bool GetPhysicalMonitorDescription(IntPtr hPhysicalMonitor, uint characters, StringBuilder description);

  [DllImport("dxva2.dll", SetLastError = true)]
  public static extern bool DestroyPhysicalMonitors(uint count, ref PHYSICAL_MONITOR[] monitors);

  // Read-only probe surface mirroring observe-syndocal-window-geometry.ps1 so
  // this runner stays self-sufficient on hosts that cannot parse that file
  // (Windows PowerShell 5.1 rejects its uint literal suffixes).
  public delegate bool SyndocalAcceptanceWindowEnumProc(IntPtr hWnd, IntPtr lParam);

  [StructLayout(LayoutKind.Sequential)]
  public struct POINT {
    public int X;
    public int Y;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct BY_HANDLE_FILE_INFORMATION {
    public uint FileAttributes;
    public uint CreationTimeLow;
    public uint CreationTimeHigh;
    public uint LastAccessTimeLow;
    public uint LastAccessTimeHigh;
    public uint LastWriteTimeLow;
    public uint LastWriteTimeHigh;
    public uint VolumeSerialNumber;
    public uint FileSizeHigh;
    public uint FileSizeLow;
    public uint NumberOfLinks;
    public uint FileIndexHigh;
    public uint FileIndexLow;
  }

  [DllImport("user32.dll")]
  public static extern bool EnumWindows(SyndocalAcceptanceWindowEnumProc callback, IntPtr lParam);

  [DllImport("user32.dll")]
  public static extern bool IsWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool IsWindowVisible(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool IsIconic(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool IsZoomed(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool IsHungAppWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern int GetWindowTextLength(IntPtr hWnd);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);

  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern bool GetClientRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern bool ClientToScreen(IntPtr hWnd, ref POINT point);

  [DllImport("user32.dll")]
  public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

  [DllImport("user32.dll")]
  public static extern IntPtr MonitorFromWindow(IntPtr hWnd, uint flags);

  [DllImport("user32.dll")]
  public static extern uint GetDpiForWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool IsValidDpiAwarenessContext(IntPtr value);

  [DllImport("user32.dll")]
  public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr dpiContext);

  [DllImport("kernel32.dll", SetLastError = true)]
  public static extern IntPtr OpenProcess(uint desiredAccess, bool inheritHandle, uint processId);

  [DllImport("kernel32.dll", SetLastError = true)]
  public static extern bool CloseHandle(IntPtr handle);

  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern bool QueryFullProcessImageNameW(IntPtr processHandle, uint flags, StringBuilder exeName, ref uint exeNameLengthChars);

  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern IntPtr CreateFileW(string fileName, uint desiredAccess, uint shareMode, IntPtr securityAttributes, int creationDisposition, int flagsAndAttributes, IntPtr templateFileHandle);

  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern uint GetFinalPathNameByHandleW(IntPtr handle, StringBuilder buffer, uint bufferLengthChars, uint flags);

  [DllImport("kernel32.dll", SetLastError = true)]
  public static extern bool GetFileInformationByHandle(IntPtr handle, out BY_HANDLE_FILE_INFORMATION information);

  public static IntPtr[] CollectMonitorHandles() {
    List<IntPtr> handles = new List<IntPtr>();
    SyndocalAcceptanceMonitorEnumProc callback = delegate(IntPtr hMonitor, IntPtr hdcMonitor, ref RECT rect, IntPtr dwData) {
      handles.Add(hMonitor);
      return true;
    };
    EnumDisplayMonitors(IntPtr.Zero, IntPtr.Zero, callback, IntPtr.Zero);
    return handles.ToArray();
  }

  public static string TryGetPhysicalMonitorDescription(IntPtr hMonitor) {
    PHYSICAL_MONITOR[] monitors = new PHYSICAL_MONITOR[1];
    bool acquired = false;
    try {
      if (!GetPhysicalMonitorsFromHMONITOR(hMonitor, 1u, monitors)) {
        return "";
      }
      acquired = true;
      if (monitors[0].Description == null) {
        return "";
      }
      return monitors[0].Description.Trim();
    } catch {
      return "";
    } finally {
      if (acquired) {
        try {
          DestroyPhysicalMonitors(1u, ref monitors);
        } catch {
        }
      }
    }
  }

  public static uint TryGetMonitorEffectiveDpi(IntPtr hMonitor) {
    try {
      uint dpiX;
      uint dpiY;
      if (GetDpiForMonitor(hMonitor, 0, out dpiX, out dpiY) == 0) {
        return dpiX;
      }
    } catch {
    }
    return 0u;
  }
}
'@
}

# ---------------------------------------------------------------------------
# Generic helpers
# ---------------------------------------------------------------------------

function Get-AcceptanceUtcTimestamp {
  [DateTime]::UtcNow.ToString("o")
}

function Invoke-WithPhysicalDpiContext {
  # Sets per-monitor-v2 awareness for THIS thread only around Body, always
  # restoring the previous context in finally (nested uses stack correctly),
  # mirroring the observer contract so rectangle queries are provably
  # physical pixels.
  param([Parameter(Mandatory = $true)][scriptblock]$Body)

  if (-not [SyndocalAcceptanceRunnerNative]::IsValidDpiAwarenessContext([IntPtr](-4))) {
    throw "This Windows version cannot prove per-monitor-v2 coordinates; refusing to act on unprovable units."
  }
  $previousContext = [SyndocalAcceptanceRunnerNative]::SetThreadDpiAwarenessContext([IntPtr](-4))
  if ($previousContext -eq [IntPtr]::Zero) {
    throw "SetThreadDpiAwarenessContext failed; refusing to proceed with unprovable coordinate units."
  }
  try {
    & $Body
  } finally {
    [void][SyndocalAcceptanceRunnerNative]::SetThreadDpiAwarenessContext($previousContext)
  }
}

function ConvertTo-AcceptanceRectDetail {
  param([Parameter(Mandatory = $true)]$Rect)

  [pscustomobject]@{
    left = $Rect.Left
    top = $Rect.Top
    right = $Rect.Right
    bottom = $Rect.Bottom
    width = $Rect.Right - $Rect.Left
    height = $Rect.Bottom - $Rect.Top
  }
}

# ---------------------------------------------------------------------------
# SEAM: external facts (deterministic tests override every one of these)
# ---------------------------------------------------------------------------

function Get-ExecutableSha256 {
  # SEAM: production hashes the exact expected executable file.
  param([Parameter(Mandatory = $true)][string]$Path)

  (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-ExecutableProductVersion {
  # SEAM: production reads VersionInfo.ProductVersion of the exact file.
  param([Parameter(Mandatory = $true)][string]$Path)

  [string](Get-Item -LiteralPath $Path).VersionInfo.ProductVersion
}

function Resolve-GitHeadFromRepository {
  # SEAM: production asks git for this checkout's HEAD commit (full 40-hex).
  # The repository is read only; nothing is staged, committed, or pushed.
  param([Parameter(Mandatory = $true)][string]$CheckoutRootPath)

  $output = & git -C $CheckoutRootPath rev-parse HEAD 2>&1
  if ($LASTEXITCODE -ne 0) {
    throw "Fail closed: git rev-parse HEAD failed in '$CheckoutRootPath'."
  }
  $headLine = ([string]@($output)[0]).Trim()
  if ($headLine -notmatch "^[0-9a-fA-F]{40}$") {
    throw "Fail closed: git rev-parse HEAD returned an unexpected form '$headLine'."
  }
  $headLine.ToLowerInvariant()
}

function Get-MonitorDescriptionWmiFallback {
  # Conservative fallback for the physical monitor description: only usable
  # when exactly ONE WmiMonitorID instance exists (otherwise the mapping to a
  # specific monitor would be guesswork, which this runner never does).
  # Returns "" when unavailable or ambiguous.
  $instances = @(Get-CimInstance -Namespace "root/wmi" -ClassName "WmiMonitorID" -ErrorAction SilentlyContinue)
  if ($instances.Count -ne 1) { return "" }
  $rawBytes = @($instances[0].UserFriendlyName)
  if ($rawBytes.Count -eq 0) { return "" }
  $characters = New-Object System.Collections.Generic.List[char]
  foreach ($byteValue in $rawBytes) {
    $codePoint = [int]$byteValue
    if ($codePoint -eq 0) { break }
    if ($codePoint -ge 32 -and $codePoint -le 126) {
      $characters.Add([char]$codePoint)
    }
  }
  return ((@($characters | ForEach-Object { [string]$_ }) -join "")).Trim()
}

function Get-DisplayMonitorIdentities {
  # SEAM: production enumerates EVERY connected monitor with its exact
  # identity tuple parts. The description comes from the monitor driver via
  # dxva2 with the conservative WMI fallback above; an empty description
  # means identity is unprovable and fails closed wherever it matters.
  # DISPLAY ordinals are recorded but carry no decision weight.
  $identities = [System.Collections.Generic.List[object]]::new()
  Invoke-WithPhysicalDpiContext {
    $handles = [SyndocalAcceptanceRunnerNative]::CollectMonitorHandles()
    foreach ($handle in $handles) {
      $info = [SyndocalAcceptanceRunnerNative+MONITORINFOEXW]::new()
      $info.Size = [Runtime.InteropServices.Marshal]::SizeOf([type][SyndocalAcceptanceRunnerNative+MONITORINFOEXW])
      if (-not [SyndocalAcceptanceRunnerNative]::GetMonitorInfoW($handle, [ref]$info)) {
        throw "Fail closed: GetMonitorInfoW failed for a monitor during display enumeration."
      }
      $description = [SyndocalAcceptanceRunnerNative]::TryGetPhysicalMonitorDescription($handle)
      $descriptionSource = "dxva2_get_physical_monitor_description"
      if ($description -eq "") {
        $description = Get-MonitorDescriptionWmiFallback
        $descriptionSource = "wmi_wmimonitorid_single_instance_fallback"
        if ($description -eq "") {
          $descriptionSource = "unavailable"
        }
      }
      $monitorDpi = [int][SyndocalAcceptanceRunnerNative]::TryGetMonitorEffectiveDpi($handle)
      $identities.Add([pscustomobject]@{
        handle_decimal = [long]$handle.ToInt64()
        device_name = [string]$info.DeviceName
        description = [string]$description
        description_source = [string]$descriptionSource
        effective_dpi = $monitorDpi
        physical_bounds = ConvertTo-AcceptanceRectDetail -Rect $info.Monitor
        work_area = ConvertTo-AcceptanceRectDetail -Rect $info.Work
      })
    }
  }
  return @($identities)
}

function Get-MonitorHandleForWindow {
  # SEAM: production maps an HWND to its nearest monitor handle.
  param([Parameter(Mandatory = $true)][long]$HwndDecimal)

  $resolved = Invoke-WithPhysicalDpiContext {
    [SyndocalAcceptanceRunnerNative]::MonitorFromWindow([IntPtr]$HwndDecimal, [uint32]2)
  }
  if ($resolved -eq [IntPtr]::Zero) {
    throw "Fail closed: MonitorFromWindow returned no monitor for HWND $HwndDecimal."
  }
  return [long]$resolved.ToInt64()
}

function Invoke-ShowWindowState {
  # SEAM: the ONLY ShowWindow path. Production fails closed on a FALSE
  # return; subsequent sampling remains the authoritative state verifier.
  param(
    [Parameter(Mandatory = $true)][IntPtr]$Handle,
    [Parameter(Mandatory = $true)][int]$StateCommand
  )

  $succeeded = [SyndocalAcceptanceRunnerNative]::ShowWindow($Handle, $StateCommand)
  if (-not $succeeded) {
    throw (
      "Fail closed: ShowWindow(state=$StateCommand) returned FALSE for HWND " +
      "$($Handle.ToInt64()); refusing to continue without verified effect.")
  }
  return $true
}

function Invoke-SetWindowPlacementRect {
  # SEAM: the ONLY SetWindowPos path. NOZORDER + NOACTIVATE only: never
  # changes Z order, never steals foreground or focus.
  param(
    [Parameter(Mandatory = $true)][IntPtr]$Handle,
    [Parameter(Mandatory = $true)][int]$X,
    [Parameter(Mandatory = $true)][int]$Y,
    [Parameter(Mandatory = $true)][int]$Width,
    [Parameter(Mandatory = $true)][int]$Height
  )

  $flags = [uint32]($script:SetWindowPosNoZOrder -bor $script:SetWindowPosNoActivate)
  $succeeded = [SyndocalAcceptanceRunnerNative]::SetWindowPos(
    $Handle, [IntPtr]::Zero, $X, $Y, $Width, $Height, $flags)
  if (-not $succeeded) {
    throw (
      "Fail closed: SetWindowPos(x=$X y=$Y w=$Width h=$Height) returned FALSE " +
      "for HWND $($Handle.ToInt64()); refusing to continue without verified effect.")
  }
  return $true
}

function Invoke-SampleDelay {
  # SEAM: separation delay between stability samples. Returns the requested
  # duration so callers can record the provenance of sample spacing.
  param([Parameter(Mandatory = $true)][int]$Milliseconds)

  Start-Sleep -Milliseconds $Milliseconds
  return $Milliseconds
}

# ---------------------------------------------------------------------------
# Invocation authorization (fail closed)
# ---------------------------------------------------------------------------

function Test-AcceptanceSha256Format {
  param([Parameter(Mandatory = $true)][string]$Value)
  return ($Value -match "^[0-9a-fA-F]{64}$")
}

function Assert-AcceptanceInvocationAuthorization {
  # Validates parameter well-formedness in BOTH modes whenever provided, and
  # in apply mode requires every expectation present AND exactly equal to the
  # pinned physical monitor tuple. Returns the normalized expectations object
  # for apply mode, or $null for dry-run mode.
  param(
    [Parameter(Mandatory = $true)][bool]$IsApplyMode,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ExecutablePath,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Sha256,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ProductVersion,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$GitHead,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$MonitorIdentifier,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$MonitorPhysicalResolution,
    [Parameter(Mandatory = $true)][int]$EffectiveDpi,
    [Parameter(Mandatory = $true)][int]$SampleIntervalMsValue,
    [Parameter(Mandatory = $true)][int]$MaxSampleAttemptsValue
  )

  if ($SampleIntervalMsValue -lt $script:PinnedAcceptanceConstants.MinimumSampleIntervalMs) {
    throw (
      "Fail closed: SampleIntervalMs=$SampleIntervalMsValue is below the pinned minimum " +
      "$($script:PinnedAcceptanceConstants.MinimumSampleIntervalMs) ms required between stability samples.")
  }
  if ($MaxSampleAttemptsValue -lt $script:PinnedAcceptanceConstants.RequiredConsecutiveStableSamples) {
    throw (
      "Fail closed: MaxSampleAttempts=$MaxSampleAttemptsValue is below the required consecutive " +
      "stable sample count $($script:PinnedAcceptanceConstants.RequiredConsecutiveStableSamples).")
  }

  if ($Sha256 -ne "" -and -not (Test-AcceptanceSha256Format -Value $Sha256)) {
    throw "Fail closed: ExpectedSha256 '$Sha256' is malformed; expected exactly 64 hexadecimal characters."
  }
  if ($GitHead -ne "" -and $GitHead -notmatch "^[0-9a-fA-F]{40}$") {
    throw "Fail closed: ExpectedGitHead '$GitHead' is malformed; expected exactly 40 hexadecimal characters."
  }
  if ($ExecutablePath -ne "") {
    if (-not [IO.Path]::IsPathRooted($ExecutablePath)) {
      throw "Fail closed: ExpectedExecutablePath '$ExecutablePath' must be an absolute path."
    }
    if ((Split-Path -Leaf $ExecutablePath) -ine "syndocal.exe") {
      throw "Fail closed: ExpectedExecutablePath leaf must be exactly 'syndocal.exe', got '$(Split-Path -Leaf $ExecutablePath)'."
    }
  }

  $normalized = [pscustomobject]@{
    ExecutablePath = $(if ($ExecutablePath -ne "") { [IO.Path]::GetFullPath($ExecutablePath) } else { "" })
    RequestedExecutablePath = $ExecutablePath
    Sha256 = $(if ($Sha256 -ne "") { $Sha256.ToLowerInvariant() } else { "" })
    ProductVersion = $ProductVersion
    GitHead = $(if ($GitHead -ne "") { $GitHead.ToLowerInvariant() } else { "" })
    MonitorIdentifier = $MonitorIdentifier
    MonitorPhysicalResolution = $MonitorPhysicalResolution
    EffectiveDpi = $EffectiveDpi
    TargetMonitorHandleDecimal = [long]0
    TargetMonitorWorkArea = $null
    InformationalOnly = (-not $IsApplyMode)
  }
  if (-not $IsApplyMode) {
    return $normalized
  }

  $missingFields = [System.Collections.Generic.List[string]]::new()
  if ($ExecutablePath -eq "") { $missingFields.Add("ExpectedExecutablePath") }
  if ($Sha256 -eq "") { $missingFields.Add("ExpectedSha256") }
  if ($ProductVersion -eq "") { $missingFields.Add("ExpectedProductVersion") }
  if ($GitHead -eq "") { $missingFields.Add("ExpectedGitHead") }
  if ($MonitorIdentifier -eq "") { $missingFields.Add("ExpectedMonitorIdentifier") }
  if ($MonitorPhysicalResolution -eq "") { $missingFields.Add("ExpectedMonitorPhysicalResolution") }
  if ($EffectiveDpi -le 0) { $missingFields.Add("ExpectedEffectiveDpi") }
  if ($missingFields.Count -gt 0) {
    throw (
      "Fail closed: -Apply requires explicit expectations but missing or empty: " +
      "$(($missingFields -join ", ")). Re-run with every expected value supplied.")
  }

  $pin = $script:PinnedAcceptanceConstants
  if ($MonitorIdentifier -cne $pin.MonitorIdentifier) {
    throw (
      "Fail closed: ExpectedMonitorIdentifier '$MonitorIdentifier' does not equal the pinned target panel " +
      "'$($pin.MonitorIdentifier)' for this acceptance tranche.")
  }
  if ($MonitorPhysicalResolution -cne $pin.MonitorPhysicalResolution) {
    throw (
      "Fail closed: ExpectedMonitorPhysicalResolution '$MonitorPhysicalResolution' does not equal the pinned " +
      "'$($pin.MonitorPhysicalResolution)'.")
  }
  if ($EffectiveDpi -ne $pin.EffectiveDpi) {
    throw "Fail closed: ExpectedEffectiveDpi=$EffectiveDpi does not equal the pinned $($pin.EffectiveDpi)."
  }

  $normalized.ExecutablePath = [IO.Path]::GetFullPath($ExecutablePath)
  $normalized.Sha256 = $Sha256.ToLowerInvariant()
  $normalized.GitHead = $GitHead.ToLowerInvariant()
  return $normalized
}

function New-PinnedInformationalExpectations {
  # Dry-run expectations: the pinned acceptance constants used ONLY for an
  # informational criteria evaluation; no deep identity comparison happens.
  param([Parameter(Mandatory = $true)][string]$ExecutablePathForReference)

  $pin = $script:PinnedAcceptanceConstants
  return [pscustomobject]@{
    ExecutablePath = $ExecutablePathForReference
    RequestedExecutablePath = $ExecutablePathForReference
    Sha256 = ""
    ProductVersion = ""
    GitHead = ""
    MonitorIdentifier = $pin.MonitorIdentifier
    MonitorPhysicalResolution = $pin.MonitorPhysicalResolution
    EffectiveDpi = $pin.EffectiveDpi
    TargetMonitorHandleDecimal = [long]0
    TargetMonitorWorkArea = $null
    InformationalOnly = $true
  }
}

function Confirm-TargetProcessIdentity {
  # Deep identity re-proof against the operator's expected values: fresh
  # candidate enumeration, native image path + final-path + volume/file
  # identity match (observer machinery), then SHA256 and product version of
  # the exact expected file. Any mismatch or unreadable query fails closed.
  param([Parameter(Mandatory = $true)]$Expectations)

  $fingerprint = Get-CheckoutTargetExecutableFingerprint -TargetExecutablePath $Expectations.ExecutablePath
  $candidateIds = @(Get-SyndocalCandidateProcessIds)
  $provenPidValue = Select-SyndocalTargetProcessId `
    -CandidateProcessIds $candidateIds `
    -TargetFingerprint $fingerprint

  $observedSha256 = Get-ExecutableSha256 -Path $Expectations.ExecutablePath
  if ($observedSha256 -cne $Expectations.Sha256) {
    throw (
      "authorization identity mismatch: observed sha256 '$observedSha256' does not equal the " +
      "expected sha256 '$($Expectations.Sha256)' for '$($Expectations.ExecutablePath)'.")
  }
  $observedVersion = Get-ExecutableProductVersion -Path $Expectations.ExecutablePath
  if ($observedVersion -cne $Expectations.ProductVersion) {
    throw (
      "authorization identity mismatch: observed product version '$observedVersion' does not equal " +
      "the expected product version '$($Expectations.ProductVersion)'.")
  }

  return [pscustomobject]@{
    ProvenPid = [uint32]$provenPidValue
    ObservedSha256 = $observedSha256
    ObservedProductVersion = $observedVersion
    Fingerprint = $fingerprint
  }
}

function Assert-ObservedGitHeadMatchesExpectation {
  param(
    [Parameter(Mandatory = $true)][string]$ObservedHead,
    [Parameter(Mandatory = $true)][string]$ExpectedHead
  )

  if ($ObservedHead -cne $ExpectedHead) {
    throw (
      "authorization identity mismatch: observed git HEAD '$ObservedHead' does not equal the " +
      "expected git HEAD '$ExpectedHead'.")
  }
  return $true
}

# ---------------------------------------------------------------------------
# Evidence directory safety (new, empty, non-reparse, safe slug, owned)
# ---------------------------------------------------------------------------

function Test-AcceptanceEvidenceSlugSafe {
  param([Parameter(Mandatory = $true)][string]$Slug)

  if ($Slug -cnotmatch "^[a-z0-9][a-z0-9._-]{0,63}$") { return $false }
  if ($Slug.EndsWith(".")) { return $false }
  $reservedNames = @(
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
  )
  if ($reservedNames -contains $Slug.ToUpperInvariant()) { return $false }
  return $true
}

function Test-AcceptanceEvidenceOwnerSidAccepted {
  # Pure ownership decision: only the current user, BUILTIN\Administrators,
  # or NT AUTHORITY\SYSTEM may own the evidence parent directory.
  param([Parameter(Mandatory = $true)][string]$OwnerSidValue)

  $acceptedSids = @("S-1-5-32-544", "S-1-5-18")
  $acceptedSids += [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
  return ($acceptedSids -contains $OwnerSidValue)
}

function Assert-AcceptanceEvidenceDirectoryOwnerAccepted {
  param([Parameter(Mandatory = $true)][string]$PathToCheck)

  $acl = Get-Acl -LiteralPath $PathToCheck
  $ownerSidValue = $null
  try {
    $ownerSidValue =
      (New-Object Security.Principal.NTAccount($acl.Owner)).Translate(
        [Security.Principal.SecurityIdentifier]).Value
  } catch {
    throw "Fail closed: evidence parent owner '$($acl.Owner)' cannot be translated to a SID; refusing unattributable location."
  }
  if (-not (Test-AcceptanceEvidenceOwnerSidAccepted -OwnerSidValue $ownerSidValue)) {
    throw (
      "Fail closed: evidence parent owner SID '$ownerSidValue' ('$($acl.Owner)') is not the current user, " +
      "Administrators, or SYSTEM; refusing a foreign-controlled evidence location.")
  }
  return $ownerSidValue
}

function Assert-NoReparsePointInEvidenceAncestry {
  # Walks from the evidence root upward to the drive root and rejects ANY
  # reparse point (junction/symlink mount) along the way.
  param([Parameter(Mandatory = $true)][string]$RootFullPath)

  $currentPath = $RootFullPath
  while ($true) {
    if ([IO.Directory]::Exists($currentPath)) {
      $item = Get-Item -LiteralPath $currentPath -Force
      if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Fail closed: reparse point detected in evidence path ancestry at '$currentPath'; refusing redirected locations."
      }
    }
    $parentPath = Split-Path -Parent $currentPath
    if ([string]::IsNullOrEmpty($parentPath) -or ($parentPath -ieq $currentPath)) { break }
    $currentPath = $parentPath
  }
}

function New-AcceptanceEvidenceDirectory {
  # Creates and proves a NEW EMPTY non-reparse DIRECT child of the validated
  # root. An existing candidate is never reused. Empty Slug auto-generates a
  # deterministic timestamped name with collision suffixes.
  param(
    [Parameter(Mandatory = $true)][string]$RootPath,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Slug
  )

  if ([string]::IsNullOrWhiteSpace($RootPath)) {
    throw "Fail closed: evidence root path is empty."
  }
  $rootFullPath = [IO.Path]::GetFullPath($RootPath)
  if (-not [IO.Directory]::Exists($rootFullPath)) {
    [void](New-Item -ItemType Directory -Path $rootFullPath -Force)
  }
  $rootItem = Get-Item -LiteralPath $rootFullPath -Force
  if (-not $rootItem.PSIsContainer) {
    throw "Fail closed: evidence root '$rootFullPath' is not a directory."
  }
  Assert-NoReparsePointInEvidenceAncestry -RootFullPath $rootFullPath
  [void](Assert-AcceptanceEvidenceDirectoryOwnerAccepted -PathToCheck $rootFullPath)

  if ($Slug -eq "") {
    $baseName = "native4k-" + (Get-Date -Format "yyyy-MM-dd-HHmmss")
    $generated = $false
    foreach ($suffixIndex in 0..15) {
      $candidateSlug = $baseName
      if ($suffixIndex -gt 0) { $candidateSlug = "{0}-{1:x2}" -f $baseName, $suffixIndex }
      if (-not ([IO.Directory]::Exists((Join-Path $rootFullPath $candidateSlug)))) {
        $Slug = $candidateSlug
        $generated = $true
        break
      }
    }
    if (-not $generated) {
      throw "Fail closed: could not generate a free evidence slug under '$rootFullPath'."
    }
  }

  if (-not (Test-AcceptanceEvidenceSlugSafe -Slug $Slug)) {
    throw (
      "Fail closed: unsafe evidence slug '$Slug'; expected 1..64 chars matching " +
      "[a-z0-9] then [a-z0-9._-], no reserved device names, no trailing dot.")
  }

  $candidatePath = Join-Path $rootFullPath $Slug
  if ((Split-Path -Leaf $candidatePath) -cne $Slug) {
    throw "Fail closed: traversal defense triggered; candidate leaf '$(Split-Path -Leaf $candidatePath)' differs from slug '$Slug'."
  }
  if ([IO.Directory]::Exists($candidatePath) -or [IO.File]::Exists($candidatePath)) {
    throw "Fail closed: refusing to reuse or overwrite existing evidence path '$candidatePath'."
  }
  $requiredPrefix = $rootFullPath.TrimEnd('\') + [IO.Path]::DirectorySeparatorChar
  if (-not $candidatePath.StartsWith($requiredPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Fail closed: candidate '$candidatePath' escaped the evidence root '$rootFullPath'."
  }

  [void](New-Item -ItemType Directory -Path $candidatePath)
  $created = Get-Item -LiteralPath $candidatePath -Force
  if (-not $created.PSIsContainer) {
    throw "Fail closed: candidate '$candidatePath' is not a directory after creation."
  }
  if (($created.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Fail closed: candidate '$candidatePath' became a reparse point; refusing."
  }
  if (@(Get-ChildItem -LiteralPath $candidatePath -Force).Count -ne 0) {
    throw "Fail closed: newly created evidence directory '$candidatePath' is not empty."
  }
  return $created.FullName
}

function Add-AcceptanceEvidenceArtifact {
  # Writes one UTF-8 (no BOM) JSON artifact deterministically and records it
  # in the manifest for SHA256SUMS.txt.
  param(
    [Parameter(Mandatory = $true)][string]$EvidenceDirectoryPath,
    [Parameter(Mandatory = $true)][string]$FileName,
    [Parameter(Mandatory = $true)][string]$JsonText,
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][System.Collections.Generic.List[string]]$ManifestList
  )

  $fullPath = Join-Path $EvidenceDirectoryPath $FileName
  [IO.File]::WriteAllText($fullPath, $JsonText, (New-Object Text.UTF8Encoding($false)))
  [void]$ManifestList.Add($FileName)
  return $fullPath
}

function Complete-AcceptanceEvidenceManifest {
  # Computes real hashes over everything written and emits SHA256SUMS.txt as
  # the LAST artifact. Safe to call again after a late failure.json.
  param(
    [Parameter(Mandatory = $true)][string]$EvidenceDirectoryPath,
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][System.Collections.Generic.List[string]]$ManifestList
  )

  $lines = [System.Collections.Generic.List[string]]::new()
  foreach ($fileName in (@($ManifestList) | Sort-Object -Unique)) {
    $hash = (Get-FileHash -LiteralPath (Join-Path $EvidenceDirectoryPath $fileName) -Algorithm SHA256).Hash.ToLowerInvariant()
    $lines.Add("$hash  $fileName")
  }
  $sumsText = (($lines -join "`r`n") + "`r`n")
  [IO.File]::WriteAllText(
    (Join-Path $EvidenceDirectoryPath "SHA256SUMS.txt"),
    $sumsText,
    (New-Object Text.UTF8Encoding($false)))
  if (-not ($ManifestList -contains "SHA256SUMS.txt")) {
    [void]$ManifestList.Add("SHA256SUMS.txt")
  }
}

function Write-AcceptanceFailureRecord {
  # Failure evidence is ALWAYS retained. Best-effort: never masks the
  # original error.
  param(
    [Parameter(Mandatory = $true)][string]$Mode,
    [Parameter(Mandatory = $true)][string]$Stage,
    [Parameter(Mandatory = $true)][string]$Message,
    [AllowNull()]$EvidenceDirectoryPath,
    [AllowNull()][System.Collections.Generic.List[string]]$ManifestList
  )

  if ([string]::IsNullOrEmpty($EvidenceDirectoryPath)) { return }
  try {
    $payload = [pscustomobject][ordered]@{
      schema_version = 1
      tool = "run-syndocal-native-4k-acceptance"
      mode = $Mode
      stage = $Stage
      utc = (Get-AcceptanceUtcTimestamp)
      message = $Message
      succeeded = $false
      note = "evidence retained; failure.json is authoritative for this run"
    }
    $json = $payload | ConvertTo-Json -Depth 6
    $null = Add-AcceptanceEvidenceArtifact `
      -EvidenceDirectoryPath $EvidenceDirectoryPath `
      -FileName "failure.json" `
      -JsonText $json `
      -ManifestList $ManifestList
    Complete-AcceptanceEvidenceManifest `
      -EvidenceDirectoryPath $EvidenceDirectoryPath `
      -ManifestList $ManifestList
  } catch {
    # Swallow only the evidence-write failure itself.
  }
}

# ---------------------------------------------------------------------------
# Target monitor resolution (tuple only; DISPLAY ordinals never decide)
# ---------------------------------------------------------------------------

function Resolve-AcceptanceTargetMonitor {
  # Matches monitors by the EXACT tuple (description, physical resolution,
  # effective DPI). Zero matches always fail. In apply mode more than one
  # match is ambiguous and fatal: this tranche accepts exactly one physical
  # 4K panel identity.
  param(
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][object[]]$MonitorIdentities,
    [Parameter(Mandatory = $true)][string]$Identifier,
    [Parameter(Mandatory = $true)][string]$PhysicalResolution,
    [Parameter(Mandatory = $true)][int]$EffectiveDpi,
    [Parameter(Mandatory = $true)][bool]$RequireUnique
  )

  $tupleMatches = @(
    $MonitorIdentities | Where-Object {
      ([string]$_.description -ceq $Identifier) -and
      (([string]"$($_.physical_bounds.width)x$($_.physical_bounds.height)") -ceq $PhysicalResolution) -and
      ([int]$_.effective_dpi -eq $EffectiveDpi)
    }
  )
  if ($tupleMatches.Count -eq 0) {
    if (-not $RequireUnique) {
      # Dry-run records the miss informationally instead of aborting.
      return [pscustomobject]@{ MatchCount = 0; UniqueMatch = $null }
    }
    throw (
      "Fail closed: 0 monitors match the exact tuple '$Identifier' '$PhysicalResolution' dpi=$EffectiveDpi; " +
      "refusing to guess among the enumerated displays.")
  }
  if ($RequireUnique -and $tupleMatches.Count -gt 1) {
    throw (
      "Fail closed: $($tupleMatches.Count) monitors match the exact tuple '$Identifier' '$PhysicalResolution' " +
      "dpi=$EffectiveDpi; the target physical panel is ambiguous.")
  }
  return [pscustomobject]@{
    MatchCount = $tupleMatches.Count
    UniqueMatch = $(if ($tupleMatches.Count -eq 1) { $tupleMatches[0] } else { $null })
  }
}

function Find-IdentityForMonitorHandle {
  param(
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][object[]]$MonitorIdentities,
    [Parameter(Mandatory = $true)][long]$MonitorHandleDecimal
  )

  foreach ($identity in $MonitorIdentities) {
    if ([long]$identity.handle_decimal -eq $MonitorHandleDecimal) { return $identity }
  }
  return $null
}

# ---------------------------------------------------------------------------
# Main-window selection (exactly one exact title of the proven PID)
# ---------------------------------------------------------------------------

function Select-MainAcceptanceWindow {
  param(
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][object[]]$VisibleTopLevelEntries,
    [Parameter(Mandatory = $true)][uint32]$ProvenPid
  )

  $titleMatches = @(
    $VisibleTopLevelEntries | Where-Object {
      [string]::Equals($_.Title, $script:AcceptanceMainWindowTitle, [StringComparison]::Ordinal)
    }
  )
  if ($titleMatches.Count -ne 1) {
    $titles = if ($VisibleTopLevelEntries.Count -gt 0) {
      (@($VisibleTopLevelEntries) | ForEach-Object { "'$($_.Title)' (hwnd=$($_.Handle.ToInt64()))" }) -join ", "
    } else {
      "<none>"
    }
    throw (
      "Fail closed: expected exactly one '$($script:AcceptanceMainWindowTitle)' main window for PID $ProvenPid, " +
      "found $($titleMatches.Count). Visible top-level titles: $titles")
  }

  $mainHwnd = [IntPtr]$titleMatches[0].Handle
  if (-not (Test-NativeWindowAlive -Handle $mainHwnd)) {
    throw "Fail closed: main window HWND $($mainHwnd.ToInt64()) vanished during selection."
  }
  $ownerPidValue = Get-WindowOwnerProcessId -Handle $mainHwnd
  if ($ownerPidValue -ne $ProvenPid) {
    throw (
      "Fail closed: main window HWND $($mainHwnd.ToInt64()) is owned by PID $ownerPidValue, not the proven " +
      "target PID $ProvenPid.")
  }
  if (-not (Test-WindowNotHung -Handle $mainHwnd)) {
    throw "Fail closed: main window HWND $($mainHwnd.ToInt64()) is not responding at selection."
  }
  return [pscustomobject]@{
    Handle = $mainHwnd
    HwndDecimal = [long]$mainHwnd.ToInt64()
  }
}

# ---------------------------------------------------------------------------
# State sampling + acceptance criteria
# ---------------------------------------------------------------------------

function New-AcceptanceStateSample {
  # One read-only sample of the main window against the acceptance criteria.
  # DeepIdentityProof additionally re-proves process identity (apply mode).
  param(
    [Parameter(Mandatory = $true)][long]$HwndDecimal,
    [Parameter(Mandatory = $true)][uint32]$ProvenPid,
    [Parameter(Mandatory = $true)]$Expectations,
    [Parameter(Mandatory = $true)][bool]$DeepIdentityProof
  )

  Invoke-WithPhysicalDpiContext {
    $hwndPointer = [IntPtr]$HwndDecimal

    if (-not (Test-NativeWindowAlive -Handle $hwndPointer)) {
      throw "Fail closed: main window HWND $HwndDecimal vanished before sampling."
    }
    $ownerPidValue = Get-WindowOwnerProcessId -Handle $hwndPointer
    $ownerMatches = ([uint32]$ownerPidValue -eq [uint32]$ProvenPid)

    $rectangles = Get-ObservationWindowRectangles -Handle $hwndPointer
    $monitorInfo = Get-ObservationMonitorInfo -Handle $hwndPointer
    $windowDpi = Get-EffectiveDpiForWindow -Handle $hwndPointer
    if ($windowDpi -eq 0) {
      throw "Fail closed: GetDpiForWindow returned 0 for HWND $HwndDecimal; scale factor unprovable."
    }
    $stateFlags = Get-ObservationWindowStateFlags -Handle $hwndPointer

    $nearestMonitorDecimal = Get-MonitorHandleForWindow -HwndDecimal $HwndDecimal
    $monitorIdentities = @(Get-DisplayMonitorIdentities)
    $matchedIdentity = Find-IdentityForMonitorHandle `
      -MonitorIdentities $monitorIdentities `
      -MonitorHandleDecimal $nearestMonitorDecimal

    $clientWidth = $rectangles.Client.Right - $rectangles.Client.Left
    $clientHeight = $rectangles.Client.Bottom - $rectangles.Client.Top
    $scaleFactor = $windowDpi / 96.0
    $logicalWidth = [int][math]::Round($clientWidth / $scaleFactor)
    $logicalHeight = [int][math]::Round($clientHeight / $scaleFactor)
    $pin = $script:PinnedAcceptanceConstants

    $monitorTupleMatch = $false
    if ($null -ne $matchedIdentity) {
      $observedResolution = "$($matchedIdentity.physical_bounds.width)x$($matchedIdentity.physical_bounds.height)"
      $monitorTupleMatch =
        ([string]$matchedIdentity.description -ceq $Expectations.MonitorIdentifier) -and
        ($observedResolution -ceq $Expectations.MonitorPhysicalResolution) -and
        ([int]$matchedIdentity.effective_dpi -eq [int]$Expectations.EffectiveDpi)
    }

    $shaMatch = $null
    $versionMatch = $null
    $freshPidMatches = $true
    if ($DeepIdentityProof) {
      $proof = Confirm-TargetProcessIdentity -Expectations $Expectations
      $freshPidMatches = ([uint32]$proof.ProvenPid -eq [uint32]$ProvenPid)
      $shaMatch = $true
      $versionMatch = $true
    }

    $aliveAndOwned = ((Test-NativeWindowAlive -Handle $hwndPointer) -and $ownerMatches)
    $criteria = [pscustomobject][ordered]@{
      monitor_tuple_match = [bool]$monitorTupleMatch
      window_dpi_match = ([int]$windowDpi -eq [int]$Expectations.EffectiveDpi)
      maximized = [bool]$stateFlags.Maximized
      not_minimized = (-not [bool]$stateFlags.Minimized)
      responsive = [bool]$stateFlags.Responding
      alive_and_owned = [bool]$aliveAndOwned
      client_physical_match =
        (($clientWidth -eq [int]$pin.ClientPhysicalWidth) -and ($clientHeight -eq [int]$pin.ClientPhysicalHeight))
      client_logical_match =
        (($logicalWidth -eq [int]$pin.ClientLogicalWidth) -and ($logicalHeight -eq [int]$pin.ClientLogicalHeight))
      process_identity_ok = $(if ($DeepIdentityProof) { $freshPidMatches } else { $aliveAndOwned })
    }

    $allMet =
      $criteria.monitor_tuple_match -and $criteria.window_dpi_match -and
      $criteria.maximized -and $criteria.not_minimized -and $criteria.responsive -and
      $criteria.alive_and_owned -and $criteria.client_physical_match -and
      $criteria.client_logical_match -and $criteria.process_identity_ok

    return [pscustomobject][ordered]@{
      utc = (Get-AcceptanceUtcTimestamp)
      hwnd_decimal = [long]$HwndDecimal
      owner_pid = [uint32]$ownerPidValue
      visible = [bool]$stateFlags.Visible
      responding = [bool]$stateFlags.Responding
      minimized = [bool]$stateFlags.Minimized
      maximized = [bool]$stateFlags.Maximized
      nearest_monitor_handle_decimal = [long]$nearestMonitorDecimal
      monitor_device_name = [string]$monitorInfo.DeviceName
      matched_monitor_description = $(if ($null -ne $matchedIdentity) { [string]$matchedIdentity.description } else { "" })
      outer_physical = ConvertTo-AcceptanceRectDetail -Rect $rectangles.Outer
      client_physical = [pscustomobject][ordered]@{
        left = $rectangles.ClientOrigin.X
        top = $rectangles.ClientOrigin.Y
        width = $clientWidth
        height = $clientHeight
      }
      client_logical = [pscustomobject][ordered]@{
        width = $logicalWidth
        height = $logicalHeight
        derivation = "client_physical / (window_effective_dpi / 96)"
      }
      window_effective_dpi = [int]$windowDpi
      deep_identity_checked = $DeepIdentityProof
      sha256_match = $shaMatch
      product_version_match = $versionMatch
      fresh_pid_matches_proven_pid = [bool]$freshPidMatches
      criteria = $criteria
      all_criteria_met = [bool]$allMet
    }
  }
}

# ---------------------------------------------------------------------------
# Pre-change revalidation + authorized operations (apply mode)
# ---------------------------------------------------------------------------

function Confirm-PreChangeRevalidation {
  # Full fail-closed gate immediately BEFORE any state change: HWND alive,
  # owner PID unchanged, still responsive, deep process identity re-proved
  # (path/hash/version), and window still on the target physical monitor.
  param(
    [Parameter(Mandatory = $true)][long]$HwndDecimal,
    [Parameter(Mandatory = $true)][uint32]$ProvenPid,
    [Parameter(Mandatory = $true)]$Expectations,
    [Parameter(Mandatory = $true)][string]$OperationKind
  )

  $hwndPointer = [IntPtr]$HwndDecimal
  if (-not (Test-NativeWindowAlive -Handle $hwndPointer)) {
    throw "Fail closed: HWND $HwndDecimal vanished before $OperationKind; aborting remaining operations."
  }
  $ownerPidValue = Get-WindowOwnerProcessId -Handle $hwndPointer
  if ([uint32]$ownerPidValue -ne [uint32]$ProvenPid) {
    throw (
      "Fail closed: ownership changed before $OperationKind; HWND $HwndDecimal is owned by PID " +
      "$ownerPidValue, not proven PID $ProvenPid.")
  }
  if (-not (Test-WindowNotHung -Handle $hwndPointer)) {
    throw "Fail closed: main window HWND $HwndDecimal is not responding before $OperationKind; refusing to mutate a hung window."
  }

  $identityProof = Confirm-TargetProcessIdentity -Expectations $Expectations
  if ([uint32]$identityProof.ProvenPid -ne [uint32]$ProvenPid) {
    throw (
      "Fail closed: proven PID changed to $($identityProof.ProvenPid) before $OperationKind " +
      "(was $ProvenPid); aborting.")
  }

  $nearestMonitorDecimal = Get-MonitorHandleForWindow -HwndDecimal $HwndDecimal
  if ([long]$nearestMonitorDecimal -ne [long]$Expectations.TargetMonitorHandleDecimal) {
    throw (
      "Fail closed: HWND $HwndDecimal is no longer on the target monitor before $OperationKind " +
      "(nearest=$nearestMonitorDecimal expected=$($Expectations.TargetMonitorHandleDecimal)).")
  }

  return [pscustomobject]@{
    passed = $true
    operation_kind = $OperationKind
    checks = [pscustomobject][ordered]@{
      hwnd_alive = $true
      owner_pid_matches = $true
      responsive = $true
      process_identity_reproved = $true
      on_target_monitor = $true
    }
    utc = (Get-AcceptanceUtcTimestamp)
  }
}

function Decide-AuthorizedOperations {
  param(
    [Parameter(Mandatory = $true)]$InitialSample,
    [Parameter(Mandatory = $true)][long]$TargetMonitorHandleDecimal
  )

  $planned = [System.Collections.Generic.List[string]]::new()
  $restored = $false
  if ([bool]$InitialSample.minimized) {
    $planned.Add("sw_restore")
    $restored = $true
  }
  $onWrongMonitor = ([long]$InitialSample.nearest_monitor_handle_decimal -ne $TargetMonitorHandleDecimal)
  if ($onWrongMonitor) {
    if ([bool]$InitialSample.maximized) {
      # A maximized window must leave the maximized state before it can be
      # moved to another monitor deterministically.
      $planned.Add("sw_restore_unmaximize_for_move")
      $restored = $true
    }
    $planned.Add("reposition_to_target_monitor")
  }
  $needsMaximize = ((-not [bool]$InitialSample.maximized) -or $restored)
  if ($needsMaximize) {
    $planned.Add("sw_maximize")
  }
  return @($planned)
}

function Invoke-AuthorizedWindowOperation {
  # Executes ONE authorized state change after a full pre-change revalidation
  # and records both in the operation log. Any revalidation failure throws,
  # which aborts all remaining operations (fail closed, evidence retained).
  param(
    [Parameter(Mandatory = $true)][string]$OperationKind,
    [Parameter(Mandatory = $true)][long]$HwndDecimal,
    [Parameter(Mandatory = $true)][uint32]$ProvenPid,
    [Parameter(Mandatory = $true)]$Expectations,
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][System.Collections.Generic.List[object]]$OperationsLog
  )

  $revalidation = Confirm-PreChangeRevalidation `
    -HwndDecimal $HwndDecimal `
    -ProvenPid $ProvenPid `
    -Expectations $Expectations `
    -OperationKind $OperationKind

  $mutationParams = [pscustomobject]@{}
  switch ($OperationKind) {
    "sw_restore" {
      [void](Invoke-ShowWindowState -Handle ([IntPtr]$HwndDecimal) -StateCommand $script:SwRestoreCommand)
      $mutationParams = [pscustomobject]@{ api = "ShowWindow"; command = "SW_RESTORE" }
    }
    "sw_restore_unmaximize_for_move" {
      [void](Invoke-ShowWindowState -Handle ([IntPtr]$HwndDecimal) -StateCommand $script:SwRestoreCommand)
      $mutationParams = [pscustomobject]@{ api = "ShowWindow"; command = "SW_RESTORE"; reason = "unmaximize_before_cross_monitor_move" }
    }
    "reposition_to_target_monitor" {
      $work = $Expectations.TargetMonitorWorkArea
      $width = [int][math]::Floor(($work.width * 2) / 3)
      $height = [int][math]::Floor(($work.height * 2) / 3)
      $x = [int]$work.left + [int][math]::Floor(($work.width - $width) / 2)
      $y = [int]$work.top + [int][math]::Floor(($work.height - $height) / 2)
      [void](Invoke-SetWindowPlacementRect -Handle ([IntPtr]$HwndDecimal) -X $x -Y $y -Width $width -Height $height)
      $mutationParams = [pscustomobject][ordered]@{ api = "SetWindowPos"; x = $x; y = $y; width = $width; height = $height; flags = "NOZORDER|NOACTIVATE" }
    }
    "sw_maximize" {
      [void](Invoke-ShowWindowState -Handle ([IntPtr]$HwndDecimal) -StateCommand $script:SwMaximizeCommand)
      $mutationParams = [pscustomobject]@{ api = "ShowWindow"; command = "SW_MAXIMIZE" }
    }
    default {
      throw "Fail closed: unknown operation kind '$OperationKind'."
    }
  }

  $entry = [pscustomobject][ordered]@{
    seq = $OperationsLog.Count + 1
    kind = $OperationKind
    params = $mutationParams
    pre_change_revalidation = $revalidation
    executed_utc = (Get-AcceptanceUtcTimestamp)
    note = "effect verified by subsequent stability sampling"
  }
  [void]$OperationsLog.Add($entry)
  return $entry
}

# ---------------------------------------------------------------------------
# Stability verification
# ---------------------------------------------------------------------------

function Invoke-StabilityVerification {
  param(
    [Parameter(Mandatory = $true)][long]$HwndDecimal,
    [Parameter(Mandatory = $true)][uint32]$ProvenPid,
    [Parameter(Mandatory = $true)]$Expectations,
    [Parameter(Mandatory = $true)][bool]$DeepIdentityProof,
    [Parameter(Mandatory = $true)][int]$SampleIntervalMsValue,
    [Parameter(Mandatory = $true)][int]$MaxSampleAttemptsValue,
    [Parameter(Mandatory = $true)][int]$RequiredConsecutiveStableSamples
  )

  $samples = [System.Collections.Generic.List[object]]::new()
  $delaysRequestedMs = [System.Collections.Generic.List[int]]::new()
  $consecutiveStable = 0
  $attemptsUsed = 0
  $achieved = $false

  while ($true) {
    if ($samples.Count -gt 0) {
      $actualDelayMs = Invoke-SampleDelay -Milliseconds $SampleIntervalMsValue
      [void]$delaysRequestedMs.Add([int]$actualDelayMs)
    }
    $attemptsUsed++
    $sample = New-AcceptanceStateSample `
      -HwndDecimal $HwndDecimal `
      -ProvenPid $ProvenPid `
      -Expectations $Expectations `
      -DeepIdentityProof $DeepIdentityProof
    [void]$samples.Add($sample)

    if ([bool]$sample.all_criteria_met) {
      $consecutiveStable++
    } else {
      $consecutiveStable = 0
    }
    if ($consecutiveStable -ge $RequiredConsecutiveStableSamples) {
      $achieved = $true
      break
    }
    if ($attemptsUsed -ge $MaxSampleAttemptsValue) { break }
  }

  return [pscustomobject][ordered]@{
    required_consecutive_stable_samples = $RequiredConsecutiveStableSamples
    minimum_sample_interval_ms = $script:PinnedAcceptanceConstants.MinimumSampleIntervalMs
    configured_sample_interval_ms = $SampleIntervalMsValue
    attempts_used = $attemptsUsed
    consecutive_stable_achieved = $consecutiveStable
    achieved = [bool]$achieved
    delays_requested_ms = @($delaysRequestedMs)
    samples = @($samples)
  }
}

# ---------------------------------------------------------------------------
# Orchestration
# ---------------------------------------------------------------------------

function Invoke-AcceptanceRun {
  # Core entry point. NEVER throws: every failure is captured into the
  # returned result (tests assert on fields; the thin main wrapper maps the
  # result to stdout/stderr lines and the process exit code).
  [CmdletBinding()]
  param(
    [switch]$Apply,

    [string]$ExpectedExecutablePath = "",
    [string]$ExpectedSha256 = "",
    [string]$ExpectedProductVersion = "",
    [string]$ExpectedGitHead = "",
    [string]$ExpectedMonitorIdentifier = "",
    [string]$ExpectedMonitorPhysicalResolution = "",
    [int]$ExpectedEffectiveDpi = 0,

    [string]$EvidenceSlug = "",
    [string]$EvidenceRootPath = "",

    [int]$SampleIntervalMs = 250,
    [int]$MaxSampleAttempts = 24
  )

  $mode = "dry-run"
  if ($Apply) { $mode = "apply" }
  $toolName = "run-syndocal-native-4k-acceptance"
  $pin = $script:PinnedAcceptanceConstants

  $rt = [pscustomobject]@{
    Stage = "start"
    Errors = [System.Collections.Generic.List[string]]::new()
    ManifestList = [System.Collections.Generic.List[string]]::new()
    EvidenceDirectoryPath = $null
    MainHwndDecimal = [long]0
    ProvenPid = [uint32]0
    OperationsLog = [System.Collections.Generic.List[object]]::new()
    Stability = $null
    StartedUtc = (Get-AcceptanceUtcTimestamp)
  }

  $plannedOps = @()
  $expectations = $null
  $criteriaBasis = $null
  $fingerprint = $null
  $exeItem = $null
  $effectiveExe = ""
  $observedSha256 = ""
  $observedVersion = ""
  $observedHead = ""
  $monitorIdentities = @()
  $targetSelection = $null
  $beforeSample = $null
  $otherTitles = @()
  $verdict = "not_set"
  $succeeded = $false

  try {
    # ---- authorization -----------------------------------------------------
    $rt.Stage = "authorization"
    $expectations = Assert-AcceptanceInvocationAuthorization `
      -IsApplyMode ([bool]$Apply) `
      -ExecutablePath $ExpectedExecutablePath `
      -Sha256 $ExpectedSha256 `
      -ProductVersion $ExpectedProductVersion `
      -GitHead $ExpectedGitHead `
      -MonitorIdentifier $ExpectedMonitorIdentifier `
      -MonitorPhysicalResolution $ExpectedMonitorPhysicalResolution `
      -EffectiveDpi $ExpectedEffectiveDpi `
      -SampleIntervalMsValue $SampleIntervalMs `
      -MaxSampleAttemptsValue $MaxSampleAttempts

    # ---- evidence directory -------------------------------------------------
    $rt.Stage = "evidence_directory"
    $effectiveRoot = $EvidenceRootPath
    if ([string]::IsNullOrWhiteSpace($effectiveRoot)) {
      $effectiveRoot = Join-Path $script:CheckoutRoot "qa\artifacts\native-physical-acceptance"
    }
    $rt.EvidenceDirectoryPath = New-AcceptanceEvidenceDirectory -RootPath $effectiveRoot -Slug $EvidenceSlug

    # ---- target resolution ---------------------------------------------------
    $rt.Stage = "target_resolution"
    if ($mode -eq "apply") {
      $effectiveExe = $expectations.ExecutablePath
    } elseif ($expectations.ExecutablePath -ne "") {
      $effectiveExe = $expectations.ExecutablePath
    } else {
      $effectiveExe = [IO.Path]::GetFullPath((Join-Path $script:CheckoutRoot "target\release\syndocal.exe"))
    }
    $fingerprint = Get-CheckoutTargetExecutableFingerprint -TargetExecutablePath $effectiveExe
    $candidateIds = @(Get-SyndocalCandidateProcessIds)
    $provenPidValue = Select-SyndocalTargetProcessId `
      -CandidateProcessIds $candidateIds `
      -TargetFingerprint $fingerprint
    $rt.ProvenPid = [uint32]$provenPidValue
    $observedSha256 = Get-ExecutableSha256 -Path $effectiveExe
    $observedVersion = Get-ExecutableProductVersion -Path $effectiveExe
    $observedHead = Resolve-GitHeadFromRepository -CheckoutRootPath $script:CheckoutRoot
    $exeItem = Get-Item -LiteralPath $effectiveExe
    if ($mode -eq "apply") {
      if ($observedSha256 -cne $expectations.Sha256) {
        throw (
          "authorization identity mismatch: observed sha256 '$observedSha256' does not equal the expected " +
          "sha256 '$($expectations.Sha256)' for '$effectiveExe'.")
      }
      if ($observedVersion -cne $expectations.ProductVersion) {
        throw (
          "authorization identity mismatch: observed product version '$observedVersion' does not equal the " +
          "expected '$($expectations.ProductVersion)'.")
      }
      $null = Assert-ObservedGitHeadMatchesExpectation -ObservedHead $observedHead -ExpectedHead $expectations.GitHead
    }

    # ---- display topology + window resolution -------------------------------
    $rt.Stage = "display_and_window_resolution"
    $monitorIdentities = @(Get-DisplayMonitorIdentities)
    if ($mode -eq "apply") {
      $criteriaBasis = $expectations
    } else {
      $criteriaBasis = New-PinnedInformationalExpectations -ExecutablePathForReference $effectiveExe
    }
    $targetSelection = Resolve-AcceptanceTargetMonitor `
      -MonitorIdentities $monitorIdentities `
      -Identifier $criteriaBasis.MonitorIdentifier `
      -PhysicalResolution $criteriaBasis.MonitorPhysicalResolution `
      -EffectiveDpi $criteriaBasis.EffectiveDpi `
      -RequireUnique ($mode -eq "apply")
    if ($mode -eq "apply") {
      $uniqueIdentity = $targetSelection.UniqueMatch
      $expectations.TargetMonitorHandleDecimal = [long]$uniqueIdentity.handle_decimal
      $workDetail = $uniqueIdentity.work_area
      $expectations.TargetMonitorWorkArea = [pscustomobject]@{
        left = [int]$workDetail.left
        top = [int]$workDetail.top
        width = [int]$workDetail.width
        height = [int]$workDetail.height
      }
    }

    $visibleTopLevel = @(Get-VisibleTopLevelWindowsForPid -OwnerPid ([uint32]$provenPidValue))
    $mainSelection = Select-MainAcceptanceWindow `
      -VisibleTopLevelEntries $visibleTopLevel `
      -ProvenPid ([uint32]$provenPidValue)
    $rt.MainHwndDecimal = [long]$mainSelection.HwndDecimal
    $otherTitles = @(
      $visibleTopLevel |
        Where-Object { $_.Handle.ToInt64() -ne $mainSelection.HwndDecimal } |
        ForEach-Object { $_.Title }
    )

    $beforeSample = New-AcceptanceStateSample `
      -HwndDecimal $mainSelection.HwndDecimal `
      -ProvenPid ([uint32]$provenPidValue) `
      -Expectations $criteriaBasis `
      -DeepIdentityProof ($mode -eq "apply")

    # ---- evidence: provenance / display / before ----------------------------
    $provenancePayload = [ordered]@{
      schema_version = 1
      tool = $toolName
      mode = $mode
      started_utc = $rt.StartedUtc
      finished_utc = (Get-AcceptanceUtcTimestamp)
      checkout_root = $script:CheckoutRoot
      harness_path = $script:AcceptanceHarnessFilePath
      git_head_observed = $observedHead
      host = [ordered]@{
        ps_version = [string]$PSVersionTable.PSVersion
        ps_edition = [string]$PSVersionTable.PSEdition
        os_version = [Environment]::OSVersion.VersionString
      }
      pinned_acceptance_constants = [ordered]@{
        monitor_identifier = $pin.MonitorIdentifier
        monitor_physical_resolution = $pin.MonitorPhysicalResolution
        effective_dpi = $pin.EffectiveDpi
        client_physical_width = $pin.ClientPhysicalWidth
        client_physical_height = $pin.ClientPhysicalHeight
        client_logical_width = $pin.ClientLogicalWidth
        client_logical_height = $pin.ClientLogicalHeight
        required_consecutive_stable_samples = $pin.RequiredConsecutiveStableSamples
        minimum_sample_interval_ms = $pin.MinimumSampleIntervalMs
      }
      expectations_provided = [ordered]@{
        executable_path = $expectations.RequestedExecutablePath
        sha256 = $expectations.Sha256
        product_version = $expectations.ProductVersion
        git_head = $expectations.GitHead
        monitor_identifier = $expectations.MonitorIdentifier
        monitor_physical_resolution = $expectations.MonitorPhysicalResolution
        effective_dpi = $expectations.EffectiveDpi
        informational_only = [bool]$expectations.InformationalOnly
      }
      target_executable = [ordered]@{
        requested_path = $effectiveExe
        native_final_path = $fingerprint.FinalPath
        volume_serial_number = $fingerprint.Identity.VolumeSerialNumber
        file_index = $fingerprint.Identity.FileIndex
        file_size_bytes = $exeItem.Length
        observed_sha256 = $observedSha256
        observed_product_version = $observedVersion
        pid = [uint32]$provenPidValue
      }
      process_match = [ordered]@{
        strategy = "inherited_from_observe_syndocal_window_geometry_ps1_native_final_path_and_volume_file_identity"
        candidate_count = $candidateIds.Count
        matched_process_count = 1
      }
      screenshot_claimed = $false
    }
    [void](Add-AcceptanceEvidenceArtifact -EvidenceDirectoryPath $rt.EvidenceDirectoryPath -FileName "provenance.json" `
      -JsonText (($provenancePayload | ConvertTo-Json -Depth 12)) -ManifestList $rt.ManifestList)

    $displayPayload = [ordered]@{
      schema_version = 1
      tool = $toolName
      utc = (Get-AcceptanceUtcTimestamp)
      trusted_identity_basis = "description_string_plus_physical_pixel_bounds_plus_effective_dpi"
      display_ordinal_trusted = $false
      monitors = @($monitorIdentities)
      selection = [ordered]@{
        required_unique = ($mode -eq "apply")
        match_count = $targetSelection.MatchCount
        target_monitor_handle_decimal = $(
          if ($null -ne $targetSelection.UniqueMatch) { [long]$targetSelection.UniqueMatch.handle_decimal } else { $null })
        ambiguous_duplicate_tuples_rejected_in_apply_mode = $true
      }
    }
    [void](Add-AcceptanceEvidenceArtifact -EvidenceDirectoryPath $rt.EvidenceDirectoryPath -FileName "display.json" `
      -JsonText (($displayPayload | ConvertTo-Json -Depth 12)) -ManifestList $rt.ManifestList)

    $beforePayload = [ordered]@{
      schema_version = 1
      tool = $toolName
      utc = (Get-AcceptanceUtcTimestamp)
      expected_main_window_title = $script:AcceptanceMainWindowTitle
      other_visible_top_level_titles = @($otherTitles)
      sample = $beforeSample
    }
    [void](Add-AcceptanceEvidenceArtifact -EvidenceDirectoryPath $rt.EvidenceDirectoryPath -FileName "before.json" `
      -JsonText (($beforePayload | ConvertTo-Json -Depth 12)) -ManifestList $rt.ManifestList)

    # ---- operations ----------------------------------------------------------
    $rt.Stage = "operations"
    if ($mode -eq "apply") {
      $plannedOps = Decide-AuthorizedOperations `
        -InitialSample $beforeSample `
        -TargetMonitorHandleDecimal ([long]$expectations.TargetMonitorHandleDecimal)
      foreach ($operationKind in $plannedOps) {
        [void](Invoke-AuthorizedWindowOperation `
          -OperationKind $operationKind `
          -HwndDecimal $mainSelection.HwndDecimal `
          -ProvenPid ([uint32]$provenPidValue) `
          -Expectations $expectations `
          -OperationsLog $rt.OperationsLog)
      }
    }

    $operationPayload = [ordered]@{
      schema_version = 1
      tool = $toolName
      utc = (Get-AcceptanceUtcTimestamp)
      mode = $mode
      performed = ($mode -eq "apply")
      reason = $(if ($mode -eq "apply") { "" } else { "apply_not_authorized_dry_run_read_only" })
      planned_operations = @($plannedOps)
      executed_operations = @($rt.OperationsLog)
      pane_children_operated = $false
      state_changing_api_paths = @("Invoke-ShowWindowState", "Invoke-SetWindowPlacementRect")
    }
    [void](Add-AcceptanceEvidenceArtifact -EvidenceDirectoryPath $rt.EvidenceDirectoryPath -FileName "operation.json" `
      -JsonText (($operationPayload | ConvertTo-Json -Depth 12)) -ManifestList $rt.ManifestList)

    # ---- stability verification ----------------------------------------------
    $rt.Stage = "stability_verification"
    $stability = Invoke-StabilityVerification `
      -HwndDecimal $mainSelection.HwndDecimal `
      -ProvenPid ([uint32]$provenPidValue) `
      -Expectations $criteriaBasis `
      -DeepIdentityProof ($mode -eq "apply") `
      -SampleIntervalMsValue $SampleIntervalMs `
      -MaxSampleAttemptsValue $MaxSampleAttempts `
      -RequiredConsecutiveStableSamples ([int]$pin.RequiredConsecutiveStableSamples)
    $rt.Stability = $stability

    $lastSample = $null
    if (@($stability.samples).Count -gt 0) {
      $lastSample = @($stability.samples)[@($stability.samples).Count - 1]
    }

    if ($mode -eq "apply") {
      if ([bool]$stability.achieved) {
        $verdict = "acceptance_passed"
        $succeeded = $true
      } else {
        $verdict = "acceptance_failed_stability_not_achieved"
        $succeeded = $false
      }
    } else {
      $verdict = "dry_run_observation_complete"
      $succeeded = $true
    }

    $finalPayload = [ordered]@{
      schema_version = 1
      tool = $toolName
      utc = (Get-AcceptanceUtcTimestamp)
      mode = $mode
      verdict = $verdict
      succeeded = [bool]$succeeded
      stability = [ordered]@{
        required_consecutive_stable_samples = [int]$pin.RequiredConsecutiveStableSamples
        attempts_used = $stability.attempts_used
        consecutive_stable_achieved = $stability.consecutive_stable_achieved
        achieved = [bool]$stability.achieved
        configured_sample_interval_ms = $SampleIntervalMs
        minimum_sample_interval_ms = [int]$pin.MinimumSampleIntervalMs
        delays_requested_ms = @($stability.delays_requested_ms)
        samples = @($stability.samples)
      }
      last_sample_all_criteria_met = $(if ($null -ne $lastSample) { [bool]$lastSample.all_criteria_met } else { $false })
      screenshots_captured = 0
    }
    [void](Add-AcceptanceEvidenceArtifact -EvidenceDirectoryPath $rt.EvidenceDirectoryPath -FileName "final.json" `
      -JsonText (($finalPayload | ConvertTo-Json -Depth 12)) -ManifestList $rt.ManifestList)

    if (-not $succeeded) {
      Write-AcceptanceFailureRecord `
        -Mode $mode `
        -Stage $rt.Stage `
        -Message ("stability gate failed: {0}/{1} consecutive stable samples after {2} attempts" -f `
          $stability.consecutive_stable_achieved, $pin.RequiredConsecutiveStableSamples, $stability.attempts_used) `
        -EvidenceDirectoryPath $rt.EvidenceDirectoryPath `
        -ManifestList $rt.ManifestList
    }

    # ---- finalization ----------------------------------------------------------
    $rt.Stage = "evidence_finalization"
    Complete-AcceptanceEvidenceManifest `
      -EvidenceDirectoryPath $rt.EvidenceDirectoryPath `
      -ManifestList $rt.ManifestList
  } catch {
    $failureMessage = $_.Exception.Message
    [void]$rt.Errors.Add($failureMessage)
    $failedStage = [string]$rt.Stage
    if ($mode -eq "apply") {
      $verdict = "acceptance_failed_at_$failedStage"
    } else {
      $verdict = "dry_run_failed_at_$failedStage"
    }
    $succeeded = $false
    try {
      if ((-not (@($rt.ManifestList) -contains "operation.json")) -and
          (-not [string]::IsNullOrEmpty($rt.EvidenceDirectoryPath))) {
        $partialOperationPayload = [ordered]@{
          schema_version = 1
          tool = $toolName
          utc = (Get-AcceptanceUtcTimestamp)
          mode = $mode
          performed = ($mode -eq "apply")
          reason = "aborted_by_failure_at_stage_$failedStage"
          planned_operations = @($plannedOps)
          executed_operations = @($rt.OperationsLog)
          pane_children_operated = $false
        }
        [void](Add-AcceptanceEvidenceArtifact -EvidenceDirectoryPath $rt.EvidenceDirectoryPath -FileName "operation.json" `
          -JsonText (($partialOperationPayload | ConvertTo-Json -Depth 12)) -ManifestList $rt.ManifestList)
      }
    } catch {
    }
    Write-AcceptanceFailureRecord `
      -Mode $mode `
      -Stage $failedStage `
      -Message $failureMessage `
      -EvidenceDirectoryPath $rt.EvidenceDirectoryPath `
      -ManifestList $rt.ManifestList
  }

  $samplesCollected = 0
  $consecutiveStableAchieved = 0
  $lastCriteriaMet = $false
  if ($null -ne $rt.Stability) {
    $samplesCollected = [int]$rt.Stability.attempts_used
    $consecutiveStableAchieved = [int]$rt.Stability.consecutive_stable_achieved
    $stabilitySamples = @($rt.Stability.samples)
    if ($stabilitySamples.Count -gt 0) {
      $lastCriteriaMet = [bool]$stabilitySamples[$stabilitySamples.Count - 1].all_criteria_met
    }
  }

  return [pscustomobject]@{
    Succeeded = [bool]$succeeded
    Mode = $mode
    Verdict = $verdict
    Stage = [string]$rt.Stage
    EvidencePath = $rt.EvidenceDirectoryPath
    Errors = @($rt.Errors)
    PlannedOperations = @($plannedOps)
    OperationsPerformed = $rt.OperationsLog.Count
    SamplesCollected = $samplesCollected
    ConsecutiveStableAchieved = $consecutiveStableAchieved
    LastSampleAllCriteriaMet = [bool]$lastCriteriaMet
    ProvenPid = [uint32]$rt.ProvenPid
    MainWindowHandle = [long]$rt.MainHwndDecimal
  }
}

# ---------------------------------------------------------------------------
# Process entry point
# ---------------------------------------------------------------------------

function Invoke-AcceptanceMain {
  # Thin wrapper: maps the run result onto console output + exit code.
  param([Parameter(Mandatory = $true)][hashtable]$RunParameters)

  $result = $null
  try {
    $result = Invoke-AcceptanceRun @RunParameters
  } catch {
    # Only failures outside the run's own capture reach here.
    [Console]::Error.WriteLine("ACCEPTANCE-RUNNER-ERROR: $($_.Exception.Message)")
    exit 1
  }
  "MODE: $($result.Mode)"
  "VERDICT: $($result.Verdict)"
  "STAGE: $($result.Stage)"
  "EVIDENCE: $($result.EvidencePath)"
  foreach ($errorMessage in $result.Errors) {
    [Console]::Error.WriteLine("ACCEPTANCE-ERROR: $errorMessage")
  }
  if ($result.Succeeded) { exit 0 } else { exit 1 }
}

# Direct execution runs the acceptance flow; dot-sourcing (tests) only
# defines functions and never touches any window or process.
if ($MyInvocation.InvocationName -ne ".") {
  Invoke-AcceptanceMain -RunParameters @{
    Apply = [bool]$Apply
    ExpectedExecutablePath = $ExpectedExecutablePath
    ExpectedSha256 = $ExpectedSha256
    ExpectedProductVersion = $ExpectedProductVersion
    ExpectedGitHead = $ExpectedGitHead
    ExpectedMonitorIdentifier = $ExpectedMonitorIdentifier
    ExpectedMonitorPhysicalResolution = $ExpectedMonitorPhysicalResolution
    ExpectedEffectiveDpi = $ExpectedEffectiveDpi
    EvidenceSlug = $EvidenceSlug
    EvidenceRootPath = $EvidenceRootPath
    SampleIntervalMs = $SampleIntervalMs
    MaxSampleAttempts = $MaxSampleAttempts
  }
}
