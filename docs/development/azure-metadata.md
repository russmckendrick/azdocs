# Azure display metadata

Azure Resource Graph returns machine-oriented values such as `uksouth` and
`GlobalDocumentDB`. azdocs preserves those values unchanged in SQLite and uses
checked-in metadata only at presentation boundaries.

| File | Key | Example |
|---|---|---|
| `data/display_names.toml` | Lowercase ARM resource type | `microsoft.compute/virtualmachines` → `Virtual Machine` |
| `data/azure_locations.toml` | Lowercase programmatic location | `uksouth` → `UK South` |
| `data/azure_kinds.toml` | Lowercase `<ARM type>:<kind>` | `microsoft.documentdb/databaseaccounts:globaldocumentdb` → `Global Document DB` |

All three files are embedded in the Rust binary. The desktop backend includes
the effective location and kind maps in its offline snapshot DTO; the frontend
uses the friendly values for display while filters continue to use the stored
codes. Unknown locations remain unchanged. Unknown kinds receive conservative
CamelCase and separator splitting.

## Refreshing locations

Microsoft maintains a public [Azure regions list][regions-list] containing the
display name and programmatic name for each public-cloud region. Refresh the
checked-in location file explicitly:

```sh
cargo run --example update_azure_locations
git diff -- data/azure_locations.toml
```

To fail when the checked-in file differs from Microsoft's current table:

```sh
cargo run --example update_azure_locations -- --check
```

The regular Cargo and desktop builds never fetch the network. Keeping refresh
separate makes builds reproducible, preserves the project's offline test
contract, and leaves source changes visible for review. Microsoft's
authenticated [List Locations REST API][list-locations] is subscription-aware;
it is useful for runtime discovery but is not a stable input to a general build.

## Maintaining kinds

Azure does not publish one cross-provider catalogue of `kind` display names.
Kinds are defined by individual resource-provider schemas, so mappings stay
scoped to the ARM type in `data/azure_kinds.toml`. Add an entry when a raw kind
is materially clearer with product-specific wording. Do not turn kinds into a
global map unless Microsoft defines them globally.

## User overrides

Site-specific names can be added without rebuilding by creating either of:

- `<platform config dir>/azdocs/azure_locations.toml`
- `<platform config dir>/azdocs/azure_kinds.toml`

They use the same flat TOML shapes as the built-in files and merge over the
checked-in entries. Invalid override files are ignored with a warning.

[regions-list]: https://learn.microsoft.com/azure/reliability/regions-list
[list-locations]: https://learn.microsoft.com/rest/api/resources/subscriptions/list-locations?view=rest-resources-2022-12-01
