# Railway.app Deployment Guide

This guide covers deploying the AirCade API to [Railway.app](https://railway.app), a modern platform-as-a-service with excellent Rust support and built-in PostgreSQL.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Initial Setup](#initial-setup)
- [Database Configuration](#database-configuration)
- [Environment Variables](#environment-variables)
- [Deployment Process](#deployment-process)
- [Monitoring & Logs](#monitoring--logs)
- [Custom Domains](#custom-domains)
- [CI/CD Integration](#cicd-integration)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

1. **Railway Account**: Sign up at [railway.app](https://railway.app)
2. **Railway CLI** (optional but recommended):
   ```bash
   # Install via npm
   npm install -g @railway/cli

   # Or via Homebrew (macOS/Linux)
   brew install railway

   # Or via Cargo (Rust)
   cargo install railway-cli
   ```
3. **Git Repository**: Your code should be in a Git repository (GitHub, GitLab, or Bitbucket)

---

## Initial Setup

### Option 1: Deploy via Railway Dashboard (Recommended for beginners)

1. **Login to Railway**: Go to [railway.app](https://railway.app) and sign in
2. **Create New Project**: Click "New Project"
3. **Deploy from GitHub**:
    - Select "Deploy from GitHub repo"
    - Authorize Railway to access your repositories
    - Select `aircade-org/aircade-api` repository
    - Railway will auto-detect Rust and begin building

### Option 2: Deploy via Railway CLI

```bash
# Login to Railway
railway login

# Navigate to your project directory
cd /path/to/aircade-api

# Initialize Railway project
railway init

# Link to existing project (optional)
railway link [project-id]

# Deploy
railway up
```

---

## Database Configuration

Railway provides managed PostgreSQL databases with zero configuration.

### 1. Add PostgreSQL Service

**Via Dashboard:**

1. Go to your Railway project
2. Click "New" → "Database" → "Add PostgreSQL"
3. Railway automatically creates a PostgreSQL instance

**Via CLI:**

```bash
railway add --database postgresql
```

### 2. Access Database Connection String

Railway automatically injects a `DATABASE_URL` environment variable:

```
DATABASE_URL=postgresql://user:password@hostname:port/database
```

**To view it:**

- **Dashboard**: Navigate to PostgreSQL service → "Variables" tab
- **CLI**: `railway variables`

### 3. Connect Your API to Database

Railway automatically provides the `DATABASE_URL` to your Rust application. Your code should read it via:

```rust
use std::env;

let database_url = env::var("DATABASE_URL")
.expect("DATABASE_URL must be set");
```

### 4. Run Migrations

**Option A: Railway CLI (Recommended)**

```bash
# Connect to your Railway project
railway link

# Run migrations
railway run sea-orm-cli migrate up
```

**Option B: Add to Build Command**

Update `railway.toml`:

```toml
[build]
buildCommand = "cargo build --release && sea-orm-cli migrate up"
```

**Option C: Run Migrations on Startup**

Add migration logic to your `main.rs`:

```rust
use sea_orm_migration::MigratorTrait;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to database
    let db = Database::connect(&database_url).await?;

    // Run migrations
    migration::Migrator::up(&db, None).await?;

    // Start server
    start_server().await
}
```

---

## Environment Variables

### Required Variables

Configure these in Railway Dashboard → Your Service → "Variables" tab:

| Variable       | Description                  | Example                                               |
|----------------|------------------------------|-------------------------------------------------------|
| `DATABASE_URL` | PostgreSQL connection string | `postgresql://user:pass@host:5432/db` (auto-provided) |
| `SERVER_HOST`  | Server bind address          | `0.0.0.0`                                             |
| `SERVER_PORT`  | Server port                  | `$PORT` (Railway provides this)                       |
| `ENVIRONMENT`  | Runtime environment          | `production`                                          |
| `LOG_LEVEL`    | Logging verbosity            | `info`                                                |
| `RUST_LOG`     | Rust-specific logging        | `aircade_api=info,tower_http=debug`                   |

### Railway-Specific Variables

Railway automatically provides:

- `PORT`: The port your application should listen on (dynamic)
- `RAILWAY_ENVIRONMENT`: Current environment (`production`, `staging`)
- `RAILWAY_PROJECT_ID`: Unique project identifier
- `RAILWAY_SERVICE_ID`: Unique service identifier
- `RAILWAY_DEPLOYMENT_ID`: Current deployment ID

### Setting Variables

**Via Dashboard:**

1. Go to your service → "Variables" tab
2. Click "New Variable"
3. Add key-value pairs
4. Click "Deploy" to apply changes

**Via CLI:**

```bash
# Set a variable
railway variables set SERVER_HOST=0.0.0.0

# Set multiple variables
railway variables set \
  ENVIRONMENT=production \
  LOG_LEVEL=info \
  SERVER_HOST=0.0.0.0

# View all variables
railway variables
```

### Example Configuration

```bash
# Production settings
ENVIRONMENT=production
LOG_LEVEL=info
SERVER_HOST=0.0.0.0
SERVER_PORT=$PORT
RUST_LOG=aircade_api=info,tower_http=debug,axum=info

# Database (auto-provided by Railway)
DATABASE_URL=${{Postgres.DATABASE_URL}}
```

---

## Deployment Process

### Automatic Deployments (Recommended)

Railway automatically deploys on every push to your main branch:

1. Push code to GitHub:
   ```bash
   git add .
   git commit -m "feat: add new feature"
   git push origin main
   ```

2. Railway detects the push and:
    - Pulls latest code
    - Runs `cargo build --release`
    - Deploys new version
    - Performs health checks

### Manual Deployments

**Via Dashboard:**

- Go to your service → "Deployments" tab
- Click "Deploy" → "Redeploy"

**Via CLI:**

```bash
railway up
```

### Deployment Stages

1. **Build Phase**: Compiles Rust code with release profile
2. **Deploy Phase**: Starts the binary with environment variables
3. **Health Check**: Railway pings `/health` endpoint
4. **Live**: New version receives traffic

---

## Monitoring & Logs

### View Logs

**Via Dashboard:**

1. Go to your service
2. Click "Deployments" tab
3. Select a deployment to view logs in real-time

**Via CLI:**

```bash
# Stream logs in real-time
railway logs

# Follow logs (like tail -f)
railway logs --follow
```

### Metrics Dashboard

Railway provides built-in metrics:

- **CPU Usage**: Real-time CPU consumption
- **Memory Usage**: RAM utilization
- **Network**: Inbound/outbound traffic
- **Request Count**: HTTP requests per minute

Access via: Service → "Metrics" tab

### Alerts (Pro Plan)

Set up alerts for:

- High CPU/memory usage
- Deployment failures
- Service downtime

---

## Custom Domains

### Add Custom Domain

1. **Via Dashboard:**
    - Go to your service → "Settings" tab
    - Scroll to "Domains" section
    - Click "Add Domain"
    - Enter your domain (e.g., `api.aircade.com`)

2. **Configure DNS:**
    - Add a CNAME record pointing to Railway's provided endpoint:
      ```
      CNAME api.aircade.com -> yourproject.up.railway.app
      ```

3. **SSL Certificate:**
    - Railway automatically provisions SSL certificates via Let's Encrypt
    - HTTPS is enabled by default

### Multiple Domains

You can add multiple domains to the same service:

- `api.aircade.com` (production)
- `staging-api.aircade.com` (staging environment)

---

## CI/CD Integration

### GitHub Actions

Create `.github/workflows/deploy.yml`:

```yaml
name: Deploy to Railway

on:
  push:
    branches: [ main ]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Railway CLI
        run: npm install -g @railway/cli

      - name: Deploy to Railway
        env:
          RAILWAY_TOKEN: ${{ secrets.RAILWAY_TOKEN }}
        run: railway up --detach
```

**Get Railway Token:**

```bash
# Generate token
railway login

# Get token
railway whoami --token
```

Add the token to GitHub Secrets: Repository → Settings → Secrets → "RAILWAY_TOKEN"

### GitLab CI

Create `.gitlab-ci.yml`:

```yaml
deploy:
  stage: deploy
  image: node:18
  only:
    - main
  script:
    - npm install -g @railway/cli
    - railway up --detach
  environment:
    name: production
    url: https://yourproject.up.railway.app
```

---

## Troubleshooting

### Build Failures

**Error: "Out of memory during build"**

Solution: Increase build resources in Railway settings or optimize `Cargo.toml`:

```toml
[profile.release]
# Reduce optimization to save memory
opt-level = 2  # instead of 3
lto = "thin"   # instead of "fat"
codegen-units = 4  # instead of 1
```

**Error: "Build timeout"**

Solution: Railway's default timeout is 15 minutes. Complex Rust projects may need more time. Contact Railway support to increase timeout.

### Runtime Failures

**Error: "Health check failed"**

Solution: Ensure your `/health` endpoint is implemented:

```rust
use axum::{Router, routing::get, Json};
use serde_json::json;

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

let app = Router::new()
.route("/health", get(health_check));
```

**Error: "Connection refused"**

Solution: Ensure you're binding to `0.0.0.0` and using Railway's `$PORT`:

```rust
let port = env::var("PORT").unwrap_or_else( | _ | "3000".to_string());
let addr = format!("0.0.0.0:{}", port);
let listener = tokio::net::TcpListener::bind(addr).await?;
```

### Database Issues

**Error: "Could not connect to database"**

Solution: Verify `DATABASE_URL` is set:

```bash
railway variables | grep DATABASE_URL
```

**Error: "SSL required"**

Solution: Add SSL parameters to your SeaORM connection:

```rust
let mut opt = ConnectOptions::new(database_url);
opt.sqlx_logging(true)
.sqlx_logging_level(log::LevelFilter::Debug);

let db = Database::connect(opt).await?;
```

### Performance Issues

**Problem: Slow cold starts**

Solution: Railway keeps services running on paid plans. Free tier has cold starts.

**Problem: High memory usage**

Solution: Monitor via Railway metrics and optimize Rust code. Consider using `jemalloc` allocator:

```toml
[dependencies]
jemallocator = "0.5"
```

```rust
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
```

---

## Cost Optimization

### Free Tier Limits

- $5 of usage per month (free)
- Includes compute, database, and egress
- Cold starts after 10 minutes of inactivity

### Production Recommendations

- **Starter Plan** ($5/month): Removes cold starts, better for APIs
- **Database Scaling**: Start small, scale as needed
- **Monitor Usage**: Check dashboard regularly to avoid surprises

---

## Additional Resources

- [Railway Documentation](https://docs.railway.app)
- [Railway Discord Community](https://discord.gg/railway)
- [Railway Status Page](https://status.railway.app)
- [Rust on Railway Guide](https://docs.railway.app/guides/rust)

---

## Quick Reference

```bash
# Essential commands
railway login              # Authenticate
railway init               # Create new project
railway link               # Link to existing project
railway up                 # Deploy current directory
railway logs               # View logs
railway logs --follow      # Stream logs
railway variables          # List environment variables
railway variables set KEY=VALUE  # Set variable
railway run <command>      # Run command in Railway environment
railway open               # Open project in browser
railway status             # Check deployment status
```

---

**Need Help?** Open an issue in the repository or contact the Railway support team.
