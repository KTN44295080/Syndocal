# Observe the native window geometry of this checkout's Syndocal release build.
#
# READ-ONLY OBSERVER CONTRACT
# This harness is strictly observational. It never manipulates the target
# process or any window: SetForegroundWindow, ShowWindow, ShowWindowAsync,
# MoveWindow, SetWindowPos, SendInput, keybd_event, mouse_event,
# AttachThreadInput, BringWindowToTop, PostMessage and SendMessage are
# PROHIBITED here and none of them are even declared below. The only mutable
# call is SetThreadDpiAwarenessContext on THIS observer thread (the previous
# context is always restored in a finally block) so that Win32 rectangle
# queries return physical pixels; it never touches the target process. Host
# state such as the console output encoding is never modified.
#
# PROCESS IDENTITY PROOF (fail closed)
# 1. Candidates come exclusively from Get-Process -Name syndocal.
# 2. Every candidate's native image path is queried with
#    OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION) plus
#    QueryFullProcessImageNameW. An unreadable or incomplete query fails the
#    whole observation; there is no silent skip path anywhere.
# 3. The candidate image and this checkout's target\release\syndocal.exe are
#    both opened with CreateFileW (FILE_READ_ATTRIBUTES, maximal sharing), and
#    each handle yields its native final path via GetFinalPathNameByHandleW
#    plus stable volume-serial-number/file-index identity via
#    GetFileInformationByHandle. An EXACT match requires normalized
#    final-path equality AND identical volume/file identity. Raw
#    GetFullPath/lexical string equality alone is never accepted as proof:
#    a lexical alias (8.3 short name, different casing) of the same underlying
#    file is accepted, while a lexically equal path backed by a different file
#    generation (rebuilt exe) is rejected. A hard link to that same underlying
#    file placed at a DIFFERENT final path is intentionally rejected as well:
#    acceptance requires BOTH the exact resolved checkout path (normalized
#    final-path equality) AND native volume/file identity, so neither an alias
#    trick nor a same-identity link at another location can satisfy it alone.
# 4. Zero syndocal candidates, zero exact matches, and multiple exact matches
#    are all hard failures.
#
# The label-"main" editor window carries the exact title "Syndocal"
# (app/src-tauri/tauri.conf.json); QA handoffs call it the "titled `Syndocal`
# main window". This observer therefore requires exactly one visible top-level
# window whose title equals "Syndocal" (ordinal comparison). That selection is
# fail closed BEFORE any geometry is read: zero eligible windows (missing or
# wrong-title worlds) and multiple eligible windows (ambiguous duplicates)
# both abort the observation with no success output. The single selected main
# window must additionally still exist (HWND liveness re-check), be owned by
# the proven target PID (re-verified via GetWindowThreadProcessId), and be
# responsive (IsHungAppWindow); an unresponsive (hung) main window is also a
# hard failure.
#
# OUTPUT CONTRACT: exactly one JSON object (schema_version 2) on stdout and
# nothing else. Diagnostics go to stderr only; any failure exits nonzero with
# NO success payload on stdout. This harness writes no files and has no
# output-path parameter of any kind.
#
# Deterministic focused tests/seams:
#   qa/harnesses/tests/observe-syndocal-window-geometry.Tests.ps1
# Dot-sourcing this file defines the functions without running the observer,
# which is exactly what those tests rely on. Process identity and window
# enumeration are seams, and so are the per-window native probes (liveness,
# owner pid, rectangles, monitor info, DPI, responsiveness), letting those
# tests drive duplicate/wrong-title/vanished-HWND/hung/monitor-failure/DPI-
# failure worlds deterministically without touching any real window.
#
# Usage:
#   pwsh qa/harnesses/observe-syndocal-window-geometry.ps1

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$script:HarnessFilePath = $PSCommandPath
$script:ExpectedMainWindowTitle = "Syndocal"
$script:SyndocalProcessName = "syndocal"

# Pinned Win32 constants used below:
#   FILE_READ_ATTRIBUTES              = 0x80
#   FILE_SHARE_READ|WRITE|DELETE      = 0x7
#   OPEN_EXISTING                     = 3
#   FILE_FLAG_BACKUP_SEMANTICS        = 0x02000000
#   PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
#   VOLUME_NAME_DOS                   = 0
#   ERROR_INSUFFICIENT_BUFFER         = 122

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
  throw "The Syndocal geometry observer requires Windows."
}

