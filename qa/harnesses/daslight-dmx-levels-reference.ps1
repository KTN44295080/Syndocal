# Converts a Daslight 5 "DMX LEVELS" screenshot into the same final-frame
# evidence shape as artnet-monitor.mjs. Exact bytes are transcribed from the
# visible values; the blue level bars independently verify the active channel
# set and reject obvious transcription mistakes.
param(
  [string]$InputPng,
  [string]$OutPath,
  [string]$Assignments = "",
  [ValidateRange(0, 32767)][int]$Universe = 0,
  [ValidateRange(0, 255)][int]$BarTolerance = 25,
  [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function ConvertFrom-DmxAssignments {
  param([string]$Text)

  $values = [int[]]::new(512)
  $seen = [Collections.Generic.HashSet[int]]::new()
  if ([string]::IsNullOrWhiteSpace($Text)) { return $values }

  foreach ($entry in $Text -split ',') {
    if ($entry.Trim() -notmatch '^(\d+)\s*=\s*(\d+)$') {
      throw "Invalid assignment '$entry'. Use comma-separated channel=value pairs."
    }
    $channel = [int]$Matches[1]
    $value = [int]$Matches[2]
    if ($channel -lt 1 -or $channel -gt 512) {
      throw "DMX channel $channel is outside 1..512."
    }
    if ($value -lt 0 -or $value -gt 255) {
      throw "DMX value $value for channel $channel is outside 0..255."
    }
    if (-not $seen.Add($channel)) {
      throw "DMX channel $channel was assigned more than once."
    }
    $values[$channel - 1] = $value
  }
  return $values
}

function Invoke-SelfTest {
  $values = ConvertFrom-DmxAssignments '1=255, 82=130, 512=7'
  if ($values.Length -ne 512 -or $values[0] -ne 255 -or $values[81] -ne 130 -or $values[511] -ne 7) {
    throw 'Assignment parsing self-test failed.'
  }
  if (($values | Where-Object { $_ -ne 0 }).Count -ne 3) {
    throw 'Implicit-zero self-test failed.'
  }
  foreach ($bad in @('0=1', '513=1', '1=256', '2=1,2=2', 'not-an-assignment')) {
    $rejected = $false
    try { $null = ConvertFrom-DmxAssignments $bad } catch { $rejected = $true }
    if (-not $rejected) { throw "Malformed assignment '$bad' was accepted." }
  }
  Write-Host 'Daslight DMX Levels reference self-test: 7 assertions passed'
}

if ($SelfTest) {
  Invoke-SelfTest
  return
}
if ([string]::IsNullOrWhiteSpace($InputPng) -or [string]::IsNullOrWhiteSpace($OutPath)) {
  throw 'Provide -InputPng and -OutPath, or use -SelfTest.'
}

Add-Type -AssemblyName System.Drawing
$resolvedInput = (Resolve-Path -LiteralPath $InputPng).Path
$values = ConvertFrom-DmxAssignments $Assignments
$bitmap = [Drawing.Bitmap]::new($resolvedInput)

try {
  # Daslight 5 currently renders this child surface at 730 x 601 physical
  # pixels. Reject a crop/layout change instead of reading the wrong pixels.
  if ($bitmap.Width -ne 730 -or $bitmap.Height -ne 601) {
    throw "Expected a 730x601 DMX LEVELS child screenshot, got $($bitmap.Width)x$($bitmap.Height)."
  }

  $originX = 12
  $originY = 78
  $cellWidth = 22
  $cellHeight = 32
  $valueRegionHeight = 18
  $activeThreshold = 12
  $detectedActive = [Collections.Generic.List[int]]::new()
  $barEstimates = [Collections.Generic.List[object]]::new()

  for ($channel = 1; $channel -le 512; $channel += 1) {
    $column = ($channel - 1) % 32
    $row = [Math]::Floor(($channel - 1) / 32)
    $x0 = $originX + ($column * $cellWidth)
    $y0 = $originY + ($row * $cellHeight)
    $bluePixels = 0

    for ($y = 0; $y -lt $valueRegionHeight; $y += 1) {
      for ($x = 1; $x -lt 21; $x += 1) {
        $pixel = $bitmap.GetPixel($x0 + $x, $y0 + $y)
        if ($pixel.B - $pixel.R -ge 25 -and $pixel.B -ge 90) { $bluePixels += 1 }
      }
    }

    if ($bluePixels -ge $activeThreshold) {
      $detectedActive.Add($channel)
      # Anti-aliased white digits replace some blue pixels. This estimate is
      # deliberately tolerant and is only an independent sanity check; the
      # visible numeric text remains the exact byte source.
      $estimated = [Math]::Round(($bluePixels / (20.0 * $valueRegionHeight)) * 255)
      $barEstimates.Add([ordered]@{
        channel = $channel
        expected = $values[$channel - 1]
        estimated = [int]$estimated
        delta = [int]$estimated - $values[$channel - 1]
        bluePixels = $bluePixels
      })
    }
  }

  $expectedActive = @(
    for ($index = 0; $index -lt $values.Length; $index += 1) {
      if ($values[$index] -gt 0) { $index + 1 }
    }
  )
  $missingBars = @($expectedActive | Where-Object { $_ -notin $detectedActive })
  $unexpectedBars = @($detectedActive | Where-Object { $_ -notin $expectedActive })
  $outOfTolerance = @(
    $barEstimates | Where-Object {
      [Math]::Abs($_.delta) -gt $BarTolerance
    }
  )
  if ($missingBars.Count -gt 0 -or $unexpectedBars.Count -gt 0 -or $outOfTolerance.Count -gt 0) {
    $details = [ordered]@{
      missingBars = $missingBars
      unexpectedBars = $unexpectedBars
      outOfTolerance = $outOfTolerance
    } | ConvertTo-Json -Depth 6 -Compress
    throw "Screenshot validation failed: $details"
  }

  $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedInput).Hash
  $capturedAt = (Get-Item -LiteralPath $resolvedInput).LastWriteTimeUtc.ToString('o')
  $nonZero = $expectedActive.Count
  $maxValue = ($values | Measure-Object -Maximum).Maximum
  $firstNonZero = if ($nonZero -gt 0) {
    [ordered]@{ channel = $expectedActive[0]; value = $values[$expectedActive[0] - 1] }
  } else { $null }
  $frame = [ordered]@{
    at = $capturedAt
    source = 'Daslight 5 DMX LEVELS / visible Universe 1'
    universe = $Universe
    sequence = $null
    digest = $null
    data = $values
    nonZeroChannels = $nonZero
    maxValue = $maxValue
    firstNonZero = $firstNonZero
  }
  $evidence = [ordered]@{
    schema = 1
    product = 'Daslight DMX Levels screen reference'
    capturedAt = $capturedAt
    universe = $Universe
    sourceImage = [ordered]@{
      path = $resolvedInput
      sha256 = $hash
      width = $bitmap.Width
      height = $bitmap.Height
    }
    extraction = [ordered]@{
      method = 'screen-assisted exact transcription with independent blue-bar validation'
      unspecifiedChannels = 'zero'
      barTolerance = $BarTolerance
      detectedActiveChannels = @($detectedActive)
      barEstimates = @($barEstimates)
    }
    transitions = @()
    lastFrames = [ordered]@{ "$Universe" = $frame }
    lastFrame = $frame
  }
} finally {
  $bitmap.Dispose()
}

$resolvedOutput = [IO.Path]::GetFullPath($OutPath)
$outputDirectory = Split-Path -Parent $resolvedOutput
if ($outputDirectory) { New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null }
$evidence | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $resolvedOutput -Encoding utf8
Write-Host "Daslight DMX reference: $($evidence.lastFrame.nonZeroChannels) non-zero channels -> $resolvedOutput"
