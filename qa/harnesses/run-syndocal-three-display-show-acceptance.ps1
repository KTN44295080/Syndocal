# Fail-closed native three-display Syndocal show acceptance harness.
#
# This is an observation-first harness for exactly one checkout-local
# syndocal.exe, its one "Syndocal" editor window, and exactly two live native
# Display-output windows.  It never creates outputs, changes Syndocal
# settings, launches or terminates a process, talks to hardware/network, takes
# screenshots, injects input, changes focus, or changes Z-order.  The default
# is a read-only dry-run.  Dry-run always writes a clearly labelled observation
# verdict; it never calls a window mutation API and never calls the result an
# acceptance.  -Apply is deliberately narrow: after every identity and
# placement proof has passed, it may maximize the already-correct editor window
# only.  It never moves an output window, so a swapped/default/first-monitor
# placement is rejected rather than repaired into an accidental pass.
#
# Required physical-role contract (all monitor selection is by the explicit
# stable monitor identity, NEVER by resolution, DISPLAY ordinal, primary flag,
# or first match):
#   editor/operator  1920x1080
#   LED output        1920x1080
#   projector output  3840x2160
# The two 1920x1080 roles must therefore still provide distinct identities.
# The stable identity is the active DisplayConfig monitor-device path, matched
# back to the exact GDI monitor name from GetMonitorInfoW.  It is reported in
# inventory evidence verbatim; users copy it into the three expected-identity
# parameters.  Blank, duplicate, stale, disconnected, or unresolvable paths
# fail closed.  Resolution is checked only AFTER exact identity selection.
#
# Native output roles are proven by the current production contract:
#   label video-output-<output id>
#   live title "Syndocal Output - <output label>"
# plus one exact HWND, visible/responsive/non-minimized state, owner PID equal
# to the proven checkout process, and exact monitor identity.  The Tauri label
# is not a Win32 property, so the evidence records its deterministic native
# label derived from the explicit output ID and proves the live native role by
# the exact corresponding title/owner/placement.  Test-pattern titles are not
# accepted as live output roles.
#
# Evidence is always retained under a NEW, unique, non-reparse direct child of
# the evidence root.  No file or directory is overwritten or deleted.
# Artifacts: provenance.json, monitors.json, before.json, operation.json,
# final.json, optional failure.json, and SHA256SUMS.txt written last.
# Screenshots are neither taken nor claimed.
#
# Self-test seams are the functions marked SEAM.  The companion self-test
# replaces every UI/process/network-facing seam with deterministic synthetic
# values; it performs no real UI/process/network mutation.

[CmdletBinding()]
param(
  [switch]$Apply,

  [string]$ExpectedExecutablePath = "",
  [string]$ExpectedSha256 = "",
  [string]$ExpectedProductVersion = "",
  [string]$ExpectedGitHead = "",

  [string]$ExpectedEditorMonitorIdentity = "",
  [string]$ExpectedLedMonitorIdentity = "",
  [string]$ExpectedProjectorMonitorIdentity = "",

  [UInt64]$LedOutputId = 0,
  [string]$LedOutputLabel = "",
  [UInt64]$ProjectorOutputId = 0,
  [string]$ProjectorOutputLabel = "",

  [string]$EvidenceSlug = "",
  [string]$EvidenceRootPath = "",
  [int]$SampleIntervalMs = 300,
  [int]$MaxSampleAttempts = 12
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$script:ThreeDisplayHarnessPath = $PSCommandPath
$script:ThreeDisplayCheckoutRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$script:ThreeDisplayMainTitle = "Syndocal"
$script:ThreeDisplayOutputTitlePrefix = "Syndocal Output - "
$script:ThreeDisplayRequiredSamples = 3
$script:ThreeDisplayMinimumSampleIntervalMs = 200
$script:ThreeDisplaySchemaVersion = 1

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
  throw "The Syndocal three-display acceptance harness requires Windows."
}

if (-not ("SyndocalThreeDisplayNative" -as [type])) {
  Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public delegate bool SyndocalThreeDisplayEnumProc(IntPtr hWnd, IntPtr lParam);
public delegate bool SyndocalThreeDisplayMonitorEnumProc(IntPtr monitor, IntPtr hdc, IntPtr rect, IntPtr data);

public static class SyndocalThreeDisplayNative {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
  [StructLayout(LayoutKind.Sequential)]
  public struct POINT { public int X; public int Y; }
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct MONITORINFOEXW {
    public int Size;
    public RECT Monitor;
    public RECT Work;
    public uint Flags;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string DeviceName;
  }
  [StructLayout(LayoutKind.Sequential)]
  public struct LUID { public uint LowPart; public int HighPart; }
  [StructLayout(LayoutKind.Sequential)]
  public struct DISPLAYCONFIG_PATH_SOURCE_INFO {
    public LUID AdapterId; public uint Id; public uint ModeInfoIdx; public uint StatusFlags;
  }
  [StructLayout(LayoutKind.Sequential)]
  public struct DISPLAYCONFIG_RATIONAL { public uint Numerator; public uint Denominator; }
  [StructLayout(LayoutKind.Sequential)]
  public struct DISPLAYCONFIG_PATH_TARGET_INFO {
    public LUID AdapterId; public uint Id; public uint ModeInfoIdx; public uint OutputTechnology;
    public uint Rotation; public uint Scaling; public DISPLAYCONFIG_RATIONAL RefreshRate;
    public uint ScanLineOrdering; [MarshalAs(UnmanagedType.Bool)] public bool TargetAvailable;
    public uint StatusFlags;
  }
  [StructLayout(LayoutKind.Sequential)]
  public struct DISPLAYCONFIG_PATH_INFO {
    public DISPLAYCONFIG_PATH_SOURCE_INFO SourceInfo;
    public DISPLAYCONFIG_PATH_TARGET_INFO TargetInfo;
    public uint Flags;
  }
  [StructLayout(LayoutKind.Explicit, Size = 64)]
  public struct DISPLAYCONFIG_MODE_INFO { }
  [StructLayout(LayoutKind.Sequential)]
  public struct DISPLAYCONFIG_DEVICE_INFO_HEADER { public uint Type; public uint Size; public LUID AdapterId; public uint Id; }
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct DISPLAYCONFIG_SOURCE_DEVICE_NAME {
    public DISPLAYCONFIG_DEVICE_INFO_HEADER Header;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string ViewGdiDeviceName;
  }
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct DISPLAYCONFIG_TARGET_DEVICE_NAME {
    public DISPLAYCONFIG_DEVICE_INFO_HEADER Header;
    public uint Flags; public uint OutputTechnology; public ushort EdidManufactureId;
    public ushort EdidProductCodeId; public uint ConnectorInstance;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 64)] public string MonitorFriendlyDeviceName;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string MonitorDevicePath;
  }

  [DllImport("user32.dll")] public static extern bool EnumWindows(SyndocalThreeDisplayEnumProc callback, IntPtr data);
  [DllImport("user32.dll")] public static extern bool EnumDisplayMonitors(IntPtr hdc, IntPtr clip, SyndocalThreeDisplayMonitorEnumProc callback, IntPtr data);
  [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr handle);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr handle);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr handle);
  [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr handle);
  [DllImport("user32.dll")] public static extern bool IsHungAppWindow(IntPtr handle);
  [DllImport("user32.dll")] public static extern int GetWindowTextLengthW(IntPtr handle);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr handle, StringBuilder text, int maxCount);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr handle, out uint processId);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr handle, out RECT rect);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr handle, out RECT rect);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr handle, ref POINT point);
  [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr handle, uint flags);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool GetMonitorInfoW(IntPtr monitor, ref MONITORINFOEXW info);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr handle);
  [DllImport("user32.dll")] public static extern bool IsValidDpiAwarenessContext(IntPtr value);
  [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr handle, int command);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern IntPtr OpenProcess(uint access, bool inheritHandle, uint processId);
  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] public static extern bool QueryFullProcessImageNameW(IntPtr process, uint flags, StringBuilder path, ref uint size);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool CloseHandle(IntPtr handle);
  [DllImport("user32.dll", SetLastError = true)] public static extern int GetDisplayConfigBufferSizes(uint flags, out uint pathCount, out uint modeCount);
  [DllImport("user32.dll", SetLastError = true)] public static extern int QueryDisplayConfig(uint flags, ref uint pathCount, [In, Out] DISPLAYCONFIG_PATH_INFO[] paths, ref uint modeCount, [In, Out] DISPLAYCONFIG_MODE_INFO[] modes, IntPtr topology);
  [DllImport("user32.dll", SetLastError = true)] public static extern int DisplayConfigGetDeviceInfo(ref DISPLAYCONFIG_SOURCE_DEVICE_NAME deviceName);
  [DllImport("user32.dll", SetLastError = true)] public static extern int DisplayConfigGetDeviceInfo(ref DISPLAYCONFIG_TARGET_DEVICE_NAME deviceName);
  [DllImport("shcore.dll")] public static extern int GetDpiForMonitor(IntPtr monitor, int dpiType, out uint dpiX, out uint dpiY);
}
"@
}

