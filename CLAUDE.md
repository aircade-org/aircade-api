# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**aircade-api** is the backend API for AirCade, a browser-based party game platform where any screen becomes the console and everyone plays together using their phones as controllers. This Rust API serves as the backend infrastructure, currently in the initial setup phase with strict quality gates configured.

Related repositories:
- Frontend: Next.js TypeScript implementation (separate repository)
- Documentation: https://github.com/aircade-org/aircade-doc (note: technical stack may be outdated)

## Essential Commands

### Development
```bash
# Run the application
cargo run

# Run with optimizations (faster than dev, slower than release)
cargo run --release

# Watch mode (requires cargo-watch)
cargo watch -x run
```

### Testing
```bash
# Run all tests
cargo test

# Run a specific test
cargo test <test_name>

# Run tests and show output
cargo test -- --nocapture

# Run tests in a specific module
cargo test <module_name>::
```

### Linting & Code Quality
```bash
# Check code without building
cargo check

# Run Clippy linter (very strict in this project)
cargo clippy

# Run Clippy with all warnings
cargo clippy -- -W clippy::all -W clippy::pedantic

# Format code
cargo fmt

# Check formatting without modifying files
cargo fmt -- --check
```

### Building
```bash
# Development build
cargo build

# Release build (heavily optimized)
cargo build --release
```

## Architecture

This is a layered Rust backend using **Axum** web framework and **SeaORM** for database access.

### Module Responsibilities

- **`main.rs`**: Application entry point, initializes all modules
- **`api.rs`**: Central orchestrator that launches the API server
- **`routes/`**: HTTP endpoint handlers (controllers in MVC terms)
- **`services/`**: Business logic layer - core application logic lives here
- **`entities/`**: SeaORM database entity models (corresponds to DB tables)
- **`dto/`**: Data Transfer Objects for request/response payloads
- **`middleware/`**: HTTP middleware for cross-cutting concerns (auth, logging, etc.)
- **`errors/`**: Custom error types and error handling utilities
- **`config/`**: Application configuration management (.env loading, settings)
- **`migrations/`**: SeaORM database migration files
- **`utils/`**: Shared utility functions

### Data Flow Pattern

```
HTTP Request
    ↓
routes/ (handlers)
    ↓
middleware/ (auth, validation, logging)
    ↓
services/ (business logic)
    ↓
entities/ (ORM models) + dto/ (serialization)
    ↓
PostgreSQL Database
    ↓
errors/ (propagate errors back up)
```

### Tech Stack

- **Web Framework**: Axum 0.8 (built on Tower/Tokio)
- **Database**: PostgreSQL via SeaORM 1.1 + SQLx
- **Async Runtime**: Tokio 1.49 (full features)
- **Serialization**: Serde + serde_json
- **Error Handling**: anyhow for context-rich errors
- **Logging**: tracing + tracing-subscriber
- **Config**: dotenvy for .env file loading

## Code Quality Standards

This project enforces **extremely strict** linting rules:

### Forbidden
- `unsafe` code blocks (memory safety)
- `.unwrap()` calls (must handle errors properly)
- `.expect()` calls (must handle errors properly)
- `panic!()` macro (must fail gracefully)

### Error Handling
All errors must be propagated using `Result<T, E>` and `?` operator. Use `anyhow::Result` for convenience or custom error types from `errors/` module.

❌ **Never do this:**
```rust
let value = some_operation().unwrap();
let value = some_operation().expect("failed");
```

✅ **Always do this:**
```rust
let value = some_operation()?;
// or
let value = some_operation().map_err(|e| /* convert error */)?;
```

### Linting Levels
- All standard Clippy lints: **deny** (compilation error)
- Pedantic lints: **warn**
- Nursery lints: **warn**
- Cargo lints: **warn**

Run `cargo clippy` frequently. The CI/CD will reject code that doesn't pass.

## Database & Migrations

### SeaORM Setup

The project uses SeaORM with:
- PostgreSQL backend (`sqlx-postgres`)
- Tokio runtime with TLS (`runtime-tokio-rustls`)
- Entity derive macros

### Creating Migrations

```bash
# Create a new migration
sea-orm-cli migrate generate <migration_name>

# Run pending migrations
sea-orm-cli migrate up

# Rollback last migration
sea-orm-cli migrate down

# Check migration status
sea-orm-cli migrate status
```

### Defining Entities

Entities go in `src/entities/`. Each entity maps to a database table. Use SeaORM's derive macros:

```rust
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub username: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

## Build Profiles

### Development (`cargo build`)
- Optimization level: 1 (basic optimizations for faster iteration)

### Release (`cargo build --release`)
Aggressively optimized for production:
- Optimization level: 3 (maximum)
- Link-Time Optimization: enabled (fat LTO)
- Single codegen unit
- Debug symbols stripped (~30-40% smaller binary)
- Panic mode: abort (no stack unwinding)
- Overflow checks: disabled

**Note**: Release builds take significantly longer to compile but produce highly optimized binaries.

## Environment Configuration

Copy `.env.example` to `.env` and configure:

```bash
cp .env.example .env
```

The application uses `dotenvy` to load environment variables. Add database URL, API keys, etc. to `.env`.

## Current State

This project is freshly initialized with:
- ✅ Framework and dependency setup complete
- ✅ Strict linting configuration in place
- ✅ Optimized build profiles configured
- ⏳ Business logic modules (routes, services, entities, etc.) are empty placeholders
- ⏳ No tests implemented yet
- ⏳ No database migrations created yet

When implementing features, follow the layered architecture pattern and maintain the strict error handling standards.
