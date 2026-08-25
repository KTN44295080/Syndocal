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
$script:SdkArchiveVariableName = "SYNDOCAL_ASIO_SDK_ARCHIVE_PATH"
$script:CanonicalDllName = "syndocal_asio_bridge.dll"
$script:ExpectedExports = @(
    "syndocal_asio_v2_abi_version",
    "syndocal_asio_v2_build_flags",
    "syndocal_asio_v2_capabilities_json",
    "syndocal_asio_v2_close",
    "syndocal_asio_v2_drivers_json",
    "syndocal_asio_v2_start",
    "syndocal_asio_v2_stop",
    "syndocal_asio_v2_string_free",
    "syndocal_asio_v2_telemetry_json"
)
$script:ExpectedHeaderPrototypes = @(
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_abi_version(void);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_build_flags(void);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_drivers_json(SyndocalAsioStringV2 *out_json, SyndocalAsioStringV2 *out_error_json);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_capabilities_json(const uint8_t *request_json, size_t request_json_len, SyndocalAsioStringV2 *out_json, SyndocalAsioStringV2 *out_error_json);",
    "void SYNDOCAL_ASIO_CALL syndocal_asio_v2_string_free(SyndocalAsioStringV2 string);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_start(const uint8_t *request_json, size_t request_json_len, SyndocalAsioSampleCallbackV2 sample_callback, SyndocalAsioEventCallbackV2 event_callback, void *context, SyndocalAsioHandle **out_handle, SyndocalAsioStringV2 *out_result_json, SyndocalAsioStringV2 *out_error_json);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_stop(SyndocalAsioHandle *handle, SyndocalAsioStringV2 *out_result_json, SyndocalAsioStringV2 *out_error_json);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_close(SyndocalAsioHandle **handle, SyndocalAsioStringV2 *out_result_json, SyndocalAsioStringV2 *out_error_json);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_telemetry_json(const SyndocalAsioHandle *handle, SyndocalAsioStringV2 *out_json, SyndocalAsioStringV2 *out_error_json);"
)

function Get-CanonicalPathValue {
    param([string]$Value)

    if ([string]::IsNullOrWhiteSpace($Value)) {
        return $null
    }
    try {
        return [IO.Path]::GetFullPath($Value).TrimEnd("\")
    } catch {
        return $null
    }
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
        return New-LinkerPinOutcome $false "VCToolsInstallDir is not set. Enter vcvars64.bat -vcvars_ver=14.44 before this harness."
    }
    if (-not $canonicalVcTools.Equals($script:RequiredVcToolsInstallDir, [StringComparison]::OrdinalIgnoreCase)) {
        return New-LinkerPinOutcome $false "VCToolsInstallDir canonicalizes to '$canonicalVcTools', not '$($script:RequiredVcToolsInstallDir)'."
    }
    if (-not $VcToolsDirectoryPresent) {
        return New-LinkerPinOutcome $false "The pinned MSVC toolset directory is missing: $($script:RequiredVcToolsInstallDir)"
    }
    if (-not $HostLinkerPresent) {
        return New-LinkerPinOutcome $false "The pinned host x64 linker is missing: $($script:RequiredHostX64Linker)"
    }

    $canonicalLinkerPin = Get-CanonicalPathValue -Value $CargoLinkerPin
    if ($null -eq $canonicalLinkerPin) {
        return New-LinkerPinOutcome $false "$($script:CargoLinkerVariableName) is not set."
    }
    if (-not $canonicalLinkerPin.Equals($script:RequiredHostX64Linker, [StringComparison]::OrdinalIgnoreCase)) {
        return New-LinkerPinOutcome $false "$($script:CargoLinkerVariableName) is '$canonicalLinkerPin', not '$($script:RequiredHostX64Linker)'."
    }
    if ($WhereLinkExitCode -ne 0) {
        return New-LinkerPinOutcome $false "where.exe link.exe exited with $WhereLinkExitCode."
    }
    $resolvedEntries = @($LinkResolutionOrder | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($resolvedEntries.Count -eq 0) {
        return New-LinkerPinOutcome $false "where.exe link.exe produced no usable output."
    }
    if (-not $resolvedEntries[0].Equals($script:RequiredHostX64Linker, [StringComparison]::OrdinalIgnoreCase)) {
        return New-LinkerPinOutcome $false "'$($resolvedEntries[0])' resolves first instead of '$($script:RequiredHostX64Linker)'."
    }
    return New-LinkerPinOutcome $true ""
}

function Get-WhereLinkResolution {
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $rawEntries = @(& where.exe link.exe 2>&1)
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousPreference
    }
    return @{
        ExitCode = $exitCode
        Entries = @($rawEntries | ForEach-Object { "$_".Trim() })
    }
}