function ConvertTo-ThreeDisplayRect {
  param([Parameter(Mandatory = $true)]$Rect)
  [ordered]@{
    left = [int]$Rect.Left
    top = [int]$Rect.Top
    right = [int]$Rect.Right
    bottom = [int]$Rect.Bottom
    width = [int]($Rect.Right - $Rect.Left)
    height = [int]($Rect.Bottom - $Rect.Top)
  }
}

function Invoke-WithThreeDisplayPhysicalDpiContext {
  param([Parameter(Mandatory = $true)][scriptblock]$Action)
  if (-not [SyndocalThreeDisplayNative]::IsValidDpiAwarenessContext([IntPtr](-4))) {
    throw "Fail closed: Windows cannot prove per-monitor-v2 coordinates for this observer thread."
  }
  $previous = [SyndocalThreeDisplayNative]::SetThreadDpiAwarenessContext([IntPtr](-4))
  if ($previous -eq [IntPtr]::Zero) {
    throw "Fail closed: SetThreadDpiAwarenessContext failed; physical coordinates are unprovable."
  }
  try { & $Action } finally { [void][SyndocalThreeDisplayNative]::SetThreadDpiAwarenessContext($previous) }
}

function Get-ThreeDisplayWindowTitle {
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)
  $length = [SyndocalThreeDisplayNative]::GetWindowTextLengthW($Handle)
  if ($length -le 0) { return "" }
  $builder = New-Object Text.StringBuilder ($length + 1)
  [void][SyndocalThreeDisplayNative]::GetWindowTextW($Handle, $builder, $builder.Capacity)
  return $builder.ToString()
}

function Get-ThreeDisplayNativeProcessPath {
  param([Parameter(Mandatory = $true)][uint32]$ProcessId)
  $handle = [SyndocalThreeDisplayNative]::OpenProcess([uint32]0x1000, $false, $ProcessId)
  if ($handle -eq [IntPtr]::Zero) {
    throw "Fail closed: cannot query image path for PID $ProcessId (OpenProcess failed)."
  }
  try {
    [uint32]$capacity = 1024
    while ($true) {
      $buffer = New-Object Text.StringBuilder ([int]$capacity)
      $size = $capacity
      if ([SyndocalThreeDisplayNative]::QueryFullProcessImageNameW($handle, [uint32]0, $buffer, [ref]$size)) {
        $path = $buffer.ToString()
        if ([string]::IsNullOrWhiteSpace($path)) { throw "Fail closed: PID $ProcessId returned an empty image path." }
        return [IO.Path]::GetFullPath($path)
      }
      if ($capacity -ge 32768) { throw "Fail closed: QueryFullProcessImageNameW failed for PID $ProcessId." }
      $capacity = [uint32]($capacity * 2)
    }
  } finally { [void][SyndocalThreeDisplayNative]::CloseHandle($handle) }
}

function Get-SyndocalCandidateProcesses {
  # SEAM: synthetic tests replace all real process enumeration and image-path queries.
  $processes = @(Get-Process -Name syndocal -ErrorAction SilentlyContinue -ErrorVariable errors)
  foreach ($record in @($errors)) {
    if ("$($record.FullyQualifiedErrorId)" -notmatch "^NoProcessFound\b") {
      throw "Fail closed: syndocal process enumeration failed: $($record.Exception.Message)"
    }
  }
  $result = [System.Collections.Generic.List[object]]::new()
  foreach ($process in $processes) {
    $result.Add([pscustomobject]@{ process_id = [uint32]$process.Id; native_image_path = Get-ThreeDisplayNativeProcessPath -ProcessId ([uint32]$process.Id) })
  }
  return @($result)
}

