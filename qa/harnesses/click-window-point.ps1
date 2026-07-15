# Send one left click at a window-relative point, for read-only view switching
# during competitive UI capture (e.g. selecting a tab). Coordinates are relative
# to the window rectangle captured by capture-window.ps1.
param(
  [Parameter(Mandatory = $true)][string]$TitlePattern,
  [Parameter(Mandatory = $true)][int]$RelativeX,
  [Parameter(Mandatory = $true)][int]$RelativeY,
  [int]$SettleMilliseconds = 900
)

$ErrorActionPreference = "Stop"

if (-not ("CompetitiveClickNative" -as [type])) {
  Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public delegate bool CompetitiveClickEnumProc(IntPtr hWnd, IntPtr lParam);

public static class CompetitiveClickNative {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

  [DllImport("user32.dll")]
  public static extern bool EnumWindows(CompetitiveClickEnumProc callback, IntPtr lParam);

  [DllImport("user32.dll")]
  public static extern int GetWindowTextLength(IntPtr hWnd);

  [DllImport("user32.dll", CharSet = CharSet.Unicode)]
  public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxLength);

  [DllImport("user32.dll")]
  public static extern bool IsWindowVisible(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern bool SetForegroundWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool SetCursorPos(int x, int y);

  [DllImport("user32.dll")]
  public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extraInfo);

  [DllImport("user32.dll")]
  public static extern bool SetProcessDPIAware();
}
"@
}

# Physical-pixel coordinates so window rects match captured bitmaps on scaled monitors.
[void][CompetitiveClickNative]::SetProcessDPIAware()

function Get-ClickWindowTitle {
  param([IntPtr]$Handle)
  $length = [CompetitiveClickNative]::GetWindowTextLength($Handle)
  if ($length -le 0) { return "" }
  $builder = [Text.StringBuilder]::new($length + 1)
  [void][CompetitiveClickNative]::GetWindowText($Handle, $builder, $builder.Capacity)
  $builder.ToString()
}

$script:clickTarget = [IntPtr]::Zero
$script:clickTargetArea = 0
$script:clickTitlePattern = $TitlePattern
$callback = [CompetitiveClickEnumProc]{
  param([IntPtr]$Handle, [IntPtr]$Unused)
  if ([CompetitiveClickNative]::IsWindowVisible($Handle)) {
    $title = Get-ClickWindowTitle -Handle $Handle
    if ($title -match $script:clickTitlePattern) {
      $rect = New-Object CompetitiveClickNative+RECT
      [void][CompetitiveClickNative]::GetWindowRect($Handle, [ref]$rect)
      $area = ($rect.Right - $rect.Left) * ($rect.Bottom - $rect.Top)
      if ($area -gt $script:clickTargetArea) {
        $script:clickTarget = $Handle
        $script:clickTargetArea = $area
      }
    }
  }
  return $true
}
[void][CompetitiveClickNative]::EnumWindows($callback, [IntPtr]::Zero)
if ($script:clickTarget -eq [IntPtr]::Zero) {
  throw "No visible window title matched pattern '$TitlePattern'."
}

[void][CompetitiveClickNative]::SetForegroundWindow($script:clickTarget)
Start-Sleep -Milliseconds $SettleMilliseconds

$rect = New-Object CompetitiveClickNative+RECT
[void][CompetitiveClickNative]::GetWindowRect($script:clickTarget, [ref]$rect)
$x = $rect.Left + $RelativeX
$y = $rect.Top + $RelativeY
[void][CompetitiveClickNative]::SetCursorPos($x, $y)
Start-Sleep -Milliseconds 150
[CompetitiveClickNative]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)  # left down
Start-Sleep -Milliseconds 60
[CompetitiveClickNative]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)  # left up
Write-Host "clicked window-relative ($RelativeX,$RelativeY) -> screen ($x,$y)"
