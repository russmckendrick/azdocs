# Working with snapshots

Every `collect` run creates a snapshot; reports, diagrams, and the TUI all
read from one. Snapshot ids can be abbreviated to any unique prefix.

```sh
azdocs snapshots list                  # selected tenant snapshots with counts
azdocs snapshots show latest           # per-query row counts, durations, errors
```

## Tenant isolation

History, `latest`, previous-snapshot comparisons and pruning are filtered in
SQLite by tenant ID. Pass `--tenant <profile-reference-or-tenant-id>`, configure
a default tenant, or use the sole profile. An unscoped mixed-tenant database
requires a tenant selection for these operations.

```sh
azdocs --tenant acme snapshots list
azdocs --tenant acme snapshots prune --keep 5 --yes
azdocs --db archived.db snapshots show <snapshot-id>
```

Explicit snapshot IDs remain available offline without credentials. An explicit
CLI tenant selection enforces ownership, and Rust rejects comparisons between
different tenants. Renaming/removing a profile never deletes snapshots. Reports
use the snapshot tenant's matching branding overrides, or shared defaults when
its profile no longer exists.

## Diffing estates over time

```sh
azdocs snapshots diff <a> <b>
azdocs snapshots diff e4fb3710 latest --format json   # machine-readable
```

Resources are compared by ARM id: **added**, **removed**, or **changed**
(the properties JSON differs). The JSON format is designed for CI drift
detection — see [CI](ci.md).

## Pruning

```sh
azdocs snapshots prune --keep 5 --yes
azdocs snapshots prune --older-than 90 --yes
```

Deletes cascade — all resources, edges, findings, and query results for a
pruned snapshot go with it.

Next: [Queries](queries.md)
