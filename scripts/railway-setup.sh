#!/bin/bash
# ============================================
# Railway.app Quick Setup Script
# ============================================
# This script automates the initial Railway setup for AirCade API
# Usage: ./scripts/railway-setup.sh

set -e  # Exit on error

echo "🚂 AirCade API - Railway.app Setup"
echo "===================================="
echo ""

# Check if Railway CLI is installed
if ! command -v railway &> /dev/null; then
    echo "❌ Railway CLI not found. Installing..."
    echo ""

    # Detect OS and install accordingly
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "📦 Detected macOS. Installing via Homebrew..."
        brew install railway
    elif command -v npm &> /dev/null; then
        echo "📦 Installing via npm..."
        npm install -g @railway/cli
    elif command -v cargo &> /dev/null; then
        echo "📦 Installing via Cargo..."
        cargo install railway-cli
    else
        echo "❌ Could not install Railway CLI automatically."
        echo "Please install manually from: https://docs.railway.app/quick-start"
        exit 1
    fi

    echo "✅ Railway CLI installed successfully"
    echo ""
else
    echo "✅ Railway CLI is already installed"
    echo ""
fi

# Login to Railway
echo "🔐 Logging in to Railway..."
echo "This will open your browser for authentication."
echo ""
railway login

echo ""
echo "✅ Logged in successfully"
echo ""

# Initialize Railway project
echo "🚀 Initializing Railway project..."
echo ""
railway init

echo ""
echo "✅ Project initialized"
echo ""

# Add PostgreSQL database
echo "🗄️  Adding PostgreSQL database..."
echo ""
railway add --database postgresql

echo ""
echo "✅ PostgreSQL database added"
echo ""

# Set environment variables
echo "⚙️  Setting environment variables..."
echo ""

railway variables set \
  ENVIRONMENT=production \
  SERVER_HOST=0.0.0.0 \
  LOG_LEVEL=info \
  RUST_LOG=aircade_api=info,tower_http=info,axum=info

echo ""
echo "✅ Environment variables configured"
echo ""

# Display configuration
echo "📋 Current Configuration:"
echo "========================"
railway variables

echo ""
echo "🎉 Setup Complete!"
echo "=================="
echo ""
echo "Next steps:"
echo "1. Review your environment variables: railway variables"
echo "2. Deploy your application: railway up"
echo "3. View logs: railway logs --follow"
echo "4. Open in browser: railway open"
echo ""
echo "📚 Full documentation: docs/railway-deployment.md"
echo ""