if (-not ("SyndocalGeometryObserverNative" -as [type])) {
  Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public delegate bool SyndocalGeometryEnumProc(IntPtr hWnd, IntPtr lParam);

public static class SyndocalGeometryObserverNative {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
  }

  [StructLayout(LayoutKind.Sequential)]
  public struct POINT {
    public int X;
    public int Y;
  }

  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct MONITORINFOEX {
    public int Size;
    public RECT Monitor;
    public RECT Work;
    public uint Flags;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)]
    public string DeviceName;
  }

  // Layout mirrors Win32 BY_HANDLE_FILE_INFORMATION; FILETIMEs are kept as
  // raw DWORD pairs so the sequential layout stays exact.
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
  public static extern bool EnumWindows(SyndocalGeometryEnumProc callback, IntPtr lParam);

  [DllImport("user32.dll")]
  public static extern bool EnumChildWindows(IntPtr parent, SyndocalGeometryEnumProc callback, IntPtr lParam);

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

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern bool GetMonitorInfo(IntPtr monitor, ref MONITORINFOEX info);

  [DllImport("user32.dll")]
  public static extern uint GetDpiForWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool IsValidDpiAwarenessContext(IntPtr value);

  [DllImport("user32.dll")]
  public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr dpiContext);

  [DllImport("user32.dll")]
  public static extern IntPtr GetThreadDpiAwarenessContext();

  [DllImport("kernel32.dll", SetLastError = true)]
  public static extern IntPtr OpenProcess(uint desiredAccess, bool inheritHandle, uint processId);

  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern bool QueryFullProcessImageNameW(IntPtr processHandle, uint flags, StringBuilder exeName, ref uint exeNameLengthChars);

  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern IntPtr CreateFileW(string fileName, uint desiredAccess, uint shareMode, IntPtr securityAttributes, int creationDisposition, int flagsAndAttributes, IntPtr templateFileHandle);

  [DllImport("kernel32.dll", SetLastError = true)]
  public static extern bool CloseHandle(IntPtr handle);

  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern uint GetFinalPathNameByHandleW(IntPtr handle, StringBuilder buffer, uint bufferLengthChars, uint flags);

  [DllImport("kernel32.dll", SetLastError = true)]
  public static extern bool GetFileInformationByHandle(IntPtr handle, out BY_HANDLE_FILE_INFORMATION information);
}
"@
}

function Get-ObserverWindowTitle {
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  $length = [SyndocalGeometryObserverNative]::GetWindowTextLength($Handle)
  if ($length -le 0) { return "" }
  $builder = [Text.StringBuilder]::new($length + 1)
  [void][SyndocalGeometryObserverNative]::GetWindowText($Handle, $builder, $builder.Capacity)
  $builder.ToString()
}

function ConvertTo-RectDetail {
  param([Parameter(Mandatory = $true)]$Rect)

  [ordered]@{
    left = $Rect.Left
    top = $Rect.Top
    right = $Rect.Right
    bottom = $Rect.Bottom
    width = $Rect.Right - $Rect.Left
    height = $Rect.Bottom - $Rect.Top
  }
}

