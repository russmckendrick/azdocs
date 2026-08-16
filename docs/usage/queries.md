# Queries

## Ad-hoc queries

Run any KQL against Resource Graph without storing anything:

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
the exact path). Same-named files **override** built-ins; new names join the
pack and run on every `collect`.

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
- End with `| order by id asc` (or another deterministic sort) — multi-page
  results paginate via `$skipToken`, which needs stable ordering.
- Never name a projected column `count` — it's a KQL reserved word and ARG
  rejects it with HTTP 400.

The full built-in pack is documented in the
[query reference](../reference/queries.md).

Next: [CI](ci.md)
