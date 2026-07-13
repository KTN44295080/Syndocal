[CmdletBinding()]
param(
  [string]$MinimumMaximizedClient = "1920x1000",
  [string]$ExpectedFullscreen = "1920x1080",
  [ValidateRange(0, 16)]
  [int]$TolerancePx = 2,
  [ValidateRange(30, 900)]
  [int]$StartupTimeoutSeconds = 420,
  [string]$EvidenceDir = ""
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
    [int]$TimeoutSeconds = 20
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
    if ($largeEnough -and $currentKey -eq $lastKey) {
      $stableMatches += 1
      if ($stableMatches -ge 3) {
        return $last
      }
    } elseif ($largeEnough) {
      $stableMatches = 1
    } else {
      $stableMatches = 0
    }
    $lastKey = $currentKey
    Start-Sleep -Milliseconds 200
  }
  throw "Timed out waiting for at least $($Minimum.Width)x$($Minimum.Height); last client was $($last.Width)x$($last.Height)."
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
$ffmpegDir = $env:FFMPEG_DIR
if ([string]::IsNullOrWhiteSpace($ffmpegDir)) {
  $localSdkRoot = "C:\temp\ffmpeg-n8.1-lgpl-shared"
  if (Test-Path $localSdkRoot) {
    $ffmpegDir = Get-ChildItem $localSdkRoot -Directory -Filter "ffmpeg-n8.1-*-win64-lgpl-shared*" |
      Select-Object -First 1 -ExpandProperty FullName
  }
}
if (
  [string]::IsNullOrWhiteSpace($ffmpegDir) -or
  -not (Test-Path (Join-Path $ffmpegDir "include")) -or
  -not (Test-Path (Join-Path $ffmpegDir "lib")) -or
  -not (Test-Path (Join-Path $ffmpegDir "bin"))
) {
  throw "Set FFMPEG_DIR to the LGPL shared SDK root (include/lib/bin) before native acceptance."
}
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
  $preexistingQaWindow = Find-WindowByTitle -Title $qaTitle
  if ($preexistingQaWindow -ne [IntPtr]::Zero) {
    throw "A pre-existing '$qaTitle' window is open. Close that stale QA instance before collecting current-revision evidence."
  }
  Write-Host "NATIVE PRIMARY GATE: maximized client >= $MinimumMaximizedClient -> F11 fullscreen $ExpectedFullscreen -> Esc exact restore"
  Write-Host "Evidence: $EvidenceDir"
  $devProcess = Start-Process -FilePath $pnpm.Source `
    -ArgumentList @("exec", "tauri", "dev", "--config", "src-tauri/tauri.native-acceptance.conf.json", "--no-watch") `
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

  [void][SyndocalNativeWindow]::ShowWindowAsync($qaWindow, 3)
  Start-Sleep -Seconds 2
  $monitorSize = Get-MonitorDimensions -Handle $qaWindow
  if (-not (Test-Dimensions -Actual $monitorSize -Expected $expectedFullscreenSize -AllowedTolerancePx 0)) {
    throw "The QA window monitor is $($monitorSize.Width)x$($monitorSize.Height), but the primary gate requires $ExpectedFullscreen."
  }

  $maximized = Wait-ForMinimumClientDimensions -Handle $qaWindow -Minimum $minimumMaximizedSize
  if (-not [SyndocalNativeWindow]::IsZoomed($qaWindow)) {
    throw "The QA window reached $($maximized.Width)x$($maximized.Height) but Windows does not report it as maximized."
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

  $report = [ordered]@{
    gate = "windows-native-primary-1920"
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
    screenshots = @($maximizedScreenshot, $fullscreenScreenshot, $restoredScreenshot)
    compact_browser_fallbacks = @("1366x768", "1280x720")
  }
  $reportPath = Join-Path $EvidenceDir "native-window-acceptance.json"
  $report | ConvertTo-Json -Depth 5 | Set-Content -Path $reportPath -Encoding UTF8
  Write-Host "PASS native 1920 acceptance; report $reportPath"
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
}
