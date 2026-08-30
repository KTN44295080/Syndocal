[CmdletBinding()]
param(
  [string]$MinimumMaximizedClient = "1920x1000",
  [string]$ExpectedFullscreen = "1920x1080",
  [ValidateRange(0, 16)]
  [int]$TolerancePx = 2,
  [ValidateRange(30, 900)]
  [int]$StartupTimeoutSeconds = 420,
  [string]$EvidenceDir = "",
  [ValidateRange(1024, 65535)]
  [int]$CdpPort = 5188,
  [switch]$SelfTest
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
  throw "The automated native window acceptance currently requires Windows."
}

Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public delegate bool SyndocalEnumWindowsProc(IntPtr hWnd, IntPtr lParam);

public static class SyndocalNativeWindow {
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

  [StructLayout(LayoutKind.Sequential)]
  public struct MONITORINFO {
    public int Size;
    public RECT Monitor;
    public RECT Work;
    public uint Flags;
  }

  [DllImport("user32.dll")]
  public static extern bool EnumWindows(SyndocalEnumWindowsProc callback, IntPtr lParam);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);

  [DllImport("user32.dll")]
  public static extern int GetWindowTextLength(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool IsWindowVisible(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool IsZoomed(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool GetClientRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern bool ClientToScreen(IntPtr hWnd, ref POINT point);

  [DllImport("user32.dll")]
  public static extern bool ShowWindowAsync(IntPtr hWnd, int command);

  [DllImport("user32.dll")]
  public static extern bool SetForegroundWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern IntPtr SetFocus(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool GetCursorPos(out POINT point);

  [DllImport("user32.dll")]
  public static extern bool SetCursorPos(int x, int y);

  [DllImport("user32.dll")]
  public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);

  [DllImport("user32.dll")]
  public static extern IntPtr GetForegroundWindow();

  [DllImport("user32.dll")]
  public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

  [DllImport("kernel32.dll")]
  public static extern uint GetCurrentThreadId();

  [DllImport("user32.dll")]
  public static extern bool AttachThreadInput(uint sourceThreadId, uint targetThreadId, bool attach);

  [DllImport("user32.dll")]
  public static extern bool BringWindowToTop(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern void keybd_event(byte virtualKey, byte scanCode, uint flags, UIntPtr extraInfo);

  [DllImport("user32.dll")]
  public static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);

  [DllImport("user32.dll")]
  public static extern bool PrintWindow(IntPtr hWnd, IntPtr destination, uint flags);

  [DllImport("user32.dll")]
  public static extern IntPtr MonitorFromWindow(IntPtr hWnd, uint flags);

  [DllImport("user32.dll")]
  public static extern bool GetMonitorInfo(IntPtr monitor, ref MONITORINFO info);
}
"@

# Marker for any CDP trust-boundary failure: a raced/foreign listener, a
# non-conforming WebSocket endpoint, or a competing remote-debugging argument.
# Wait loops must rethrow this type immediately instead of polling through it.
class SyndocalCdpTrustViolation : Exception {
  SyndocalCdpTrustViolation([string]$Message) : base($Message) {}
}

function ConvertFrom-DimensionText {
  param([Parameter(Mandatory = $true)][string]$Value)

  if ($Value -notmatch '^(\d+)x(\d+)$') {
    throw "Expected a WIDTHxHEIGHT dimension, received '$Value'."
  }

  [pscustomobject]@{
    Width = [int]$Matches[1]
    Height = [int]$Matches[2]
  }
}

function Get-WindowTitle {
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  $length = [SyndocalNativeWindow]::GetWindowTextLength($Handle)
  $builder = [Text.StringBuilder]::new([Math]::Max(1, $length + 1))
  [void][SyndocalNativeWindow]::GetWindowText($Handle, $builder, $builder.Capacity)
  $builder.ToString()
}

function Find-WindowByTitle {
  param([Parameter(Mandatory = $true)][string]$Title)

  $script:syndocalMatchedWindow = [IntPtr]::Zero
  $callback = [SyndocalEnumWindowsProc]{
    param([IntPtr]$Handle, [IntPtr]$Unused)
    if ([SyndocalNativeWindow]::IsWindowVisible($Handle) -and (Get-WindowTitle -Handle $Handle) -eq $Title) {
      $script:syndocalMatchedWindow = $Handle
      return $false
    }
    return $true
  }
  [void][SyndocalNativeWindow]::EnumWindows($callback, [IntPtr]::Zero)
  $script:syndocalMatchedWindow
}

function Find-WindowByTitleAndProcess {
  param(
    [Parameter(Mandatory = $true)][string]$Title,
    [Parameter(Mandatory = $true)][uint32]$ProcessId
  )

  $script:syndocalMatchedWindow = [IntPtr]::Zero
  $callback = [SyndocalEnumWindowsProc]{
    param([IntPtr]$Handle, [IntPtr]$Unused)
    if (-not [SyndocalNativeWindow]::IsWindowVisible($Handle) -or (Get-WindowTitle -Handle $Handle) -ne $Title) {
      return $true
    }
    [uint32]$candidateProcessId = 0
    [void][SyndocalNativeWindow]::GetWindowThreadProcessId($Handle, [ref]$candidateProcessId)
    if ($candidateProcessId -eq $ProcessId) {
      $script:syndocalMatchedWindow = $Handle
      return $false
    }
    return $true
  }
  [void][SyndocalNativeWindow]::EnumWindows($callback, [IntPtr]::Zero)
  $script:syndocalMatchedWindow
}

function Wait-ForWindowByTitleAndProcess {
  param(
    [Parameter(Mandatory = $true)][string]$Title,
    [Parameter(Mandatory = $true)][uint32]$ProcessId,
    [ValidateRange(1, 120)][int]$TimeoutSeconds = 30
  )

  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  while ([DateTime]::UtcNow -lt $deadline) {
    $window = Find-WindowByTitleAndProcess -Title $Title -ProcessId $ProcessId
    if ($window -ne [IntPtr]::Zero) {
      return $window
    }
    Start-Sleep -Milliseconds 200
  }
  throw "Timed out waiting for verified '$Title' window in QA PID $ProcessId."
}

function Wait-ForWindowToCloseByTitleAndProcess {
  param(
    [Parameter(Mandatory = $true)][string]$Title,
    [Parameter(Mandatory = $true)][uint32]$ProcessId,
    [ValidateRange(1, 120)][int]$TimeoutSeconds = 30
  )

  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  while ([DateTime]::UtcNow -lt $deadline) {
    if ((Find-WindowByTitleAndProcess -Title $Title -ProcessId $ProcessId) -eq [IntPtr]::Zero) {
      return
    }
    Start-Sleep -Milliseconds 200
  }
  throw "Timed out waiting for '$Title' in QA PID $ProcessId to close."
}

function Get-ClientDimensions {
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  $rect = [SyndocalNativeWindow+RECT]::new()
  if (-not [SyndocalNativeWindow]::GetClientRect($Handle, [ref]$rect)) {
    throw "GetClientRect failed for the native QA window."
  }
  [pscustomobject]@{
    Width = $rect.Right - $rect.Left
    Height = $rect.Bottom - $rect.Top
  }
}

function Get-MonitorDimensions {
  param([Parameter(Mandatory = $true)][IntPtr]$Handle)

  $monitor = [SyndocalNativeWindow]::MonitorFromWindow($Handle, 2)
  $info = [SyndocalNativeWindow+MONITORINFO]::new()
  $info.Size = [Runtime.InteropServices.Marshal]::SizeOf([type][SyndocalNativeWindow+MONITORINFO])
  if ($monitor -eq [IntPtr]::Zero -or -not [SyndocalNativeWindow]::GetMonitorInfo($monitor, [ref]$info)) {
    throw "GetMonitorInfo failed for the native QA window."
  }
  [pscustomobject]@{
    Width = $info.Monitor.Right - $info.Monitor.Left
    Height = $info.Monitor.Bottom - $info.Monitor.Top
    WorkWidth = $info.Work.Right - $info.Work.Left
    WorkHeight = $info.Work.Bottom - $info.Work.Top
  }
}

function Wait-ForMinimumClientDimensions {
  param(
    [Parameter(Mandatory = $true)][IntPtr]$Handle,
    [Parameter(Mandatory = $true)]$Minimum,
    [int]$TimeoutSeconds = 20,
    [switch]$RequireMaximized
  )

  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $stableMatches = 0
  $lastKey = ""
  $last = Get-ClientDimensions -Handle $Handle
  while ([DateTime]::UtcNow -lt $deadline) {
    $last = Get-ClientDimensions -Handle $Handle
    $currentKey = "$($last.Width)x$($last.Height)"
    $largeEnough =
      $last.Width -ge ($Minimum.Width - $TolerancePx) -and
      $last.Height -ge ($Minimum.Height - $TolerancePx)
    $requiredModeReached = -not $RequireMaximized -or [SyndocalNativeWindow]::IsZoomed($Handle)
    if ($largeEnough -and $requiredModeReached -and $currentKey -eq $lastKey) {
      $stableMatches += 1
      if ($stableMatches -ge 3) {
        return $last
      }
    } elseif ($largeEnough -and $requiredModeReached) {
      $stableMatches = 1
    } else {
      $stableMatches = 0
    }
    $lastKey = $currentKey
    Start-Sleep -Milliseconds 200
  }
  $modeRequirement = if ($RequireMaximized) { " in app-owned maximized mode" } else { "" }
  throw "Timed out waiting for at least $($Minimum.Width)x$($Minimum.Height)$modeRequirement; last client was $($last.Width)x$($last.Height)."
}

function Test-Dimensions {
  param(
    [Parameter(Mandatory = $true)]$Actual,
    [Parameter(Mandatory = $true)]$Expected,
    [ValidateRange(0, 16)]
    [int]$AllowedTolerancePx = $TolerancePx
  )
  [Math]::Abs($Actual.Width - $Expected.Width) -le $AllowedTolerancePx -and
    [Math]::Abs($Actual.Height - $Expected.Height) -le $AllowedTolerancePx
}

function Wait-ForClientDimensions {
  param(
    [Parameter(Mandatory = $true)][IntPtr]$Handle,
    [Parameter(Mandatory = $true)]$Expected,
    [ValidateRange(0, 16)]
    [int]$AllowedTolerancePx = $TolerancePx,
    [int]$TimeoutSeconds = 20
  )

  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $stableMatches = 0
  $last = Get-ClientDimensions -Handle $Handle
  while ([DateTime]::UtcNow -lt $deadline) {
    $last = Get-ClientDimensions -Handle $Handle
    if (Test-Dimensions -Actual $last -Expected $Expected -AllowedTolerancePx $AllowedTolerancePx) {
      $stableMatches += 1
      if ($stableMatches -ge 3) {
        return $last
      }
    } else {
      $stableMatches = 0
    }
    Start-Sleep -Milliseconds 200
  }
  throw "Timed out waiting for $($Expected.Width)x$($Expected.Height); last client was $($last.Width)x$($last.Height)."
}

function Send-NativeKey {
  param(
    [Parameter(Mandatory = $true)][IntPtr]$Handle,
    [Parameter(Mandatory = $true)][ValidateRange(0, 255)][int]$VirtualKey
  )

  [void][SyndocalNativeWindow]::ShowWindowAsync($Handle, 5)
  $currentThread = [SyndocalNativeWindow]::GetCurrentThreadId()
  $foregroundWindow = [SyndocalNativeWindow]::GetForegroundWindow()
  [uint32]$ignoredProcessId = 0
  $foregroundThread = if ($foregroundWindow -eq [IntPtr]::Zero) {
    0
  } else {
    [SyndocalNativeWindow]::GetWindowThreadProcessId($foregroundWindow, [ref]$ignoredProcessId)
  }
  $ignoredProcessId = 0
  $targetThread = [SyndocalNativeWindow]::GetWindowThreadProcessId($Handle, [ref]$ignoredProcessId)
  $attachedThreads = [System.Collections.Generic.List[uint32]]::new()
  try {
    foreach ($thread in @($foregroundThread, $targetThread) | Select-Object -Unique) {
      if ($thread -ne 0 -and $thread -ne $currentThread) {
        if (-not [SyndocalNativeWindow]::AttachThreadInput($currentThread, $thread, $true)) {
          throw "Could not attach to native window thread $thread."
        }
        $attachedThreads.Add($thread)
      }
    }

    [void][SyndocalNativeWindow]::BringWindowToTop($Handle)
    [void][SyndocalNativeWindow]::SetForegroundWindow($Handle)
    [void][SyndocalNativeWindow]::SetFocus($Handle)
  } finally {
    for ($index = $attachedThreads.Count - 1; $index -ge 0; $index -= 1) {
      [void][SyndocalNativeWindow]::AttachThreadInput(
        $currentThread,
        $attachedThreads[$index],
        $false
      )
    }
  }

  Start-Sleep -Milliseconds 250
  if ([SyndocalNativeWindow]::GetForegroundWindow() -ne $Handle) {
    throw "Refusing to inject a key because the exact isolated native QA window is not foreground."
  }

  # Foregrounding the top-level HWND does not guarantee that WebView2 owns the
  # keyboard focus on a freshly started frameless window. Click a known inert
  # point in the center of the drag-only top bar, then restore the user's cursor.
  $clientRect = [SyndocalNativeWindow+RECT]::new()
  if (-not [SyndocalNativeWindow]::GetClientRect($Handle, [ref]$clientRect)) {
    throw "GetClientRect failed while focusing the native QA WebView."
  }
  $clientOrigin = [SyndocalNativeWindow+POINT]::new()
  if (-not [SyndocalNativeWindow]::ClientToScreen($Handle, [ref]$clientOrigin)) {
    throw "ClientToScreen failed while focusing the native QA WebView."
  }
  $savedCursor = [SyndocalNativeWindow+POINT]::new()
  if (-not [SyndocalNativeWindow]::GetCursorPos([ref]$savedCursor)) {
    throw "GetCursorPos failed before focusing the native QA WebView."
  }
  $focusX = $clientOrigin.X + [int](($clientRect.Right - $clientRect.Left) / 2)
  $focusY = $clientOrigin.Y + 15
  try {
    if (-not [SyndocalNativeWindow]::SetCursorPos($focusX, $focusY)) {
      throw "SetCursorPos failed while focusing the native QA WebView."
    }
    [SyndocalNativeWindow]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    [SyndocalNativeWindow]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
  } finally {
    [void][SyndocalNativeWindow]::SetCursorPos($savedCursor.X, $savedCursor.Y)
  }
  Start-Sleep -Milliseconds 100
  [SyndocalNativeWindow]::keybd_event([byte]$VirtualKey, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 40
  [SyndocalNativeWindow]::keybd_event([byte]$VirtualKey, 0, 2, [UIntPtr]::Zero)
}

function Save-ClientScreenshot {
  param(
    [Parameter(Mandatory = $true)][IntPtr]$Handle,
    [Parameter(Mandatory = $true)][string]$Path
  )

  $size = Get-ClientDimensions -Handle $Handle
  $bitmap = [Drawing.Bitmap]::new($size.Width, $size.Height)
  $graphics = [Drawing.Graphics]::FromImage($bitmap)
  $deviceContext = [IntPtr]::Zero
  try {
    # CopyFromScreen records whichever app happens to cover these coordinates. PrintWindow
    # targets this exact HWND and includes DirectComposition/WebView content even when the
    # Codex or terminal window is in front of the QA app.
    $graphics.Clear([Drawing.Color]::Magenta)
    $deviceContext = $graphics.GetHdc()
    if (-not [SyndocalNativeWindow]::PrintWindow($Handle, $deviceContext, 3)) {
      throw "PrintWindow failed for the isolated native QA window."
    }
  } finally {
    if ($deviceContext -ne [IntPtr]::Zero) {
      $graphics.ReleaseHdc($deviceContext)
    }
  }
  try {
    $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $graphics.Dispose()
    $bitmap.Dispose()
  }
}

function Get-ScreenshotVisualMetrics {
  param([Parameter(Mandatory = $true)][string]$Path)

  $bitmap = [Drawing.Bitmap]::FromFile($Path)
  try {
    $stepX = [Math]::Max(1, [int][Math]::Floor($bitmap.Width / 160))
    $stepY = [Math]::Max(1, [int][Math]::Floor($bitmap.Height / 100))
    $colors = [System.Collections.Generic.HashSet[int]]::new()
    $sampleCount = 0
    $darkCount = 0
    $nonWhiteCount = 0
    for ($y = 0; $y -lt $bitmap.Height; $y += $stepY) {
      for ($x = 0; $x -lt $bitmap.Width; $x += $stepX) {
        $color = $bitmap.GetPixel($x, $y)
        [void]$colors.Add($color.ToArgb())
        $sampleCount += 1
        if ([Math]::Max($color.R, [Math]::Max($color.G, $color.B)) -lt 96) {
          $darkCount += 1
        }
        if ([Math]::Min($color.R, [Math]::Min($color.G, $color.B)) -lt 240) {
          $nonWhiteCount += 1
        }
      }
    }
    [pscustomobject]@{
      Width = $bitmap.Width
      Height = $bitmap.Height
      SampleCount = $sampleCount
      UniqueSampledColors = $colors.Count
      DarkPixelRatio = [Math]::Round($darkCount / $sampleCount, 4)
      NonWhitePixelRatio = [Math]::Round($nonWhiteCount / $sampleCount, 4)
    }
  } finally {
    $bitmap.Dispose()
  }
}

function Save-VerifiedClientScreenshot {
  param(
    [Parameter(Mandatory = $true)][IntPtr]$Handle,
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Stage,
    [ValidateRange(1, 60)][int]$TimeoutSeconds = 20
  )

  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $lastMetrics = $null
  do {
    Save-ClientScreenshot -Handle $Handle -Path $Path
    $lastMetrics = Get-ScreenshotVisualMetrics -Path $Path
    if (
      $lastMetrics.UniqueSampledColors -ge 32 -and
      $lastMetrics.DarkPixelRatio -ge 0.10 -and
      $lastMetrics.NonWhitePixelRatio -ge 0.25
    ) {
      return $lastMetrics
    }
    Start-Sleep -Milliseconds 500
  } while ([DateTime]::UtcNow -lt $deadline)

  throw "$Stage screenshot is blank or visually unready: $($lastMetrics.UniqueSampledColors) sampled colors, dark ratio $($lastMetrics.DarkPixelRatio), non-white ratio $($lastMetrics.NonWhitePixelRatio)."
}

function Test-ApprovedCdpWebSocketUrl {
  # Fail-closed endpoint pin: only an exact ws://127.0.0.1:<Port>/devtools/page/
  # URL may ever be dialed. A remote host, other scheme, wrong port, non-page
  # path, credentials, query/fragment, or unparsable value is hostile evidence
  # from a raced or spoofed /json/list response, never a retryable nuisance.
  param(
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$WebSocketDebuggerUrl,
    [Parameter(Mandatory = $true)][ValidateRange(1, 65535)][int]$Port
  )

  $parsed = $null
  if (-not [Uri]::TryCreate($WebSocketDebuggerUrl, [UriKind]::Absolute, [ref]$parsed)) {
    throw [SyndocalCdpTrustViolation]::new("CDP WebSocket URL '$WebSocketDebuggerUrl' is not an absolute URI.")
  }
  if ($parsed.Scheme -ne "ws") {
    throw [SyndocalCdpTrustViolation]::new("CDP endpoint '$WebSocketDebuggerUrl' uses scheme '$($parsed.Scheme)' instead of the required loopback ws scheme.")
  }
  if ($parsed.HostNameType -ne [UriHostNameType]::IPv4 -or $parsed.Host -cne "127.0.0.1") {
    throw [SyndocalCdpTrustViolation]::new("CDP endpoint '$WebSocketDebuggerUrl' targets host '$($parsed.Host)'; only the exact loopback literal 127.0.0.1 is accepted.")
  }
  if ($parsed.Port -ne $Port) {
    throw [SyndocalCdpTrustViolation]::new("CDP endpoint '$WebSocketDebuggerUrl' dials port $($parsed.Port), not the configured QA CDP port $Port.")
  }
  if (-not [string]::IsNullOrEmpty($parsed.UserInfo)) {
    throw [SyndocalCdpTrustViolation]::new("CDP endpoint '$WebSocketDebuggerUrl' embeds credentials; that is never part of the harness contract.")
  }
  if (-not [string]::IsNullOrEmpty($parsed.Query) -or -not [string]::IsNullOrEmpty($parsed.Fragment)) {
    throw [SyndocalCdpTrustViolation]::new("CDP endpoint '$WebSocketDebuggerUrl' carries a query or fragment; page endpoints are bare paths.")
  }
  # '/devtools/page/' is exactly 15 characters; at least one target id must follow.
  if ($parsed.AbsolutePath -notlike "/devtools/page/*" -or $parsed.AbsolutePath.Length -le 15) {
    throw [SyndocalCdpTrustViolation]::new("CDP endpoint '$WebSocketDebuggerUrl' path '$($parsed.AbsolutePath)' is not an exact /devtools/page/ target.")
  }
  return $parsed
}

function Get-CdpProcessLineageRecord {
  param([Parameter(Mandatory = $true)][uint32]$ProcessId)

  $instance = Get-CimInstance -ClassName Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction SilentlyContinue
  if ($null -eq $instance) { return $null }
  # An unparsable creation time is reported as absent ($null), never guessed:
  # the ownership proof treats any unverifiable birth time as a failure.
  $createdAtUtc = $null
  $createdProperty = $instance.PSObject.Properties["CreationDate"]
  if ($null -ne $createdProperty -and $null -ne $createdProperty.Value) {
    try {
      $createdAtUtc = ([DateTime]$createdProperty.Value).ToUniversalTime()
    } catch {
      $createdAtUtc = $null
    }
  }
  [pscustomobject]@{
    ProcessId = [uint32]$instance.ProcessId
    ParentProcessId = [uint32]$instance.ParentProcessId
    CreatedAtUtc = $createdAtUtc
  }
}

function Test-CdpListenerOwnershipEntries {
  # Pure decision core for CDP listener ownership over one snapshot of TCP
  # rows. Every row must bind exactly 127.0.0.1, and every owning PID must be
  # the verified QA process itself or reach it through a parent chain that
  # never repeats a PID, never disappears mid-walk, and never predates the QA
  # process (which would expose PID reuse). The age comparison is mandatory:
  # a missing or unparsable QA (or ancestor) creation time makes the PID-reuse
  # proof unverifiable and therefore fails closed instead of being skipped.
  # Deterministic: the process table arrives through $ResolveProcess so
  # self-tests can seed synthetic trees.
  param(
    [AllowEmptyCollection()][object[]]$Listeners,
    [Parameter(Mandatory = $true)][uint32]$QaProcessId,
    $QaCreatedAtUtc,
    [ValidateRange(0, 3600)][int]$AncestorClockSkewToleranceSeconds = 5,
    [Parameter(Mandatory = $true)][scriptblock]$ResolveProcess
  )

  $newFailure = {
    param([string]$Reason)
    [pscustomobject]@{ Passed = $false; Reason = $Reason; VerifiedListenerPids = @() }
  }

  if ($null -eq $QaCreatedAtUtc) {
    return (& $newFailure "verified QA process creation time is unavailable; the ancestry age proof against PID reuse cannot run")
  }
  try {
    $qaCreatedAtUtcVerified = ([DateTime]$QaCreatedAtUtc).ToUniversalTime()
  } catch {
    return (& $newFailure "verified QA process creation time '$QaCreatedAtUtc' is not a valid timestamp; the ancestry age proof against PID reuse cannot run")
  }

  $rows = @($Listeners | Where-Object { $null -ne $_ })
  if ($rows.Count -eq 0) {
    return (& $newFailure "no TCP listener owns the CDP port; nothing provably serves it")
  }
  $misbound = @($rows | Where-Object {
    [string]$_.PSObject.Properties["LocalAddress"].Value -cne "127.0.0.1"
  })
  if ($misbound.Count -gt 0) {
    $addresses = (@($misbound | ForEach-Object {
      [string]$_.PSObject.Properties["LocalAddress"].Value
    }) | Select-Object -Unique) -join ", "
    return (& $newFailure "listener(s) bound outside the configured 127.0.0.1 address: $addresses")
  }

  $verifiedOwners = [Collections.Generic.List[uint32]]::new()
  $ownerPids = @(@($rows | ForEach-Object {
    [uint32]$_.PSObject.Properties["OwningProcess"].Value
  }) | Sort-Object -Unique)
  foreach ($ownerPid in $ownerPids) {
    if ($ownerPid -eq $QaProcessId) {
      $verifiedOwners.Add($ownerPid)
      continue
    }
    $visited = [Collections.Generic.HashSet[uint32]]::new()
    $current = $ownerPid
    while ($true) {
      if (-not $visited.Add($current)) {
        return (& $newFailure "repeated ancestry at PID $current while proving listener owner $ownerPid")
      }
      $record = & $ResolveProcess $current
      if ($null -eq $record) {
        return (& $newFailure "ancestry broke at PID $current (process gone or unreadable) before reaching QA PID $QaProcessId")
      }
      if ($current -ne $QaProcessId) {
        $createdAtProperty = $record.PSObject.Properties["CreatedAtUtc"]
        $createdAt = if ($null -ne $createdAtProperty) { $createdAtProperty.Value } else { $null }
        if ($null -eq $createdAt) {
          return (& $newFailure "creation time unavailable for ancestor PID $current; lineage age is unverifiable")
        }
        try {
          $ancestorCreatedAtUtc = ([DateTime]$createdAt).ToUniversalTime()
        } catch {
          return (& $newFailure "creation time for ancestor PID $current is not a valid timestamp; lineage age is unverifiable")
        }
        $oldestAllowed = $qaCreatedAtUtcVerified.AddSeconds(-$AncestorClockSkewToleranceSeconds)
        if ($ancestorCreatedAtUtc -lt $oldestAllowed) {
          return (& $newFailure "ancestor PID $current predates the verified QA process (possible PID reuse)")
        }
      }
      if ($current -eq $QaProcessId) { break }
      $parentProperty = $record.PSObject.Properties["ParentProcessId"]
      $parent = if ($null -ne $parentProperty) { [uint32]$parentProperty.Value } else { [uint32]0 }
      if ($parent -eq [uint32]0) {
        return (& $newFailure "ancestry terminated at PID 0 before reaching QA PID $QaProcessId")
      }
      $current = $parent
    }
    $verifiedOwners.Add($ownerPid)
  }

  [pscustomobject]@{
    Passed = $true
    Reason = "every listener is 127.0.0.1-bound and owned by QA PID $QaProcessId or its non-repeating live descendants"
    VerifiedListenerPids = @($verifiedOwners)
  }
}

function Assert-CdpLoopbackListenerOwnership {
  # Live gate run before every CDP touch: the configured port's listeners must
  # all be loopback-bound and owned by the verified QA syndocal process or one
  # of its proven descendants. Any foreign or raced listener fails closed.
  # Residual micro-TOCTOU: the OS could theoretically swap the owning process
  # between this ownership snapshot and the WebSocket dial. The harness accepts
  # that narrow window by re-running this gate immediately before every dial
  # and treating any observed anomaly as an immediate SyndocalCdpTrustViolation;
  # it deliberately does not widen the ownership definition or add a fallback
  # path to paper over the race.
  param(
    [Parameter(Mandatory = $true)][ValidateRange(1, 65535)][int]$Port,
    [Parameter(Mandatory = $true)][uint32]$QaProcessId
  )

  $qaRecord = Get-CdpProcessLineageRecord -ProcessId $QaProcessId
  if ($null -eq $qaRecord) {
    throw [SyndocalCdpTrustViolation]::new("Verified QA process PID $QaProcessId is gone; refusing all CDP use on port $Port.")
  }
  $listeners = @(Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)
  $proof = Test-CdpListenerOwnershipEntries -Listeners $listeners -QaProcessId $QaProcessId `
    -QaCreatedAtUtc $qaRecord.CreatedAtUtc `
    -ResolveProcess { param($ProcessIdToResolve) Get-CdpProcessLineageRecord -ProcessId ([uint32]$ProcessIdToResolve) }
  if (-not $proof.Passed) {
    throw [SyndocalCdpTrustViolation]::new(
      "CDP port $Port listener ownership failed closed: $($proof.Reason). " +
      "Expected only 127.0.0.1-bound listeners owned by QA PID $QaProcessId or its live non-repeating descendants."
    )
  }
}

function Assert-NoPreexistingWebView2RemoteDebuggingArguments {
  # Duplicate remote-debugging-* flags would let argument precedence decide
  # which endpoint WebView2 opens. This gate must own the CDP surface alone,
  # so any pre-existing remote-debugging port OR address OR pipe fails closed
  # before the harness appends its own loopback pair. The WebView2/Chromium
  # command line accepts '-', '--', and Windows slash-prefixed '/switch'
  # spellings, and 'remote-debugging-pipe' would hand CDP to whatever owns the
  # inherited stdio handles instead of a loopback port, so all three prefixes
  # of all three switches are hostile here; anything that merely looks like
  # one of these switches (even case-mangled) is rejected, not normalized.
  param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Value)

  if ([string]::IsNullOrWhiteSpace($Value)) { return }
  $matchedArgument = [regex]::Match(
    $Value,
    '(?i)(?:^|\s)(?:--|[-/])remote-debugging-(?:port|address|pipe)(?:=|\s|$)'
  )
  if ($matchedArgument.Success) {
    throw [SyndocalCdpTrustViolation]::new(
      "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS already contains '$($matchedArgument.Value.Trim())'; native pane acceptance requires its isolated loopback CDP arguments with no competing remote-debugging endpoint."
    )
  }
}

function Get-CdpPageTargets {
  param(
    [Parameter(Mandatory = $true)][int]$Port,
    [Parameter(Mandatory = $true)][uint32]$QaProcessId
  )

  # WebView2's CDP endpoint is deliberately loopback-only and supplied by the
  # process-scoped WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS below. Before touching
  # it, prove the port's listener belongs to the verified QA process lineage;
  # then accept only exact ws://127.0.0.1:<Port>/devtools/page/ endpoints.
  # Foreign, malformed, or mis-pointed targets are hostile evidence, not noise.
  Assert-CdpLoopbackListenerOwnership -Port $Port -QaProcessId $QaProcessId
  $response = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/json/list" -TimeoutSec 2 -ErrorAction Stop
  $pages = [Collections.Generic.List[object]]::new()
  foreach ($entry in @($response)) {
    if ($null -eq $entry) { continue }
    $typeProperty = $entry.PSObject.Properties["type"]
    if ($null -eq $typeProperty -or [string]$typeProperty.Value -ne "page") { continue }
    $urlProperty = $entry.PSObject.Properties["url"]
    $displayUrl = if ($null -ne $urlProperty) { [string]$urlProperty.Value } else { "<unknown>" }
    $wsProperty = $entry.PSObject.Properties["webSocketDebuggerUrl"]
    $wsUrl = if ($null -ne $wsProperty) { [string]$wsProperty.Value } else { "" }
    if ([string]::IsNullOrWhiteSpace($wsUrl)) {
      throw [SyndocalCdpTrustViolation]::new("CDP page target '$displayUrl' carried no webSocketDebuggerUrl; refusing to treat the listing as trusted.")
    }
    [void](Test-ApprovedCdpWebSocketUrl -WebSocketDebuggerUrl $wsUrl -Port $Port)
    $entry | Add-Member -NotePropertyName "cdp_port" -NotePropertyValue $Port
    $entry | Add-Member -NotePropertyName "qa_process_id" -NotePropertyValue $QaProcessId
    $pages.Add($entry)
  }
  @($pages)
}

function Invoke-CdpRuntimeEvaluate {
  param(
    [Parameter(Mandatory = $true)]$Page,
    [Parameter(Mandatory = $true)][string]$Expression,
    [ValidateRange(1, 60)][int]$TimeoutSeconds = 15
  )

  # Every evaluation re-proves the exact endpoint contract and the live
  # listener ownership immediately before dialing, so a raced or spoofed
  # target can never redirect traffic off the verified QA loopback pair.
  $socketUrlProperty = $Page.PSObject.Properties["webSocketDebuggerUrl"]
  $portProperty = $Page.PSObject.Properties["cdp_port"]
  $qaProcessProperty = $Page.PSObject.Properties["qa_process_id"]
  if ($null -eq $socketUrlProperty -or $null -eq $portProperty -or $null -eq $qaProcessProperty) {
    throw "Refusing CDP evaluation on a page descriptor without exact port/QA-process binding."
  }
  $approvedUri = Test-ApprovedCdpWebSocketUrl `
    -WebSocketDebuggerUrl ([string]$socketUrlProperty.Value) `
    -Port ([int]$portProperty.Value)
  Assert-CdpLoopbackListenerOwnership -Port ([int]$portProperty.Value) -QaProcessId ([uint32]$qaProcessProperty.Value)

  $socket = [System.Net.WebSockets.ClientWebSocket]::new()
  $cancellation = [Threading.CancellationTokenSource]::new()
  $cancellation.CancelAfter([TimeSpan]::FromSeconds($TimeoutSeconds))
  try {
    $socket.ConnectAsync($approvedUri, $cancellation.Token).GetAwaiter().GetResult()
    $request = [ordered]@{
      id = 1
      method = "Runtime.evaluate"
      params = [ordered]@{
        expression = $Expression
        awaitPromise = $true
        returnByValue = $true
      }
    } | ConvertTo-Json -Depth 8 -Compress
    $payload = [Text.Encoding]::UTF8.GetBytes($request)
    $socket.SendAsync(
      [ArraySegment[byte]]::new($payload),
      [System.Net.WebSockets.WebSocketMessageType]::Text,
      $true,
      $cancellation.Token
    ).GetAwaiter().GetResult()

    $buffer = [byte[]]::new(65536)
    while ($true) {
      $stream = [IO.MemoryStream]::new()
      try {
        do {
          $receive = $socket.ReceiveAsync(
            [ArraySegment[byte]]::new($buffer),
            $cancellation.Token
          ).GetAwaiter().GetResult()
          if ($receive.MessageType -eq [System.Net.WebSockets.WebSocketMessageType]::Close) {
            throw "CDP target closed while evaluating the native pane contract."
          }
          if ($receive.Count -gt 0) {
            $stream.Write($buffer, 0, $receive.Count)
          }
        } while (-not $receive.EndOfMessage)
        $responseText = [Text.Encoding]::UTF8.GetString($stream.ToArray())
      } finally {
        $stream.Dispose()
      }
      $response = $responseText | ConvertFrom-Json -ErrorAction Stop
      $responseId = $response.PSObject.Properties["id"]
      if ($null -eq $responseId -or $responseId.Value -ne 1) {
        # No CDP domains are enabled, but do not mistake an unsolicited event
        # for this evaluation's response if WebView2 ever emits one.
        continue
      }
      $errorPayload = $response.PSObject.Properties["error"]
      if ($null -ne $errorPayload -and $null -ne $errorPayload.Value) {
        throw "CDP Runtime.evaluate failed: $($errorPayload.Value.message)"
      }
      $evaluationResult = $response.PSObject.Properties["result"]
      if ($null -eq $evaluationResult -or $null -eq $evaluationResult.Value) {
        throw "CDP Runtime.evaluate returned no result envelope."
      }
      $exceptionDetails = $evaluationResult.Value.PSObject.Properties["exceptionDetails"]
      if ($null -ne $exceptionDetails -and $null -ne $exceptionDetails.Value) {
        $detail = $exceptionDetails.Value.text
        throw "Native pane DOM evaluation threw: $detail"
      }
      $remoteResult = $evaluationResult.Value.PSObject.Properties["result"]
      $remoteValue = if ($null -eq $remoteResult -or $null -eq $remoteResult.Value) {
        $null
      } else {
        $remoteResult.Value.PSObject.Properties["value"]
      }
      if ($null -eq $remoteValue) {
        throw "Native pane DOM evaluation returned no serializable value."
      }
      return $remoteValue.Value
    }
  } finally {
    $cancellation.Dispose()
    $socket.Dispose()
  }
}

function Assert-SingleMatchingCdpPage {
  # Pure decision seam for page identity: exactly one matching page wins;
  # zero matches returns $null so a wait loop keeps polling; more than one
  # matching page is duplicate native pane identity — ambiguous, hostile
  # evidence of a raced or duplicated target — and is raised as a typed
  # SyndocalCdpTrustViolation so callers fail immediately instead of timing out.
  param(
    [AllowEmptyCollection()][object[]]$MatchingPages,
    [Parameter(Mandatory = $true)][string]$ExpectedMode
  )

  if ($MatchingPages.Count -eq 1) { return $MatchingPages[0] }
  if ($MatchingPages.Count -gt 1) {
    throw [SyndocalCdpTrustViolation]::new(
      "CDP exposes $($MatchingPages.Count) '$ExpectedMode' pages; exact native pane identity is ambiguous."
    )
  }
  return $null
}

function Wait-ForCdpAppPage {
  param(
    [Parameter(Mandatory = $true)][int]$Port,
    [Parameter(Mandatory = $true)][uint32]$QaProcessId,
    [Parameter(Mandatory = $true)][ValidateSet("main", "stage", "timeline")][string]$ExpectedMode,
    [ValidateRange(1, 120)][int]$TimeoutSeconds = 45
  )

  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $lastObservation = "CDP endpoint not reachable"
  while ([DateTime]::UtcNow -lt $deadline) {
    try {
      $matchingPages = [Collections.Generic.List[object]]::new()
      foreach ($target in Get-CdpPageTargets -Port $Port -QaProcessId $QaProcessId) {
        $targetUrlProperty = $target.PSObject.Properties["url"]
        $targetUrl = if ($null -ne $targetUrlProperty) { [string]$targetUrlProperty.Value } else { "<unknown>" }
        try {
          $mode = Invoke-CdpRuntimeEvaluate -Page $target -Expression @'
(() => document.querySelector('.app')?.getAttribute('data-pane-window-mode') ?? '')()
'@
          $lastObservation = "$targetUrl => $mode"
          if ($mode -eq $ExpectedMode) {
            $matchingPages.Add($target)
          }
        } catch [SyndocalCdpTrustViolation] {
          throw
        } catch {
          $lastObservation = "$targetUrl`: $($_.Exception.Message)"
        }
      }
      # More than one '$ExpectedMode' page is a typed trust violation that
      # escapes this loop immediately; only zero matches keeps polling.
      $singlePage = Assert-SingleMatchingCdpPage -MatchingPages @($matchingPages) -ExpectedMode $ExpectedMode
      if ($null -ne $singlePage) {
        return $singlePage
      }
    } catch [SyndocalCdpTrustViolation] {
      throw
    } catch {
      $lastObservation = $_.Exception.Message
    }
    Start-Sleep -Milliseconds 250
  }
  throw "Timed out waiting for exactly one native '$ExpectedMode' CDP page on port $Port. Last observation: $lastObservation"
}

function Wait-ForCdpCondition {
  param(
    [Parameter(Mandatory = $true)]$Page,
    [Parameter(Mandatory = $true)][string]$Expression,
    [Parameter(Mandatory = $true)][string]$Description,
    [ValidateRange(1, 120)][int]$TimeoutSeconds = 30
  )

  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $lastValue = "no evaluation"
  while ([DateTime]::UtcNow -lt $deadline) {
    try {
      $lastValue = Invoke-CdpRuntimeEvaluate -Page $Page -Expression $Expression
      if ($lastValue -eq $true -or $lastValue -eq "true") {
        return
      }
    } catch [SyndocalCdpTrustViolation] {
      # A raced/foreign CDP endpoint is a trust takeover, not a nuisance to
      # poll through: rethrow immediately instead of burning the full timeout.
      throw
    } catch {
      $lastValue = $_.Exception.Message
    }
    Start-Sleep -Milliseconds 200
  }
  throw "Timed out waiting for native pane condition '$Description'. Last result: $lastValue"
}

function New-CdpVisibleDomClickExpression {
  # Pure seam so self-tests can prove encoding safety. Both caller-controlled
  # strings are embedded only as JSON string literals (which are also valid
  # JavaScript literals), so quotes, backslashes, and newlines in a selector
  # or description can never terminate or rewrite the evaluated program.
  param(
    [Parameter(Mandatory = $true)][string]$Selector,
    [Parameter(Mandatory = $true)][string]$Description
  )

  $selectorJson = $Selector | ConvertTo-Json -Compress
  $descriptionJson = $Description | ConvertTo-Json -Compress
  @"
(() => {
  const element = document.querySelector($selectorJson);
  const description = $descriptionJson;
  if (!(element instanceof HTMLElement)) throw new Error(description + ': selector did not resolve');
  const rect = element.getBoundingClientRect();
  const style = getComputedStyle(element);
  if (rect.width <= 0 || rect.height <= 0 || style.display === 'none' || style.visibility === 'hidden') {
    throw new Error(description + ': target is not visibly actionable');
  }
  if (element instanceof HTMLButtonElement && element.disabled) {
    throw new Error(description + ': target is disabled');
  }
  element.click();
  return true;
})()
"@
}

function Invoke-CdpVisibleDomClick {
  param(
    [Parameter(Mandatory = $true)]$Page,
    [Parameter(Mandatory = $true)][string]$Selector,
    [Parameter(Mandatory = $true)][string]$Description
  )

  $expression = New-CdpVisibleDomClickExpression -Selector $Selector -Description $Description
  $clicked = Invoke-CdpRuntimeEvaluate -Page $Page -Expression $expression
  if ($clicked -ne $true -and $clicked -ne "true") {
    throw "Native pane action '$Description' did not acknowledge a DOM click."
  }
}

function Get-NativePaneDomState {
  param([Parameter(Mandatory = $true)]$Page)

  $raw = Invoke-CdpRuntimeEvaluate -Page $Page -Expression @'
(async () => {
  const visible = (selector) => {
    const element = document.querySelector(selector);
    if (!(element instanceof Element)) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  };
  const rect = (selector) => {
    const element = document.querySelector(selector);
    if (!(element instanceof Element)) return { x: 0, y: 0, w: 0, h: 0, right: 0, bottom: 0 };
    const box = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    if (box.width <= 0 || box.height <= 0 || style.display === 'none' || style.visibility === 'hidden') {
      return { x: 0, y: 0, w: 0, h: 0, right: 0, bottom: 0 };
    }
    return {
      x: Math.round(box.x), y: Math.round(box.y), w: Math.round(box.width), h: Math.round(box.height),
      right: Math.round(box.right), bottom: Math.round(box.bottom),
    };
  };
  const paneMode = document.querySelector('.app')?.getAttribute('data-pane-window-mode') ?? '';
  const labels = await window.__TAURI_INTERNALS__.invoke('plugin:window|get_all_windows');
  const reports = paneMode === 'main'
    ? await window.__TAURI_INTERNALS__.invoke('capture_pane_window_placements')
    : null;
  const timelineVisible = [...document.querySelectorAll('.timelineShowSurface')].filter((element) => {
    const box = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return box.width > 0 && box.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  });
  return JSON.stringify({
    url: window.location.href,
    paneMode,
    labels,
    reports,
    root: rect('[data-workspace-split-root="true"]'),
    band: rect('.mappingPersistentWorkspaceBand'),
    context: rect('[data-workspace-pane="lower-right"]'),
    lowerLeft: rect('[data-workspace-pane="lower-left"]'),
    timelineHost: rect('[data-timeline-arranger-upper]'),
    stageVisible: visible('[data-persistent-band-part="stage"]'),
    groupsVisible: visible('[data-persistent-band-part="groups"]'),
    sourceVisible: visible('[data-timeline-source-shelf]'),
    timelineVisible: timelineVisible.length > 0,
    timelineCount: timelineVisible.length,
    appScrollFree: (() => {
      const doc = document.documentElement;
      const body = document.body;
      const app = document.querySelector('.app');
      return window.scrollX === 0 && window.scrollY === 0
        && doc.scrollWidth === doc.clientWidth && doc.scrollHeight === doc.clientHeight
        && body.scrollWidth === doc.clientWidth && body.scrollHeight === doc.clientHeight
        && (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight));
    })(),
  });
})()
'@
  try {
    return $raw | ConvertFrom-Json -ErrorAction Stop
  } catch {
    throw "Native pane DOM state was not valid JSON: $raw"
  }
}

function Test-RectFills {
  param(
    [Parameter(Mandatory = $true)]$Outer,
    [Parameter(Mandatory = $true)]$Inner,
    [ValidateRange(0, 32)][int]$Tolerance = 4
  )

  $Outer.w -gt 0 -and $Outer.h -gt 0 -and $Inner.w -gt 0 -and $Inner.h -gt 0 -and
    $Inner.x -le ($Outer.x + $Tolerance) -and $Inner.y -le ($Outer.y + $Tolerance) -and
    $Inner.right -ge ($Outer.right - $Tolerance) -and $Inner.bottom -ge ($Outer.bottom - $Tolerance)
}

function Test-NativePaneCensus {
  param(
    [Parameter(Mandatory = $true)]$State,
    # Deliberately not Mandatory: PowerShell rejects an empty-array argument
    # bound to a Mandatory [string[]], and "no detached panes" is exactly the
    # integrated-baseline census this gate must assert.
    [AllowEmptyCollection()][string[]]$ExpectedLabels,
    [AllowEmptyCollection()][string[]]$ExpectedReportPanes
  )
  if ($null -eq $ExpectedLabels) { $ExpectedLabels = @() }
  if ($null -eq $ExpectedReportPanes) { $ExpectedReportPanes = @() }

  $reportEntries = @($State.reports | Where-Object { $null -ne $_ })
  $actualLabels = @($State.labels | ForEach-Object { [string]$_ } | Sort-Object)
  $expectedSortedLabels = @($ExpectedLabels | Sort-Object)
  $actualReportPanes = @($reportEntries | ForEach-Object { [string]$_.pane } | Sort-Object)
  $expectedSortedReports = @($ExpectedReportPanes | Sort-Object)
  $reportsCarryExactIdentity = @($reportEntries | Where-Object {
    [string]::IsNullOrWhiteSpace([string]$_.instance_id)
  }).Count -eq 0
  $labelsMatch = $actualLabels.Count -eq $expectedSortedLabels.Count -and
    -not (Compare-Object -ReferenceObject $expectedSortedLabels -DifferenceObject $actualLabels)
  $reportsMatch = $actualReportPanes.Count -eq $expectedSortedReports.Count -and
    -not (Compare-Object -ReferenceObject $expectedSortedReports -DifferenceObject $actualReportPanes)
  [pscustomobject]@{
    Passed = $labelsMatch -and $reportsMatch -and $reportsCarryExactIdentity
    LabelsMatch = $labelsMatch
    ReportsMatch = $reportsMatch
    ReportsCarryExactIdentity = $reportsCarryExactIdentity
    ActualLabels = $actualLabels
    ActualReportPanes = $actualReportPanes
  }
}

function Assert-NativePaneState {
  param(
    [Parameter(Mandatory = $true)]$State,
    [Parameter(Mandatory = $true)][string]$Description,
    [Parameter(Mandatory = $true)][ValidateSet("main", "stage", "timeline")][string]$ExpectedMode,
    [Parameter(Mandatory = $true)][string[]]$ExpectedLabels,
    [Parameter(Mandatory = $true)][string[]]$ExpectedReportPanes,
    [Parameter(Mandatory = $true)][scriptblock]$AdditionalContract
  )

  $census = Test-NativePaneCensus -State $State -ExpectedLabels $ExpectedLabels -ExpectedReportPanes $ExpectedReportPanes
  $contractOk = & $AdditionalContract $State
  if ($State.paneMode -ne $ExpectedMode -or -not $census.Passed -or -not $contractOk) {
    $serialized = $State | ConvertTo-Json -Depth 8 -Compress
    throw "Native pane contract '$Description' failed. expected mode=$ExpectedMode labels=$($ExpectedLabels -join ',') reports=$($ExpectedReportPanes -join ','); actual=$serialized"
  }
}

function Wait-ForNativePaneDomState {
  param(
    [Parameter(Mandatory = $true)]$Page,
    [Parameter(Mandatory = $true)][string]$Description,
    [Parameter(Mandatory = $true)][scriptblock]$Predicate,
    [ValidateRange(1, 120)][int]$TimeoutSeconds = 30
  )

  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $lastState = $null
  while ([DateTime]::UtcNow -lt $deadline) {
    try {
      $lastState = Get-NativePaneDomState -Page $Page
      if (& $Predicate $lastState) {
        return $lastState
      }
    } catch [SyndocalCdpTrustViolation] {
      # A raced/foreign CDP endpoint is a trust takeover, not a nuisance to
      # poll through: rethrow immediately instead of burning the full timeout.
      throw
    } catch {
      $lastState = $_.Exception.Message
    }
    Start-Sleep -Milliseconds 250
  }
  $serialized = if ($lastState -is [string]) { $lastState } elseif ($null -eq $lastState) {
    "no DOM state"
  } else {
    $lastState | ConvertTo-Json -Depth 8 -Compress
  }
  throw "Timed out waiting for native pane DOM state '$Description'. Last state: $serialized"
}

function Open-NativeWorkspacePaneMenu {
  param([Parameter(Mandatory = $true)]$MainPage)

  $isOpen = Invoke-CdpRuntimeEvaluate -Page $MainPage -Expression @'
(() => {
  const element = document.querySelector('[data-workspace-pane-toggle="stage"]');
  if (!(element instanceof HTMLElement)) return false;
  const rect = element.getBoundingClientRect();
  const style = getComputedStyle(element);
  return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
})()
'@
  if ($isOpen -ne $true -and $isOpen -ne "true") {
    Invoke-CdpVisibleDomClick -Page $MainPage -Selector ".workspaceOperationsButton" -Description "open Workspaces pane menu"
  }
  Wait-ForCdpCondition -Page $MainPage -Description "visible Workspaces pane controls" -Expression @'
(() => {
  const element = document.querySelector('[data-workspace-pane-toggle="stage"]');
  if (!(element instanceof HTMLElement)) return false;
  const rect = element.getBoundingClientRect();
  const style = getComputedStyle(element);
  return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
})()
'@
}

function Maximize-VerifiedQaWindow {
  param(
    [Parameter(Mandatory = $true)][IntPtr]$Handle,
    [Parameter(Mandatory = $true)][uint32]$ProcessId,
    [Parameter(Mandatory = $true)][string]$Title,
    [Parameter(Mandatory = $true)]$Minimum
  )

  [uint32]$actualProcessId = 0
  [void][SyndocalNativeWindow]::GetWindowThreadProcessId($Handle, [ref]$actualProcessId)
  if ($actualProcessId -ne $ProcessId -or (Get-WindowTitle -Handle $Handle) -ne $Title) {
    throw "Refusing to maximize an unverified native pane window. Expected '$Title' in PID $ProcessId."
  }
  [void][SyndocalNativeWindow]::ShowWindowAsync($Handle, 3)
  $dimensions = Wait-ForMinimumClientDimensions -Handle $Handle -Minimum $Minimum
  if (-not [SyndocalNativeWindow]::IsZoomed($Handle)) {
    throw "Verified '$Title' reached $($dimensions.Width)x$($dimensions.Height) but Windows did not report it maximized."
  }
  return $dimensions
}

function Get-NativePaneWindowTitle {
  # Must stay byte-identical to backend `pane_window_title` (main.rs); the
  # self-test pins both stage and timeline spellings.
  param([Parameter(Mandatory = $true)][ValidateSet("stage", "timeline")][string]$PaneKind)

  if ($PaneKind -eq "stage") {
    "Syndocal Stage - 2D Map"
  } else {
    "Syndocal Timeline"
  }
}

function Get-NativePaneMainExpectation {
  param([Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$DetachedPanes)

  $detached = @($DetachedPanes | Sort-Object -Unique)
  foreach ($pane in $detached) {
    if ($pane -ne "stage" -and $pane -ne "timeline") {
      throw "Unknown detached pane '$pane'; native main-space expectations cover only stage and timeline."
    }
  }
  $stageDetached = $detached -contains "stage"
  $timelineDetached = $detached -contains "timeline"
  $expectedLabels = [Collections.Generic.List[string]]::new()
  $expectedLabels.Add("main")
  if ($stageDetached) { $expectedLabels.Add("pane-stage") }
  if ($timelineDetached) { $expectedLabels.Add("pane-timeline") }
  $geometryKind = "integrated-columns-present"
  if ($stageDetached -and $timelineDetached) {
    $geometryKind = "workspace-fills-root"
  } elseif ($stageDetached) {
    $geometryKind = "context-fills-band"
  }
  $timelineCount = 1
  if ($timelineDetached) { $timelineCount = 0 }
  [pscustomobject]@{
    DetachedPanes = $detached
    ExpectedLabels = @($expectedLabels)
    ExpectedReportPanes = $detached
    StageVisible = (-not $stageDetached)
    GroupsVisible = (-not $stageDetached)
    SourceVisible = $true
    TimelineVisible = (-not $timelineDetached)
    TimelineCount = $timelineCount
    GeometryKind = $geometryKind
  }
}

function Get-NativePaneChildExpectation {
  param(
    [Parameter(Mandatory = $true)][ValidateSet("stage", "timeline")][string]$PaneKind,
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$AllDetachedPanes
  )

  $detached = @($AllDetachedPanes | Sort-Object -Unique)
  if ($detached -notcontains $PaneKind) {
    throw "Child expectation for '$PaneKind' requires '$PaneKind' among the detached panes."
  }
  $expectedLabels = [Collections.Generic.List[string]]::new()
  $expectedLabels.Add("main")
  foreach ($pane in $detached) {
    $expectedLabels.Add("pane-$pane")
  }
  [pscustomobject]@{
    PaneKind = $PaneKind
    ExpectedLabels = @($expectedLabels)
    UrlPattern = "[?&]syndocalPaneWindow=" + $PaneKind + "(?:&|$)"
  }
}

function Test-NativePaneReportsHaveNoPendingClose {
  param([Parameter(Mandatory = $true)]$State)

  foreach ($entry in @($State.reports)) {
    if ($null -eq $entry) { continue }
    $property = $entry.PSObject.Properties["pending_close_request_id"]
    if ($null -ne $property -and $null -ne $property.Value) { return $false }
  }
  return $true
}

function Test-NativePaneMainContract {
  param(
    [Parameter(Mandatory = $true)]$State,
    [Parameter(Mandatory = $true)]$Expectation
  )

  if ($State.paneMode -ne "main") { return $false }
  if ([bool]$State.stageVisible -ne [bool]$Expectation.StageVisible) { return $false }
  if ([bool]$State.groupsVisible -ne [bool]$Expectation.GroupsVisible) { return $false }
  if ([bool]$State.sourceVisible -ne [bool]$Expectation.SourceVisible) { return $false }
  if ([bool]$State.timelineVisible -ne [bool]$Expectation.TimelineVisible) { return $false }
  if ([int]$State.timelineCount -ne [int]$Expectation.TimelineCount) { return $false }
  switch ($Expectation.GeometryKind) {
    "context-fills-band" {
      if (-not (Test-RectFills -Outer $State.band -Inner $State.context)) { return $false }
    }
    "workspace-fills-root" {
      if (-not (Test-RectFills -Outer $State.root -Inner $State.band)) { return $false }
      if (-not (Test-RectFills -Outer $State.root -Inner $State.context)) { return $false }
    }
    "integrated-columns-present" {
      if ([int]$State.lowerLeft.w -le 0 -or [int]$State.context.w -le 0) { return $false }
    }
    default { return $false }
  }
  if (-not [bool]$State.appScrollFree) { return $false }
  $census = Test-NativePaneCensus -State $State `
    -ExpectedLabels $Expectation.ExpectedLabels `
    -ExpectedReportPanes $Expectation.ExpectedReportPanes
  if (-not $census.Passed) { return $false }
  return (Test-NativePaneReportsHaveNoPendingClose -State $State)
}

function Test-NativePaneChildContract {
  param(
    [Parameter(Mandatory = $true)]$State,
    [Parameter(Mandatory = $true)]$Expectation
  )

  if ($State.paneMode -ne $Expectation.PaneKind) { return $false }
  if ($State.url -notmatch $Expectation.UrlPattern) { return $false }
  if (-not [bool]$State.appScrollFree) { return $false }
  if ($Expectation.PaneKind -eq "stage") {
    if (-not [bool]$State.stageVisible) { return $false }
    if (-not [bool]$State.groupsVisible) { return $false }
    if ([bool]$State.sourceVisible) { return $false }
    if ([bool]$State.timelineVisible) { return $false }
    if ([int]$State.timelineCount -ne 0) { return $false }
  } else {
    if ([bool]$State.stageVisible) { return $false }
    if ([bool]$State.groupsVisible) { return $false }
    if ([bool]$State.sourceVisible) { return $false }
    if (-not [bool]$State.timelineVisible) { return $false }
    if ([int]$State.timelineCount -ne 1) { return $false }
    if ([int]$State.band.w -ne 0) { return $false }
  }
  $census = Test-NativePaneCensus -State $State `
    -ExpectedLabels $Expectation.ExpectedLabels -ExpectedReportPanes @()
  return [bool]$census.Passed
}

function Test-ExactStringCensus {
  param(
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$Actual,
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$Expected
  )

  $actualSorted = @($Actual | Sort-Object)
  $expectedSorted = @($Expected | Sort-Object)
  if ($actualSorted.Count -ne $expectedSorted.Count) { return $false }
  return $null -eq (Compare-Object -ReferenceObject $expectedSorted -DifferenceObject $actualSorted)
}

function Get-OwnedVisibleWindowTitleCensus {
  # Exact visible top-level window-label census for one verified PID. This is
  # complementary to the in-page Tauri label census: it catches duplicate or
  # unexpected native windows that a label-only census cannot see.
  param([Parameter(Mandatory = $true)][uint32]$ProcessId)

  $script:syndocalOwnedWindowTitles = [Collections.Generic.List[string]]::new()
  $callback = [SyndocalEnumWindowsProc]{
    param([IntPtr]$Handle, [IntPtr]$Unused)
    if ([SyndocalNativeWindow]::IsWindowVisible($Handle)) {
      [uint32]$ownerProcessId = 0
      [void][SyndocalNativeWindow]::GetWindowThreadProcessId($Handle, [ref]$ownerProcessId)
      if ($ownerProcessId -eq $ProcessId) {
        $title = Get-WindowTitle -Handle $Handle
        if (-not [string]::IsNullOrEmpty($title)) {
          $script:syndocalOwnedWindowTitles.Add($title)
        }
      }
    }
    return $true
  }
  [void][SyndocalNativeWindow]::EnumWindows($callback, [IntPtr]::Zero)
  @($script:syndocalOwnedWindowTitles)
}

function Assert-NativeWindowTitleCensus {
  param(
    [Parameter(Mandatory = $true)][uint32]$ProcessId,
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$ExpectedTitles,
    [Parameter(Mandatory = $true)][string]$Description
  )

  $actualTitles = Get-OwnedVisibleWindowTitleCensus -ProcessId $ProcessId
  if (-not (Test-ExactStringCensus -Actual $actualTitles -Expected $ExpectedTitles)) {
    throw (
      "Native window-title census '$Description' failed. expected=[$($ExpectedTitles -join ', ')] " +
      "actual=[$($actualTitles -join ', ')]"
    )
  }
}

function Close-VerifiedOwnedNativeWindow {
  # Same owned-window policy as the script's existing finally cleanup: a real
  # WM_CLOSE goes only to an HWND whose exact title and owning PID were just
  # re-verified, never to another checkout's or another app's window.
  param(
    [Parameter(Mandatory = $true)][IntPtr]$Handle,
    [Parameter(Mandatory = $true)][uint32]$ProcessId,
    [Parameter(Mandatory = $true)][string]$Title,
    [ValidateRange(1, 120)][int]$TimeoutSeconds = 20
  )

  [uint32]$actualProcessId = 0
  [void][SyndocalNativeWindow]::GetWindowThreadProcessId($Handle, [ref]$actualProcessId)
  if ($actualProcessId -ne $ProcessId -or (Get-WindowTitle -Handle $Handle) -ne $Title) {
    throw "Refusing to close an unverified native window. Expected '$Title' in PID $ProcessId."
  }
  [void][SyndocalNativeWindow]::PostMessage($Handle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
  Wait-ForWindowToCloseByTitleAndProcess -Title $Title -ProcessId $ProcessId -TimeoutSeconds $TimeoutSeconds
}

function Invoke-NativeTimelineDeskShowSelection {
  param([Parameter(Mandatory = $true)]$TimelinePage)

  Wait-ForCdpCondition -Page $TimelinePage -Description "detached Timeline desk selector" -Expression @'
(() => document.querySelector('[data-timeline-desk-surface="show"]') instanceof HTMLElement)()
'@
  Invoke-CdpVisibleDomClick -Page $TimelinePage `
    -Selector '[data-timeline-desk-surface="show"]' `
    -Description "select detached Timeline Show desk"
}

function Invoke-NativePaneDetachStep {
  param(
    [Parameter(Mandatory = $true)]$MainPage,
    [Parameter(Mandatory = $true)][IntPtr]$MainWindow,
    [Parameter(Mandatory = $true)][uint32]$ProcessId,
    [Parameter(Mandatory = $true)][int]$Port,
    [Parameter(Mandatory = $true)]$Minimum,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [Parameter(Mandatory = $true)][ValidateSet("stage", "timeline")][string]$PaneKind,
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$AlreadyDetachedPanes,
    [Parameter(Mandatory = $true)][int]$EvidenceIndex,
    [Parameter(Mandatory = $true)][string]$EvidenceSlug
  )

  Open-NativeWorkspacePaneMenu -MainPage $MainPage
  Invoke-CdpVisibleDomClick -Page $MainPage `
    -Selector ('[data-workspace-pane-toggle="{0}"]' -f $PaneKind) `
    -Description ("detach {0}" -f $PaneKind)
  $paneTitle = Get-NativePaneWindowTitle -PaneKind $PaneKind
  $paneWindow = Wait-ForWindowByTitleAndProcess -Title $paneTitle -ProcessId $ProcessId
  $paneMaximized = Maximize-VerifiedQaWindow -Handle $paneWindow -ProcessId $ProcessId `
    -Title $paneTitle -Minimum $Minimum
  $panePage = Wait-ForCdpAppPage -Port $Port -QaProcessId $ProcessId -ExpectedMode $PaneKind
  if ($PaneKind -eq "timeline") {
    Invoke-NativeTimelineDeskShowSelection -TimelinePage $panePage
  }
  $allDetached = @(@($AlreadyDetachedPanes) + @($PaneKind))
  $childExpectation = Get-NativePaneChildExpectation -PaneKind $PaneKind -AllDetachedPanes $allDetached
  $childState = Wait-ForNativePaneDomState -Page $panePage `
    -Description ("{0} child content" -f $PaneKind) -Predicate {
      param($state)
      Test-NativePaneChildContract -State $state -Expectation $childExpectation
    }
  Assert-NativePaneState -State $childState -Description ("{0} child pane contract" -f $PaneKind) `
    -ExpectedMode $PaneKind -ExpectedLabels $childExpectation.ExpectedLabels -ExpectedReportPanes @() `
    -AdditionalContract { param($state) Test-NativePaneChildContract -State $state -Expectation $childExpectation }
  $childScreenshot = Join-Path $OutputDirectory ("{0:D2}-{1}-child-maximized.png" -f $EvidenceIndex, $EvidenceSlug)
  $childVisual = Save-VerifiedClientScreenshot -Handle $paneWindow -Path $childScreenshot `
    -Stage ("{0} pane child" -f $PaneKind)

  $mainExpectation = Get-NativePaneMainExpectation -DetachedPanes $allDetached
  $mainState = Wait-ForNativePaneDomState -Page $MainPage `
    -Description ("main reflow after {0} detach" -f $PaneKind) -Predicate {
      param($state)
      Test-NativePaneMainContract -State $state -Expectation $mainExpectation
    }
  Assert-NativePaneState -State $mainState -Description ("main after {0} detach" -f $PaneKind) `
    -ExpectedMode "main" -ExpectedLabels $mainExpectation.ExpectedLabels `
    -ExpectedReportPanes $mainExpectation.ExpectedReportPanes `
    -AdditionalContract { param($state) Test-NativePaneMainContract -State $state -Expectation $mainExpectation }
  $mainScreenshot = Join-Path $OutputDirectory ("{0:D2}-main-after-{1}-detach.png" -f ($EvidenceIndex + 1), $EvidenceSlug)
  $mainVisual = Save-VerifiedClientScreenshot -Handle $MainWindow -Path $mainScreenshot `
    -Stage ("main after {0} detach" -f $PaneKind)

  [pscustomobject]@{
    pane_kind = $PaneKind
    pane_title = $paneTitle
    maximized_client = $paneMaximized
    child_dom = $childState
    child_visual = $childVisual
    main_dom = $mainState
    main_visual = $mainVisual
    screenshots = @($childScreenshot, $mainScreenshot)
  }
}

function Invoke-NativePaneRejoinStep {
  param(
    [Parameter(Mandatory = $true)]$MainPage,
    [Parameter(Mandatory = $true)][IntPtr]$MainWindow,
    [Parameter(Mandatory = $true)][uint32]$ProcessId,
    [Parameter(Mandatory = $true)][ValidateSet("stage", "timeline")][string]$PaneKind,
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$RemainingDetachedPanes,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [Parameter(Mandatory = $true)][int]$EvidenceIndex,
    [Parameter(Mandatory = $true)][string]$EvidenceSlug
  )

  $paneTitle = Get-NativePaneWindowTitle -PaneKind $PaneKind
  Open-NativeWorkspacePaneMenu -MainPage $MainPage
  Invoke-CdpVisibleDomClick -Page $MainPage `
    -Selector ('[data-workspace-pane-toggle="{0}"]' -f $PaneKind) `
    -Description ("reintegrate {0}" -f $PaneKind)
  Wait-ForWindowToCloseByTitleAndProcess -Title $paneTitle -ProcessId $ProcessId
  $mainExpectation = Get-NativePaneMainExpectation -DetachedPanes $RemainingDetachedPanes
  $mainState = Wait-ForNativePaneDomState -Page $MainPage `
    -Description ("main reflow after {0} reintegration" -f $PaneKind) -Predicate {
      param($state)
      Test-NativePaneMainContract -State $state -Expectation $mainExpectation
    }
  Assert-NativePaneState -State $mainState -Description ("main after {0} reintegration" -f $PaneKind) `
    -ExpectedMode "main" -ExpectedLabels $mainExpectation.ExpectedLabels `
    -ExpectedReportPanes $mainExpectation.ExpectedReportPanes `
    -AdditionalContract { param($state) Test-NativePaneMainContract -State $state -Expectation $mainExpectation }
  $mainScreenshot = Join-Path $OutputDirectory ("{0:D2}-main-after-{1}.png" -f $EvidenceIndex, $EvidenceSlug)
  $mainVisual = Save-VerifiedClientScreenshot -Handle $MainWindow -Path $mainScreenshot `
    -Stage ("main after {0} reintegration" -f $PaneKind)

  [pscustomobject]@{
    pane_kind = $PaneKind
    main_dom = $mainState
    main_visual = $mainVisual
    screenshots = @($mainScreenshot)
  }
}

function Invoke-TimelineFirstStageSecondLifecycleAcceptance {
  # Second detach order for SHOW-P0-3: Timeline detaches first while Stage stays
  # integrated, then Stage detaches while Timeline is already its own window.
  # Every step carries the exact Tauri label/report census, the exact visible
  # native window-title census, and main-space reflow assertions.
  param(
    [Parameter(Mandatory = $true)]$MainPage,
    [Parameter(Mandatory = $true)][IntPtr]$MainWindow,
    [Parameter(Mandatory = $true)][uint32]$ProcessId,
    [Parameter(Mandatory = $true)]$Minimum,
    [Parameter(Mandatory = $true)][int]$Port,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
  )

  $baselineExpectation = Get-NativePaneMainExpectation -DetachedPanes @()
  $baseline = Wait-ForNativePaneDomState -Page $MainPage `
    -Description "order-B integrated baseline" -Predicate {
      param($state)
      Test-NativePaneMainContract -State $state -Expectation $baselineExpectation
    }
  Assert-NativePaneState -State $baseline -Description "order-B integrated baseline" -ExpectedMode "main" `
    -ExpectedLabels $baselineExpectation.ExpectedLabels `
    -ExpectedReportPanes $baselineExpectation.ExpectedReportPanes `
    -AdditionalContract { param($state) Test-NativePaneMainContract -State $state -Expectation $baselineExpectation }

  $firstDetach = Invoke-NativePaneDetachStep -MainPage $MainPage -MainWindow $MainWindow `
    -ProcessId $ProcessId -Port $Port -Minimum $Minimum -OutputDirectory $OutputDirectory `
    -PaneKind "timeline" -AlreadyDetachedPanes @() -EvidenceIndex 10 -EvidenceSlug "order-b-timeline-first"
  Assert-NativeWindowTitleCensus -ProcessId $ProcessId `
    -ExpectedTitles @($qaTitle, $firstDetach.pane_title) `
    -Description "after Timeline-first detach"

  $secondDetach = Invoke-NativePaneDetachStep -MainPage $MainPage -MainWindow $MainWindow `
    -ProcessId $ProcessId -Port $Port -Minimum $Minimum -OutputDirectory $OutputDirectory `
    -PaneKind "stage" -AlreadyDetachedPanes @("timeline") -EvidenceIndex 12 -EvidenceSlug "order-b-stage-second"
  Assert-NativeWindowTitleCensus -ProcessId $ProcessId `
    -ExpectedTitles @($qaTitle, $firstDetach.pane_title, $secondDetach.pane_title) `
    -Description "after Timeline-then-Stage detach"

  [pscustomobject]@{
    baseline = $baseline
    first_detach_timeline = $firstDetach
    second_detach_stage = $secondDetach
    screenshots = @($firstDetach.screenshots) + @($secondDetach.screenshots)
  }
}

function Invoke-NativePaneLifecycleAcceptance {
  param(
    [Parameter(Mandatory = $true)]$MainPage,
    [Parameter(Mandatory = $true)][IntPtr]$MainWindow,
    [Parameter(Mandatory = $true)][uint32]$ProcessId,
    [Parameter(Mandatory = $true)]$Minimum,
    [Parameter(Mandatory = $true)][int]$Port,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
  )

  # This is intentionally real-Tauri-only. The CDP channel is attached to the
  # QA WebView2 instances, while each action remains a real click on the
  # shipped DOM and each child is independently proven by its HWND, process,
  # title, Tauri label census, route, and rendered content.
  Wait-ForCdpCondition -Page $MainPage -Description "main workspace controls" -Expression @'
(() => document.querySelector('.app')?.getAttribute('data-pane-window-mode') === 'main'
  && document.querySelector('[data-workspace-option="control"]') instanceof HTMLElement)()
'@
  Invoke-CdpVisibleDomClick -Page $MainPage -Selector '[data-workspace-option="control"]' -Description "select Control workspace"
  Wait-ForCdpCondition -Page $MainPage -Description "Control mode chooser" -Expression @'
(() => document.querySelector('[data-control-mode-option="live"]') instanceof HTMLElement)()
'@
  Invoke-CdpVisibleDomClick -Page $MainPage -Selector '[data-control-mode-option="live"]' -Description "select Live control mode"
  Wait-ForCdpCondition -Page $MainPage -Description "integrated Timeline and Source shelf" -Expression @'
(() => {
  const visible = (selector) => {
    const element = document.querySelector(selector);
    if (!(element instanceof Element)) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  };
  return document.querySelector('.layout.layoutControl.controlModeLive') instanceof HTMLElement
    && visible('[data-timeline-arranger-upper]')
    && visible('[data-timeline-source-shelf]');
})()
'@
  Invoke-CdpVisibleDomClick -Page $MainPage -Selector '[data-timeline-desk-surface="show"]' -Description "select Timeline Show desk"

  $baseline = Wait-ForNativePaneDomState -Page $MainPage -Description "integrated baseline" -Predicate {
    param($state)
    $state.paneMode -eq "main" -and $state.stageVisible -and $state.groupsVisible -and
      $state.sourceVisible -and $state.timelineVisible -and $state.timelineCount -eq 1 -and
      $state.appScrollFree -and
      (Test-NativePaneCensus -State $state -ExpectedLabels @("main") -ExpectedReportPanes @()).Passed
  }
  Assert-NativePaneState -State $baseline -Description "integrated baseline" -ExpectedMode "main" `
    -ExpectedLabels @("main") -ExpectedReportPanes @() -AdditionalContract {
      param($state)
      $state.stageVisible -and $state.groupsVisible -and $state.sourceVisible -and
        $state.timelineVisible -and $state.timelineCount -eq 1 -and $state.appScrollFree
    }

  Open-NativeWorkspacePaneMenu -MainPage $MainPage
  Invoke-CdpVisibleDomClick -Page $MainPage -Selector '[data-workspace-pane-toggle="stage"]' -Description "detach Stage"
  $stageWindow = Wait-ForWindowByTitleAndProcess -Title "Syndocal Stage - 2D Map" -ProcessId $ProcessId
  $stageMaximized = Maximize-VerifiedQaWindow -Handle $stageWindow -ProcessId $ProcessId `
    -Title "Syndocal Stage - 2D Map" -Minimum $Minimum
  $stagePage = Wait-ForCdpAppPage -Port $Port -QaProcessId $ProcessId -ExpectedMode "stage"
  $stageChild = Wait-ForNativePaneDomState -Page $stagePage -Description "Stage child content" -Predicate {
    param($state)
    $state.paneMode -eq "stage" -and $state.stageVisible -and $state.groupsVisible -and
      -not $state.sourceVisible -and -not $state.timelineVisible -and $state.timelineCount -eq 0 -and
      $state.appScrollFree -and
      (Test-NativePaneCensus -State $state -ExpectedLabels @("main", "pane-stage") -ExpectedReportPanes @()).Passed
  }
  Assert-NativePaneState -State $stageChild -Description "Stage child pane-stage" -ExpectedMode "stage" `
    -ExpectedLabels @("main", "pane-stage") -ExpectedReportPanes @() -AdditionalContract {
      param($state)
      $state.url -match '[?&]syndocalPaneWindow=stage(?:&|$)' -and
        $state.stageVisible -and $state.groupsVisible -and -not $state.sourceVisible -and
        -not $state.timelineVisible -and $state.timelineCount -eq 0 -and $state.appScrollFree
    }
  $stageChildScreenshot = Join-Path $OutputDirectory "04-pane-stage-maximized.png"
  $stageChildVisual = Save-VerifiedClientScreenshot -Handle $stageWindow -Path $stageChildScreenshot -Stage "Stage pane child"

  $mainStageDetached = Wait-ForNativePaneDomState -Page $MainPage -Description "main Stage-detached reflow" -Predicate {
    param($state)
    $state.paneMode -eq "main" -and -not $state.stageVisible -and -not $state.groupsVisible -and
      $state.sourceVisible -and $state.timelineVisible -and $state.timelineCount -eq 1 -and
      (Test-RectFills -Outer $state.band -Inner $state.context) -and $state.appScrollFree -and
      (Test-NativePaneCensus -State $state -ExpectedLabels @("main", "pane-stage") -ExpectedReportPanes @("stage")).Passed
  }
  Assert-NativePaneState -State $mainStageDetached -Description "main after Stage detach" -ExpectedMode "main" `
    -ExpectedLabels @("main", "pane-stage") -ExpectedReportPanes @("stage") -AdditionalContract {
      param($state)
      -not $state.stageVisible -and -not $state.groupsVisible -and $state.sourceVisible -and
        $state.timelineVisible -and $state.timelineCount -eq 1 -and
        (Test-RectFills -Outer $state.band -Inner $state.context) -and $state.appScrollFree
    }
  $mainStageDetachedScreenshot = Join-Path $OutputDirectory "05-main-stage-detached-source-fills-band.png"
  $mainStageDetachedVisual = Save-VerifiedClientScreenshot -Handle $MainWindow -Path $mainStageDetachedScreenshot -Stage "Main after Stage detach"

  Open-NativeWorkspacePaneMenu -MainPage $MainPage
  Invoke-CdpVisibleDomClick -Page $MainPage -Selector '[data-workspace-pane-toggle="timeline"]' -Description "detach Timeline"
  $timelineWindow = Wait-ForWindowByTitleAndProcess -Title "Syndocal Timeline" -ProcessId $ProcessId
  $timelineMaximized = Maximize-VerifiedQaWindow -Handle $timelineWindow -ProcessId $ProcessId `
    -Title "Syndocal Timeline" -Minimum $Minimum
  $timelinePage = Wait-ForCdpAppPage -Port $Port -QaProcessId $ProcessId -ExpectedMode "timeline"
  Wait-ForCdpCondition -Page $timelinePage -Description "Timeline child desk selector" -Expression @'
(() => document.querySelector('[data-timeline-desk-surface="show"]') instanceof HTMLElement)()
'@
  Invoke-CdpVisibleDomClick -Page $timelinePage -Selector '[data-timeline-desk-surface="show"]' -Description "select Timeline child Show desk"
  $timelineChild = Wait-ForNativePaneDomState -Page $timelinePage -Description "Timeline child content" -Predicate {
    param($state)
    $state.paneMode -eq "timeline" -and -not $state.stageVisible -and -not $state.groupsVisible -and
      -not $state.sourceVisible -and $state.timelineVisible -and $state.timelineCount -eq 1 -and
      $state.band.w -eq 0 -and $state.appScrollFree -and
      (Test-NativePaneCensus -State $state -ExpectedLabels @("main", "pane-stage", "pane-timeline") -ExpectedReportPanes @()).Passed
  }
  Assert-NativePaneState -State $timelineChild -Description "Timeline child pane-timeline" -ExpectedMode "timeline" `
    -ExpectedLabels @("main", "pane-stage", "pane-timeline") -ExpectedReportPanes @() -AdditionalContract {
      param($state)
      $state.url -match '[?&]syndocalPaneWindow=timeline(?:&|$)' -and
        -not $state.stageVisible -and -not $state.groupsVisible -and -not $state.sourceVisible -and
        $state.timelineVisible -and $state.timelineCount -eq 1 -and $state.band.w -eq 0 -and
        $state.appScrollFree
    }
  $timelineChildScreenshot = Join-Path $OutputDirectory "06-pane-timeline-maximized-single-arranger.png"
  $timelineChildVisual = Save-VerifiedClientScreenshot -Handle $timelineWindow -Path $timelineChildScreenshot -Stage "Timeline pane child"

  $mainBothDetached = Wait-ForNativePaneDomState -Page $MainPage -Description "main both-detached Source fill" -Predicate {
    param($state)
    $state.paneMode -eq "main" -and -not $state.stageVisible -and -not $state.groupsVisible -and
      $state.sourceVisible -and -not $state.timelineVisible -and $state.timelineCount -eq 0 -and
      (Test-RectFills -Outer $state.root -Inner $state.band) -and
      (Test-RectFills -Outer $state.root -Inner $state.context) -and $state.appScrollFree -and
      (Test-NativePaneCensus -State $state -ExpectedLabels @("main", "pane-stage", "pane-timeline") -ExpectedReportPanes @("stage", "timeline")).Passed
  }
  Assert-NativePaneState -State $mainBothDetached -Description "main after Stage and Timeline detach" -ExpectedMode "main" `
    -ExpectedLabels @("main", "pane-stage", "pane-timeline") -ExpectedReportPanes @("stage", "timeline") -AdditionalContract {
      param($state)
      -not $state.stageVisible -and -not $state.groupsVisible -and $state.sourceVisible -and
        -not $state.timelineVisible -and $state.timelineCount -eq 0 -and
        (Test-RectFills -Outer $state.root -Inner $state.band) -and
        (Test-RectFills -Outer $state.root -Inner $state.context) -and $state.appScrollFree
    }
  $mainBothDetachedScreenshot = Join-Path $OutputDirectory "07-main-both-detached-source-fills-workspace.png"
  $mainBothDetachedVisual = Save-VerifiedClientScreenshot -Handle $MainWindow -Path $mainBothDetachedScreenshot -Stage "Main after both panes detach"

  Open-NativeWorkspacePaneMenu -MainPage $MainPage
  Invoke-CdpVisibleDomClick -Page $MainPage -Selector '[data-workspace-pane-toggle="stage"]' -Description "reintegrate Stage"
  Wait-ForWindowToCloseByTitleAndProcess -Title "Syndocal Stage - 2D Map" -ProcessId $ProcessId
  $mainTimelineDetached = Wait-ForNativePaneDomState -Page $MainPage -Description "main Stage reintegrated while Timeline detached" -Predicate {
    param($state)
    $state.paneMode -eq "main" -and $state.stageVisible -and $state.groupsVisible -and
      $state.sourceVisible -and -not $state.timelineVisible -and $state.timelineCount -eq 0 -and
      $state.lowerLeft.w -gt 0 -and $state.context.w -gt 0 -and $state.appScrollFree -and
      (Test-NativePaneCensus -State $state -ExpectedLabels @("main", "pane-timeline") -ExpectedReportPanes @("timeline")).Passed
  }
  Assert-NativePaneState -State $mainTimelineDetached -Description "Stage reintegrated with Timeline detached" -ExpectedMode "main" `
    -ExpectedLabels @("main", "pane-timeline") -ExpectedReportPanes @("timeline") -AdditionalContract {
      param($state)
      $state.stageVisible -and $state.groupsVisible -and $state.sourceVisible -and
        -not $state.timelineVisible -and $state.timelineCount -eq 0 -and
        $state.lowerLeft.w -gt 0 -and $state.context.w -gt 0 -and $state.appScrollFree
    }
  $mainStageReintegratedScreenshot = Join-Path $OutputDirectory "08-main-stage-reintegrated-timeline-detached.png"
  $mainStageReintegratedVisual = Save-VerifiedClientScreenshot -Handle $MainWindow -Path $mainStageReintegratedScreenshot -Stage "Main Stage reintegrated"

  Open-NativeWorkspacePaneMenu -MainPage $MainPage
  Invoke-CdpVisibleDomClick -Page $MainPage -Selector '[data-workspace-pane-toggle="timeline"]' -Description "reintegrate Timeline"
  Wait-ForWindowToCloseByTitleAndProcess -Title "Syndocal Timeline" -ProcessId $ProcessId
  $finalMain = Wait-ForNativePaneDomState -Page $MainPage -Description "fully reintegrated main" -Predicate {
    param($state)
    $state.paneMode -eq "main" -and $state.stageVisible -and $state.groupsVisible -and
      $state.sourceVisible -and $state.timelineVisible -and $state.timelineCount -eq 1 -and
      $state.lowerLeft.w -gt 0 -and $state.context.w -gt 0 -and $state.appScrollFree -and
      (Test-NativePaneCensus -State $state -ExpectedLabels @("main") -ExpectedReportPanes @()).Passed
  }
  Assert-NativePaneState -State $finalMain -Description "fully reintegrated main" -ExpectedMode "main" `
    -ExpectedLabels @("main") -ExpectedReportPanes @() -AdditionalContract {
      param($state)
      $state.stageVisible -and $state.groupsVisible -and $state.sourceVisible -and
        $state.timelineVisible -and $state.timelineCount -eq 1 -and
        $state.lowerLeft.w -gt 0 -and $state.context.w -gt 0 -and $state.appScrollFree
    }
  $finalMainScreenshot = Join-Path $OutputDirectory "09-main-fully-reintegrated.png"
  $finalMainVisual = Save-VerifiedClientScreenshot -Handle $MainWindow -Path $finalMainScreenshot -Stage "Main fully reintegrated"

  [pscustomobject]@{
    cdp_port = $Port
    baseline = $baseline
    stage_child = [ordered]@{ maximized_client = $stageMaximized; dom = $stageChild; visual = $stageChildVisual }
    main_stage_detached = [ordered]@{ dom = $mainStageDetached; visual = $mainStageDetachedVisual }
    timeline_child = [ordered]@{ maximized_client = $timelineMaximized; dom = $timelineChild; visual = $timelineChildVisual }
    main_both_detached = [ordered]@{ dom = $mainBothDetached; visual = $mainBothDetachedVisual }
    main_stage_reintegrated = [ordered]@{ dom = $mainTimelineDetached; visual = $mainStageReintegratedVisual }
    main_fully_reintegrated = [ordered]@{ dom = $finalMain; visual = $finalMainVisual }
    screenshots = @(
      $stageChildScreenshot,
      $mainStageDetachedScreenshot,
      $timelineChildScreenshot,
      $mainBothDetachedScreenshot,
      $mainStageReintegratedScreenshot,
      $finalMainScreenshot
    )
  }
}

function Stop-NativeAcceptanceDevServer {
  param([Parameter(Mandatory = $true)][DateTime]$NotBefore)

  $listeners = @(Get-NetTCPConnection -LocalPort 5187 -State Listen -ErrorAction SilentlyContinue)
  foreach ($listener in $listeners) {
    $listenerProcess = Get-Process -Id $listener.OwningProcess -ErrorAction SilentlyContinue
    if ($null -ne $listenerProcess -and $listenerProcess.StartTime.ToUniversalTime() -ge $NotBefore.AddSeconds(-2)) {
      & taskkill.exe /PID $listenerProcess.Id /T /F *> $null
    }
  }
}

function Test-FfmpegSdkRoot {
  param([Parameter(Mandatory = $true)][string]$Root)

  $missing = [Collections.Generic.List[string]]::new()
  $resolvedRoot = $null
  try {
    $resolvedRoot = (Resolve-Path -LiteralPath $Root -ErrorAction Stop).Path
  } catch {
    $missing.Add("SDK root does not exist")
  }

  if ($null -eq $resolvedRoot -or -not (Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    if ($missing.Count -eq 0) { $missing.Add("SDK root is not a directory") }
  } else {
    $includeDir = Join-Path $resolvedRoot "include"
    $libDir = Join-Path $resolvedRoot "lib"
    $binDir = Join-Path $resolvedRoot "bin"
    foreach ($directory in @(@("include", $includeDir), @("lib", $libDir), @("bin", $binDir))) {
      if (-not (Test-Path -LiteralPath $directory[1] -PathType Container)) {
        $missing.Add("$($directory[0]) directory")
      }
    }

    if (Test-Path -LiteralPath $includeDir -PathType Container) {
      foreach ($header in @(
        "libavcodec/avcodec.h",
        "libavformat/avformat.h",
        "libavutil/avutil.h",
        "libswscale/swscale.h"
      )) {
        $headerPath = Join-Path $includeDir $header
        $headerItem = Get-Item -LiteralPath $headerPath -ErrorAction SilentlyContinue
        if ($null -eq $headerItem -or $headerItem.Length -le 0) {
          $missing.Add("header $header")
        }
      }
    }

    if (Test-Path -LiteralPath $libDir -PathType Container) {
      foreach ($library in @("avcodec", "avformat", "avutil", "swscale")) {
        $importLibrary = Join-Path $libDir "$library.lib"
        $importLibraryItem = Get-Item -LiteralPath $importLibrary -ErrorAction SilentlyContinue
        if ($null -eq $importLibraryItem -or $importLibraryItem.Length -le 0) {
          $missing.Add("import library $library.lib")
        }
      }
    }

    if (Test-Path -LiteralPath $binDir -PathType Container) {
      foreach ($library in @("avcodec", "avformat", "avutil", "swscale")) {
        $definitionFiles = @(Get-ChildItem -LiteralPath $libDir -File -Filter "$library-*.def" -ErrorAction SilentlyContinue)
        $expectedRuntimeNames = @($definitionFiles | ForEach-Object { "$($_.BaseName).dll" })
        if ($expectedRuntimeNames.Count -gt 0) {
          $runtimeDescription = $expectedRuntimeNames -join " or "
          $runtimeCandidates = @($expectedRuntimeNames | ForEach-Object {
            Get-Item -LiteralPath (Join-Path $binDir $_) -ErrorAction SilentlyContinue
          })
        } else {
          $runtimeDescription = "$library*.dll"
          $runtimeCandidates = @(Get-ChildItem -LiteralPath $binDir -File -Filter $runtimeDescription -ErrorAction SilentlyContinue)
        }
        $runtimeDll = @($runtimeCandidates | Where-Object { $null -ne $_ -and $_.Length -gt 0 })
        if ($runtimeDll.Count -eq 0) {
          $missing.Add("runtime DLL $runtimeDescription")
        }
      }
    }
  }

  [pscustomobject]@{
    Root = if ($null -ne $resolvedRoot) { $resolvedRoot } else { $Root }
    Valid = $missing.Count -eq 0
    Missing = @($missing)
  }
}

function Resolve-FfmpegSdkRoot {
  $explicitRoot = [Environment]::GetEnvironmentVariable("FFMPEG_DIR")
  if (-not [string]::IsNullOrWhiteSpace($explicitRoot)) {
    $validation = Test-FfmpegSdkRoot -Root $explicitRoot.Trim()
    if (-not $validation.Valid) {
      throw "FFMPEG_DIR '$explicitRoot' is incomplete or invalid. Missing: $($validation.Missing -join ', '). Required: FFmpeg headers, MSVC import libraries, and runtime DLLs for avcodec/avformat/avutil/swscale."
    }
    return $validation.Root
  }

  $candidates = [Collections.Generic.List[string]]::new()
  $localAppData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
  $wingetRoot = Join-Path $localAppData "Microsoft\WinGet\Packages"
  if (Test-Path -LiteralPath $wingetRoot -PathType Container) {
    foreach ($package in @(Get-ChildItem -LiteralPath $wingetRoot -Directory -Filter "Gyan.FFmpeg.Shared_*" -ErrorAction SilentlyContinue |
      Sort-Object -Property @{ Expression = "LastWriteTime"; Descending = $true }, @{ Expression = "Name"; Descending = $true })) {
      if ((Test-Path -LiteralPath (Join-Path $package.FullName "include") -PathType Container) -and
          (Test-Path -LiteralPath (Join-Path $package.FullName "lib") -PathType Container) -and
          (Test-Path -LiteralPath (Join-Path $package.FullName "bin") -PathType Container)) {
        $candidates.Add($package.FullName)
      }
      foreach ($sdk in @(Get-ChildItem -LiteralPath $package.FullName -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -like "ffmpeg-*-full_build-shared*" } |
        Sort-Object -Property @{ Expression = "LastWriteTime"; Descending = $true }, @{ Expression = "Name"; Descending = $true })) {
        $candidates.Add($sdk.FullName)
      }
    }
  }

  $localSdkRoot = "C:\temp\ffmpeg-n8.1-lgpl-shared"
  if (Test-Path -LiteralPath $localSdkRoot -PathType Container) {
    $candidates.Add($localSdkRoot)
    foreach ($sdk in @(Get-ChildItem -LiteralPath $localSdkRoot -Directory -Filter "ffmpeg-n8.1-*-win64-lgpl-shared*" -ErrorAction SilentlyContinue |
      Sort-Object -Property @{ Expression = "LastWriteTime"; Descending = $true }, @{ Expression = "Name"; Descending = $true })) {
      $candidates.Add($sdk.FullName)
    }
  }

  $diagnostics = [Collections.Generic.List[string]]::new()
  foreach ($candidate in @($candidates | Select-Object -Unique)) {
    $validation = Test-FfmpegSdkRoot -Root $candidate
    if ($validation.Valid) {
      return $validation.Root
    }
    $diagnostics.Add("$candidate [$($validation.Missing -join ', ')]")
  }

  $checked = if ($diagnostics.Count -eq 0) { "no candidate SDK directories" } else { $diagnostics -join "; " }
  throw "No complete shared FFmpeg SDK was found. Set FFMPEG_DIR to a valid SDK root, or install the Gyan FFmpeg Shared SDK. Checked: $checked"
}

if ($SelfTest) {
  # Deterministic seam checks for the pure pane-contract logic that the native
  # lifecycle acceptance depends on. No process, window, port, env var, or file
  # outside this run is touched; normal native execution never enters here.
  function New-SelfTestRect {
    param([int]$X, [int]$Y, [int]$W, [int]$H)
    [pscustomobject]@{ x = $X; y = $Y; w = $W; h = $H; right = ($X + $W); bottom = ($Y + $H) }
  }
  function New-SelfTestPaneReport {
    param([string]$Pane, [string]$InstanceId, $PendingCloseRequestId = $null)
    [pscustomobject]@{
      pane = $Pane
      x = 0
      y = 0
      width = 1280
      height = 720
      maximized = $false
      instance_id = $InstanceId
      pending_close_request_id = $PendingCloseRequestId
    }
  }
  function New-SelfTestPaneState {
    param(
      [string]$Mode,
      [string]$Url = "http://127.0.0.1:5187/",
      [string[]]$Labels = @(),
      $Reports = @(),
      [bool]$StageVisible = $true,
      [bool]$GroupsVisible = $true,
      [bool]$SourceVisible = $true,
      [bool]$TimelineVisible = $true,
      [int]$TimelineCount = 1,
      $Root = $null,
      $Band = $null,
      $Context = $null,
      $LowerLeft = $null,
      [int]$BandWidthForChildCheck = -1
    )
    if ($null -eq $Root) { $Root = New-SelfTestRect -X 0 -Y 0 -W 1920 -H 1000 }
    if ($null -eq $Band) { $Band = New-SelfTestRect -X 0 -Y 0 -W 1920 -H 1000 }
    if ($null -eq $Context) { $Context = New-SelfTestRect -X 0 -Y 520 -W 1920 -H 480 }
    if ($null -eq $LowerLeft) { $LowerLeft = New-SelfTestRect -X 0 -Y 520 -W 940 -H 480 }
    if ($BandWidthForChildCheck -ge 0) { $Band.w = $BandWidthForChildCheck }
    [pscustomobject]@{
      paneMode = $Mode
      url = $Url
      labels = @($Labels)
      reports = $Reports
      root = $Root
      band = $Band
      context = $Context
      lowerLeft = $LowerLeft
      timelineHost = $Root
      stageVisible = $StageVisible
      groupsVisible = $GroupsVisible
      sourceVisible = $SourceVisible
      timelineVisible = $TimelineVisible
      timelineCount = $TimelineCount
      appScrollFree = $true
    }
  }
  $selfTestChecks = [System.Collections.Generic.List[object]]::new()
  $addSelfTestCheck = {
    param([string]$Name, [bool]$Passed, [string]$Detail)
    $selfTestChecks.Add([pscustomobject]@{ Name = $Name; Passed = $Passed; Detail = $Detail })
  }
  $expectSelfTestThrow = {
    param([string]$Name, [scriptblock]$Action)
    try {
      & $Action
      & $addSelfTestCheck -Name $Name -Passed $false -Detail "did not throw"
    } catch {
      & $addSelfTestCheck -Name $Name -Passed $true -Detail $_.Exception.Message
    }
  }

  # Expectation builder: exact flags, geometry kinds, labels, and reports.
  $emptyExpectation = Get-NativePaneMainExpectation -DetachedPanes @()
  & $addSelfTestCheck -Name "main expectation integrated baseline" -Passed (
    $emptyExpectation.GeometryKind -eq "integrated-columns-present" -and
    $emptyExpectation.StageVisible -and $emptyExpectation.GroupsVisible -and
    $emptyExpectation.SourceVisible -and $emptyExpectation.TimelineVisible -and
    $emptyExpectation.TimelineCount -eq 1 -and
    (@(Compare-Object -ReferenceObject @("main") -DifferenceObject $emptyExpectation.ExpectedLabels).Count -eq 0) -and
    $emptyExpectation.ExpectedReportPanes.Count -eq 0
  ) -Detail ($emptyExpectation | ConvertTo-Json -Compress)
  $stageOnlyExpectation = Get-NativePaneMainExpectation -DetachedPanes @("stage")
  & $addSelfTestCheck -Name "main expectation stage-only detach" -Passed (
    $stageOnlyExpectation.GeometryKind -eq "context-fills-band" -and
    -not $stageOnlyExpectation.StageVisible -and -not $stageOnlyExpectation.GroupsVisible -and
    $stageOnlyExpectation.SourceVisible -and $stageOnlyExpectation.TimelineVisible -and
    $stageOnlyExpectation.TimelineCount -eq 1 -and
    (@(Compare-Object -ReferenceObject @("main", "pane-stage") -DifferenceObject $stageOnlyExpectation.ExpectedLabels).Count -eq 0) -and
    (@(Compare-Object -ReferenceObject @("stage") -DifferenceObject $stageOnlyExpectation.ExpectedReportPanes).Count -eq 0)
  ) -Detail ($stageOnlyExpectation | ConvertTo-Json -Compress)
  $timelineOnlyExpectation = Get-NativePaneMainExpectation -DetachedPanes @("timeline")
  & $addSelfTestCheck -Name "main expectation timeline-only detach" -Passed (
    $timelineOnlyExpectation.GeometryKind -eq "integrated-columns-present" -and
    $timelineOnlyExpectation.StageVisible -and -not $timelineOnlyExpectation.TimelineVisible -and
    $timelineOnlyExpectation.TimelineCount -eq 0 -and
    (@(Compare-Object -ReferenceObject @("main", "pane-timeline") -DifferenceObject $timelineOnlyExpectation.ExpectedLabels).Count -eq 0) -and
    (@(Compare-Object -ReferenceObject @("timeline") -DifferenceObject $timelineOnlyExpectation.ExpectedReportPanes).Count -eq 0)
  ) -Detail ($timelineOnlyExpectation | ConvertTo-Json -Compress)
  $bothExpectation = Get-NativePaneMainExpectation -DetachedPanes @("timeline", "stage")
  & $addSelfTestCheck -Name "main expectation both detached" -Passed (
    $bothExpectation.GeometryKind -eq "workspace-fills-root" -and
    -not $bothExpectation.StageVisible -and -not $bothExpectation.TimelineVisible -and
    $bothExpectation.TimelineCount -eq 0 -and
    (@(Compare-Object -ReferenceObject @("main", "pane-stage", "pane-timeline") -DifferenceObject $bothExpectation.ExpectedLabels).Count -eq 0) -and
    (@(Compare-Object -ReferenceObject @("stage", "timeline") -DifferenceObject $bothExpectation.ExpectedReportPanes).Count -eq 0)
  ) -Detail ($bothExpectation | ConvertTo-Json -Compress)
  & $addSelfTestCheck -Name "duplicate detached panes deduplicate" -Passed (
    (Get-NativePaneMainExpectation -DetachedPanes @("stage", "stage")).ExpectedReportPanes.Count -eq 1
  ) -Detail "dedupe check"
  & $expectSelfTestThrow -Name "unknown detached pane fails closed" -Action {
    Get-NativePaneMainExpectation -DetachedPanes @("mixer")
  }

  # Main contract tester: positives and targeted negatives per geometry kind.
  # Fixture rects mirror the shipped reflow shapes: an intact workspace keeps a
  # full-height band above the lower columns; once Stage detaches, the band
  # collapses around the surviving context surface; once both detach, the band
  # and context each span the whole split root.
  foreach ($case in @(
    @{
      Name = "integrated"
      Expectation = $emptyExpectation
      Band = $null
      Context = $null
      LowerLeft = $null
    },
    @{
      Name = "stage-only"
      Expectation = $stageOnlyExpectation
      Band = (New-SelfTestRect -X 0 -Y 520 -W 1920 -H 480)
      Context = (New-SelfTestRect -X 0 -Y 520 -W 1920 -H 480)
      LowerLeft = $null
    },
    @{
      Name = "timeline-only"
      Expectation = $timelineOnlyExpectation
      Band = $null
      Context = $null
      LowerLeft = $null
    },
    @{
      Name = "both"
      Expectation = $bothExpectation
      Band = (New-SelfTestRect -X 0 -Y 0 -W 1920 -H 1000)
      Context = (New-SelfTestRect -X 0 -Y 0 -W 1920 -H 1000)
      LowerLeft = (New-SelfTestRect -X 0 -Y 0 -W 940 -H 1000)
    }
  )) {
    $state = New-SelfTestPaneState -Mode "main" -Labels $case.Expectation.ExpectedLabels `
      -StageVisible $case.Expectation.StageVisible -GroupsVisible $case.Expectation.GroupsVisible `
      -SourceVisible $case.Expectation.SourceVisible -TimelineVisible $case.Expectation.TimelineVisible `
      -TimelineCount $case.Expectation.TimelineCount `
      -Band $case.Band -Context $case.Context -LowerLeft $case.LowerLeft `
      -Reports (@($case.Expectation.ExpectedReportPanes | ForEach-Object {
        New-SelfTestPaneReport -Pane $_ -InstanceId ("id-" + $_)
      }))
    & $addSelfTestCheck -Name ("main contract accepts compliant {0} state" -f $case.Name) -Passed (
      [bool](Test-NativePaneMainContract -State $state -Expectation $case.Expectation)
    ) -Detail "positive"
  }
  $wrongFlagState = New-SelfTestPaneState -Mode "main" -StageVisible $false -Reports @()
  & $addSelfTestCheck -Name "main contract rejects wrong stage flag" -Passed (
    -not (Test-NativePaneMainContract -State $wrongFlagState -Expectation $emptyExpectation)
  ) -Detail "negative flag"
  $wrongModeState = New-SelfTestPaneState -Mode "stage"
  & $addSelfTestCheck -Name "main contract rejects child mode" -Passed (
    -not (Test-NativePaneMainContract -State $wrongModeState -Expectation $emptyExpectation)
  ) -Detail "negative mode"
  $brokenFillState = New-SelfTestPaneState -Mode "main" -StageVisible $false -TimelineVisible $true `
    -TimelineCount 1 -Band (New-SelfTestRect -X 0 -Y 0 -W 1920 -H 1000) `
    -Context (New-SelfTestRect -X 0 -Y 520 -W 900 -H 400)
  & $addSelfTestCheck -Name "main contract rejects broken context fill" -Passed (
    -not (Test-NativePaneMainContract -State $brokenFillState -Expectation $stageOnlyExpectation)
  ) -Detail "negative geometry"
  $missingIdentityState = New-SelfTestPaneState -Mode "main" -StageVisible $false -GroupsVisible $false `
    -Reports @((New-SelfTestPaneReport -Pane "stage" -InstanceId ""))
  & $addSelfTestCheck -Name "main census rejects missing instance identity" -Passed (
    -not (Test-NativePaneMainContract -State $missingIdentityState -Expectation $stageOnlyExpectation)
  ) -Detail "negative identity"
  $pendingCloseState = New-SelfTestPaneState -Mode "main" -StageVisible $false -GroupsVisible $false `
    -Reports @((New-SelfTestPaneReport -Pane "stage" -InstanceId "id-1" -PendingCloseRequestId "req-1"))
  & $addSelfTestCheck -Name "main reports reject a stuck pending close" -Passed (
    -not (Test-NativePaneMainContract -State $pendingCloseState -Expectation $stageOnlyExpectation)
  ) -Detail "negative pending close"

  # Child contract tester.
  $stageChildExpectation = Get-NativePaneChildExpectation -PaneKind "stage" -AllDetachedPanes @("stage")
  $stageChildState = New-SelfTestPaneState -Mode "stage" `
    -Url "http://127.0.0.1:5187/index.html?syndocalPaneWindow=stage" `
    -Labels @("main", "pane-stage") `
    -StageVisible $true -GroupsVisible $true -SourceVisible $false -TimelineVisible $false `
    -TimelineCount 0 -BandWidthForChildCheck 1280
  & $addSelfTestCheck -Name "child contract accepts compliant stage child" -Passed (
    [bool](Test-NativePaneChildContract -State $stageChildState -Expectation $stageChildExpectation)
  ) -Detail "positive"
  $badUrlChildState = New-SelfTestPaneState -Mode "stage" `
    -Url "http://127.0.0.1:5187/index.html?syndocalPaneWindow=stages" `
    -Labels @("main", "pane-stage") `
    -StageVisible $true -GroupsVisible $true -SourceVisible $false -TimelineVisible $false `
    -TimelineCount 0 -BandWidthForChildCheck 1280
  & $addSelfTestCheck -Name "child contract rejects pane suffix spoof" -Passed (
    -not (Test-NativePaneChildContract -State $badUrlChildState -Expectation $stageChildExpectation)
  ) -Detail "negative url"
  $dualRenderChildState = New-SelfTestPaneState -Mode "stage" `
    -Url "http://127.0.0.1:5187/index.html?syndocalPaneWindow=stage" `
    -Labels @("main", "pane-stage") `
    -StageVisible $true -GroupsVisible $true -SourceVisible $false -TimelineVisible $false `
    -TimelineCount 0 -BandWidthForChildCheck 1280
  $dualRenderChildState.labels = @("main", "pane-stage", "pane-timeline")
  & $addSelfTestCheck -Name "child contract rejects duplicate pane label" -Passed (
    -not (Test-NativePaneChildContract -State $dualRenderChildState -Expectation $stageChildExpectation)
  ) -Detail "negative census"
  $timelineChildExpectation = Get-NativePaneChildExpectation -PaneKind "timeline" -AllDetachedPanes @("timeline")
  $timelineChildState = New-SelfTestPaneState -Mode "timeline" `
    -Url "http://127.0.0.1:5187/index.html?syndocalPaneWindow=timeline" `
    -Labels @("main", "pane-timeline") `
    -StageVisible $false -GroupsVisible $false -SourceVisible $false -TimelineVisible $true `
    -TimelineCount 1 -BandWidthForChildCheck 0
  & $addSelfTestCheck -Name "child contract accepts compliant timeline child" -Passed (
    [bool](Test-NativePaneChildContract -State $timelineChildState -Expectation $timelineChildExpectation)
  ) -Detail "positive"
  & $expectSelfTestThrow -Name "child expectation without own pane fails closed" -Action {
    Get-NativePaneChildExpectation -PaneKind "timeline" -AllDetachedPanes @("stage")
  }

  # Exact-string census comparator and pinned window titles.
  & $addSelfTestCheck -Name "exact string census order-insensitive equality" -Passed (
    (Test-ExactStringCensus -Actual @("b", "a") -Expected @("a", "b")) -and
    -not (Test-ExactStringCensus -Actual @("a", "a") -Expected @("a", "b")) -and
    -not (Test-ExactStringCensus -Actual @("a") -Expected @("a", "b"))
  ) -Detail "comparator"
  & $addSelfTestCheck -Name "native pane titles stay pinned to backend constants" -Passed (
    (Get-NativePaneWindowTitle -PaneKind "stage") -ceq "Syndocal Stage - 2D Map" -and
    (Get-NativePaneWindowTitle -PaneKind "timeline") -ceq "Syndocal Timeline"
  ) -Detail "title map"

  # ---- CDP trust-boundary seams (static; native transport stays unverified)
  # These checks exercise only the pure decision logic. No socket, listener,
  # process tree, environment variable, or WebView2 endpoint is ever touched.
  $approvedPageUri = Test-ApprovedCdpWebSocketUrl -WebSocketDebuggerUrl "ws://127.0.0.1:5188/devtools/page/AB12cd34" -Port 5188
  & $addSelfTestCheck -Name "CDP url pin accepts exact loopback page endpoint" -Passed (
    $approvedPageUri.Scheme -eq "ws" -and $approvedPageUri.Port -eq 5188 -and
    $approvedPageUri.AbsolutePath -ceq "/devtools/page/AB12cd34"
  ) -Detail "positive endpoint"
  foreach ($rejectedEndpoint in @(
    @{ Name = "https scheme"; Url = "https://127.0.0.1:5188/devtools/page/x" },
    @{ Name = "wss scheme"; Url = "wss://127.0.0.1:5188/devtools/page/x" },
    @{ Name = "localhost host"; Url = "ws://localhost:5188/devtools/page/x" },
    @{ Name = "remote IPv4 host"; Url = "ws://203.0.113.7:5188/devtools/page/x" },
    @{ Name = "IPv6 loopback host"; Url = "ws://[::1]:5188/devtools/page/x" },
    @{ Name = "wrong port"; Url = "ws://127.0.0.1:5189/devtools/page/x" },
    @{ Name = "default-port elision"; Url = "ws://127.0.0.1/devtools/page/x" },
    @{ Name = "browser non-page path"; Url = "ws://127.0.0.1:5188/devtools/browser/guid-guid" },
    @{ Name = "bare host path"; Url = "ws://127.0.0.1:5188/" },
    @{ Name = "userinfo injection"; Url = "ws://attacker@127.0.0.1:5188/devtools/page/x" },
    @{ Name = "query suffix"; Url = "ws://127.0.0.1:5188/devtools/page/x?x=1" },
    @{ Name = "relative garbage"; Url = "devtools/page/x" },
    @{ Name = "empty url"; Url = "" }
  )) {
    & $expectSelfTestThrow -Name ("CDP url pin rejects {0}" -f $rejectedEndpoint.Name) -Action {
      [void](Test-ApprovedCdpWebSocketUrl -WebSocketDebuggerUrl $rejectedEndpoint.Url -Port 5188)
    }
  }

  # Listener ownership matrix over synthetic process trees and TCP rows.
  $selfTestBaseTime = [DateTime]::UtcNow.AddMinutes(-1)
  $newSelfTestLineageRecord = {
    param([int]$ProcessId, [int]$ParentProcessId, [int]$OffsetSeconds)
    [pscustomobject]@{
      ProcessId = [uint32]$ProcessId
      ParentProcessId = [uint32]$ParentProcessId
      CreatedAtUtc = $selfTestBaseTime.AddSeconds($OffsetSeconds)
    }
  }
  $newSelfTestOwnerRow = {
    param([string]$Address, [int]$OwnerPid)
    [pscustomobject]@{ LocalAddress = $Address; OwningProcess = [uint32]$OwnerPid }
  }
  $runSelfTestOwnership = {
    param($Rows, $Table, [uint32]$QaProcessId)
    Test-CdpListenerOwnershipEntries -Listeners $Rows -QaProcessId $QaProcessId `
      -QaCreatedAtUtc $Table[[int]$QaProcessId].CreatedAtUtc `
      -ResolveProcess { param($LookupPid) $Table[[int]$LookupPid] }
  }
  $selfTestExactTable = @{ 4000 = (& $newSelfTestLineageRecord 4000 100 0) }
  $exactResult = & $runSelfTestOwnership @(& $newSelfTestOwnerRow "127.0.0.1" 4000) $selfTestExactTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership accepts exact verified QA listener" -Passed (
    [bool]$exactResult.Passed -and $exactResult.VerifiedListenerPids.Count -eq 1
  ) -Detail $exactResult.Reason
  $selfTestChildTable = @{
    4000 = (& $newSelfTestLineageRecord 4000 100 0)
    5001 = (& $newSelfTestLineageRecord 5001 4000 1)
  }
  $childResult = & $runSelfTestOwnership @(& $newSelfTestOwnerRow "127.0.0.1" 5001) $selfTestChildTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership accepts direct WebView2 child listener" -Passed (
    [bool]$childResult.Passed
  ) -Detail $childResult.Reason
  $selfTestGrandchildTable = @{
    4000 = (& $newSelfTestLineageRecord 4000 100 0)
    6002 = (& $newSelfTestLineageRecord 6002 4000 1)
    6001 = (& $newSelfTestLineageRecord 6001 6002 2)
  }
  $grandchildResult = & $runSelfTestOwnership @(& $newSelfTestOwnerRow "127.0.0.1" 6001) $selfTestGrandchildTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership accepts grandchild ancestry chain" -Passed (
    [bool]$grandchildResult.Passed
  ) -Detail $grandchildResult.Reason
  $selfTestMultiOwnedTable = @{
    4000 = (& $newSelfTestLineageRecord 4000 100 0)
    5001 = (& $newSelfTestLineageRecord 5001 4000 1)
  }
  $multiOwnedResult = & $runSelfTestOwnership @(
    & $newSelfTestOwnerRow "127.0.0.1" 4000
    & $newSelfTestOwnerRow "127.0.0.1" 5001
  ) $selfTestMultiOwnedTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership accepts multiple fully-owned listeners" -Passed (
    [bool]$multiOwnedResult.Passed -and $multiOwnedResult.VerifiedListenerPids.Count -eq 2
  ) -Detail $multiOwnedResult.Reason

  $selfTestForeignTable = @{
    4000 = (& $newSelfTestLineageRecord 4000 100 0)
    3000 = (& $newSelfTestLineageRecord 3000 0 -9)
    7002 = (& $newSelfTestLineageRecord 7002 3000 9)
    7001 = (& $newSelfTestLineageRecord 7001 7002 10)
  }
  $foreignResult = & $runSelfTestOwnership @(& $newSelfTestOwnerRow "127.0.0.1" 7001) $selfTestForeignTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership rejects foreign lineage rooted outside QA" -Passed (
    -not [bool]$foreignResult.Passed
  ) -Detail $foreignResult.Reason
  $selfTestDeadAncestorTable = @{
    4000 = (& $newSelfTestLineageRecord 4000 100 0)
    8001 = (& $newSelfTestLineageRecord 8001 8999 1)
  }
  $deadAncestorResult = & $runSelfTestOwnership @(& $newSelfTestOwnerRow "127.0.0.1" 8001) $selfTestDeadAncestorTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership rejects zero/broken ancestry" -Passed (
    -not [bool]$deadAncestorResult.Passed
  ) -Detail $deadAncestorResult.Reason
  $selfTestZeroTerminationTable = @{
    4000 = (& $newSelfTestLineageRecord 4000 100 0)
    7200 = (& $newSelfTestLineageRecord 7200 0 1)
  }
  $zeroTerminationResult = & $runSelfTestOwnership @(& $newSelfTestOwnerRow "127.0.0.1" 7200) $selfTestZeroTerminationTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership rejects PID 0 termination before QA" -Passed (
    -not [bool]$zeroTerminationResult.Passed
  ) -Detail $zeroTerminationResult.Reason
  $selfTestCycleTable = @{
    4000 = (& $newSelfTestLineageRecord 4000 100 0)
    9001 = (& $newSelfTestLineageRecord 9001 9002 1)
    9002 = (& $newSelfTestLineageRecord 9002 9001 1)
  }
  $cycleResult = & $runSelfTestOwnership @(& $newSelfTestOwnerRow "127.0.0.1" 9001) $selfTestCycleTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership rejects repeating ancestry cycle" -Passed (
    -not [bool]$cycleResult.Passed
  ) -Detail $cycleResult.Reason
  $selfTestSelfParentTable = @{
    4000 = (& $newSelfTestLineageRecord 4000 100 0)
    9501 = (& $newSelfTestLineageRecord 9501 9501 1)
  }
  $selfParentResult = & $runSelfTestOwnership @(& $newSelfTestOwnerRow "127.0.0.1" 9501) $selfTestSelfParentTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership rejects self-parent immediate cycle" -Passed (
    -not [bool]$selfParentResult.Passed
  ) -Detail $selfParentResult.Reason
  $selfTestPredatingTable = @{
    4000 = (& $newSelfTestLineageRecord 4000 100 0)
    4100 = (& $newSelfTestLineageRecord 4100 4000 -40)
  }
  $predatingResult = & $runSelfTestOwnership @(& $newSelfTestOwnerRow "127.0.0.1" 4100) $selfTestPredatingTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership rejects ancestor predating QA (PID reuse)" -Passed (
    -not [bool]$predatingResult.Passed
  ) -Detail $predatingResult.Reason
  $emptyListenerResult = Test-CdpListenerOwnershipEntries -Listeners @() -QaProcessId ([uint32]4000) `
    -QaCreatedAtUtc $selfTestBaseTime -ResolveProcess { param($LookupPid) $null }
  & $addSelfTestCheck -Name "CDP ownership rejects empty listener snapshot" -Passed (
    -not [bool]$emptyListenerResult.Passed
  ) -Detail $emptyListenerResult.Reason

  # Missing or unparsable QA creation time must fail closed: without a usable
  # birth timestamp the PID-reuse age proof is unverifiable and may never be
  # silently skipped.
  $missingQaTimeResult = Test-CdpListenerOwnershipEntries `
    -Listeners @(& $newSelfTestOwnerRow "127.0.0.1" 4000) -QaProcessId ([uint32]4000) `
    -QaCreatedAtUtc $null -ResolveProcess { param($LookupPid) $selfTestExactTable[[int]$LookupPid] }
  & $addSelfTestCheck -Name "CDP ownership fails closed when QA creation time is missing" -Passed (
    (-not [bool]$missingQaTimeResult.Passed) -and
    ($missingQaTimeResult.VerifiedListenerPids.Count -eq 0) -and
    ($missingQaTimeResult.Reason -like "*creation time*")
  ) -Detail $missingQaTimeResult.Reason
  $invalidQaTimeResult = Test-CdpListenerOwnershipEntries `
    -Listeners @(& $newSelfTestOwnerRow "127.0.0.1" 4000) -QaProcessId ([uint32]4000) `
    -QaCreatedAtUtc "definitely-not-a-timestamp" `
    -ResolveProcess { param($LookupPid) $selfTestExactTable[[int]$LookupPid] }
  & $addSelfTestCheck -Name "CDP ownership fails closed when QA creation time is invalid" -Passed (
    (-not [bool]$invalidQaTimeResult.Passed) -and
    ($invalidQaTimeResult.VerifiedListenerPids.Count -eq 0)
  ) -Detail $invalidQaTimeResult.Reason
  # An ancestor with an unparsable creation time equally breaks the age proof.
  $ancestorBadTimeTable = @{
    4000 = (& $newSelfTestLineageRecord 4000 100 0)
    4300 = [pscustomobject]@{
      ProcessId = [uint32]4300
      ParentProcessId = [uint32]4000
      CreatedAtUtc = "garbage-timestamp"
    }
  }
  $ancestorBadTimeResult = Test-CdpListenerOwnershipEntries `
    -Listeners @(& $newSelfTestOwnerRow "127.0.0.1" 4300) -QaProcessId ([uint32]4000) `
    -QaCreatedAtUtc $selfTestBaseTime -ResolveProcess { param($LookupPid) $ancestorBadTimeTable[[int]$LookupPid] }
  & $addSelfTestCheck -Name "CDP ownership fails closed when ancestor creation time is invalid" -Passed (
    (-not [bool]$ancestorBadTimeResult.Passed) -and
    ($ancestorBadTimeResult.Reason -like "*4300*")
  ) -Detail $ancestorBadTimeResult.Reason
  $wildcardResult = & $runSelfTestOwnership @(& $newSelfTestOwnerRow "0.0.0.0" 4000) $selfTestExactTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership rejects wildcard-bound listener" -Passed (
    -not [bool]$wildcardResult.Passed
  ) -Detail $wildcardResult.Reason
  $ipv6Result = & $runSelfTestOwnership @(& $newSelfTestOwnerRow "::1" 4000) $selfTestExactTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership rejects non-configured ::1 listener" -Passed (
    -not [bool]$ipv6Result.Passed
  ) -Detail $ipv6Result.Reason
  $selfTestMixedTable = @{
    4000 = (& $newSelfTestLineageRecord 4000 100 0)
    7002 = (& $newSelfTestLineageRecord 7002 0 1)
    7001 = (& $newSelfTestLineageRecord 7001 7002 2)
  }
  $mixedResult = & $runSelfTestOwnership @(
    & $newSelfTestOwnerRow "127.0.0.1" 4000
    & $newSelfTestOwnerRow "127.0.0.1" 7001
  ) $selfTestMixedTable ([uint32]4000)
  & $addSelfTestCheck -Name "CDP ownership rejects one foreign listener among owned" -Passed (
    -not [bool]$mixedResult.Passed
  ) -Detail $mixedResult.Reason

  # Pre-existing WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS guard.
  $benignGuardOk = $true
  try {
    Assert-NoPreexistingWebView2RemoteDebuggingArguments -Value "--disable-features=Alpha --enable-blink-features=Beta"
    Assert-NoPreexistingWebView2RemoteDebuggingArguments -Value ""
    Assert-NoPreexistingWebView2RemoteDebuggingArguments -Value "/disable-features=Alpha --enable-blink-features=Beta"
    Assert-NoPreexistingWebView2RemoteDebuggingArguments -Value "--remote-debugging-pipeline"
    Assert-NoPreexistingWebView2RemoteDebuggingArguments -Value "--enable-features=RemoteDebuggingPipeDemo --disable-gpu"
  } catch {
    $benignGuardOk = $false
  }
  & $addSelfTestCheck -Name "pre-existing arg guard accepts benign, empty, slash-prefixed benign, and pipe-lookalike values" -Passed (
    $benignGuardOk
  ) -Detail "benign/empty/slash-benign/pipe-near-miss"
  foreach ($hostileArgs in @(
    @{ Name = "remote-debugging-port="; Value = "--remote-debugging-port=9333" },
    @{ Name = "remote-debugging-address="; Value = "--remote-debugging-address=0.0.0.0" },
    @{ Name = "leading port flag"; Value = "--remote-debugging-port=9333 --disable-gpu" },
    @{ Name = "trailing bare address flag"; Value = "--disable-gpu --remote-debugging-address" },
    @{ Name = "case-insensitive match"; Value = "--REMOTE-DEBUGGING-PORT=1" },
    @{ Name = "embedded middle flag"; Value = "--disable-gpu --remote-debugging-address=127.0.0.1 --no-sandbox" },
    @{ Name = "slash-prefixed port"; Value = "/remote-debugging-port=9333" },
    @{ Name = "slash-prefixed address"; Value = "/remote-debugging-address=0.0.0.0" },
    @{ Name = "leading slash port flag"; Value = "/remote-debugging-port=9333 --disable-gpu" },
    @{ Name = "embedded slash middle flag"; Value = "--disable-gpu /REMOTE-DEBUGGING-ADDRESS=127.0.0.1 --no-sandbox" },
    @{ Name = "trailing slash bare address flag"; Value = "--disable-gpu /remote-debugging-address" },
    @{ Name = "single-hyphen port"; Value = "-remote-debugging-port=9333" },
    @{ Name = "single-hyphen address"; Value = "-remote-debugging-address=0.0.0.0" },
    @{ Name = "leading single-hyphen port flag"; Value = "-remote-debugging-port=9333 --disable-gpu" },
    @{ Name = "trailing single-hyphen bare address flag"; Value = "--disable-gpu -REMOTE-DEBUGGING-ADDRESS" },
    @{ Name = "bare double-hyphen pipe flag"; Value = "--remote-debugging-pipe" },
    @{ Name = "single-hyphen pipe flag"; Value = "-remote-debugging-pipe" },
    @{ Name = "slash-prefixed pipe flag"; Value = "/remote-debugging-pipe" },
    @{ Name = "case-insensitive pipe flag"; Value = "--Remote-Debugging-Pipe" },
    @{ Name = "embedded middle pipe flag"; Value = "--disable-gpu --remote-debugging-pipe --no-sandbox" },
    @{ Name = "equal-form pipe flag"; Value = "--disable-gpu --remote-debugging-pipe=1" },
    @{ Name = "trailing slash bare pipe flag"; Value = "--disable-gpu /remote-debugging-pipe" }
  )) {
    & $expectSelfTestThrow -Name ("pre-existing arg guard rejects {0}" -f $hostileArgs.Name) -Action {
      Assert-NoPreexistingWebView2RemoteDebuggingArguments -Value $hostileArgs.Value
    }
  }

  # Single-page selection seam: zero matches keep polling; one match wins;
  # multiple matching pages are a typed immediate trust violation.
  $selectedSinglePage = Assert-SingleMatchingCdpPage `
    -MatchingPages @([pscustomobject]@{ url = "http://127.0.0.1:5187/" }) -ExpectedMode "main"
  & $addSelfTestCheck -Name "CDP single matching page resolves" -Passed (
    ($null -ne $selectedSinglePage) -and
    ([string]$selectedSinglePage.url -ceq "http://127.0.0.1:5187/")
  ) -Detail "positive single"
  $zeroMatchSelection = Assert-SingleMatchingCdpPage -MatchingPages @() -ExpectedMode "main"
  & $addSelfTestCheck -Name "CDP zero matching pages defers to the wait loop" -Passed (
    $null -eq $zeroMatchSelection
  ) -Detail "zero keeps waiting"
  $ambiguousThrowTypeName = ""
  $ambiguousMessage = ""
  try {
    [void](Assert-SingleMatchingCdpPage -MatchingPages @(
      [pscustomobject]@{ url = "http://127.0.0.1:5187/?a" },
      [pscustomobject]@{ url = "http://127.0.0.1:5187/?b" }
    ) -ExpectedMode "stage")
  } catch {
    $ambiguousThrowTypeName = $_.Exception.GetType().FullName
    $ambiguousMessage = $_.Exception.Message
  }
  & $addSelfTestCheck -Name "CDP multi-page ambiguity is a typed immediate trust violation" -Passed (
    ($ambiguousThrowTypeName -eq [SyndocalCdpTrustViolation].FullName) -and
    ($ambiguousMessage -like "*2 'stage' pages*")
  ) -Detail "${ambiguousThrowTypeName} :: ${ambiguousMessage}"

  # Click-expression encoding: a hostile description (quotes, backslashes,
  # newlines) must appear only as one JSON-encoded literal, never raw text,
  # so it cannot terminate or rewrite the evaluated JavaScript program.
  $hostileDescription = 'quote " back\slash' + "`r`nsecond 'line"
  $encodedClickExpression =
    (New-CdpVisibleDomClickExpression -Selector '[data-x="quoted"]' -Description $hostileDescription).TrimEnd()
  $expectedDescriptionLiteral = $hostileDescription | ConvertTo-Json -Compress
  $expectedSelectorLiteral = '[data-x="quoted"]' | ConvertTo-Json -Compress
  & $addSelfTestCheck -Name "click expression encodes hostile description safely" -Passed (
    $encodedClickExpression.Contains($expectedDescriptionLiteral) -and
    $encodedClickExpression.Contains($expectedSelectorLiteral) -and
    (-not $encodedClickExpression.Contains($hostileDescription)) -and
    (@([regex]::Matches($encodedClickExpression, [regex]::Escape($expectedDescriptionLiteral))).Count -eq 1) -and
    $encodedClickExpression.StartsWith("(() => {") -and
    $encodedClickExpression.EndsWith("})()") -and
    (@([regex]::Matches($encodedClickExpression, "throw new Error\(description")).Count -eq 3)
  ) -Detail $expectedDescriptionLiteral

  & $addSelfTestCheck -Name "CDP native transport intentionally unverified by self-test" -Passed $true `
    -Detail "static seams only; no socket, live listener, real process tree, or WebView2 endpoint is exercised"


  $failedSelfTests = @($selfTestChecks | Where-Object { -not $_.Passed })
  foreach ($check in $selfTestChecks) {
    $status = if ($check.Passed) { "PASS" } else { "FAIL" }
    Write-Host "$status $($check.Name) -- $($check.Detail)"
  }
  Write-Host "SUMMARY: $($selfTestChecks.Count) self-test checks, $($failedSelfTests.Count) failed"
  if ($failedSelfTests.Count -gt 0) { exit 1 }
  exit 0
}