function ConvertTo-NormalizedFinalPath {
  # Normalizes a GetFinalPathNameByHandleW result: strips the device-form
  # prefix and maps UNC form so both sides of the comparison transform the
  # same way. This is presentation normalization only; proof additionally
  # requires matching volume serial + file index.
  param([Parameter(Mandatory = $true)][string]$FinalPath)

  if (-not $FinalPath.StartsWith("\\?\", [StringComparison]::Ordinal)) {
    throw "GetFinalPathNameByHandleW returned an unrecognized form without the '\\?\' prefix: '$FinalPath'."
  }
  $withoutPrefix = $FinalPath.Substring(4)
  if ($withoutPrefix.StartsWith("UNC\", [StringComparison]::OrdinalIgnoreCase)) {
    return "\" + $withoutPrefix.Substring(3)
  }
  return $withoutPrefix
}

function Get-SyndocalCandidateProcessIds {
  # SEAM: deterministic tests override this to supply fixed candidate sets.
  # Production behavior: enumerate ALL processes named 'syndocal'. Zero hits
  # returns an empty set; Select-SyndocalTargetProcessId rejects zero later.
  # Any enumeration error OTHER than "no process with that name" propagates
  # fail closed with its detail instead of masquerading as zero candidates.
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

function Get-NativeProcessImagePath {
  # Native kernel-backed image path for one candidate process id. Any failure
  # throws: callers fail the whole observation instead of skipping.
  param([Parameter(Mandatory = $true)][uint32]$ProcessId)

  $processHandle = [SyndocalGeometryObserverNative]::OpenProcess(0x1000u, $false, $ProcessId)
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
      $ok = [SyndocalGeometryObserverNative]::QueryFullProcessImageNameW(
        $processHandle, 0u, $buffer, [ref]$lengthChars)
      if ($ok) {
        $imagePath = $buffer.ToString()
        if ([string]::IsNullOrWhiteSpace($imagePath)) {
          throw "Fail closed: syndocal candidate PID $ProcessId image-path query is incomplete (empty result)."
        }
        return $imagePath
      }
      $lastError = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
      if ($lastError -eq 122 -and $capacityChars -lt 32768u) {
        $capacityChars = $capacityChars * 2u
        continue
      }
      throw (
        "Fail closed: syndocal candidate PID $ProcessId image-path query failed; " +
        "QueryFullProcessImageNameW returned FALSE (Win32 error $lastError).")
    }
  } finally {
    [void][SyndocalGeometryObserverNative]::CloseHandle($processHandle)
  }
}

function Open-StableIdentityHandle {
  # Opens a path with read-attributes access and maximal sharing so a running
  # executable image can always be inspected without perturbing it.
  param([Parameter(Mandatory = $true)][string]$Path)

  $handle = [SyndocalGeometryObserverNative]::CreateFileW(
    $Path, 0x80u, 7u, [IntPtr]::Zero, 3, 0x02000000, [IntPtr]::Zero)
  if ($handle -eq [IntPtr](-1)) {
    throw (
      "Fail closed: stable-identity handle could not be opened for '$Path' " +
      "(Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())).")
  }
  return $handle
}

function Get-FinalPathFromHandle {
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  $requiredChars = [SyndocalGeometryObserverNative]::GetFinalPathNameByHandleW($Handle, $null, 0u, 0u)
  if ($requiredChars -eq 0u) {
    throw (
      "Fail closed: GetFinalPathNameByHandleW sizing call failed " +
      "(Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())).")
  }
  $buffer = [Text.StringBuilder]::new([int]$requiredChars)
  $writtenChars = [SyndocalGeometryObserverNative]::GetFinalPathNameByHandleW($Handle, $buffer, $requiredChars, 0u)
  if ($writtenChars -eq 0u) {
    throw (
      "Fail closed: GetFinalPathNameByHandleW failed " +
      "(Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())).")
  }
  return $buffer.ToString()
}

function Get-FileIdentityFromHandle {
  # Stable identity of the opened file: volume serial number plus 64-bit file
  # index. Two handles to the SAME underlying file agree on both values even
  # when reached through lexically different aliases.
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  $information = [SyndocalGeometryObserverNative+BY_HANDLE_FILE_INFORMATION]::new()
  if (-not [SyndocalGeometryObserverNative]::GetFileInformationByHandle($Handle, [ref]$information)) {
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
  # Native fingerprint of THIS checkout's expected executable: normalized
  # final path plus stable volume/file identity. RequestedPath is kept for
  # diagnostics only; it never proves anything by itself.
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
    [void][SyndocalGeometryObserverNative]::CloseHandle($identityHandle)
  }
  [pscustomobject]@{
    RequestedPath = $requestedPath
    FinalPath = $finalPath
    Identity = $identity
  }
}

function Select-SyndocalTargetProcessId {
  # Core fail-closed selection. Every named candidate MUST be fully readable
  # (native image path, openable file, final path, stable identity); ANY
  # unreadable or incomplete query aborts the whole observation. Exactly one
  # candidate may match the target fingerprint by normalized final path AND
  # volume/file identity; zero or multiple matches abort.
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
        [void][SyndocalGeometryObserverNative]::CloseHandle($identityHandle)
      }
    }
    $isExactMatch =
      ([string]::Equals($finalPath, $TargetFingerprint.FinalPath, [StringComparison]::OrdinalIgnoreCase)) -and
      (Test-FileIdentityEqual -Left $identity -Right $TargetFingerprint.Identity)
    $resolutions.Add([pscustomobject]@{
      ProcessId = $candidateId
      NativeImagePath = $nativeImagePath
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
    $observed = (
      @($resolutions | ForEach-Object {
        ("PID {0} -> '{1}' vol=0x{2:x8} idx={3}" -f $_.ProcessId, $_.FinalPath, $_.Identity.VolumeSerialNumber, $_.Identity.FileIndex)
      }) -join "; ")
    throw (
      "Fail closed: found 0 exact identity matches for '$($TargetFingerprint.RequestedPath)' " +
      "among $($candidateIds.Count) syndocal candidate(s). Resolved: $observed.")
  }
  return [uint32]$exactMatches[0].ProcessId
}

