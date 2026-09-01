# Fail-closed native three-display Syndocal show acceptance harness.
#
# This is an observation-first harness for exactly one proven executable, its
# one "Syndocal" editor window, and exactly two live native Display-output
# windows.  It never creates outputs, changes Syndocal settings, launches or
# terminates a Syndocal process, talks to hardware, takes screenshots, injects
# input, changes focus, or changes Z-order.  Its only network use is read-only
# observation of the explicitly supplied loopback 127.0.0.1 CDP port; it never
# contacts any non-loopback address.  The default is a read-only dry-run.
# Dry-run always writes a clearly labelled observation verdict; it never calls
# a window mutation API and never calls the result an acceptance.  -Apply is
# deliberately narrow: after every identity and placement proof has passed, it
# may maximize the already-correct editor window only.  It never moves an
# output window, so a swapped/default/first-monitor placement is rejected
# rather than repaired into an accidental pass.
#
# Authority modes (-AuthorityMode; StandardRelease is the default):
#   StandardRelease - the historical contract.  Apply accepts only this
#     checkout's exact target\release\syndocal.exe as the candidate process.
#   ShowAsioLocal - the separately licensed, same-host, local-only Show-ASIO
#     artifact route (qa\ASIO_SHOW_LOCAL_ONLY.md).  This mode exists so the
#     combined ASIO + three-display native acceptance can target
#     syndocal-show-asio.exe instead of being structurally impossible.  It is
#     NOT a generic alternate-exe escape:
#       * Before any process or UI mutation, and again immediately before the
#         executable-consuming phase (the maximize/stability reliance), the
#         harness executes THIS checkout's app\scripts\check-show-asio-artifact.mjs
#         with this checkout's pinned Node interpreter, requires exit code 0,
#         requires empty stderr, and parses ONLY its exact single-line PASS
#         contract ("Show-ASIO local artifact PASS: <dir> files=<N>
#         distributionApproved=false"); anything else fails closed.
#       * The canonical artifact directory and syndocal-show-asio.exe path are
#         derived from that verified result alone: they must be inside this
#         checkout, directly under target\show-asio-local, match the exact
#         Syndocal_Show_ASIO_<version>_<commit12>_x64 leaf naming, and the
#         extracted version/commit12 must equal the caller-supplied expected
#         product version/artifact source S; current checkout/evidence HEAD E is
#         checked independently.  Caller path/hash/version/S/E mismatch,
#         wrong checkout, missing/future/legacy manifest, extra or mutated
#         artifact files, reparse-backed ancestry, hard-linked executables,
#         non-current commit/host/source identity, any inherited Git
#         repository/index/object/config authority override, and any installer/updater
#         payload all fail closed (the checker enforces the manifest/tree/
#         host-binding half; this harness enforces derivation, containment,
#         link-count, and expectation cross-checks).
#       * Both verification times, the checker identity (path + SHA-256), the
#         artifact flavor, and the exact derived executable path/hash/version,
#         source S, source branch B, and evidence HEAD E are bound into the run evidence (final.json
#         authority_verifications and provenance.json).
#       * Running the checker spawns one short-lived Node child process with a
#         hard timeout; on timeout only that child is terminated.  No Syndocal
#         process is ever launched or stopped by this harness.
#
# Required release/physical-role contract (all monitor selection is by the
# explicit stable monitor identity, and the executable must be the exact
# expected artifact for artifact source S plus evidence HEAD E and SHA-256; selection is
# NEVER by GDI display number, resolution, primary flag, or first match):
#   editor/operator  stable identity, 1920x1080 at DPI 96
#   LED output       stable identity, 1920x1080 at DPI 144
#   projector output stable identity, 3840x2160 at DPI 144 (Windows 150%)
# The two 1920x1080 roles must therefore still provide distinct identities.
# The current GDI device name is read after stable-identity selection, must be
# nonblank, and is retained in inventory/window evidence only. GDI numbers are
# transient (for example, a physical LED may be renumbered) and are never
# compared with a role-hardcoded value.
# This acceptance scope is deliberately only these three named roles in the
# current five-display topology.  The other two connected displays are not
# selected, bound, or individually asserted here; this harness never claims a
# complete five-display identity acceptance.
# The stable identity is the active DisplayConfig monitor-device path, matched
# back to the exact GDI monitor name from GetMonitorInfoW.  It is reported in
# inventory evidence verbatim; users copy it into the three expected-identity
# parameters.  Blank, duplicate, stale, disconnected, or unresolvable paths
# fail closed.  Nonblank current GDI name, exact resolution, effective DPI,
# and client bounds are checked only AFTER exact identity selection.
#
# Native output roles are proven by the current production contract:
#   label video-output-<output id>
#   live title "Syndocal Output - <output label>"
# plus one exact HWND, visible/responsive/non-minimized state, owner PID equal
# to the proven checkout process, and exact monitor identity.  Exact output-ID
# to HWND evidence must come from the typed result of app command
# get_video_output_window_observation_v1 through an app-owned read-only
# observation provider; this PowerShell file never accepts an
# operator-authored JSON substitute.
# The provider attaches only to an explicitly supplied loopback WebView2 CDP
# port whose listener process descends from the exact checkout PID; it never
# discovers a port, reads an environment variable, or uses a window title to
# select a browser page. Test-pattern titles are not accepted as live output
# roles.
#
# Evidence is always retained under a NEW, unique, non-reparse direct child of
# the evidence root.  No file or directory is overwritten or deleted.
# Artifacts: provenance.json, monitors.json, before.json, operation.json,
# final.json, optional failure.json, and SHA256SUMS.txt written last.
# Screenshots are neither taken nor claimed.  The default evidence root is
# this checkout's gitignored target\qa tree, so evidence written by one run
# can never dirty a later run's exact-artifact clean-checkout gate.
#
# Diagnostics (every thrown message echo of observed titles, every catch
# message stored in JSON, every console error line) pass through one
# centralized Unicode-safe single-line sanitizer capped at 400 UTF-16 code
# units; raw multi-line or oversized text never reaches evidence or console.
#
# Self-test seams are the functions marked SEAM.  The companion self-test
# replaces every UI/process/network/Node-facing seam - including the loopback
# CDP transport, the narrow native window seams around IsWindow, owner-PID
# lookup, title read, and ShowWindow, the Show-ASIO checker invocation, and
# the hard-link probe - with deterministic synthetic values; it performs no
# real UI/process/network mutation and never invokes a real Node runtime.

[CmdletBinding()]
param(
  [switch]$Apply,

  [ValidateSet("StandardRelease", "ShowAsioLocal")]
  [string]$AuthorityMode = "StandardRelease",

  [string]$ExpectedExecutablePath = "",
  [string]$ExpectedSha256 = "",
  [string]$ExpectedProductVersion = "",
  [string]$ExpectedGitHead = "",
  [string]$ExpectedArtifactSourceHead = "",
  [string]$ExpectedArtifactSourceBranch = "",

  [string]$ShowAsioNodeExecutablePath = "",

  [string]$ExpectedEditorMonitorIdentity = "",
  [string]$ExpectedLedMonitorIdentity = "",
  [string]$ExpectedProjectorMonitorIdentity = "",

  [UInt64]$LedOutputId = 0,
  [string]$LedOutputLabel = "",
  [UInt64]$ProjectorOutputId = 0,
  [string]$ProjectorOutputLabel = "",

  [int]$CdpPort = 0,

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
$script:ThreeDisplayStandardReleaseArtifactAuthority = [ordered]@{
  product_version = "1.2.0-alpha.53"
  byte_size = [uint64]62424576
  sha256 = "69E89678FFF38CD50F631BCA2E36D1FAFDB38FC0CB49F6C98254E715CC187178"
  source_branch = "codex/syndocal-v1.2"
  source_head = "27f45d1689f415a9423e431d1bf9bf285c0bd634"
  source_provenance = "post-build StandardRelease artifact identity, source 27f45d1689f415a9423e431d1bf9bf285c0bd634"
}
$script:ThreeDisplayRequiredProductVersion = [string]$script:ThreeDisplayStandardReleaseArtifactAuthority.product_version
$script:ThreeDisplaySwMaximize = 3
$script:ThreeDisplayMaximumDiagnosticLength = 400
$script:ThreeDisplayAuthorityStandardRelease = "StandardRelease"
$script:ThreeDisplayAuthorityShowAsioLocal = "ShowAsioLocal"
# Show-ASIO local-only artifact authority contract (qa\ASIO_SHOW_LOCAL_ONLY.md
# and app\scripts\check-show-asio-artifact.mjs are the authorities; these
# mirrors exist only so this harness can independently reject a checker result
# that drifts from them).
$script:ThreeDisplayShowAsioCheckerRelativePath = "app\scripts\check-show-asio-artifact.mjs"
$script:ThreeDisplayShowAsioManifestFilename = "show-asio-local-manifest.json"
$script:ThreeDisplayShowAsioArtifactFlavor = "windows-show-asio-local-only"
$script:ThreeDisplayShowAsioArtifactRelativeParent = "target\show-asio-local"
$script:ThreeDisplayShowAsioArtifactLeafPrefix = "Syndocal_Show_ASIO_"
$script:ThreeDisplayShowAsioExecutableName = "syndocal-show-asio.exe"
$script:ThreeDisplayShowAsioProcessName = "syndocal-show-asio"
# The checker owns the exact 70-path source identity, including the
# ASIO/PROGRAM/CUE AudioOutput UI execution-time source app\src\uiLocalization.ts.
$script:ThreeDisplayShowAsioSourceIdentityCount = 70
$script:ThreeDisplayShowAsioUiLocalizationSourcePath = "app\src\uiLocalization.ts"
# These environment names can redirect Git's repository, work tree, index,
# object store, refs, ancestry, or configuration authority.  They are rejected
# per process; the operator's environment is never mutated or rewritten.
$script:ThreeDisplayGitAuthorityEnvironmentNames = @(
  "GIT_DIR",
  "GIT_WORK_TREE",
  "GIT_INDEX_FILE",
  "GIT_OBJECT_DIRECTORY",
  "GIT_OBJECT_DIRECTORY_RELATIVE",
  "GIT_ALTERNATE_OBJECT_DIRECTORIES",
  "GIT_COMMON_DIR",
  "GIT_NAMESPACE",
  "GIT_CEILING_DIRECTORIES",
  "GIT_DISCOVERY_ACROSS_FILESYSTEM",
  "GIT_GRAFT_FILE",
  "GIT_SHALLOW_FILE",
  "GIT_REPLACE_REF_BASE",
  "GIT_NO_REPLACE_OBJECTS",
  "GIT_CONFIG",
  "GIT_CONFIG_GLOBAL",
  "GIT_CONFIG_SYSTEM",
  "GIT_CONFIG_NOSYSTEM",
  "GIT_CONFIG_COUNT",
  "GIT_CONFIG_PARAMETERS"
)
$script:ThreeDisplayShowAsioPassLineRegex = '^Show-ASIO local artifact PASS: (.+) files=([1-9][0-9]*) distributionApproved=false$'
$script:ThreeDisplayShowAsioLeafRegex = '^Syndocal_Show_ASIO_([0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?)_([0-9a-f]{12})_x64$'
$script:ThreeDisplayShowAsioCheckerTimeoutMs = 180000
$script:ThreeDisplayRoleContracts = @{
  editor = [pscustomobject]@{ effective_dpi = 96; physical_width = 1920; physical_height = 1080 }
  led = [pscustomobject]@{ effective_dpi = 144; physical_width = 1920; physical_height = 1080 }
  projector = [pscustomobject]@{ effective_dpi = 144; physical_width = 3840; physical_height = 2160 }
}

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
  [StructLayout(LayoutKind.Sequential)]
  public struct SyndocalThreeDisplayFileTime { public uint LowPart; public int HighPart; }
  [StructLayout(LayoutKind.Sequential)]
  public struct BY_HANDLE_FILE_INFORMATION {
    public uint FileAttributes;
    public SyndocalThreeDisplayFileTime CreationTime;
    public SyndocalThreeDisplayFileTime LastAccessTime;
    public SyndocalThreeDisplayFileTime LastWriteTime;
    public uint VolumeSerialNumber;
    public uint FileSizeHigh;
    public uint FileSizeLow;
    public uint NumberOfLinks;
    public uint FileIndexHigh;
    public uint FileIndexLow;
  }
  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] public static extern IntPtr CreateFileW(string fileName, uint desiredAccess, uint shareMode, IntPtr securityAttributes, uint creationDisposition, uint flagsAndAttributes, IntPtr templateFile);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool GetFileInformationByHandle(IntPtr handle, out BY_HANDLE_FILE_INFORMATION information);
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