function Get-ExecutableSha256 {
  # SEAM
  param([Parameter(Mandatory = $true)][string]$Path)
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Fail closed: exact executable '$Path' does not exist." }
  return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-ExecutableProductVersion {
  # SEAM
  param([Parameter(Mandatory = $true)][string]$Path)
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Fail closed: exact executable '$Path' does not exist." }
  $value = [string](Get-Item -LiteralPath $Path).VersionInfo.ProductVersion
  if ([string]::IsNullOrWhiteSpace($value)) { throw "Fail closed: ProductVersion is unavailable for '$Path'." }
  return $value
}

function Resolve-ThreeDisplayGitHead {
  # SEAM: read-only git query only.
  param([Parameter(Mandatory = $true)][string]$CheckoutRootPath)
  $answer = & git -C $CheckoutRootPath rev-parse HEAD 2>&1
  if ($LASTEXITCODE -ne 0) { throw "Fail closed: git rev-parse HEAD failed for '$CheckoutRootPath'." }
  $head = ([string]@($answer)[0]).Trim()
  if ($head -notmatch "^[0-9a-fA-F]{40}$") { throw "Fail closed: git HEAD is not a full 40-hex commit ('$head')." }
  return $head.ToLowerInvariant()
}

function Get-ThreeDisplaySourceDeviceName {
  param([Parameter(Mandatory = $true)]$Path)
  $source = [SyndocalThreeDisplayNative+DISPLAYCONFIG_SOURCE_DEVICE_NAME]::new()
  $sourceHeader = [SyndocalThreeDisplayNative+DISPLAYCONFIG_DEVICE_INFO_HEADER]::new()
  $sourceHeader.Type = 1
  $sourceHeader.Size = [uint32][Runtime.InteropServices.Marshal]::SizeOf([type][SyndocalThreeDisplayNative+DISPLAYCONFIG_SOURCE_DEVICE_NAME])
  $sourceHeader.AdapterId = $Path.SourceInfo.AdapterId
  $sourceHeader.Id = $Path.SourceInfo.Id
  $source.Header = $sourceHeader
  if ([SyndocalThreeDisplayNative]::DisplayConfigGetDeviceInfo([ref]$source) -ne 0 -or [string]::IsNullOrWhiteSpace($source.ViewGdiDeviceName)) {
    throw "Fail closed: DisplayConfig source-name query failed for an active display path."
  }
  return $source.ViewGdiDeviceName
}

function Get-ThreeDisplayTargetDeviceName {
  param([Parameter(Mandatory = $true)]$Path)
  $target = [SyndocalThreeDisplayNative+DISPLAYCONFIG_TARGET_DEVICE_NAME]::new()
  $targetHeader = [SyndocalThreeDisplayNative+DISPLAYCONFIG_DEVICE_INFO_HEADER]::new()
  $targetHeader.Type = 2
  $targetHeader.Size = [uint32][Runtime.InteropServices.Marshal]::SizeOf([type][SyndocalThreeDisplayNative+DISPLAYCONFIG_TARGET_DEVICE_NAME])
  $targetHeader.AdapterId = $Path.TargetInfo.AdapterId
  $targetHeader.Id = $Path.TargetInfo.Id
  $target.Header = $targetHeader
  if ([SyndocalThreeDisplayNative]::DisplayConfigGetDeviceInfo([ref]$target) -ne 0 -or [string]::IsNullOrWhiteSpace($target.MonitorDevicePath)) {
    throw "Fail closed: DisplayConfig target-name query failed or returned no stable monitor-device path."
  }
  return $target
}

function Get-ThreeDisplayDisplayConfigPaths {
  [uint32]$pathCount = 0
  [uint32]$modeCount = 0
  $onlyActivePaths = [uint32]2
  if ([SyndocalThreeDisplayNative]::GetDisplayConfigBufferSizes($onlyActivePaths, [ref]$pathCount, [ref]$modeCount) -ne 0 -or $pathCount -eq 0) {
    throw "Fail closed: active DisplayConfig topology is unavailable or empty."
  }
  $paths = New-Object 'SyndocalThreeDisplayNative+DISPLAYCONFIG_PATH_INFO[]' ([int]$pathCount)
  $modes = New-Object 'SyndocalThreeDisplayNative+DISPLAYCONFIG_MODE_INFO[]' ([int]$modeCount)
  $returnedPaths = $pathCount
  $returnedModes = $modeCount
  if ([SyndocalThreeDisplayNative]::QueryDisplayConfig($onlyActivePaths, [ref]$returnedPaths, $paths, [ref]$returnedModes, $modes, [IntPtr]::Zero) -ne 0) {
    throw "Fail closed: QueryDisplayConfig(active paths) failed."
  }
  return @($paths | Select-Object -First ([int]$returnedPaths))
}

function Get-ThreeDisplayMonitorInventory {
  # SEAM: enumerates only currently active monitor paths and binds each Win32
  # HMONITOR/GDI name to exactly one stable DisplayConfig target path.
  $paths = @(Get-ThreeDisplayDisplayConfigPaths)
  $pathByGdiName = @{}
  foreach ($path in $paths) {
    if (-not $path.TargetInfo.TargetAvailable) { throw "Fail closed: an active DisplayConfig target reports unavailable." }
    $gdiName = Get-ThreeDisplaySourceDeviceName -Path $path
    $target = Get-ThreeDisplayTargetDeviceName -Path $path
    $key = $gdiName.ToUpperInvariant()
    if ($pathByGdiName.ContainsKey($key)) { throw "Fail closed: active DisplayConfig has multiple targets for '$gdiName'; monitor binding is ambiguous." }
    $pathByGdiName[$key] = $target
  }
  $monitors = [System.Collections.Generic.List[object]]::new()
  Invoke-WithThreeDisplayPhysicalDpiContext {
    $callback = [SyndocalThreeDisplayMonitorEnumProc]{
      param([IntPtr]$handle, [IntPtr]$hdc, [IntPtr]$rect, [IntPtr]$unused)
      $info = [SyndocalThreeDisplayNative+MONITORINFOEXW]::new()
      $info.Size = [Runtime.InteropServices.Marshal]::SizeOf([type][SyndocalThreeDisplayNative+MONITORINFOEXW])
      if (-not [SyndocalThreeDisplayNative]::GetMonitorInfoW($handle, [ref]$info)) { throw "Fail closed: GetMonitorInfoW failed while enumerating monitors." }
      $gdiName = [string]$info.DeviceName
      $key = $gdiName.ToUpperInvariant()
      if (-not $pathByGdiName.ContainsKey($key)) { throw "Fail closed: Win32 monitor '$gdiName' has no unique active DisplayConfig target path." }
      $target = $pathByGdiName[$key]
      [uint32]$dpiX = 0; [uint32]$dpiY = 0
      if ([SyndocalThreeDisplayNative]::GetDpiForMonitor($handle, 0, [ref]$dpiX, [ref]$dpiY) -ne 0 -or $dpiX -eq 0 -or $dpiX -ne $dpiY) {
        throw "Fail closed: effective DPI is unprovable for monitor '$gdiName'."
      }
      $monitors.Add([pscustomobject]@{
        stable_identity = [string]$target.MonitorDevicePath
        device_name = $gdiName
        friendly_name = [string]$target.MonitorFriendlyDeviceName
        monitor_handle_decimal = [long]$handle.ToInt64()
        connected = $true
        effective_dpi = [int]$dpiX
        physical_bounds = ConvertTo-ThreeDisplayRect -Rect $info.Monitor
        work_area = ConvertTo-ThreeDisplayRect -Rect $info.Work
      })
      return $true
    }
    if (-not [SyndocalThreeDisplayNative]::EnumDisplayMonitors([IntPtr]::Zero, [IntPtr]::Zero, $callback, [IntPtr]::Zero)) {
      throw "Fail closed: EnumDisplayMonitors failed."
    }
  }
  return @($monitors)
}

function Get-ThreeDisplayTopLevelWindows {
  # SEAM: returns every visible top-level window for the proven PID.  The
  # collection is complete before role filtering, so duplicate/wrong output
  # titles cannot be hidden by a first-match selection.
  param([Parameter(Mandatory = $true)][uint32]$OwnerPid)
  $windows = [System.Collections.Generic.List[object]]::new()
  $callback = [SyndocalThreeDisplayEnumProc]{
    param([IntPtr]$handle, [IntPtr]$unused)
    [uint32]$windowOwnerPid = 0
    [void][SyndocalThreeDisplayNative]::GetWindowThreadProcessId($handle, [ref]$windowOwnerPid)
    if ($windowOwnerPid -eq $OwnerPid -and [SyndocalThreeDisplayNative]::IsWindowVisible($handle)) {
      $windows.Add([pscustomobject]@{
        handle_decimal = [long]$handle.ToInt64()
        title = Get-ThreeDisplayWindowTitle -Handle $handle
      })
    }
    return $true
  }
  if (-not [SyndocalThreeDisplayNative]::EnumWindows($callback, [IntPtr]::Zero)) {
    throw "Fail closed: EnumWindows failed."
  }
  return @($windows | Sort-Object handle_decimal)
}

function Get-ThreeDisplayWindowMetrics {
  # SEAM: returns physical coordinates because the call is enclosed in the
  # observer-thread PMv2 context.  Logical dimensions are calculations from
  # the observed window DPI and are retained alongside physical dimensions.
  param([Parameter(Mandatory = $true)][long]$HandleDecimal)
  $handle = [IntPtr]$HandleDecimal
  if (-not [SyndocalThreeDisplayNative]::IsWindow($handle)) { throw "Fail closed: HWND $HandleDecimal disappeared." }
  [uint32]$ownerPid = 0
  [void][SyndocalThreeDisplayNative]::GetWindowThreadProcessId($handle, [ref]$ownerPid)
  $outer = [SyndocalThreeDisplayNative+RECT]::new()
  $client = [SyndocalThreeDisplayNative+RECT]::new()
  $origin = [SyndocalThreeDisplayNative+POINT]::new()
  if (-not [SyndocalThreeDisplayNative]::GetWindowRect($handle, [ref]$outer)) { throw "Fail closed: GetWindowRect failed for HWND $HandleDecimal." }
  if (-not [SyndocalThreeDisplayNative]::GetClientRect($handle, [ref]$client)) { throw "Fail closed: GetClientRect failed for HWND $HandleDecimal." }
  if (-not [SyndocalThreeDisplayNative]::ClientToScreen($handle, [ref]$origin)) { throw "Fail closed: ClientToScreen failed for HWND $HandleDecimal." }
  $monitor = [SyndocalThreeDisplayNative]::MonitorFromWindow($handle, [uint32]2)
  if ($monitor -eq [IntPtr]::Zero) { throw "Fail closed: MonitorFromWindow returned no monitor for HWND $HandleDecimal." }
  $info = [SyndocalThreeDisplayNative+MONITORINFOEXW]::new()
  $info.Size = [Runtime.InteropServices.Marshal]::SizeOf([type][SyndocalThreeDisplayNative+MONITORINFOEXW])
  if (-not [SyndocalThreeDisplayNative]::GetMonitorInfoW($monitor, [ref]$info)) { throw "Fail closed: GetMonitorInfoW failed for HWND $HandleDecimal." }
  $dpi = [int][SyndocalThreeDisplayNative]::GetDpiForWindow($handle)
  if ($dpi -le 0) { throw "Fail closed: GetDpiForWindow returned no DPI for HWND $HandleDecimal." }
  $clientWidth = [int]($client.Right - $client.Left)
  $clientHeight = [int]($client.Bottom - $client.Top)
  [pscustomobject]@{
    handle_decimal = $HandleDecimal
    title = Get-ThreeDisplayWindowTitle -Handle $handle
    owner_pid = [uint32]$ownerPid
    alive = $true
    visible = [bool][SyndocalThreeDisplayNative]::IsWindowVisible($handle)
    responding = (-not [bool][SyndocalThreeDisplayNative]::IsHungAppWindow($handle))
    minimized = [bool][SyndocalThreeDisplayNative]::IsIconic($handle)
    maximized = [bool][SyndocalThreeDisplayNative]::IsZoomed($handle)
    monitor_handle_decimal = [long]$monitor.ToInt64()
    monitor_device_name = [string]$info.DeviceName
    effective_dpi = $dpi
    outer_physical_bounds = ConvertTo-ThreeDisplayRect -Rect $outer
    client_physical_bounds = [ordered]@{
      left = [int]$origin.X; top = [int]$origin.Y
      right = [int]($origin.X + $clientWidth); bottom = [int]($origin.Y + $clientHeight)
      width = $clientWidth; height = $clientHeight
    }
    client_logical_size = [ordered]@{
      width = [Math]::Round(($clientWidth * 96.0) / $dpi, 3)
      height = [Math]::Round(($clientHeight * 96.0) / $dpi, 3)
    }
  }
}

function Invoke-ThreeDisplayWindowMaximize {
  # SEAM: the only mutation seam.  It is called only by -Apply after a fresh,
  # full identity/monitor/output revalidation.  No output window is moved or
  # modified by this harness.
  param([Parameter(Mandatory = $true)][long]$HandleDecimal)
  if (-not [SyndocalThreeDisplayNative]::ShowWindow([IntPtr]$HandleDecimal, 3)) {
    throw "Fail closed: ShowWindow(SW_MAXIMIZE) failed for editor HWND $HandleDecimal."
  }
  return $true
}

function Invoke-ThreeDisplaySampleDelay {
  # SEAM
  param([Parameter(Mandatory = $true)][int]$Milliseconds)
  Start-Sleep -Milliseconds $Milliseconds
  return $Milliseconds
}

function Test-ThreeDisplaySha256Format {
  param([string]$Value)
  return ($Value -match "^[0-9a-fA-F]{64}$")
}

function Test-ThreeDisplayGitHeadFormat {
  param([string]$Value)
  return ($Value -match "^[0-9a-fA-F]{40}$")
}

function Test-ThreeDisplayIdentityFormat {
  param([string]$Value)
  return (-not [string]::IsNullOrWhiteSpace($Value) -and $Value.Length -le 512 -and $Value -notmatch "[\r\n]")
}

function New-ThreeDisplayConfiguration {
  param(
    [Parameter(Mandatory = $true)][bool]$IsApply,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ExecutablePath,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Sha256,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ProductVersion,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$GitHead,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$EditorIdentity,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$LedIdentity,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ProjectorIdentity,
    [Parameter(Mandatory = $true)][UInt64]$LedId,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$LedLabel,
    [Parameter(Mandatory = $true)][UInt64]$ProjectorId,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ProjectorLabel,
    [Parameter(Mandatory = $true)][int]$IntervalMs,
    [Parameter(Mandatory = $true)][int]$Attempts,
    [Parameter(Mandatory = $true)][string]$CheckoutRootPath
  )
  if ($IntervalMs -lt $script:ThreeDisplayMinimumSampleIntervalMs) {
    throw "Fail closed: SampleIntervalMs=$IntervalMs is below $($script:ThreeDisplayMinimumSampleIntervalMs) ms."
  }
  if ($Attempts -lt $script:ThreeDisplayRequiredSamples) {
    throw "Fail closed: MaxSampleAttempts=$Attempts is below required stable sample count $($script:ThreeDisplayRequiredSamples)."
  }
  $defaultExecutablePath = [IO.Path]::GetFullPath((Join-Path $CheckoutRootPath "target\release\syndocal.exe"))
  $effectiveExecutablePath = if ([string]::IsNullOrWhiteSpace($ExecutablePath)) { $defaultExecutablePath } else { [IO.Path]::GetFullPath($ExecutablePath) }
  if ($IsApply -and -not [string]::Equals($effectiveExecutablePath, $defaultExecutablePath, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Fail closed: -Apply ExpectedExecutablePath must be this checkout's exact target\\release\\syndocal.exe path; another checkout is prohibited."
  }
  $values = [ordered]@{
    ExpectedSha256 = $Sha256; ExpectedGitHead = $GitHead
    ExpectedEditorMonitorIdentity = $EditorIdentity; ExpectedLedMonitorIdentity = $LedIdentity
    ExpectedProjectorMonitorIdentity = $ProjectorIdentity; LedOutputLabel = $LedLabel; ProjectorOutputLabel = $ProjectorLabel
  }
  foreach ($entry in $values.GetEnumerator()) {
    if ($entry.Value -match "[\r\n]") { throw "Fail closed: $($entry.Key) contains a line break." }
  }
  if ($Sha256 -ne "" -and -not (Test-ThreeDisplaySha256Format $Sha256)) { throw "Fail closed: ExpectedSha256 must be exactly 64 hexadecimal characters." }
  if ($GitHead -ne "" -and -not (Test-ThreeDisplayGitHeadFormat $GitHead)) { throw "Fail closed: ExpectedGitHead must be exactly 40 hexadecimal characters." }
  foreach ($identity in @($EditorIdentity, $LedIdentity, $ProjectorIdentity)) {
    if ($identity -ne "" -and -not (Test-ThreeDisplayIdentityFormat $identity)) { throw "Fail closed: every monitor identity must be one non-empty stable DisplayConfig device path without line breaks." }
  }
  $fullyConfigured =
    (-not [string]::IsNullOrWhiteSpace($Sha256)) -and
    (-not [string]::IsNullOrWhiteSpace($ProductVersion)) -and
    (-not [string]::IsNullOrWhiteSpace($GitHead)) -and
    (Test-ThreeDisplayIdentityFormat $EditorIdentity) -and
    (Test-ThreeDisplayIdentityFormat $LedIdentity) -and
    (Test-ThreeDisplayIdentityFormat $ProjectorIdentity) -and
    ($LedId -gt 0) -and (-not [string]::IsNullOrWhiteSpace($LedLabel)) -and
    ($ProjectorId -gt 0) -and (-not [string]::IsNullOrWhiteSpace($ProjectorLabel))
  if ($IsApply -and -not $fullyConfigured) {
    throw "Fail closed: -Apply requires exact hash/version/HEAD, all three stable monitor identities, and both explicit live output IDs/labels."
  }
  if ($fullyConfigured) {
    if ($EditorIdentity -eq $LedIdentity -or $EditorIdentity -eq $ProjectorIdentity -or $LedIdentity -eq $ProjectorIdentity) {
      throw "Fail closed: editor, LED, and projector monitor identities must be three distinct values."
    }
    if ($LedId -eq $ProjectorId) { throw "Fail closed: LED and projector output IDs must be distinct." }
    if ($LedLabel -eq $ProjectorLabel) { throw "Fail closed: LED and projector output labels must be distinct." }
  }
  [pscustomobject]@{
    apply = $IsApply; fully_configured = $fullyConfigured; checkout_root = [IO.Path]::GetFullPath($CheckoutRootPath)
    expected_executable_path = $effectiveExecutablePath; expected_sha256 = $Sha256.ToLowerInvariant()
    expected_product_version = $ProductVersion; expected_git_head = $GitHead.ToLowerInvariant()
    sample_interval_ms = $IntervalMs; max_sample_attempts = $Attempts
    roles = @(
      [pscustomobject]@{ role = "editor"; stable_identity = $EditorIdentity; physical_width = 1920; physical_height = 1080; native_window_label = "main"; exact_title = $script:ThreeDisplayMainTitle },
      [pscustomobject]@{ role = "led"; stable_identity = $LedIdentity; physical_width = 1920; physical_height = 1080; native_window_label = "video-output-$LedId"; exact_title = "$($script:ThreeDisplayOutputTitlePrefix)$LedLabel" },
      [pscustomobject]@{ role = "projector"; stable_identity = $ProjectorIdentity; physical_width = 3840; physical_height = 2160; native_window_label = "video-output-$ProjectorId"; exact_title = "$($script:ThreeDisplayOutputTitlePrefix)$ProjectorLabel" }
    )
  }
}

function Test-ThreeDisplayReparseAncestry {
  param([Parameter(Mandatory = $true)][string]$Path)
  $current = Get-Item -LiteralPath $Path -Force
  while ($null -ne $current) {
    if (($current.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Fail closed: evidence ancestry contains reparse point '$($current.FullName)'."
    }
    $parentPath = Split-Path -Parent $current.FullName
    if ([string]::IsNullOrWhiteSpace($parentPath) -or $parentPath -eq $current.FullName) { break }
    $current = Get-Item -LiteralPath $parentPath -Force
  }
}

function New-ThreeDisplayEvidenceDirectory {
  # SEAM: creates exactly one new direct child; no existing child is reused.
  param([Parameter(Mandatory = $true)][string]$RootPath, [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Slug)
  if (-not (Test-Path -LiteralPath $RootPath -PathType Container)) {
    throw "Fail closed: evidence root '$RootPath' must already exist; refusing to create an unverified ancestry."
  }
  $root = [IO.Path]::GetFullPath($RootPath)
  Test-ThreeDisplayReparseAncestry -Path $root
  if ($Slug -ne "" -and $Slug -notmatch "^[a-z0-9][a-z0-9._-]{0,63}$") {
    throw "Fail closed: EvidenceSlug must be 1..64 lowercase [a-z0-9._-] characters and start alphanumeric."
  }
  $safeSlug = if ($Slug -eq "") { "three-display-show" } else { $Slug }
  $childName = "{0}-{1}" -f $safeSlug, [Guid]::NewGuid().ToString("N")
  $child = Join-Path $root $childName
  if (Test-Path -LiteralPath $child) { throw "Fail closed: evidence candidate '$child' already exists; it will not be reused." }
  [void](New-Item -ItemType Directory -Path $child -ErrorAction Stop)
  $created = Get-Item -LiteralPath $child -Force
  if (($created.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Fail closed: created evidence child is a reparse point." }
  if ((Split-Path -Parent $created.FullName) -ne $root) { throw "Fail closed: evidence directory escaped its requested direct parent." }
  return $created.FullName
}

function Write-ThreeDisplayEvidenceJson {
  param(
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
    [Parameter(Mandatory = $true)][string]$FileName,
    [Parameter(Mandatory = $true)]$Value
  )
  if ($FileName -notmatch "^[a-z0-9._-]+\.json$") { throw "Fail closed: unsafe evidence JSON name '$FileName'." }
  $path = Join-Path $EvidenceDirectory $FileName
  if (Test-Path -LiteralPath $path) { throw "Fail closed: evidence file '$path' already exists; overwrite prohibited." }
  $json = $Value | ConvertTo-Json -Depth 16
  [IO.File]::WriteAllText($path, $json + [Environment]::NewLine, (New-Object Text.UTF8Encoding($false)))
  return $path
}

function Write-ThreeDisplayEvidenceSums {
  param([Parameter(Mandatory = $true)][string]$EvidenceDirectory)
  $sumPath = Join-Path $EvidenceDirectory "SHA256SUMS.txt"
  if (Test-Path -LiteralPath $sumPath) { throw "Fail closed: SHA256SUMS.txt already exists; overwrite prohibited." }
  $lines = [System.Collections.Generic.List[string]]::new()
  foreach ($file in @(Get-ChildItem -LiteralPath $EvidenceDirectory -File | Sort-Object Name)) {
    if ($file.Name -eq "SHA256SUMS.txt") { continue }
    $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $lines.Add("$hash  $($file.Name)")
  }
  [IO.File]::WriteAllText($sumPath, (($lines -join [Environment]::NewLine) + [Environment]::NewLine), (New-Object Text.UTF8Encoding($false)))
  return $sumPath
}

function Get-ThreeDisplayExpectedMonitor {
  param([Parameter(Mandatory = $true)]$Role, [Parameter(Mandatory = $true)][object[]]$Inventory)
  if (-not (Test-ThreeDisplayIdentityFormat $Role.stable_identity)) {
    throw "Fail closed: role '$($Role.role)' has no explicit stable monitor identity; resolution-only matching is prohibited."
  }
  $matches = @($Inventory | Where-Object { $_.stable_identity -eq $Role.stable_identity })
  if ($matches.Count -ne 1) {
    throw "Fail closed: role '$($Role.role)' stable monitor identity matched $($matches.Count) currently connected monitor(s); exactly one is required."
  }
  $monitor = $matches[0]
  if (-not [bool]$monitor.connected) { throw "Fail closed: role '$($Role.role)' monitor identity is not currently connected." }
  if ([string]::IsNullOrWhiteSpace([string]$monitor.device_name)) { throw "Fail closed: role '$($Role.role)' monitor has no GDI device binding." }
  if ([int]$monitor.physical_bounds.width -ne [int]$Role.physical_width -or [int]$monitor.physical_bounds.height -ne [int]$Role.physical_height) {
    throw "Fail closed: role '$($Role.role)' monitor '$($Role.stable_identity)' native physical resolution is $($monitor.physical_bounds.width)x$($monitor.physical_bounds.height), expected $($Role.physical_width)x$($Role.physical_height)."
  }
  return $monitor
}

function Get-ThreeDisplayExactCheckoutProcess {
  param([Parameter(Mandatory = $true)]$Configuration)
  $expected = [IO.Path]::GetFullPath($Configuration.expected_executable_path)
  $matches = @(
    Get-SyndocalCandidateProcesses | Where-Object {
      -not [string]::IsNullOrWhiteSpace([string]$_.native_image_path) -and
      [string]::Equals([IO.Path]::GetFullPath([string]$_.native_image_path), $expected, [StringComparison]::OrdinalIgnoreCase)
    }
  )
  if ($matches.Count -ne 1) {
    throw "Fail closed: exact checkout process selection found $($matches.Count) exact syndocal.exe match(es) for '$expected'; exactly one is required."
  }
  return $matches[0]
}

function Get-ThreeDisplayWindowForRole {
  param([Parameter(Mandatory = $true)]$Role, [Parameter(Mandatory = $true)][object[]]$TopLevelWindows)
  $matches = @($TopLevelWindows | Where-Object { [string]::Equals([string]$_.title, [string]$Role.exact_title, [StringComparison]::Ordinal) })
  if ($matches.Count -ne 1) {
    throw "Fail closed: role '$($Role.role)' exact native label/title '$($Role.native_window_label)'/'$($Role.exact_title)' matched $($matches.Count) visible top-level HWND(s); exactly one is required."
  }
  return $matches[0]
}

function Assert-ThreeDisplayWindowState {
  param(
    [Parameter(Mandatory = $true)]$Role,
    [Parameter(Mandatory = $true)]$Window,
    [Parameter(Mandatory = $true)]$Metrics,
    [Parameter(Mandatory = $true)][uint32]$ExpectedPid,
    [Parameter(Mandatory = $true)]$ExpectedMonitor,
    [Parameter(Mandatory = $true)][bool]$RequireEditorMaximized
  )
  if ([uint32]$Metrics.owner_pid -ne $ExpectedPid) { throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) owner PID $($Metrics.owner_pid) differs from exact checkout PID $ExpectedPid." }
  if (-not [bool]$Metrics.alive -or -not [bool]$Metrics.visible) { throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) is missing or invisible." }
  if (-not [bool]$Metrics.responding) { throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) is hung/unresponsive." }
  if ([bool]$Metrics.minimized) { throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) is minimized." }
  if (-not [string]::Equals([string]$Metrics.monitor_device_name, [string]$ExpectedMonitor.device_name, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) is on '$($Metrics.monitor_device_name)', not its explicit monitor identity '$($Role.stable_identity)'; swap/default/first-monitor fallback is rejected."
  }
  if ([long]$Metrics.monitor_handle_decimal -ne [long]$ExpectedMonitor.monitor_handle_decimal) {
    throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) monitor handle does not match its explicit identity binding."
  }
  if ($Role.role -eq "editor" -and $RequireEditorMaximized -and -not [bool]$Metrics.maximized) {
    throw "Fail closed: editor HWND $($Metrics.handle_decimal) is not maximized after placement verification."
  }
  if ($Role.role -ne "editor") {
    if ([int]$Metrics.client_physical_bounds.width -ne [int]$Role.physical_width -or [int]$Metrics.client_physical_bounds.height -ne [int]$Role.physical_height) {
      throw "Fail closed: $($Role.role) live output HWND $($Metrics.handle_decimal) client is $($Metrics.client_physical_bounds.width)x$($Metrics.client_physical_bounds.height) physical pixels, expected $($Role.physical_width)x$($Role.physical_height)."
    }
  }
}

function Get-ThreeDisplayStrictSample {
  # Full sample seam composition.  Every invocation independently re-proves
  # current process path/hash/version/HEAD, monitor identities, title counts,
  # PID ownership, window health, placement, and physical client size.
  param(
    [Parameter(Mandatory = $true)]$Configuration,
    [Parameter(Mandatory = $true)][bool]$RequireEditorMaximized
  )
  if (-not $Configuration.fully_configured) {
    throw "Fail closed: strict three-display acceptance requires explicit hash/version/HEAD, all role monitor identities, and exact LED/projector output IDs and labels."
  }
  $process = Get-ThreeDisplayExactCheckoutProcess -Configuration $Configuration
  $actualHash = Get-ExecutableSha256 -Path $Configuration.expected_executable_path
  if (-not [string]::Equals($actualHash, $Configuration.expected_sha256, [StringComparison]::OrdinalIgnoreCase)) { throw "Fail closed: executable SHA256 mismatch (actual '$actualHash')." }
  $actualVersion = Get-ExecutableProductVersion -Path $Configuration.expected_executable_path
  if (-not [string]::Equals($actualVersion, $Configuration.expected_product_version, [StringComparison]::Ordinal)) { throw "Fail closed: executable ProductVersion mismatch (actual '$actualVersion')." }
  $actualHead = Resolve-ThreeDisplayGitHead -CheckoutRootPath $Configuration.checkout_root
  if (-not [string]::Equals($actualHead, $Configuration.expected_git_head, [StringComparison]::OrdinalIgnoreCase)) { throw "Fail closed: checkout git HEAD mismatch (actual '$actualHead')." }

  $inventory = @(Get-ThreeDisplayMonitorInventory)
  if ($inventory.Count -eq 0) { throw "Fail closed: monitor inventory is empty." }
  $duplicateStableIdentity = @($inventory | Group-Object stable_identity | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.Name) -or $_.Count -ne 1 })
  if ($duplicateStableIdentity.Count -gt 0) { throw "Fail closed: monitor inventory contains blank or duplicate stable identities; identity proof is ambiguous." }
  $expectedMonitors = @{}
  foreach ($role in @($Configuration.roles)) { $expectedMonitors[$role.role] = Get-ThreeDisplayExpectedMonitor -Role $role -Inventory $inventory }

  $topLevelWindows = @(Get-ThreeDisplayTopLevelWindows -OwnerPid ([uint32]$process.process_id))
  $selected = @{}
  foreach ($role in @($Configuration.roles)) { $selected[$role.role] = Get-ThreeDisplayWindowForRole -Role $role -TopLevelWindows $topLevelWindows }
  $handles = @($selected.Values | ForEach-Object { [long]$_.handle_decimal })
  if ((@($handles | Select-Object -Unique)).Count -ne 3) { throw "Fail closed: editor/LED/projector role selection resolved duplicate HWNDs." }
  $knownOutputTitles = @($Configuration.roles | Where-Object { $_.role -ne "editor" } | ForEach-Object { [string]$_.exact_title })
  $unexpectedOutputWindows = @($topLevelWindows | Where-Object { ([string]$_.title).StartsWith($script:ThreeDisplayOutputTitlePrefix, [StringComparison]::Ordinal) -and $knownOutputTitles -notcontains [string]$_.title })
  if ($unexpectedOutputWindows.Count -gt 0) {
    $titles = (@($unexpectedOutputWindows | ForEach-Object { "'$($_.title)'" }) -join ", ")
    throw "Fail closed: unexpected visible native output label/title(s) for the exact checkout PID: $titles."
  }

  $windowEvidence = [System.Collections.Generic.List[object]]::new()
  Invoke-WithThreeDisplayPhysicalDpiContext {
    foreach ($role in @($Configuration.roles)) {
      $metrics = Get-ThreeDisplayWindowMetrics -HandleDecimal ([long]$selected[$role.role].handle_decimal)
      Assert-ThreeDisplayWindowState -Role $role -Window $selected[$role.role] -Metrics $metrics -ExpectedPid ([uint32]$process.process_id) -ExpectedMonitor $expectedMonitors[$role.role] -RequireEditorMaximized $RequireEditorMaximized
      $windowEvidence.Add([pscustomobject]@{
        role = $role.role
        expected_native_window_label = $role.native_window_label
        expected_title = $role.exact_title
        expected_monitor_identity = $role.stable_identity
        expected_native_resolution = ("{0}x{1}" -f @($role.physical_width, $role.physical_height))
        window = $metrics
      })
    }
  }
  [pscustomobject]@{
    observed_at_utc = [DateTime]::UtcNow.ToString("o")
    process = [pscustomobject]@{ process_id = [uint32]$process.process_id; native_image_path = [string]$process.native_image_path; sha256 = $actualHash; product_version = $actualVersion; git_head = $actualHead }
    monitors = @($inventory)
    windows = @($windowEvidence)
  }
}

function Get-ThreeDisplayDryRunObservation {
  param([Parameter(Mandatory = $true)]$Configuration)
  if (-not $Configuration.fully_configured) {
    return [pscustomobject]@{
      verdict = "not-configured"
      accepted = $false
      message = "Read-only discovery completed without a complete identity/provenance/output-role configuration; no acceptance was evaluated."
      inventory = @(Get-ThreeDisplayMonitorInventory)
      sample = $null
      errors = @()
    }
  }
  $inventory = @(Get-ThreeDisplayMonitorInventory)
  try {
    $sample = Get-ThreeDisplayStrictSample -Configuration $Configuration -RequireEditorMaximized $true
    return [pscustomobject]@{
      verdict = "dry-run-would-accept"
      accepted = $false
      message = "Read-only evidence met the configured criteria. This dry-run is not an acceptance or hardware claim."
      inventory = @($sample.monitors)
      sample = $sample
      errors = @()
    }
  } catch {
    return [pscustomobject]@{
      verdict = "dry-run-rejected"
      accepted = $false
      message = "Read-only evidence visibly rejected the configured state before acceptance."
      inventory = @($inventory)
      sample = $null
      errors = @($_.Exception.Message)
    }
  }
}

function Invoke-ThreeDisplayApplyAcceptance {
  param([Parameter(Mandatory = $true)]$Configuration)
  $operation = [ordered]@{ performed = $false; kind = "none"; target_role = $null; prechange_revalidation = $null; reason = "already maximized or no action required" }
  $initial = Get-ThreeDisplayStrictSample -Configuration $Configuration -RequireEditorMaximized $false
  $editor = @($initial.windows | Where-Object { $_.role -eq "editor" })[0]
  if (-not [bool]$editor.window.maximized) {
    # Revalidate immediately before the only reversible action.  In
    # particular, all output placements must still be correct: this harness
    # never masks a swapped/default output by moving it.
    $prechange = Get-ThreeDisplayStrictSample -Configuration $Configuration -RequireEditorMaximized $false
    $currentEditor = @($prechange.windows | Where-Object { $_.role -eq "editor" })[0]
    [void](Invoke-ThreeDisplayWindowMaximize -HandleDecimal ([long]$currentEditor.window.handle_decimal))
    $operation = [ordered]@{
      performed = $true; kind = "maximize-editor"; target_role = "editor"
      prechange_revalidation = $prechange.observed_at_utc
      reason = "explicit -Apply allowed only the already-correct editor HWND to maximize"
    }
  }
  $stable = [System.Collections.Generic.List[object]]::new()
  $consecutive = 0
  for ($attempt = 1; $attempt -le $Configuration.max_sample_attempts; $attempt++) {
    try {
      $sample = Get-ThreeDisplayStrictSample -Configuration $Configuration -RequireEditorMaximized $true
      $consecutive++
      $stable.Add([pscustomobject]@{ attempt = $attempt; stable = $true; consecutive = $consecutive; sample = $sample; error = $null })
      if ($consecutive -ge $script:ThreeDisplayRequiredSamples) {
        return [pscustomobject]@{ accepted = $true; operation = $operation; before = $initial; samples = @($stable); failure = $null }
      }
    } catch {
      $consecutive = 0
      $stable.Add([pscustomobject]@{ attempt = $attempt; stable = $false; consecutive = 0; sample = $null; error = $_.Exception.Message })
    }
    if ($attempt -lt $Configuration.max_sample_attempts) { [void](Invoke-ThreeDisplaySampleDelay -Milliseconds $Configuration.sample_interval_ms) }
  }
  return [pscustomobject]@{ accepted = $false; operation = $operation; before = $initial; samples = @($stable); failure = "Fail closed: no $($script:ThreeDisplayRequiredSamples) consecutive stable three-display samples within $($Configuration.max_sample_attempts) attempts." }
}

function Invoke-ThreeDisplayAcceptance {
  # Main programmatic entry point.  It returns an object for deterministic
  # self-tests; only Invoke-ThreeDisplayAcceptanceMain maps that result to an
  # exit code.  All evidence writes are new-file writes through the helpers.
  param(
    [Parameter(Mandatory = $true)]$Configuration,
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
  )
  $provenance = [ordered]@{
    schema_version = $script:ThreeDisplaySchemaVersion
    tool = "run-syndocal-three-display-show-acceptance.ps1"
    mode = if ($Configuration.apply) { "apply" } else { "dry-run" }
    observed_at_utc = [DateTime]::UtcNow.ToString("o")
    checkout_root = $Configuration.checkout_root
    expected_executable_path = $Configuration.expected_executable_path
    fully_configured = $Configuration.fully_configured
    expectations = [ordered]@{
      sha256 = if ($Configuration.expected_sha256) { $Configuration.expected_sha256 } else { $null }
      product_version = if ($Configuration.expected_product_version) { $Configuration.expected_product_version } else { $null }
      git_head = if ($Configuration.expected_git_head) { $Configuration.expected_git_head } else { $null }
      roles = @($Configuration.roles)
    }
    safety = [ordered]@{
      read_only_default = $true
      creates_or_changes_syndocal_outputs = $false
      launches_or_terminates_processes = $false
      screenshots_taken = $false
      hardware_or_network_access = $false
      output_window_repositioning = $false
    }
  }
  [void](Write-ThreeDisplayEvidenceJson -EvidenceDirectory $EvidenceDirectory -FileName "provenance.json" -Value $provenance)
  $before = $null
  $operation = [ordered]@{ performed = $false; kind = "none"; reason = "dry-run: no mutation path is reachable" }
  $final = $null
  $failure = $null
  try {
    if ($Configuration.apply) {
      $applyResult = Invoke-ThreeDisplayApplyAcceptance -Configuration $Configuration
      $before = $applyResult.before
      $operation = $applyResult.operation
      $final = [ordered]@{
        verdict = if ($applyResult.accepted) { "accepted" } else { "rejected" }
        accepted = [bool]$applyResult.accepted
        required_consecutive_samples = $script:ThreeDisplayRequiredSamples
        samples = @($applyResult.samples)
        native_hardware_claim = $false
      }
      if (-not $applyResult.accepted) { $failure = $applyResult.failure }
    } else {
      $dry = Get-ThreeDisplayDryRunObservation -Configuration $Configuration
      $before = $dry.sample
      $final = [ordered]@{
        verdict = $dry.verdict
        accepted = $false
        message = $dry.message
        errors = @($dry.errors)
        native_hardware_claim = $false
      }
      if ($dry.errors.Count -gt 0) { $failure = @($dry.errors)[0] }
      [void](Write-ThreeDisplayEvidenceJson -EvidenceDirectory $EvidenceDirectory -FileName "monitors.json" -Value ([ordered]@{ inventory = @($dry.inventory); decision = $dry.verdict }))
    }
  } catch {
    $failure = $_.Exception.Message
    $final = [ordered]@{ verdict = "rejected"; accepted = $false; errors = @($failure); native_hardware_claim = $false }
  }
  if ($Configuration.apply) {
    $monitorValue = if ($null -ne $before) { @($before.monitors) } else { @() }
    [void](Write-ThreeDisplayEvidenceJson -EvidenceDirectory $EvidenceDirectory -FileName "monitors.json" -Value ([ordered]@{ inventory = $monitorValue; decision = $final.verdict }))
  }
  [void](Write-ThreeDisplayEvidenceJson -EvidenceDirectory $EvidenceDirectory -FileName "before.json" -Value ([ordered]@{ sample = $before }))
  [void](Write-ThreeDisplayEvidenceJson -EvidenceDirectory $EvidenceDirectory -FileName "operation.json" -Value $operation)
  [void](Write-ThreeDisplayEvidenceJson -EvidenceDirectory $EvidenceDirectory -FileName "final.json" -Value $final)
  if ($failure) { [void](Write-ThreeDisplayEvidenceJson -EvidenceDirectory $EvidenceDirectory -FileName "failure.json" -Value ([ordered]@{ failure = $failure })) }
  [void](Write-ThreeDisplayEvidenceSums -EvidenceDirectory $EvidenceDirectory)
  return [pscustomobject]@{
    evidence_directory = $EvidenceDirectory
    succeeded = ([bool]$final.accepted -or -not $Configuration.apply)
    accepted = [bool]$final.accepted
    verdict = [string]$final.verdict
    failure = $failure
    final = $final
  }
}

function Invoke-ThreeDisplayAcceptanceMain {
  $root = if ([string]::IsNullOrWhiteSpace($EvidenceRootPath)) { Join-Path $script:ThreeDisplayCheckoutRoot "qa\artifacts" } else { $EvidenceRootPath }
  $evidenceDirectory = New-ThreeDisplayEvidenceDirectory -RootPath $root -Slug $EvidenceSlug
  try {
    $configuration = New-ThreeDisplayConfiguration -IsApply ([bool]$Apply) -ExecutablePath $ExpectedExecutablePath -Sha256 $ExpectedSha256 -ProductVersion $ExpectedProductVersion -GitHead $ExpectedGitHead -EditorIdentity $ExpectedEditorMonitorIdentity -LedIdentity $ExpectedLedMonitorIdentity -ProjectorIdentity $ExpectedProjectorMonitorIdentity -LedId $LedOutputId -LedLabel $LedOutputLabel -ProjectorId $ProjectorOutputId -ProjectorLabel $ProjectorOutputLabel -IntervalMs $SampleIntervalMs -Attempts $MaxSampleAttempts -CheckoutRootPath $script:ThreeDisplayCheckoutRoot
    $result = Invoke-ThreeDisplayAcceptance -Configuration $configuration -EvidenceDirectory $evidenceDirectory
    [Console]::Out.WriteLine((@{ evidence_directory = $result.evidence_directory; verdict = $result.verdict; accepted = $result.accepted; native_hardware_claim = $false } | ConvertTo-Json -Compress))
    if ($result.succeeded) { exit 0 }
    [Console]::Error.WriteLine("Three-display acceptance rejected: $($result.failure). Evidence: $evidenceDirectory")
    exit 1
  } catch {
    $message = $_.Exception.Message
    try {
      [void](Write-ThreeDisplayEvidenceJson -EvidenceDirectory $evidenceDirectory -FileName "failure.json" -Value ([ordered]@{ failure = $message }))
      [void](Write-ThreeDisplayEvidenceSums -EvidenceDirectory $evidenceDirectory)
    } catch { }
    [Console]::Error.WriteLine("Three-display harness failed closed: $message. Evidence: $evidenceDirectory")
    exit 1
  }
}

if ($MyInvocation.InvocationName -ne ".") {
  Invoke-ThreeDisplayAcceptanceMain
}
