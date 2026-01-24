# AirCade API - Quick Start Guide

Get up and running with the AirCade API in 5 minutes.

## Prerequisites

- [Docker Desktop](https://www.docker.com/products/docker-desktop/)

## Setup (First Time)

```bash
# 1. Clone the repository
git clone <your-repo-url>
cd aircade-api

# 2. Copy environment configuration
cp .env.example .env

# 3. Start PostgreSQL
docker-compose up -d

# 4. Run the API (migrations run automatically)
cargo run
```

The server starts on `http://localhost:3000`

## Verify Installation

```bash
# Test the health endpoint
curl http://localhost:3000/health

# Expected response:
# {"status":"ok","version":"0.1.0","database":"connected"}
```

## Development Commands

```bash
# Run the server
cargo run

# Run with auto-reload (install cargo-watch first)
cargo install cargo-watch
cargo watch -x run

# Format code
cargo fmt

# Check for issues
cargo clippy

# Build release version
cargo build --release
```

## Database Commands

```bash
# Load test data
docker exec -i aircade-postgres psql -U aircade -d aircade < scripts/seed.sql

# Connect to database
docker exec -it aircade-postgres psql -U aircade -d aircade

# View tables
\dt

# Query data
SELECT * FROM users;
SELECT * FROM games;
SELECT * FROM players;

# Exit psql
\q
```

## Stop Services

```bash
# Stop the API: Ctrl+C in terminal

# Stop PostgreSQL
docker-compose down

# Stop and remove data
docker-compose down -v
```

## Troubleshooting

**Database connection fails:**

```bash
# Check if PostgreSQL is running
docker-compose ps

# View logs
docker-compose logs postgres

# Restart
docker-compose restart
```

**Port 3000 already in use:**

```bash
# Edit .env and change SERVER_PORT
SERVER_PORT=3001
```

**Clippy errors:**

- Never use `.unwrap()` or `.expect()` - use `?` operator
- All errors must be handled with `Result<T, E>`

## Next Steps

1. Read the full [README.md](README.md) for detailed documentation
2. Check `docs/milestones_implementation_summaries/*` for architecture details
3. Review the [milestone roadmap](docs/milestones.md) for upcoming features

## Quick Reference

**Environment Variables** (.env)

```bash
DATABASE_URL=postgres://aircade:aircade@localhost:5432/aircade
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
ENVIRONMENT=development
LOG_LEVEL=debug
```

**API Endpoints**

- `GET /health` - Health check

**Database Schema**

- `users` - User accounts
- `games` - Game sessions
- `players` - Game participants

## Support

For issues or questions:

- Check the [README.md](README.md) troubleshooting section
- Review the [CLAUDE.md](CLAUDE.md) project guidelines
- Open an issue on GitHub
