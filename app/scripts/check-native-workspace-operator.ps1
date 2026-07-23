[CmdletBinding()]
param(
  [ValidateRange(30, 900)]
  [int]$StartupTimeoutSeconds = 420,
  [string]$EvidenceDir = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
  throw "The native workspace acceptance currently requires Windows."
}

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public delegate bool SyndocalWorkspaceEnumProc(IntPtr hWnd, IntPtr lParam);

public static class SyndocalWorkspaceWindow {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

  [DllImport("user32.dll")]
  public static extern bool EnumWindows(SyndocalWorkspaceEnumProc callback, IntPtr lParam);

  [DllImport("user32.dll")]
  public static extern bool IsWindowVisible(IntPtr hWnd);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);

  [DllImport("user32.dll")]
  public static extern int GetWindowTextLength(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

  [DllImport("user32.dll")]
  public static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);
}
"@

function Get-VisibleWindows {
  $windows = [Collections.Generic.List[object]]::new()
  $callback = [SyndocalWorkspaceEnumProc]{
    param([IntPtr]$Handle, [IntPtr]$Unused)
    if (-not [SyndocalWorkspaceWindow]::IsWindowVisible($Handle)) { return $true }
    $length = [SyndocalWorkspaceWindow]::GetWindowTextLength($Handle)
    if ($length -le 0) { return $true }
    $title = [Text.StringBuilder]::new($length + 1)
    [void][SyndocalWorkspaceWindow]::GetWindowText($Handle, $title, $title.Capacity)
    $rect = [SyndocalWorkspaceWindow+RECT]::new()
    if (-not [SyndocalWorkspaceWindow]::GetWindowRect($Handle, [ref]$rect)) { return $true }
    [uint32]$processId = 0
    [void][SyndocalWorkspaceWindow]::GetWindowThreadProcessId($Handle, [ref]$processId)
    $windows.Add([pscustomobject]@{
      Handle = $Handle
      Title = $title.ToString()
      ProcessId = $processId
      X = $rect.Left
      Y = $rect.Top
      Width = $rect.Right - $rect.Left
      Height = $rect.Bottom - $rect.Top
    })
    return $true
  }
  [void][SyndocalWorkspaceWindow]::EnumWindows($callback, [IntPtr]::Zero)
  $windows
}

$mainTitle = "Syndocal QA - Workspace Acceptance"
$paneTitles = @(
  "Syndocal Stage - 2D Map",
  "Syndocal Timeline",
  "Syndocal Programmer",
  "Syndocal Setup",
  "Syndocal Live Desk",
  "Syndocal Live Mixer",
  "Syndocal Touch"
)
$allTitles = @($mainTitle) + $paneTitles
$preexisting = @(Get-VisibleWindows | Where-Object { $_.Title -in $allTitles })
if ($preexisting.Count -gt 0) {
  throw "Close the pre-existing Syndocal workspace QA window(s) before collecting current evidence."
}

$scriptDir = Split-Path -Parent $PSCommandPath
$appRoot = (Resolve-Path (Join-Path $scriptDir "..")).Path
if ([string]::IsNullOrWhiteSpace($EvidenceDir)) {
  $stamp = [DateTime]::UtcNow.ToString("yyyyMMdd-HHmmss")
  $EvidenceDir = Join-Path ([IO.Path]::GetTempPath()) "syndocal-workspace-acceptance-$stamp"
}
$EvidenceDir = [IO.Path]::GetFullPath($EvidenceDir)
[void](New-Item -ItemType Directory -Path $EvidenceDir -Force)

$pnpm = Get-Command pnpm.cmd -ErrorAction Stop
$oldTargetDir = $env:CARGO_TARGET_DIR
$oldFfmpegDir = $env:FFMPEG_DIR
$oldPath = $env:PATH
$ffmpegDir = $env:FFMPEG_DIR
if ([string]::IsNullOrWhiteSpace($ffmpegDir)) {
  $sdkRoot = "C:\temp\ffmpeg-n8.1-lgpl-shared"
  if (Test-Path $sdkRoot) {
    $ffmpegDir = Get-ChildItem $sdkRoot -Directory -Filter "ffmpeg-n8.1-*-win64-lgpl-shared*" |
      Select-Object -First 1 -ExpandProperty FullName
  }
}
if (
  [string]::IsNullOrWhiteSpace($ffmpegDir) -or
  -not (Test-Path (Join-Path $ffmpegDir "include")) -or
  -not (Test-Path (Join-Path $ffmpegDir "lib")) -or
  -not (Test-Path (Join-Path $ffmpegDir "bin"))
) {
  throw "Set FFMPEG_DIR to the LGPL shared SDK root before native workspace acceptance."
}

