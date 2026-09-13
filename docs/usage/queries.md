# Queries

Live query execution accepts `--tenant <reference>` and repeats the shared
[permission preflight](permissions.md). Stored query evidence remains offline.

## Ad-hoc queries

Run an ARG-supported KQL query and print its result without creating a snapshot:

```sh
azdocs query list                        # full pack + your custom queries
azdocs query show nsg_open_to_internet   # print the KQL
azdocs query run virtual_machines                   # by name
azdocs query run ./my-query.kql --format json       # from a file
echo 'resources | count' | azdocs query run -       # from stdin
azdocs query run subnets --format csv > subnets.csv
```

## Custom queries

Drop TOML files into the user queries directory (`azdocs query list` prints
the exact path). Definitions with the same TOML `name` **override** built-ins, regardless of
filename; new names join the pack and run on an unfiltered `collect`.

```toml
name = "expensive_vm_sizes"          # snake_case, unique
category = "compute"                 # new categories create new report sections
kind = "inventory"                   # "inventory" or "finding"
description = "VMs using E-series sizes"
kql = '''
resources
| where type == "microsoft.compute/virtualmachines"
| where properties.hardwareProfile.vmSize startswith "Standard_E"
| project id, name, subscriptionId, vmSize = tostring(properties.hardwareProfile.vmSize)
| order by id asc
'''
```

### Rules

- **Finding queries** additionally need
  `severity = "high"|"medium"|"low"|"info"` and optionally
  `title_field = "<column>"` (falls back to `name`, then `id`).
- `resource_id_field = "<column>"` optionally names the affected resource ID
  for a finding; it defaults to `id`. This separates a unique policy evidence
  row from its target resource. A missing/empty configured field leaves the
  finding at estate/scope level; it never falls back to the evidence ID.
- End with `| order by id asc` (or another deterministic sort) — multi-page
  results paginate via `$skipToken`, which needs stable ordering. Avoid
  `take`/`limit`/`sample` when all rows are required, and retain scalar output
  columns; sorting alone does not make every query pageable. Truncated results
  without a continuation token are an error.
- Never name a projected column `count` — it's a KQL reserved word and ARG
  rejects it with HTTP 400.

The full built-in pack is documented in the
[query reference](../reference/queries.md).


## Source metadata, inherited scope and age checks

A query definition can preserve its source and specify assignment scope. These
options apply to both `collect` and `query run <name-or-toml-file>`. Raw KQL files
and stdin retain ARG's default scope; they do not infer options from comments.

```toml
name = "custom_patch_assessments"
category = "operations"
kind = "inventory"
description = "Patch assessment evidence"
kql = "patchassessmentresources | project id, assessedAt = properties.lastModifiedDateTime | order by id asc"

[source]
urls = ["https://learn.microsoft.com/en-us/azure/update-manager/query-logs"]
reviewed_on = "2026-09-13"
# revision = "upstream-commit-or-release" # Optional; omit when unknown.

[freshness]
timestamp_field = "assessedAt"
max_age_hours = 72
```

For an assignment query, set `authorization_scope = "AtScopeAboveAndBelow"` at
the top level, before any TOML table headers. Accepted values are
`AtScopeAndBelow` (default), `AtScopeAndAbove`, `AtScopeAboveAndBelow` and
`AtScopeExact`. Only set this option for tables supporting assignment scope.
It cannot grant access to an inaccessible parent scope.

`freshness.timestamp_field` names a projected timestamp column;
`max_age_hours` must be positive. The report compares valid timestamps with the
snapshot instant, never the export time. Missing/invalid and future timestamps
are separate states. A threshold is a review aid, not a provider retention
period or an assertion that a workload's backup RPO has been breached.

Copy the complete built-in TOML into `queries.d/` with the same `name` to change
its threshold. User queries replace a complete definition rather than merging
individual fields. Arbitrary inventory queries can opt into freshness this way.
New collected query runs retain the exact KQL, SHA-256, category, kind,
description, severity, title/affected-resource fields, source metadata, freshness
settings and requested scope in SQLite.
See [operational evidence](../reference/operational-evidence.md).

Next: [CI](ci.md)