function Get-VisibleTopLevelWindowsForPid {
  # SEAM: deterministic tests override this to force exceptions inside the
  # DPI-protected region without touching real windows.
  param([Parameter(Mandatory = $true)][uint32]$OwnerPid)

  $visibleTopLevel = [System.Collections.Generic.List[object]]::new()
  $topLevelCallback = [SyndocalGeometryEnumProc]{
    param([IntPtr]$Handle, [IntPtr]$Unused)
    [uint32]$windowOwnerPid = 0
    [void][SyndocalGeometryObserverNative]::GetWindowThreadProcessId($Handle, [ref]$windowOwnerPid)
    if ($windowOwnerPid -eq $OwnerPid -and [SyndocalGeometryObserverNative]::IsWindowVisible($Handle)) {
      $visibleTopLevel.Add([pscustomobject]@{
        Handle = $Handle
        Title = Get-ObserverWindowTitle -Handle $Handle
      })
    }
    return $true
  }
  [void][SyndocalGeometryObserverNative]::EnumWindows($topLevelCallback, [IntPtr]::Zero)
  return @($visibleTopLevel | Sort-Object -Property { $_.Handle.ToInt64() })
}

function Test-NativeWindowAlive {
  # SEAM: wraps IsWindow so deterministic tests can force HWND disappearance
  # mid-observation without touching any real window.
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  [bool][SyndocalGeometryObserverNative]::IsWindow($Handle)
}

function Get-WindowOwnerProcessId {
  # SEAM: wraps GetWindowThreadProcessId so deterministic tests can prove or
  # mismatch main-window ownership; a failed query yields 0, which always
  # mismatches the proven target PID and fails closed.
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  [uint32]$windowOwnerPid = 0
  [void][SyndocalGeometryObserverNative]::GetWindowThreadProcessId($Handle, [ref]$windowOwnerPid)
  $windowOwnerPid
}

function Test-WindowNotHung {
  # SEAM: wraps -IsHungAppWindow; $false means the window is not responding.
  # Used both for the fail-closed main-window gate and the reported per-window
  # "responding" field.
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  -not [bool][SyndocalGeometryObserverNative]::IsHungAppWindow($Handle)
}

function Get-ObservationWindowRectangles {
  # SEAM: wraps GetWindowRect + GetClientRect + ClientToScreen so tests can
  # drive the geometry path with synthetic rectangles. Any FALSE result is a
  # hard failure.
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  $outerRect = [SyndocalGeometryObserverNative+RECT]::new()
  if (-not [SyndocalGeometryObserverNative]::GetWindowRect($Handle, [ref]$outerRect)) {
    throw "Fail closed: GetWindowRect failed for HWND $($Handle.ToInt64())."
  }
  $clientRect = [SyndocalGeometryObserverNative+RECT]::new()
  if (-not [SyndocalGeometryObserverNative]::GetClientRect($Handle, [ref]$clientRect)) {
    throw "Fail closed: GetClientRect failed for HWND $($Handle.ToInt64())."
  }
  $clientOrigin = [SyndocalGeometryObserverNative+POINT]::new()
  if (-not [SyndocalGeometryObserverNative]::ClientToScreen($Handle, [ref]$clientOrigin)) {
    throw "Fail closed: ClientToScreen failed for HWND $($Handle.ToInt64())."
  }
  [pscustomobject]@{
    Outer = $outerRect
    Client = $clientRect
    ClientOrigin = $clientOrigin
  }
}