$env:FFMPEG_DIR = $ffmpegDir
$env:PATH = "$(Join-Path $ffmpegDir 'bin');$oldPath"
$env:CARGO_TARGET_DIR = Join-Path ([IO.Path]::GetTempPath()) "syndocal-workspace-acceptance-target"
$devProcess = $null
$mainWindow = $null
$startedAt = [DateTime]::UtcNow

try {
  if (Get-NetTCPConnection -LocalPort 5191 -State Listen -ErrorAction SilentlyContinue) {
    throw "The isolated native workspace QA port 5191 is already in use."
  }
  Write-Host "NATIVE WORKSPACE GATE: seven WebView2 pane windows with applied native placement"
  Write-Host "Evidence: $EvidenceDir"
  $devProcess = Start-Process -FilePath $pnpm.Source `
    -ArgumentList @("exec", "tauri", "dev", "--config", "src-tauri/tauri.workspace-operator-acceptance.conf.json", "--no-watch") `
    -WorkingDirectory $appRoot `
    -PassThru `
    -NoNewWindow

  $deadline = [DateTime]::UtcNow.AddSeconds($StartupTimeoutSeconds)
  $matched = @()
  while ([DateTime]::UtcNow -lt $deadline) {
    $visible = @(Get-VisibleWindows)
    $matched = @($visible | Where-Object { $_.Title -in $allTitles })
    if (@($matched | Select-Object -ExpandProperty Title -Unique).Count -eq $allTitles.Count) { break }
    if ($devProcess.HasExited) {
      throw "The native workspace launcher exited with code $($devProcess.ExitCode) before all pane windows appeared."
    }
    Start-Sleep -Milliseconds 500
  }

  $matched = @($matched | Sort-Object Title)
  $foundTitles = @($matched | Select-Object -ExpandProperty Title -Unique)
  $missingTitles = @($allTitles | Where-Object { $_ -notin $foundTitles })
  if ($missingTitles.Count -gt 0) {
    throw "Timed out waiting for native pane window(s): $($missingTitles -join ', ')."
  }
  if ($matched.Count -ne $allTitles.Count) {
    throw "Expected exactly $($allTitles.Count) isolated QA windows, found $($matched.Count)."
  }
  $processIds = @($matched | Select-Object -ExpandProperty ProcessId -Unique)
  if ($processIds.Count -ne 1) {
    throw "Native pane windows do not belong to one isolated Syndocal process: $($processIds -join ', ')."
  }
  $tooSmall = @($matched | Where-Object { $_.Width -lt 320 -or $_.Height -lt 240 })
  if ($tooSmall.Count -gt 0) {
    throw "Native pane window geometry is below the supported minimum: $($tooSmall.Title -join ', ')."
  }

  $expectedPlacements = @{}
  for ($index = 0; $index -lt $paneTitles.Count; $index += 1) {
    $expectedPlacements[$paneTitles[$index]] = [pscustomobject]@{
      X = 48 + $index * 42
      Y = 64 + $index * 34
    }
  }
  $misplaced = @($matched | Where-Object {
    if (-not $expectedPlacements.ContainsKey($_.Title)) { return $false }
    $expected = $expectedPlacements[$_.Title]
    [Math]::Abs($_.X - $expected.X) -gt 16 -or
      [Math]::Abs($_.Y - $expected.Y) -gt 16 -or
      $_.Width -lt 840 -or $_.Width -gt 900 -or
      $_.Height -lt 500 -or $_.Height -gt 580
  })
  if ($misplaced.Count -gt 0) {
    throw "Native pane placement was not applied: $($misplaced.Title -join ', ')."
  }

  $mainWindow = $matched | Where-Object Title -eq $mainTitle | Select-Object -First 1
  $report = [ordered]@{
    gate = "windows-native-workspace-panes"
    started_at_utc = $startedAt.ToString("o")
    completed_at_utc = [DateTime]::UtcNow.ToString("o")
    process_id = $processIds[0]
    pane_count = $paneTitles.Count
    windows = @($matched | ForEach-Object {
      [ordered]@{ title = $_.Title; x = $_.X; y = $_.Y; width = $_.Width; height = $_.Height }
    })
  }
  $reportPath = Join-Path $EvidenceDir "native-workspace-acceptance.json"
  $report | ConvertTo-Json -Depth 5 | Set-Content -Path $reportPath -Encoding UTF8
  Write-Host "PASS native WebView2 workspace windows: $($paneTitles.Count)/$($paneTitles.Count); report $reportPath"
} finally {
  if ($null -ne $mainWindow) {
    [void][SyndocalWorkspaceWindow]::PostMessage($mainWindow.Handle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 500
  }
  if ($null -ne $devProcess -and -not $devProcess.HasExited) {
    & taskkill.exe /PID $devProcess.Id /T /F *> $null
  }
  $env:CARGO_TARGET_DIR = $oldTargetDir
  $env:FFMPEG_DIR = $oldFfmpegDir
  $env:PATH = $oldPath
}
