param([int]$AppProcessId,[int]$Seconds=300)
$ErrorActionPreference='Stop'
$expected=(Resolve-Path 'target/release/syndocal.exe').Path
if((Get-Process -Id $AppProcessId).Path -ne $expected){throw 'Wrong app process'}
Add-Type @"
using System;using System.Runtime.InteropServices;using System.Text;
public class PerfDialog{
public delegate bool EnumProc(IntPtr h,IntPtr l);
[DllImport("user32.dll")]public static extern bool EnumWindows(EnumProc cb,IntPtr l);
[DllImport("user32.dll")]public static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
[DllImport("user32.dll",CharSet=CharSet.Unicode)]public static extern int GetClassName(IntPtr h,StringBuilder s,int n);
[DllImport("user32.dll",CharSet=CharSet.Unicode)]public static extern int GetWindowText(IntPtr h,StringBuilder s,int n);
[DllImport("user32.dll")]public static extern bool PostMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
}
"@
$title1=[string]::new([char[]](0x51fa,0x529b,0x5236,0x5fa1,0x306e,0x78ba,0x8a8d))
$title2=[string]::new([char[]](0x30a8,0x30c7,0x30a3,0x30bf,0x753b,0x9762,0x3078,0x306e,0x51fa,0x529b,0x78ba,0x8a8d))
$until=(Get-Date).AddSeconds($Seconds)
while((Get-Date) -lt $until -and (Get-Process -Id $AppProcessId -ErrorAction SilentlyContinue)){
 $cb=[PerfDialog+EnumProc]{param($h,$l)
  $owner=0;[void][PerfDialog]::GetWindowThreadProcessId($h,[ref]$owner)
  if($owner -eq $AppProcessId){
   $c=New-Object Text.StringBuilder 256;$t=New-Object Text.StringBuilder 256
   [void][PerfDialog]::GetClassName($h,$c,256);[void][PerfDialog]::GetWindowText($h,$t,256)
   if($c.ToString() -eq '#32770' -and ($t.ToString() -eq $title1 -or $t.ToString() -eq $title2 -or $t.ToString() -match 'Output control|editor.*output')){
    [void][PerfDialog]::PostMessage($h,0x111,[IntPtr]6,[IntPtr]::Zero)
   }
  };return $true
 }
 [void][PerfDialog]::EnumWindows($cb,[IntPtr]::Zero);Start-Sleep -Milliseconds 300
}
