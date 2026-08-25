[CmdletBinding()]
param(
    [ValidateRange(1, 60)]
    [int]$DurationSeconds = 8
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$qaTargetRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot 'target\qa'))
$outputDirectory = [System.IO.Path]::GetFullPath((Join-Path $qaTargetRoot 'near-show-media'))
$pathComparison = [System.StringComparison]::OrdinalIgnoreCase
if (-not $outputDirectory.StartsWith($qaTargetRoot + [System.IO.Path]::DirectorySeparatorChar, $pathComparison)) {
    throw "Refusing to generate media outside the checkout QA target: $outputDirectory"
}

$ffmpeg = Get-Command ffmpeg -ErrorAction Stop
$ffprobe = Get-Command ffprobe -ErrorAction Stop
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null

function Invoke-FfmpegChecked {
    param(
        [Parameter(Mandatory)]
        [string[]]$Arguments
    )

    & $ffmpeg.Source @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "ffmpeg failed with exit code $LASTEXITCODE"
    }
}

function New-TestClip {
    param(
        [Parameter(Mandatory)]
        [string]$FileName,
        [Parameter(Mandatory)]
        [string]$Size,
        [Parameter(Mandatory)]
        [string]$VideoFilter,
        [Parameter(Mandatory)]
        [string]$Title
    )

    $finalPath = [System.IO.Path]::GetFullPath((Join-Path $outputDirectory $FileName))
    $partialPath = "$finalPath.partial.mp4"
    foreach ($candidate in @($finalPath, $partialPath)) {
        if (-not $candidate.StartsWith($outputDirectory + [System.IO.Path]::DirectorySeparatorChar, $pathComparison)) {
            throw "Refusing to replace media outside the generated output directory: $candidate"
        }
        if (Test-Path -LiteralPath $candidate) {
            Remove-Item -LiteralPath $candidate -Force
        }
    }

    Invoke-FfmpegChecked -Arguments @(
        '-hide_banner',
        '-loglevel', 'warning',
        '-f', 'lavfi',
        '-i', "testsrc2=size=$Size`:rate=30",
        '-t', $DurationSeconds.ToString([System.Globalization.CultureInfo]::InvariantCulture),
        '-vf', $VideoFilter,
        '-an',
        '-c:v', 'libx264',
        '-preset', 'ultrafast',
        '-crf', '28',
        '-pix_fmt', 'yuv420p',
        '-g', '30',
        '-movflags', '+faststart',
        '-metadata', "title=$Title",
        '-metadata', 'comment=Syndocal near-show synthetic QA media; not final show artwork',
        '-y',
        $partialPath
    )
    Move-Item -LiteralPath $partialPath -Destination $finalPath
    return $finalPath
}

$ledPath = New-TestClip `
    -FileName 'syndocal-near-show-led-1920x1080.mp4' `
    -Size '1920x1080' `
    -VideoFilter 'drawbox=x=0:y=0:w=iw/4:h=ih/4:color=red@0.85:t=fill' `
    -Title 'SYNDOCAL LED QA 1920x1080'

$projectorPath = New-TestClip `
    -FileName 'syndocal-near-show-projector-3840x2160.mp4' `
    -Size '3840x2160' `
    -VideoFilter 'hue=h=120,drawbox=x=3*iw/4:y=0:w=iw/4:h=ih/4:color=green@0.85:t=fill' `
    -Title 'SYNDOCAL PROJECTOR QA 3840x2160'

$clips = foreach ($entry in @(
    @{ Role = 'led'; Path = $ledPath; Width = 1920; Height = 1080 },
    @{ Role = 'projector'; Path = $projectorPath; Width = 3840; Height = 2160 }
)) {
    $probeJson = & $ffprobe.Source @(
        '-v', 'error',
        '-select_streams', 'v:0',
        '-show_entries', 'stream=codec_name,width,height,pix_fmt,r_frame_rate,duration',
        '-of', 'json',
        $entry.Path
    )
    if ($LASTEXITCODE -ne 0) {
        throw "ffprobe failed for $($entry.Path) with exit code $LASTEXITCODE"
    }
    $probe = ($probeJson -join [Environment]::NewLine) | ConvertFrom-Json
    $stream = $probe.streams | Select-Object -First 1
    if ($null -eq $stream -or [int]$stream.width -ne $entry.Width -or [int]$stream.height -ne $entry.Height) {
        throw "Generated $($entry.Role) clip does not match required $($entry.Width)x$($entry.Height)"
    }
    $file = Get-Item -LiteralPath $entry.Path
    [ordered]@{
        role = $entry.Role
        path = $file.FullName
        bytes = $file.Length
        sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
        codec = $stream.codec_name
        width = [int]$stream.width
        height = [int]$stream.height
        pixel_format = $stream.pix_fmt
        frame_rate = $stream.r_frame_rate
        duration_seconds = [double]$stream.duration
    }
}

$manifest = [ordered]@{
    schema = 'syndocal-near-show-test-media-v1'
    purpose = 'Temporary deterministic routing media; not final show artwork'
    editor_resolution = '1920x1080'
    generated_at = [DateTimeOffset]::Now.ToString('o')
    generator = $ffmpeg.Source
    clips = @($clips)
}
$manifestPath = Join-Path $outputDirectory 'manifest.json'
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $manifestPath -Encoding utf8

Write-Output $manifestPath
$clips | ForEach-Object {
    Write-Output ("{0}: {1} {2}x{3} {4} bytes SHA256={5}" -f $_.role, $_.path, $_.width, $_.height, $_.bytes, $_.sha256)
}
