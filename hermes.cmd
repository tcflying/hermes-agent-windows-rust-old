@echo off
REM Hermes Agent Windows Launcher (batch version)
REM Usage: hermes.cmd chat
REM        hermes.cmd status
REM        hermes.cmd dashboard

setlocal enabledelayedexpansion
cd /d "%~dp0"

if not exist "venv\Scripts\activate.bat" (
    echo.
    echo ❌ Virtual environment not found.
    echo Run: bash setup-hermes.sh
    echo.
    exit /b 1
)

call venv\Scripts\activate.bat
python cli.py %*
