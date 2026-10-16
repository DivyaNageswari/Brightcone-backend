# Backend Status

Last reviewed: 16 October 2026. This file reports the code currently in this branch; it does not certify production readiness. See [PROJECT_ASSESSMENT.md](PROJECT_ASSESSMENT.md) for detailed API/schema and release review.

## Implemented and currently exposed

- PostgreSQL persistence for users, refresh sessions, password resets, audit events, and the framework queue.
- Login, rotating refresh cookies, logout, password recovery, current-user profile, role/permission checks, admin user management, and audit read APIs.
- Request IDs, security headers, request limits/timeouts, configured frontend CORS, and queued email infrastructure.
- Refresh-token cleanup worker registration.
- Frontend authentication and People Plus shell are in the separate frontend repository.

## Not implemented as production features

- Employee/org/team records, leave, payroll, expenses, recruitment, benefits, schedules, learning, feed, and reporting APIs.
- Durable upload metadata/object storage and durable approvals. Existing handlers are prototypes and are not registered as routes.
- Tenant isolation; current roles are global. Multi-replica rate limiting and operational deployment controls.

## Branch work

- Removed the committed development JWT fallback and sanitized tracked documentation that contained demo credentials. Rotate any credential ever used outside disposable local data; Git history may retain older versions.
- Corrected CI migration URL YAML syntax.
- Bound CORS to the configured frontend origin and reject invalid origin configuration.
- Tightened the API Content Security Policy.
- Removed prototype Files/Approvals handlers from the active HTTP routes until persistence and scope checks exist.
- Applied self-decision prevention to approval rejection as well as approval.

## Verification

- `cargo fmt --all -- --check`: passed.
- `cargo test --all-features -j 1`: not completed; Windows native dependency linking failed with `LNK1114` while building `ring`.
- Clippy: not verified in this environment.

## Release decision

Suitable for local development and controlled evaluation only. Do not store real employee, payroll, identity, or uploaded document data until the missing domain persistence, tenant/row policies, distributed abuse controls, operations, and passing CI checks are complete.
