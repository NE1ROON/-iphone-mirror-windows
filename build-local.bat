@echo off
chcp 65001 >nul
echo ========================================
echo  iPhone Mirror Windows - Local Build
echo ========================================
echo.

REM Check for Rust
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Rust not found!
    echo.
    echo Install Rust first:
    echo   winget install Rustlang.Rust.GNU
    echo.
    echo Or download from: https://rustup.rs/
    pause
    exit /b 1
)

echo [OK] Rust found:
cargo --version

REM Check for Tauri CLI
cargo tauri --version >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] Installing Tauri CLI...
    cargo install tauri-cli
    if %errorlevel% neq 0 (
        echo [ERROR] Failed to install Tauri CLI
        pause
        exit /b 1
    )
)

echo.
echo [BUILD] Starting Tauri build...
echo This may take 5-15 minutes on first run...
echo.

cargo tauri build

if %errorlevel% equ 0 (
    echo.
    echo ========================================
    echo  BUILD SUCCESS!
    echo ========================================
    echo.
    echo MSI installer location:
    echo   src-tauri\target\release\bundle\msi\iphone-mirror-windows_1.0.0_x64.msi
    echo.
    echo You can now install and distribute this .msi file.
) else (
    echo.
    echo [ERROR] Build failed!
    echo Check the output above for errors.
)

pause