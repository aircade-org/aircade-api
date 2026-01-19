# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AirCade API is the Rust backend for AirCade — a browser-based multiplayer game creation and playing platform. The API serves three frontend experiences: the **Creative Studio** (code editor for building games), the **Console** (big screen game rendering), and the **Controller** (smartphone touch interface). Game sessions use WebSockets for real-time communication between Console and Controller clients.

This is one of three repositories:
- **aircade-doc** — Specifications and documentation (included as `docs/` git submodule)
- **aircade-api** (this repo) — Rust backend
- **aircade-web** — Next.js frontend

## Build & Development Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Optimized release build

# Run
cargo run                      # Run the API server locally

# Test
cargo test --verbose           # Run all tests
cargo test <test_name>         # Run a single test by name
cargo test --test <file>       # Run a specific integration test file

# Lint & Format
cargo fmt -- --check           # Check formatting (CI enforced)
cargo fmt                      # Auto-format code
cargo clippy --all-targets --all-features -- -D warnings  # Lint (CI enforced)

# Database (local dev)
docker-compose up -d           # Start PostgreSQL (postgres:16-alpine)
docker-compose down            # Stop PostgreSQL
```

## Strict Linting Rules

The project enforces unusually strict Clippy and Rust lints — CI will reject violations:

- **`unsafe_code`**: Forbidden entirely
- **`.unwrap()` and `.expect()`**: Denied — use proper error handling with `anyhow::Result` or `?` operator
- **`panic!()`**: Denied — all error paths must be handled gracefully
- **All Clippy lints**: Denied (pedantic and nursery as warnings)

## Tech Stack

- **Framework**: Axum 0.8 with Tower middleware
- **Async Runtime**: Tokio (full features)
- **ORM**: SeaORM 1.1 with PostgreSQL (`sea-orm-migration` for schema migrations)
- **Error Handling**: `anyhow` for error propagation, `tracing` for structured logging
- **Serialization**: Serde / serde_json
- **Config**: `dotenvy` for .env loading

## Architecture

The API follows a layered design:

1. **Routes** (Axum) — REST endpoints under `/api/v1/` + WebSocket handlers for real-time game sessions
2. **Business Logic** — Auth, game lifecycle, session management, real-time state relay
3. **Data Access** (SeaORM) — Entity models, repository pattern, PostgreSQL

Key domains: Auth (email/password, Google/GitHub OAuth), Users, Games (CRUD, versioning, publishing), Sessions (WebSocket relay between Console/Controller), Community (reviews, favorites), Discovery, Templates, Admin.

The server acts as a **message relay** for game sessions — it routes player input to the game screen and game state back to controllers, without interpreting game logic.

## Configuration

Copy `.env.example` to `.env` for local development. Key variables:
- `DATABASE_URL` — PostgreSQL connection string (default: `postgres://aircade:aircade@localhost:5432/aircade`)
- `SERVER_HOST` / `SERVER_PORT` — Bind address (default: `127.0.0.1:3000`)
- `ENVIRONMENT` — `development`, `staging`, or `production`
- `LOG_LEVEL` — `trace`, `debug`, `info`, `warn`, `error`

## Database Schema

The schema (defined in `docs/app/shared/entities.md`) has 15 entities with UUIDs as primary keys and soft deletes (`deletedAt`) on User, Game, GameAsset, and Review. Key entities: User, AuthProvider, Game, GameVersion, GameAsset, Tag, Session, Player, Review, Favorite, PlayHistory, Template, Collection.

## Deployment

- **Platform**: Railway.app (config in `railway.toml`)
- **Docker**: Multi-stage build — `rust:1-slim-bookworm` builder, `debian:bookworm-slim` runtime, runs as non-root user on port 3000
- **Health check**: `/health` endpoint (required by Railway)
- **CI**: GitHub Actions on push to `main`/`staging` and PRs to `main` — runs fmt, clippy, tests, and release build

## Documentation Reference

Comprehensive specs live in the `docs/` submodule. Key files:
- `docs/app/api/api-endpoints.md` — Complete REST API specification (~80K)
- `docs/app/shared/specification.md` — Functional specification
- `docs/app/shared/entities.md` — Full database schema
- `docs/app/shared/technical-stack.md` — Technology choices with versions
- `docs/general/milestones.md` — Development roadmap with task tracking