function Get-ObservationMonitorInfo {
  # SEAM: wraps MonitorFromWindow + GetMonitorInfo; zero monitor handle or a
  # FALSE GetMonitorInfo result is a hard failure.
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  $monitorHandle = [SyndocalGeometryObserverNative]::MonitorFromWindow($Handle, 2u)
  $monitorInfo = [SyndocalGeometryObserverNative+MONITORINFOEX]::new()
  $monitorInfo.Size =
    [Runtime.InteropServices.Marshal]::SizeOf([type][SyndocalGeometryObserverNative+MONITORINFOEX])
  if (
    $monitorHandle -eq [IntPtr]::Zero -or
    -not [SyndocalGeometryObserverNative]::GetMonitorInfo($monitorHandle, [ref]$monitorInfo)
  ) {
    throw "Fail closed: GetMonitorInfo failed for HWND $($Handle.ToInt64())."
  }
  $monitorInfo
}

function Get-EffectiveDpiForWindow {
  # SEAM: wraps GetDpiForWindow; callers reject 0 as an unprovable scale factor.
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  [uint32][SyndocalGeometryObserverNative]::GetDpiForWindow($Handle)
}

function Get-ObservationWindowStateFlags {
  # SEAM: read-only per-window state bits reported in the observation payload.
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  [pscustomobject]@{
    Visible = [bool][SyndocalGeometryObserverNative]::IsWindowVisible($Handle)
    Responding = Test-WindowNotHung -Handle $Handle
    Minimized = [bool][SyndocalGeometryObserverNative]::IsIconic($Handle)
    Maximized = [bool][SyndocalGeometryObserverNative]::IsZoomed($Handle)
  }
}

