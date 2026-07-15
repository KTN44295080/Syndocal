# Capture a visible desktop window to PNG for competitive UI comparison.
# Read-only: brings the target window to the foreground and photographs its
# on-screen rectangle. It never sends input to the captured application.
#
# Usage:
#   pwsh qa/harnesses/capture-window.ps1 -TitlePattern "Daslight" -OutPath target/qa/ui-comparison/daslight.png
#   pwsh qa/harnesses/capture-window.ps1 -ListWindows
param(
  [string]$TitlePattern,
  [string]$OutPath,
  [int]$SettleMilliseconds = 1200,
  [switch]$ListWindows
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

if (-not ("CompetitiveCaptureNative" -as [type])) {
  Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public delegate bool CompetitiveCaptureEnumProc(IntPtr hWnd, IntPtr lParam);

public static class CompetitiveCaptureNative {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

  [DllImport("user32.dll")]
  public static extern bool EnumWindows(CompetitiveCaptureEnumProc callback, IntPtr lParam);

  [DllImport("user32.dll")]
  public static extern int GetWindowTextLength(IntPtr hWnd);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxLength);

  [DllImport("user32.dll")]
  public static extern bool IsWindowVisible(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool IsIconic(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern bool SetForegroundWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool ShowWindowAsync(IntPtr hWnd, int command);

  [DllImport("user32.dll")]
  public static extern bool SetProcessDPIAware();
}
"@
}

# Physical-pixel coordinates so captures and window rects agree on scaled monitors.
[void][CompetitiveCaptureNative]::SetProcessDPIAware()

function Get-CaptureWindowTitle {
  param([IntPtr]$Handle)
  $length = [CompetitiveCaptureNative]::GetWindowTextLength($Handle)
  if ($length -le 0) { return "" }
  $builder = [Text.StringBuilder]::new($length + 1)
  [void][CompetitiveCaptureNative]::GetWindowText($Handle, $builder, $builder.Capacity)
  $builder.ToString()
}

$script:captureWindows = [System.Collections.Generic.List[pscustomobject]]::new()
$callback = [CompetitiveCaptureEnumProc]{
  param([IntPtr]$Handle, [IntPtr]$Unused)
  if ([CompetitiveCaptureNative]::IsWindowVisible($Handle)) {
    $title = Get-CaptureWindowTitle -Handle $Handle
    if ($title.Trim().Length -gt 0) {
      $rect = New-Object CompetitiveCaptureNative+RECT
      [void][CompetitiveCaptureNative]::GetWindowRect($Handle, [ref]$rect)
      $script:captureWindows.Add([pscustomobject]@{
        Handle = $Handle
        Title = $title
        Width = $rect.Right - $rect.Left
        Height = $rect.Bottom - $rect.Top
      })
    }
  }
  return $true
}
[void][CompetitiveCaptureNative]::EnumWindows($callback, [IntPtr]::Zero)

if ($ListWindows) {
  $script:captureWindows | Where-Object { $_.Width -gt 200 -and $_.Height -gt 150 } |
    Sort-Object -Property Width -Descending |
    Format-Table -AutoSize Title, Width, Height
  return
}

if (-not $TitlePattern -or -not $OutPath) {
  throw "Provide -TitlePattern and -OutPath, or -ListWindows."
}

$target = $script:captureWindows |
  Where-Object { $_.Title -match $TitlePattern -and $_.Width -gt 200 -and $_.Height -gt 150 } |
  Sort-Object -Property { $_.Width * $_.Height } -Descending |
  Select-Object -First 1
if (-not $target) {
  throw "No visible window title matched pattern '$TitlePattern'."
}

# Restore if minimized (SW_RESTORE = 9), then bring to front and let it repaint.
if ([CompetitiveCaptureNative]::IsIconic($target.Handle)) {
  [void][CompetitiveCaptureNative]::ShowWindowAsync($target.Handle, 9)
  Start-Sleep -Milliseconds 700
}
[void][CompetitiveCaptureNative]::SetForegroundWindow($target.Handle)
Start-Sleep -Milliseconds $SettleMilliseconds

$rect = New-Object CompetitiveCaptureNative+RECT
[void][CompetitiveCaptureNative]::GetWindowRect($target.Handle, [ref]$rect)
$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top
if ($width -le 0 -or $height -le 0) { throw "Window '$($target.Title)' has an empty rectangle." }

$resolvedOut = [System.IO.Path]::GetFullPath($OutPath)
$outDirectory = Split-Path -Parent $resolvedOut
if ($outDirectory) { New-Item -ItemType Directory -Force -Path $outDirectory | Out-Null }

$bitmap = [System.Drawing.Bitmap]::new($width, $height)
try {
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  try {
    $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, [System.Drawing.Size]::new($width, $height))
  } finally {
    $graphics.Dispose()
  }
  $bitmap.Save($resolvedOut, [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
  $bitmap.Dispose()
}

Write-Host "captured '$($target.Title)' ($width x $height) -> $resolvedOut"
