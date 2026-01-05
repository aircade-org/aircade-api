# AirCade API

Backend API for AirCade - a browser-based party game platform where any screen becomes the console and everyone plays together using their phones as controllers.

## Deployed on Railway.app

- [AirCade API](https://railway.com/project/002c50d8-5995-4f1a-9745-699c0286d272)
- [AirCade DataBase](https://railway.com/project/e5cd0061-1bab-4e29-944b-6999217086fb)

## Tech Stack

- **Language**: Rust
- **Web Framework**: Axum 0.8
- **Database**: PostgreSQL via SeaORM 1.1
- **Async Runtime**: Tokio 1.49
- **Logging**: Tracing + Tracing Subscriber
- **Serialization**: Serde + JSON

## Prerequisites

- Rust (install via [rustup](https://rustup.rs/))
- Docker & Docker Compose (for PostgreSQL)
- PostgreSQL client tools (optional, for manual DB access)

## Quick Start

### 1. Clone the Repository

```bash
git clone <repository-url>
cd aircade-api
```

### 2. Start PostgreSQL Database

```bash
docker-compose up -d
```

This starts a PostgreSQL 16 instance on `localhost:5432` with:

- Database: `aircade`
- User: `aircade`
- Password: `aircade`

### 3. Configure Environment

```bash
cp .env.example .env
```

Edit `.env` if needed. Default values work with the Docker Compose setup.

### 4. Run the Application

```bash
cargo run
```

The API will:

1. Load configuration from `.env`
2. Connect to PostgreSQL
3. Run database migrations automatically
4. Start the HTTP server on `http://127.0.0.1:3000`

### 5. Test the Health Check

```bash
curl http://localhost:3000/health
```

Expected response:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "database": "connected"
}
```

## Development

### Running in Development Mode

```bash
# Standard dev mode
cargo run

# With file watching (requires cargo-watch)
cargo install cargo-watch
cargo watch -x run
```

### Running in Release Mode

```bash
# Optimized build (slower compile, faster runtime)
cargo run --release
```

### Code Quality Checks

```bash
# Check code without building
cargo check

# Run linter (very strict in this project)
cargo clippy

# Format code
cargo fmt

# Run all checks
cargo fmt && cargo clippy && cargo test
```

### Database Management

#### Run Migrations

Migrations run automatically on startup. To run manually:

```bash
# Apply migrations
cargo run -- migrate up

# Rollback last migration
cargo run -- migrate down

# Check migration status
cargo run -- migrate status
```

#### Seed Development Data

```bash
# Using psql
psql -U aircade -d aircade -f scripts/seed.sql

# Or via Docker
docker exec -i aircade-postgres psql -U aircade -d aircade < scripts/seed.sql
```

#### Connect to Database

```bash
# Using psql
psql -U aircade -d aircade -h localhost

# Or via Docker
docker exec -it aircade-postgres psql -U aircade -d aircade
```

## Project Structure

```
aircade-api/
├── src/
│   ├── main.rs              # Application entry point
│   ├── api.rs               # API server orchestrator
│   ├── config/              # Configuration management
│   ├── entities/            # SeaORM database models
│   │   ├── users.rs
│   │   ├── games.rs
│   │   └── players.rs
│   ├── migrations/          # Database migrations
│   ├── routes/              # HTTP endpoint handlers
│   │   └── health.rs
│   ├── services/            # Business logic layer
│   ├── dto/                 # Data Transfer Objects
│   ├── middleware/          # HTTP middleware
│   ├── errors/              # Error types and handling
│   └── utils/               # Shared utilities
├── scripts/
│   └── seed.sql             # Development data seeding
├── docker-compose.yml       # Local PostgreSQL setup
├── .env.example             # Environment template
├── Cargo.toml               # Dependencies and config
└── README.md
```

## Database Schema

### Users Table

- `id` (integer, PK)
- `username` (string, unique)
- `created_at` (timestamp)
- `updated_at` (timestamp)

### Games Table

- `id` (integer, PK)
- `code` (string(6), unique) - Game join code
- `host_id` (integer, FK → users.id)
- `status` (string) - lobby, playing, finished
- `settings` (json, optional)
- `created_at` (timestamp)

### Players Table

- `id` (integer, PK)
- `game_id` (integer, FK → games.id)
- `user_id` (integer, FK → users.id)
- `nickname` (string)
- `color` (string) - Hex color code
- `joined_at` (timestamp)
- Unique constraints: (game_id, user_id), (game_id, nickname)

## API Endpoints

### Health Check

```
GET /health

Response:
{
  "status": "ok",
  "version": "0.1.0",
  "database": "connected"
}
```

## Code Quality Standards

This project enforces **extremely strict** linting rules:

### Forbidden

- `unsafe` code blocks
- `.unwrap()` calls
- `.expect()` calls
- `panic!()` macro

### Error Handling

All errors must be handled using `Result<T, E>` and the `?` operator.

### Linting

- All standard Clippy lints: **deny** (compilation error)
- Pedantic lints: **warn**
- Nursery lints: **warn**

Run `cargo clippy` before committing. CI/CD will reject code that doesn't pass.

## Environment Variables

| Variable       | Default       | Description                                 |
|:---------------|:--------------|:--------------------------------------------|
| `DATABASE_URL` | (required)    | PostgreSQL connection string                |
| `SERVER_HOST`  | `127.0.0.1`   | Server bind address                         |
| `SERVER_PORT`  | `3000`        | Server port                                 |
| `ENVIRONMENT`  | `development` | Environment mode                            |
| `LOG_LEVEL`    | `info`        | Log level (trace, debug, info, warn, error) |

## Troubleshooting

### Database Connection Fails

1. Ensure Docker Compose is running: `docker-compose ps`
2. Check database logs: `docker-compose logs postgres`
3. Verify connection string in `.env`

### Migrations Fail

1. Check database is accessible
2. Drop and recreate database:
   ```bash
   docker-compose down -v
   docker-compose up -d
   ```

### Clippy Warnings

This project has strict linting. Common issues:

- Never use `.unwrap()` - use `?` or handle errors properly
- Never use `.expect()` - use `?` or handle errors properly
- Avoid `panic!()` - return errors instead

## Contributing

1. Run `cargo fmt` to format code
2. Run `cargo clippy` and fix all warnings
3. Run `cargo test` to ensure tests pass
4. Commit with descriptive messages

## Related Projects

- Frontend: Next.js TypeScript implementation (separate repository): https://github.com/aircade-org/aircade-web
- Documentation: https://github.com/aircade-org/aircade-doc
