# Running in CI

For environment-only or legacy configuration, supply the credentials through
environment variables:

```sh
export AZDOCS_TENANT_ID=... AZDOCS_CLIENT_ID=... AZDOCS_CLIENT_SECRET=...

azdocs collect --notes "$GIT_SHA"
azdocs report --format all
azdocs snapshots diff "$BASELINE" latest --format json > drift.json
```

Example GitHub Actions job:

```yaml
audit:
  runs-on: ubuntu-latest
  steps:
    - name: Collect and report
      env:
        AZDOCS_TENANT_ID: ${{ secrets.AZDOCS_TENANT_ID }}
        AZDOCS_CLIENT_ID: ${{ secrets.AZDOCS_CLIENT_ID }}
        AZDOCS_CLIENT_SECRET: ${{ secrets.AZDOCS_CLIENT_SECRET }}
      run: |
        azdocs collect --notes "${{ github.sha }}"
        azdocs report --format all
    - uses: actions/upload-artifact@v4
      with:
        name: azure-estate-report
        path: output/
```

Exit codes are non-zero on hard failure. A `partial` snapshot (some queries
failed, rest usable) does **not** fail the run — parse
`azdocs snapshots show latest` if you need stricter gating.

Next: [Troubleshooting](troubleshooting.md)

## Named profiles in automation

For a version 2 configuration, identity is explicit in `[tenants.<reference>]`.
Set `secret_env = "AZDOCS_ACME_SECRET"` and inject that variable from the CI
secret manager. Global legacy variables do not replace named-profile identity.
Pass `--tenant acme` for collection, queries, reports and history operations.
Headless runners do not need an OS credential store when using environment
references; offline exports do not fetch credentials at all.

`azdocs --tenant acme check` reports advisory RBAC evidence. Do not interpret a
successful exit as tenant-wide effective-access certification. Authentication
failures are errors; broader or unverifiable grants remain warnings.
