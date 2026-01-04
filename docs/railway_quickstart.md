# Railway.app Quick Start Guide

Deploy AirCade API to Railway.app in 5 minutes.

[>> AirCade API on Railway.app <<](https://railway.com/project/002c50d8-5995-4f1a-9745-699c0286d272)

## Prerequisites

- Railway account ([sign up](https://railway.app))
- Git repository connected to GitHub

## Method 1: One-Click Deploy (Fastest)

### Via Railway Dashboard

1. Go to [railway.app](https://railway.app)
2. Click **"New Project"**
3. Select **"Deploy from GitHub repo"**
4. Choose `aircade-org/aircade-api`
5. Railway auto-detects Rust and deploys

### Add Database

1. In your project, click **"New"** → **"Database"** → **"PostgreSQL"**
2. Railway automatically connects it via `DATABASE_URL`

### Set Environment Variables

1. Go to your service → **"Variables"** tab
2. Add these variables:
   ```
   ENVIRONMENT=production
   SERVER_HOST=0.0.0.0
   LOG_LEVEL=info
   ```
3. `DATABASE_URL` and `PORT` are auto-provided by Railway

## Method 2: Railway CLI

### Install CLI

```bash
# npm
npm install -g @railway/cli

# Homebrew (macOS/Linux)
brew install railway

# Cargo (Rust)
cargo install railway-cli
```

### Deploy

```bash
# Login
railway login

# Initialize project
railway init

# Add PostgreSQL
railway add --database postgresql

# Set environment variables
railway variables set ENVIRONMENT=production SERVER_HOST=0.0.0.0 LOG_LEVEL=info

# Deploy
railway up

# View logs
railway logs --follow

# Open in browser
railway open
```

## Method 3: Automated Setup Script

### Linux/macOS

```bash
chmod +x scripts/railway-setup.sh
./scripts/railway-setup.sh
```

### Windows

```cmd
.\scripts\railway-setup.bat
```

## Verify Deployment

Check your deployment status:

```bash
# Via CLI
railway status

# View logs
railway logs

# Check health endpoint
curl https://your-app.railway.app/health
```

## Environment Variables Reference

| Variable       | Value              | Description                  |
|----------------|--------------------|------------------------------|
| `DATABASE_URL` | Auto-provided      | PostgreSQL connection string |
| `PORT`         | Auto-provided      | Dynamic port (use `$PORT`)   |
| `ENVIRONMENT`  | `production`       | Runtime environment          |
| `SERVER_HOST`  | `0.0.0.0`          | Bind to all interfaces       |
| `LOG_LEVEL`    | `info`             | Logging verbosity            |
| `RUST_LOG`     | `aircade_api=info` | Rust-specific logging        |

## Custom Domain (Optional)

1. Go to your service → **"Settings"** → **"Domains"**
2. Click **"Add Domain"**
3. Enter your domain (e.g., `api.aircade.com`)
4. Add CNAME record in your DNS:
   ```
   CNAME api.aircade.com -> yourproject.up.railway.app
   ```
5. Railway auto-provisions SSL certificate

## CI/CD Integration

### Get Railway Token

```bash
railway login
railway whoami --token
```

### Add to GitHub Secrets

1. Go to repository → **Settings** → **Secrets** → **Actions**
2. Add `RAILWAY_TOKEN` with the token value

### Enable GitHub Actions

The workflow is already configured in `.github/workflows/railway-deploy.yml`. Every push to `main` will automatically deploy.

## Troubleshooting

### Build fails with "out of memory"

Reduce optimization in `Cargo.toml`:

```toml
[profile.release]
opt-level = 2  # instead of 3
lto = "thin"   # instead of "fat"
```

### Health check fails

Ensure your API has a `/health` endpoint:

```rust
async fn health_check() -> &'static str {
    "OK"
}
```

### Can't connect to database

Verify `DATABASE_URL` is set:

```bash
railway variables | grep DATABASE_URL
```

## Next Steps

- [Full Deployment Guide](railway_deployment.md) - Comprehensive documentation
- [Railway Documentation](https://docs.railway.app) - Official Railway docs
- Monitor via Railway Dashboard for metrics and logs

## Cost

- **Free Tier**: $5/month usage credit
- **Starter Plan**: $5/month (no cold starts)
- **Pro Plan**: $20/month (team features)

## Support

- [Railway Discord](https://discord.gg/railway)
- [Railway Support](https://help.railway.app)
- [GitHub Issues](https://github.com/aircade-org/aircade-api/issues)
