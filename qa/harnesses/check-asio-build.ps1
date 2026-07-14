[CmdletBinding()]
param(
    [switch]$PreflightOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-RequiredDirectoryFromEnvironment {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    $value = [Environment]::GetEnvironmentVariable($Name, "Process")
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "$Name must be set explicitly. This supported ASIO build entrypoint never downloads the Steinberg SDK."
    }

    $resolved = [IO.Path]::GetFullPath($value)
    if (-not (Test-Path -LiteralPath $resolved -PathType Container)) {
        throw "$Name must name an existing directory: $resolved"
    }

    return $resolved
}

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw "The Syndocal ASIO build check is supported only on Windows."
}

$asioSdkDirectory = Get-RequiredDirectoryFromEnvironment -Name "CPAL_ASIO_DIR"
$libclangDirectory = Get-RequiredDirectoryFromEnvironment -Name "LIBCLANG_PATH"

foreach ($relativePath in @("common\asio.h", "common\asiosys.h", "host\asiodrivers.h")) {
    $requiredPath = Join-Path $asioSdkDirectory $relativePath
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
        throw "CPAL_ASIO_DIR does not contain the required ASIO SDK file: $requiredPath"
    }
}

$libclangDll = Join-Path $libclangDirectory "libclang.dll"
if (-not (Test-Path -LiteralPath $libclangDll -PathType Leaf)) {
    throw "LIBCLANG_PATH does not contain the required libclang.dll: $libclangDll"
}

Write-Host "ASIO build preflight passed. SDK and libclang will be used only from the explicit local paths."

if ($PreflightOnly) {
    return
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$targetDirectory = Join-Path $repoRoot "target\asio-qa"
$backendManifest = Join-Path $repoRoot "tools\asio-bridge\Cargo.toml"
if (-not (Test-Path -LiteralPath $backendManifest -PathType Leaf)) {
    throw "The isolated ASIO bridge manifest is missing: $backendManifest"
}
Get-Command cargo -ErrorAction Stop | Out-Null

$originalAsioSdkDirectory = $env:CPAL_ASIO_DIR
$originalLibclangDirectory = $env:LIBCLANG_PATH
try {
    $env:CPAL_ASIO_DIR = $asioSdkDirectory
    $env:LIBCLANG_PATH = $libclangDirectory
    Push-Location $repoRoot
    try {
        & cargo check --manifest-path $backendManifest --no-default-features --features asio --locked --target-dir $targetDirectory
        if ($LASTEXITCODE -ne 0) {
            throw "ASIO cargo check failed with exit code $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }
}
finally {
    $env:CPAL_ASIO_DIR = $originalAsioSdkDirectory
    $env:LIBCLANG_PATH = $originalLibclangDirectory
}