function ConvertTo-ThreeDisplayOneLineDiagnostic {
  # Centralized diagnostic sanitizer for every title echo, catch message,
  # failure JSON value, and console error line: collapses every Unicode
  # line-break and control character to spaces, trims, truncates to at most
  # 400 UTF-16 code units without splitting a surrogate pair, and never
  # returns an empty result when a non-empty fallback is supplied.
  param(
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value,
    [int]$MaxLength = $script:ThreeDisplayMaximumDiagnosticLength,
    [string]$Fallback = ""
  )
  if ($MaxLength -lt 1) { throw "Fail closed: diagnostic maximum length must be positive." }
  $text = [regex]::Replace($Value, "[\x00-\x1F\x7F\u0085\u2028\u2029]+", " ").Trim()
  if ($text.Length -gt $MaxLength) {
    $keep = $MaxLength
    if ([char]::IsHighSurrogate($text[$keep - 1])) { $keep-- }
    $text = $text.Substring(0, $keep)
  }
  while ($text.Length -gt 0) {
    $last = $text[$text.Length - 1]
    if ([char]::IsHighSurrogate($last)) { $text = $text.Substring(0, $text.Length - 1); continue }
    if ([char]::IsLowSurrogate($last) -and (($text.Length -eq 1) -or -not [char]::IsHighSurrogate($text[$text.Length - 2]))) {
      $text = $text.Substring(0, $text.Length - 1)
      continue
    }
    break
  }
  if ([string]::IsNullOrWhiteSpace($text)) { return $Fallback }
  return $text
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
  # SEAM: narrow title-read seam.  Deterministic self-tests replace this so
  # the real maximize body executes against synthetic HWNDs.
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)
  $length = [SyndocalThreeDisplayNative]::GetWindowTextLengthW($Handle)
  if ($length -le 0) { return "" }
  $builder = New-Object Text.StringBuilder ($length + 1)
  [void][SyndocalThreeDisplayNative]::GetWindowTextW($Handle, $builder, $builder.Capacity)
  return $builder.ToString()
}

function Test-ThreeDisplayNativeIsWindow {
  # SEAM: narrow IsWindow seam used only by Invoke-ThreeDisplayWindowMaximize.
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)
  return [bool][SyndocalThreeDisplayNative]::IsWindow($Handle)
}

function Get-ThreeDisplayNativeOwnerProcessId {
  # SEAM: narrow GetWindowThreadProcessId owner-PID seam used only by
  # Invoke-ThreeDisplayWindowMaximize.
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)
  [uint32]$ownerPid = 0
  [void][SyndocalThreeDisplayNative]::GetWindowThreadProcessId($Handle, [ref]$ownerPid)
  return [uint32]$ownerPid
}

function Invoke-ThreeDisplayNativeShowWindow {
  # SEAM: narrow ShowWindow seam.  The only call site passes exactly
  # SW_MAXIMIZE (3).
  param(
    [Parameter(Mandatory = $true)][IntPtr]$Handle,
    [Parameter(Mandatory = $true)][int]$Command
  )
  return [bool][SyndocalThreeDisplayNative]::ShowWindow($Handle, $Command)
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
  # SEAM: synthetic tests replace all real process enumeration and image-path
  # queries.  The process image name is authority-mode dependent: only
  # "syndocal" (StandardRelease) or exactly "syndocal-show-asio"
  # (ShowAsioLocal) may be requested; the exact executable-path equality
  # filter downstream is the real acceptance boundary.
  param([Parameter(Mandatory = $true)][ValidateSet("syndocal", "syndocal-show-asio")][string]$ProcessName)
  $processes = @(Get-Process -Name $ProcessName -ErrorAction SilentlyContinue -ErrorVariable errors)
  foreach ($record in @($errors)) {
    if ("$($record.FullyQualifiedErrorId)" -notmatch "^NoProcessFound\b") {
      throw "Fail closed: syndocal process enumeration failed: $(ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$record.Exception.Message) -Fallback 'process enumeration failed without a readable single-line diagnostic')"
    }
  }
  $result = [System.Collections.Generic.List[object]]::new()
  foreach ($process in $processes) {
    $result.Add([pscustomobject]@{ process_id = [uint32]$process.Id; native_image_path = Get-ThreeDisplayNativeProcessPath -ProcessId ([uint32]$process.Id) })
  }
  return @($result)
}

function Assert-ThreeDisplayGitEnvironmentSafe {
  # Git authority is bound to the explicit -C checkout path.  Any inherited
  # repository/index/object/config override is rejected before every Git child
  # process; this function never changes the operator's environment.
  param([hashtable]$Environment = $null)
  $entries = if ($null -eq $Environment) { @(Get-ChildItem Env:) } else {
    @($Environment.GetEnumerator() | ForEach-Object { [pscustomobject]@{ Name = [string]$_.Key } })
  }
  $overrides = @(
    $entries | Where-Object {
      $name = ([string]$_.Name).ToUpperInvariant()
      ($script:ThreeDisplayGitAuthorityEnvironmentNames -contains $name) -or $name.StartsWith("GIT_CONFIG_", [StringComparison]::Ordinal)
    } | ForEach-Object { [string]$_.Name } | Sort-Object
  )
  if ($overrides.Count -ne 0) {
    throw "Fail closed: Git commands reject repository/index/object/config authority environment overrides: $($overrides -join ', ')."
  }
  return $true
}

function Get-ExecutableSha256 {
  # SEAM
  param([Parameter(Mandatory = $true)][string]$Path)
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Fail closed: exact executable '$Path' does not exist." }
  return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-ExecutableByteSize {
  # SEAM
  param([Parameter(Mandatory = $true)][string]$Path)
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Fail closed: exact executable '$Path' does not exist." }
  $length = [uint64](Get-Item -LiteralPath $Path -Force).Length
  if ($length -eq 0) { throw "Fail closed: exact executable '$Path' has zero bytes." }
  return $length
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
  [void](Assert-ThreeDisplayGitEnvironmentSafe)
  $answer = & git --no-replace-objects -C $CheckoutRootPath rev-parse HEAD 2>&1
  if ($LASTEXITCODE -ne 0) { throw "Fail closed: git rev-parse HEAD failed for '$CheckoutRootPath'." }
  $head = ([string]@($answer)[0]).Trim()
  if ($head -notmatch "^[0-9a-fA-F]{40}$") { throw "Fail closed: git HEAD is not a full 40-hex commit ('$head')." }
  return $head.ToLowerInvariant()
}

function Resolve-ThreeDisplayGitBranch {
  # SEAM: read-only git query only.
  param([Parameter(Mandatory = $true)][string]$CheckoutRootPath)
  [void](Assert-ThreeDisplayGitEnvironmentSafe)
  $answer = & git --no-replace-objects -C $CheckoutRootPath branch --show-current 2>&1
  if ($LASTEXITCODE -ne 0) { throw "Fail closed: git branch --show-current failed for '$CheckoutRootPath'." }
  $branch = ([string]@($answer)[0]).Trim()
  if ([string]::IsNullOrWhiteSpace($branch) -or $branch -match "[\r\n]") {
    throw "Fail closed: current harness checkout branch is unavailable or detached."
  }
  return $branch
}

function Test-ThreeDisplayGitAncestor {
  # SEAM: read-only ancestry query only.
  param(
    [Parameter(Mandatory = $true)][string]$AncestorHead,
    [Parameter(Mandatory = $true)][string]$DescendantHead,
    [Parameter(Mandatory = $true)][string]$CheckoutRootPath
  )
  if (-not (Test-ThreeDisplayGitHeadFormat $AncestorHead) -or -not (Test-ThreeDisplayGitHeadFormat $DescendantHead)) {
    throw "Fail closed: source and harness Git HEADs must both be full 40-hex commits before ancestry is checked."
  }
  [void](Assert-ThreeDisplayGitEnvironmentSafe)
  & git --no-replace-objects -C $CheckoutRootPath merge-base --is-ancestor $AncestorHead $DescendantHead 2>&1 | Out-Null
  if ($LASTEXITCODE -eq 0) { return $true }
  if ($LASTEXITCODE -eq 1) {
    throw "Fail closed: artifact source HEAD '$AncestorHead' is not an ancestor of current harness HEAD '$DescendantHead'."
  }
  throw "Fail closed: git merge-base --is-ancestor failed for artifact source HEAD '$AncestorHead' and current harness HEAD '$DescendantHead'."
}

function Test-ThreeDisplayCheckoutClean {
  # SEAM: exact artifact acceptance cannot treat an uncommitted working tree
  # as the supplied Git HEAD.
  param([Parameter(Mandatory = $true)][string]$CheckoutRootPath)
  [void](Assert-ThreeDisplayGitEnvironmentSafe)
  $answer = & git --no-replace-objects -C $CheckoutRootPath status --porcelain=v1 2>&1
  if ($LASTEXITCODE -ne 0) { throw "Fail closed: git status --porcelain=v1 failed for '$CheckoutRootPath'." }
  $entries = @($answer | ForEach-Object { ([string]$_).TrimEnd() } | Where-Object { $_ -ne "" })
  if ($entries.Count -ne 0) {
    throw "Fail closed: exact artifact acceptance requires a clean checkout; git status reported $($entries.Count) change(s)."
  }
  return $true
}

function Resolve-ThreeDisplayShowAsioNodeExecutablePath {
  # SEAM: resolves the exact Node interpreter for this checkout's Show-ASIO
  # checker.  Never guesses from PATH: either the explicitly supplied
  # -ShowAsioNodeExecutablePath is used, or the pinned per-user dev toolchain
  # location documented in qa\CODEX_HANDOFF_2026-08-19.md.  Missing or
  # reparse-backed interpreters fail closed.
  param(
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ConfiguredNodeExecutablePath
  )
  $candidate = if (-not [string]::IsNullOrWhiteSpace($ConfiguredNodeExecutablePath)) {
    $ConfiguredNodeExecutablePath
  } else {
    (Join-Path $env:LOCALAPPDATA "SyndocalDev\node-v22.22.1-win-x64\node.exe")
  }
  if ($candidate -match "[\r\n]") { throw "Fail closed: Show-ASIO Node executable path contains a line break." }
  $full = [IO.Path]::GetFullPath($candidate)
  if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
    throw "Fail closed: Show-ASIO checker Node interpreter '$full' does not exist; supply -ShowAsioNodeExecutablePath with this checkout's exact node.exe."
  }
  $item = Get-Item -LiteralPath $full -Force
  if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Fail closed: Show-ASIO checker Node interpreter '$full' is a reparse point."
  }
  return [IO.Path]::GetFullPath($item.FullName)
}

function Invoke-ThreeDisplayShowAsioArtifactVerification {
  # SEAM: the complete Node/checker process boundary.  The real body spawns
  # THIS checkout's app\scripts\check-show-asio-artifact.mjs under THIS
  # checkout's pinned Node as one short-lived child with a hard timeout and
  # captures stdout/stderr/exit verbatim for the strict single-line PASS
  # parser.  It never interprets the result here, never launches or stops any
  # Syndocal process, and on timeout terminates only its own hung child.
  param(
    [Parameter(Mandatory = $true)][string]$CheckoutRootPath,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$NodeExecutablePath,
    [Parameter(Mandatory = $true)][string]$ArtifactSourceHead,
    [Parameter(Mandatory = $true)][string]$EvidenceHead,
    [Parameter(Mandatory = $true)][string]$SourceBranch
  )
  if ($ArtifactSourceHead -notmatch "^[0-9a-fA-F]{40}$") { throw "Fail closed: Show-ASIO artifact source HEAD is not exactly 40 hexadecimal characters." }
  if ($EvidenceHead -notmatch "^[0-9a-fA-F]{40}$") { throw "Fail closed: Show-ASIO evidence HEAD is not exactly 40 hexadecimal characters." }
  if ($SourceBranch -notmatch "^[A-Za-z0-9][A-Za-z0-9._/-]*$") { throw "Fail closed: Show-ASIO source branch is not one exact named Git branch." }
  $checkoutRoot = [IO.Path]::GetFullPath($CheckoutRootPath)
  $checkerPath = Join-Path $checkoutRoot $script:ThreeDisplayShowAsioCheckerRelativePath
  if (-not (Test-Path -LiteralPath $checkerPath -PathType Leaf)) {
    throw "Fail closed: this checkout's Show-ASIO authority checker '$checkerPath' does not exist."
  }
  $checkerItem = Get-Item -LiteralPath $checkerPath -Force
  if (($checkerItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Fail closed: this checkout's Show-ASIO authority checker '$checkerPath' is a reparse point."
  }
  $checkerSha256 = Get-ExecutableSha256 -Path ([IO.Path]::GetFullPath($checkerItem.FullName))
  $nodePath = Resolve-ThreeDisplayShowAsioNodeExecutablePath -ConfiguredNodeExecutablePath $NodeExecutablePath
  $invokedAtUtc = [DateTime]::UtcNow.ToString("o")
  $startInfo = New-Object System.Diagnostics.ProcessStartInfo
  $startInfo.FileName = $nodePath
  $startInfo.Arguments = ('"' + $checkerPath + '" --artifact-source ' + $ArtifactSourceHead.ToLowerInvariant() + ' --evidence-head ' + $EvidenceHead.ToLowerInvariant() + ' --source-branch ' + $SourceBranch)
  $startInfo.WorkingDirectory = $checkoutRoot
  $startInfo.UseShellExecute = $false
  $startInfo.CreateNoWindow = $true
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true
  $process = [System.Diagnostics.Process]::Start($startInfo)
  if ($null -eq $process) { throw "Fail closed: Show-ASIO authority checker child process could not be started." }
  try {
    $standardOutTask = $process.StandardOutput.ReadToEndAsync()
    $standardErrorTask = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit($script:ThreeDisplayShowAsioCheckerTimeoutMs)) {
      try { $process.Kill() } catch { }
      throw "Fail closed: Show-ASIO authority checker exceeded $($script:ThreeDisplayShowAsioCheckerTimeoutMs) ms and was terminated."
    }
    # The parameterless wait guarantees the redirected-stream async readers
    # have drained before their tasks are consumed.
    $process.WaitForExit()
    $exitCode = [int]$process.ExitCode
    $standardOut = [string]$standardOutTask.GetAwaiter().GetResult()
    $standardError = [string]$standardErrorTask.GetAwaiter().GetResult()
  } finally {
    $process.Dispose()
  }
  return [pscustomobject]@{
    exit_code = $exitCode
    standard_out = $standardOut
    standard_error = $standardError
    checker_path = [IO.Path]::GetFullPath($checkerItem.FullName)
    checker_sha256 = $checkerSha256
    node_executable_path = $nodePath
    artifact_source_head = $ArtifactSourceHead.ToLowerInvariant()
    evidence_head = $EvidenceHead.ToLowerInvariant()
    source_branch = $SourceBranch
    checker_arguments = @("--artifact-source", $ArtifactSourceHead.ToLowerInvariant(), "--evidence-head", $EvidenceHead.ToLowerInvariant(), "--source-branch", $SourceBranch)
    invoked_at_utc = $invokedAtUtc
  }
}

