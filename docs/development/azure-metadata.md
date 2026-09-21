# Azure display metadata

Azure Resource Graph returns machine-oriented values such as `uksouth` and
`GlobalDocumentDB`. azdocs preserves those values unchanged in SQLite and uses
checked-in metadata only at presentation boundaries.

| File | Key | Example |
|---|---|---|
| `data/display_names.toml` | Lowercase ARM resource type | `microsoft.compute/virtualmachines` → `Virtual Machine` |
| `data/azure_locations.toml` | Lowercase programmatic location | `uksouth` → `UK South` |
| `data/azure_kinds.toml` | Lowercase `<ARM type>:<kind>` | `microsoft.documentdb/databaseaccounts:globaldocumentdb` → `Global Document DB` |
| `data/azure_regions.toml` | Lowercase programmatic location | `uksouth` → London, United Kingdom, 50.94 N 0.80 W |

The first three files are embedded in the Rust binary and used at both output
surfaces, so one snapshot reads the same way whichever you render:

- **Reports and the TUI** resolve through `model::azure_values` as they render —
  the geographic footprint table, resource and group rows, the detail pages'
  location/kind properties, and the TUI record.
- **The desktop** receives the effective maps in its snapshot DTO and resolves
  in the frontend, so filters keep working against the stored codes. It also
  receives the region catalogue's coordinates (`azureMetadata.regions`), which
  draw the Overview's resource-locations map; a location the catalogue cannot
  place is listed but not plotted.

Stored values are never rewritten. `ReportContext::NameCount` carries the code
in `name` and the friendly value in `display`, the same split `TypeCount`
already used, so anything joining on a location still matches SQLite.

Unknown locations remain unchanged. Unknown kinds receive conservative
CamelCase and separator splitting.

> Because the desktop resolves in TypeScript, `humanize_identifier` exists twice
> — `src/model/azure_values.rs` and `desktop/src/azure-values.ts`. Both are live.
> `desktop/src/azure-values.test.ts` asserts they agree on a shared case list and
> that the Rust tests still cover it; change one and that test fails.

## Refreshing locations

Microsoft maintains a public [Azure regions list][regions-list] containing the
display name, physical location and programmatic name for each public-cloud
region, and its [datacenter map][datacenter-map] publishes a latitude and
longitude for every open and announced region. One example joins the two and
rewrites both checked-in files:

```sh
cargo run --example update_azure_locations
git diff -- data/azure_locations.toml data/azure_regions.toml
```

The catalogue records where each coordinate came from, and carries the
facts the desktop's region details dialog shows: availability-zone support
and the paired region from the regions list, and the opening year, open or
announced status and data-residency statement from the datacenter map. Restricted-access
regions the map omits (Korea South, West India, Norway West and so on) carry
the approximate centre of the city the regions list names, marked
`source = "physical-location"`; the table of those lives in the example and
only needs attention if Microsoft starts publishing them. Regions with no
coordinates at all (`global`) stay in the display-name file only.

To fail when the checked-in files differ from Microsoft's current data:

```sh
cargo run --example update_azure_locations -- --check
```

Regular builds do not refresh Azure metadata. Cargo and pnpm may still need
network access to obtain dependencies that are not cached locally. Keeping
metadata refresh separate preserves the offline runtime/test contract and
leaves source changes visible for review. Microsoft's
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
[datacenter-map]: https://datacenters.microsoft.com/globe/explore
[list-locations]: https://learn.microsoft.com/rest/api/resources/subscriptions/list-locations?view=rest-resources-2022-12-01
