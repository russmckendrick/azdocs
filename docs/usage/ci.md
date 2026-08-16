# Running in CI

Keep the secret out of the config file — use environment variables:

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
