# Project status

Authoritative status for the Brightcone HR Portal, aligned to the agreed
**Sprint 1 task list** (6 modules, 29 tasks, tickets `P0-01` … `P1-22`).

Last updated: 13 October 2026.

---

## Summary

| | Count |
|---|---|
| ✅ Complete | **25** |
| ⚠️ Partial | **1** |
| ❌ Not started | **3** |
| **Total Sprint 1 tasks** | **29** |

| Module | Complete | Partial | Not started |
|---|---|---|---|
| 1. Infrastructure & Bootstrap | 7 | 0 | 0 |
| 2. Core Platform Foundations | 4 | 0 | 0 |
| 3. Authentication & Session Management | 5 | 0 | 0 |
| 4. Access Control & Auditing | 2 | 0 | 0 |
| 5. Shared Backend Services | 0 | 0 | 3 |
| 6. Frontend Foundation | 7 | 1 | 0 |

Sprints 2, 3 and 4 have not started. They are out of scope for this report.

### How "complete" was verified

Not by reading the code. By running it:

| Check | Command | Result |
|---|---|---|
| Backend tests | `cargo test --all-features` | **51 passed, 0 failed** |
| Frontend unit tests | `npm test` | **16 passed, 0 failed** |
| Frontend end-to-end | `npm run test:e2e` | **5 passed, 0 failed** |
| Lint | `cargo fmt --check`, `cargo clippy -D warnings`, `oxlint`, `tsc` | clean |
| The running app | `cargo run -- start` + `npm run dev` | health `200`, login `200`, authenticated reads `200` |

---

## Module 1 — Infrastructure & Bootstrap

| # | Task | Ticket | Priority | Status |
|---|---|---|---|---|
| 1.1 | Backend project bootstrap | P0-01 | P1 | ✅ Complete |
| 1.2 | Local development environment | P0-02 | P1 | ✅ Complete |
| 1.3 | Frontend project bootstrap | P0-04 | P1 | ✅ Complete |
| 1.4 | Team and repository standards | P0-06 | P1 | ✅ Complete |
| 1.5 | Backend continuous integration | P0-03 | P1 | ✅ Complete |
| 1.6 | Frontend continuous integration | P0-05 | P1 | ✅ Complete |
| 1.7 | API contract and type generation | P0-07 | P1 | ✅ Complete |

**What exists.** A Loco 1.2 Rust app on PostgreSQL 16 answering `GET /api/health` with a
real database ping. `make db-up` brings up PostgreSQL and Mailpit in one command, with
three least-privilege roles created automatically. A React 19 + Vite + TypeScript app
proxiesing `/api` to the backend so the browser stays same-origin. GitHub Actions runs
format, lint, tests and end-to-end tests on every push and pull request, in both
repos. `openapi.json` is committed, served live in development, and a test fails if it
drifts; the frontend generates its TypeScript types from it.

**Evidence.** `.github/workflows/ci.yaml` and `ci.yml`. `Makefile`. `docker-compose.yml`.
`tests/openapi.rs`.

---

## Module 2 — Core Platform Foundations

| # | Task | Ticket | Priority | Status |
|---|---|---|---|---|
| 2.1 | Database schema and migrations | P1-01 | P1 | ✅ Complete |
| 2.2 | Error handling and observability | P1-02 | P1 | ✅ Complete |
| 2.3 | Global security hardening | P1-03 | P1 | ✅ Complete |
| 2.4 | Demo and seed data | P1-14 | P1 | ✅ Complete |

**2.1 — Schema.** Four tables: `users`, `refresh_tokens`, `password_resets`,
`audit_logs`. E-mail is `citext`, so `Ana@` and `ana@` are the same account. Check
constraints pin `role` to the four valid values and `status` to three. The audit log
has triggers that block `UPDATE`, `DELETE` and `TRUNCATE`, and `app_runtime` holds no
grant for them — the table cannot be rewritten even by its owner. A regression test
proves it.

**2.2 — Errors and observability.** Every response is `{ error, description }`, and
descriptions never contain internals. Every request gets a UUID request ID, generated
if the client did not supply one, echoed in the `X-Request-Id` header and written onto
every audit row — so one identifier ties an HTTP request to its log entries.

**2.3 — Hardening.** Applied once in `src/app.rs` to every response: CSP with
`frame-ancestors 'none'`, HSTS, `nosniff`, `Referrer-Policy`, `Cache-Control: no-store`
on token responses, gzip and Brotli, a 10 MB body limit, a 15-second timeout, CORS
locked to the frontend origin with credentials, and per-IP rate limiting (10 a minute
on auth routes). Verified live against the running server.

**2.4 — Seed.** `make seed` creates 14 accounts — 1 HR Admin, 3 managers, 1 finance,
9 employees — and prints each password once. It **refuses to run in production**, and
the output is never written to a file.

