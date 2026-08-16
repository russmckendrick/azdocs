# Built-in query pack reference

The pack ships 41 queries, embedded in the binary from `queries/`. List the
live set (including your custom queries) with `azdocs query list`, print any
query's KQL with `azdocs query show <name>`, and run one ad-hoc with
`azdocs query run <name>`. Override or extend via the user `queries.d/`
directory — see [usage.md](usage.md#custom-queries).

> Regenerate this table after changing the pack: `azdocs query list`.

## Inventory

| Name | Category | Description |
|---|---|---|
| `all_resources` | inventory | Every resource with its full properties bag — the base inventory that populates the resources table |
| `subscriptions` | inventory | All subscriptions visible to the credential |
| `resource_groups` | inventory | All resource groups |
| `resource_type_counts` | inventory | Resource counts by type across the estate |
| `virtual_networks` | networking | Virtual networks with address spaces and DNS settings |
| `subnets` | networking | Subnets expanded from every virtual network |
| `vnet_peerings` | networking | VNet peerings with state and remote network |
| `network_security_groups` | networking | NSGs with rule counts and attachment summary |
| `public_ip_addresses` | networking | Public IP addresses and what they are attached to |
| `load_balancers` | networking | Load balancers with SKU and rule counts |
| `app_gateways` | networking | Application gateways with SKU, WAF state, and listener counts |
| `firewalls` | networking | Azure Firewalls with SKU and policy |
| `private_endpoints` | networking | Private endpoints and their target resources |
| `private_dns_zones` | networking | Private DNS zones with record and VNet link counts |
| `vpn_er_gateways` | networking | VPN and ExpressRoute gateways plus circuits |
| `virtual_machines` | compute | Virtual machines with size, OS, and power state |
| `vm_scale_sets` | compute | VM scale sets with capacity and orchestration mode |
| `managed_disks` | compute | Managed disks with size, SKU, encryption, and attachment |
| `aks_clusters` | compute | AKS clusters with version, node pools, and network plugin |
| `app_service_plans` | compute | App Service plans with SKU and app counts |
| `web_apps` | compute | App Services and Function Apps with runtime and TLS settings |
| `container_apps` | compute | Container Apps and their environments |
| `storage_accounts` | storage | Storage accounts with SKU, access tier, and network rules |
| `recovery_vaults` | storage | Recovery Services and Backup vaults |
| `sql_servers_and_dbs` | databases | SQL servers and databases with version and public network access |
| `cosmos_accounts` | databases | Cosmos DB accounts with API kind and consistency |
| `postgres_mysql_flexible` | databases | PostgreSQL and MySQL flexible servers |
| `redis_caches` | databases | Redis caches with SKU and TLS settings |
| `key_vaults` | identity | Key vaults with protection and network settings |
| `managed_identities` | identity | User-assigned managed identities |
| `resources_with_system_identity` | identity | Resources running with a system-assigned managed identity |

## Security findings

| Name | Severity | Description |
|---|---|---|
| `nsg_open_to_internet` | high | NSG rules allowing inbound traffic from the Internet |
| `storage_public_blob_access` | high | Storage accounts allowing anonymous public blob access |
| `sql_public_network_access` | high | Database servers reachable from public networks |
| `public_ip_exposed_resources` | high | Public IPs attached to NICs, exposing resources directly |
| `storage_http_allowed` | medium | Storage accounts accepting plain HTTP traffic |
| `disks_unencrypted_or_pmk` | medium | Managed disks without customer-managed or double encryption |
| `keyvault_no_purge_protection` | medium | Key vaults without purge protection |
| `keyvault_public_access` | medium | Key vaults reachable from all networks |
| `vms_without_managed_disks` | low | VMs still using unmanaged (blob) OS disks |
| `orphaned_resources` | info | Unattached disks, unassociated public IPs, and orphaned NICs |

## Rust-side audits (not KQL)

These run as post-passes during `collect` because they need configuration or
cross-resource context that a single query can't see:

| Name | Severity | Source |
|---|---|---|
| `missing_required_tags` | low | `collect/audit.rs`, driven by `[audit] required_tags` in the config |

Relationship **edges** (subnets, peerings, NSG associations, NIC/VM/disk
attachment, private endpoints, DNS links) are also derived in Rust
(`collect/extractors.rs`) rather than queried — see
[development.md](development.md#design-decisions-worth-knowing).
