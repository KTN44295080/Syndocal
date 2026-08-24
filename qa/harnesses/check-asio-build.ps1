[CmdletBinding()]
param(
    [switch]$PreflightOnly,
    [switch]$SelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script:RequiredVcToolsInstallDir = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207"
$script:RequiredHostX64Linker = Join-Path $script:RequiredVcToolsInstallDir "bin\Hostx64\x64\link.exe"
$script:CargoLinkerVariableName = "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER"

function Get-CanonicalPathValue {
    param([string]$Value)

    if ([string]::IsNullOrWhiteSpace($Value)) {
        return $null
    }
    try {
        $fullPath = [IO.Path]::GetFullPath($Value)
    } catch {
        return $null
    }
    return $fullPath.TrimEnd("\")
}

function New-LinkerPinOutcome {
    param([bool]$Ok, [string]$Reason)

    return @{
        Ok = $Ok
        Reason = $Reason
        CanonicalVcTools = $script:RequiredVcToolsInstallDir
        CanonicalLinker = $script:RequiredHostX64Linker
    }
}

function Test-MsvcLinkerPinContract {
    param(
        [string]$VctoolsInstallDir,
        [string]$CargoLinkerPin,
        [bool]$VcToolsDirectoryPresent,
        [bool]$HostLinkerPresent,
        [int]$WhereLinkExitCode,
        [string[]]$LinkResolutionOrder
    )

    $canonicalVcTools = Get-CanonicalPathValue -Value $VctoolsInstallDir
    if ($null -eq $canonicalVcTools) {
        return New-LinkerPinOutcome $false "VCToolsInstallDir is not set. Enter the pinned developer shell first (vcvars64.bat -vcvars_ver=14.44); the harness never launches it or mutates the parent shell."
    }
    if (-not $canonicalVcTools.Equals($script:RequiredVcToolsInstallDir, [StringComparison]::OrdinalIgnoreCase)) {
        return New-LinkerPinOutcome $false "VCToolsInstallDir canonicalizes to '$canonicalVcTools', not the pinned toolset '$($script:RequiredVcToolsInstallDir)'."
    }
    if (-not $VcToolsDirectoryPresent) {
        return New-LinkerPinOutcome $false "The pinned MSVC toolset directory does not exist: $($script:RequiredVcToolsInstallDir)"
    }
    if (-not $HostLinkerPresent) {
        return New-LinkerPinOutcome $false "The pinned host x64 linker is missing: $($script:RequiredHostX64Linker)"
    }

    $canonicalLinkerPin = Get-CanonicalPathValue -Value $CargoLinkerPin
    if ($null -eq $canonicalLinkerPin) {
        return New-LinkerPinOutcome $false "$($script:CargoLinkerVariableName) is not set. Pin it to exactly '$($script:RequiredHostX64Linker)' before building; the harness never guesses a linker."
    }
    if (-not $canonicalLinkerPin.Equals($script:RequiredHostX64Linker, [StringComparison]::OrdinalIgnoreCase)) {
        return New-LinkerPinOutcome $false "$($script:CargoLinkerVariableName) is '$canonicalLinkerPin', not the pinned linker '$($script:RequiredHostX64Linker)'."
    }

    if ($WhereLinkExitCode -ne 0) {
        return New-LinkerPinOutcome $false "where.exe link.exe exited with code $WhereLinkExitCode; an unprovable linker resolution must not reach Cargo."
    }
    $resolvedEntries = @($LinkResolutionOrder | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($resolvedEntries.Count -eq 0) {
        return New-LinkerPinOutcome $false "where.exe link.exe produced no usable output; an empty linker resolution must not reach Cargo."
    }
    if (-not $resolvedEntries[0].Equals($script:RequiredHostX64Linker, [StringComparison]::OrdinalIgnoreCase)) {
        return New-LinkerPinOutcome $false "'$($resolvedEntries[0])' resolves first for link.exe; Cargo would link with it instead of the pinned '$($script:RequiredHostX64Linker)'."
    }

    return New-LinkerPinOutcome $true ""
}

function Get-WhereLinkResolution {
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $rawEntries = @(& where.exe link.exe 2>&1)
        $exitCode = $LASTEXITCODE
    } catch {
        throw "where.exe link.exe could not be executed: $($_.Exception.Message)"
    } finally {
        $ErrorActionPreference = $previousPreference
    }
    $entries = @($rawEntries | ForEach-Object { "$_".Trim() })
    return @{ ExitCode = $exitCode; Entries = $entries }
}

function Invoke-SelfTest {
    $parseErrors = $null
    $null = [System.Management.Automation.Language.Parser]::ParseFile($PSCommandPath, [ref]$null, [ref]$parseErrors)
    if ($null -ne $parseErrors -and $parseErrors.Count -gt 0) {
        throw "check-asio-build.ps1 failed its own parse gate with $($parseErrors.Count) error(s); first: $($parseErrors[0].Message)"
    }
    $assertions = 1

    $source = Get-Content -LiteralPath $PSCommandPath -Raw
    if (-not $source.Contains($script:RequiredVcToolsInstallDir) -or -not $source.Contains("bin\Hostx64\x64\link.exe")) {
        throw "check-asio-build.ps1 no longer states the pinned MSVC toolset and host linker literally."
    }
    $assertions++

    $preflightGuardNeedle = 'if (' + '$PreflightOnly)'
    $preflightGuardIndex = $source.IndexOf($preflightGuardNeedle)
    $cargoInvocationNeedle = '&' + ' cargo'
    $cargoInvocationIndex = $source.IndexOf($cargoInvocationNeedle)
    if ($preflightGuardIndex -lt 0 -or $cargoInvocationIndex -lt 0 -or $cargoInvocationIndex -lt $preflightGuardIndex) {
        throw "The -PreflightOnly return must come before this script can ever invoke Cargo."
    }
    $assertions++

    $liveWiringNeedle = '-VctoolsInstallDir ' + '$vcToolsInstallDirRaw'
    $liveWiringIndex = $source.IndexOf($liveWiringNeedle)
    if ($liveWiringIndex -lt 0 -or $liveWiringIndex -gt $preflightGuardIndex) {
        throw "The live linker pin contract must run before the -PreflightOnly return."
    }
    $assertions++

    $baseSplat = @{
        VctoolsInstallDir = $script:RequiredVcToolsInstallDir
        CargoLinkerPin = $script:RequiredHostX64Linker
        VcToolsDirectoryPresent = $true
        HostLinkerPresent = $true
        WhereLinkExitCode = 0
        LinkResolutionOrder = @($script:RequiredHostX64Linker, "C:\Program Files\Git\usr\bin\link.exe")
    }

    $negativeCases = @(
        @{ Name = "missing VCToolsInstallDir"; Overrides = @{ VctoolsInstallDir = "" } },
        @{ Name = "blank VCToolsInstallDir"; Overrides = @{ VctoolsInstallDir = "   " } },
        @{ Name = "relative VCToolsInstallDir"; Overrides = @{ VctoolsInstallDir = "VC\Tools\MSVC\14.44.35207" } },
        @{ Name = "stale MSVC toolset"; Overrides = @{ VctoolsInstallDir = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.29.30133" } },
        @{ Name = "absent toolset directory"; Overrides = @{ VcToolsDirectoryPresent = $false } },
        @{ Name = "missing pinned link.exe"; Overrides = @{ HostLinkerPresent = $false } },
        @{ Name = "bare linker pin"; Overrides = @{ CargoLinkerPin = "" } },
        @{ Name = "blank linker pin"; Overrides = @{ CargoLinkerPin = "   " } },
        @{ Name = "stale linker pin"; Overrides = @{ CargoLinkerPin = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.29.30133\bin\Hostx64\x64\link.exe" } },
        @{ Name = "unrelated linker first"; Overrides = @{ LinkResolutionOrder = @("C:\cygwin64\bin\link.exe", $script:RequiredHostX64Linker) } },
        @{ Name = "Git link.exe ahead of MSVC"; Overrides = @{ LinkResolutionOrder = @("C:\Program Files\Git\usr\bin\link.exe", $script:RequiredHostX64Linker) } },
        @{ Name = "where.exe failure"; Overrides = @{ WhereLinkExitCode = 9009; LinkResolutionOrder = @() } },
        @{ Name = "empty where.exe output"; Overrides = @{ LinkResolutionOrder = @("", "   ") } }
    )

    foreach ($negativeCase in $negativeCases) {
        $caseSplat = $baseSplat.Clone()
        foreach ($overrideKey in $negativeCase.Overrides.Keys) {
            $caseSplat[$overrideKey] = $negativeCase.Overrides[$overrideKey]
        }
        $outcome = Test-MsvcLinkerPinContract @caseSplat
        if ($outcome.Ok) {
            throw "Self-test case '$($negativeCase.Name)' unexpectedly satisfied the linker pin contract."
        }
        if ([string]::IsNullOrWhiteSpace($outcome.Reason)) {
            throw "Self-test case '$($negativeCase.Name)' was rejected without a reason."
        }
        $assertions += 2
    }

    $positiveCases = @(
        @{ Name = "MSVC first with Git second"; Overrides = @{} },
        @{ Name = "MSVC linker only"; Overrides = @{ LinkResolutionOrder = @($script:RequiredHostX64Linker) } },
        @{ Name = "case and trailing separator normalization"; Overrides = @{
            VctoolsInstallDir = "c:\program files\microsoft visual studio\2022\community\vc\tools\msvc\14.44.35207\"
            CargoLinkerPin = "C:\PROGRAM FILES\MICROSOFT VISUAL STUDIO\2022\COMMUNITY\VC\TOOLS\MSVC\14.44.35207\BIN\HOSTX64\X64\LINK.EXE"
            LinkResolutionOrder = @($script:RequiredHostX64Linker.ToUpperInvariant(), "C:\Program Files\Git\usr\bin\link.exe")
        } }
    )

    foreach ($positiveCase in $positiveCases) {
        $caseSplat = $baseSplat.Clone()
        foreach ($overrideKey in $positiveCase.Overrides.Keys) {
            $caseSplat[$overrideKey] = $positiveCase.Overrides[$overrideKey]
        }
        $outcome = Test-MsvcLinkerPinContract @caseSplat
        if (-not $outcome.Ok) {
            throw "Self-test case '$($positiveCase.Name)' wrongly rejected a compliant toolchain: $($outcome.Reason)"
        }
        if (-not $outcome.CanonicalVcTools.Equals($script:RequiredVcToolsInstallDir, [StringComparison]::Ordinal) -or
            -not $outcome.CanonicalLinker.Equals($script:RequiredHostX64Linker, [StringComparison]::Ordinal)) {
            throw "Self-test case '$($positiveCase.Name)' did not repin the exact canonical values."
        }
        $assertions += 2
    }

    Write-Host "check-asio-build.ps1 self-test: $assertions assertions passed (parse gate, source contract, linker pin matrix; Cargo was never invoked)."
}

if ($SelfTest) {
    Invoke-SelfTest
    return
}

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

$vcToolsInstallDirRaw = [Environment]::GetEnvironmentVariable("VCToolsInstallDir", "Process")
$cargoLinkerPinRaw = [Environment]::GetEnvironmentVariable($script:CargoLinkerVariableName, "Process")
$vcToolsDirectoryPresent = Test-Path -LiteralPath $script:RequiredVcToolsInstallDir -PathType Container
$hostLinkerPresent = Test-Path -LiteralPath $script:RequiredHostX64Linker -PathType Leaf
$whereLinkResolution = Get-WhereLinkResolution
Write-Host "where.exe link.exe resolution order (the pinned MSVC linker must be first):"
if ($whereLinkResolution.Entries.Count -eq 0) {
    Write-Host "  (no output)"
} else {
    for ($entryIndex = 0; $entryIndex -lt $whereLinkResolution.Entries.Count; $entryIndex++) {
        Write-Host "  $($entryIndex + 1): $($whereLinkResolution.Entries[$entryIndex])"
    }
}
$linkerPinContract = Test-MsvcLinkerPinContract -VctoolsInstallDir $vcToolsInstallDirRaw -CargoLinkerPin $cargoLinkerPinRaw -VcToolsDirectoryPresent $vcToolsDirectoryPresent -HostLinkerPresent $hostLinkerPresent -WhereLinkExitCode $whereLinkResolution.ExitCode -LinkResolutionOrder $whereLinkResolution.Entries
if (-not $linkerPinContract.Ok) {
    throw $linkerPinContract.Reason
}
[Environment]::SetEnvironmentVariable("VCToolsInstallDir", $linkerPinContract.CanonicalVcTools, "Process")
[Environment]::SetEnvironmentVariable($script:CargoLinkerVariableName, $linkerPinContract.CanonicalLinker, "Process")
Write-Host "Pinned MSVC toolset: $($linkerPinContract.CanonicalVcTools)"
Write-Host "Pinned host x64 linker: $($linkerPinContract.CanonicalLinker)"

Write-Host "ASIO build preflight passed. SDK, libclang, MSVC toolset and linker pin will be used only from the explicit pinned paths."

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
