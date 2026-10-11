function Get-SyndocalInstallations {
    if (-not [Environment]::Is64BitProcess) { throw 'Use the 64-bit Install-Windows.cmd or Uninstall-Windows.cmd.' }
    $taskEntries = @()
    foreach ($taskRoot in @('HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall','HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall')) {
        foreach ($taskKey in Get-ChildItem -LiteralPath $taskRoot -ErrorAction SilentlyContinue) {
            $taskEntry = Get-ItemProperty -LiteralPath $taskKey.PSPath
            if ($taskEntry.DisplayName -ne 'Syndocal') { continue }
            if ($taskEntry.Publisher -ne 'Seraf() / KTN') { throw 'Unrecognized Syndocal publisher; inspect the existing installation before updating/removing.' }
            if ($taskEntry.UninstallString -match '^MsiExec\.exe /[IX]({[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}})$') {
                $taskEntries += [pscustomobject]@{Kind='msi';File=(Join-Path $env:SystemRoot 'System32\msiexec.exe');Args=@('/x',$Matches[1])}
            } elseif ($taskEntry.UninstallString -match '^"(.+\\uninstall\.exe)"$') {
                $taskUninstaller = [IO.Path]::GetFullPath($Matches[1])
                $taskInstall = [IO.Path]::GetFullPath($taskEntry.InstallLocation).TrimEnd('\')
                if ([IO.Path]::GetDirectoryName($taskUninstaller) -ne $taskInstall -or
                    -not (Test-Path -LiteralPath (Join-Path $taskInstall 'syndocal.exe'))) { throw 'Unverified Syndocal uninstall path' }
                $taskEntries += [pscustomobject]@{Kind='nsis';File=$taskUninstaller;Args=@()}
            } else { throw 'Unsupported Syndocal uninstall registration' }
        }
    }
    return $taskEntries
}