---

## Module 3 — Authentication & Session Management

| # | Task | Ticket | Priority | Status |
|---|---|---|---|---|
| 3.1 | Login system | P1-04 | P1 | ✅ Complete |
| 3.2 | Session continuity | P1-05 | P1 | ✅ Complete |
| 3.3 | Session termination | P1-06 | P1 | ✅ Complete |
| 3.4 | Identity endpoint | P1-07 | P1 | ✅ Complete |
| 3.5 | Password recovery and account administration | P1-11 | P1 | ✅ Complete |

**3.1 — Login.** Email and password, Argon2id. Locks the account after 5 wrong tries,
and the counter is incremented inside a transaction with `SELECT … FOR UPDATE`, so
parallel guesses cannot slip under the limit. An unknown email still costs a full
Argon2 verification against a dummy hash, so response time cannot be used to discover
which addresses exist. A locked or disabled account is only revealed to someone who
already has the password.

**3.2 — Session continuity.** A 15-minute JWT plus a rotating refresh token in an
`HttpOnly`, `SameSite=Strict` cookie scoped to `/api/auth`. Every refresh issues a new
token and marks the old one used. Presenting an already-used token is treated as
theft and revokes the entire token family.

**3.3 — Termination.** Logout revokes the token. `?all=true` revokes every session for
the account, and disabling an account does the same automatically.

**3.4 — Identity.** `GET /api/me` returns the signed-in user, their role, whether a
password change is forced, and their resolved permission list — which is what the
frontend uses to decide what to render.

**3.5 — Recovery and administration.** Forgot password issues a single-use token and
always answers identically. Reset sets the new password and kills every session. HR
can create accounts, change roles, unlock, disable, and issue temporary passwords —
with two guardrails: you cannot demote, disable or self-lock the **last** active
HR Admin, and you cannot lock yourself out.

---

## Module 4 — Access Control & Auditing

| # | Task | Ticket | Priority | Status |
|---|---|---|---|---|
| 4.1 | Role-based access control | P1-08 | P1 | ✅ Complete |
| 4.2 | Audit trail | P1-09 | P1 | ✅ Complete |

**4.1 — Roles.** Four roles — Employee, Manager, Finance, HR Admin — against eight
permissions, each scoped `own`, `team` or `all`. The matrix is **deny-by-default**:
anything not listed is denied to everyone. A test walks every registered route and
asserts an anonymous caller gets `401`, so a new endpoint cannot ship unguarded.

*Known limit, deliberate:* the `own`/`team` scope is resolved and returned but nothing
filters records by it yet, because there are no per-employee records until Sprint 2.
Ownership enforcement is scheduled there, not missed.

**4.2 — Audit.** 14 distinct action types are recorded — login, failed login, lockout,
logout, token replay, password reset requested and completed, account created,
unlocked, disabled, role changed, password changed. Append-only at the database level,
enforced twice over, with a test that proves even the table owner cannot change a row.
Nothing sensitive is ever written to the summary field.

---

## Module 5 — Shared Backend Services

| # | Task | Ticket | Priority | Status |
|---|---|---|---|---|
| 5.1 | Background jobs and email | P1-10 | P1 | ❌ Not started |
| 5.2 | Secure file storage | P1-12 | P1 | ❌ Not started |
| 5.3 | Approvals engine | P1-13 | P1 | ❌ Not started |

**Nothing in this module is built.** Being explicit, because the permission matrix
already advertises capabilities that have no implementation behind them:

- **5.1** — `src/workers/mod.rs` is empty and `connect_workers` is a no-op. The
  Postgres job queue is configured and connects at startup, but nothing is ever
  enqueued or consumed, so email is sent synchronously inside the request. A
  consequence worth naming: `refresh_tokens::Model::delete_expired` exists but nothing
  calls it, so expired tokens and reset rows are never purged.
- **5.2** — No upload endpoint exists. `files:upload` and `files:read` are declared
  permissions that no handler uses.
- **5.3** — No approvals engine. `approvals:submit`, `:read` and `:decide` are declared
  permissions with no implementing handler.

---

## Module 6 — Frontend Foundation

| # | Task | Ticket | Priority | Status |
|---|---|---|---|---|
| 6.1 | Design system | P1-15 | P1 | ✅ Complete |
| 6.2a | API client and auth state | P1-16 | P1 | ✅ Complete |
| 6.2b | Login screen | P1-17 | P1 | ✅ Complete |
| 6.2c | Logout and session expiry | P1-18 | P1 | ✅ Complete |
| 6.2d | Forgot and reset password screens | P1-19 | P1 | ✅ Complete |
| 6.3a | App shell and route guards | P1-20 | P1 | ✅ Complete |
| 6.3b | Form, modal, drawer, toast and table kit | P1-21 | P1 | ⚠️ **Partial** |
| 6.4 | Core end-to-end test and demo | P1-22 | P1 | ✅ Complete |

