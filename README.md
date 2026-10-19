# Brightcone HR API

Rust/Loco API with PostgreSQL. The current release is a platform foundation for authentication and administration; it is **not a complete HR suite or production-ready for employee/payroll data**. See [STATUS.md](STATUS.md) and [PROJECT_ASSESSMENT.md](PROJECT_ASSESSMENT.md) for shipped scope and release gaps.

## Implemented API

Paths are relative to the API origin. Authenticated requests use `Authorization: Bearer <access-token>`. Cookie-authenticated refresh/logout requests also require the configured CSRF token/header.

| Method | Path | Access | Purpose |
|---|---|---|---|
| GET | `/api/health` | Public | API and database health |
| POST | `/api/auth/login` | Public | Sign in; issue access JWT and refresh cookie |
| POST | `/api/auth/refresh` | Refresh cookie + CSRF | Rotate refresh token |
| POST | `/api/auth/logout?all={bool}` | Refresh cookie + CSRF | Revoke current or all sessions |
| POST | `/api/auth/forgot` | Public | Request password reset |
| POST | `/api/auth/reset` | Reset token | Set a new password |
| GET | `/api/me` | Authenticated | Current identity and effective permissions |
| PUT | `/api/me/password` | Authenticated | Change current password |
| GET | `/api/admin/users` | `users:manage` | List users |
| POST | `/api/admin/users` | `users:manage` | Create user |
| POST | `/api/admin/users/{pid}/unlock` | `users:manage` | Unlock account |
| POST | `/api/admin/users/{pid}/disable` | `users:manage` | Disable account and revoke sessions |
| PUT | `/api/admin/users/{pid}/role` | `users:manage` | Change user role |
| POST | `/api/admin/users/{pid}/reset-password` | `users:manage` | Issue temporary password and revoke sessions |
| GET | `/api/audit` | `audit:read` | Read audit events |

The OpenAPI document is the contract for request/response fields. There are no active Files or Approvals HTTP routes. Their current prototypes are in-memory and intentionally not mounted; do not use them for real data.

## Local development

Requirements: Rust toolchain from `rust-toolchain.toml`, Docker Desktop, Node.js/npm for the frontend, and Windows MSVC C++ build tools on Windows.

From this directory in PowerShell:

```powershell
Copy-Item .env.example .env
```

Generate a unique local signing secret and put it in ignored `.env`:

```powershell
openssl rand -base64 48
```

Never commit `.env` or reuse demo credentials outside disposable local development.

```powershell
docker compose up -d --wait postgres mailpit
make run        # API on http://localhost:5150
# In another terminal, optionally create local-only demo accounts:
make seed       # prints temporary passwords to terminal
```

The API applies additive schema sync at startup. `SCHEMA_SYNC_DATABASE_URL` must
identify a PostgreSQL role permitted to create tables, add columns, and create
indexes. The API uses the lower-privilege `DATABASE_URL` for normal queries.
In production, provision these credentials separately; schema sync never drops
tables or columns. Review `src/database/schema/manifest.rs` before deploying schema changes.

The Vite frontend defaults to proxying `/api` to `http://localhost:5150`. Mailpit is available at `http://localhost:8025`.

## Checks

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
make line-limit
```

For Docker-only backend commands, see `make help` and the Makefile. Run these checks on a clean environment before merging; native Windows dependencies can require MSVC and a writable Cargo target directory.

## Structure

```text
src/app.rs                 routes, middleware and worker registration
src/database.rs            Diesel pool and startup schema-sync lifecycle hook
src/database/schema/       additive PostgreSQL schema manifest and sync logic
src/controllers/           HTTP request handlers
src/dtos/                  request and response contracts
src/models/                persistent database models
src/{authz,audit,session}/ shared auth, authorization and audit services
src/workers/               queued background workers
config/                    development, test and production configuration
docker/                    local PostgreSQL roles and initialization
tests/                     API and integration tests
scripts/                   source-size enforcement
```

## Security and scope

Secrets are read from environment/config; the development JWT fallback was removed. Use HTTPS and production cookie settings in deployment. Current roles are global, not tenant-scoped. The rate limiter is process-local. HR domain records, durable file metadata/storage, and team-scoped approvals are not implemented. Review the assessment before exposing this API publicly.
