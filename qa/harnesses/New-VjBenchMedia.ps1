<#
.SYNOPSIS
Generates the VJ benchmark media pack (testsrc2 clips with a frame counter).

.DESCRIPTION
Every clip is 12 s of FFmpeg's testsrc2 pattern with a burnt-in frame
number, so a rendered frame can be traced to its source frame. Only
encoders present in an LGPL FFmpeg build are used (libopenh264, the GPU
H.264/HEVC encoders, prores_ks, hap). A manifest with the SHA-256 of every
file is written next to the media, so a run can name exactly what it read.

.PARAMETER OutputDirectory
Where the media goes (default: %LOCALAPPDATA%\SyndocalDev\vj-bench-media).

.PARAMETER Ffmpeg
The ffmpeg executable (default: ffmpeg on PATH).
#>
param(
  [string]$OutputDirectory = (Join-Path $env:LOCALAPPDATA "SyndocalDev\vj-bench-media"),
  [string]$Ffmpeg = "ffmpeg"
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null

$encoders = (& $Ffmpeg -hide_banner -encoders 2>$null) -join "`n"
function Test-Encoder([string]$Name) { return $encoders -match "\s$Name\s" }

$h264 = if (Test-Encoder "libopenh264") { @("-c:v", "libopenh264", "-b:v", "12M") }
        elseif (Test-Encoder "h264_amf") { @("-c:v", "h264_amf", "-b:v", "12M") }
        elseif (Test-Encoder "h264_mf") { @("-c:v", "h264_mf", "-b:v", "12M") }
        else { throw "No H.264 encoder is available in $Ffmpeg" }
$h264_4k = $h264.Clone(); $h264_4k[3] = "40M"

$clips = @(
  @{ Name = "h264_1080p30.mp4"; Size = "1920x1080"; Rate = 30; Codec = $h264 },
  @{ Name = "h264_1080p60.mp4"; Size = "1920x1080"; Rate = 60; Codec = $h264 },
  @{ Name = "h264_2160p30.mp4"; Size = "3840x2160"; Rate = 30; Codec = $h264_4k },
  @{ Name = "prores422_1080p30.mov"; Size = "1920x1080"; Rate = 30; Codec = @("-c:v", "prores_ks", "-profile:v", "2", "-pix_fmt", "yuv422p10le") }
)
if (Test-Encoder "hevc_amf") {
  $clips += @{ Name = "hevc_1080p30.mp4"; Size = "1920x1080"; Rate = 30; Codec = @("-c:v", "hevc_amf", "-b:v", "8M") }
}
if (Test-Encoder "hap") {
  $clips += @{ Name = "hap_1080p30.mov"; Size = "1920x1080"; Rate = 30; Codec = @("-c:v", "hap", "-format", "hap") }
  $clips += @{ Name = "hapq_1080p30.mov"; Size = "1920x1080"; Rate = 30; Codec = @("-c:v", "hap", "-format", "hap_q") }
}

$manifest = @()
# drawtext needs a font file; Windows has Arial.
$font = Join-Path $env:WINDIR "Fonts\arial.ttf"
$fontOption = if (Test-Path $font) { "fontfile='" + ($font -replace '\\', '/' -replace ':', '\:') + "':" } else { "" }

foreach ($clip in $clips) {
  $path = Join-Path $OutputDirectory $clip.Name
  if (-not (Test-Path $path)) {
    $source = "testsrc2=size=$($clip.Size):rate=$($clip.Rate):duration=12," +
      "drawtext=${fontOption}text='%{frame_num}':x=40:y=40:fontsize=96:fontcolor=white:box=1:boxcolor=black"
    $arguments = @("-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", $source) + $clip.Codec + @("-an", $path)
    & $Ffmpeg @arguments
    if ($LASTEXITCODE -ne 0) {
      # drawtext needs a font; retry without the counter rather than fail.
      $source = "testsrc2=size=$($clip.Size):rate=$($clip.Rate):duration=12"
      $arguments = @("-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", $source) + $clip.Codec + @("-an", $path)
      & $Ffmpeg @arguments
      if ($LASTEXITCODE -ne 0) { throw "ffmpeg failed for $($clip.Name)" }
    }
  }
  $manifest += [ordered]@{
    file = $clip.Name
    size = $clip.Size
    rate = $clip.Rate
    sha256 = (Get-FileHash -Algorithm SHA256 $path).Hash.ToLowerInvariant()
    bytes = (Get-Item $path).Length
  }
}
$manifestPath = Join-Path $OutputDirectory "manifest.json"
$manifest | ConvertTo-Json | ForEach-Object { [System.IO.File]::WriteAllText($manifestPath, $_) }
Write-Host "VJ benchmark media: $($manifest.Count) clips in $OutputDirectory"
