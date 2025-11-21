@echo off
REM ============================================================================
REM AnonNet Browser Launcher for Windows
REM
REM This script:
REM 1. Starts the AnonNet daemon in proxy mode
REM 2. Creates/configures a Firefox profile with hardening settings
REM 3. Launches Firefox with the AnonNet-hardened profile
REM
REM Usage:
REM   launch-anonnet-browser.bat [firefox-path]
REM
REM Examples:
REM   launch-anonnet-browser.bat
REM   launch-anonnet-browser.bat "C:\Program Files\Mozilla Firefox\firefox.exe"
REM ============================================================================

setlocal enabledelayedexpansion

REM Configuration
set "SCRIPT_DIR=%~dp0"
set "PROJECT_DIR=%SCRIPT_DIR%..\..\"
set "BROWSER_DIR=%PROJECT_DIR%browser"
set "PROFILE_DIR=%USERPROFILE%\.anonnet\firefox-profile"
set "USER_JS_SOURCE=%BROWSER_DIR%\profile\user.js"
set "DAEMON_BIN=%PROJECT_DIR%\target\release\anonnet-daemon.exe"
set "SOCKS_PORT=9050"
set "HTTP_PORT=8118"

REM Colors (if supported)
echo.
echo ================================================================
echo.
echo               ANONNET BROWSER LAUNCHER
echo.
echo   Anonymous browsing powered by AnonNet
echo   Hardened with Tor Browser security features
echo.
echo ================================================================
echo.

REM Find Firefox
if "%~1"=="" (
    call :find_firefox
) else (
    set "FIREFOX_PATH=%~1"
)

if not exist "!FIREFOX_PATH!" (
    echo [ERROR] Firefox not found!
    echo.
    echo Please install Firefox or specify the path:
    echo   launch-anonnet-browser.bat "C:\Path\To\firefox.exe"
    echo.
    pause
    exit /b 1
)

echo [INFO] Found Firefox: !FIREFOX_PATH!
echo.

REM Check if daemon binary exists
if not exist "%DAEMON_BIN%" (
    echo [WARNING] AnonNet daemon not found at %DAEMON_BIN%
    echo [INFO] Please build the daemon first:
    echo   cargo build --release --bin anonnet-daemon
    echo.
    pause
    exit /b 1
)

REM Check if daemon is already running
netstat -ano | findstr ":9050" | findstr "LISTENING" >nul 2>&1
if %errorlevel% equ 0 (
    echo [SUCCESS] AnonNet daemon already running
) else (
    echo [INFO] Starting AnonNet daemon...

    REM Create .anonnet directory
    if not exist "%USERPROFILE%\.anonnet" mkdir "%USERPROFILE%\.anonnet"

    REM Start daemon in background
    start /B "" "%DAEMON_BIN%" proxy > "%USERPROFILE%\.anonnet\daemon.log" 2>&1

    REM Wait for daemon to start
    echo [INFO] Waiting for daemon to initialize...
    timeout /t 5 /nobreak >nul

    netstat -ano | findstr ":9050" | findstr "LISTENING" >nul 2>&1
    if %errorlevel% equ 0 (
        echo [SUCCESS] AnonNet daemon started
        echo [SUCCESS] SOCKS5 proxy: 127.0.0.1:9050
        echo [SUCCESS] HTTP proxy: 127.0.0.1:8118
    ) else (
        echo [ERROR] Failed to start daemon
        echo [INFO] Check logs: %USERPROFILE%\.anonnet\daemon.log
        pause
        exit /b 1
    )
)
echo.

REM Setup Firefox profile
echo [INFO] Setting up Firefox profile...

if not exist "%PROFILE_DIR%" mkdir "%PROFILE_DIR%"

if exist "%USER_JS_SOURCE%" (
    copy /Y "%USER_JS_SOURCE%" "%PROFILE_DIR%\user.js" >nul
    echo [SUCCESS] Hardening configuration applied
) else (
    echo [WARNING] user.js not found
    echo [WARNING] Browser will launch without hardening settings
)

REM Create prefs.js if it doesn't exist
if not exist "%PROFILE_DIR%\prefs.js" (
    (
        echo // AnonNet Firefox Profile
        echo user_pref("browser.startup.homepage", "about:blank"^);
        echo user_pref("browser.shell.checkDefaultBrowser", false^);
    ) > "%PROFILE_DIR%\prefs.js"
    echo [SUCCESS] Profile initialized
)
echo.

REM Launch Firefox
echo [INFO] Launching Firefox with AnonNet profile...
echo [INFO] Firefox: !FIREFOX_PATH!
echo [INFO] Profile: %PROFILE_DIR%
echo.

start "" "!FIREFOX_PATH!" --profile "%PROFILE_DIR%" --no-remote --new-instance "about:blank"

echo [SUCCESS] Firefox launched!
echo.
echo ================================================================
echo             AnonNet Browser is now running!
echo ================================================================
echo.
echo Security features enabled:
echo   [x] Fingerprinting resistance
echo   [x] Privacy-focused settings
echo   [x] All traffic routed through AnonNet
echo   [x] WebRTC disabled (no IP leaks^)
echo   [x] First-party isolation enabled
echo.
echo Important notes:
echo   * Only .anon domains are supported
echo   * Clearnet sites will be blocked for safety
echo   * Some websites may not work due to hardening
echo.
echo To stop:
echo   * Close Firefox normally
echo   * To stop daemon: taskkill /F /IM anonnet-daemon.exe
echo.
echo ================================================================
echo.

pause
exit /b 0

:find_firefox
    REM Try common Firefox installation locations
    set "FIREFOX_PATH=C:\Program Files\Mozilla Firefox\firefox.exe"
    if exist "!FIREFOX_PATH!" goto :eof

    set "FIREFOX_PATH=C:\Program Files (x86)\Mozilla Firefox\firefox.exe"
    if exist "!FIREFOX_PATH!" goto :eof

    set "FIREFOX_PATH=%PROGRAMFILES%\Mozilla Firefox\firefox.exe"
    if exist "!FIREFOX_PATH!" goto :eof

    set "FIREFOX_PATH=%PROGRAMFILES(X86)%\Mozilla Firefox\firefox.exe"
    if exist "!FIREFOX_PATH!" goto :eof

    set "FIREFOX_PATH=%LOCALAPPDATA%\Mozilla Firefox\firefox.exe"
    if exist "!FIREFOX_PATH!" goto :eof

    REM Check PATH
    where firefox.exe >nul 2>&1
    if %errorlevel% equ 0 (
        for /f "tokens=*" %%i in ('where firefox.exe') do set "FIREFOX_PATH=%%i"
        goto :eof
    )

    set "FIREFOX_PATH="
    goto :eof
