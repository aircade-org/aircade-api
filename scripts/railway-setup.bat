@echo off
REM ============================================
REM Railway.app Quick Setup Script (Windows)
REM ============================================
REM This script automates the initial Railway setup for AirCade API
REM Usage: .\scripts\railway-setup.bat

echo.
echo 🚂 AirCade API - Railway.app Setup
echo ====================================
echo.

REM Check if Railway CLI is installed
where railway >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Railway CLI not found. Installing...
    echo.

    REM Check if npm is available
    where npm >nul 2>nul
    if %ERRORLEVEL% EQU 0 (
        echo 📦 Installing via npm...
        call npm install -g @railway/cli
        echo.
        echo ✅ Railway CLI installed successfully
        echo.
    ) else (
        echo ❌ npm not found. Please install Node.js first.
        echo Download from: https://nodejs.org/
        echo Or install Railway CLI manually from: https://docs.railway.app/quick-start
        pause
        exit /b 1
    )
) else (
    echo ✅ Railway CLI is already installed
    echo.
)

REM Login to Railway
echo 🔐 Logging in to Railway...
echo This will open your browser for authentication.
echo.
call railway login

echo.
echo ✅ Logged in successfully
echo.

REM Initialize Railway project
echo 🚀 Initializing Railway project...
echo.
call railway init

echo.
echo ✅ Project initialized
echo.

REM Add PostgreSQL database
echo 🗄️  Adding PostgreSQL database...
echo.
call railway add --database postgresql

echo.
echo ✅ PostgreSQL database added
echo.

REM Set environment variables
echo ⚙️  Setting environment variables...
echo.

call railway variables set ENVIRONMENT=production
call railway variables set SERVER_HOST=0.0.0.0
call railway variables set LOG_LEVEL=info
call railway variables set RUST_LOG=aircade_api=info,tower_http=info,axum=info

echo.
echo ✅ Environment variables configured
echo.

REM Display configuration
echo 📋 Current Configuration:
echo ========================
call railway variables

echo.
echo 🎉 Setup Complete!
echo ==================
echo.
echo Next steps:
echo 1. Review your environment variables: railway variables
echo 2. Deploy your application: railway up
echo 3. View logs: railway logs --follow
echo 4. Open in browser: railway open
echo.
echo 📚 Full documentation: docs\railway-deployment.md
echo.

pause
