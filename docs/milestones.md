# AirCade API Development Roadmap

This document outlines the versioned milestones for the AirCade backend API, a Rust-based infrastructure powering a browser-based party game platform where phones become controllers.

## Overview

**Tech Stack**: Rust, Axum, SeaORM, PostgreSQL, Tokio, WebSockets
**Architecture**: Layered REST/WebSocket API with real-time game state synchronization
**Quality Gates**: Strict error handling, no unwrap/panic, comprehensive testing

---

## v0.1.0 - Foundation & Core Infrastructure

**Target**: Week 1-2
**Status**: In Progress

### Goals

Establish foundational database schema, basic API structure, and development workflows.

### Deliverables

- [ ] **Database Schema Design**
    - Users table (id, username, created_at, updated_at)
    - Games table (id, code, host_id, status, settings, created_at)
    - Players table (id, game_id, user_id, nickname, color, joined_at)
    - Basic indexes and foreign key constraints

- [ ] **SeaORM Migrations**
    - Initial migration for core tables
    - Entity models generated in `src/entities/`
    - Migration rollback tested

- [ ] **Basic API Server**
    - Axum router setup with health check endpoint
    - Database connection pool initialization
    - Environment configuration loading (.env)
    - Graceful shutdown handling

- [ ] **Error Handling Framework**
    - Custom error types in `src/errors/`
    - API error responses (JSON format with error codes)
    - Error logging with tracing

- [ ] **Development Tooling**
    - Docker Compose for local PostgreSQL
    - Database seeding script for development
    - README with setup instructions

### Success Criteria

- `cargo clippy` passes with zero warnings
- Health check endpoint returns 200 OK
- Database migrations run successfully
- Error responses follow consistent JSON schema

---

## v0.2.0 - Game Session Management

**Target**: Week 3-4
**Status**: Not Started

### Goals

Implement core game lobby creation, joining, and basic state management.

### Deliverables

- [ ] **Game Creation API**
    - `POST /api/games` - Create new game session
    - Generate unique 6-character game codes (alphanumeric)
    - Return game ID and join code
    - Validate game settings (max players, game type)

- [ ] **Game Join API**
    - `POST /api/games/:code/join` - Join game by code
    - Player nickname validation and uniqueness
    - Assign player colors automatically
    - Return player token for authentication

- [ ] **Game State API**
    - `GET /api/games/:id` - Fetch game details
    - `GET /api/games/:id/players` - List all players
    - Game status enum (Lobby, Playing, Finished)

- [ ] **Game Lifecycle**
    - `POST /api/games/:id/start` - Host starts game
    - `DELETE /api/games/:id/players/:player_id` - Kick player (host only)
    - `POST /api/games/:id/leave` - Player leaves game

- [ ] **DTOs & Validation**
    - Request/response DTOs in `src/dto/`
    - Input validation with custom validators
    - Proper error messages for validation failures

### Success Criteria

- Can create game and receive unique code
- Multiple players can join same game
- Host can start game and change state
- All endpoints have integration tests
- API documentation (OpenAPI/Swagger generated)

---

## v0.3.0 - Authentication & Authorization

**Target**: Week 5-6
**Status**: Not Started

### Goals

Secure API with JWT-based authentication and role-based authorization.

### Deliverables

- [ ] **User Registration**
    - `POST /api/auth/register` - Create user account
    - Password hashing with bcrypt/argon2
    - Email/username uniqueness validation

- [ ] **User Authentication**
    - `POST /api/auth/login` - Login with credentials
    - JWT token generation (access + refresh tokens)
    - Token expiration and refresh flow
    - `POST /api/auth/refresh` - Refresh access token

- [ ] **Authorization Middleware**
    - JWT validation middleware
    - Extract user identity from token
    - Role-based access control (Host, Player, Spectator)
    - Rate limiting middleware

- [ ] **Guest Access**
    - Guest user creation (no password)
    - Anonymous gameplay support
    - Guest-to-registered user migration

- [ ] **Security Hardening**
    - CORS configuration
    - Request size limits
    - SQL injection prevention (SeaORM handles this)
    - Sensitive data masking in logs

### Success Criteria

- Users can register and login
- Protected endpoints reject invalid tokens
- Host-only actions reject non-host players
- Security audit passes (no clippy warnings)
- Integration tests for auth flows

---

## v0.4.0 - Real-Time WebSocket Communication

**Target**: Week 7-9
**Status**: Not Started

### Goals

Enable real-time bidirectional communication for live gameplay using WebSockets.

### Deliverables

- [ ] **WebSocket Infrastructure**
    - WebSocket endpoint: `WS /api/games/:id/ws`
    - Connection authentication via JWT
    - Per-game WebSocket rooms/channels
    - Connection lifecycle (connect, disconnect, reconnect)