**6.1 — Design system.** 15 components, 24 design tokens with a full dark counterpart
for every one, and `color-scheme` flipping with the theme. Applied before first paint,
so no flash on reload. Accessibility handled: visible focus rings, `role="alert"` on
errors, `aria-invalid` and `aria-describedby`, labelled controls, and
`prefers-reduced-motion` honoured. Responsive below 860px, where the sidebar becomes
an off-canvas drawer.

**6.2 — Client and auth state.** One fetch wrapper is the only thing that talks to the
backend. The access token lives in a JavaScript variable and **nowhere else** — not
`localStorage`, not `sessionStorage`, not a cookie — and a test asserts it. The
refresh token is an `HttpOnly` cookie the JavaScript cannot read. On a `401` the client
refreshes once and replays the request; concurrent `401`s share one in-flight refresh.
A `401` on `/api/auth/*` is never treated as expiry, so a wrong password cannot cause
a refresh loop. Signing out clears the TanStack cache, and a `BroadcastChannel` signs
out every open tab.

**6.3a — Shell and guards.** Sidebar, top bar, mobile menu, theme toggle, user block.
The guard handles four cases: still loading, not signed in, password change forced,
and permission missing. Permission checks work and are tested, though no route needs
them yet — every page so far is either public or open to any signed-in user.

**6.3b — PARTIAL.** The ticket asks for forms, modals, drawers, toasts and tables.

- Done: `Button`, `TextField`, `PasswordField`, `Alert`, `Card`, `Spinner`, `Pill`,
  `Avatar` — all in use.
- Done but not yet used: `SelectField`, `TextareaField`, `Tabs`, `ProgressBar`,
  `StatTile`, `EmptyState`, `Skeleton`.
- **Missing: `FormModal`, `ConfirmDialog`, `Drawer`, `Toast`, `DataTable`.** There is
  no modal, no drawer, no toast system and no table primitive of any kind.

This is the one task in Sprint 1 that is not finished, and it is unfinished because
nothing in Sprint 1 needs them yet. They land with the first real data screen.

**6.4 — End-to-end.** Five Playwright tests covering the whole journey: an anonymous
visitor is redirected to sign-in; sign in lands on home; sign out returns to sign-in; a
wrong password shows the server's message and stays put; a refresh cookie restores the
session on reload; and the dark theme survives a reload. The API is stubbed at the
network boundary so they run in CI without a database, but everything above that line
is the real router, the real client and the real form validation. **All 5 pass.**

---

## Known issues and deliberate limits

Not bugs in what was built — things a reader should know so the code is not
misread as more complete than it is.

| Where | What |
|---|---|
| Backend | No background workers run. See 5.1. |
| Backend | Expired tokens and reset rows are never purged. A scheduler fixes it, and there is no scheduler yet. |
| Backend | Nothing is encrypted at rest. Secrets are *hashed*, not encrypted. No sensitive columns exist yet — no bank details, PAN, UAN or salary — because employee records are Sprint 2. |
| Backend | No row-level security. Authorisation is enforced in Rust. |
| Backend | The `own`/`team` scope is computed but not used to filter records. See 4.1. |
| Backend | The common-password denylist is 10 hardcoded entries, not an external wordlist. |
| Backend | Rate limiting keys on the socket address, so it needs `trust_proxy` set to honour `X-Forwarded-For` behind a load balancer. Default is off, deliberately. |
| Frontend | `HomePage` is a placeholder. The real home is a Sprint 2 ticket. |
| Frontend | `getHealth()` is written but never called. |
| Frontend | Seven design-system components are built but unused. See 6.3b. |
| Frontend | The `permission` prop on the route guard is tested but wired into no route, because no route needs it yet. |
| Frontend | Dark mode is reachable only through the toggle. On a first visit with no stored preference and a dark operating system, the page renders light until you toggle. |
| Both | On Windows, if the repository sits inside a OneDrive folder, set `CARGO_TARGET_DIR` to a local path. See the backend README. |

---

## What to build next

In priority order.

1. **5.1 Background jobs and email** — move mail off the request path, and wire the
   scheduler that purges expired tokens.
2. **5.2 File storage** — unblocks receipts in Sprint 2.
3. **6.3b Finish the UI kit** — `FormModal`, `ConfirmDialog`, `Drawer`, `Toast`,
   `DataTable`. Unblocks every screen in Sprints 2–4.
4. **5.3 Approvals engine** — unblocks leave, expenses, assets and letters at once.
5. **2.1 → Sprint 2: employee records**, then directory and self-service.

The dependency order that matters: **5.2 and 5.3 first**, because employee records,
leave, expenses and assets all sit on top of the approvals engine and file storage.