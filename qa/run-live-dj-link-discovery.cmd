@echo off
setlocal EnableExtensions EnableDelayedExpansion

set "VCVARS=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
set "EXPECTED_LINK=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe"
set "FFMPEG_DIR=C:\Users\kouty\AppData\Local\Microsoft\WinGet\Packages\Gyan.FFmpeg.Shared_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg-8.1.2-full_build-shared"
set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"

if not exist "%VCVARS%" (
  echo [ERROR] Missing vcvars64.bat: %VCVARS%
  exit /b 41
)
if not exist "%EXPECTED_LINK%" (
  echo [ERROR] Missing exact MSVC linker: %EXPECTED_LINK%
  exit /b 42
)
if not exist "%FFMPEG_DIR%\bin" (
  echo [ERROR] Missing shared FFmpeg SDK: %FFMPEG_DIR%
  exit /b 45
)
if not exist "%LIBCLANG_PATH%\libclang.dll" (
  echo [ERROR] Missing libclang.dll: %LIBCLANG_PATH%
  exit /b 46
)

call "%VCVARS%" -vcvars_ver=14.44
if errorlevel 1 exit /b %ERRORLEVEL%

set "PATH=%FFMPEG_DIR%\bin;!PATH!"

set "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=%EXPECTED_LINK%"
echo PINNED=%CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER%
echo where.exe link.exe:
where.exe link.exe
if errorlevel 1 exit /b 43

set "FIRST_LINK="
for /f "delims=" %%L in ('where.exe link.exe') do if not defined FIRST_LINK set "FIRST_LINK=%%L"
echo FIRST_LINK=!FIRST_LINK!
if /i not "!FIRST_LINK!"=="%EXPECTED_LINK%" (
  echo [ERROR] Wrong linker is first.
  exit /b 44
)

cargo test -p syndocal --bin syndocal sockaddr_in_reader_uses_sin_addr_instead_of_padding
if errorlevel 1 exit /b %ERRORLEVEL%
cargo test -p syndocal --bin syndocal live_ipv4_discovery_reports_typed_result -- --ignored --nocapture
exit /b %ERRORLEVEL%
