# GitHub project setup

## Repository automation in source control

- `.github/labels.yml` is the canonical label list.
- `.github/ISSUE_TEMPLATE/` supplies feature and bug intake forms with testable acceptance criteria.
- `.github/workflows/publish-api-docs.yml` publishes the committed OpenAPI contract and Scalar reference on pushes to `main`.
- `tests/openapi.rs` detects stale backend OpenAPI output. The frontend CI regenerates all API type files and fails if any generated file changes.

## One-time repository settings

1. In repository Settings → Pages, select **GitHub Actions** as the build and deployment source. The workflow needs the `github-pages` environment and the repository permissions already declared in the workflow.
2. Run the `Publish API reference` workflow once or merge an OpenAPI change. The deployed page URL is shown in the workflow deployment environment; it is typically `https://<owner>.github.io/<repository>/`.
3. Create the GitHub Project named **Brightcone HR-Portal** and add the API and UI repositories as sources. Use fields `Status` (Backlog, Todo, In progress, Blocked, Done), `Priority` (urgent, high, medium, low), `Owner`, `Sprint`, and `Task ID`.
4. Add board views grouped by Status and Sprint. Automate status to In progress on assignment or PR linkage, and Done only on merged PR; use issue/PR links as completion evidence.
5. Apply `.github/labels.yml` when bootstrapping repository labels. GitHub Projects are account/org resources and are not created by repository YAML alone.

## Contract update sequence

1. Update backend route DTO/OpenAPI annotations.
2. Regenerate and commit `repos/brightcone-hr-backend/openapi.json`.
3. Copy that OpenAPI file to `repos/brightcone-hr-frontend/openapi.json`.
4. Run `npm run gen:api` in the frontend and commit every changed file under `src/api`.
5. Backend OpenAPI tests and frontend CI verify the generated contracts remain committed and current.
