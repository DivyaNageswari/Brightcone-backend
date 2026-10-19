# People Plus task delivery

Reviewed: 19 October 2026
Branch: `feat/people-plus-production-hardening`

This checklist describes the implementation in this branch. “Implemented” means code is present; it does not mean the task has passed production verification.

| Task | Implementation in this branch | Remaining verification or decision |
|---|---|---|
| PGS-02 — GitHub project, API docs, frontend types | OpenAPI contract, generated UI declarations, API docs publishing workflow, CI drift checks, issue and label templates, project setup instructions. | Enable Pages and configure the organization project and labels. |
| PGS-05 — Forgot password and HR account tools | Password recovery and account administration APIs, role checks, audit records, and request tests; frontend screens are in Brightcone-HR-UI. | Run PostgreSQL-backed request tests and verify configured email delivery and expiry. |
| PGS-08 — Record registry and validation engine | Versioned 50-type catalog, typed/required/enum/date/email validation, fixed computed fields, and normalized transactional uniqueness. | Product owner must confirm the record contract before access is widened. |
| PGS-09 — Generic CRUD, archive/restore, history | Generic list/create/update/archive/restore/history APIs, soft-delete, audit integration, and append-only before/after history. | Run PostgreSQL-backed tests and approve retention, ownership, team scope, and restore policy. |
| PGS-10 — CSV export and backup/restore | Permission-aware CSV and bounded versioned JSON backup/import with validation, deduplication, transactional import, and no overwrite/delete behavior. | Run database-backed tests and approve backup encryption/key handling and cross-environment identity rules. |
| PGS-11 — Rules engine and six-role access control | Six-role deny-by-default scopes, nine disabled-by-default rules, transactional outbox, idempotent worker, in-app notifications, admin metrics and retry API. | Run database/queue tests and configure operational alerting. Role-wide recipients remain until team/manager relationships are available. |

## Verification snapshot

- Backend formatting check passed on 19 October 2026.
- Current backend compilation could not be verified: the configured Cargo target path was inaccessible; an in-repository retry failed in the native `ring` build with MSVC LNK1114 / Windows error 5.
- PostgreSQL-backed tests and startup sync were not run because no database service was reachable in the review environment.
- The separate Phase 1-3 module handlers use in-memory sample stores and are not registered in the HTTP router; persistent storage and authenticated authorization are prerequisites to enabling them.
- See `PROJECT_ASSESSMENT.md` and `STATUS.md` for architecture and release limitations.
