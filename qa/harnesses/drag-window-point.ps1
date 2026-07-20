# Drag from one window-relative point to another, for exercising sliders during
# local UI verification. Resolves the largest visible window matching the title,
# same as click-window-point.ps1. Read-only intent: only synthesizes one left
# button drag inside the target window.
param(
  [Parameter(Mandatory = $true)][string]$TitlePattern,
  [Parameter(Mandatory = $true)][int]$FromX,
  [Parameter(Mandatory = $true)][int]$FromY,
  [Parameter(Mandatory = $true)][int]$ToX,
  [Parameter(Mandatory = $true)][int]$ToY,
  [int]$Steps = 10,
  [int]$SettleMilliseconds = 400
)

$ErrorActionPreference = "Stop"

if (-not ("DragWindowNative" -as [type])) {
  Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public delegate bool DragWindowEnumProc(IntPtr hWnd, IntPtr lParam);

public static class DragWindowNative {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

  [DllImport("user32.dll")] public static extern bool EnumWindows(DragWindowEnumProc callback, IntPtr lParam);
  [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxLength);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
}

[void][DragWindowNative]::SetProcessDPIAware()

function Get-DragWindowTitle {
  param([IntPtr]$Handle)
  $length = [DragWindowNative]::GetWindowTextLength($Handle)
  if ($length -le 0) { return "" }
  $builder = [Text.StringBuilder]::new($length + 1)
  [void][DragWindowNative]::GetWindowText($Handle, $builder, $builder.Capacity)
  $builder.ToString()
}

$script:dragTarget = [IntPtr]::Zero
$script:dragTargetArea = 0
$script:dragTitlePattern = $TitlePattern
$callback = [DragWindowEnumProc]{
  param([IntPtr]$Handle, [IntPtr]$Unused)
  if ([DragWindowNative]::IsWindowVisible($Handle)) {
    $title = Get-DragWindowTitle -Handle $Handle
    if ($title -match $script:dragTitlePattern) {
      $rect = New-Object DragWindowNative+RECT
      [void][DragWindowNative]::GetWindowRect($Handle, [ref]$rect)
      $area = ($rect.Right - $rect.Left) * ($rect.Bottom - $rect.Top)
      if ($area -gt $script:dragTargetArea) {
        $script:dragTarget = $Handle
        $script:dragTargetArea = $area
      }
    }
  }
  return $true
}
[void][DragWindowNative]::EnumWindows($callback, [IntPtr]::Zero)
if ($script:dragTarget -eq [IntPtr]::Zero) {
  throw "No visible window title matched pattern '$TitlePattern'."
}

[void][DragWindowNative]::SetForegroundWindow($script:dragTarget)
Start-Sleep -Milliseconds $SettleMilliseconds

$rect = New-Object DragWindowNative+RECT
[void][DragWindowNative]::GetWindowRect($script:dragTarget, [ref]$rect)
$sx = $rect.Left + $FromX
$sy = $rect.Top + $FromY
$ex = $rect.Left + $ToX
$ey = $rect.Top + $ToY

[void][DragWindowNative]::SetCursorPos($sx, $sy)
Start-Sleep -Milliseconds 120
[DragWindowNative]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)  # left down
Start-Sleep -Milliseconds 80
for ($step = 1; $step -le $Steps; $step++) {
  $ix = [int]($sx + ($ex - $sx) * $step / $Steps)
  $iy = [int]($sy + ($ey - $sy) * $step / $Steps)
  [void][DragWindowNative]::SetCursorPos($ix, $iy)
  Start-Sleep -Milliseconds 35
}
Start-Sleep -Milliseconds 60
[DragWindowNative]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)  # left up
Write-Host "dragged window-relative ($FromX,$FromY) -> ($ToX,$ToY) screen ($sx,$sy)->($ex,$ey)"
