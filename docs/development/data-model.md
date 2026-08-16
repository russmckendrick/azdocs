# Data model

One SQLite database (bundled — no system dependency), WAL mode, snapshot-scoped
rows with `ON DELETE CASCADE` from `snapshots`.

```mermaid
erDiagram
    snapshots ||--o{ subscriptions : has
    snapshots ||--o{ resource_groups : has
    snapshots ||--o{ resources : has
    snapshots ||--o{ edges : has
    snapshots ||--o{ findings : has
    snapshots ||--o{ query_runs : has
    snapshots ||--o{ query_results : has

    snapshots {
        text id PK "uuid"
        text created_at "RFC3339"
        text tenant_id
        text status "running|complete|partial|failed"
        text notes
    }
    resources {
        text id PK "lowercased ARM id"
        text display_id "original casing"
        text name
        text type "lowercased"
        text location
        text resource_group "lowercased"
        text subscription_id
        text tags "JSON"
        text properties "full JSON bag"
    }
    edges {
        text source_id "lowercased ARM id"
        text target_id "lowercased ARM id"
        text edge_type "subnet_of, peered_with, ..."
        text properties "JSON"
    }
    findings {
        text query_name "provenance"
        text category
        text severity "high|medium|low|info"
        text resource_id
        text title
        text detail "JSON row"
    }
    query_runs {
        text query_name
        int row_count
        int duration_ms
        text error
    }
    query_results {
        text query_name
        int row_index
        text row "raw JSON row"
    }
```

## Key points

- **Lowercased ids** are the join key everywhere (`resources.id`, edge
  endpoints, `findings.resource_id`); `display_id` keeps original casing for
  display. See [architecture](architecture.md#design-decisions) for why.
- **`query_results`** holds raw rows of the shaped inventory queries
  (mv-expanded subnets, peerings, …) verbatim — report category tables render
  straight from them.
- **`query_runs`** is the per-snapshot audit trail: which query ran, how many
  rows, how long, what failed.
- **Snapshot diff** is one `FULL OUTER JOIN` over `resources` between two
  snapshot ids (`store/snapshots.rs`): added / removed / changed (properties
  text differs).

## Migrations

Ordered SQL strings in `store/schema.rs`; `meta.schema_version` records how
many have run. **Append new migrations, never edit existing ones.** New tables
must cascade-delete from `snapshots`.