function Get-RequiredDirectoryFromEnvironment {
    param([Parameter(Mandatory = $true)][string]$Name)

    $value = [Environment]::GetEnvironmentVariable($Name, "Process")
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "$Name must be set explicitly; this harness never downloads an SDK or guesses a directory."
    }
    $resolved = [IO.Path]::GetFullPath($value)
    if (-not (Test-Path -LiteralPath $resolved -PathType Container)) {
        throw "$Name must name an existing directory: $resolved"
    }
    return $resolved.TrimEnd("\")
}

function Get-RequiredFileFromEnvironment {
    param([Parameter(Mandatory = $true)][string]$Name)

    $value = [Environment]::GetEnvironmentVariable($Name, "Process")
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "$Name must name the manually acquired pinned SDK ZIP."
    }
    $resolved = [IO.Path]::GetFullPath($value)
    if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
        throw "$Name must name an existing file: $resolved"
    }
    return $resolved
}

function Read-StrictSdkPin {
    param([Parameter(Mandatory = $true)][string]$Path)

    $pin = Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    $expectedProperties = @(
        "schema_version",
        "bridge_abi_version",
        "canonical_dll_filename",
        "sdk_name",
        "version",
        "archive_filename",
        "sha256",
        "archive_root",
        "source_page",
        "acquisition",
        "distribution_approved",
        "distribution_gate"
    ) | Sort-Object
    $actualProperties = @($pin.PSObject.Properties.Name | Sort-Object)
    if (Compare-Object $expectedProperties $actualProperties) {
        throw "ASIO_SDK_PIN.json properties differ from the exact schema v2 contract."
    }
    if ($pin.schema_version -ne 2 -or $pin.bridge_abi_version -ne 2) {
        throw "ASIO_SDK_PIN.json must pin schema_version=2 and bridge_abi_version=2."
    }
    if ($pin.canonical_dll_filename -cne $script:CanonicalDllName) {
        throw "ASIO_SDK_PIN.json canonical DLL is '$($pin.canonical_dll_filename)', expected '$script:CanonicalDllName'."
    }
    if ($pin.distribution_approved -ne $false) {
        throw "ASIO distribution approval must remain false until the recorded licensing gate is actually resolved."
    }
    if ($pin.acquisition -cne "manual") {
        throw "ASIO SDK acquisition must remain explicit and manual."
    }
    if ($pin.sha256 -notmatch '^[0-9A-F]{64}$') {
        throw "ASIO SDK SHA-256 must be 64 uppercase hexadecimal characters."
    }
    if ([string]::IsNullOrWhiteSpace($pin.archive_root) -or $pin.archive_root.Contains("/") -or $pin.archive_root.Contains("\")) {
        throw "ASIO SDK archive_root must be one exact top-level directory name."
    }
    return $pin
}

function Get-StreamSha256 {
    param([Parameter(Mandatory = $true)][IO.Stream]$Stream)

    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        return [Convert]::ToHexString($sha.ComputeHash($Stream))
    } finally {
        $sha.Dispose()
    }
}

