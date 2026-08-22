param(
    [string]$OutputPath = (Join-Path $PSScriptRoot "../../app/src-tauri/assets/timeline-guide/en/complete.wav")
)

$ErrorActionPreference = "Stop"
$voiceName = "Microsoft Zira Desktop"
$rate = 2
$volume = 100
$sampleRate = 22050

Add-Type -AssemblyName System.Speech

$synthesizer = New-Object System.Speech.Synthesis.SpeechSynthesizer
$temporaryPath = $null
$backupPath = $null
try {
    $voice = $synthesizer.GetInstalledVoices() |
        Where-Object { $_.Enabled -and $_.VoiceInfo.Name -ceq $voiceName } |
        Select-Object -First 1
    if ($null -eq $voice) {
        throw "Required Windows voice '$voiceName' is not installed and enabled; existing output was not changed."
    }

    $resolvedOutput = [System.IO.Path]::GetFullPath($OutputPath)
    $outputDirectory = [System.IO.Path]::GetDirectoryName($resolvedOutput)
    [System.IO.Directory]::CreateDirectory($outputDirectory) | Out-Null
    $temporaryPath = Join-Path $outputDirectory ("." + [System.IO.Path]::GetRandomFileName() + ".wav")

    $format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(
        $sampleRate,
        [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen,
        [System.Speech.AudioFormat.AudioChannel]::Mono
    )
    $synthesizer.SelectVoice($voiceName)
    $synthesizer.Rate = $rate
    $synthesizer.Volume = $volume
    $synthesizer.SetOutputToWaveFile($temporaryPath, $format)
    $synthesizer.Speak("Complete")
    $synthesizer.SetOutputToNull()

    $bytes = [System.IO.File]::ReadAllBytes($temporaryPath)
    if ($bytes.Length -lt 44 -or
        [System.Text.Encoding]::ASCII.GetString($bytes, 0, 4) -cne "RIFF" -or
        [System.Text.Encoding]::ASCII.GetString($bytes, 8, 4) -cne "WAVE") {
        throw "Generated Complete asset is not a valid RIFF/WAVE file; existing output was not changed."
    }
    $audioFormat = [System.BitConverter]::ToUInt16($bytes, 20)
    $channels = [System.BitConverter]::ToUInt16($bytes, 22)
    $actualSampleRate = [System.BitConverter]::ToUInt32($bytes, 24)
    $bitsPerSample = [System.BitConverter]::ToUInt16($bytes, 34)
    if ($audioFormat -ne 1 -or $channels -ne 1 -or $actualSampleRate -ne $sampleRate -or $bitsPerSample -ne 16) {
        throw "Generated Complete asset format is not PCM16 mono 22050 Hz; existing output was not changed."
    }
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $temporaryPath).Hash.ToLowerInvariant()

    if ([System.IO.File]::Exists($resolvedOutput)) {
        $backupPath = $resolvedOutput + ".replace-backup"
        if ([System.IO.File]::Exists($backupPath)) {
            [System.IO.File]::Delete($backupPath)
        }
        [System.IO.File]::Replace($temporaryPath, $resolvedOutput, $backupPath)
        $temporaryPath = $null
        try {
            [System.IO.File]::Delete($backupPath)
        } catch {
            # The validated replacement is already committed. A cleanup failure
            # must not turn successful synthesis into a destructive false failure.
        }
        $backupPath = $null
    } else {
        [System.IO.File]::Move($temporaryPath, $resolvedOutput)
        $temporaryPath = $null
    }

    [ordered]@{
        outputPath = $resolvedOutput
        voice = $voiceName
        text = "Complete"
        rate = $rate
        volume = $volume
        codec = "pcm_s16le"
        sampleRate = $sampleRate
        channels = 1
        bitsPerSample = 16
        sha256 = $hash
    } | ConvertTo-Json -Compress
} finally {
    $synthesizer.Dispose()
    if ($null -ne $temporaryPath -and [System.IO.File]::Exists($temporaryPath)) {
        [System.IO.File]::Delete($temporaryPath)
    }
    # On replacement failure, retain any backup as a recovery artifact instead
    # of deleting the only possible copy of the previous output.
}
