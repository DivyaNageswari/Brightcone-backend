# Backend Status

Last reviewed: 19 October 2026. This file reports the code currently in this branch; it does not certify production readiness. See [PROJECT_ASSESSMENT.md](PROJECT_ASSESSMENT.md) for detailed API/schema and release review, and [PGS_TASKS.md](PGS_TASKS.md) for the delivery checklist.

## Implemented and currently exposed

- PostgreSQL persistence for users, refresh sessions, password resets, audit events, and the framework queue.
- Login, rotating refresh cookies, logout, password recovery, current-user profile, role/permission checks, admin user management, and audit read APIs.
- Request IDs, security headers, request limits/timeouts, configured frontend CORS, and queued email infrastructure.
- Refresh-token cleanup worker registration.
- Frontend authentication and People Plus shell are in the separate frontend repository.

## People Plus platform tasks

- PGS-02: OpenAPI specification, API documentation publishing workflow, generated frontend declarations, CI drift check, issue/label templates, and GitHub project setup instructions are present. Pages and the organization project board still require GitHub organization configuration.
- PGS-05: Password recovery and HR account administration endpoints, UI integration in the frontend repo, role checks, audit events, and request tests are implemented. Database and mail runtime checks remain.
- PGS-08: Versioned 50-type record catalog, typed validation, fixed computed fields, unique claims, and catalog endpoint are implemented against the initial contract. Product approval is required before broad access.
- PGS-09: Generic record CRUD, archive/restore, append-only history, and audit integration are implemented. PostgreSQL verification and retention/access policy approval remain.
- PGS-10: Bounded CSV and versioned JSON backup/import with validation, deduplication, transactionality, and no-overwrite/no-delete behavior are implemented. Database verification and backup key/encryption policy review remain.
- PGS-11: Six-role deny-by-default authorization, nine disabled-by-default rules, durable event outbox, notification worker and APIs are implemented. Queue/database verification and operational alerting remain.
- These shared services do not replace module-specific workflows. Employee, leave, payroll, recruitment, expense, and other domain modules still require durable records and operational authorization before production use.

## Not implemented as production features

- Durable employee/org/team records, leave, payroll, expenses, recruitment, benefits, schedules, learning, feed, and reporting workflows.
- Durable upload metadata/object storage and durable approvals. Existing handlers are prototypes and are not registered as routes.
- Tenant isolation; current roles are global. Multi-replica rate limiting and operational deployment controls.
- Phase 1-3 handlers under `src/modules/` use process-local sample stores and lack the required authentication/authorization integration. Their router is deliberately not registered; do not expose these endpoints until persistent storage and access controls are implemented.

## Branch work

- Replaced Loco/SeaORM persistence with Diesel async PostgreSQL pool and additive startup schema sync; removed the migration crate, generated entity models, and migration CLI commands.
- Ported auth/session, password reset, user administration, audit, health, task, worker, and request test database operations to Diesel.
- Added a dedicated schema-sync database role and kept the runtime role separate; startup sync creates missing tables/columns, indexes, and constraints without dropping tables or columns.
- Removed the committed development JWT fallback and sanitized tracked documentation that contained demo credentials. Rotate any credential ever used outside disposable local data; Git history may retain older versions.
- Updated CI to provision a schema-sync role and run startup schema sync.
- Bound CORS to the configured frontend origin and reject invalid origin configuration.
- Tightened the API Content Security Policy.
- Removed prototype Files/Approvals handlers from the active HTTP routes until persistence and scope checks exist.
- Applied self-decision prevention to approval rejection as well as approval.
- Kept the in-memory domain handler prototypes out of the active HTTP routes pending durable persistence and authenticated authorization.

## Verification

- `cargo fmt --all -- --check`: passed.
- Clippy and the 400-line source check passed before the destination branch added domain handler prototypes; they were not reconfirmed against that newer snapshot.
- Database-backed request tests and live PostgreSQL startup sync were not run; no PostgreSQL service was available in the host environment.
- `cargo check --all-targets --all-features -j 1` could not be reconfirmed in this environment: the configured target directory was inaccessible, and an in-repo retry failed while MSVC attempted to overwrite the `ring` archive (LNK1114 / Windows error 5).

## Release decision

Suitable for local development and controlled evaluation only. Do not store real employee, payroll, identity, or uploaded document data until domain persistence, tenant/row policies, distributed abuse controls, operations, and passing CI checks are complete.