function Assert-SdkArchiveExtractionProvenance {
    param(
        [Parameter(Mandatory = $true)]$Pin,
        [Parameter(Mandatory = $true)][string]$ArchivePath,
        [Parameter(Mandatory = $true)][string]$SdkDirectory
    )

    $archive = Get-Item -LiteralPath $ArchivePath
    if ($archive.Name -cne $Pin.archive_filename) {
        throw "SDK ZIP filename '$($archive.Name)' does not equal pin '$($Pin.archive_filename)'."
    }
    $archiveHash = (Get-FileHash -LiteralPath $archive.FullName -Algorithm SHA256).Hash
    if (-not $archiveHash.Equals($Pin.sha256, [StringComparison]::OrdinalIgnoreCase)) {
        throw "SDK ZIP SHA-256 '$archiveHash' does not equal pin '$($Pin.sha256)'."
    }
    if ((Split-Path -Leaf $SdkDirectory) -cne $Pin.archive_root) {
        throw "CPAL_ASIO_DIR leaf '$(Split-Path -Leaf $SdkDirectory)' does not equal archive_root '$($Pin.archive_root)'."
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($archive.FullName)
    try {
        $rootPrefix = "$($Pin.archive_root)/"
        $entries = @{}
        foreach ($entry in $zip.Entries) {
            $name = $entry.FullName
            if ([string]::IsNullOrWhiteSpace($name) -or $name.EndsWith("/")) {
                continue
            }
            if (-not $name.StartsWith($rootPrefix, [StringComparison]::Ordinal)) {
                throw "SDK ZIP file is outside the pinned root '$rootPrefix': $name"
            }
            $relative = $name.Substring($rootPrefix.Length)
            $segments = $relative.Split('/')
            $unsafeSegments = @($segments | Where-Object {
                ($_ -in @("", ".", "..")) -or ($_.Contains(":")) -or ($_.Contains("\"))
            })
            if ($segments.Count -eq 0 -or $unsafeSegments.Count -ne 0) {
                throw "SDK ZIP contains an unsafe path: $name"
            }
            $key = $relative.ToLowerInvariant()
            if ($entries.ContainsKey($key)) {
                throw "SDK ZIP contains a duplicate case-insensitive path: $relative"
            }
            $entries[$key] = $entry
        }
        if ($entries.Count -eq 0) {
            throw "SDK ZIP contains no files below '$rootPrefix'."
        }

        $extractedFiles = @(Get-ChildItem -LiteralPath $SdkDirectory -File -Recurse)
        if ($extractedFiles.Count -ne $entries.Count) {
            throw "Extracted SDK file count $($extractedFiles.Count) does not equal ZIP file count $($entries.Count)."
        }
        foreach ($file in $extractedFiles) {
            $relative = [IO.Path]::GetRelativePath($SdkDirectory, $file.FullName).Replace("\", "/")
            $key = $relative.ToLowerInvariant()
            if (-not $entries.ContainsKey($key)) {
                throw "Extracted SDK contains a file absent from the pinned ZIP: $relative"
            }
            $entryStream = $entries[$key].Open()
            try {
                $entryHash = Get-StreamSha256 -Stream $entryStream
            } finally {
                $entryStream.Dispose()
            }
            $fileHash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
            if (-not $entryHash.Equals($fileHash, [StringComparison]::OrdinalIgnoreCase)) {
                throw "Extracted SDK file differs from the pinned ZIP: $relative"
            }
        }
    } finally {
        $zip.Dispose()
    }

    foreach ($relativePath in @("common\asio.h", "common\asiosys.h", "host\asiodrivers.h")) {
        $requiredPath = Join-Path $SdkDirectory $relativePath
        if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
            throw "Pinned SDK extraction lacks required input: $requiredPath"
        }
    }
    Write-Host "ASIO SDK provenance passed: archive=$($archive.FullName) sha256=$archiveHash extracted_files=$($entries.Count) root=$SdkDirectory"
}

function Invoke-CargoChecked {
    param(
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$Label,
        [switch]$InventoryWarnings
    )

    Write-Host "Cargo gate: $Label"
    Write-Host "  cargo $($Arguments -join ' ')"
    if ($InventoryWarnings) {
        $lines = @(& cargo @Arguments --message-format=json 2>&1)
        $exitCode = $LASTEXITCODE
        $firstPartyWarnings = 0
        $linkWarnings = @()
        foreach ($line in $lines) {
            $text = "$line"
            if ($text -match '(?i)warning\s+LNK\d+') {
                $linkWarnings += $text
            }
            try {
                $message = $text | ConvertFrom-Json -ErrorAction Stop
                if ($message.reason -eq "compiler-message" -and
                    $message.message.level -eq "warning" -and
                    "$($message.package_id)" -match 'syndocal-asio-bridge') {
                    $firstPartyWarnings++
                    Write-Host $message.message.rendered
                }
            } catch {
                if (-not [string]::IsNullOrWhiteSpace($text)) {
                    Write-Host $text
                }
            }
        }
        if ($exitCode -ne 0) {
            throw "$Label failed with exit code $exitCode."
        }
        if ($firstPartyWarnings -ne 0) {
            throw "$Label emitted $firstPartyWarnings first-party warning(s)."
        }
        if ($linkWarnings.Count -ne 0) {
            throw "$Label emitted linker warning(s): $($linkWarnings -join ' | ')"
        }
        Write-Host "$Label warning inventory: first_party=0 linker=0"
        return
    }

    & cargo @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE."
    }
}

function Assert-ExactBridgeExports {
    param([Parameter(Mandatory = $true)][string]$DllPath)

    $dumpbin = Get-Command dumpbin.exe -ErrorAction Stop
    $output = @(& $dumpbin.Source /nologo /exports $DllPath 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw "dumpbin export inspection failed with exit code $LASTEXITCODE."
    }
    $actual = @($output | ForEach-Object {
        if ("$_" -match '^\s+\d+\s+[0-9A-F]+\s+[0-9A-F]+\s+(\S+?)(?:\s+=.*)?\s*$') {
            $Matches[1]
        }
    } | Sort-Object -Unique)
    $expected = @($script:ExpectedExports | Sort-Object)
    $difference = @(Compare-Object $expected $actual)
    if ($difference.Count -ne 0) {
        throw "Bridge exports differ from the exact ABI v2 set: $($difference | Out-String)"
    }
    Write-Host "ASIO bridge export gate passed: $($actual.Count) exact v2 symbols; v1/play/free symbols absent."
}

function ConvertTo-NormalizedCPrototype {
    param([Parameter(Mandatory = $true)][string]$Prototype)

    $normalized = [regex]::Replace($Prototype.Trim(), '\s+', ' ')
    $normalized = [regex]::Replace($normalized, '\(\s+', '(')
    $normalized = [regex]::Replace($normalized, '\s+\)', ')')
    return [regex]::Replace($normalized, '\s*,\s*', ', ')
}

function Get-HeaderContractPrototypesFromSource {
    param([Parameter(Mandatory = $true)][string]$Source)

    $declarationSource = [regex]::Replace($Source, '(?s)/\*.*?\*/', '')
    $declarationSource = [regex]::Replace($declarationSource, '(?m)//.*$', '')
    return @([regex]::Matches(
        $declarationSource,
        '(?ms)^\s*(?:uint32_t|void)\s+SYNDOCAL_ASIO_CALL\s+syndocal_asio_v2_[a-z_]+\s*\(.*?\);\s*$'
    ) | ForEach-Object { ConvertTo-NormalizedCPrototype -Prototype $_.Value })
}

function Assert-HeaderContract {
    param([Parameter(Mandatory = $true)][string]$HeaderPath)

    $source = Get-Content -LiteralPath $HeaderPath -Raw
    if (-not $source.Contains("#define SYNDOCAL_ASIO_ABI_VERSION 2u") -or
        -not $source.Contains("#define SYNDOCAL_ASIO_JSON_SCHEMA_VERSION 2u") -or
        -not $source.Contains("#define SYNDOCAL_ASIO_CANONICAL_DLL `"$script:CanonicalDllName`"")) {
        throw "ASIO bridge header does not state the exact ABI/schema/DLL v2 identity."
    }
    $expectedPrototypes = @($script:ExpectedHeaderPrototypes |
        ForEach-Object { ConvertTo-NormalizedCPrototype -Prototype $_ })
    $prototypes = @(Get-HeaderContractPrototypesFromSource -Source $source)
    $difference = @(Compare-Object $expectedPrototypes $prototypes)
    if ($difference.Count -ne 0) {
        throw "ASIO bridge header prototypes differ from the exact ABI v2 signatures: $($difference | Out-String)"
    }
    if ($prototypes.Count -ne $script:ExpectedHeaderPrototypes.Count) {
        throw "ASIO bridge header has duplicate or missing ABI v2 prototypes (expected $($script:ExpectedHeaderPrototypes.Count), found $($prototypes.Count))."
    }
    if ($source -match '\bsyndocal_asio_(?!v2_)[a-z_]+\s*\(' -or $source -match '\bSyndocalAsio\w+V1\b') {
        throw "ASIO bridge header still exposes an ABI v1 declaration."
    }
    Write-Host "ASIO bridge header gate passed: exact ABI/schema/DLL identity and 9 v2 declarations."
}

function Invoke-SelfTest {
    $parseErrors = $null
    $null = [Management.Automation.Language.Parser]::ParseFile($PSCommandPath, [ref]$null, [ref]$parseErrors)
    if ($parseErrors.Count -ne 0) {
        throw "check-asio-build.ps1 parse gate failed: $($parseErrors[0].Message)"
    }
    $source = Get-Content -LiteralPath $PSCommandPath -Raw
    foreach ($needle in @(
        $script:RequiredVcToolsInstallDir,
        "SYNDOCAL_ASIO_SDK_ARCHIVE_PATH",
        "Assert-SdkArchiveExtractionProvenance",
        "syndocal_asio_v2_close",
        "dumpbin.exe"
    )) {
        if (-not $source.Contains($needle)) {
            throw "Self-test source contract is missing: $needle"
        }
    }

    $base = @{
        VctoolsInstallDir = $script:RequiredVcToolsInstallDir
        CargoLinkerPin = $script:RequiredHostX64Linker
        VcToolsDirectoryPresent = $true
        HostLinkerPresent = $true
        WhereLinkExitCode = 0
        LinkResolutionOrder = @($script:RequiredHostX64Linker, "C:\Program Files\Git\usr\bin\link.exe")
    }
    $negativeCases = @(
        @{ VctoolsInstallDir = "" },
        @{ CargoLinkerPin = "" },
        @{ VctoolsInstallDir = "C:\stale" },
        @{ CargoLinkerPin = "C:\Program Files\Git\usr\bin\link.exe" },
        @{ VcToolsDirectoryPresent = $false },
        @{ HostLinkerPresent = $false },
        @{ WhereLinkExitCode = 1 },
        @{ LinkResolutionOrder = @("C:\Program Files\Git\usr\bin\link.exe", $script:RequiredHostX64Linker) },
        @{ LinkResolutionOrder = @() }
    )
    foreach ($overrides in $negativeCases) {
        $case = $base.Clone()
        foreach ($key in $overrides.Keys) { $case[$key] = $overrides[$key] }
        if ((Test-MsvcLinkerPinContract @case).Ok) {
            throw "A negative linker-pin self-test unexpectedly passed."
        }
    }
    if (-not (Test-MsvcLinkerPinContract @base).Ok) {
        throw "The positive linker-pin self-test failed."
    }
    $expectedPrototypes = @($script:ExpectedHeaderPrototypes |
        ForEach-Object { ConvertTo-NormalizedCPrototype -Prototype $_ })
    $canonicalHeaderProbe = $script:ExpectedHeaderPrototypes -join [Environment]::NewLine
    $commentedHeaderProbe = $canonicalHeaderProbe + @'

/* uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v2_removed(void); */
// void SYNDOCAL_ASIO_CALL syndocal_asio_v2_legacy(void);
'@
    $probeDeclarations = @(Get-HeaderContractPrototypesFromSource -Source $commentedHeaderProbe)
    if (@(Compare-Object $expectedPrototypes $probeDeclarations).Count -ne 0 -or
        $probeDeclarations.Count -ne $script:ExpectedHeaderPrototypes.Count) {
        throw "Header parser self-test accepted an export name from a comment."
    }
    $inlineCommentHeaderProbe = $canonicalHeaderProbe.Replace(
        "SyndocalAsioStringV2 *out_json",
        "SyndocalAsioStringV2 /* caller-owned output */ *out_json"
    )
    $inlineCommentDeclarations = @(Get-HeaderContractPrototypesFromSource -Source $inlineCommentHeaderProbe)
    if (@(Compare-Object $expectedPrototypes $inlineCommentDeclarations).Count -ne 0 -or
        $inlineCommentDeclarations.Count -ne $script:ExpectedHeaderPrototypes.Count) {
        throw "Header parser self-test failed to strip an inline block comment from a signature."
    }
    $mutatedHeaderProbe = $canonicalHeaderProbe.Replace(
        "syndocal_asio_v2_abi_version(void);",
        "syndocal_asio_v2_abi_version(uint32_t legacy_argument);"
    )
    $mutatedDeclarations = @(Get-HeaderContractPrototypesFromSource -Source $mutatedHeaderProbe)
    if (@(Compare-Object $expectedPrototypes $mutatedDeclarations).Count -eq 0) {
        throw "Header parser self-test accepted a mutated ABI v2 parameter list."
    }
    $duplicateHeaderProbe = $canonicalHeaderProbe + [Environment]::NewLine + $script:ExpectedHeaderPrototypes[0]
    $duplicateDeclarations = @(Get-HeaderContractPrototypesFromSource -Source $duplicateHeaderProbe)
    if ($duplicateDeclarations.Count -eq $script:ExpectedHeaderPrototypes.Count) {
        throw "Header parser self-test did not preserve a duplicate declaration for rejection."
    }
    Assert-HeaderContract -HeaderPath (Join-Path $PSScriptRoot "..\..\tools\asio-bridge\include\syndocal_asio_bridge.h")
    Write-Host "check-asio-build.ps1 self-test passed: parser, exact header signatures, whole-line/inline-comment, parameter and duplicate mutations, 9 negative linker cases, 1 positive linker case; Cargo not invoked."
}

if ($SelfTest) {
    Invoke-SelfTest
    return
}

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw "The Syndocal ASIO build gate is Windows-only."
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$pin = Read-StrictSdkPin -Path (Join-Path $repoRoot "qa\ASIO_SDK_PIN.json")
Assert-HeaderContract -HeaderPath (Join-Path $repoRoot "tools\asio-bridge\include\syndocal_asio_bridge.h")
$asioSdkDirectory = Get-RequiredDirectoryFromEnvironment -Name "CPAL_ASIO_DIR"
$sdkArchive = Get-RequiredFileFromEnvironment -Name $script:SdkArchiveVariableName
$libclangDirectory = Get-RequiredDirectoryFromEnvironment -Name "LIBCLANG_PATH"
$libclangDll = Join-Path $libclangDirectory "libclang.dll"
if (-not (Test-Path -LiteralPath $libclangDll -PathType Leaf)) {
    throw "LIBCLANG_PATH lacks libclang.dll: $libclangDll"
}
Assert-SdkArchiveExtractionProvenance -Pin $pin -ArchivePath $sdkArchive -SdkDirectory $asioSdkDirectory

$whereLinkResolution = Get-WhereLinkResolution
Write-Host "Pinned Cargo linker variable: $([Environment]::GetEnvironmentVariable($script:CargoLinkerVariableName, 'Process'))"
Write-Host "where.exe link.exe resolution order:"
$whereLinkResolution.Entries | ForEach-Object { Write-Host "  $_" }
$linkerPinContract = Test-MsvcLinkerPinContract `
    -VctoolsInstallDir ([Environment]::GetEnvironmentVariable("VCToolsInstallDir", "Process")) `
    -CargoLinkerPin ([Environment]::GetEnvironmentVariable($script:CargoLinkerVariableName, "Process")) `
    -VcToolsDirectoryPresent (Test-Path -LiteralPath $script:RequiredVcToolsInstallDir -PathType Container) `
    -HostLinkerPresent (Test-Path -LiteralPath $script:RequiredHostX64Linker -PathType Leaf) `
    -WhereLinkExitCode $whereLinkResolution.ExitCode `
    -LinkResolutionOrder $whereLinkResolution.Entries
if (-not $linkerPinContract.Ok) {
    throw $linkerPinContract.Reason
}
[Environment]::SetEnvironmentVariable("VCToolsInstallDir", $linkerPinContract.CanonicalVcTools, "Process")
[Environment]::SetEnvironmentVariable($script:CargoLinkerVariableName, $linkerPinContract.CanonicalLinker, "Process")
Write-Host "Pinned MSVC toolset: $($linkerPinContract.CanonicalVcTools)"
Write-Host "Pinned host x64 linker: $($linkerPinContract.CanonicalLinker)"

if ($PreflightOnly) {
    Write-Host "ASIO preflight-only gate passed; Cargo was not invoked."
    return
}

$manifest = Join-Path $repoRoot "tools\asio-bridge\Cargo.toml"
$targetDirectory = Join-Path $repoRoot "target\asio-qa"
Push-Location $repoRoot
try {
    Invoke-CargoChecked -Label "SDK-free ABI and lifecycle tests" -Arguments @(
        "test", "--manifest-path", $manifest, "--no-default-features", "--locked", "--all-targets", "--target-dir", $targetDirectory
    )
    Invoke-CargoChecked -Label "ASIO feature all-target check" -InventoryWarnings -Arguments @(
        "check", "--manifest-path", $manifest, "--no-default-features", "--features", "asio", "--locked", "--all-targets", "--target-dir", $targetDirectory
    )
    Invoke-CargoChecked -Label "ASIO feature deterministic tests" -Arguments @(
        "test", "--manifest-path", $manifest, "--no-default-features", "--features", "asio", "--locked", "--all-targets", "--target-dir", $targetDirectory
    )
    Invoke-CargoChecked -Label "ASIO canonical release DLL" -InventoryWarnings -Arguments @(
        "build", "--manifest-path", $manifest, "--no-default-features", "--features", "asio", "--locked", "--release", "--lib", "--target-dir", $targetDirectory
    )
} finally {
    Pop-Location
}

$releaseDirectory = Join-Path $targetDirectory "release"
$canonicalDll = Join-Path $releaseDirectory $script:CanonicalDllName
if (-not (Test-Path -LiteralPath $canonicalDll -PathType Leaf)) {
    throw "Canonical ASIO bridge DLL is missing: $canonicalDll"
}
$bridgeDlls = @(Get-ChildItem -LiteralPath $releaseDirectory -File -Filter "*asio*bridge*.dll")
if ($bridgeDlls.Count -ne 1 -or $bridgeDlls[0].Name -cne $script:CanonicalDllName) {
    throw "Release output must contain one canonical bridge DLL only; found: $($bridgeDlls.Name -join ', ')"
}
Assert-ExactBridgeExports -DllPath $canonicalDll
$dllHash = (Get-FileHash -LiteralPath $canonicalDll -Algorithm SHA256).Hash
Write-Host "ASIO bridge gate PASS: abi=2 dll=$canonicalDll sha256=$dllHash first_party_warnings=0 linker_warnings=0 hardware=NOT_RUN distribution_approved=false"
