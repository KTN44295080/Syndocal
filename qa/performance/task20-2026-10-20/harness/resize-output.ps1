param([int]$AppProcessId,[string]$WindowHandle,[int]$Width,[int]$Height,[switch]$Present)
$ErrorActionPreference='Stop'
$expected=(Resolve-Path target/release/syndocal.exe).Path
$p=Get-Process -Id $AppProcessId
if($p.Path -ne $expected){throw 'Wrong app executable'}
if($WindowHandle -notmatch '^[1-9][0-9]*$' -or $Width -notin 1920,3840 -or $Height -notin 1080,2160){throw 'Invalid QA geometry'}
Add-Type -AssemblyName System.Windows.Forms
if([System.Windows.Forms.Screen]::AllScreens.Count -ne 1 -or ![System.Windows.Forms.Screen]::PrimaryScreen.Primary){throw 'Primary-only QA topology required'}
Add-Type @"
using System;using System.Runtime.InteropServices;using System.Text;
public class PerfGeometry{
public delegate bool EP(IntPtr h,IntPtr l);
[DllImport("user32.dll")]public static extern bool EnumWindows(EP cb,IntPtr l);
[StructLayout(LayoutKind.Sequential)]public struct RECT{public int L,T,R,B;}
[DllImport("user32.dll")]public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
[DllImport("user32.dll",CharSet=CharSet.Unicode)]public static extern int GetWindowText(IntPtr h,StringBuilder s,int n);
[DllImport("user32.dll")]public static extern int GetWindowLong(IntPtr h,int i);
[DllImport("user32.dll")]public static extern bool AdjustWindowRectEx(ref RECT r,uint style,bool menu,uint exstyle);
[DllImport("user32.dll")]public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int w,int height,uint flags);
[DllImport("user32.dll")]public static extern bool GetClientRect(IntPtr h,out RECT r);
[DllImport("user32.dll")]public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")]public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
[DllImport("user32.dll")]public static extern uint GetDpiForWindow(IntPtr h);
[DllImport("user32.dll")]public static extern int GetSystemMetrics(int index);
[DllImport("user32.dll")]public static extern bool AdjustWindowRectExForDpi(ref RECT r,uint style,bool menu,uint exstyle,uint dpi);
public static IntPtr MainWindow(uint pid){var matches=new System.Collections.Generic.List<IntPtr>();EnumWindows((h,l)=>{uint owner;GetWindowThreadProcessId(h,out owner);if(owner==pid){var title=new StringBuilder(256);GetWindowText(h,title,256);if(title.ToString()=="Syndocal")matches.Add(h);}return true;},IntPtr.Zero);if(matches.Count!=1)throw new Exception("Expected exactly one Syndocal main window");return matches[0];}
public static RECT Resize(IntPtr h,int width,int height){
var old=SetThreadDpiAwarenessContext(new IntPtr(-4));
try{var r=new RECT{R=width,B=height};AdjustWindowRectExForDpi(ref r,unchecked((uint)GetWindowLong(h,-16)),false,unchecked((uint)GetWindowLong(h,-20)),GetDpiForWindow(h));var x=width>GetSystemMetrics(0)?Math.Max(32,GetSystemMetrics(0)-160):Math.Max(32,GetSystemMetrics(0)-width-32);SetWindowPos(h,IntPtr.Zero,x,32,r.R-r.L,r.B-r.T,0x0014);System.Threading.Thread.Sleep(350);RECT actual;GetClientRect(h,out actual);return actual;}
finally{SetThreadDpiAwarenessContext(old);}}
}
"@
$h=[IntPtr]::new([long]$WindowHandle);$owner=0
$mainWindow=[PerfGeometry]::MainWindow([uint32]$AppProcessId)
[void][PerfGeometry]::GetWindowThreadProcessId($h,[ref]$owner)
if($owner -ne $AppProcessId -or $h -eq $mainWindow){throw 'Output handle identity mismatch'}
$title=New-Object Text.StringBuilder 256;[void][PerfGeometry]::GetWindowText($h,$title,256)
if($title.ToString() -notmatch 'QA Task20 Display'){throw "Unexpected output title: $title"}
$actual=[PerfGeometry]::Resize($h,$Width,$Height)
if($Present){[void][PerfGeometry]::SetForegroundWindow($h)}else{[void][PerfGeometry]::SetForegroundWindow($mainWindow)}
[pscustomobject]@{title=$title.ToString();width=$actual.R-$actual.L;height=$actual.B-$actual.T;primaryOnly=$true;oversizedQaWindow=$true} | ConvertTo-Json -Compress
if(($actual.R-$actual.L) -ne $Width -or ($actual.B-$actual.T) -ne $Height){throw 'QA client extent mismatch'}
