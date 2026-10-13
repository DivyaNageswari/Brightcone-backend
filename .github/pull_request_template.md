Closes #

## What changed

## Definition of Done

- [ ] Reviewed by someone else
- [ ] Tests written and passing in CI (includes "wrong role / other user gets 403 or 404")
- [ ] No secrets or personal data in code or logs
- [ ] New endpoints added to `src/openapi.rs`; `openapi.json` regenerated (`make openapi`)
- [ ] Every new endpoint declares the permission it needs
- [ ] New tables have indexes on foreign keys and filter columns
