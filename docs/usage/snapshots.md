# Working with snapshots

Every `collect` run creates a snapshot; reports, diagrams, and the TUI all
read from one. Snapshot ids can be abbreviated to any unique prefix.

```sh
azdocs snapshots list                  # all snapshots with counts
azdocs snapshots show latest           # per-query row counts, durations, errors
```

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
