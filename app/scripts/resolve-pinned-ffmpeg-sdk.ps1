# Shared native-harness adapter. Resolution only reads SDK files; it never
# launches FFmpeg, the application, a build, or a physical device.
function Resolve-FfmpegSdkRoot {
  $resolver = Join-Path $PSScriptRoot "resolve-pinned-ffmpeg-sdk.mjs"
  $output = @(& node $resolver)
  $resolverExitCode = $LASTEXITCODE
  $result = ($output -join "`n") | ConvertFrom-Json
  if ($resolverExitCode -ne 0) {
    throw $result.error
  }
  if ([string]::IsNullOrWhiteSpace($result.root)) {
    throw "The pinned LGPL FFmpeg SDK resolver returned no root."
  }
  return $result.root
}
