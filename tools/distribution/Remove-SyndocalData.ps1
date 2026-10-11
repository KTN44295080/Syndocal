param([switch]$ValidateOnly)
$ErrorActionPreference = 'Stop'
# The installer removes binaries, associations and shortcuts. This opt-in tool
# removes only this user's product-owned state; never external .sdc/media files.
if (Get-Process -Name syndocal -ErrorAction SilentlyContinue) {
    throw 'Close every Syndocal instance before removing its data.'
}
$taskTargets = New-Object 'System.Collections.Generic.List[string]'
foreach ($taskRoot in @($env:APPDATA, $env:LOCALAPPDATA)) {
    if (-not $taskRoot -or -not [IO.Path]::IsPathRooted($taskRoot)) { throw 'Invalid application data root' }
    $taskTargets.Add((Join-Path $taskRoot 'jp.seraf.ktn.syndocal'))
}
if (-not $env:TEMP -or -not [IO.Path]::IsPathRooted($env:TEMP)) { throw 'Invalid temporary root' }
$taskTargets.Add((Join-Path $env:TEMP 'Syndocal\crash-reports'))
foreach ($taskEntry in Get-ChildItem -LiteralPath $env:TEMP -Force) {
    if ($taskEntry.Name -cmatch '^syndocal-media-snapshot-[0-9]+-[0-9a-f]+-[0-9a-f]+$' -or
        $taskEntry.Name -cmatch '^syndocal-gdtf-share-[0-9]+-[0-9]+\.cookies$') {
        $taskTargets.Add($taskEntry.FullName)
    }
}
# Preflight ALL paths before any deletion. Reject junctions/symlinks in the
# target, ancestors, and children instead of following redirected data.
foreach ($taskTarget in $taskTargets) {
    $taskAbsolute = [IO.Path]::GetFullPath($taskTarget)
    $taskUnderRoot = $false
    foreach ($taskRoot in @($env:APPDATA, $env:LOCALAPPDATA, $env:TEMP)) {
        $taskPrefix = [IO.Path]::GetFullPath($taskRoot).TrimEnd('\') + '\'
        if ($taskAbsolute.StartsWith($taskPrefix, [StringComparison]::OrdinalIgnoreCase)) { $taskUnderRoot = $true }
    }
    if (-not $taskUnderRoot) { throw 'Cleanup target escaped its root' }
    $taskCursor = $taskAbsolute
    while ($taskCursor) {
        if (Test-Path -LiteralPath $taskCursor) {
            $taskItem = Get-Item -LiteralPath $taskCursor -Force
            if ($taskItem.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Redirected cleanup path rejected' }
        }
        $taskCursor = [IO.Path]::GetDirectoryName($taskCursor)
    }
    if (Test-Path -LiteralPath $taskAbsolute) {
        foreach ($taskItem in Get-ChildItem -LiteralPath $taskAbsolute -Force -Recurse) {
            if ($taskItem.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Redirected cleanup content rejected' }
        }
    }
}
if ($ValidateOnly) { Write-Output 'PASS: cleanup paths verified; nothing removed.'; exit 0 }

# OS credentials are not files inside AppData. Remove only exact owned targets.
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class SyndocalUninstallCredentials {
 [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
 public struct Credential { public uint Flags; public uint Type; public string TargetName; }
 [DllImport("advapi32.dll", EntryPoint="CredEnumerateW", CharSet=CharSet.Unicode, SetLastError=true)]
 public static extern bool Enumerate(string filter, uint flags, out uint count, out IntPtr credentials);
 [DllImport("advapi32.dll", EntryPoint="CredDeleteW", CharSet=CharSet.Unicode, SetLastError=true)]
 public static extern bool Delete(string target, uint type, uint flags);
 [DllImport("advapi32.dll", EntryPoint="CredFree")] public static extern void Free(IntPtr buffer);
}
'@
$taskCredentialNames = New-Object 'System.Collections.Generic.List[string]'
$taskCredentialNames.Add('jp.seraf.ktn.syndocal/dj-link/v1')
$taskCount = [uint32]0; $taskCredentials = [IntPtr]::Zero
if ([SyndocalUninstallCredentials]::Enumerate('Syndocal/AgentAuthority/v1/*', 0, [ref]$taskCount, [ref]$taskCredentials)) {
    try {
        for ($taskIndex = 0; $taskIndex -lt $taskCount; ++$taskIndex) {
            $taskPointer = [Runtime.InteropServices.Marshal]::ReadIntPtr($taskCredentials, $taskIndex * [IntPtr]::Size)
            $taskCredential = [Runtime.InteropServices.Marshal]::PtrToStructure($taskPointer, [type][SyndocalUninstallCredentials+Credential])
            if ($taskCredential.Type -eq 1 -and $taskCredential.TargetName -cmatch '^Syndocal/AgentAuthority/v1/[0-9a-f]{64}$') {
                $taskCredentialNames.Add($taskCredential.TargetName)
            }
        }
    } finally { [SyndocalUninstallCredentials]::Free($taskCredentials) }
} elseif ([Runtime.InteropServices.Marshal]::GetLastWin32Error() -ne 1168) { throw 'Credential enumeration failed' }
foreach ($taskName in $taskCredentialNames) {
    if (-not [SyndocalUninstallCredentials]::Delete($taskName, 1, 0) -and
        [Runtime.InteropServices.Marshal]::GetLastWin32Error() -ne 1168) { throw 'Credential deletion failed' }
}
foreach ($taskTarget in $taskTargets) {
    if (Test-Path -LiteralPath $taskTarget) { Remove-Item -LiteralPath $taskTarget -Recurse -Force }
    if (Test-Path -LiteralPath $taskTarget) { throw 'Cleanup did not remove an owned path' }
}
$taskCrashParent = Join-Path $env:TEMP 'Syndocal'
if ((Test-Path -LiteralPath $taskCrashParent -PathType Container) -and
    @(Get-ChildItem -LiteralPath $taskCrashParent -Force).Count -eq 0) {
    Remove-Item -LiteralPath $taskCrashParent
}
Write-Output 'Syndocal settings, caches, recovery files and owned credentials removed. External projects/media preserved.'