function ConvertTo-ThreeDisplayShowAsioVerifiedPassContract {
  # Pure fail-closed parser of the checker result.  Accepts ONLY exit code 0,
  # empty stderr, exactly one non-empty stdout line that matches the exact
  # single-line PASS contract, and a plausible absolute artifact directory;
  # every other shape is rejected before any path derivation happens.
  param([Parameter(Mandatory = $true)]$Verification)
  Assert-ThreeDisplayExactPropertyNames -Value $Verification -Expected @(
    "exit_code", "standard_out", "standard_error", "checker_path", "checker_sha256", "node_executable_path", "artifact_source_head", "evidence_head", "source_branch", "checker_arguments", "invoked_at_utc"
  ) -Subject "Show-ASIO authority verification"
  if (-not ($Verification.exit_code -is [int]) -or [int]$Verification.exit_code -ne 0) {
    throw "Fail closed: Show-ASIO authority checker exited with '$($Verification.exit_code)' instead of 0."
  }
  if (-not [string]::IsNullOrWhiteSpace([string]$Verification.standard_error)) {
 throw "Fail closed: Show-ASIO authority checker wrote unexpected stderr output."
  }
  $standardOut = [string]$Verification.standard_out
  $lines = @($standardOut -split "\r?\n")
  if ($lines.Count -gt 0 -and $lines[$lines.Count - 1] -eq "") { $lines = @($lines | Select-Object -First ($lines.Count - 1)) }
  if ($lines.Count -ne 1) {
    throw "Fail closed: Show-ASIO authority checker stdout is not exactly one line ($($lines.Count) candidate lines)."
  }
  $passLine = [string]$lines[0]
  if ($passLine -cne $passLine.Trim()) {
    throw "Fail closed: Show-ASIO authority PASS line carries leading or trailing whitespace."
  }
  $passMatch = [regex]::Match($passLine, $script:ThreeDisplayShowAsioPassLineRegex)
  if (-not $passMatch.Success -or $passMatch.Index -ne 0 -or $passMatch.Length -ne $passLine.Length) {
    throw "Fail closed: Show-ASIO authority checker line is not the exact single-line PASS contract."
  }
  $artifactDirectoryRaw = $passMatch.Groups[1].Value
  $filesVerified = [int64]::Parse($passMatch.Groups[2].Value, [Globalization.CultureInfo]::InvariantCulture)
  if ([string]::IsNullOrWhiteSpace($artifactDirectoryRaw) -or -not [IO.Path]::IsPathRooted($artifactDirectoryRaw) -or $artifactDirectoryRaw -match '[/\r\n"]') {
    throw "Fail closed: Show-ASIO authority PASS directory is not one plain rooted Windows path."
  }
  return [pscustomobject]@{
    pass_line = $passLine
    artifact_directory_raw = $artifactDirectoryRaw
    files_verified = $filesVerified
  }
}

