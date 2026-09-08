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
  public static extern bool GetClientRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

  [DllImport("user32.dll", SetLastError = true)]
  private static extern bool SetProcessDpiAwarenessContext(IntPtr dpiAwarenessContext);

  [DllImport("user32.dll")]
  private static extern IntPtr GetThreadDpiAwarenessContext();

  [DllImport("user32.dll")]
  private static extern int GetAwarenessFromDpiAwarenessContext(IntPtr dpiAwarenessContext);

  [DllImport("shcore.dll", SetLastError = true)]
  private static extern int SetProcessDpiAwareness(int value);

  [DllImport("user32.dll")]
  public static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);

  public static bool EnsurePerMonitorDpiAwareness() {
    // PROCESS_PER_MONITOR_DPI_AWARE = 2; the context value -4 is
    // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2. The fallback covers older
    // Windows 10 builds while the query makes an already-aware process safe.
    if (SetProcessDpiAwarenessContext(new IntPtr(-4))) return true;
    if (GetAwarenessFromDpiAwarenessContext(GetThreadDpiAwarenessContext()) >= 2) return true;
    return SetProcessDpiAwareness(2) == 0;
  }
}
"@

if (-not [SyndocalWorkspaceWindow]::EnsurePerMonitorDpiAwareness()) {
  throw "The native workspace checker could not enable Per-Monitor DPI awareness; refusing virtualized window geometry."
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
    $clientRect = [SyndocalWorkspaceWindow+RECT]::new()
    if (-not [SyndocalWorkspaceWindow]::GetClientRect($Handle, [ref]$clientRect)) { return $true }
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
      ClientWidth = $clientRect.Right - $clientRect.Left
      ClientHeight = $clientRect.Bottom - $clientRect.Top
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
$ffmpegDir = Resolve-FfmpegSdkRoot

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
    -ArgumentList @("tauri", "dev", "--config", "src-tauri/tauri.workspace-operator-acceptance.conf.json", "--no-watch") `
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
  # Tauri's placement width/height are inner (client) dimensions. The outer
  # frame includes DPI-scaled title-bar/border pixels, so validate the client
  # area while retaining the native outer position and minimum-size checks.
  $misplaced = @($matched | Where-Object {
    if (-not $expectedPlacements.ContainsKey($_.Title)) { return $false }
    $expected = $expectedPlacements[$_.Title]
    [Math]::Abs($_.X - $expected.X) -gt 16 -or
      [Math]::Abs($_.Y - $expected.Y) -gt 16 -or
      $_.ClientWidth -lt 840 -or $_.ClientWidth -gt 900 -or
      $_.ClientHeight -lt 500 -or $_.ClientHeight -gt 580
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
      [ordered]@{
        title = $_.Title
        x = $_.X
        y = $_.Y
        width = $_.Width
        height = $_.Height
        client_width = $_.ClientWidth
        client_height = $_.ClientHeight
      }
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
