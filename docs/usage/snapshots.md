# Working with snapshots

Every `collect` run creates a snapshot; reports, diagrams, and the TUI all
read from one. Snapshot ids can be abbreviated to any unique prefix.

```sh
azdocs snapshots list                  # selected tenant snapshots with counts
azdocs snapshots show latest           # per-query row counts, dropped rows, durations, errors
azdocs snapshots list --format json    # the same, for scripts
```

`latest` always means the newest `complete` or `partial` snapshot. A `failed`
or `cancelled` snapshot, or one still `running`, is only reachable by id.

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
(the stored properties JSON text differs). Tags and other top-level resource
fields are separate columns, so changes confined to those fields do not appear
as `changed`. This is a resource-properties comparison, not a complete audit
of every stored field or finding. See [CI](ci.md) for retaining a baseline.

## Deleting and pruning

```sh
azdocs snapshots delete <id> --yes            # one snapshot
azdocs snapshots prune --keep 5 --yes
azdocs snapshots prune --older-than 90 --yes --vacuum
```

Deletes cascade — all resources, edges, findings, query results and website
captures for a pruned snapshot go with it, in one transaction. `prune` never
selects a `running` snapshot and does not count one toward `--keep`; `delete`
refuses a `running` snapshot unless `--force` is passed. `--vacuum` reclaims
the file space afterwards, which matters once screenshots have been stored.

## Interrupted collections

A collect writes a heartbeat every 30 seconds. If the process dies, the next
writable open (a collect, prune, delete or `verify`) marks any `running`
snapshot without a heartbeat in the last ten minutes as `failed` and records
when that happened; `snapshots show` prints the interruption. Nothing is left
permanently `running`.

## Verifying a database

```sh
azdocs snapshots verify
azdocs snapshots verify --format json
```

Applies pending migrations, runs SQLite's integrity and foreign-key checks,
reconciles abandoned collects and prints the schema version. It exits non-zero
when the file is damaged. This is the command to run after restoring a
database from backup, and the one a read-only command names when it meets a
database that still needs migrating.

Next: [Queries](queries.md)
