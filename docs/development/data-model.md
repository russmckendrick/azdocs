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

## The desktop wire contract

The Tauri frontend does not see these tables. It receives DTOs from
`desktop/src-tauri/src/dto.rs` and `topology.rs`, and their TypeScript
declarations are **generated**, not written by hand:

```sh
cargo test -p azdocs-desktop     # rewrites desktop/src/generated.ts
```

CI runs the same tests and then `git diff --exit-code`, so a struct change that
is not regenerated fails the build. Never edit `generated.ts`.

| File | Written by | Holds |
|---|---|---|
| `desktop/src/generated.ts` | ts-rs, via `src-tauri/src/bindings.rs` | Every DTO the IPC returns |
| `desktop/src/api-types.ts` | Hand | Closed string sets Rust serialises with `as_str()` — severity, edge kind, snapshot status — which ts-rs cannot infer. Referenced from the structs with `#[ts(type = "…")]` |
| `desktop/src/types.ts` | Hand | UI-only types, and the re-export surface everything imports |

Two serde behaviours this encodes, both of which had already produced bugs:

- `#[serde(rename_all)]` on an **enum** renames the variants. The fields of a
  struct variant need `rename_all_fields` — without it a payload goes out
  snake_case while every sibling field is camelCase.
- `Option<T>` serialises to `null`, not an absent key, so the generated type is
  `field?: T | null`.
