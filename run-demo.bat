@echo off
REM Run forge-e2e: E2E validation tool for forge-demo
REM Usage: run-demo.bat [--all]
REM
REM Downloads forge-e2e from GitHub releases (or builds locally)
REM Requires forge-demo binary in bin\ (build from main forge repo)

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"
set "BIN_DIR=%SCRIPT_DIR%bin"
set "FORGE_E2E=%BIN_DIR%\forge-e2e.exe"
set "FORGE_DEMO=%BIN_DIR%\forge-demo.exe"
set "E2E_ARCHIVE=forge-e2e-x86_64-pc-windows-msvc.zip"
set "E2E_URL=https://github.com/royalbit/forge-demo/releases/latest/download/%E2E_ARCHIVE%"

REM Create bin directory
if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"

REM Check for forge-demo binary
if not exist "%FORGE_DEMO%" (
    echo forge-demo binary not found at %FORGE_DEMO%
    echo.
    echo To build forge-demo from the main forge repo:
    echo   cd \path\to\forge
    echo   cargo build --release --bin forge-demo
    echo   copy target\release\forge-demo.exe %BIN_DIR%\
    echo.

    REM Try to build from parent directory if forge repo exists
    set "FORGE_REPO=%SCRIPT_DIR%..\forge"
    if exist "!FORGE_REPO!\Cargo.toml" (
        echo Found forge repo at !FORGE_REPO!, building...
        cargo build --release --bin forge-demo --manifest-path "!FORGE_REPO!\Cargo.toml"
        copy "!FORGE_REPO!\target\release\forge-demo.exe" "%FORGE_DEMO%"
        echo Built forge-demo successfully
    ) else (
        echo Error: forge repo not found. Please build forge-demo manually.
        exit /b 1
    )
)

REM Download forge-e2e if not present or outdated
echo Checking forge-e2e...
set "ARCHIVE_PATH=%BIN_DIR%\%E2E_ARCHIVE%"

if not exist "%FORGE_E2E%" (
    echo Downloading forge-e2e...
    curl -fsSL -o "%ARCHIVE_PATH%" "%E2E_URL%"
    if errorlevel 1 (
        echo Download failed. Building locally...
        where cargo >nul 2>nul
        if errorlevel 1 (
            echo Error: cargo not found. Install Rust or wait for forge-e2e release.
            exit /b 1
        )
        cargo build --release --manifest-path "%SCRIPT_DIR%Cargo.toml"
        copy "%SCRIPT_DIR%target\release\forge-e2e.exe" "%FORGE_E2E%"
        echo Built forge-e2e successfully
    ) else (
        echo Extracting...
        powershell -Command "Expand-Archive -Path '%ARCHIVE_PATH%' -DestinationPath '%BIN_DIR%' -Force"
        del "%ARCHIVE_PATH%"
        echo Downloaded forge-e2e
    )
) else (
    echo forge-e2e is available
)

echo.
"%FORGE_E2E%" %*