- [ ] **Message Protocol**
    - JSON-based message format (type, payload, timestamp)
    - Server-to-client events (player_joined, player_left, game_started, game_state_update)
    - Client-to-server commands (input, action, ready_check)
    - Message validation and error handling

- [ ] **Game State Broadcasting**
    - Broadcast state changes to all connected players
    - Player-specific messages (e.g., private game data)
    - Heartbeat/ping-pong for connection health
    - Graceful degradation on disconnect

- [ ] **Input Handling**
    - Phone controller input events (touch, swipe, button press)
    - Input buffering and reconciliation
    - Latency compensation basics
    - Input validation and anti-cheat basics

- [ ] **Connection Management**
    - Track active connections per game
    - Auto-remove disconnected players after timeout
    - Reconnection support with state sync
    - Connection limits per game

### Success Criteria

- WebSocket connections stable for 30+ minutes
- Sub-100ms message latency (local network)
- All players receive broadcast messages
- Handles 8+ concurrent players per game
- Load tests pass (100+ concurrent connections)

---

## v0.5.0 - First Party Game Implementation

**Target**: Week 10-12
**Status**: Not Started

### Goals

Implement one complete party game as a reference implementation and validate the platform.

### Deliverables

- [ ] **Game Engine Abstraction**
    - Generic game trait/interface
    - Game state management pattern
    - Turn-based and real-time game support
    - Plugin architecture for multiple games

- [ ] **Reference Game: "Quick Draw"** (Drawing/Guessing Game)
    - Game-specific database schema (rounds, drawings, guesses)
    - Round management (phases: drawing, guessing, scoring)
    - Drawing data transmission (vector paths)
    - Scoring algorithm and leaderboard
    - Game completion detection

- [ ] **Game Flow Endpoints**
    - `POST /api/games/:id/rounds` - Start new round
    - `POST /api/games/:id/rounds/:round_id/submit` - Submit drawing/guess
    - `GET /api/games/:id/leaderboard` - Fetch scores

- [ ] **WebSocket Game Events**
    - `round_started` - New round begins
    - `drawing_update` - Real-time drawing sync
    - `guess_submitted` - Player submitted guess
    - `round_ended` - Scores revealed
    - `game_finished` - Final leaderboard

### Success Criteria

- Complete game playable from start to finish
- 4-8 players can play simultaneously
- No game state corruption or desyncs
- Game runs smoothly on mobile browsers
- End-to-end tests for full game flow

---

## v0.6.0 - Admin & Analytics

**Target**: Week 13-14
**Status**: Not Started

### Goals

Add administrative capabilities and usage analytics for platform monitoring.

### Deliverables

- [ ] **Admin Authentication**
    - Admin user roles and permissions
    - Admin-only endpoints (protected)
    - API key authentication for admin actions

- [ ] **Game Moderation**
    - `GET /api/admin/games` - List all active games
    - `DELETE /api/admin/games/:id` - Force end game
    - `POST /api/admin/users/:id/ban` - Ban abusive users
    - Content moderation tools (for drawings, chat)

- [ ] **Analytics & Metrics**
    - Game session metrics (duration, players, completion rate)
    - User engagement metrics (games played, retention)
    - System health metrics (API latency, error rates)
    - Export to Prometheus/Grafana

- [ ] **Audit Logging**
    - Log all admin actions
    - User activity logs (login, game join/leave)
    - Database audit trail for sensitive operations

### Success Criteria

- Admins can monitor and moderate platform
- Metrics dashboard shows real-time data
- Audit logs are tamper-proof
- Performance metrics under acceptable thresholds

---

## v0.7.0 - Spectator Mode & Social Features

**Target**: Week 15-16
**Status**: Not Started

### Goals

Enable spectators to watch games and add social features for player engagement.

### Deliverables

- [ ] **Spectator Support**
    - `POST /api/games/:code/spectate` - Join as spectator
    - Read-only WebSocket connection
    - Spectator count display
    - Spectator chat (optional)

- [ ] **Social Features**
    - User profiles (avatar, stats, game history)
    - Friend system (add, remove, list friends)
    - Friend invitations to games
    - Public game lobbies (discoverable games)

- [ ] **Chat System**
    - In-game text chat (WebSocket-based)
    - Chat message persistence (optional)
    - Profanity filter
    - Emoji support

- [ ] **Notifications**
    - Game invitations
    - Friend requests
    - Game start notifications
    - WebSocket event subscriptions

### Success Criteria

- Spectators can watch without affecting gameplay
- Friends can easily invite each other
- Chat messages delivered reliably
- Notification system works across devices

---

## v0.8.0 - Performance & Scaling

**Target**: Week 17-18
**Status**: Not Started

### Goals

Optimize API for production scale and multi-region deployment.

### Deliverables

- [ ] **Database Optimization**
    - Query optimization and indexing
    - Connection pooling tuning
    - Read replicas for scaling reads
    - Database query profiling

- [ ] **Caching Layer**
    - Redis integration for session data
    - Cache frequently accessed game states
    - Cache invalidation strategy
    - Distributed cache support