$minimumMaximizedSize = ConvertFrom-DimensionText -Value $MinimumMaximizedClient
$expectedFullscreenSize = ConvertFrom-DimensionText -Value $ExpectedFullscreen
$scriptDir = Split-Path -Parent $PSCommandPath
$appRoot = (Resolve-Path (Join-Path $scriptDir "..")).Path
$qaTitle = "Syndocal QA - Native 1920 Acceptance"
if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
  $stamp = [DateTime]::UtcNow.ToString("yyyyMMdd-HHmmss")
  $EvidenceDir = Join-Path ([IO.Path]::GetTempPath()) "syndocal-native-acceptance-$stamp"
}
$EvidenceDir = [IO.Path]::GetFullPath($EvidenceDir)
[void](New-Item -ItemType Directory -Path $EvidenceDir -Force)

$pnpm = Get-Command pnpm.cmd -ErrorAction Stop
$oldTargetDir = $env:CARGO_TARGET_DIR
$oldFfmpegDir = $env:FFMPEG_DIR
$oldPath = $env:PATH
$oldWebView2AdditionalBrowserArguments = [Environment]::GetEnvironmentVariable(
  "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
  [EnvironmentVariableTarget]::Process
)
$webView2AdditionalBrowserArgumentsConfigured = $false
$ffmpegDir = Resolve-FfmpegSdkRoot
$env:FFMPEG_DIR = $ffmpegDir
$env:PATH = "$(Join-Path $ffmpegDir 'bin');$oldPath"
$env:CARGO_TARGET_DIR = Join-Path ([IO.Path]::GetTempPath()) "syndocal-native-acceptance-target"
$expectedQaExecutable = [IO.Path]::GetFullPath(
  (Join-Path $env:CARGO_TARGET_DIR "debug\syndocal.exe")
)
$devProcess = $null
$qaWindow = [IntPtr]::Zero
$qaProcess = $null
$qaWindowVerified = $false
$startedAt = [DateTime]::UtcNow

