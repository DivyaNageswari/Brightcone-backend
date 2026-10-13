# Brightcone People â€” Backend

Rust + [Loco](https://loco.rs) API with PostgreSQL for the Brightcone HR portal.

The React frontend lives in [`brightcone-hr-frontend`](https://github.com/koushikpranank/brightcone-hr-frontend).

---

## Status: what actually works today

Sprint 1 is the only sprint built. Everything below is **verified working** â€” the
repository's own test suite proves it, not just the source code.

| Sprint 1 module | State |
|---|---|
| 1. Infrastructure & Bootstrap | **Complete** â€” Rust + PostgreSQL app, one-command Docker database, CI on every push |
| 2. Core Platform Foundations | **Complete** â€” schema and migrations, request IDs and one error format, global security hardening |
| 3. Authentication & Session Management | **Complete** â€” login, rotating refresh tokens, logout, identity endpoint, password recovery |
| 4. Access Control & Auditing | **Complete** â€” four roles with a permission matrix, append-only audit trail |
| 5. Shared Backend Services | **Not started** â€” no background workers, no file storage, no approvals engine |
| 6. Frontend Foundation | In the frontend repo â€” design system, API client, login state, app shell, e2e test |

Sprint 1 ticket-by-ticket status is in [STATUS.md](STATUS.md).

### What the API can do right now

| Method | Path | What it does |
|---|---|---|
| `GET` | `/api/health` | Liveness plus a real database ping. `200` ok, `503` if the database is unreachable. |
| `GET` | `/api/openapi.json` | The full API contract. Development only; not mounted in production. |
| `POST` | `/api/auth/login` | Signs in with email and password. Returns a 15-minute JWT plus an `HttpOnly` refresh cookie. Locks the account after 5 wrong tries. |
| `POST` | `/api/auth/refresh` | Rotates the refresh token. Replaying an old one revokes the whole token family. |
| `POST` | `/api/auth/logout` | Revokes the token. `?all=true` revokes every session for the account. |
| `POST` | `/api/auth/forgot` | Sends a reset link. Always answers the same way, so it cannot be used to discover which emails exist. |
| `POST` | `/api/auth/reset` | Sets a new password from the emailed token, then signs out every session. |
| `GET` | `/api/me` | Who is signed in and what they may do. |
| `PUT` | `/api/me/password` | Changes the password and re-issues the session. |
| `GET` | `/api/admin/users` | Lists accounts. Requires `users:manage`. |
| `POST` | `/api/admin/users` | Creates an account. The temporary password is returned once. |
| `POST` | `/api/admin/users/{pid}/unlock` | Unlocks a locked account. |
| `POST` | `/api/admin/users/{pid}/disable` | Disables an account and revokes its tokens. |
| `PUT` | `/api/admin/users/{pid}/role` | Changes an account's role. |
| `POST` | `/api/admin/users/{pid}/reset-password` | Issues a temporary password and revokes tokens. |
| `GET` | `/api/audit` | Reads the audit trail. Requires `audit:read`. |

---

## Run it

### You need

- Rust `1.99.0` (pinned in `rust-toolchain.toml`)
- Docker, for PostgreSQL and Mailpit
- On Windows: the MSVC C++ build tools, which Rust needs to link

### Then

```bash
cp .env.example .env

make db-up       # PostgreSQL 16 + Mailpit in Docker
make migrate     # create the schema
make seed        # 14 demo accounts; prints each password ONCE
make run         # http://localhost:5150
```

```bash
curl http://localhost:5150/api/health
# {"status":"ok","database":"ok"}
```

`make seed` prints passwords to the terminal and they are never written to a file.
Write them down as they scroll past.

### No Rust installed? Use Docker for everything.

```bash
cp .env.example .env
docker compose up -d --wait postgres mailpit
docker compose --profile app run --rm api cargo run -- db migrate
docker compose --profile app up api
```

The first build takes roughly 20 minutes. Later builds are fast because the compiled
output lives in a Docker volume.

No `make`? Every target is one or two plain commands â€” see the [Makefile](Makefile).

### If you are on Windows and Cargo fails oddly

Cargo build scripts can fail with `output path is not a writable directory` when the
repository sits inside a OneDrive folder, because OneDrive locks files while it
syncs. Point the build cache somewhere local:

```powershell
$env:CARGO_TARGET_DIR = "C:\Users\$env:USERNAME\AppData\Local\Temp\cargo-target-brightcone"
```

---

## How it is put together

```
src/app.rs          Hooks: routes, middlewares, tasks, migrations
src/controllers/    HTTP handlers. Parse, call a model, render.
src/models/         Database models and the domain rules that belong to them
src/views/          JSON response shapes. Handlers never return raw rows.
src/authz.rs        The role and permission matrix
src/audit.rs        Writes audit rows
src/session.rs      Refresh-cookie and anti-CSRF handling
src/rate_limit.rs   Per-IP rate limiting
src/request_meta.rs Pulls the IP and request ID out of each request
src/settings.rs     Tunables read from config/*.yaml
src/openapi.rs      The API contract
migration/          Database change scripts, applied in order
config/             development / test / production yaml. Secrets come from env.
docker/             Postgres init script: roles and databases
tests/              API and model tests
```

### Two database roles, on purpose

| Role | Can do | Used by |
|---|---|---|
| `app_migrator` | Create and change tables | `make migrate`, and the tests |
| `app_runtime` | Read and write rows only â€” cannot CREATE, ALTER or DROP | the running API |
| `app_queue` | Create objects only inside its own `queue` schema | the Postgres job queue |

A bug or an attack on the running API cannot change or drop the table structure.

The roles are created by [`docker/postgres-init/01-roles.sh`](docker/postgres-init/01-roles.sh)
the first time the Postgres volume is created. To start over from an empty database,
run `make db-reset`.

### Security decisions worth knowing

- **Passwords** are hashed with Argon2id. Tokens and reset links are stored as SHA-256
  hashes, so a database leak does not hand out working credentials.
- **Refresh tokens rotate.** Each use issues a new token. Presenting an
  already-used token is treated as theft and revokes the whole family.
- **The refresh token is an `HttpOnly` cookie** scoped to `/api/auth` with
  `SameSite=Strict`. Cookie-authenticated endpoints also require an
  `X-Requested-With: brightcone` header, which a cross-site form cannot set.
- **The access token is never written to web storage.** It lives in a JavaScript
  variable in the frontend and disappears on reload; the frontend then exchanges the
  cookie for a new one.
- **The audit log is append-only**, enforced by the database rather than by
  convention: a `BEFORE UPDATE OR DELETE` trigger and a `BEFORE TRUNCATE` trigger both
  raise, and `app_runtime` holds no `UPDATE`, `DELETE` or `TRUNCATE` grant. Even the
  table owner cannot rewrite history. A regression test proves it.
- **Login does not leak which emails exist.** An unknown email still costs a full
  Argon2 verification against a dummy hash, so the response time is indistinguishable.
- **Failed logins are counted inside a transaction with `SELECT â€¦ FOR UPDATE`,**
  so parallel guesses cannot slip past the lockout threshold.

### Roles and permissions

Four roles. Anything not listed is denied to everyone â€” the matrix is deny-by-default.

| Permission | employee | manager | finance | hr_admin |
|---|---|---|---|---|
| `profile:read` | own | own | own | own |
| `approvals:submit` | own | own | own | own |
| `files:upload` | own | own | own | own |
| `approvals:read` | â€” | team | all | all |
| `approvals:decide` | â€” | team | all | all |
| `files:read` | own | own | own | all |
| `users:manage` | â€” | â€” | â€” | all |
| `audit:read` | â€” | â€” | â€” | all |

### Global hardening

Applied once, to every response, in `src/app.rs`:

- `Content-Security-Policy` with `frame-ancestors 'none'`
- `Strict-Transport-Security`
- `X-Content-Type-Options: nosniff`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Cache-Control: no-store` on anything that carries a token
- gzip and Brotli compression
- A 10 MB request body limit and a 15-second request timeout
- CORS restricted to the frontend origin, with credentials
- Per-IP rate limiting â€” 10 attempts a minute on the auth routes
- A UUID request ID on every request, echoed back and recorded in the audit log

---

## API contract

[`openapi.json`](openapi.json) is committed and is the contract between the two repos.

- While developing, `GET /api/openapi.json` serves it live.
- It is **not** mounted in production â€” the docs are for developers.
- A test fails if the committed file is stale. Regenerate with `make openapi`, then
  copy it into the frontend repo and run `npm run gen:api`.

---

## Environment variables

See [`.env.example`](.env.example). Development has safe defaults.
**Production refuses to start without** `DATABASE_URL`, `QUEUE_URL`, `JWT_SECRET`,
`HOST` and `MAILER_HOST`.

Two of them catch people out:

- **`JWT_SECRET` must be valid standard base64.** Loco signs tokens with
  `EncodingKey::from_base64_secret()`, so a plain string like `my-secret` fails at
  login with `jwt error Base64(InvalidByte(..))`. Generate one with
  `openssl rand -base64 48`.
- **Tests use `TEST_DATABASE_URL`** (the `brightcone_test` database) and **wipe it**.
  They never read `DATABASE_URL`, so they cannot erase your development data.

---

## Checks

The same commands CI runs:

```bash
make lint    # cargo fmt --check + clippy -D warnings
make test    # 51 tests, needs the test database from make db-up
make openapi # regenerate openapi.json
```

## Tech choices

| Question | Answer | Why |
|---|---|---|
| Framework | Loco `1.2` | Rails for Rust. Gives routing, migrations, the queue, the mailer and a test harness in one dependency. |
| Database | PostgreSQL 16, SeaORM 2 | |
| Background queue | **Postgres-backed** | One less service to run in development. No Redis. |
| Mailer | SMTP | Mailpit catches mail locally; the inbox is at http://localhost:8025 |
| Tests | `cargo test` + `axum-test` | Real HTTP requests against a real database, so tests cannot pass against a mock that drifted. |

---

## What is deliberately not built yet

Sprint 1 modules 5 is not started. Being explicit so nobody reads this file as a
claim of completeness:

- **No background workers.** `src/workers/mod.rs` is empty. The Postgres job queue is
  configured and connected, but nothing is ever enqueued or consumed.
- **No file storage.** The `files:upload` and `files:read` permissions are declared in
  the matrix, but no endpoint uses them.
- **No approvals engine.** `approvals:submit`, `:read` and `:decide` are declared
  permissions with no implementing handler.
- **No encryption at rest.** Secrets are hashed, not encrypted. There are no
  sensitive columns yet â€” no bank details, PAN, UAN or salary â€” because the employee
  records module has not been built.
- **No row-level security.** Authorisation is enforced in Rust. Ownership and
  team-scope filtering arrives with the employee records in Sprint 2.
- **Expired rows are never purged.** `refresh_tokens::Model::delete_expired` exists but
  nothing calls it, because there is no scheduler yet.
- **No Sprint 2â€“4 modules.** No employee records, directory, leave, attendance,
  expenses, helpdesk, payroll, recruitment, assets, reports or audit viewer UI.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for branch names, commit style and the
definition of done. Every pull request is checked automatically; broken code cannot
reach `main`.