- [ ] **Load Balancing**
    - Stateless API design verification
    - Session affinity for WebSockets
    - Health check endpoints for load balancers
    - Horizontal scaling tests

- [ ] **Rate Limiting & DDoS Protection**
    - Per-IP rate limiting
    - Per-user rate limiting
    - Endpoint-specific limits
    - Rate limit headers in responses

- [ ] **Performance Testing**
    - Load tests (1000+ concurrent users)
    - Stress tests (find breaking points)
    - Latency benchmarks
    - WebSocket scalability tests

### Success Criteria

- API handles 1000+ concurrent users
- P95 latency < 200ms for REST endpoints
- WebSocket message latency < 100ms
- Database queries optimized (< 50ms)
- Cache hit rate > 80% for hot data

---

## v0.9.0 - Additional Games & Platform Polish

**Target**: Week 19-21
**Status**: Not Started

### Goals

Add 2-3 more party games and polish the platform for beta release.

### Deliverables

- [ ] **Game #2: "Trivia Battle"** (Quiz Game)
    - Question database and categories
    - Timed answering mechanics
    - Live leaderboard updates
    - Streak bonuses

- [ ] **Game #3: "Reaction Time"** (Reflex Challenge)
    - Synchronized start countdown
    - Millisecond precision timing
    - Multiple reaction challenges
    - Tournament bracket mode

- [ ] **Platform Improvements**
    - Game discovery/lobby browser
    - Game replay/recording system (optional)
    - Improved error messages and UX
    - Tutorial/onboarding flow

- [ ] **API Documentation**
    - Complete OpenAPI specification
    - Interactive API docs (Swagger UI)
    - WebSocket protocol documentation
    - SDK examples (JavaScript/TypeScript)

### Success Criteria

- 3+ games fully functional
- Platform feels polished and stable
- API documentation complete
- Beta users can play without issues

---

## v1.0.0 - Production Release

**Target**: Week 22-24
**Status**: Not Started

### Goals

Production-ready release with comprehensive monitoring, security, and deployment.

### Deliverables

- [ ] **Production Infrastructure**
    - Docker containerization
    - Kubernetes deployment manifests (optional)
    - CI/CD pipeline (GitHub Actions)
    - Automated testing in pipeline
    - Blue-green deployment support

- [ ] **Security Hardening**
    - Security audit and penetration testing
    - HTTPS enforcement
    - Content Security Policy headers
    - OWASP Top 10 compliance check

- [ ] **Monitoring & Observability**
    - Structured logging (JSON format)
    - Distributed tracing (OpenTelemetry)
    - Error tracking (Sentry integration)
    - Uptime monitoring
    - Alert configuration

- [ ] **Backup & Recovery**
    - Automated database backups
    - Disaster recovery plan
    - Point-in-time recovery tested
    - Data retention policies

- [ ] **Compliance & Legal**
    - Privacy policy implementation
    - GDPR compliance (EU users)
    - Terms of service enforcement
    - User data export/deletion APIs

- [ ] **Final Testing**
    - End-to-end test suite (100+ scenarios)
    - Security regression tests
    - Performance regression tests
    - Cross-browser compatibility tests
    - Mobile device testing matrix

### Success Criteria

- 99.9% uptime SLA capability
- All security audits passed
- Zero critical bugs
- Comprehensive monitoring in place
- Production deployment successful
- Beta users migrated smoothly

---

## Future Roadmap (Post v1.0)

### v1.1.0 - Mobile Apps

- Native iOS/Android controller apps
- Push notifications
- Offline mode support
- App store deployment

### v1.2.0 - Advanced Game Features

- Custom game creation tools
- User-generated content
- Game modding support
- Tournament mode with brackets

### v1.3.0 - Monetization

- Premium user accounts
- Custom avatars/themes
- Ad-free experience
- Game analytics for hosts

### v1.4.0 - Internationalization

- Multi-language support
- Regional game servers
- Localized content
- Currency support for payments

---

## Versioning Strategy

- **Major versions (x.0.0)**: Breaking API changes, major features
- **Minor versions (0.x.0)**: New features, backward compatible
- **Patch versions (0.0.x)**: Bug fixes, security patches

## Release Process

1. Feature development on feature branches
2. Pull request with tests and documentation
3. Code review (Clippy must pass)
4. Merge to `main` after approval
5. Tag release with version number
6. Deploy to staging environment
7. QA testing on staging
8. Deploy to production (with rollback plan)

## Success Metrics

- **Code Quality**: Zero Clippy warnings, 80%+ test coverage
- **Performance**: < 100ms API latency (P95), < 50ms WebSocket latency
- **Reliability**: 99.9% uptime, < 1% error rate
- **Scalability**: 1000+ concurrent users, 100+ simultaneous games
- **Security**: No critical vulnerabilities, OWASP compliant