function Test-ThreeDisplaySingleLinkFile {
  # SEAM: proves via kernel32 that the file has exactly one hard link (no
  # second directory entry aliases the same file data).
  param([Parameter(Mandatory = $true)][string]$Path)
  $GENERIC_NONE = [uint32]0
  $SHARE_ALL = [uint32]7
  $OPEN_EXISTING = [uint32]3
  $FILE_ATTRIBUTE_NORMAL = [uint32]0x80
  $handle = [SyndocalThreeDisplayNative]::CreateFileW($Path, $GENERIC_NONE, $SHARE_ALL, [IntPtr]::Zero, $OPEN_EXISTING, $FILE_ATTRIBUTE_NORMAL, [IntPtr]::Zero)
  if ($handle -eq [IntPtr](-1) -or $handle -eq [IntPtr]::Zero) {
    throw "Fail closed: cannot open '$Path' to prove it has exactly one hard link."
  }
  try {
    $information = New-Object SyndocalThreeDisplayNative+BY_HANDLE_FILE_INFORMATION
    if (-not [SyndocalThreeDisplayNative]::GetFileInformationByHandle($handle, [ref]$information)) {
      throw "Fail closed: GetFileInformationByHandle failed for '$Path'."
    }
    if (($information.FileAttributes -band [uint32][IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "Fail closed: '$Path' carries a reparse-point attribute at open time."
    }
    if ([uint32]$information.NumberOfLinks -ne [uint32]1) {
      throw "Fail closed: '$Path' reports $($information.NumberOfLinks) hard links; an aliased Show-ASIO executable is prohibited."
    }
    return $true
  } finally { [void][SyndocalThreeDisplayNative]::CloseHandle($handle) }
}

function Assert-ThreeDisplayShowAsioInsideCheckout {
  param(
    [Parameter(Mandatory = $true)][string]$CandidatePath,
    [Parameter(Mandatory = $true)][string]$CheckoutRootPath,
    [Parameter(Mandatory = $true)][string]$Subject
  )
  $root = [IO.Path]::GetFullPath($CheckoutRootPath).TrimEnd('\', '/')
  $candidate = [IO.Path]::GetFullPath($CandidatePath)
  if (-not $candidate.StartsWith($root + "\", [StringComparison]::OrdinalIgnoreCase)) {
    throw "Fail closed: $Subject '$candidate' is outside this checkout '$root'; wrong-checkout artifacts are prohibited."
  }
  return $candidate
}

function Get-ThreeDisplayShowAsioAuthorityRecord {
  # Derives the canonical artifact directory/executable exclusively from a
  # verified PASS result, rejects every caller/identity mismatch, and returns
  # one evidence record binding flavor, manifest/checker identity, exact
  # executable identity, and this verification time.
  param(
    [Parameter(Mandatory = $true)]$Configuration,
    [Parameter(Mandatory = $true)]$ParsedPass,
    [Parameter(Mandatory = $true)]$Verification,
    [Parameter(Mandatory = $true)][string]$Phase
  )
  foreach ($expectation in @("expected_sha256", "expected_product_version", "expected_git_head", "artifact_source_head", "artifact_source_branch")) {
    if ([string]::IsNullOrWhiteSpace([string]$Configuration.$expectation)) {
      throw "Fail closed: ShowAsioLocal authority requires exact $expectation before the checker may authorize anything."
    }
  }
  if (-not [string]::Equals([string]$Verification.artifact_source_head, [string]$Configuration.artifact_source_head, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Fail closed: Show-ASIO checker artifact source S differs from the configured artifact source S."
  }
  if (-not [string]::Equals([string]$Verification.evidence_head, [string]$Configuration.expected_git_head, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Fail closed: Show-ASIO checker evidence HEAD E differs from the configured evidence HEAD E."
  }
  if (-not [string]::Equals([string]$Verification.source_branch, [string]$Configuration.artifact_source_branch, [StringComparison]::Ordinal)) {
    throw "Fail closed: Show-ASIO checker source branch B differs from the configured source branch B."
  }
  $expectedCheckerArguments = @(
    "--artifact-source", [string]$Configuration.artifact_source_head.ToLowerInvariant(),
    "--evidence-head", [string]$Configuration.expected_git_head.ToLowerInvariant(),
    "--source-branch", [string]$Configuration.artifact_source_branch
  )
  if (-not ($null -ne $Verification.checker_arguments) -or @($Verification.checker_arguments).Count -ne $expectedCheckerArguments.Count) {
    throw "Fail closed: Show-ASIO checker argv evidence is missing or has the wrong shape."
  }
  for ($argumentIndex = 0; $argumentIndex -lt $expectedCheckerArguments.Count; $argumentIndex++) {
    if ([string]$Verification.checker_arguments[$argumentIndex] -cne $expectedCheckerArguments[$argumentIndex]) {
      throw "Fail closed: Show-ASIO checker argv evidence differs from the exact S/E/B authority invocation."
    }
  }
  $checkoutRoot = [IO.Path]::GetFullPath($Configuration.checkout_root)
  $artifactDirectory = $null
  try {
    $artifactDirectory = Assert-ThreeDisplayShowAsioInsideCheckout -CandidatePath $ParsedPass.artifact_directory_raw -CheckoutRootPath $checkoutRoot -Subject "Show-ASIO artifact directory"
  } catch [System.ArgumentException] {
    throw "Fail closed: Show-ASIO authority PASS directory is not a valid Windows path."
  }
  $expectedParent = [IO.Path]::GetFullPath((Join-Path $checkoutRoot ($script:ThreeDisplayShowAsioArtifactRelativeParent + "\"))).TrimEnd('\')
  $actualParent = (Split-Path -Parent $artifactDirectory)
  if (-not [string]::Equals($actualParent, $expectedParent, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Fail closed: Show-ASIO artifact directory parent '$actualParent' is not the exact authority root '$expectedParent'."
  }
  $leafName = Split-Path -Leaf $artifactDirectory
  $leafMatch = [regex]::Match($leafName, $script:ThreeDisplayShowAsioLeafRegex)
  if (-not $leafMatch.Success -or $leafMatch.Length -ne $leafName.Length) {
    throw "Fail closed: Show-ASIO artifact directory leaf '$leafName' is not the exact versioned authority name."
  }
  $leafVersion = $leafMatch.Groups[1].Value
  $leafCommit12 = $leafMatch.Groups[2].Value
  if (-not [string]::Equals($leafVersion, [string]$Configuration.expected_product_version, [StringComparison]::Ordinal)) {
    throw "Fail closed: Show-ASIO artifact directory binds version '$leafVersion', expected '$($Configuration.expected_product_version)'; caller/version drift is rejected."
  }
  if (-not [string]::Equals($leafCommit12, [string]$Configuration.artifact_source_head.ToLowerInvariant().Substring(0, 12), [StringComparison]::Ordinal)) {
    throw "Fail closed: Show-ASIO artifact directory binds source commit '$($leafCommit12)', expected artifact source S prefix '$($Configuration.artifact_source_head.Substring(0, 12))'; caller/source drift is rejected."
  }
  Test-ThreeDisplayReparseAncestry -Path $artifactDirectory
  $executablePath = Join-Path $artifactDirectory $script:ThreeDisplayShowAsioExecutableName
  if (-not (Test-Path -LiteralPath $executablePath -PathType Leaf)) {
    throw "Fail closed: verified Show-ASIO application executable '$executablePath' does not exist."
  }
  $executableItem = Get-Item -LiteralPath $executablePath -Force
  if (($executableItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Fail closed: verified Show-ASIO application executable '$executablePath' is a reparse point."
  }
  if (-not (Test-ThreeDisplaySingleLinkFile -Path $executablePath)) {
    throw "Fail closed: '$executablePath' did not prove exactly one hard link; an aliased Show-ASIO executable is prohibited."
  }
  $canonicalExecutable = [IO.Path]::GetFullPath($executableItem.FullName)
  $callerExpected = $Configuration.caller_expected_executable_path
  if ($null -ne $callerExpected -and -not [string]::IsNullOrWhiteSpace([string]$callerExpected)) {
    if (-not [string]::Equals([IO.Path]::GetFullPath([string]$callerExpected), $canonicalExecutable, [StringComparison]::OrdinalIgnoreCase)) {
      throw "Fail closed: caller ExpectedExecutablePath '$callerExpected' differs from the checker-derived artifact executable '$canonicalExecutable'."
    }
  }
  $executableSha256 = Get-ExecutableSha256 -Path $canonicalExecutable
  if (-not [string]::Equals($executableSha256, [string]$Configuration.expected_sha256, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Fail closed: Show-ASIO executable SHA-256 mismatch between checks or versus caller expectation (actual '$executableSha256')."
  }
  $executableByteSize = Get-ExecutableByteSize -Path $canonicalExecutable
  $executableVersion = Get-ExecutableProductVersion -Path $canonicalExecutable
  if (-not [string]::Equals($executableVersion, [string]$Configuration.expected_product_version, [StringComparison]::Ordinal)) {
    throw "Fail closed: Show-ASIO executable ProductVersion mismatch (actual '$executableVersion')."
  }
  $checkoutHead = Resolve-ThreeDisplayGitHead -CheckoutRootPath $checkoutRoot
  if (-not [string]::Equals($checkoutHead, [string]$Configuration.expected_git_head, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Fail closed: checkout git HEAD mismatch during Show-ASIO authority verification (actual '$checkoutHead')."
  }
  return [ordered]@{
    phase = $Phase
    authority_mode = $script:ThreeDisplayAuthorityShowAsioLocal
    artifact_flavor = $script:ThreeDisplayShowAsioArtifactFlavor
    manifest_filename = $script:ThreeDisplayShowAsioManifestFilename
    checker_relative_path = $script:ThreeDisplayShowAsioCheckerRelativePath
    checker_path = [string]$Verification.checker_path
    checker_sha256 = [string]$Verification.checker_sha256
    node_executable_path = [string]$Verification.node_executable_path
    pass_line = (ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$ParsedPass.pass_line))
    files_verified = [long]$ParsedPass.files_verified
    distribution_approved = $false
    artifact_directory = $artifactDirectory
    executable_path = $canonicalExecutable
    executable_sha256 = $executableSha256
    executable_byte_size = [uint64]$executableByteSize
     executable_product_version = $executableVersion
     checkout_git_head = $checkoutHead
     artifact_source_head = [string]$Verification.artifact_source_head
     artifact_source_branch = [string]$Verification.source_branch
     evidence_head = [string]$Verification.evidence_head
     checker_arguments = @($Verification.checker_arguments)
     verified_at_utc = [string]$Verification.invoked_at_utc
  }
}

function Invoke-ThreeDisplayShowAsioAuthorityGate {
  # The only ShowAsioLocal authorization boundary.  Runs the real checker
  # seam once per call, derives the executable from the verified result, and
  # pins it into the live configuration so every downstream consumer uses the
  # checker-derived path.  StandardRelease passes through untouched ($null).
  param(
    [Parameter(Mandatory = $true)]$Configuration,
    [Parameter(Mandatory = $true)][ValidateSet("pre-mutation", "pre-executable-use", "dry-run-pre-executable-use")][string]$Phase
  )
  if ([string]$Configuration.authority_mode -ne $script:ThreeDisplayAuthorityShowAsioLocal) { return $null }
  $verification = Invoke-ThreeDisplayShowAsioArtifactVerification `
    -CheckoutRootPath $Configuration.checkout_root `
    -NodeExecutablePath ([string]$Configuration.show_asio_node_executable_path) `
    -ArtifactSourceHead ([string]$Configuration.artifact_source_head) `
    -EvidenceHead ([string]$Configuration.expected_git_head) `
    -SourceBranch ([string]$Configuration.artifact_source_branch)
  $parsedPass = ConvertTo-ThreeDisplayShowAsioVerifiedPassContract -Verification $verification
  $record = Get-ThreeDisplayShowAsioAuthorityRecord -Configuration $Configuration -ParsedPass $parsedPass -Verification $verification -Phase $Phase
  $Configuration.expected_executable_path = [string]$record.executable_path
  $Configuration.expected_byte_size = [uint64]$record.executable_byte_size
  return [pscustomobject]$record
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

function Assert-ThreeDisplayNonblankCurrentGdiName {
  param(
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Name,
    [Parameter(Mandatory = $true)][string]$FailureMessage
  )
  if ([string]::IsNullOrWhiteSpace($Name)) { throw "Fail closed: $FailureMessage" }
  return $Name
}

function Get-ThreeDisplayMonitorInventory {
  # SEAM: enumerates only currently active monitor paths and binds each Win32
  # HMONITOR/GDI name to exactly one stable DisplayConfig target path.  The
  # stable target path is the authority; the current GDI name is retained only
  # as nonblank observation evidence because Windows can renumber it.
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
  $enumFailure = @{}
  Invoke-WithThreeDisplayPhysicalDpiContext {
    $callback = [SyndocalThreeDisplayMonitorEnumProc]{
      param([IntPtr]$handle, [IntPtr]$hdc, [IntPtr]$rect, [IntPtr]$unused)
      try {
        $info = [SyndocalThreeDisplayNative+MONITORINFOEXW]::new()
        $info.Size = [Runtime.InteropServices.Marshal]::SizeOf([type][SyndocalThreeDisplayNative+MONITORINFOEXW])
        if (-not [SyndocalThreeDisplayNative]::GetMonitorInfoW($handle, [ref]$info)) { throw "Fail closed: GetMonitorInfoW failed while enumerating monitors." }
        $gdiName = Assert-ThreeDisplayNonblankCurrentGdiName -Name ([string]$info.DeviceName) -FailureMessage "current GDI device name is blank for an enumerated monitor."
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
      } catch {
        # Never let an exception cross the reverse-PInvoke callback boundary.
        # Capture the failure, stop enumeration, and rethrow only after
        # EnumDisplayMonitors has returned to managed code.
        $enumFailure["failure"] = $_.Exception
        return $false
      }
    }
    $enumerated = [SyndocalThreeDisplayNative]::EnumDisplayMonitors([IntPtr]::Zero, [IntPtr]::Zero, $callback, [IntPtr]::Zero)
    if ($null -ne $enumFailure["failure"]) {
      throw $enumFailure["failure"]
    }
    if (-not $enumerated) {
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
  # modified by this harness.  Every user32 touch below goes through the
  # narrow SEAM wrappers so deterministic self-tests execute this real body
  # and prove each failure plus the exact SW_MAXIMIZE command.
  param(
    [Parameter(Mandatory = $true)][long]$HandleDecimal,
    [Parameter(Mandatory = $true)][uint32]$ExpectedProcessId,
    [Parameter(Mandatory = $true)][string]$ExpectedTitle
  )
  $handle = [IntPtr]$HandleDecimal
  if (-not (Test-ThreeDisplayNativeIsWindow -Handle $handle)) {
    throw "Fail closed: editor HWND $HandleDecimal disappeared before maximize."
  }
  $ownerPid = Get-ThreeDisplayNativeOwnerProcessId -Handle $handle
  if ([uint32]$ownerPid -ne $ExpectedProcessId) {
    throw "Fail closed: editor HWND $HandleDecimal owner PID changed before maximize."
  }
  $title = Get-ThreeDisplayWindowTitle -Handle $handle
  if ([string]$title -cne $ExpectedTitle) {
    throw "Fail closed: editor HWND $HandleDecimal title changed before maximize."
  }
  if (-not (Invoke-ThreeDisplayNativeShowWindow -Handle $handle -Command $script:ThreeDisplaySwMaximize)) {
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
  return (
    -not [string]::IsNullOrWhiteSpace($Value) -and
    $Value.Length -le 512 -and
    $Value -notmatch "[\r\n]" -and
    $Value.StartsWith("\\?\DISPLAY#", [StringComparison]::OrdinalIgnoreCase) -and
    $Value.Contains("#{")
  )
}

function Test-ThreeDisplayProductVersionFormat {
  param([AllowEmptyString()][string]$Value)
  return $Value -match '^\d+\.\d+\.\d+(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$'
}

function New-ThreeDisplayConfiguration {
  param(
    [Parameter(Mandatory = $true)][bool]$IsApply,
    [ValidateSet("StandardRelease", "ShowAsioLocal")][string]$AuthorityMode = $script:ThreeDisplayAuthorityStandardRelease,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ExecutablePath,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$Sha256,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ProductVersion,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$GitHead,
    [AllowEmptyString()][string]$ArtifactSourceHead = "",
    [AllowEmptyString()][string]$ArtifactSourceBranch = "",
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$EditorIdentity,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$LedIdentity,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ProjectorIdentity,
    [Parameter(Mandatory = $true)][UInt64]$LedId,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$LedLabel,
    [Parameter(Mandatory = $true)][UInt64]$ProjectorId,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ProjectorLabel,
    [Parameter(Mandatory = $true)][int]$CdpPort,
    [Parameter(Mandatory = $true)][int]$IntervalMs,
    [Parameter(Mandatory = $true)][int]$Attempts,
    [Parameter(Mandatory = $true)][string]$CheckoutRootPath,
    [AllowEmptyString()][string]$ShowAsioNodeExecutablePath = ""
  )
  if ($IntervalMs -lt $script:ThreeDisplayMinimumSampleIntervalMs) {
    throw "Fail closed: SampleIntervalMs=$IntervalMs is below $($script:ThreeDisplayMinimumSampleIntervalMs) ms."
  }
  if ($Attempts -lt $script:ThreeDisplayRequiredSamples) {
    throw "Fail closed: MaxSampleAttempts=$Attempts is below required stable sample count $($script:ThreeDisplayRequiredSamples)."
  }
  if ($CdpPort -lt 0 -or $CdpPort -gt 65535) {
    throw "Fail closed: CdpPort must be 0 (unconfigured) or an exact TCP port in 1..65535."
  }
  if ($ShowAsioNodeExecutablePath -match "[\r\n]") { throw "Fail closed: ShowAsioNodeExecutablePath contains a line break." }
  $checkoutRoot = [IO.Path]::GetFullPath($CheckoutRootPath)
  $isStandardRelease = [string]::Equals($AuthorityMode, $script:ThreeDisplayAuthorityStandardRelease, [StringComparison]::Ordinal)
  $artifactAuthority = if ($isStandardRelease) { $script:ThreeDisplayStandardReleaseArtifactAuthority } else { $null }
  $effectiveArtifactSourceHead = if ($isStandardRelease) { [string]$artifactAuthority.source_head } else { $ArtifactSourceHead }
  $effectiveArtifactSourceBranch = if ($isStandardRelease) { [string]$artifactAuthority.source_branch } else { $ArtifactSourceBranch }
  $defaultExecutablePath = [IO.Path]::GetFullPath((Join-Path $checkoutRoot "target\release\syndocal.exe"))
  $showAsioArtifactParent = [IO.Path]::GetFullPath((Join-Path $checkoutRoot ($script:ThreeDisplayShowAsioArtifactRelativeParent + "\")))
  $callerExpectedExecutablePath = $null
  if ([string]::Equals($AuthorityMode, $script:ThreeDisplayAuthorityStandardRelease, [StringComparison]::Ordinal)) {
    $effectiveExecutablePath = if ([string]::IsNullOrWhiteSpace($ExecutablePath)) { $defaultExecutablePath } else { [IO.Path]::GetFullPath($ExecutablePath) }
    if ($IsApply -and -not [string]::Equals($effectiveExecutablePath, $defaultExecutablePath, [StringComparison]::OrdinalIgnoreCase)) {
      throw "Fail closed: -Apply StandardRelease ExpectedExecutablePath must be this checkout's exact target\\release\\syndocal.exe path; another checkout is prohibited."
    }
  } else {
    # ShowAsioLocal: the authoritative path is derived only from a verified
    # checker PASS result at gate time.  A caller-supplied path is accepted
    # only as a cross-check expectation and must already name the exact
    # artifact executable shape under this checkout's show-asio-local root.
    $effectiveExecutablePath = if ([string]::IsNullOrWhiteSpace($ExecutablePath)) { "" } else { [IO.Path]::GetFullPath($ExecutablePath) }
    if ($effectiveExecutablePath -ne "") {
      $callerExpectedExecutablePath = $effectiveExecutablePath
      if (-not $callerExpectedExecutablePath.StartsWith($showAsioArtifactParent, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Fail closed: ShowAsioLocal ExpectedExecutablePath must live under this checkout's $script:ThreeDisplayShowAsioArtifactRelativeParent tree; generic alternate executables are prohibited."
      }
      if ((Split-Path -Leaf $callerExpectedExecutablePath) -cne $script:ThreeDisplayShowAsioExecutableName) {
        throw "Fail closed: ShowAsioLocal ExpectedExecutablePath must end in exactly $script:ThreeDisplayShowAsioExecutableName."
      }
      $callerDirectoryLeaf = Split-Path -Leaf (Split-Path -Parent $callerExpectedExecutablePath)
      if (-not $callerDirectoryLeaf.EndsWith("_x64", [StringComparison]::OrdinalIgnoreCase)) {
        throw "Fail closed: ShowAsioLocal ExpectedExecutablePath directory leaf '$callerDirectoryLeaf' must carry the _x64 authority suffix."
      }
    }
  }
  $values = [ordered]@{
    ExpectedSha256 = $Sha256; ExpectedGitHead = $GitHead
    ExpectedArtifactSourceHead = $effectiveArtifactSourceHead; ExpectedArtifactSourceBranch = $effectiveArtifactSourceBranch
    ExpectedEditorMonitorIdentity = $EditorIdentity; ExpectedLedMonitorIdentity = $LedIdentity
    ExpectedProjectorMonitorIdentity = $ProjectorIdentity; LedOutputLabel = $LedLabel; ProjectorOutputLabel = $ProjectorLabel
  }
  foreach ($entry in $values.GetEnumerator()) {
    if ($entry.Value -match "[\r\n]") { throw "Fail closed: $($entry.Key) contains a line break." }
  }
  if ($Sha256 -ne "" -and -not (Test-ThreeDisplaySha256Format $Sha256)) { throw "Fail closed: ExpectedSha256 must be exactly 64 hexadecimal characters." }
  if ($GitHead -ne "" -and -not (Test-ThreeDisplayGitHeadFormat $GitHead)) { throw "Fail closed: ExpectedGitHead must be exactly 40 hexadecimal characters." }
  if ($effectiveArtifactSourceHead -ne "" -and -not (Test-ThreeDisplayGitHeadFormat $effectiveArtifactSourceHead)) { throw "Fail closed: ExpectedArtifactSourceHead must be exactly 40 hexadecimal characters." }
  if ($effectiveArtifactSourceBranch -ne "" -and $effectiveArtifactSourceBranch -notmatch "^[A-Za-z0-9][A-Za-z0-9._/-]*$") { throw "Fail closed: ExpectedArtifactSourceBranch must be one exact named Git branch." }
  if ($ProductVersion -ne "" -and -not (Test-ThreeDisplayProductVersionFormat $ProductVersion)) {
    throw "Fail closed: ExpectedProductVersion must be a canonical SemVer value."
  }
  if ($null -ne $artifactAuthority) {
    if ($ProductVersion -ne "" -and -not [string]::Equals($ProductVersion, [string]$artifactAuthority.product_version, [StringComparison]::Ordinal)) {
      throw "Fail closed: StandardRelease ExpectedProductVersion must be exactly $($artifactAuthority.product_version) for the recorded artifact authority."
    }
    if ($Sha256 -ne "" -and -not [string]::Equals($Sha256, [string]$artifactAuthority.sha256, [StringComparison]::OrdinalIgnoreCase)) {
      throw "Fail closed: StandardRelease ExpectedSha256 must match the exact $($artifactAuthority.product_version) artifact authority recorded at $($artifactAuthority.source_provenance)."
    }
    if (-not [string]::Equals($effectiveArtifactSourceHead, [string]$artifactAuthority.source_head, [StringComparison]::OrdinalIgnoreCase) -or
        -not [string]::Equals($effectiveArtifactSourceBranch, [string]$artifactAuthority.source_branch, [StringComparison]::Ordinal)) {
      throw "Fail closed: StandardRelease artifact source commit/branch must match its exact recorded authority."
    }
  }
  foreach ($identity in @($EditorIdentity, $LedIdentity, $ProjectorIdentity)) {
    if ($identity -ne "" -and -not (Test-ThreeDisplayIdentityFormat $identity)) { throw "Fail closed: every monitor identity must be one raw stable DisplayConfig monitor-device path (\\?\DISPLAY#...) without line breaks." }
  }
  $fullyConfigured =
    (-not [string]::IsNullOrWhiteSpace($Sha256)) -and
    (-not [string]::IsNullOrWhiteSpace($ProductVersion)) -and
    (-not [string]::IsNullOrWhiteSpace($GitHead)) -and
    (-not [string]::IsNullOrWhiteSpace($effectiveArtifactSourceHead)) -and
    (-not [string]::IsNullOrWhiteSpace($effectiveArtifactSourceBranch)) -and
    (Test-ThreeDisplayIdentityFormat $EditorIdentity) -and
    (Test-ThreeDisplayIdentityFormat $LedIdentity) -and
    (Test-ThreeDisplayIdentityFormat $ProjectorIdentity) -and
    ($LedId -gt 0) -and (-not [string]::IsNullOrWhiteSpace($LedLabel)) -and
    ($ProjectorId -gt 0) -and (-not [string]::IsNullOrWhiteSpace($ProjectorLabel)) -and
    ($CdpPort -gt 0)
  if ($IsApply -and -not $fullyConfigured) {
    $authorityRequirement = if ($isStandardRelease) {
      "exact $($artifactAuthority.product_version) hash/version/byte-size with recorded artifact source HEAD ancestry"
    } else {
      "exact checker/manifest-bound hash/version/byte-size with artifact source S, evidence HEAD E, and source branch B"
    }
    throw "Fail closed: -Apply requires $authorityRequirement, all three stable monitor identities, both explicit live output IDs/labels, and one explicit loopback CDP port."
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
    authority_mode = [string]$AuthorityMode
    artifact_authority = $(if ($null -eq $artifactAuthority) { $null } else { [pscustomobject]$artifactAuthority })
    caller_expected_executable_path = $callerExpectedExecutablePath
    show_asio_node_executable_path = $(if ([string]::IsNullOrWhiteSpace($ShowAsioNodeExecutablePath)) { "" } else { [IO.Path]::GetFullPath($ShowAsioNodeExecutablePath) })
    expected_executable_path = $effectiveExecutablePath; expected_sha256 = $Sha256.ToLowerInvariant()
    expected_byte_size = $(if ($null -eq $artifactAuthority) { [uint64]0 } else { [uint64]$artifactAuthority.byte_size })
    expected_product_version = $ProductVersion; expected_git_head = $GitHead.ToLowerInvariant()
    artifact_source_head = $(if ([string]::IsNullOrWhiteSpace($effectiveArtifactSourceHead)) { $null } else { [string]$effectiveArtifactSourceHead.ToLowerInvariant() })
    artifact_source_branch = $(if ([string]::IsNullOrWhiteSpace($effectiveArtifactSourceBranch)) { $null } else { [string]$effectiveArtifactSourceBranch })
    sample_interval_ms = $IntervalMs; max_sample_attempts = $Attempts; cdp_port = $CdpPort
    roles = @(
      [pscustomobject]@{ role = "editor"; output_id = $null; stable_identity = $EditorIdentity; expected_effective_dpi = $script:ThreeDisplayRoleContracts.editor.effective_dpi; physical_width = $script:ThreeDisplayRoleContracts.editor.physical_width; physical_height = $script:ThreeDisplayRoleContracts.editor.physical_height; native_window_label = "main"; exact_title = $script:ThreeDisplayMainTitle },
      [pscustomobject]@{ role = "led"; output_id = $LedId; stable_identity = $LedIdentity; expected_effective_dpi = $script:ThreeDisplayRoleContracts.led.effective_dpi; physical_width = $script:ThreeDisplayRoleContracts.led.physical_width; physical_height = $script:ThreeDisplayRoleContracts.led.physical_height; native_window_label = "video-output-$LedId"; exact_title = "$($script:ThreeDisplayOutputTitlePrefix)$LedLabel" },
      [pscustomobject]@{ role = "projector"; output_id = $ProjectorId; stable_identity = $ProjectorIdentity; expected_effective_dpi = $script:ThreeDisplayRoleContracts.projector.effective_dpi; physical_width = $script:ThreeDisplayRoleContracts.projector.physical_width; physical_height = $script:ThreeDisplayRoleContracts.projector.physical_height; native_window_label = "video-output-$ProjectorId"; exact_title = "$($script:ThreeDisplayOutputTitlePrefix)$ProjectorLabel" }
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
  [void](Assert-ThreeDisplayNonblankCurrentGdiName -Name ([string]$monitor.device_name) -FailureMessage "role '$($Role.role)' stable identity resolved without a nonblank current GDI device name.")
  if ([int]$monitor.effective_dpi -ne [int]$Role.expected_effective_dpi) {
    throw "Fail closed: role '$($Role.role)' monitor '$($Role.stable_identity)' current GDI '$($monitor.device_name)' effective DPI is $($monitor.effective_dpi), expected $($Role.expected_effective_dpi)."
  }
  if ([int]$monitor.physical_bounds.width -ne [int]$Role.physical_width -or [int]$monitor.physical_bounds.height -ne [int]$Role.physical_height) {
    throw "Fail closed: role '$($Role.role)' monitor '$($Role.stable_identity)' native physical resolution is $($monitor.physical_bounds.width)x$($monitor.physical_bounds.height), expected $($Role.physical_width)x$($Role.physical_height)."
  }
  return $monitor
}

function Get-ThreeDisplayExactCheckoutProcess {
  param([Parameter(Mandatory = $true)]$Configuration)
  $expected = [IO.Path]::GetFullPath($Configuration.expected_executable_path)
  $processName = if ([string]$Configuration.authority_mode -eq $script:ThreeDisplayAuthorityShowAsioLocal) { $script:ThreeDisplayShowAsioProcessName } else { "syndocal" }
  $matches = @(
    Get-SyndocalCandidateProcesses -ProcessName $processName | Where-Object {
      -not [string]::IsNullOrWhiteSpace([string]$_.native_image_path) -and
      [string]::Equals([IO.Path]::GetFullPath([string]$_.native_image_path), $expected, [StringComparison]::OrdinalIgnoreCase)
    }
  )
  if ($matches.Count -ne 1) {
    throw "Fail closed: exact checkout process selection found $($matches.Count) exact $processName match(es) for '$expected'; exactly one is required."
  }
  return $matches[0]
}

function Get-ThreeDisplayWindowForRole {
  param([Parameter(Mandatory = $true)]$Role, [Parameter(Mandatory = $true)][object[]]$TopLevelWindows)
  $matches = @($TopLevelWindows | Where-Object { [string]::Equals([string]$_.title, [string]$Role.exact_title, [StringComparison]::Ordinal) })
  if ($matches.Count -ne 1) {
    $expectedTitleEcho = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$Role.exact_title)
    throw "Fail closed: role '$($Role.role)' exact native label/title '$($Role.native_window_label)'/'$expectedTitleEcho' matched $($matches.Count) visible top-level HWND(s); exactly one is required."
  }
  return $matches[0]
}

function Get-ThreeDisplayExactOutputWindowObservation {
  # The transport seam below is the sole process/network-facing boundary. The
  # provider validates its complete result before returning it: no title,
  # arbitrary JSON, file, environment, or browser-state substitute can enter
  # the output ID-to-HWND proof.
  param([Parameter(Mandatory = $true)]$Configuration)
  if ([int]$Configuration.cdp_port -le 0) {
    throw "Fail closed: exact output ID-to-HWND observation requires one explicit loopback CDP port."
  }
  $process = Get-ThreeDisplayExactCheckoutProcess -Configuration $Configuration
  $expectedPid = ConvertTo-ThreeDisplayStrictProcessId -Value $process.process_id -Subject "exact checkout PID"
  $transport = Get-ThreeDisplayCdpTransportObservation -CdpPort ([int]$Configuration.cdp_port) -ExpectedAncestorProcessId $expectedPid
  if ($null -eq $transport) {
    throw "Fail closed: exact output observation CDP transport returned no evidence."
  }
  $listenerPid = ConvertTo-ThreeDisplayStrictProcessId -Value $transport.listener_process_id -Subject "CDP listener PID"
  $ancestorValues = $transport.PSObject.Properties["listener_ancestor_process_ids"]
  if ($null -eq $ancestorValues -or -not ($ancestorValues.Value -is [System.Array]) -or $ancestorValues.Value.Count -eq 0) {
    throw "Fail closed: CDP listener ancestry is missing or not an array."
  }
  $ancestors = @($ancestorValues.Value | ForEach-Object {
    ConvertTo-ThreeDisplayStrictProcessId -Value $_ -Subject "CDP listener ancestor PID"
  })
  if ($ancestors[0] -ne $listenerPid) {
    throw "Fail closed: CDP listener ancestry does not begin with listener PID $listenerPid."
  }
  if ((@($ancestors | Select-Object -Unique)).Count -ne $ancestors.Count) {
    throw "Fail closed: CDP listener ancestry contains a repeated PID."
  }
  if ($ancestors -notcontains $expectedPid) {
    throw "Fail closed: CDP listener PID $listenerPid is not descended from exact checkout PID $expectedPid."
  }
  $pageValues = $transport.PSObject.Properties["pages"]
  if ($null -eq $pageValues -or -not ($pageValues.Value -is [System.Array])) {
    throw "Fail closed: CDP transport pages are missing or not an array."
  }
  $readerPages = [System.Collections.Generic.List[object]]::new()
  foreach ($page in @($pageValues.Value)) {
    Assert-ThreeDisplayExactPropertyNames -Value $page -Expected @(
      "strict_reader_succeeded", "command_result", "failure"
    ) -Subject "CDP strict-reader page result"
    $succeeded = $page.PSObject.Properties["strict_reader_succeeded"].Value
    if (-not ($succeeded -is [bool])) {
      throw "Fail closed: CDP strict-reader page result success flag is not Boolean."
    }
    $failure = $page.PSObject.Properties["failure"].Value
    $commandResult = $page.PSObject.Properties["command_result"].Value
    if ($succeeded) {
      if ($null -ne $failure) {
        throw "Fail closed: successful CDP strict-reader page result also contains a failure."
      }
      $readerPages.Add($page)
    } else {
      if (-not ($failure -is [string]) -or [string]::IsNullOrWhiteSpace($failure) -or $failure -match "[\r\n]") {
        throw "Fail closed: rejected CDP strict-reader page result has no exact single-line failure."
      }
      if ($null -ne $commandResult) {
        throw "Fail closed: rejected CDP strict-reader page result also contains a command result."
      }
    }
  }
  if ($readerPages.Count -ne 1) {
    throw "Fail closed: exact checkout CDP endpoint exposed $($readerPages.Count) self-verified main frontend reader(s); exactly one is required."
  }
  $commandResult = $readerPages[0].PSObject.Properties["command_result"]
  if ($null -eq $commandResult -or $null -eq $commandResult.Value) {
    throw "Fail closed: exact checkout self-verified main frontend reader returned no output observation result."
  }
  return ConvertTo-ThreeDisplayStrictOutputWindowObservation -Value $commandResult.Value
}

function ConvertTo-ThreeDisplayStrictProcessId {
  param([Parameter(Mandatory = $true)]$Value, [Parameter(Mandatory = $true)][string]$Subject)
  if (-not ($Value -is [uint32]) -or [uint32]$Value -eq 0) {
    throw "Fail closed: $Subject must be one nonzero UInt32."
  }
  return [uint32]$Value
}

function Assert-ThreeDisplayExactPropertyNames {
  param(
    [Parameter(Mandatory = $true)]$Value,
    [Parameter(Mandatory = $true)][string[]]$Expected,
    [Parameter(Mandatory = $true)][string]$Subject
  )
  if ($null -eq $Value -or $Value -is [string] -or $Value -is [System.Array]) {
    throw "Fail closed: $Subject must be one typed object."
  }
  $actual = @($Value.PSObject.Properties | ForEach-Object { [string]$_.Name } | Sort-Object)
  $expectedSorted = @($Expected | Sort-Object)
  if ($actual.Count -ne $expectedSorted.Count -or (@(Compare-Object -ReferenceObject $expectedSorted -DifferenceObject $actual).Count -ne 0)) {
    throw "Fail closed: $Subject schema properties are not exactly '$($expectedSorted -join ", ")'."
  }
}

function ConvertTo-ThreeDisplayCanonicalPositiveDecimal {
  param([Parameter(Mandatory = $true)]$Value, [Parameter(Mandatory = $true)][string]$Subject)
  if (-not ($Value -is [string])) {
    throw "Fail closed: $Subject must be a canonical positive decimal string."
  }
  $text = [string]$Value
  if ($text -notmatch "^[1-9][0-9]*$") {
    throw "Fail closed: $Subject must be a canonical positive decimal string."
  }
  try {
    $parsed = [UInt64]::Parse($text, [Globalization.NumberStyles]::None, [Globalization.CultureInfo]::InvariantCulture)
  } catch {
    throw "Fail closed: $Subject exceeds the supported unsigned 64-bit decimal range."
  }
  if ($parsed -eq 0 -or $parsed.ToString([Globalization.CultureInfo]::InvariantCulture) -cne $text) {
    throw "Fail closed: $Subject must be a canonical positive decimal string."
  }
  return [UInt64]$parsed
}

function ConvertTo-ThreeDisplayStrictOutputWindowObservation {
  param([Parameter(Mandatory = $true)]$Value)
  Assert-ThreeDisplayExactPropertyNames -Value $Value -Expected @("schema_version", "source", "outputs") -Subject "app-owned output observation"
  if (-not (($Value.schema_version -is [int]) -or ($Value.schema_version -is [long])) -or [Int64]$Value.schema_version -ne 1) {
    throw "Fail closed: exact output observation must be schema_version 1 from the app-owned-read-only provider."
  }
  if (-not ($Value.source -is [string]) -or [string]$Value.source -cne "app-owned-read-only") {
    throw "Fail closed: exact output observation must be schema_version 1 from the app-owned-read-only provider."
  }
  if (-not ($Value.outputs -is [System.Array])) {
    throw "Fail closed: exact output observation outputs must be an array."
  }
  $seenOutputIds = [System.Collections.Generic.HashSet[UInt64]]::new()
  $outputs = [System.Collections.Generic.List[object]]::new()
  foreach ($status in @($Value.outputs)) {
    Assert-ThreeDisplayExactPropertyNames -Value $status -Expected @("output_id", "label", "live_open", "live_window_label", "native_window_handle_decimal") -Subject "app-owned output observation status"
    $outputId = ConvertTo-ThreeDisplayCanonicalPositiveDecimal -Value $status.output_id -Subject "Display output ID"
    if (-not $seenOutputIds.Add($outputId)) {
      throw "Fail closed: exact output observation contains duplicate Display output ID '$($status.output_id)'."
    }
    foreach ($field in @("label", "live_window_label")) {
      $fieldValue = $status.$field
      if (-not ($fieldValue -is [string]) -or [string]::IsNullOrWhiteSpace($fieldValue) -or $fieldValue -match "[\r\n]") {
        throw "Fail closed: exact output observation $field must be one non-empty single-line string."
      }
    }
    $expectedLiveWindowLabel = "video-output-$($status.output_id)"
    $observedLiveWindowLabelDiagnostic = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$status.live_window_label) -Fallback "<empty live window label>"
    $expectedLiveWindowLabelDiagnostic = ConvertTo-ThreeDisplayOneLineDiagnostic -Value $expectedLiveWindowLabel -Fallback "<invalid expected live window label>"
    if ([string]$status.live_window_label -cne $expectedLiveWindowLabel) {
      throw "Fail closed: exact output observation live_window_label '$observedLiveWindowLabelDiagnostic' must equal '$expectedLiveWindowLabelDiagnostic'."
    }
    if (-not ($status.live_open -is [bool])) {
      throw "Fail closed: exact output observation live_open must be boolean."
    }
    $nativeWindowHandleDecimal = $null
    if ($status.live_open) {
      $null = ConvertTo-ThreeDisplayCanonicalPositiveDecimal -Value $status.native_window_handle_decimal -Subject "Native window HWND"
      $nativeWindowHandleDecimal = [string]$status.native_window_handle_decimal
    } elseif ($null -ne $status.native_window_handle_decimal) {
      throw "Fail closed: closed exact output observation status must report native_window_handle_decimal null."
    }
    $outputs.Add([pscustomobject]@{
      output_id = [string]$status.output_id
      label = [string]$status.label
      live_open = [bool]$status.live_open
      live_window_label = [string]$status.live_window_label
      native_window_handle_decimal = $nativeWindowHandleDecimal
    })
  }
  return [pscustomobject]@{
    schema_version = 1
    source = "app-owned-read-only"
    outputs = @($outputs)
  }
}

function Get-ThreeDisplayCdpPageTargets {
  param([Parameter(Mandatory = $true)][int]$CdpPort)
  try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:$CdpPort/json/list" -TimeoutSec 2 -ErrorAction Stop
  } catch {
    throw "Fail closed: loopback CDP endpoint 127.0.0.1:$CdpPort cannot be queried."
  }
  $pages = [System.Collections.Generic.List[object]]::new()
  foreach ($target in @($response)) {
    if ($null -eq $target -or [string]$target.type -ne "page") { continue }
    if (-not ($target.webSocketDebuggerUrl -is [string])) {
      throw "Fail closed: loopback CDP page target has no WebSocket debugger URL."
    }
    try { $uri = [Uri][string]$target.webSocketDebuggerUrl } catch { throw "Fail closed: loopback CDP page target has an invalid WebSocket debugger URL." }
    if ($uri.Scheme -ne "ws" -or $uri.Host -ne "127.0.0.1" -or $uri.Port -ne $CdpPort) {
      throw "Fail closed: CDP page target WebSocket endpoint is not the requested loopback port."
    }
    $pages.Add($target)
  }
  if ($pages.Count -eq 0) { throw "Fail closed: loopback CDP endpoint exposes no page targets." }
  return @($pages)
}

function Invoke-ThreeDisplayCdpRuntimeEvaluate {
  param(
    [Parameter(Mandatory = $true)][string]$WebSocketDebuggerUrl,
    [Parameter(Mandatory = $true)][string]$Expression
  )
  $socket = [System.Net.WebSockets.ClientWebSocket]::new()
  $cancellation = [Threading.CancellationTokenSource]::new()
  $cancellation.CancelAfter([TimeSpan]::FromSeconds(15))
  try {
    $socket.ConnectAsync([Uri]$WebSocketDebuggerUrl, $cancellation.Token).GetAwaiter().GetResult()
    $request = [ordered]@{
      id = 1
      method = "Runtime.evaluate"
      params = [ordered]@{ expression = $Expression; awaitPromise = $true; returnByValue = $true }
    } | ConvertTo-Json -Depth 8 -Compress
    $payload = [Text.Encoding]::UTF8.GetBytes($request)
    $socket.SendAsync([ArraySegment[byte]]::new($payload), [System.Net.WebSockets.WebSocketMessageType]::Text, $true, $cancellation.Token).GetAwaiter().GetResult()
    $buffer = [byte[]]::new(65536)
    while ($true) {
      $stream = [IO.MemoryStream]::new()
      try {
        do {
          $receive = $socket.ReceiveAsync([ArraySegment[byte]]::new($buffer), $cancellation.Token).GetAwaiter().GetResult()
          if ($receive.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) { throw "CDP target closed during app-owned output observation." }
          if ($receive.Count -gt 0) { $stream.Write($buffer, 0, $receive.Count) }
        } while (-not $receive.EndOfMessage)
        $response = ([Text.Encoding]::UTF8.GetString($stream.ToArray())) | ConvertFrom-Json -ErrorAction Stop
      } finally { $stream.Dispose() }
      $responseId = $response.PSObject.Properties["id"]
      if ($null -eq $responseId -or $responseId.Value -ne 1) { continue }
      $errorPayload = $response.PSObject.Properties["error"]
      if ($null -ne $errorPayload -and $null -ne $errorPayload.Value) { throw "CDP Runtime.evaluate failed: $($errorPayload.Value.message)" }
      $evaluation = $response.PSObject.Properties["result"]
      if ($null -eq $evaluation -or $null -eq $evaluation.Value) { throw "CDP Runtime.evaluate returned no result envelope." }
      $exception = $evaluation.Value.PSObject.Properties["exceptionDetails"]
      if ($null -ne $exception -and $null -ne $exception.Value) { throw "CDP Runtime.evaluate threw: $($exception.Value.text)" }
      $remote = $evaluation.Value.PSObject.Properties["result"]
      $value = if ($null -eq $remote -or $null -eq $remote.Value) { $null } else { $remote.Value.PSObject.Properties["value"] }
      if ($null -eq $value) { throw "CDP Runtime.evaluate returned no serializable value." }
      return $value.Value
    }
  } finally {
    $cancellation.Dispose()
    $socket.Dispose()
  }
}

function Get-ThreeDisplayProcessRecord {
  # SEAM: the narrow process-parent record boundary used by the CDP ancestry
  # proof.  The proof deliberately stops at the exact checkout process, so it
  # never needs to inspect that process's parent or any stale launcher above it.
  param([Parameter(Mandatory = $true)][uint32]$ProcessId)
  return Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction Stop
}

function Get-ThreeDisplayProcessAncestorIds {
  param(
    [Parameter(Mandatory = $true)][uint32]$ProcessId,
    [Parameter(Mandatory = $true)][uint32]$ExpectedAncestorProcessId
  )
  if ($ExpectedAncestorProcessId -eq 0) {
    throw "Fail closed: expected checkout ancestor PID must be one nonzero UInt32."
  }
  $ancestors = [System.Collections.Generic.List[uint32]]::new()
  $seen = [System.Collections.Generic.HashSet[uint32]]::new()
  [uint32]$current = $ProcessId
  for ($depth = 0; $depth -lt 64; $depth++) {
    if ($current -eq 0 -or -not $seen.Add($current)) { throw "Fail closed: CDP listener process ancestry is invalid or cyclic." }
    $ancestors.Add($current)
    if ($current -eq $ExpectedAncestorProcessId) { return @($ancestors) }
    $record = Get-ThreeDisplayProcessRecord -ProcessId $current
    if ($null -eq $record) { throw "Fail closed: CDP listener process $current disappeared during ancestry proof." }
    if ([uint64]$record.ParentProcessId -gt [uint64][uint32]::MaxValue) { throw "Fail closed: CDP listener process ancestry contains an invalid parent PID." }
    [uint32]$parent = [uint32]$record.ParentProcessId
    if ($parent -eq 0) { throw "Fail closed: CDP listener process ancestry terminated before exact checkout PID $ExpectedAncestorProcessId." }
    $current = $parent
  }
  throw "Fail closed: CDP listener process ancestry exceeded 64 levels."
}

function Get-ThreeDisplayCdpTransportObservation {
  # SEAM: this is the complete live-process/CDP transport boundary. It only
  # observes an explicitly supplied loopback port and never changes a process,
  # window, Tauri state, focus, or Z-order.
  param(
    [Parameter(Mandatory = $true)][int]$CdpPort,
    [Parameter(Mandatory = $true)][uint32]$ExpectedAncestorProcessId
  )
  if ($ExpectedAncestorProcessId -eq 0) {
    throw "Fail closed: expected checkout ancestor PID must be one nonzero UInt32."
  }
  $listeners = @(Get-NetTCPConnection -LocalPort $CdpPort -State Listen -ErrorAction Stop)
  if ($listeners.Count -ne 1) { throw "Fail closed: loopback CDP port $CdpPort has $($listeners.Count) listening endpoints; exactly one is required." }
  $listener = $listeners[0]
  if ([string]$listener.LocalAddress -cne "127.0.0.1") { throw "Fail closed: CDP port $CdpPort is not bound exactly to 127.0.0.1." }
  $listenerPid = ConvertTo-ThreeDisplayStrictProcessId -Value ([uint32]$listener.OwningProcess) -Subject "CDP listener PID"
  $ancestors = Get-ThreeDisplayProcessAncestorIds -ProcessId $listenerPid -ExpectedAncestorProcessId $ExpectedAncestorProcessId
  $commandExpression = @'
(async () => {
  const read = window.__syndocalReadVideoOutputWindowObservationV1;
  if (typeof read !== 'function') throw new Error('strict frontend output observation reader is unavailable');
  return await read();
})()
'@
  $pages = [System.Collections.Generic.List[object]]::new()
  foreach ($page in @(Get-ThreeDisplayCdpPageTargets -CdpPort $CdpPort)) {
    try {
      $result = Invoke-ThreeDisplayCdpRuntimeEvaluate -WebSocketDebuggerUrl ([string]$page.webSocketDebuggerUrl) -Expression $commandExpression
      $pages.Add([pscustomobject]@{
        strict_reader_succeeded = $true
        command_result = $result
        failure = $null
      })
    } catch {
      $failure = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$_.Exception.Message) -Fallback "strict frontend output observation reader rejected this page"
      $pages.Add([pscustomobject]@{
        strict_reader_succeeded = $false
        command_result = $null
        failure = $failure
      })
    }
  }
  $successfulPages = @($pages | Where-Object { $_.strict_reader_succeeded -eq $true })
  if ($successfulPages.Count -ne 1) {
    throw "Fail closed: exact checkout CDP endpoint exposed $($successfulPages.Count) self-verified main frontend reader(s); exactly one is required."
  }
  return [pscustomobject]@{
    listener_process_id = $listenerPid
    listener_ancestor_process_ids = @($ancestors)
    pages = @($pages)
  }
}

function Assert-ThreeDisplayExactOutputWindowObservation {
  param(
    [Parameter(Mandatory = $true)]$Configuration,
    [Parameter(Mandatory = $true)]$Observation,
    [Parameter(Mandatory = $true)]$Selected
  )
  $Observation = ConvertTo-ThreeDisplayStrictOutputWindowObservation -Value $Observation
  $statuses = @($Observation.outputs)
  if ($statuses.Count -eq 0) {
    throw "Fail closed: exact output observation contains no output statuses."
  }
  $liveStatuses = @($statuses | Where-Object { [bool]$_.live_open })
  if ($liveStatuses.Count -ne 2) {
    throw "Fail closed: exact output observation reports $($liveStatuses.Count) live Display outputs; exactly LED and projector are required."
  }
  foreach ($role in @($Configuration.roles | Where-Object { $_.role -ne "editor" })) {
    $matches = @($statuses | Where-Object {
      (ConvertTo-ThreeDisplayCanonicalPositiveDecimal -Value $_.output_id -Subject "Display output ID") -eq [UInt64]$role.output_id
    })
    if ($matches.Count -ne 1) {
      throw "Fail closed: exact output observation matched $($matches.Count) status record(s) for $($role.native_window_label); exactly one is required."
    }
    $status = $matches[0]
    if (-not [bool]$status.live_open) {
      throw "Fail closed: exact output observation reports $($role.native_window_label) closed."
    }
    $expectedLabel = $role.exact_title.Substring($script:ThreeDisplayOutputTitlePrefix.Length)
    $observedLabelDiagnostic = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$status.label) -Fallback "<empty output label>"
    $expectedLabelDiagnostic = ConvertTo-ThreeDisplayOneLineDiagnostic -Value $expectedLabel -Fallback "<invalid expected output label>"
    $observedLiveWindowLabelDiagnostic = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$status.live_window_label) -Fallback "<empty live window label>"
    $expectedLiveWindowLabelDiagnostic = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$role.native_window_label) -Fallback "<invalid expected live window label>"
    if (-not [string]::Equals([string]$status.label, $expectedLabel, [StringComparison]::Ordinal)) {
      throw "Fail closed: exact output observation label '$observedLabelDiagnostic' for $expectedLiveWindowLabelDiagnostic differs from expected '$expectedLabelDiagnostic'."
    }
    if (-not [string]::Equals([string]$status.live_window_label, [string]$role.native_window_label, [StringComparison]::Ordinal)) {
      throw "Fail closed: exact output observation live window label '$observedLiveWindowLabelDiagnostic' differs from expected '$expectedLiveWindowLabelDiagnostic'."
    }
    $observedHandle = ConvertTo-ThreeDisplayCanonicalPositiveDecimal -Value $status.native_window_handle_decimal -Subject "Native window HWND"
    if ([long]$Selected[$role.role].handle_decimal -le 0) {
      throw "Fail closed: native title-selected HWND $($Selected[$role.role].handle_decimal) is not positive."
    }
    $selectedHandle = [UInt64][long]$Selected[$role.role].handle_decimal
    if ($observedHandle -ne $selectedHandle) {
      throw "Fail closed: exact output observation HWND $($status.native_window_handle_decimal) for $($role.native_window_label) differs from the native title-selected HWND $($Selected[$role.role].handle_decimal)."
    }
  }
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
  if ([long]$Metrics.handle_decimal -ne [long]$Window.handle_decimal) {
    throw "Fail closed: role '$($Role.role)' fresh metrics HWND $($Metrics.handle_decimal) differs from title-selected HWND $($Window.handle_decimal); title/handle selection changed after the census."
  }
  if (-not [string]::Equals([string]$Metrics.title, [string]$Role.exact_title, [StringComparison]::Ordinal)) {
    $observedTitleDiagnostic = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$Metrics.title) -Fallback "<empty native window title>"
    $expectedTitleDiagnostic = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$Role.exact_title) -Fallback "<invalid expected native window title>"
    throw "Fail closed: role '$($Role.role)' fresh metrics title '$observedTitleDiagnostic' differs from exact title '$expectedTitleDiagnostic'; title/handle selection changed after the census."
  }
  if ([uint32]$Metrics.owner_pid -ne $ExpectedPid) { throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) owner PID $($Metrics.owner_pid) differs from exact checkout PID $ExpectedPid." }
  if (-not [bool]$Metrics.alive -or -not [bool]$Metrics.visible) { throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) is missing or invisible." }
  if (-not [bool]$Metrics.responding) { throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) is hung/unresponsive." }
  if ([bool]$Metrics.minimized) { throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) is minimized." }
  [void](Assert-ThreeDisplayNonblankCurrentGdiName -Name ([string]$Metrics.monitor_device_name) -FailureMessage "role '$($Role.role)' HWND $($Metrics.handle_decimal) has no nonblank current GDI device name for its explicit monitor identity '$($Role.stable_identity)'.")
  if (-not [string]::Equals([string]$Metrics.monitor_device_name, [string]$ExpectedMonitor.device_name, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) current GDI device '$($Metrics.monitor_device_name)' does not match the current GDI device '$($ExpectedMonitor.device_name)' recorded for its explicit monitor identity '$($Role.stable_identity)'; swap/default/first-monitor fallback is rejected."
  }
  if ([long]$Metrics.monitor_handle_decimal -ne [long]$ExpectedMonitor.monitor_handle_decimal) {
    throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) monitor handle does not match its explicit identity binding."
  }
  if ([int]$Metrics.effective_dpi -ne [int]$ExpectedMonitor.effective_dpi) {
    throw "Fail closed: role '$($Role.role)' HWND $($Metrics.handle_decimal) effective DPI is $($Metrics.effective_dpi), expected monitor DPI $($ExpectedMonitor.effective_dpi)."
  }
  if ($Role.role -eq "editor" -and $RequireEditorMaximized -and -not [bool]$Metrics.maximized) {
    throw "Fail closed: editor HWND $($Metrics.handle_decimal) is not maximized after placement verification."
  }
  if ($Role.role -ne "editor") {
    if ([int]$Metrics.client_physical_bounds.width -ne [int]$Role.physical_width -or [int]$Metrics.client_physical_bounds.height -ne [int]$Role.physical_height) {
      throw "Fail closed: $($Role.role) live output HWND $($Metrics.handle_decimal) client is $($Metrics.client_physical_bounds.width)x$($Metrics.client_physical_bounds.height) physical pixels, expected $($Role.physical_width)x$($Role.physical_height)."
    }
    $client = $Metrics.client_physical_bounds
    $monitorBounds = $ExpectedMonitor.physical_bounds
    if (
      [int]$client.left -ne [int]$monitorBounds.left -or
      [int]$client.top -ne [int]$monitorBounds.top -or
      [int]$client.right -ne [int]$monitorBounds.right -or
      [int]$client.bottom -ne [int]$monitorBounds.bottom
    ) {
      throw "Fail closed: $($Role.role) live output HWND $($Metrics.handle_decimal) client physical bounds $($client.left),$($client.top),$($client.right),$($client.bottom) do not exactly fill its explicit monitor bounds $($monitorBounds.left),$($monitorBounds.top),$($monitorBounds.right),$($monitorBounds.bottom)."
    }
  }
}

function Get-ThreeDisplayStrictSample {
  # Full sample seam composition.  Every invocation independently re-proves
  # current process path/hash/version/evidence HEAD/branch, artifact-source S
  # ancestry, monitor identities, title counts, PID ownership,
  # window health, placement, and physical client size.
  param(
    [Parameter(Mandatory = $true)]$Configuration,
    [Parameter(Mandatory = $true)][bool]$RequireEditorMaximized
  )
  if (-not $Configuration.fully_configured) {
    throw "Fail closed: strict three-display acceptance requires explicit hash/version/HEAD, all role monitor identities, and exact LED/projector output IDs and labels."
  }
  if ([string]$Configuration.authority_mode -eq $script:ThreeDisplayAuthorityShowAsioLocal) {
    # Defense in depth: even if some future code path bypassed the authority
    # gate, a strict sample in ShowAsioLocal mode refuses to consume anything
    # that is not exactly the checker-derived artifact executable shape.
    $showExecutable = [string]$Configuration.expected_executable_path
    if ([string]::IsNullOrWhiteSpace($showExecutable)) {
      throw "Fail closed: ShowAsioLocal strict sample ran before the authority gate resolved the artifact executable."
    }
    $canonicalShowExecutable = Assert-ThreeDisplayShowAsioInsideCheckout -CandidatePath $showExecutable -CheckoutRootPath $Configuration.checkout_root -Subject "Show-ASIO executable"
    $expectedShowRoot = [IO.Path]::GetFullPath((Join-Path $Configuration.checkout_root ($script:ThreeDisplayShowAsioArtifactRelativeParent + "\"))).TrimEnd('\', '/')
    $expectedShowDirectory = [IO.Path]::GetFullPath((Join-Path $expectedShowRoot ("Syndocal_Show_ASIO_{0}_{1}_x64" -f @(
      [string]$Configuration.expected_product_version,
      ([string]$Configuration.artifact_source_head).ToLowerInvariant().Substring(0, 12)
    ))))
    $actualShowDirectory = [IO.Path]::GetFullPath((Split-Path -Parent $canonicalShowExecutable))
    $actualShowParent = [IO.Path]::GetFullPath((Split-Path -Parent $actualShowDirectory))
    if (-not [string]::Equals($actualShowParent, $expectedShowRoot, [StringComparison]::OrdinalIgnoreCase) -or
        -not [string]::Equals($actualShowDirectory, $expectedShowDirectory, [StringComparison]::OrdinalIgnoreCase) -or
        (Split-Path -Leaf $canonicalShowExecutable) -cne $script:ThreeDisplayShowAsioExecutableName) {
      throw "Fail closed: ShowAsioLocal strict sample refuses generic alternate executable '$canonicalShowExecutable'."
    }
  }
  $process = Get-ThreeDisplayExactCheckoutProcess -Configuration $Configuration
  $actualHash = Get-ExecutableSha256 -Path $Configuration.expected_executable_path
  if (-not [string]::Equals($actualHash, $Configuration.expected_sha256, [StringComparison]::OrdinalIgnoreCase)) { throw "Fail closed: executable SHA256 mismatch (actual '$actualHash')." }
  $actualByteSize = Get-ExecutableByteSize -Path $Configuration.expected_executable_path
  if ([uint64]$Configuration.expected_byte_size -le 0) { throw "Fail closed: exact executable byte-size authority was not established before strict sampling." }
  if ([uint64]$actualByteSize -ne [uint64]$Configuration.expected_byte_size) { throw "Fail closed: executable byte size mismatch (actual '$actualByteSize', expected '$($Configuration.expected_byte_size)')." }
  $actualVersion = Get-ExecutableProductVersion -Path $Configuration.expected_executable_path
  if (-not [string]::Equals($actualVersion, $Configuration.expected_product_version, [StringComparison]::Ordinal)) { throw "Fail closed: executable ProductVersion mismatch (actual '$actualVersion')." }
  $currentBranch = $null
  $artifactSourceHead = if ([string]::IsNullOrWhiteSpace([string]$Configuration.artifact_source_head)) { $null } else { [string]$Configuration.artifact_source_head }
  $artifactSourceBranch = if ([string]::IsNullOrWhiteSpace([string]$Configuration.artifact_source_branch)) { $null } else { [string]$Configuration.artifact_source_branch }
  if ($null -ne $artifactSourceBranch) {
    $currentBranch = Resolve-ThreeDisplayGitBranch -CheckoutRootPath $Configuration.checkout_root
    if (-not [string]::Equals($currentBranch, $artifactSourceBranch, [StringComparison]::Ordinal)) {
      $branchAuthority = if ([string]::Equals([string]$Configuration.authority_mode, $script:ThreeDisplayAuthorityStandardRelease, [StringComparison]::Ordinal)) { "required StandardRelease source branch" } else { "required artifact source branch B" }
      throw "Fail closed: current harness checkout branch '$currentBranch' differs from $branchAuthority '$artifactSourceBranch'."
    }
  }
  $actualHead = Resolve-ThreeDisplayGitHead -CheckoutRootPath $Configuration.checkout_root
  if (-not [string]::Equals($actualHead, $Configuration.expected_git_head, [StringComparison]::OrdinalIgnoreCase)) { throw "Fail closed: checkout git HEAD mismatch (actual '$actualHead')." }
  $checkoutClean = Test-ThreeDisplayCheckoutClean -CheckoutRootPath $Configuration.checkout_root
  if (-not [bool]$checkoutClean) { throw "Fail closed: checkout clean-state proof returned false." }
  if ($null -ne $artifactSourceHead) {
    [void](Test-ThreeDisplayGitAncestor -AncestorHead $artifactSourceHead -DescendantHead $actualHead -CheckoutRootPath $Configuration.checkout_root)
  }

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
    $titles = @($unexpectedOutputWindows | ForEach-Object {
      "'{0}'" -f (ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$_.title))
    })
    throw "Fail closed: unexpected visible native output label/title(s) for the exact checkout PID: $($titles -join ', ')."
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
  # Re-read each title/HWND through Get-ThreeDisplayWindowMetrics and reject
  # any post-census replacement before consuming the app-owned output-ID to
  # HWND observation.  This closes the title/handle TOCTOU between selection
  # and the engine/app evidence boundary.
  $outputObservation = Get-ThreeDisplayExactOutputWindowObservation -Configuration $Configuration
  Assert-ThreeDisplayExactOutputWindowObservation -Configuration $Configuration -Observation $outputObservation -Selected $selected
  [pscustomobject]@{
    observed_at_utc = [DateTime]::UtcNow.ToString("o")
    process = [pscustomobject]@{
      process_id = [uint32]$process.process_id
      native_image_path = [string]$process.native_image_path
      sha256 = $actualHash
      byte_size = [uint64]$actualByteSize
      product_version = $actualVersion
      git_head = $actualHead
      current_harness_head = $actualHead
      current_harness_branch = $currentBranch
      checkout_clean = [bool]$checkoutClean
      artifact_source_head = $artifactSourceHead
      artifact_source_branch = $artifactSourceBranch
    }
    monitors = @($inventory)
    output_window_observation = $outputObservation
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
      authority_verifications = @()
    }
  }
  $inventory = @(Get-ThreeDisplayMonitorInventory)
  try {
    # A fully configured ShowAsioLocal dry-run consumes the artifact identity,
    # so it too must pass the authority gate before any executable use.
    $authorityVerifications = [System.Collections.Generic.List[object]]::new()
    $dryRunAuthority = Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $Configuration -Phase "dry-run-pre-executable-use"
    if ($null -ne $dryRunAuthority) { $authorityVerifications.Add($dryRunAuthority) }
    $sample = Get-ThreeDisplayStrictSample -Configuration $Configuration -RequireEditorMaximized $true
    return [pscustomobject]@{
      verdict = "dry-run-would-accept"
      accepted = $false
      message = "Read-only evidence met the configured criteria. This dry-run is not an acceptance or hardware claim."
      inventory = @($sample.monitors)
      sample = $sample
      errors = @()
      authority_verifications = @($authorityVerifications)
    }
  } catch {
    return [pscustomobject]@{
      verdict = "dry-run-rejected"
      accepted = $false
      message = "Read-only evidence visibly rejected the configured state before acceptance."
      inventory = @($inventory)
      sample = $null
      errors = @((ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$_.Exception.Message) -Fallback "dry-run rejection without a readable single-line diagnostic"))
      authority_verifications = @()
    }
  }
}

function Invoke-ThreeDisplayApplyAcceptance {
  param([Parameter(Mandatory = $true)]$Configuration)
  # ShowAsioLocal authority verifications, in mandatory order:
  #   1. "pre-mutation" - before any process enumeration or window
  #      observation/mutation happens at all;
  #   2. "pre-executable-use" - immediately before the harness relies on the
  #      artifact executable again (the maximize mutation and the stability
  #      sampling phase), so a mutation between the two checks is caught.  Each
  #      verification carries the exact artifact source S, evidence HEAD E,
  #      and source branch B.
  $authorityVerifications = [System.Collections.Generic.List[object]]::new()
  $preMutationAuthority = Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $Configuration -Phase "pre-mutation"
  if ($null -ne $preMutationAuthority) { $authorityVerifications.Add($preMutationAuthority) }
  $operation = [ordered]@{ performed = $false; kind = "none"; target_role = $null; prechange_revalidation = $null; reason = "already maximized or no action required" }
  $initial = Get-ThreeDisplayStrictSample -Configuration $Configuration -RequireEditorMaximized $false
  $editor = @($initial.windows | Where-Object { $_.role -eq "editor" })[0]
  if (-not [bool]$editor.window.maximized) {
    # Revalidate immediately before the only reversible action.  In
    # particular, all output placements must still be correct: this harness
    # never masks a swapped/default output by moving it.
    $prechange = Get-ThreeDisplayStrictSample -Configuration $Configuration -RequireEditorMaximized $false
    $currentEditor = @($prechange.windows | Where-Object { $_.role -eq "editor" })[0]
    $preUseAuthority = Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $Configuration -Phase "pre-executable-use"
    if ($null -ne $preUseAuthority) { $authorityVerifications.Add($preUseAuthority) }
    [void](Invoke-ThreeDisplayWindowMaximize `
      -HandleDecimal ([long]$currentEditor.window.handle_decimal) `
      -ExpectedProcessId ([uint32]$currentEditor.window.owner_pid) `
      -ExpectedTitle ([string]$currentEditor.window.title))
    $operation = [ordered]@{
      performed = $true; kind = "maximize-editor"; target_role = "editor"
      prechange_revalidation = $prechange.observed_at_utc
      reason = "explicit -Apply allowed only the already-correct editor HWND to maximize"
    }
  } else {
    $preUseAuthority = Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $Configuration -Phase "pre-executable-use"
    if ($null -ne $preUseAuthority) { $authorityVerifications.Add($preUseAuthority) }
  }
  $stable = [System.Collections.Generic.List[object]]::new()
  $consecutive = 0
  for ($attempt = 1; $attempt -le $Configuration.max_sample_attempts; $attempt++) {
    try {
      $sample = Get-ThreeDisplayStrictSample -Configuration $Configuration -RequireEditorMaximized $true
      $consecutive++
      $stable.Add([pscustomobject]@{ attempt = $attempt; stable = $true; consecutive = $consecutive; sample = $sample; error = $null })
      if ($consecutive -ge $script:ThreeDisplayRequiredSamples) {
        return [pscustomobject]@{ accepted = $true; operation = $operation; before = $initial; samples = @($stable); failure = $null; authority_verifications = @($authorityVerifications) }
      }
    } catch {
      $consecutive = 0
      $stable.Add([pscustomobject]@{ attempt = $attempt; stable = $false; consecutive = 0; sample = $null; error = (ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$_.Exception.Message) -Fallback "sample attempt failed without a readable single-line diagnostic") })
    }
    if ($attempt -lt $Configuration.max_sample_attempts) { [void](Invoke-ThreeDisplaySampleDelay -Milliseconds $Configuration.sample_interval_ms) }
  }
  return [pscustomobject]@{ accepted = $false; operation = $operation; before = $initial; samples = @($stable); failure = "Fail closed: no $($script:ThreeDisplayRequiredSamples) consecutive stable three-display samples within $($Configuration.max_sample_attempts) attempts."; authority_verifications = @($authorityVerifications) }
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
    authority_mode = [string]$Configuration.authority_mode
    observed_at_utc = [DateTime]::UtcNow.ToString("o")
    checkout_root = $Configuration.checkout_root
    expected_executable_path = $(if ([string]$Configuration.authority_mode -eq $script:ThreeDisplayAuthorityShowAsioLocal -and [string]::IsNullOrWhiteSpace([string]$Configuration.expected_executable_path)) { "<derived-at-authority-gate-time>" } else { $Configuration.expected_executable_path })
    artifact_source_head = if ($Configuration.artifact_source_head) { $Configuration.artifact_source_head } else { $null }
    artifact_source_branch = if ($Configuration.artifact_source_branch) { $Configuration.artifact_source_branch } else { $null }
    evidence_head = if ($Configuration.expected_git_head) { $Configuration.expected_git_head } else { $null }
    fully_configured = $Configuration.fully_configured
    show_asio_authority_contract = $(if ([string]$Configuration.authority_mode -ne $script:ThreeDisplayAuthorityShowAsioLocal) { $null } else {
      [ordered]@{
        artifact_flavor = $script:ThreeDisplayShowAsioArtifactFlavor
        manifest_filename = $script:ThreeDisplayShowAsioManifestFilename
        checker_relative_path = $script:ThreeDisplayShowAsioCheckerRelativePath
        artifact_parent_relative = $script:ThreeDisplayShowAsioArtifactRelativeParent
        executable_name = $script:ThreeDisplayShowAsioExecutableName
        process_image_name = $script:ThreeDisplayShowAsioProcessName
        required_verifications_per_apply = 2
        distribution_approved = $false
      }
    })
    artifact_authority = $Configuration.artifact_authority
    expectations = [ordered]@{
      sha256 = if ($Configuration.expected_sha256) { $Configuration.expected_sha256 } else { $null }
      byte_size = if ([uint64]$Configuration.expected_byte_size -gt 0) { [uint64]$Configuration.expected_byte_size } else { $null }
      product_version = if ($Configuration.expected_product_version) { $Configuration.expected_product_version } else { $null }
      git_head = if ($Configuration.expected_git_head) { $Configuration.expected_git_head } else { $null }
      evidence_head = if ($Configuration.expected_git_head) { $Configuration.expected_git_head } else { $null }
      current_harness_head = if ($Configuration.expected_git_head) { $Configuration.expected_git_head } else { $null }
      current_harness_branch = if ($Configuration.artifact_source_branch) { $Configuration.artifact_source_branch } else { $null }
      artifact_source_head = if ($Configuration.artifact_source_head) { $Configuration.artifact_source_head } else { $null }
      artifact_source_branch = if ($Configuration.artifact_source_branch) { $Configuration.artifact_source_branch } else { $null }
      cdp_port = if ($Configuration.cdp_port -gt 0) { $Configuration.cdp_port } else { $null }
      roles = @($Configuration.roles)
    }
    safety = [ordered]@{
      read_only_default = $true
      creates_or_changes_syndocal_outputs = $false
      launches_or_terminates_processes = $false
      launches_or_terminates_syndocal_processes = $false
      spawns_show_asio_checker_node_child = ([string]$Configuration.authority_mode -eq $script:ThreeDisplayAuthorityShowAsioLocal)
      screenshots_taken = $false
      hardware_access = $false
      non_loopback_network_access = $false
      loopback_cdp_observation_only = $true
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
        authority_verifications = @($applyResult.authority_verifications)
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
        authority_verifications = @($dry.authority_verifications)
        native_hardware_claim = $false
      }
      if ($dry.errors.Count -gt 0) { $failure = @($dry.errors)[0] }
      [void](Write-ThreeDisplayEvidenceJson -EvidenceDirectory $EvidenceDirectory -FileName "monitors.json" -Value ([ordered]@{ inventory = @($dry.inventory); decision = $dry.verdict }))
    }
  } catch {
    $failure = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$_.Exception.Message) -Fallback "acceptance failed without a readable single-line diagnostic"
    $final = [ordered]@{ verdict = "rejected"; accepted = $false; errors = @($failure); authority_verifications = @(); native_hardware_claim = $false }
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
    # A partial dry-run is discovery only and exits successfully after its
    # labelled not-configured evidence.  A fully configured dry-run that
    # rejects any exact-show invariant (including the app-owned ID-to-HWND
    # proof) is a blocked final-show route and must not return a success code.
    succeeded = (
      [bool]$final.accepted -or
      ((-not $Configuration.apply) -and [string]$final.verdict -eq "not-configured") -or
      ((-not $Configuration.apply) -and [string]$final.verdict -eq "dry-run-would-accept")
    )
    accepted = [bool]$final.accepted
    verdict = [string]$final.verdict
    failure = $failure
    final = $final
  }
}

function Get-ThreeDisplayDefaultEvidenceRoot {
  # Default evidence root: this checkout's gitignored target\qa tree (the
  # Cargo target directory is excluded by .gitignore), so evidence written by
  # one run can never dirty a later run's exact-artifact clean-checkout gate.
  return [IO.Path]::GetFullPath((Join-Path $script:ThreeDisplayCheckoutRoot "target\qa"))
}

function Invoke-ThreeDisplayAcceptanceMain {
  $evidenceDirectory = $null
  $explicitRoot = -not [string]::IsNullOrWhiteSpace($EvidenceRootPath)
  $root = if ($explicitRoot) { $EvidenceRootPath } else { Get-ThreeDisplayDefaultEvidenceRoot }
  if (-not $explicitRoot -and -not (Test-Path -LiteralPath $root -PathType Container)) {
    # Only the checkout-local gitignored default may be created on demand;
    # explicit operator-supplied roots must already exist and are never created.
    [void](New-Item -ItemType Directory -Path $root -Force -ErrorAction Stop)
  }
  $evidenceDirectory = New-ThreeDisplayEvidenceDirectory -RootPath $root -Slug $EvidenceSlug
  try {
    $configuration = New-ThreeDisplayConfiguration -IsApply ([bool]$Apply) -AuthorityMode $AuthorityMode -ExecutablePath $ExpectedExecutablePath -Sha256 $ExpectedSha256 -ProductVersion $ExpectedProductVersion -GitHead $ExpectedGitHead -ArtifactSourceHead $ExpectedArtifactSourceHead -ArtifactSourceBranch $ExpectedArtifactSourceBranch -EditorIdentity $ExpectedEditorMonitorIdentity -LedIdentity $ExpectedLedMonitorIdentity -ProjectorIdentity $ExpectedProjectorMonitorIdentity -LedId $LedOutputId -LedLabel $LedOutputLabel -ProjectorId $ProjectorOutputId -ProjectorLabel $ProjectorOutputLabel -CdpPort $CdpPort -IntervalMs $SampleIntervalMs -Attempts $MaxSampleAttempts -CheckoutRootPath $script:ThreeDisplayCheckoutRoot -ShowAsioNodeExecutablePath $ShowAsioNodeExecutablePath
    $result = Invoke-ThreeDisplayAcceptance -Configuration $configuration -EvidenceDirectory $evidenceDirectory
    [Console]::Out.WriteLine((@{ evidence_directory = $result.evidence_directory; verdict = $result.verdict; accepted = $result.accepted; native_hardware_claim = $false } | ConvertTo-Json -Compress))
    if ($result.succeeded) { exit 0 }
    [Console]::Error.WriteLine("Three-display acceptance rejected: $($result.failure). Evidence: $evidenceDirectory")
    exit 1
  } catch {
    $message = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ([string]$_.Exception.Message) -Fallback "three-display harness failed without a readable single-line diagnostic"
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