function Invoke-SyndocalGeometryObservation {
  # Returns the observation JSON string. The observer-thread DPI context is
  # set inside try and ALWAYS restored in finally, including forced failures.
  # [IO.Path]::GetFullPath below only CONSTRUCTS the checkout-relative path
  # for display and opening; it is never used as identity proof.

  $checkoutRoot =
    Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $script:HarnessFilePath))
  $targetExecutable = [IO.Path]::GetFullPath((Join-Path $checkoutRoot "target\release\syndocal.exe"))
  $targetFingerprint = Get-CheckoutTargetExecutableFingerprint -TargetExecutablePath $targetExecutable

  $candidateIds = @(Get-SyndocalCandidateProcessIds)
  $observerPidValue = Select-SyndocalTargetProcessId `
    -CandidateProcessIds $candidateIds `
    -TargetFingerprint $targetFingerprint

  $executableItem = Get-Item -LiteralPath $targetExecutable
  $sha256 = (Get-FileHash -LiteralPath $targetExecutable -Algorithm SHA256).Hash.ToLowerInvariant()

  $previousDpiContext = [IntPtr]::Zero
  try {
    # Per-monitor-v2 awareness for the OBSERVER THREAD ONLY, so every rectangle
    # below is physical pixels regardless of how pwsh was launched. The target
    # process awareness is never modified. Fail closed if the OS cannot prove it.
    if (-not [SyndocalGeometryObserverNative]::IsValidDpiAwarenessContext([IntPtr](-4))) {
      throw "This Windows version cannot prove per-monitor-v2 coordinates; refusing to report unprovable units."
    }
    $previousDpiContext =
      [SyndocalGeometryObserverNative]::SetThreadDpiAwarenessContext([IntPtr](-4))
    if ($previousDpiContext -eq [IntPtr]::Zero) {
      throw "SetThreadDpiAwarenessContext failed; returned coordinates could not be proven physical."
    }

    $sortedTopLevel = @(Get-VisibleTopLevelWindowsForPid -OwnerPid $observerPidValue)

    # Fail-closed main-window selection BEFORE any per-window geometry query:
    # exactly one visible top-level window of the proven PID may carry the
    # exact main title. Missing/wrong-title and ambiguous-duplicate worlds
    # abort here with diagnostics and never produce success output.
    $selectedMainCandidates = @(
      $sortedTopLevel |
        Where-Object {
          [string]::Equals(
            $_.Title,
            $script:ExpectedMainWindowTitle,
            [StringComparison]::Ordinal)
        }
    )
    if ($selectedMainCandidates.Count -ne 1) {
      if ($sortedTopLevel.Count -gt 0) {
        $titles = (@($sortedTopLevel) | ForEach-Object {
          "'$($_.Title)' (hwnd=$($_.Handle.ToInt64()))"
        }) -join ", "
      } else {
        $titles = "<none>"
      }
      throw (
        "Fail closed: expected exactly one '$($script:ExpectedMainWindowTitle)' main window " +
        "for PID $observerPidValue, found $($selectedMainCandidates.Count). " +
        "Visible top-level titles of PID $observerPidValue : $titles")
    }
    $mainSelection = $selectedMainCandidates[0]
    $mainSelectionHwnd = [IntPtr]$mainSelection.Handle

    if (-not (Test-NativeWindowAlive -Handle $mainSelectionHwnd)) {
      throw (
        "Fail closed: main window HWND $($mainSelectionHwnd.ToInt64()) vanished during observation; " +
        "result would be inconsistent.")
    }
    $mainOwnerPidValue = Get-WindowOwnerProcessId -Handle $mainSelectionHwnd
    if ($mainOwnerPidValue -ne $observerPidValue) {
      throw (
        "Fail closed: main window HWND $($mainSelectionHwnd.ToInt64()) is owned by PID $mainOwnerPidValue, " +
        "not the proven target PID $observerPidValue; refusing to report it as this checkout's main window.")
    }
    if (-not (Test-WindowNotHung -Handle $mainSelectionHwnd)) {
      throw (
        "Fail closed: main window HWND $($mainSelectionHwnd.ToInt64()) is not responding " +
        "(IsHungAppWindow reported hung); refusing to report geometry of an unresponsive main window.")
    }

    $windowEntries = [System.Collections.Generic.List[object]]::new()
    foreach ($entry in $sortedTopLevel) {
      $handle = $entry.Handle
      if (-not (Test-NativeWindowAlive -Handle $handle)) {
        throw (
          "Fail closed: top-level window $($handle.ToInt64()) vanished during observation; " +
          "result would be inconsistent.")
      }

      $rectangles = Get-ObservationWindowRectangles -Handle $handle
      $outerRect = $rectangles.Outer
      $clientRect = $rectangles.Client
      $clientOrigin = $rectangles.ClientOrigin

      $monitorInfo = Get-ObservationMonitorInfo -Handle $handle

      $effectiveDpi = Get-EffectiveDpiForWindow -Handle $handle
      if ($effectiveDpi -eq 0) {
        throw (
          "Fail closed: GetDpiForWindow returned 0 for HWND $($handle.ToInt64()); " +
          "scale factor would be unprovable.")
      }
      $scaleFactor = $effectiveDpi / 96.0
      $clientPhysicalWidth = $clientRect.Right - $clientRect.Left
      $clientPhysicalHeight = $clientRect.Bottom - $clientRect.Top

      $isMainTitle = [string]::Equals(
        $entry.Title,
        $script:ExpectedMainWindowTitle,
        [StringComparison]::Ordinal
      )
      $stateFlags = Get-ObservationWindowStateFlags -Handle $handle

      $windowEntries.Add([ordered]@{
        hwnd_decimal = $handle.ToInt64()
        hwnd_hex = "0x{0:x}" -f $handle.ToInt64()
        title = $entry.Title
        is_main_title_match = $isMainTitle
        visible = $stateFlags.Visible
        responding = $stateFlags.Responding
        minimized = $stateFlags.Minimized
        maximized = $stateFlags.Maximized
        outer_physical = ConvertTo-RectDetail -Rect $outerRect
        client_physical = [ordered]@{
          left = $clientOrigin.X
          top = $clientOrigin.Y
          width = $clientPhysicalWidth
          height = $clientPhysicalHeight
        }
        monitor = [ordered]@{
          device_name = $monitorInfo.DeviceName
          physical_bounds = ConvertTo-RectDetail -Rect $monitorInfo.Monitor
          work_area = ConvertTo-RectDetail -Rect $monitorInfo.Work
        }
        effective_dpi = $effectiveDpi
        scale_factor = $scaleFactor
        webview_viewport_css = [ordered]@{
          width = [math]::Round($clientPhysicalWidth / $scaleFactor, 3)
          height = [math]::Round($clientPhysicalHeight / $scaleFactor, 3)
          derived = $true
          derivation = "client_physical / (effective_dpi / 96)"
        }
      })
    }

    $mainEntries = @($windowEntries | Where-Object { $_.is_main_title_match })
    if ($mainEntries.Count -ne 1) {
      $titles = ($sortedTopLevel | ForEach-Object { "'$($_.Title)'" }) -join ", "
      throw "Expected exactly one '$($script:ExpectedMainWindowTitle)' main window, found $($mainEntries.Count). Visible top-level titles of PID $observerPidValue : $titles"
    }
    $mainEntry = $mainEntries[0]
    $mainHwnd = [IntPtr]$mainEntry.hwnd_decimal

    $observedChildren = [System.Collections.Generic.List[object]]::new()
    $childCallback = [SyndocalGeometryEnumProc]{
      param([IntPtr]$Handle, [IntPtr]$Unused)
      $observedChildren.Add([pscustomobject]@{
        Handle = $Handle
        Title = Get-ObserverWindowTitle -Handle $Handle
        Visible = [bool][SyndocalGeometryObserverNative]::IsWindowVisible($Handle)
      })
      return $true
    }
    [void][SyndocalGeometryObserverNative]::EnumChildWindows($mainHwnd, $childCallback, [IntPtr]::Zero)
    $childEntries = @(
      $observedChildren |
        Sort-Object -Property { $_.Handle.ToInt64() } |
        ForEach-Object {
          [ordered]@{
            hwnd_decimal = $_.Handle.ToInt64()
            hwnd_hex = "0x{0:x}" -f $_.Handle.ToInt64()
            title = $_.Title
            visible = $_.Visible
          }
        }
    )

    $otherTopLevelEntries = @(
      $sortedTopLevel |
        Where-Object { $_.Handle.ToInt64() -ne $mainEntry.hwnd_decimal } |
        ForEach-Object {
          [ordered]@{
            hwnd_decimal = $_.Handle.ToInt64()
            hwnd_hex = "0x{0:x}" -f $_.Handle.ToInt64()
            title = $_.Title
          }
        }
    )

    $report = [ordered]@{
      schema_version = 2
      tool = "observe-syndocal-window-geometry"
      capture_utc = [DateTime]::UtcNow.ToString("o")
      coordinate_contract = [ordered]@{
        units = "physical_pixels"
        dpi_awareness = "per_monitor_v2_on_observer_thread_only"
        target_process_modified = $false
        state_changing_ui_api_calls = 0
      }
      target_executable = [ordered]@{
        path = $targetExecutable
        native_final_path = $targetFingerprint.FinalPath
        volume_serial_number = $targetFingerprint.Identity.VolumeSerialNumber
        file_index = $targetFingerprint.Identity.FileIndex
        pid = $observerPidValue
        file_size_bytes = $executableItem.Length
        sha256 = $sha256
      }
      process_match = [ordered]@{
        strategy = "get_process_named_syndocal_native_image_path_plus_createfilew_final_path_and_volume_file_identity"
        candidate_count = $candidateIds.Count
        matched_process_count = 1
        unreadable_candidate_count = 0
      }
      expected_main_window_title = $script:ExpectedMainWindowTitle
      main_window_count = $mainEntries.Count
      windows = @($windowEntries)
      main_window = $mainEntry
      main_child_windows = $childEntries
      other_top_level_windows = $otherTopLevelEntries
    }
    return ($report | ConvertTo-Json -Depth 8)
  } finally {
    if ($previousDpiContext -ne [IntPtr]::Zero) {
      [void][SyndocalGeometryObserverNative]::SetThreadDpiAwarenessContext($previousDpiContext)
    }
  }
}

function Invoke-ObserverMain {
  try {
    $observationJson = Invoke-SyndocalGeometryObservation
    $observationJson
    exit 0
  } catch {
    [Console]::Error.WriteLine("OBSERVER-ERROR: $($_.Exception.Message)")
    exit 1
  }
}

# Direct execution runs the observer; dot-sourcing (tests) only defines
# functions and never mutates anything.
if ($MyInvocation.InvocationName -ne ".") {
  Invoke-ObserverMain
}
