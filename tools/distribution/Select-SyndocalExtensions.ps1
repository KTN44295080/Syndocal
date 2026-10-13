# MSI upgrade wrapper: selection belongs to installation, before the app starts.
function Get-SyndocalExtensionSettingsPath {
    if (-not $env:LOCALAPPDATA -or -not [IO.Path]::IsPathRooted($env:LOCALAPPDATA)) { throw 'Invalid settings root' }
    $taskPath = Join-Path $env:LOCALAPPDATA 'jp.seraf.ktn.syndocal\extensions.ini'
    $taskCursor = $taskPath
    while ($taskCursor) {
        if ((Test-Path -LiteralPath $taskCursor) -and ((Get-Item -LiteralPath $taskCursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Redirected extension settings rejected' }
        $taskCursor = [IO.Path]::GetDirectoryName($taskCursor)
    }
    if (Test-Path -LiteralPath $taskPath) {
        if ((Get-Item -LiteralPath $taskPath).Length -gt 4096) { throw 'Extension settings exceed their size limit' }
        $taskLines = @((Get-Content -LiteralPath $taskPath -Raw).Split("`n") | ForEach-Object { $_.Trim() } | Where-Object { $_ })
        if ($taskLines.Count -ne 3 -or $taskLines[0] -cne '[SyndocalExtensions]' -or $taskLines[1] -cne 'Schema=1' -or $taskLines[2] -cnotmatch '^DJLink=[01]$') { throw 'Unsupported extension settings; nothing replaced' }
    }
    return $taskPath
}
function Select-SyndocalExtensions {
    $taskPath = Get-SyndocalExtensionSettingsPath
    Add-Type -AssemblyName System.Windows.Forms,System.Drawing
    $taskForm = New-Object System.Windows.Forms.Form
    $taskForm.Text = 'Syndocal - Extensions / 拡張機能'
    $taskForm.ClientSize = New-Object Drawing.Size(560,180)
    $taskForm.StartPosition = 'CenterScreen'
    $taskForm.FormBorderStyle = 'FixedDialog'
    $taskForm.MaximizeBox = $false
    $taskForm.MinimizeBox = $false
    $taskForm.Font = New-Object Drawing.Font('Segoe UI',11)
    $taskText = New-Object System.Windows.Forms.Label
    $taskText.SetBounds(20,20,520,45)
    $taskText.Text = 'DMX・映像・MIDIは本体に含まれます。 / Core app included.'
    $taskCheckbox = New-Object System.Windows.Forms.CheckBox
    $taskCheckbox.SetBounds(20,70,520,30)
    $taskCheckbox.Text = 'DJ Link - DJ用PCとの連携 / DJ computer integration'
    $taskCheckbox.Checked = (Test-Path -LiteralPath $taskPath) -and ((Get-Content -LiteralPath $taskPath -Raw) -match 'DJLink=1')
    $taskContinue = New-Object System.Windows.Forms.Button
    $taskContinue.SetBounds(330,125,105,35)
    $taskContinue.Text = 'Next / 次へ'
    $taskContinue.DialogResult = [Windows.Forms.DialogResult]::OK
    $taskCancel = New-Object System.Windows.Forms.Button
    $taskCancel.SetBounds(440,125,100,35)
    $taskCancel.Text = 'Cancel'
    $taskCancel.DialogResult = [Windows.Forms.DialogResult]::Cancel
    $taskForm.Controls.AddRange(@($taskText,$taskCheckbox,$taskContinue,$taskCancel))
    $taskForm.AcceptButton = $taskContinue
    $taskForm.CancelButton = $taskCancel
    try {
        if ($taskForm.ShowDialog() -ne [Windows.Forms.DialogResult]::OK) { return $null }
        return [bool]$taskCheckbox.Checked
    } finally { $taskForm.Dispose() }
}
function Save-SyndocalExtensionChoice([bool]$Enabled) {
    $taskPath = Get-SyndocalExtensionSettingsPath
    $taskParent = [IO.Path]::GetDirectoryName($taskPath)
    [IO.Directory]::CreateDirectory($taskParent) | Out-Null
    $taskTemporary = Join-Path $taskParent ('extensions-' + [guid]::NewGuid().ToString('N') + '.pending')
    try {
        [IO.File]::WriteAllText($taskTemporary,"[SyndocalExtensions]`nSchema=1`nDJLink=$([int]$Enabled)`n",(New-Object Text.UTF8Encoding($false)))
        if (Test-Path -LiteralPath $taskPath) { if (-not ('SyndocalInstallerExtensionFile' -as [type])) {
            Add-Type -TypeDefinition 'public static class SyndocalInstallerExtensionFile { public static void Replace(string source, string target) { System.IO.File.Replace(source, target, null); } }'
        }
        [SyndocalInstallerExtensionFile]::Replace($taskTemporary,$taskPath) }
        else { [IO.File]::Move($taskTemporary,$taskPath) }
    } finally { if (Test-Path -LiteralPath $taskTemporary) { Remove-Item -LiteralPath $taskTemporary -Force } }
}
