param([int]$AppProcessId,[string]$WindowHandle,[string]$Out)
$ErrorActionPreference='Stop'
$expected=(Resolve-Path target/release/syndocal.exe).Path
$p=Get-Process -Id $AppProcessId
if($p.Path -ne $expected -or $WindowHandle -notmatch '^[1-9][0-9]*$'){throw 'Wrong native output identity'}
$qa=(Resolve-Path target/qa/task20d).Path
$destination=[IO.Path]::GetFullPath($Out)
if(!$destination.StartsWith($qa+[IO.Path]::DirectorySeparatorChar) -or !$destination.EndsWith('-native-display.png')){throw 'QA screenshot destination required'}
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;using System.Runtime.InteropServices;using System.Text;
public class OutputCapture {
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
[StructLayout(LayoutKind.Sequential)] public struct POINT { public int X,Y; }
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
[DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h,StringBuilder s,int n);
[DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h,out RECT r);
[DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h,ref POINT p);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
}
"@
$h=[IntPtr]::new([long]$WindowHandle);$owner=0
[void][OutputCapture]::GetWindowThreadProcessId($h,[ref]$owner)
$title=New-Object Text.StringBuilder 256;[void][OutputCapture]::GetWindowText($h,$title,256)
if($owner -ne $AppProcessId -or $title.ToString() -notmatch 'QA Task20 Display' -or ![OutputCapture]::IsWindowVisible($h)){throw 'Visible app-owned QA Display required'}
$old=[OutputCapture]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))
try{
 $r=New-Object OutputCapture+RECT;[void][OutputCapture]::GetClientRect($h,[ref]$r)
 $pt=New-Object OutputCapture+POINT;[void][OutputCapture]::ClientToScreen($h,[ref]$pt)
 $screens=[System.Windows.Forms.Screen]::AllScreens
 if($screens.Count -ne 1 -or !$screens[0].Primary){throw 'Primary-only topology required'}
 $w=$r.R-$r.L;$height=$r.B-$r.T
 $rectangle=[Drawing.Rectangle]::new($pt.X,$pt.Y,$w,$height)
 if($w -ne 1920 -or $height -ne 1080 -or !$screens[0].Bounds.Contains($rectangle)){throw 'Fully visible 1080p QA client required'}
 [void][OutputCapture]::SetForegroundWindow($h);Start-Sleep -Milliseconds 400
 if([OutputCapture]::GetForegroundWindow() -ne $h){throw 'QA output is not foreground; screenshot would be ambiguous'}
 $bitmap=[Drawing.Bitmap]::new($w,$height);$graphics=[Drawing.Graphics]::FromImage($bitmap)
 try{
  $graphics.CopyFromScreen($pt.X,$pt.Y,0,0,$bitmap.Size)
  $bitmap.Save($destination,[Drawing.Imaging.ImageFormat]::Png)
  $lit=0;$total=0
  for($y=8;$y -lt $height;$y+=32){for($x=8;$x -lt $w;$x+=32){$c=$bitmap.GetPixel($x,$y);$total++;if([Math]::Max($c.R,[Math]::Max($c.G,$c.B)) -gt 16){$lit++}}}
  [pscustomobject]@{processId=$AppProcessId;windowTitle=$title.ToString();width=$w;height=$height;primaryOnly=$true;fullyVisibleClient=$true;afterMeasurement=$true;nonBlackSampleFraction=$lit/[double]$total;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $destination).Hash.ToLowerInvariant();file=$destination} | ConvertTo-Json -Compress
 }finally{$graphics.Dispose();$bitmap.Dispose()}
}finally{[void][OutputCapture]::SetThreadDpiAwarenessContext($old)}