try {
  if (Get-NetTCPConnection -LocalPort 5187 -State Listen -ErrorAction SilentlyContinue) {
    throw "The isolated native QA port 5187 is already in use."
  }
  if (Get-NetTCPConnection -LocalPort $CdpPort -State Listen -ErrorAction SilentlyContinue) {
    throw "The isolated native QA CDP port $CdpPort is already in use."
  }
  # Duplicate-flag precedence must never decide which CDP endpoint WebView2
  # opens: any pre-existing remote-debugging port OR address fails closed.
  Assert-NoPreexistingWebView2RemoteDebuggingArguments -Value $oldWebView2AdditionalBrowserArguments
  $preexistingQaWindow = Find-WindowByTitle -Title $qaTitle
  if ($preexistingQaWindow -ne [IntPtr]::Zero) {
    throw "A pre-existing '$qaTitle' window is open. Close that stale QA instance before collecting current-revision evidence."
  }
  $cdpArguments = "--remote-debugging-address=127.0.0.1 --remote-debugging-port=$CdpPort"
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = if ([string]::IsNullOrWhiteSpace($oldWebView2AdditionalBrowserArguments)) {
    $cdpArguments
  } else {
    "$oldWebView2AdditionalBrowserArguments $cdpArguments"
  }
  $webView2AdditionalBrowserArgumentsConfigured = $true
  Write-Host "NATIVE PRIMARY GATE: maximized client >= $MinimumMaximizedClient -> F11 fullscreen $ExpectedFullscreen -> Esc exact restore -> verified Stage/Timeline pane lifecycle in both detach orders + restart-with-detached-records, reload adoption, and direct-child-close reintegration"
  Write-Host "Native pane CDP is loopback-only on 127.0.0.1:$CdpPort."
  Write-Host "Evidence: $EvidenceDir"
  $devProcess = Start-Process -FilePath $pnpm.Source `
    -ArgumentList @("tauri", "dev", "--config", "src-tauri/tauri.native-acceptance.conf.json", "--no-watch") `
    -WorkingDirectory $appRoot `
    -PassThru `
    -NoNewWindow

  $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
  while ([DateTime]::UtcNow -lt $deadline) {
    $qaWindow = Find-WindowByTitle -Title $qaTitle
    if ($qaWindow -ne [IntPtr]::Zero) {
      break
    }
    if ($devProcess.HasExited) {
      $devProcess.WaitForExit()
      throw "The native QA launcher exited with code $($devProcess.ExitCode) before its window appeared."
    }
    Start-Sleep -Milliseconds 500
  }
  if ($qaWindow -eq [IntPtr]::Zero) {
    throw "Timed out waiting for the isolated '$qaTitle' window."
  }

  [uint32]$qaProcessId = 0
  [void][SyndocalNativeWindow]::GetWindowThreadProcessId($qaWindow, [ref]$qaProcessId)
  $qaProcess = Get-Process -Id $qaProcessId -ErrorAction Stop
  if (
    $qaProcess.StartTime.ToUniversalTime() -lt $startedAt.AddSeconds(-2) -or
    [string]::IsNullOrWhiteSpace($qaProcess.Path) -or
    [IO.Path]::GetFullPath($qaProcess.Path) -ine $expectedQaExecutable
  ) {
    throw "The '$qaTitle' window does not belong to the current isolated QA build: PID $qaProcessId, path '$($qaProcess.Path)'."
  }
  $qaWindowVerified = $true

  $maximized = Wait-ForMinimumClientDimensions -Handle $qaWindow -Minimum $minimumMaximizedSize -RequireMaximized
  $monitorSize = Get-MonitorDimensions -Handle $qaWindow
  if (-not (Test-Dimensions -Actual $monitorSize -Expected $expectedFullscreenSize -AllowedTolerancePx 0)) {
    throw "The QA window monitor is $($monitorSize.Width)x$($monitorSize.Height), but the primary gate requires $ExpectedFullscreen."
  }

  $maximizedScreenshot = Join-Path $EvidenceDir "01-maximized-$($maximized.Width)x$($maximized.Height).png"
  $maximizedVisual = Save-VerifiedClientScreenshot -Handle $qaWindow -Path $maximizedScreenshot -Stage "Maximized"
  Write-Host "PASS maximized client $($maximized.Width)x$($maximized.Height)"

  Send-NativeKey -Handle $qaWindow -VirtualKey 0x7A
  $fullscreen = Wait-ForClientDimensions -Handle $qaWindow -Expected $expectedFullscreenSize -AllowedTolerancePx 0
  $fullscreenScreenshot = Join-Path $EvidenceDir "02-fullscreen-$ExpectedFullscreen.png"
  $fullscreenVisual = Save-VerifiedClientScreenshot -Handle $qaWindow -Path $fullscreenScreenshot -Stage "Fullscreen"
  Write-Host "PASS F11 fullscreen client $($fullscreen.Width)x$($fullscreen.Height)"

  Send-NativeKey -Handle $qaWindow -VirtualKey 0x1B
  $restored = Wait-ForClientDimensions -Handle $qaWindow -Expected $maximized -AllowedTolerancePx 0
  if (-not [SyndocalNativeWindow]::IsZoomed($qaWindow)) {
    throw "Esc returned the expected client size but did not restore maximized state."
  }
  if ($restored.Width -ne $maximized.Width -or $restored.Height -ne $maximized.Height) {
    throw "Esc restored $($restored.Width)x$($restored.Height), not the original $($maximized.Width)x$($maximized.Height)."
  }
  $restoredScreenshot = Join-Path $EvidenceDir "03-restored-maximized-$($restored.Width)x$($restored.Height).png"
  $restoredVisual = Save-VerifiedClientScreenshot -Handle $qaWindow -Path $restoredScreenshot -Stage "Restored maximized"
  Write-Host "PASS Esc restored maximized client $($restored.Width)x$($restored.Height)"

  $mainPage = Wait-ForCdpAppPage -Port $CdpPort -QaProcessId $qaProcessId -ExpectedMode "main" -TimeoutSeconds 60
  $initialPaneState = Wait-ForNativePaneDomState -Page $mainPage -Description "clean native pane preflight" -Predicate {
    param($state)
    $state.paneMode -eq "main" -and
      (Test-NativePaneCensus -State $state -ExpectedLabels @("main") -ExpectedReportPanes @()).Passed
  }
  Assert-NativePaneState -State $initialPaneState -Description "clean native pane preflight" -ExpectedMode "main" `
    -ExpectedLabels @("main") -ExpectedReportPanes @() -AdditionalContract {
      param($state)
      $state.appScrollFree
    }
  $paneLifecycle = Invoke-NativePaneLifecycleAcceptance -MainPage $mainPage -MainWindow $qaWindow `
    -ProcessId $qaProcessId -Minimum $minimumMaximizedSize -Port $CdpPort -OutputDirectory $EvidenceDir
  Write-Host "PASS native Stage -> Timeline -> Stage rejoin -> Timeline rejoin pane lifecycle"

  # ---- Second detach order: Timeline first, then Stage --------------------
  $orderBLifecycle = Invoke-TimelineFirstStageSecondLifecycleAcceptance -MainPage $mainPage `
    -MainWindow $qaWindow -ProcessId $qaProcessId -Minimum $minimumMaximizedSize `
    -Port $CdpPort -OutputDirectory $EvidenceDir
  Write-Host "PASS native Timeline -> Stage detach order; both detach orders covered"

  # ---- Restart while both detached records stay popped --------------------
  # Real full-process restart of this script's own isolated QA instance. Every
  # WM_CLOSE targets an HWND whose exact title and owning PID were re-verified
  # immediately before posting, and any forced cleanup targets only this run's
  # own dev-process tree, exactly like the existing finally policy.
  $previousQaProcessId = $qaProcessId
  $restartMarker = [DateTime]::UtcNow
  Close-VerifiedOwnedNativeWindow -Handle $qaWindow -ProcessId $qaProcessId -Title $qaTitle -TimeoutSeconds 30
  $qaWindowVerified = $false
  # Closing the main window leaves its detached children alive on purpose so
  # their graceful destruction cannot clear the persisted popped records.
  foreach ($orphanPaneKind in @("stage", "timeline")) {
    $orphanTitle = Get-NativePaneWindowTitle -PaneKind $orphanPaneKind
    $orphanWindow = Find-WindowByTitleAndProcess -Title $orphanTitle -ProcessId $qaProcessId
    if ($orphanWindow -eq [IntPtr]::Zero) {
      throw "Orphaned '$orphanTitle' vanished before the controlled restart could close it; child fate is unknown."
    }
    Close-VerifiedOwnedNativeWindow -Handle $orphanWindow -ProcessId $qaProcessId -Title $orphanTitle -TimeoutSeconds 30
  }
  $exitDeadline = [DateTime]::UtcNow.AddSeconds(30)
  while (-not $devProcess.HasExited -and [DateTime]::UtcNow -lt $exitDeadline) {
    Start-Sleep -Milliseconds 250
  }
  if (-not $devProcess.HasExited) {
    & taskkill.exe /PID $devProcess.Id /T /F *> $null
    $devProcess.WaitForExit()
  }
  if (Get-NetTCPConnection -LocalPort 5187 -State Listen -ErrorAction SilentlyContinue) {
    throw "The isolated QA dev port 5187 is still bound after the owned restart shutdown."
  }
  if (Get-NetTCPConnection -LocalPort $CdpPort -State Listen -ErrorAction SilentlyContinue) {
    throw "The isolated QA CDP port $CdpPort is still bound after the owned restart shutdown."
  }

  $devProcess = Start-Process -FilePath $pnpm.Source `
    -ArgumentList @("tauri", "dev", "--config", "src-tauri/tauri.native-acceptance.conf.json", "--no-watch") `
    -WorkingDirectory $appRoot `
    -PassThru `
    -NoNewWindow
  $qaWindow = [IntPtr]::Zero
  $relaunchDeadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
  while ([DateTime]::UtcNow -lt $relaunchDeadline) {
    $qaWindow = Find-WindowByTitle -Title $qaTitle
    if ($qaWindow -ne [IntPtr]::Zero) {
      break
    }
    if ($devProcess.HasExited) {
      $devProcess.WaitForExit()
      throw "The restarted native QA launcher exited with code $($devProcess.ExitCode) before its window appeared."
    }
    Start-Sleep -Milliseconds 500
  }
  if ($qaWindow -eq [IntPtr]::Zero) {
    throw "Timed out waiting for the restarted '$qaTitle' window."
  }
  [uint32]$qaProcessId = 0
  [void][SyndocalNativeWindow]::GetWindowThreadProcessId($qaWindow, [ref]$qaProcessId)
  $qaProcess = Get-Process -Id $qaProcessId -ErrorAction Stop
  if (
    $qaProcess.StartTime.ToUniversalTime() -lt $restartMarker.AddSeconds(-2) -or
    $qaProcessId -eq $previousQaProcessId -or
    [string]::IsNullOrWhiteSpace($qaProcess.Path) -or
    [IO.Path]::GetFullPath($qaProcess.Path) -ine $expectedQaExecutable
  ) {
    throw "The restarted '$qaTitle' window is not a fresh instance of the current isolated QA executable: PID $qaProcessId, path '$($qaProcess.Path)'."
  }
  $qaWindowVerified = $true
  $restartedMaximizedClient = Wait-ForMinimumClientDimensions -Handle $qaWindow -Minimum $minimumMaximizedSize -RequireMaximized

  $restartMainPage = Wait-ForCdpAppPage -Port $CdpPort -QaProcessId $qaProcessId -ExpectedMode "main" -TimeoutSeconds 60
  $restoredExpectation = Get-NativePaneMainExpectation -DetachedPanes @("stage", "timeline")
  $restoredMainState = Wait-ForNativePaneDomState -Page $restartMainPage `
    -Description "startup restore of detached records" -Predicate {
      param($state)
      Test-NativePaneMainContract -State $state -Expectation $restoredExpectation
    } -TimeoutSeconds 60
  Assert-NativePaneState -State $restoredMainState -Description "startup restore of detached records" `
    -ExpectedMode "main" -ExpectedLabels $restoredExpectation.ExpectedLabels `
    -ExpectedReportPanes $restoredExpectation.ExpectedReportPanes `
    -AdditionalContract { param($state) Test-NativePaneMainContract -State $state -Expectation $restoredExpectation }
  $restoredStageTitle = Get-NativePaneWindowTitle -PaneKind "stage"
  $restoredTimelineTitle = Get-NativePaneWindowTitle -PaneKind "timeline"
  # The startup restore chain creates child windows briefly hidden before
  # showing them, so the exact VISIBLE native-title census runs only after
  # each HWND was individually awaited and maximized.
  $restoredStageWindow = Wait-ForWindowByTitleAndProcess -Title $restoredStageTitle -ProcessId $qaProcessId
  [void](Maximize-VerifiedQaWindow -Handle $restoredStageWindow -ProcessId $qaProcessId `
    -Title $restoredStageTitle -Minimum $minimumMaximizedSize)
  $restoredTimelineWindow = Wait-ForWindowByTitleAndProcess -Title $restoredTimelineTitle -ProcessId $qaProcessId
  [void](Maximize-VerifiedQaWindow -Handle $restoredTimelineWindow -ProcessId $qaProcessId `
    -Title $restoredTimelineTitle -Minimum $minimumMaximizedSize)
  Assert-NativeWindowTitleCensus -ProcessId $qaProcessId `
    -ExpectedTitles @($qaTitle, $restoredStageTitle, $restoredTimelineTitle) `
    -Description "after restart record restore"
  $restoredChildExpectation = Get-NativePaneChildExpectation -PaneKind "stage" -AllDetachedPanes @("stage", "timeline")
  $restoredStagePage = Wait-ForCdpAppPage -Port $CdpPort -QaProcessId $qaProcessId -ExpectedMode "stage"
  $restoredStageChild = Wait-ForNativePaneDomState -Page $restoredStagePage `
    -Description "restored Stage child content" -Predicate {
      param($state)
      Test-NativePaneChildContract -State $state -Expectation $restoredChildExpectation
    } -TimeoutSeconds 45
  Assert-NativePaneState -State $restoredStageChild -Description "restored Stage child pane-stage" `
    -ExpectedMode "stage" -ExpectedLabels $restoredChildExpectation.ExpectedLabels -ExpectedReportPanes @() `
    -AdditionalContract { param($state) Test-NativePaneChildContract -State $state -Expectation $restoredChildExpectation }
  $restoredStageChildScreenshot = Join-Path $EvidenceDir "14-restart-restored-stage-child.png"
  $restoredStageChildVisual = Save-VerifiedClientScreenshot -Handle $restoredStageWindow `
    -Path $restoredStageChildScreenshot -Stage "Restored Stage child"
  Invoke-NativeTimelineDeskShowSelection -TimelinePage (Wait-ForCdpAppPage -Port $CdpPort -QaProcessId $qaProcessId -ExpectedMode "timeline")
  $restoredTimelineChildExpectation = Get-NativePaneChildExpectation -PaneKind "timeline" -AllDetachedPanes @("stage", "timeline")
  $restoredTimelinePage = Wait-ForCdpAppPage -Port $CdpPort -QaProcessId $qaProcessId -ExpectedMode "timeline"
  $restoredTimelineChild = Wait-ForNativePaneDomState -Page $restoredTimelinePage `
    -Description "restored Timeline child content" -Predicate {
      param($state)
      Test-NativePaneChildContract -State $state -Expectation $restoredTimelineChildExpectation
    } -TimeoutSeconds 45
  Assert-NativePaneState -State $restoredTimelineChild -Description "restored Timeline child pane-timeline" `
    -ExpectedMode "timeline" -ExpectedLabels $restoredTimelineChildExpectation.ExpectedLabels -ExpectedReportPanes @() `
    -AdditionalContract { param($state) Test-NativePaneChildContract -State $state -Expectation $restoredTimelineChildExpectation }
  $restoredTimelineChildScreenshot = Join-Path $EvidenceDir "15-restart-restored-timeline-child.png"
  $restoredTimelineChildVisual = Save-VerifiedClientScreenshot -Handle $restoredTimelineWindow `
    -Path $restoredTimelineChildScreenshot -Stage "Restored Timeline child"
  $restoredMainScreenshot = Join-Path $EvidenceDir "16-restart-main-records-restored.png"
  $restoredMainVisual = Save-VerifiedClientScreenshot -Handle $qaWindow -Path $restoredMainScreenshot `
    -Stage "Restart restored detached records"
  $instanceIdsBeforeReload = @{}
  foreach ($reportEntry in @($restoredMainState.reports)) {
    if ($null -ne $reportEntry) {
      $instanceIdsBeforeReload[[string]$reportEntry.pane] = [string]$reportEntry.instance_id
    }
  }
  if ($instanceIdsBeforeReload.Keys.Count -ne 2 -or
    @(foreach ($paneKey in @("stage", "timeline")) { if ([string]::IsNullOrWhiteSpace([string]$instanceIdsBeforeReload[$paneKey])) { $paneKey } }).Count -gt 0) {
    throw "The restored detached records must carry exact tracked identities before the adoption probe."
  }
  Write-Host "PASS restart reopened both detached panes with exact census and reflow"

  # ---- Child-present adoption across a real main-window reload -------------
  # The children stay alive while only the main document reloads; the startup
  # restore chain must adopt the exact live identities instead of minting new
  # ones or dual-rendering the panes in the main window.
  [void](Invoke-CdpRuntimeEvaluate -Page $restartMainPage -Expression @'
(() => { window.__syndocalQaReloadProbe = 'armed'; return 'armed'; })()
'@)
  try {
    [void](Invoke-CdpRuntimeEvaluate -Page $restartMainPage `
      -Expression "(() => { location.reload(); return 'reloading'; })()" -TimeoutSeconds 5)
  } catch [SyndocalCdpTrustViolation] {
    # The reload evaluation re-proves listener ownership before dialing; a
    # trust violation there is terminal, not a navigation nuisance.
    throw
  } catch {
    # Navigating tears down the evaluating execution context; whether the
    # reload really happened is proven by the sentinel below, never assumed.
  }
  $reloadMainPage = Wait-ForCdpAppPage -Port $CdpPort -QaProcessId $qaProcessId -ExpectedMode "main" -TimeoutSeconds 60
  $reloadReady = $false
  $lastProbeObservation = "probe not evaluated"
  $reloadDeadline = [DateTime]::UtcNow.AddSeconds(30)
  $probeFailureStreak = 0
  while ([DateTime]::UtcNow -lt $reloadDeadline) {
    try {
      $probeState = Invoke-CdpRuntimeEvaluate -Page $reloadMainPage -Expression @'
(() => ({ ready: document.readyState === "complete", probe: window.__syndocalQaReloadProbe === undefined ? null : String(window.__syndocalQaReloadProbe) }))()
'@
      $lastProbeObservation = "$($probeState.ready)/$($probeState.probe)"
      $probeFailureStreak = 0
      if ($probeState.ready -eq $true -and $null -eq $probeState.probe) {
        $reloadReady = $true
        break
      }
    } catch [SyndocalCdpTrustViolation] {
      # Every probe evaluation re-proves CDP loopback ownership before dialing;
      # a raced/foreign endpoint must abort the leg instead of polling through.
      throw
    } catch {
      $lastProbeObservation = $_.Exception.Message
      # A navigation can invalidate the acquired target identity; re-resolve
      # the main page periodically instead of polling a dead WebSocket URL.
      $probeFailureStreak += 1
      if ($probeFailureStreak -ge 8) {
        try {
          $reloadMainPage = Wait-ForCdpAppPage -Port $CdpPort -QaProcessId $qaProcessId -ExpectedMode "main" -TimeoutSeconds 5
        } catch [SyndocalCdpTrustViolation] {
          throw
        } catch {
          $lastProbeObservation = "$($lastProbeObservation); page re-resolution failed"
        }
      }
    }
    Start-Sleep -Milliseconds 250
  }
  if (-not $reloadReady) {
    throw "The main-window reload sentinel never cleared on a fresh document; adoption proof would be unverified. Last observation: $lastProbeObservation"
  }
  $adoptedMainState = Wait-ForNativePaneDomState -Page $reloadMainPage `
    -Description "adoption after main-window reload" -Predicate {
      param($state)
      Test-NativePaneMainContract -State $state -Expectation $restoredExpectation
    } -TimeoutSeconds 60
  Assert-NativePaneState -State $adoptedMainState -Description "adoption after main-window reload" `
    -ExpectedMode "main" -ExpectedLabels $restoredExpectation.ExpectedLabels `
    -ExpectedReportPanes $restoredExpectation.ExpectedReportPanes `
    -AdditionalContract { param($state) Test-NativePaneMainContract -State $state -Expectation $restoredExpectation }
  foreach ($adoptedEntry in @($adoptedMainState.reports)) {
    if ($null -eq $adoptedEntry) { continue }
    $paneKey = [string]$adoptedEntry.pane
    if ([string]$adoptedEntry.instance_id -ne [string]$instanceIdsBeforeReload[$paneKey]) {
      throw (
        "Pane '$paneKey' did not adopt its pre-reload identity: expected " +
        "'$($instanceIdsBeforeReload[$paneKey])', found '$($adoptedEntry.instance_id)'."
      )
    }
  }
  if ((Find-WindowByTitleAndProcess -Title $restoredStageTitle -ProcessId $qaProcessId) -ne $restoredStageWindow -or
    (Find-WindowByTitleAndProcess -Title $restoredTimelineTitle -ProcessId $qaProcessId) -ne $restoredTimelineWindow) {
    throw "A detached child HWND changed across the main-window reload; adoption was not proven against the same live windows."
  }
  Assert-NativeWindowTitleCensus -ProcessId $qaProcessId `
    -ExpectedTitles @($qaTitle, $restoredStageTitle, $restoredTimelineTitle) `
    -Description "after main-window adoption"
  $adoptedMainScreenshot = Join-Path $EvidenceDir "17-main-adopted-after-reload.png"
  $adoptedMainVisual = Save-VerifiedClientScreenshot -Handle $qaWindow -Path $adoptedMainScreenshot `
    -Stage "Adopted children after main reload"
  Write-Host "PASS main-window reload adopted the exact live child identities"

  # ---- Proven child close through the direct titlebar-X terminal path -----
  # The Stage child registers no close protection, so a verified WM_CLOSE is
  # the shipped direct-titlebar-close scenario: the destroyed observer must
  # retire the identity and reintegrate Stage into the main window without a
  # duplicate pane or an unused void.
  Close-VerifiedOwnedNativeWindow -Handle $restoredStageWindow -ProcessId $qaProcessId `
    -Title $restoredStageTitle -TimeoutSeconds 30
  $afterDirectCloseExpectation = Get-NativePaneMainExpectation -DetachedPanes @("timeline")
  $afterDirectCloseState = Wait-ForNativePaneDomState -Page $reloadMainPage `
    -Description "Stage reintegration after direct child close" -Predicate {
      param($state)
      Test-NativePaneMainContract -State $state -Expectation $afterDirectCloseExpectation
    } -TimeoutSeconds 60
  Assert-NativePaneState -State $afterDirectCloseState -Description "Stage reintegration after direct child close" `
    -ExpectedMode "main" -ExpectedLabels $afterDirectCloseExpectation.ExpectedLabels `
    -ExpectedReportPanes $afterDirectCloseExpectation.ExpectedReportPanes `
    -AdditionalContract { param($state) Test-NativePaneMainContract -State $state -Expectation $afterDirectCloseExpectation }
  Assert-NativeWindowTitleCensus -ProcessId $qaProcessId `
    -ExpectedTitles @($qaTitle, $restoredTimelineTitle) `
    -Description "after direct Stage child close"
  $directCloseScreenshot = Join-Path $EvidenceDir "18-main-after-direct-stage-child-close.png"
  $directCloseVisual = Save-VerifiedClientScreenshot -Handle $qaWindow -Path $directCloseScreenshot `
    -Stage "Main after direct Stage child close"
  Write-Host "PASS direct Stage child close retired its identity and reintegrated Stage"

  $finalRestartLegState = Invoke-NativePaneRejoinStep -MainPage $reloadMainPage -MainWindow $qaWindow `
    -ProcessId $qaProcessId -PaneKind "timeline" -RemainingDetachedPanes @() `
    -OutputDirectory $EvidenceDir -EvidenceIndex 19 -EvidenceSlug "post-restart-timeline-reintegration"
  Assert-NativeWindowTitleCensus -ProcessId $qaProcessId `
    -ExpectedTitles @($qaTitle) `
    -Description "fully reintegrated after restart legs"
  Write-Host "PASS post-restart Timeline reintegration returned the fully integrated main"

  $report = [ordered]@{
    gate = "windows-native-primary-1920-and-pane-lifecycle"
    title = $qaTitle
    started_at_utc = $startedAt.ToString("o")
    completed_at_utc = [DateTime]::UtcNow.ToString("o")
    tolerance_px = $TolerancePx
    fullscreen_tolerance_px = 0
    minimum_maximized_client = $minimumMaximizedSize
    monitor = $monitorSize
    maximized = $maximized
    fullscreen = $fullscreen
    restored_maximized = $restored
    visual_metrics = [ordered]@{
      maximized = $maximizedVisual
      fullscreen = $fullscreenVisual
      restored_maximized = $restoredVisual
    }
    pane_lifecycle = [ordered]@{
      order_a_stage_first = $paneLifecycle
      order_b_timeline_first_stage_second = $orderBLifecycle
      restart_with_detached_records = [ordered]@{
        previous_process_id = $previousQaProcessId
        new_process_id = $qaProcessId
        maximized_client = $restartedMaximizedClient
        restored_main = $restoredMainState
        restored_stage_child = [ordered]@{ dom = $restoredStageChild; visual = $restoredStageChildVisual }
        restored_timeline_child = [ordered]@{ dom = $restoredTimelineChild; visual = $restoredTimelineChildVisual }
        visual = $restoredMainVisual
      }
      adopt_after_main_reload = [ordered]@{
        instance_ids_before = $instanceIdsBeforeReload
        adopted_main = $adoptedMainState
        visual = $adoptedMainVisual
      }
      direct_stage_child_close_reintegration = [ordered]@{
        main_after_close = $afterDirectCloseState
        visual = $directCloseVisual
      }
      final_fully_reintegrated = [ordered]@{ dom = $finalRestartLegState.main_dom; visual = $finalRestartLegState.main_visual }
    }
    explicitly_unverified_native_boundaries = @(
      "Proven-absent record retirement (untracked popped record retired only after an independent exact-label enumeration proves absence) was not exercised natively: no existing safe seam deterministically produces that state, so it stays explicitly unverified rather than simulated.",
      "Unknown-presence retention (record kept because child presence could not be proven either way) was not exercised natively for the same reason; every restart leg here proves presence or absence through real windows, CDP, or the owned-process census."
    )
    screenshots = @(
      $maximizedScreenshot,
      $fullscreenScreenshot,
      $restoredScreenshot
    ) + @($paneLifecycle.screenshots) + @($orderBLifecycle.screenshots) + @(
      $restoredStageChildScreenshot,
      $restoredTimelineChildScreenshot,
      $restoredMainScreenshot,
      $adoptedMainScreenshot,
      $directCloseScreenshot,
      $finalRestartLegState.screenshots[0]
    )
    compact_browser_fallbacks = @("1366x768", "1280x720")
  }
  $reportPath = Join-Path $EvidenceDir "native-window-acceptance.json"
  $report | ConvertTo-Json -Depth 16 | Set-Content -Path $reportPath -Encoding UTF8
  Write-Host "PASS native 1920 + pane lifecycle acceptance; report $reportPath"
} finally {
  if ($qaWindowVerified -and $qaWindow -ne [IntPtr]::Zero) {
    [void][SyndocalNativeWindow]::PostMessage($qaWindow, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 600
  }
  if ($null -ne $devProcess -and -not $devProcess.HasExited) {
    & taskkill.exe /PID $devProcess.Id /T /F *> $null
  }
  Stop-NativeAcceptanceDevServer -NotBefore $startedAt
  $env:CARGO_TARGET_DIR = $oldTargetDir
  $env:FFMPEG_DIR = $oldFfmpegDir
  $env:PATH = $oldPath
  if ($webView2AdditionalBrowserArgumentsConfigured) {
    [Environment]::SetEnvironmentVariable(
      "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
      $oldWebView2AdditionalBrowserArguments,
      [EnvironmentVariableTarget]::Process
    )
  }
}
