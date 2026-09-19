# Running in CI

Use a service principal with Reader access and inject credentials from the
pipeline's secret store. Environment-only operation needs no config file:

```sh
# Inject these through your CI secret manager, not literal values in a script:
# AZDOCS_TENANT_ID, AZDOCS_CLIENT_ID, AZDOCS_CLIENT_SECRET
azdocs collect --notes "pipeline collection"
azdocs report --format all --include-reference
```

## GitHub Actions example

This complete workflow builds the CLI from the checked-out azdocs source,
collects a fresh snapshot, and uploads offline reports. Put it in an azdocs
fork or adapt checkout to the repository and revision you intend to run.
Configure the three repository secrets before dispatching it.

```yaml
name: Azure estate report
on:
  workflow_dispatch:
permissions:
  contents: read
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install CLI
        run: cargo install --path . --locked
      - name: Collect and report
        env:
          AZDOCS_TENANT_ID: ${{ secrets.AZDOCS_TENANT_ID }}
          AZDOCS_CLIENT_ID: ${{ secrets.AZDOCS_CLIENT_ID }}
          AZDOCS_CLIENT_SECRET: ${{ secrets.AZDOCS_CLIENT_SECRET }}
        run: |
          azdocs check
          azdocs collect --quiet --fail-on partial --notes "$GITHUB_SHA"
          azdocs report --format all --include-reference
      - uses: actions/upload-artifact@v4
        with:
          name: azure-estate-report
          path: output/
          retention-days: 7
```

Reports contain estate details. Keep this workflow and its artifacts in a
repository whose access matches that data; do not run a real customer collection
in a public fork. Avoid automatic runs for untrusted pull requests.

## History and failure handling

A hosted runner starts without your old database. To compare or retain history,
restore a protected snapshot database before collection and persist it afterward.
Use the same explicit `--db <path>` for collection, reports and comparisons.
Close all azdocs processes before copying a SQLite database, or use a SQLite
backup operation; copying only an active `.db` can omit data still in its WAL.
Reports, diagrams and `snapshots list|show|diff` open the database read-only,
so an archived baseline is never migrated or rewritten by a newer azdocs; a
database written by a newer schema is refused rather than opened blind.
A baseline ID must exist in that database and belong to the same tenant:

```sh
azdocs --db history.db snapshots list
# Set BASELINE to a stored snapshot ID from the selected tenant.
azdocs --db history.db snapshots diff "$BASELINE" latest --format json > drift.json
```

Exit codes are the gate. `azdocs check` exits non-zero when the credential can
see no subscriptions or Azure cannot describe its identity, so a preflight with
an unassigned principal fails instead of passing on a valid token. `collect`
exits non-zero when the snapshot outcome matches `--fail-on`: the default
`failed` fails only when every query failed; `--fail-on partial` also fails
when any query failed. Either way the snapshot line is printed first. A
`failed` snapshot is never resolved as `latest`, so a later `report` step in
the same job sees the last good collection rather than an empty estate.

```sh
azdocs check --format json | jq -e .ok
azdocs collect --quiet --fail-on partial --notes "$GITHUB_SHA"
azdocs snapshots show latest --format json | jq '.query_runs[] | select(.error != null)'
```

Findings are observations and do not themselves set a failing exit code.

## Named profiles in automation

For version 2 configuration, identity is explicit in `[tenants.<reference>]`.
Set `secret_env = "AZDOCS_ACME_SECRET"` and inject that variable from the CI
secret manager. Global legacy variables do not replace named-profile identity.
Pass `--tenant acme` for collection, queries, reports and history operations.
Headless runners do not need an OS credential store when using environment
references; offline exports do not fetch credentials.

`azdocs --tenant acme check` reports advisory RBAC evidence. A successful exit
does not certify tenant-wide effective access. Authentication failures are
errors; broader or unverifiable grants remain warnings.

Next: [Troubleshooting](troubleshooting.md)
