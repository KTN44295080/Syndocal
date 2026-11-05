param(
    [Parameter(Mandatory = $true)][string]$OutDir,
    [Parameter(Mandatory = $true)][string]$RuntimeDll
)
# Builds the minimal NDI 6 SDK layout that grafton-ndi needs on Windows
# (Include, Lib\x64 import library, Bin\x64) without downloading the SDK,
# whose download requires registration. The headers are the vendored
# MIT-licensed NDI 6.3.2 headers. The import library is generated from the
# function declarations in those headers, never from the runtime binary, so
# the proprietary runtime DLL is not inspected. The DLL must be the pinned
# official runtime. Requires an initialized x64 MSVC environment (lib.exe).
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$vendor = Join-Path $repo 'vendor\ndi-sdk-6.3.2-headers'
$include = Join-Path $vendor 'Include'

# Vendored headers must match their recorded hashes exactly.
$expected = @{}
foreach ($line in Get-Content -LiteralPath (Join-Path $vendor 'SHA256SUMS')) {
    if ($line -notmatch '^([0-9a-f]{64})  (Include/[A-Za-z0-9.]+\.h)$') { throw "Malformed NDI header manifest line: $line" }
    $expected[$Matches[2]] = $Matches[1]
}
$headers = @(Get-ChildItem -LiteralPath $include -Filter '*.h' -File)
if ($headers.Count -ne $expected.Count -or $headers.Count -eq 0) {
    throw "Vendored NDI headers ($($headers.Count)) do not match the manifest ($($expected.Count))."
}
foreach ($header in $headers) {
    $key = "Include/$($header.Name)"
    $hash = (Get-FileHash -LiteralPath $header.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($expected[$key] -ne $hash) { throw "Vendored NDI header hash mismatch: $key" }
}

# The runtime DLL must be the pinned official NDI 6 runtime.
$pin = (Get-Content -LiteralPath (Join-Path $repo 'qa\FFMPEG_WINDOWS_RUNTIME_INVENTORY.json') -Raw | ConvertFrom-Json).ndi_runtime.dll
$dllHash = (Get-FileHash -LiteralPath $RuntimeDll -Algorithm SHA256).Hash
if (-not [StringComparer]::OrdinalIgnoreCase.Equals($dllHash, $pin.sha256)) {
    throw "NDI runtime $RuntimeDll ($dllHash) is not the pinned $($pin.version) ($($pin.sha256))."
}

# Exported names come from PROCESSINGNDILIB_API declarations in the headers.
$source = ($headers | ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw }) -join "`n"
$pattern = '(?m)^[ \t]*PROCESSINGNDILIB_API\b[^;{#]*?\b(NDIlib_[A-Za-z0-9_]+)\s*\('
$names = @([regex]::Matches($source, $pattern) | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique)
foreach ($required in 'NDIlib_initialize', 'NDIlib_destroy', 'NDIlib_find_create_v2', 'NDIlib_recv_create_v3', 'NDIlib_send_create', 'NDIlib_send_send_video_v2') {
    if ($names -notcontains $required) { throw "NDI header export scan did not find $required." }
}
if ($names.Count -lt 100) { throw "NDI header export scan found only $($names.Count) functions." }

New-Item -ItemType Directory -Force -Path (Join-Path $OutDir 'Include'), (Join-Path $OutDir 'Lib\x64'), (Join-Path $OutDir 'Bin\x64') | Out-Null
Copy-Item -LiteralPath $headers.FullName -Destination (Join-Path $OutDir 'Include') -Force
Copy-Item -LiteralPath $RuntimeDll -Destination (Join-Path $OutDir 'Bin\x64\Processing.NDI.Lib.x64.dll') -Force
$def = Join-Path $OutDir 'Lib\x64\Processing.NDI.Lib.x64.def'
@('LIBRARY Processing.NDI.Lib.x64.dll', 'EXPORTS') + ($names | ForEach-Object { "    $_" }) | Set-Content -LiteralPath $def -Encoding ascii
$libExe = Get-Command lib.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1
$importLib = Join-Path $OutDir 'Lib\x64\Processing.NDI.Lib.x64.lib'
& $libExe.Source /nologo /machine:x64 "/def:$def" "/out:$importLib"
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $importLib -PathType Leaf)) { throw "lib.exe could not create the NDI import library." }
Write-Host "Synthesized NDI SDK at $OutDir ($($headers.Count) headers, $($names.Count) imports, runtime $($pin.version))."
